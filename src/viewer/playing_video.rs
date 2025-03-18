use crate::video::{SeekMode, Video};
use slint::Image;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone, Default)]
pub struct PlayingVideo(Arc<Mutex<Option<Video>>>);

impl PlayingVideo {
    fn inner(&self) -> MutexGuard<Option<Video>> {
        self.0.lock().unwrap()
    }

    pub fn set(&self, video: Video) {
        *self.inner() = Some(video);
    }

    pub fn curr_video_gl_frame(&self) -> Option<Image> {
        self.inner().as_ref().and_then(|p| p.current_frame_gl_ref())
    }

    pub fn copy_current_frame_and_stop(&self) -> Option<Image> {
        let mut inner = self.inner();
        if let Some(video) = inner.as_ref() {
            let opt_frame = video.current_frame_copy();
            *inner = None;
            return opt_frame;
        }

        None
    }

    pub fn stop(&self) {
        *self.inner() = None;
    }

    pub fn toggle_play_pause(&self) {
        let inner = self.inner();
        if let Some(video) = inner.as_ref() {
            let _ = video.set_playing(!video.is_playing());
        }
    }

    pub fn video_state(&self) -> Option<VideoState> {
        let inner = self.inner();
        let video = inner.as_ref()?;

        let is_playing = video.is_playing();
        let progress = video.progress()?;

        Some(VideoState {
            is_playing,
            progress,
        })
    }

    pub fn seek_progress(&self, progress: f32) {
        let inner = self.inner();
        if let Some(video) = inner.as_ref() {
            video.seek(progress, SeekMode::Buffered);
        }
    }
}

pub struct VideoState {
    pub is_playing: bool,
    pub progress: f32,
}
