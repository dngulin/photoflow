use super::pipeline_ext::PipelineExt;
use super::SeekRequestBuffer;
use gstreamer::glib::WeakRef;
use gstreamer::message::NeedContext;
use gstreamer::prelude::*;
use gstreamer::{BusSyncReply, Context, Element, Message, MessageView, Object, Pipeline, State};
use gstreamer_gl::prelude::*;
use gstreamer_gl::GLContext;
use std::sync::{Arc, Mutex};

pub fn invoke(
    msg: &Message,
    gl_ctx: &GLContext,
    pipeline: &WeakRef<Pipeline>,
    seek_state: &Arc<Mutex<SeekRequestBuffer>>,
) -> BusSyncReply {
    match msg.view() {
        MessageView::NeedContext(nc) => provide_ctx(nc, msg.src(), gl_ctx),
        _ => send_to_slint_event_loop(msg, pipeline, seek_state),
    }
}

fn provide_ctx(msg: &NeedContext, src: Option<&Object>, gl_ctx: &GLContext) -> BusSyncReply {
    if let Some(e) = src.and_then(|s| s.downcast_ref::<Element>()) {
        match msg.context_type() {
            GST_GL_DISPLAY => e.set_context(&dsp_ctx(gl_ctx)),
            GST_GL_APP_CTX => e.set_context(&app_ctx(gl_ctx)),
            _ => {}
        }
    }

    BusSyncReply::Drop
}

const GST_GL_DISPLAY: &str = "gst.gl.GLDisplay";
const GST_GL_APP_CTX: &str = "gst.gl.app_context";

fn dsp_ctx(gl_ctx: &GLContext) -> Context {
    let ctx = Context::new(GST_GL_DISPLAY, true);
    ctx.set_gl_display(&gl_ctx.display());
    ctx
}

fn app_ctx(gl_ctx: &GLContext) -> Context {
    let mut ctx = Context::new(GST_GL_APP_CTX, true);
    {
        let ctx = ctx.get_mut().unwrap();
        let structure = ctx.structure_mut();
        structure.set("context", gl_ctx);
    }
    ctx
}

fn send_to_slint_event_loop(
    msg: &Message,
    pipeline: &WeakRef<Pipeline>,
    seek_state: &Arc<Mutex<SeekRequestBuffer>>,
) -> BusSyncReply {
    let callback = {
        let msg = msg.to_owned();
        let pipeline = pipeline.clone();
        let seek_state = seek_state.clone();
        move || match msg.view() {
            MessageView::Eos(_) => {
                seek_state.lock().unwrap().reset();
                restart_pipeline(&pipeline);
            }
            MessageView::AsyncDone(_) => {
                finish_seeking(&pipeline, &seek_state);
            }
            _ => {}
        }
    };
    let _ = slint::invoke_from_event_loop(callback);
    BusSyncReply::Drop
}

fn restart_pipeline(pipeline: &WeakRef<Pipeline>) -> Option<()> {
    let pipeline = pipeline.upgrade()?;
    pipeline.set_state(State::Ready).ok()?;
    pipeline.set_state(State::Paused).ok()?;
    Some(())
}

fn finish_seeking(
    pipeline: &WeakRef<Pipeline>,
    seek_state: &Arc<Mutex<SeekRequestBuffer>>,
) -> Option<()> {
    let pipeline = pipeline.upgrade()?;

    let mut seek_state = seek_state.lock().unwrap();
    if seek_state.current.is_some() {
        seek_state.current = seek_state.pending.take();
    }

    seek_state.pending = None;

    if let Some(progress) = seek_state.current {
        pipeline.seek_std(progress).ok()?;
    }

    Some(())
}
