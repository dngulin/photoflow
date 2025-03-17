use self::framebuffer::FrameBuffer;
use anyhow::anyhow;
use gl_context_slint::GLContextSlint;
use gstreamer::prelude::*;
use gstreamer::{ClockTime, Pipeline, SeekFlags, State};
use gstreamer_gl::prelude::*;
use gstreamer_gl::GLContext;
use slint::{ComponentHandle, GraphicsAPI, Image, Weak};
use std::path::Path;
use std::sync::{Arc, Mutex};

mod bus_msg_handler;
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
    seek_state: Arc<Mutex<SeekState>>,
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
        let seek_state = Arc::new(Mutex::new(SeekState::default()));

        let bus = pipeline.bus().ok_or_else(|| anyhow!("No pipline bus"))?;
        bus.set_sync_handler({
            let gl_ctx = gl_ctx.clone();
            let pipeline = pipeline.downgrade();
            let seek_state = seek_state.clone();
            move |_bus, msg| bus_msg_handler::invoke(msg, &gl_ctx, &pipeline, &seek_state)
        });

        pipeline.set_state(State::Ready)?;

        Ok(Self {
            pipeline,
            fb,
            seek_state,
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
        self.seek_state.lock().unwrap().reset();

        let state = if playing {
            State::Playing
        } else {
            State::Paused
        };
        self.pipeline.set_state(state)?;
        Ok(())
    }

    pub fn pos_and_duration_ms(&self) -> Option<(u64, u64)> {
        let pos = self.pipeline.query_position::<ClockTime>()?.mseconds();
        let dur = self.pipeline.query_duration::<ClockTime>()?.mseconds();
        Some((pos, dur))
    }

    pub fn seek_target(&self) -> Option<f32> {
        let seek_state = self.seek_state.lock().unwrap();
        seek_state.pending.or(seek_state.current)
    }

    pub fn seek(&self, progress: f32, now: bool) -> Option<()> {
        let mut seek_state = self.seek_state.lock().unwrap();

        if now {
            seek_state.reset();
        }

        if seek_state.current.is_some() {
            seek_state.pending = Some(progress);
            return Some(());
        }

        let dur = self.pipeline.query_duration::<ClockTime>()?.seconds_f32();

        let flags = SeekFlags::FLUSH | SeekFlags::ACCURATE;
        let pos = ClockTime::from_seconds_f32((dur * progress).clamp(0.0, dur));

        if self.pipeline.seek_simple(flags, pos).is_ok() {
            seek_state.current = Some(progress);
            seek_state.pending = None;
        }

        Some(())
    }
}

#[derive(Default)]
struct SeekState {
    pub current: Option<f32>,
    pub pending: Option<f32>,
}

impl SeekState {
    fn reset(&mut self) {
        self.pending = None;
        self.current = None;
    }
}
