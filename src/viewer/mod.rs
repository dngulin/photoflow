mod image_grid_model;
mod video_loader;

use self::image_grid_model::ImageGridModel;

use crate::db::IndexDb;
use crate::exif_orientation::ExifOrientation;
use crate::img_decoder;
use crate::ui::{MediaViewerBridge, MediaViewerModel, PhotoFlowApp, ViewerState};
use crate::viewer::video_loader::VideoLoader;
use anyhow::anyhow;
use slint::{
    ComponentHandle, Image, RenderingState, Rgb8Pixel, SharedPixelBuffer, SharedString, Weak,
};
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};

pub fn bind_models(app: &PhotoFlowApp, db: Arc<Mutex<IndexDb>>) -> anyhow::Result<()> {
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

    let weak_app = app.as_weak();
    app.on_close(move || {
        if let Some(app) = weak_app.upgrade() {
            let _ = app.window().hide();
        }
    });

    Ok(())
}

#[derive(Clone)]
struct MediaLoader {
    db: Arc<Mutex<IndexDb>>,
    requested_idx: Arc<Mutex<Option<usize>>>,
    player: Arc<Mutex<Option<VideoLoader>>>,
}

impl MediaLoader {
    pub fn new(db: Arc<Mutex<IndexDb>>) -> Self {
        Self {
            db,
            requested_idx: Default::default(),
            player: Default::default(),
        }
    }

    pub fn ensure_requested(&self, idx: usize) -> Option<()> {
        let target = self.requested_idx.lock().ok()?;

        if target.eq(&Some(idx)) {
            Some(())
        } else {
            None
        }
    }

    pub fn player(&self) -> MutexGuard<Option<VideoLoader>> {
        self.player.lock().unwrap()
    }
}

pub fn bind_media_loader(app: &PhotoFlowApp, db: Arc<Mutex<IndexDb>>) {
    let bridge = app.global::<MediaViewerBridge>();
    let loader = MediaLoader::new(db);

    {
        let weak_app = app.as_weak();
        let loader = loader.clone();
        bridge.on_load(move |idx| {
            let _ = load(weak_app.clone(), &loader, idx as usize);
        });
    }

    {
        let weak_app = app.as_weak();
        let loader = loader.clone();
        bridge.on_clear(move || {
            let _ = clear(weak_app.clone(), &loader);
        });
    }

    {
        let loader = loader.clone();
        let app_weak = app.as_weak();
        let _ = app
            .window()
            .set_rendering_notifier(move |state, api| match state {
                RenderingState::RenderingSetup => {
                    let app_weak = app_weak.clone();
                    let request_redraw = move || {
                        let _ = app_weak.upgrade_in_event_loop(|app| {
                            app.window().request_redraw();
                        });
                    };
                    *loader.player() = VideoLoader::new(api, request_redraw).ok();
                }
                RenderingState::BeforeRendering => {
                    if let Some(frame) = loader
                        .player()
                        .as_ref()
                        .and_then(|p| p.playback())
                        .and_then(|p| p.current_frame_gl_ref())
                    {
                        let app = app_weak.unwrap();
                        let bridge = app.global::<MediaViewerBridge>();
                        bridge.set_model(MediaViewerModel {
                            image: frame,
                            ..bridge.get_model()
                        });
                    }
                }
                RenderingState::RenderingTeardown => {
                    loader.player().take();
                }
                _ => {}
            });
    }
}

fn load(weak_app: Weak<PhotoFlowApp>, loader: &MediaLoader, idx: usize) -> Option<()> {
    let mut requested_idx = loader.requested_idx.lock().ok()?;

    if *requested_idx == Some(idx) {
        return None;
    }

    let app = weak_app.upgrade()?;
    let (path, orientation) = {
        let db = loader.db.lock().ok()?;
        db.get_path_and_orientation(idx).ok()?
    };
    let orientation: ExifOrientation = orientation.try_into().unwrap_or_default();
    let file_name = Path::new(&path).file_name()?.to_str()?;

    let bridge = app.global::<MediaViewerBridge>();
    let model = bridge.get_model();
    let mut image = model.image;

    if let Some(player) = loader.player.lock().unwrap().as_mut() {
        if let Some(playback) = player.playback() {
            if let Some(frame) = playback.current_frame_copy() {
                image = frame;
            }
            player.unload()
        }
    }

    bridge.set_model(MediaViewerModel {
        state: ViewerState::Loading,
        file_name: file_name.into(),
        image,
    });

    let weak_app = weak_app.clone();
    let loader = loader.clone();

    *requested_idx = Some(idx);
    rayon::spawn_fifo(move || {
        let opt_media = load_media(&loader, idx, path, orientation);

        let _ = weak_app.upgrade_in_event_loop(move |app| {
            let mut requested = loader.requested_idx.lock().unwrap();
            if *requested != Some(idx) {
                return;
            }
            *requested = None;

            match opt_media {
                None => set_failed_to_load_media(&app),
                Some(media) => match media {
                    Media::Image(buffer) => set_media_loaded(&app, Some(buffer)),
                    Media::Video => set_media_loaded(&app, None),
                },
            }
        });
    });

    Some(())
}

enum Media {
    Image(SharedPixelBuffer<Rgb8Pixel>),
    Video,
}

fn load_media(
    loader: &MediaLoader,
    idx: usize,
    path: String,
    orientation: ExifOrientation,
) -> Option<Media> {
    loader.ensure_requested(idx)?;

    let path = Path::new(&path);

    if img_decoder::is_extension_supported(path) {
        let image = img_decoder::open(Path::new(&path))
            .map(|img| img.oriented(orientation))
            .ok()?;
        let rgb = image.into_rgb8();
        let buffer = SharedPixelBuffer::<Rgb8Pixel>::clone_from_slice(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
        );
        return Some(Media::Image(buffer));
    }

    loader
        .player()
        .as_mut()
        .and_then(|player| player.load(path).ok())
        .map(|_| Media::Video)
}

fn set_media_loaded(app: &PhotoFlowApp, buffer: Option<SharedPixelBuffer<Rgb8Pixel>>) {
    let bridge = app.global::<MediaViewerBridge>();
    let model = bridge.get_model();
    bridge.set_model(MediaViewerModel {
        state: ViewerState::Loaded,
        file_name: model.file_name,
        image: buffer.map(Image::from_rgb8).unwrap_or(model.image),
    });
}

fn set_failed_to_load_media(app: &PhotoFlowApp) {
    let bridge = app.global::<MediaViewerBridge>();
    let model = bridge.get_model();
    bridge.set_model(MediaViewerModel {
        state: ViewerState::FailedToLoad,
        file_name: model.file_name,
        image: Image::default(),
    });
}

fn clear(weak_app: Weak<PhotoFlowApp>, loader: &MediaLoader) -> Option<()> {
    *loader.requested_idx.lock().ok()? = None;

    if let Some(player) = loader.player().as_mut() {
        player.unload()
    }

    let app = weak_app.upgrade()?;
    let bridge = app.global::<MediaViewerBridge>();
    bridge.set_model(MediaViewerModel {
        state: ViewerState::Loaded,
        file_name: SharedString::default(),
        image: Image::default(),
    });

    Some(())
}
