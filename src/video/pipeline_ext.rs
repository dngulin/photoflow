use gstreamer::prelude::*;
use gstreamer::{ClockTime, Pipeline, SeekFlags, State};
use std::ops::Deref;
use std::time::Duration;

pub trait PipelineExt {
    fn duration_std(&self) -> Option<Duration>;
    fn position_std(&self) -> Option<Duration>;
    fn seek_std(&self, new_pos: Duration) -> anyhow::Result<()>;
}

const ACCURATE_SEEK_THRESHOLD: Duration = Duration::from_secs(10);

impl PipelineExt for Pipeline {
    fn duration_std(&self) -> Option<Duration> {
        let duration = self.query_duration::<ClockTime>()?;
        Some(Duration::from_nanos(duration.nseconds()))
    }

    fn position_std(&self) -> Option<Duration> {
        let position = self.query_position::<ClockTime>()?;
        Some(Duration::from_nanos(position.nseconds()))
    }

    fn seek_std(&self, new_pos: Duration) -> anyhow::Result<()> {
        let pos = self
            .position_std()
            .ok_or_else(|| anyhow::anyhow!("Failed to query position"))?;

        let accurate = Duration::abs_diff(pos, new_pos) < Duration::from_secs(10);

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
        let _ = self.set_state(State::Null);
    }
}
