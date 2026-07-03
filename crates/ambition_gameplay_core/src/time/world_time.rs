//! Sim-dt bridge from the reusable time crate into the runtime crate.
//!
//! The generic time vocabulary + producer ([`ambition_time::WorldTime`],
//! `ClockDomain`, `ProperTimeScale`, the dt accessors, and
//! `refresh_world_time`) live in the reusable `ambition_time` crate; callers
//! name them there directly. What stays sandbox-side is
//! [`mirror_sim_dt_into_runtime`] — the bridge that copies the scaled sim dt
//! into the `ambition_platformer_primitives` crate's neutral `SimDt` resource.
//! It couples two sibling crates, so it belongs to the game shell, not the
//! generic time crate.

use ambition_time::WorldTime;
use bevy::prelude::{Res, ResMut};

/// Mirror [`WorldTime::sim_dt`] into the runtime crate's neutral
/// [`ambition_platformer_primitives::time::SimDt`] resource each frame.
///
/// The platformer runtime crate is sandbox-dep-free, so its generic systems
/// (gravity integration, oscillating / temporary zones, the orient-to-gravity
/// roll) read scaled dt through `SimDt` rather than `WorldTime`. This host
/// system is the inversion seam: it copies the already-scaled sim clock so the
/// runtime's value is byte-identical to the sandbox's (pause / bullet-time feel
/// preserved). Registered immediately AFTER [`refresh_world_time`] so every
/// downstream runtime reader sees a current value.
pub fn mirror_sim_dt_into_runtime(
    world_time: Res<WorldTime>,
    mut sim_dt: ResMut<ambition_platformer_primitives::time::SimDt>,
) {
    sim_dt.dt = world_time.sim_dt();
}
