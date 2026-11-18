//! Local input sources that read the body before each new input tick.

use ambition_characters::control::{PlayerSlot, SeatRawFrames, SlotControlLatches, SlotControls};
use ambition_platformer2d_core::ControlFrame;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

/// Runs outside simulation history, before the host captures local input.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LocalInputSampling;

/// The fixed host publishes captured controls into the simulation seats.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LocalInputPublication;

/// The time of a new local input sample, in simulation seconds.
#[derive(Resource, Default)]
pub struct LocalInputClock {
    pub now: f32,
    pub dt: f32,
}

/// The seats owned by input sources that sample at the tick boundary.
#[derive(Resource, Default)]
pub struct SampledLocalInputs([Option<ControlFrame>; SlotControls::MAX_SLOTS]);

impl SampledLocalInputs {
    pub fn set(&mut self, slot: PlayerSlot, frame: ControlFrame) {
        if let Some(sample) = self.0.get_mut(slot.0 as usize) {
            *sample = Some(frame);
        }
    }
}

/// Sample once for a new input tick. Historical replay does not call this.
pub fn sample_local_inputs(world: &mut World) {
    if !world.contains_resource::<SampledLocalInputs>() {
        return;
    }
    let raw_dt = world
        .get_resource::<crate::SimStepPeriod>()
        .map(|step| step.0)
        .unwrap_or_else(|| world.resource::<Time>().delta_secs());
    let scale = world
        .get_resource::<ambition_time::ClockState>()
        .map_or(1.0, |clock| clock.time_scale);
    let mut clock = world.resource_mut::<LocalInputClock>();
    clock.dt = raw_dt * scale;
    clock.now += clock.dt;
    world.run_schedule(LocalInputSampling);
}

/// Publish the sampled seats through the host's ordinary input boundary.
/// A repeated request for one tick republishes the same sample.
pub fn publish_sampled_local_inputs(world: &mut World) {
    let Some(samples) = world.get_resource::<SampledLocalInputs>() else {
        return;
    };
    let frames = samples.0;
    for (index, frame) in frames.into_iter().enumerate() {
        let Some(frame) = frame else { continue };
        let slot = PlayerSlot(index as u8);
        world.resource_mut::<SeatRawFrames>().set(slot, frame);
        if let Some(mut latches) = world.get_resource_mut::<SlotControlLatches>() {
            // This source owns the seat. Discard old render-clock samples.
            latches.take(slot);
            latches.accumulate(slot, frame);
        }
    }
}
