use gstreamer::glib::WeakRef;
use gstreamer::message::NeedContext;
use gstreamer::prelude::*;
use gstreamer::{BusSyncReply, Context, Element, Message, MessageView, Object, Pipeline, State};
use gstreamer_gl::prelude::*;
use gstreamer_gl::GLContext;

pub fn invoke(msg: &Message, gl_ctx: &GLContext, pipeline: &WeakRef<Pipeline>) -> BusSyncReply {
    match msg.view() {
        MessageView::NeedContext(nc) => provide_ctx(nc, msg.src(), gl_ctx),
        _ => send_to_slint_event_loop(msg, pipeline),
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

fn send_to_slint_event_loop(msg: &Message, pipeline: &WeakRef<Pipeline>) -> BusSyncReply {
    let callback = {
        let msg = msg.to_owned();
        let pipeline = pipeline.clone();
        move || {
            if let MessageView::Eos(_) = msg.view() {
                restart_pipeline(&pipeline);
            }
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
