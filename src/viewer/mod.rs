mod image_grid_model;
mod media_loader;
mod playing_video;

use crate::db::IndexDb;
use crate::ui::{MediaViewerBridge, MediaViewerModel, PhotoFlowApp, ViewerState};
use crate::video::VideoLoader;
use crate::viewer::image_grid_model::ImageGridModel;
use crate::viewer::media_loader::{Media, MediaLoader};
use crate::viewer::playing_video::PlayingVideo;
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
    let bridge = app.global::<MediaViewerBridge>();

    let video_loader = Arc::new(Mutex::new(None));
    let loader = MediaLoader::new(db, video_loader.clone());
    let playing = PlayingVideo::default();

    bridge.on_load({
        let app_weak = app.as_weak();
        let loader = loader.clone();
        let playing = playing.clone();

        move |idx| {
            let _ = load(&app_weak, &loader, idx as usize, &playing);
        }
    });

    bridge.on_clear({
        let app_weak = app.as_weak();
        let loader = loader.clone();
        let playing = playing.clone();

        move || {
            let _ = clear(&app_weak, &loader, &playing);
        }
    });

    app.window()
        .set_rendering_notifier({
            let app_weak = app.as_weak();
            let video_loader = video_loader.clone();
            let playing = playing.clone();

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
    playing: &PlayingVideo,
) -> anyhow::Result<()> {
    loader.load(
        idx,
        weak_app.clone(),
        {
            let playing_video = playing.clone();
            move |app, path| {
                on_load_start(app, path, playing_video);
            }
        },
        {
            let playing_video = playing.clone();
            move |app, result| {
                on_load_finish(app, playing_video, result);
            }
        },
    )
}

fn on_load_start(app: PhotoFlowApp, path: &str, playing: PlayingVideo) {
    let file_name = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path);

    let bridge = app.global::<MediaViewerBridge>();
    let model = bridge.get_model();
    let mut image = model.image;

    if let Some(frame) = playing.copy_current_frame_and_stop() {
        image = frame;
    }

    bridge.set_model(MediaViewerModel {
        state: ViewerState::Loading,
        file_name: file_name.into(),
        image,
    });
}

fn on_load_finish(app: PhotoFlowApp, playing: PlayingVideo, result: anyhow::Result<Media>) {
    let bridge = app.global::<MediaViewerBridge>();
    let model = bridge.get_model();

    match result {
        Ok(media) => {
            bridge.set_model(MediaViewerModel {
                state: ViewerState::Loaded,
                file_name: model.file_name,
                image: match media {
                    Media::Image(img) => img,
                    Media::Video(video) => {
                        playing.set(video);
                        model.image
                    }
                },
            });
        }
        Err(_) => {
            bridge.set_model(MediaViewerModel {
                state: ViewerState::FailedToLoad,
                file_name: model.file_name,
                image: Image::default(),
            });
        }
    };
}

fn clear(
    weak_app: &Weak<PhotoFlowApp>,
    loader: &MediaLoader,
    playing: &PlayingVideo,
) -> Option<()> {
    loader.cancel_loading();
    playing.stop();

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
