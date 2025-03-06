use gstreamer::Buffer;
use gstreamer_gl::gl_video_frame::Readable;
use gstreamer_gl::{GLContext, GLSyncMeta, GLVideoFrame, GLVideoFrameExt};
use gstreamer_video::{VideoFrameExt, VideoInfo};

struct FrameData {
    pub buffer: Buffer,
    pub video_info: VideoInfo,
}

impl FrameData {
    pub fn new(video_info: VideoInfo, buffer: Buffer) -> Self {
        Self { video_info, buffer }
    }
}

pub struct FrameBuffer {
    gl_ctx: GLContext,
    next_frame_data: Option<FrameData>,
    current_frame: Option<GLVideoFrame<Readable>>,
}

impl FrameBuffer {
    pub fn new(gl_ctx: GLContext) -> Self {
        Self {
            gl_ctx,
            next_frame_data: Default::default(),
            current_frame: Default::default(),
        }
    }

    pub fn set_next_frame_data(&mut self, buffer: Buffer, video_info: VideoInfo) {
        self.next_frame_data = Some(FrameData::new(video_info, buffer));
    }

    pub fn fetch_current_frame(&mut self) -> Option<slint::Image> {
        if let Some(data) = self.next_frame_data.take() {
            data.buffer.meta::<GLSyncMeta>()?.wait(&self.gl_ctx);
            if let Ok(frame) = GLVideoFrame::from_buffer_readable(data.buffer, &data.video_info) {
                self.current_frame = Some(frame);
            }
        }

        self.current_frame
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
}
