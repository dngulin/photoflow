use gstreamer::prelude::*;
use gstreamer::{ClockTime, Pipeline, SeekFlags, State};
use std::ops::Deref;

pub trait PipelineExt {
    fn progress(&self) -> anyhow::Result<f32>;
    fn seek_progress(&self, progress: f32) -> anyhow::Result<()>;
}

impl PipelineExt for Pipeline {
    fn progress(&self) -> anyhow::Result<f32> {
        let (dur, pos) = query_dur_and_pos_seconds_f32(self)
            .ok_or_else(|| anyhow::anyhow!("Failed to query duration and position"))?;

        Ok(pos / dur)
    }

    fn seek_progress(&self, progress: f32) -> anyhow::Result<()> {
        let (dur, pos) = query_dur_and_pos_seconds_f32(self)
            .ok_or_else(|| anyhow::anyhow!("Failed to query duration and position"))?;

        let new_pos = (progress * dur).clamp(0.0, dur);
        let accurate = (pos - new_pos).abs() <= ACCURATE_SEEK_THRESHOLD_SECONDS;

        let flags = if accurate {
            SeekFlags::FLUSH | SeekFlags::ACCURATE
        } else {
            SeekFlags::FLUSH | SeekFlags::KEY_UNIT | SeekFlags::SNAP_NEAREST
        };

        self.seek_simple(flags, ClockTime::from_seconds_f32(new_pos))?;
        Ok(())
    }
}

const ACCURATE_SEEK_THRESHOLD_SECONDS: f32 = 10.0;

fn query_dur_and_pos_seconds_f32(pipeline: &Pipeline) -> Option<(f32, f32)> {
    let dur = pipeline.query_duration::<ClockTime>()?.seconds_f32();
    let pos = pipeline.query_position::<ClockTime>()?.seconds_f32();
    Some((dur, pos))
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
