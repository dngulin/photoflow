use gstreamer::glib::filename_to_uri;
use gstreamer::prelude::*;
use gstreamer::{Buffer, ElementFactory, FlowError, FlowSuccess, Fraction, Pipeline};
use gstreamer_app::{AppSink, AppSinkCallbacks};
use gstreamer_gl::{GLBaseMemory, GLSyncMeta, CAPS_FEATURE_MEMORY_GL_MEMORY};
use gstreamer_video::{VideoCapsBuilder, VideoFormat, VideoInfo};
use std::path::Path;

pub fn create<F>(path: &Path, handle_new_frame: F) -> anyhow::Result<Pipeline>
where
    F: Fn(Buffer, VideoInfo) + Send + 'static,
{
    let terminator = gstreamer::parse::bin_from_description(
        "glvideoflip method=automatic ! appsink name=sink",
        true,
    )?;

    let appsink = terminator
        .by_name("sink")
        .unwrap()
        .downcast::<AppSink>()
        .unwrap();

    let caps = &VideoCapsBuilder::new()
        .features([CAPS_FEATURE_MEMORY_GL_MEMORY])
        .format(VideoFormat::Rgba)
        .field("texture-target", "2D")
        .field("pixel-aspect-ratio", Fraction::new(1, 1))
        .build();
    appsink.set_caps(Some(caps));
    appsink.set_enable_last_sample(false);
    appsink.set_max_buffers(1u32);

    let glsink = ElementFactory::make("glsinkbin")
        .property("sink", &terminator)
        .build()?;

    let uri = filename_to_uri(path, None)?;
    let pipeline = ElementFactory::make("playbin")
        .property("uri", uri)
        .property("video-sink", &glsink)
        .build()?
        .downcast::<Pipeline>()
        .unwrap();

    let callbacks = gl_frame_callbacks(handle_new_frame);
    appsink.set_callbacks(callbacks);

    Ok(pipeline)
}

fn gl_frame_callbacks<F>(handle_new_frame: F) -> AppSinkCallbacks
where
    F: Fn(Buffer, VideoInfo) + Send + 'static,
{
    AppSinkCallbacks::builder()
        .new_sample(move |appsink| {
            let sample = appsink.pull_sample().map_err(|_| FlowError::Flushing)?;

            let mut buffer = sample.buffer_owned().ok_or(FlowError::Error)?;
            set_buffer_sync_point(&mut buffer)?;

            let info = sample
                .caps()
                .and_then(|caps| VideoInfo::from_caps(caps).ok())
                .ok_or(FlowError::NotNegotiated)?;

            handle_new_frame(buffer, info);

            Ok(FlowSuccess::Ok)
        })
        .build()
}

fn set_buffer_sync_point(buffer: &mut Buffer) -> Result<(), FlowError> {
    let ctx = (buffer.n_memory() > 0)
        .then(|| buffer.peek_memory(0))
        .and_then(|m| m.downcast_memory_ref::<GLBaseMemory>())
        .map(|m| m.context().clone())
        .ok_or(FlowError::Error)?;

    // Sync point to ensure that the rendering in this context will be complete by the time the
    // Slint created GL context needs to access the texture.
    if let Some(meta) = buffer.meta::<GLSyncMeta>() {
        meta.set_sync_point(&ctx);
    } else {
        GLSyncMeta::add(buffer.make_mut(), &ctx).set_sync_point(&ctx);
    }

    Ok(())
}
