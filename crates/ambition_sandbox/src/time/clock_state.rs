//! The live sim-clock scale — the single mutable `f32` the time system
//! owns and the [`WorldTime`](crate::time::world_time::WorldTime) producer
//! reads each frame.
//!
//! Previously `time_scale` lived on the 2-field `SandboxSimState`
//! god-struct alongside `room_transition_cooldown` (genuine room state).
//! It conceptually belongs to the TIME domain — hitstop / bullet-time /
//! pause all express themselves as a scale on the sim clock — so it lives
//! here, in a time-owned resource, and the time-control pipeline
//! (`emit → apply → smooth`) is its only writer in gameplay mode.

use bevy::prelude::Resource;

/// The current sim-clock scale, smoothed toward the granted
/// [`RequestedClockScale`](crate::time::time_control::RequestedClockScale)
/// target by `smooth_sim_clock_toward_target_system`.
///
/// `1.0` is real-time; `0.0` is fully paused (a suspended frame forces
/// this); values in between are hitstop / bullet-time / dev-slowmo.
///
/// **Multiplayer caveat:** this is **global shared-world** — hitstop,
/// bullet-time, and pause affect the whole party. A future build that
/// wants per-player cognitive rates uses the per-entity
/// [`ProperTimeScale`](crate::time::time_control::ProperTimeScale) /
/// [`PlayerClock`](crate::time::world_time::ClockDomain::PlayerClock)
/// seam instead, leaving this resource shared.
#[derive(Resource, Clone, Copy, Debug)]
pub struct ClockState {
    /// `raw_dt * time_scale` is the canonical sim dt. See [`WorldTime`].
    pub time_scale: f32,
}

impl Default for ClockState {
    fn default() -> Self {
        Self { time_scale: 1.0 }
    }
}
