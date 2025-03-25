use super::pipeline_ext::PipelineStd;
use super::SeekRequestBuffer;
use gstreamer::glib::WeakRef;
use gstreamer::message::NeedContext;
use gstreamer::prelude::*;
use gstreamer::{
    glib, BusSyncReply, Context, Element, Message, MessageView, Object, Pipeline, State,
};
use gstreamer_gl::prelude::*;
use gstreamer_gl::GLContext;
use std::sync::{Arc, Condvar, Mutex};

#[derive(Default)]
pub struct LoadingWaiter {
    pub cond_var: Condvar,
    pub result: Mutex<Option<Result<(), glib::Error>>>,
}

impl LoadingWaiter {
    pub fn set_result(&self, value: Result<(), glib::Error>) {
        let mut result = self.result.lock().unwrap();
        if result.is_none() {
            *result = Some(value);
            self.cond_var.notify_one();
        }
    }

    pub fn wait(&self) -> Result<(), glib::Error> {
        let mut result = self.result.lock().unwrap();
        while result.is_none() {
            result = self.cond_var.wait(result).unwrap();
        }
        result.take().unwrap()
    }
}

pub fn loading_handler(
    msg: &Message,
    gl_ctx: &GLContext,
    waiter: &Arc<LoadingWaiter>,
) -> BusSyncReply {
    match msg.view() {
        MessageView::NeedContext(nc) => provide_ctx(nc, msg.src(), gl_ctx),
        MessageView::AsyncDone(..) => waiter.set_result(Ok(())),
        MessageView::Error(err) => waiter.set_result(Err(err.error())),
        _ => {}
    }
    BusSyncReply::Drop
}

pub fn running_handler(
    msg: &Message,
    gl_ctx: &GLContext,
    pipeline: &WeakRef<Pipeline>,
    seek_state: &Arc<Mutex<SeekRequestBuffer>>,
) -> BusSyncReply {
    match msg.view() {
        MessageView::NeedContext(nc) => provide_ctx(nc, msg.src(), gl_ctx),
        _ => send_to_slint_event_loop(msg, pipeline, seek_state),
    }
    BusSyncReply::Drop
}

fn provide_ctx(msg: &NeedContext, src: Option<&Object>, gl_ctx: &GLContext) {
    if let Some(e) = src.and_then(|s| s.downcast_ref::<Element>()) {
        match msg.context_type() {
            GST_GL_DISPLAY => e.set_context(&dsp_ctx(gl_ctx)),
            GST_GL_APP_CTX => e.set_context(&app_ctx(gl_ctx)),
            _ => {}
        }
    }
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
) {
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
    if let Err(e) = slint::invoke_from_event_loop(callback) {
        log::error!("Failed to pass gst event to the Slint event loop: {}", e);
    }
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
        pipeline.std_seek(progress).ok()?;
    }

    Some(())
}
