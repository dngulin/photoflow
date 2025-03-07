use gstreamer::Buffer;
use gstreamer_gl::gl_video_frame::Readable;
use gstreamer_gl::{GLContext, GLSyncMeta, GLVideoFrame, GLVideoFrameExt};
use gstreamer_video::{VideoFrameExt, VideoInfo};
use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::num::NonZeroU32;

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

    pub fn fetch_next_frame_data(&mut self) -> Option<()> {
        let data = self.next_frame_data.take()?;

        data.buffer.meta::<GLSyncMeta>()?.wait(&self.gl_ctx);
        let frame = GLVideoFrame::from_buffer_readable(data.buffer, &data.video_info).ok()?;
        self.current_frame = Some(frame);

        Some(())
    }

    pub fn current_frame_ref(&self) -> Option<slint::Image> {
        let frame = self.current_frame.as_ref()?;
        let tex_id = frame.texture_id(0).ok()?;

        let tex = unsafe {
            slint::BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(
                NonZeroU32::try_from(tex_id).ok()?,
                (frame.width(), frame.height()).into(),
            )
            .build()
        };

        Some(tex)
    }

    pub fn current_frame_copy(&self) -> Option<slint::Image> {
        let frame = self.current_frame.as_ref()?;
        let map = frame.buffer().map_readable().ok()?;

        let pb = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
            map.as_slice(),
            frame.width(),
            frame.height(),
        );

        Some(slint::Image::from_rgba8(pb))
    }
}
