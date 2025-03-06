use crate::viewer::player::framebuffer::FrameBuffer;
use crate::viewer::player::gst_bus_gl_sync::BusGlSync;
use anyhow::anyhow;
use gl_context_slint::GLContextSlint;
use gstreamer::prelude::*;
use gstreamer::{ClockTime, Pipeline, State};
use gstreamer_gl::prelude::*;
use gstreamer_gl::GLContext;
use slint::{GraphicsAPI, Image};
use std::path::Path;
use std::sync::{Arc, Mutex};

mod framebuffer;
mod gl_context_slint;
mod gst_bus_gl_sync;
mod pipeline;

pub struct Player {
    gl_ctx: GLContext,
    request_redraw: Arc<dyn Fn() + Send + Sync + 'static>,
    playback: Option<Playback>,
}

impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.gl_ctx.activate(false);
    }
}

impl Player {
    pub fn new<F>(api: &GraphicsAPI, request_redraw: F) -> anyhow::Result<Self>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let gl_ctx = GLContext::from_slint_graphics_api(api)?;
        Ok(Self {
            gl_ctx,
            playback: None,
            request_redraw: Arc::new(request_redraw),
        })
    }

    pub fn load(&mut self, path: &Path) -> anyhow::Result<()> {
        let playback = Playback::new(path, &self.gl_ctx, self.request_redraw.clone())?;
        playback.set_playing(true)?;
        self.playback = Some(playback);
        Ok(())
    }

    pub fn unload(&mut self) {
        self.playback = None;
    }

    pub fn playback(&self) -> Option<&Playback> {
        self.playback.as_ref()
    }
}

pub struct Playback {
    pipeline: Pipeline,
    fb: Arc<Mutex<FrameBuffer>>,
    request_redraw: Arc<dyn Fn() + Send + Sync + 'static>,
}

impl Drop for Playback {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(State::Null);
        self.request_redraw();
    }
}

impl Playback {
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
        bus.set_gl_sync_handler(gl_ctx.clone());

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
        self.fb.lock().unwrap().fetch_current_frame()
    }

    pub fn current_frame_copy(&self) -> Option<Image> {
        self.fb.lock().unwrap().dl_current_frame()
    }

    pub fn status(&self) -> bool {
        self.pipeline.state(ClockTime::NONE).1 == State::Playing
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
