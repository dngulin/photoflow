use self::bus_msg_handler::{loading_handler, running_handler};
use self::framebuffer::FrameBuffer;
use self::pipeline_ext::{PipelineOwned, PipelineStd};
use crate::video::bus_msg_handler::LoadingWaiter;
use anyhow::anyhow;
use gl_context_slint::GLContextSlint;
use gstreamer::{Bus, Pipeline, State, StateChangeSuccess};
use gstreamer_gl::prelude::*;
use gstreamer_gl::GLContext;
use slint::{ComponentHandle, GraphicsAPI, Image, Weak};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod bus_msg_handler;
mod framebuffer;
mod gl_context_slint;
mod pipeline;
mod pipeline_ext;

pub struct VideoLoader {
    gl_ctx: GLContext,
    request_redraw: Arc<dyn Fn() + Send + Sync + 'static>,
}

impl Drop for VideoLoader {
    fn drop(&mut self) {
        if let Err(e) = self.gl_ctx.activate(false) {
            log::error!("Failed to deactivate OpenGL context: {}", e);
        }
    }
}

impl VideoLoader {
    pub fn new<TApp: ComponentHandle + 'static>(
        app_weak: Weak<TApp>,
        api: &GraphicsAPI,
    ) -> anyhow::Result<Self> {
        let gl_ctx = GLContext::from_slint_graphics_api(api)?;
        let request_redraw = Arc::new(move || {
            if let Err(e) = app_weak.upgrade_in_event_loop(move |app| {
                app.window().request_redraw();
            }) {
                log::error!("Failed to request window redraw: {}", e);
            };
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

#[derive(Clone)]
pub struct Video {
    pipeline: Arc<PipelineOwned>,
    fb: Arc<Mutex<FrameBuffer>>,
    seek_state: Arc<Mutex<SeekRequestBuffer>>,
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
        let pipeline = Arc::new(PipelineOwned::new(pipeline));
        let bus = pipeline.bus().ok_or_else(|| anyhow!("No pipline bus"))?;

        wait_pipeline_paused(&pipeline, &bus, gl_ctx)?;

        let seek_state = Arc::new(Mutex::new(SeekRequestBuffer::default()));
        bus.set_sync_handler({
            let gl_ctx = gl_ctx.clone();
            let pipeline = pipeline.downgrade();
            let seek_state = seek_state.clone();
            move |_bus, msg| running_handler(msg, &gl_ctx, &pipeline, &seek_state)
        });

        Ok(Self {
            pipeline,
            fb,
            seek_state,
        })
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
        self.seek_state.lock().unwrap().reset();

        let state = if playing {
            State::Playing
        } else {
            State::Paused
        };
        self.pipeline.set_state(state)?;
        Ok(())
    }

    pub fn position(&self) -> Option<Duration> {
        self.pipeline.std_position()
    }

    pub fn duration(&self) -> Option<Duration> {
        self.pipeline.std_duration()
    }

    pub fn seek(&self, new_pos: Duration, mode: SeekMode) -> anyhow::Result<()> {
        let mut seek_state = self.seek_state.lock().unwrap();

        if mode == SeekMode::Instant {
            seek_state.reset();
        }

        if seek_state.current.is_some() {
            seek_state.pending = Some(new_pos);
            return Ok(());
        }

        self.pipeline.std_seek(new_pos)?;
        seek_state.current = Some(new_pos);
        seek_state.pending = None;

        Ok(())
    }
}

fn wait_pipeline_paused(pipeline: &Pipeline, bus: &Bus, gl_ctx: &GLContext) -> anyhow::Result<()> {
    let loading_waiter = Arc::new(LoadingWaiter::default());
    bus.set_sync_handler({
        let gl_ctx = gl_ctx.clone();
        let loading_waiter = loading_waiter.clone();
        move |_bus, msg| loading_handler(msg, &gl_ctx, &loading_waiter)
    });

    let change = pipeline.set_state(State::Paused)?;
    if change == StateChangeSuccess::Async {
        loading_waiter.wait()?;
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SeekMode {
    /// Cancels the active or pending seek requests and starts a new one
    Instant,
    /// Postpones the request if there is an active seek in progress.
    /// Helps to amortize frequent seek requests (e.g. from the progress slider)
    /// and produce more intermediate frames
    Buffered,
}

#[derive(Default)]
struct SeekRequestBuffer {
    pub current: Option<Duration>,
    pub pending: Option<Duration>,
}

impl SeekRequestBuffer {
    fn reset(&mut self) {
        self.pending = None;
        self.current = None;
    }
}
