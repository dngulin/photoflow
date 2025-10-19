use crate::db::IndexDb;
use crate::exif_orientation::ExifOrientation;
use crate::image_loader;
use crate::media::{Media, MediaType};
use crate::video::{Video, VideoLoader};
use slint::{ComponentHandle, Image, Rgb8Pixel, SharedPixelBuffer, Weak};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone)]
pub struct MediaLoader {
    db: Arc<Mutex<IndexDb>>,
    video_loader: Arc<Mutex<Option<VideoLoader>>>,
    requested_idx: Arc<Mutex<Option<usize>>>,
}

// Mutex lock wrappers
impl MediaLoader {
    fn db(&self) -> MutexGuard<'_, IndexDb> {
        self.db.lock().unwrap()
    }

    fn video_loader(&self) -> MutexGuard<'_, Option<VideoLoader>> {
        self.video_loader.lock().unwrap()
    }

    fn requested_idx(&self) -> MutexGuard<'_, Option<usize>> {
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

        let (path, metadata_raw) = self.db().get_path_and_metadata(idx)?;
        let app = weak_app
            .upgrade()
            .ok_or_else(|| anyhow::anyhow!("Failed to upgrade weak app"))?;
        on_start(app, &path);

        *requested_idx = Some(idx);
        rayon::spawn_fifo({
            let loader = self.clone();
            move || {
                let load_result = loader.load_inner(&path, metadata_raw);

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
    fn load_inner(&self, path: &str, metadata_raw: u64) -> anyhow::Result<MediaInner> {
        let path = Path::new(path);
        let media_type = MediaType::from_path(path).ok_or(anyhow::anyhow!("Invalid media type"))?;

        match media_type {
            MediaType::Image(img_type) => {
                let orientation = ExifOrientation::try_from(metadata_raw).unwrap_or_default();
                let img =
                    image_loader::open(path, img_type).map(|img| img.oriented(orientation))?;

                let rgb = img.into_rgb8();
                let buf = SharedPixelBuffer::<Rgb8Pixel>::clone_from_slice(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                );

                Ok(MediaInner::Image(buf))
            }
            MediaType::Video(_video_type) => {
                let video_loader = self.video_loader();
                let video_loader = video_loader
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Video loader is not initialized"))?;

                let video = video_loader.load(path)?;
                Ok(MediaInner::Video(video))
            }
        }
    }
}
