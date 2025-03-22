use crate::video::{SeekMode, Video};
use slint::Image;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct CurrentVideo(Arc<Mutex<Option<Video>>>);

impl CurrentVideo {
    pub fn set(&self, video: Video) {
        *self.0.lock().unwrap() = Some(video);
    }

    pub fn stop(&self) {
        *self.0.lock().unwrap() = None;
    }

    fn inner(&self) -> Option<Video> {
        self.0.lock().unwrap().clone()
    }

    pub fn curr_video_gl_frame(&self) -> Option<Image> {
        self.inner().and_then(|p| p.current_frame_gl_ref())
    }

    pub fn copy_current_frame_and_stop(&self) -> Option<Image> {
        if let Some(video) = self.inner() {
            self.stop();
            return video.current_frame_copy();
        }

        None
    }

    pub fn set_playing(&self, playing_state: bool) {
        if let Some(video) = self.inner() {
            let _ = video.set_playing(playing_state);
        }
    }

    pub fn state(&self) -> Option<VideoState> {
        let video = self.inner()?;

        let is_playing = video.is_playing();
        let progress = video.progress()?;
        let position_ms = video.position_ms()?;

        Some(VideoState {
            is_playing,
            progress,
            position_ms,
        })
    }

    pub fn seek_progress(&self, progress: f32) {
        if let Some(video) = self.inner() {
            video.seek(progress, SeekMode::Buffered);
        }
    }
}

pub struct VideoState {
    pub is_playing: bool,
    pub progress: f32,
    pub position_ms: u64,
}
