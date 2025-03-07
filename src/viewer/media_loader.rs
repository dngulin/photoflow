use crate::db::IndexDb;
use crate::exif_orientation::ExifOrientation;
use crate::image_loader;
use crate::video_loader::{Video, VideoLoader};
use slint::{ComponentHandle, Image, Rgb8Pixel, SharedPixelBuffer, Weak};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone)]
pub struct MediaLoader {
    db: Arc<Mutex<IndexDb>>,
    video_loader: Arc<Mutex<Option<VideoLoader>>>,
    requested_idx: Arc<Mutex<Option<usize>>>,
}

pub enum Media {
    Image(Image),
    Video(Video),
}

// Mutex lock wrappers
impl MediaLoader {
    fn db(&self) -> MutexGuard<IndexDb> {
        self.db.lock().unwrap()
    }

    fn video_loader(&self) -> MutexGuard<Option<VideoLoader>> {
        self.video_loader.lock().unwrap()
    }

    fn requested_idx(&self) -> MutexGuard<Option<usize>> {
        self.requested_idx.lock().unwrap()
    }
}

// Public API
impl MediaLoader {
    pub fn new(db: Arc<Mutex<IndexDb>>, video_loader: Arc<Mutex<Option<VideoLoader>>>) -> Self {
        Self {
            db,
            video_loader,
            requested_idx: Default::default(),
        }
    }

    pub fn cancel_loading(&self) {
        *self.requested_idx() = None;
    }

    pub fn load<TApp: ComponentHandle + 'static>(
        &self,
        idx: usize,
        weak_app: Weak<TApp>,
        on_start: impl FnOnce(TApp, &str),
        on_finish: impl FnOnce(TApp, anyhow::Result<Media>) + Send + 'static,
    ) -> anyhow::Result<()> {
        let mut requested_idx = self.requested_idx();

        if requested_idx.is_some() {
            return Err(anyhow::anyhow!("Loading is already in progress"));
        }

        let (path, orientation) = self.db().get_path_and_orientation(idx)?;
        let app = weak_app
            .upgrade()
            .ok_or_else(|| anyhow::anyhow!("Failed to upgrade weak app"))?;
        on_start(app, &path);

        *requested_idx = Some(idx);
        rayon::spawn_fifo({
            let loader = self.clone();
            move || {
                let orientation: ExifOrientation = orientation.try_into().unwrap_or_default();
                let load_result = loader.load_inner(&path, orientation);

                let _ = weak_app.upgrade_in_event_loop(move |app| {
                    let mut requested_idx = loader.requested_idx();
                    if *requested_idx != Some(idx) {
                        return;
                    }
                    *requested_idx = None;

                    let load_result = load_result.map(|m| match m {
                        MediaInner::Image(i) => Media::Image(Image::from_rgb8(i)),
                        MediaInner::Video(v) => Media::Video(v),
                    });

                    on_finish(app, load_result);
                });
            }
        });

        Ok(())
    }
}

// Media representation that can be sent between threads
pub enum MediaInner {
    Image(SharedPixelBuffer<Rgb8Pixel>),
    Video(Video),
}

impl MediaLoader {
    fn load_inner(&self, path: &str, orientation: ExifOrientation) -> anyhow::Result<MediaInner> {
        let path = Path::new(path);

        if image_loader::is_extension_supported(path) {
            let image = image_loader::open(path).map(|img| img.oriented(orientation))?;
            let rgb = image.into_rgb8();

            let buffer = SharedPixelBuffer::<Rgb8Pixel>::clone_from_slice(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
            );

            return Ok(MediaInner::Image(buffer));
        }

        let mut video_loader = self.video_loader();
        let video_loader = video_loader
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Video loader is not initialized"))?;

        video_loader.load(path).map(MediaInner::Video)
    }
}
