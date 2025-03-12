use self::framebuffer::FrameBuffer;
use anyhow::anyhow;
use gl_context_slint::GLContextSlint;
use gstreamer::message::NeedContext;
use gstreamer::prelude::*;
use gstreamer::{BusSyncReply, Context, Element, MessageView, Object, Pipeline, State};
use gstreamer_gl::prelude::*;
use gstreamer_gl::GLContext;
use slint::{ComponentHandle, GraphicsAPI, Image, Weak};
use std::path::Path;
use std::sync::{Arc, Mutex};

mod framebuffer;
mod gl_context_slint;
mod pipeline;

pub struct VideoLoader {
    gl_ctx: GLContext,
    request_redraw: Arc<dyn Fn() + Send + Sync + 'static>,
}

impl Drop for VideoLoader {
    fn drop(&mut self) {
        let _ = self.gl_ctx.activate(false);
    }
}

impl VideoLoader {
    pub fn new<TApp: ComponentHandle + 'static>(
        app_weak: Weak<TApp>,
        api: &GraphicsAPI,
    ) -> anyhow::Result<Self> {
        let gl_ctx = GLContext::from_slint_graphics_api(api)?;
        let request_redraw = Arc::new(move || {
            let _ = app_weak.upgrade_in_event_loop(|app| {
                app.window().request_redraw();
            });
        });

        Ok(Self {
            gl_ctx,
            request_redraw,
        })
    }

    pub fn load(&self, path: &Path) -> anyhow::Result<Video> {
        Video::new(path, &self.gl_ctx, self.request_redraw.clone())
    }
}

pub struct Video {
    pipeline: Pipeline,
    fb: Arc<Mutex<FrameBuffer>>,
    request_redraw: Arc<dyn Fn() + Send + Sync + 'static>,
}

impl Drop for Video {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(State::Null);
        self.request_redraw();
    }
}

impl Video {
    fn new(
        path: &Path,
        gl_ctx: &GLContext,
        request_redraw: Arc<dyn Fn() + Send + Sync + 'static>,
    ) -> anyhow::Result<Self> {
        let fb = FrameBuffer::new(gl_ctx.clone());
        let fb = Arc::new(Mutex::new(fb));

        let handle_new_frame = {
            let fb = fb.clone();
            let request_redraw = request_redraw.clone();
            move |buffer, info| {
                fb.lock().unwrap().set_next_frame_data(buffer, info);
                request_redraw();
            }
        };

        let pipeline = pipeline::create(path, handle_new_frame)?;

        let bus = pipeline.bus().ok_or_else(|| anyhow!("No pipline bus"))?;
        bus.set_sync_handler({
            let gl_ctx = gl_ctx.clone();
            move |_bus, msg| match msg.view() {
                MessageView::NeedContext(nc) => provide_ctx(nc, msg.src(), &gl_ctx),
                _ => BusSyncReply::Drop,
            }
        });

        pipeline.set_state(State::Ready)?;

        Ok(Self {
            pipeline,
            fb,
            request_redraw,
        })
    }

    fn request_redraw(&self) {
        (self.request_redraw)();
    }

    pub fn current_frame_gl_ref(&self) -> Option<Image> {
        let mut fb = self.fb.lock().unwrap();
        fb.fetch_next_frame_data();
        fb.current_frame_ref()
    }

    pub fn current_frame_copy(&self) -> Option<Image> {
        self.fb.lock().unwrap().current_frame_copy()
    }

    pub fn is_playing(&self) -> bool {
        self.pipeline.state(None).1 == State::Playing
    }

    pub fn set_playing(&self, playing: bool) -> anyhow::Result<()> {
        let state = if playing {
            State::Playing
        } else {
            State::Paused
        };
        self.pipeline.set_state(state)?;
        Ok(())
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
