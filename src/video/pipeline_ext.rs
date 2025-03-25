use gstreamer::prelude::*;
use gstreamer::{ClockTime, Pipeline, SeekFlags, State};
use std::ops::Deref;
use std::time::Duration;

/// Extension methods for Pipline that use standard library types
pub trait PipelineStd {
    fn std_duration(&self) -> Option<Duration>;
    fn std_position(&self) -> Option<Duration>;
    fn std_seek(&self, new_pos: Duration) -> anyhow::Result<()>;
}

const ACCURATE_SEEK_THRESHOLD: Duration = Duration::from_secs(10);

impl PipelineStd for Pipeline {
    fn std_duration(&self) -> Option<Duration> {
        let duration = self.query_duration::<ClockTime>()?;
        Some(Duration::from_nanos(duration.nseconds()))
    }

    fn std_position(&self) -> Option<Duration> {
        let position = self.query_position::<ClockTime>()?;
        Some(Duration::from_nanos(position.nseconds()))
    }

    fn std_seek(&self, new_pos: Duration) -> anyhow::Result<()> {
        let pos = self
            .std_position()
            .ok_or_else(|| anyhow::anyhow!("Failed to query position"))?;

        let accurate = Duration::abs_diff(pos, new_pos) < ACCURATE_SEEK_THRESHOLD;

        let flags = if accurate {
            SeekFlags::FLUSH | SeekFlags::ACCURATE
        } else {
            SeekFlags::FLUSH | SeekFlags::KEY_UNIT | SeekFlags::SNAP_NEAREST
        };

        self.seek_simple(flags, ClockTime::from_nseconds(new_pos.as_nanos() as _))?;
        Ok(())
    }
}

/// Sets pipline state to Null on Drop
pub struct PipelineOwned(Pipeline);

impl PipelineOwned {
    pub fn new(pipeline: Pipeline) -> Self {
        Self(pipeline)
    }
}

impl Deref for PipelineOwned {
    type Target = Pipeline;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for PipelineOwned {
    fn drop(&mut self) {
        if let Err(e) = self.set_state(State::Null) {
            log::error!("Failed to cleanup gstreamer pipeline: {}", e);
        }
    }
}
