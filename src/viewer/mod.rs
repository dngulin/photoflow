mod image_grid_model;
mod media_loader;
mod playing_video;

use self::image_grid_model::ImageGridModel;
use self::media_loader::MediaLoader;
use self::playing_video::CurrentVideo;
use crate::db::IndexDb;
use crate::media::{Media, MediaType};
use crate::ui::{MediaViewerBridge, MediaViewerModel, PhotoFlowApp, TimeFormatter, ViewerState};
use crate::util;
use crate::video::VideoLoader;
use anyhow::anyhow;
use slint::{ComponentHandle, Image, RenderingState, Weak};
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub fn bind_gallery_models(app: &PhotoFlowApp, db: Arc<Mutex<IndexDb>>) -> anyhow::Result<()> {
    {
        let db = db.lock().map_err(|_| anyhow!("Failed to lock IndexDB"))?;
        let item_count = db.get_item_count()?;
        app.invoke_set_item_count(item_count as i32);
    }

    let image_grid_model = Rc::new(ImageGridModel::new(db.clone()));
    app.set_grid_model(image_grid_model.clone().into());
    app.on_set_grid_visible_range(move |offset, len| {
        image_grid_model.set_range(offset as usize, len as usize);
    });

    Ok(())
}

pub fn bind_media_viewer(app: &PhotoFlowApp, db: Arc<Mutex<IndexDb>>) {
    let formatter = app.global::<TimeFormatter>();
    formatter.on_hh_mm_ss(move |duration_seconds| {
        util::hh_mm_ss((duration_seconds * 1000.0).round() as u64).into()
    });

    let bridge = app.global::<MediaViewerBridge>();

    let video_loader = Arc::new(Mutex::new(None));
    let loader = MediaLoader::new(db, video_loader.clone());
    let curr_video = CurrentVideo::default();

    bridge.on_load({
        let app_weak = app.as_weak();
        let loader = loader.clone();
        let curr_video = curr_video.clone();
        move |idx| {
            let _ = load(&app_weak, &loader, idx as usize, &curr_video);
        }
    });

    bridge.on_clear({
        let app_weak = app.as_weak();
        let loader = loader.clone();
        let curr_video = curr_video.clone();
        move || {
            clear(&app_weak, &loader, &curr_video);
        }
    });

    bridge.on_poll_video_state({
        let app_weak = app.as_weak();
        let curr_video = curr_video.clone();
        move || {
            set_video_state(&app_weak, &curr_video);
        }
    });

    bridge.on_video_set_playing({
        let curr_video = curr_video.clone();
        move |playing_state| {
            curr_video.set_playing(playing_state);
        }
    });

    bridge.on_video_seek_progress({
        let curr_video = curr_video.clone();
        move |progress| {
            curr_video.seek(progress);
        }
    });

    app.window()
        .set_rendering_notifier({
            let app_weak = app.as_weak();
            let video_loader = video_loader.clone();
            let playing = curr_video.clone();

            move |state, api| match state {
                RenderingState::RenderingSetup => {
                    if let Ok(loader) = VideoLoader::new(app_weak.clone(), api) {
                        video_loader.lock().unwrap().replace(loader);
                    }
                }
                RenderingState::BeforeRendering => {
                    if let Some(frame) = playing.curr_video_gl_frame() {
                        try_set_bridge_image(&app_weak, frame);
                    }
                }
                RenderingState::RenderingTeardown => {
                    video_loader.lock().unwrap().take();
                }
                _ => {}
            }
        })
        .ok();
}

fn load(
    weak_app: &Weak<PhotoFlowApp>,
    loader: &MediaLoader,
    idx: usize,
    curr_video: &CurrentVideo,
) -> anyhow::Result<()> {
    loader.load(
        idx,
        weak_app.clone(),
        {
            let playing_video = curr_video.clone();
            move |app, path| {
                on_load_start(app, path, playing_video);
            }
        },
        {
            let playing_video = curr_video.clone();
            move |app, result| {
                on_load_finish(app, playing_video, result);
            }
        },
    )
}

fn on_load_start(app: PhotoFlowApp, path: &str, curr_video: CurrentVideo) {
    let file_name = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path);

    let bridge = app.global::<MediaViewerBridge>();
    let model = bridge.get_model();
    let mut image = model.image;

    if let Some(frame) = curr_video.copy_current_frame_and_stop() {
        image = frame;
    }

    let is_video = MediaType::from_path(path)
        .map(|mt| match mt {
            MediaType::Image(_) => false,
            MediaType::Video(_) => true,
        })
        .unwrap_or(false);

    bridge.set_model(MediaViewerModel {
        state: ViewerState::Loading,
        file_name: file_name.into(),
        image,
        is_video,
        ..Default::default()
    });
}

fn on_load_finish(app: PhotoFlowApp, curr_video: CurrentVideo, result: anyhow::Result<Media>) {
    let bridge = app.global::<MediaViewerBridge>();
    let model = bridge.get_model();

    let model = match result {
        Ok(media) => match media {
            Media::Image(image) => MediaViewerModel {
                state: ViewerState::Loaded,
                image,
                ..model
            },
            Media::Video(video) => {
                let duration = video.duration().unwrap_or_default();
                let _ = video.set_playing(true);
                curr_video.set(video);
                MediaViewerModel {
                    state: ViewerState::Loaded,
                    video_duration: duration,
                    ..model
                }
            }
        },
        Err(_) => MediaViewerModel {
            state: ViewerState::FailedToLoad,
            image: Image::default(),
            ..model
        },
    };

    bridge.set_model(model);
}

fn clear(
    weak_app: &Weak<PhotoFlowApp>,
    loader: &MediaLoader,
    curr_video: &CurrentVideo,
) -> Option<()> {
    loader.cancel_loading();
    curr_video.stop();

    let app = weak_app.upgrade()?;
    let bridge = app.global::<MediaViewerBridge>();
    bridge.set_model(MediaViewerModel::default());

    Some(())
}

fn try_set_bridge_image(app_weak: &Weak<PhotoFlowApp>, image: Image) {
    if let Some(app) = app_weak.upgrade() {
        let bridge = app.global::<MediaViewerBridge>();
        let model = bridge.get_model();
        bridge.set_model(MediaViewerModel { image, ..model });
    }
}

fn set_video_state(weak_app: &Weak<PhotoFlowApp>, curr_video: &CurrentVideo) -> Option<()> {
    let app = weak_app.upgrade()?;
    let bridge = app.global::<MediaViewerBridge>();
    let model = bridge.get_model();

    let video_state = curr_video.state()?;

    bridge.set_model(MediaViewerModel {
        video_is_playing: video_state.is_playing,
        video_position: video_state.position,
        ..model
    });

    Some(())
}
