use gstreamer_gl::gl_video_frame::Readable;
use gstreamer_gl::{GLContext, GLSyncMeta, GLVideoFrame, GLVideoFrameExt};
use gstreamer_video::VideoFrameExt;
use std::sync::{Arc, Mutex};

struct FrameData {
    pub video_info: gstreamer_video::VideoInfo,
    pub buffer: gstreamer::Buffer,
}

pub struct FrameBuffer {
    next_frame_data: Arc<Mutex<Option<FrameData>>>,
    current_frame: Mutex<Option<GLVideoFrame<Readable>>>,
}

impl FrameBuffer {
    pub fn get_frame(&self, gl_ctx: &GLContext) -> Option<slint::Image> {
        if let Some(data) = self.next_frame_data.lock().unwrap().take() {
            data.buffer.meta::<GLSyncMeta>()?.wait(gl_ctx);
            if let Ok(frame) = GLVideoFrame::from_buffer_readable(data.buffer, &data.video_info) {
                *self.current_frame.lock().unwrap() = Some(frame);
            }
        }

        self.current_frame
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|frame| {
                frame
                    .texture_id(0)
                    .ok()
                    .and_then(|id| id.try_into().ok())
                    .map(|texture| (frame, texture))
            })
            .map(|(frame, texture)| unsafe {
                slint::BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(
                    texture,
                    (frame.width(), frame.height()).into(),
                )
                .build()
            })
    }

    pub fn clear(&self) {
        self.next_frame_data.lock().unwrap().take();
        self.current_frame.lock().unwrap().take();
    }
}
