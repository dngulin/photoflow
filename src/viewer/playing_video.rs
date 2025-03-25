use crate::video::{SeekMode, Video};
use slint::Image;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
            if let Err(e) = video.set_playing(playing_state) {
                log::error!(
                    "Failed to set video playing state to {}: {}",
                    playing_state,
                    e
                );
            }
        }
    }

    pub fn state(&self) -> Option<VideoState> {
        let video = self.inner()?;

        let is_playing = video.is_playing();
        let position = video.position()?;

        Some(VideoState {
            is_playing,
            position,
        })
    }

    pub fn seek(&self, new_pos: Duration) {
        if let Some(video) = self.inner() {
            if let Err(e) = video.seek(new_pos, SeekMode::Buffered) {
                log::error!("Failed to execute seek request: {}", e);
            }
        }
    }
}

pub struct VideoState {
    pub is_playing: bool,
    pub position: Duration,
}
