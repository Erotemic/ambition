//! Neutral simulation-time resource for the platformer runtime.
//!
//! The runtime's generic systems (gravity integration, oscillating /
//! temporary zones, the orient-to-gravity roll) advance world-anchored state and
//! must scale with bullet-time / hitstop / pause. In the sandbox that scaled dt
//! is [`ambition_sandbox::WorldTime::sim_dt`], but the runtime crate cannot
//! depend on the sandbox. [`SimDt`] is the inversion seam: a content-free
//! resource the runtime reads, which the **host** mirrors from its own clock
//! each frame BEFORE any system that reads it.
//!
//! In the sandbox, `mirror_sim_dt`-style host system copies
//! `WorldTime.sim_dt()` into [`SimDt::dt`] right after `refresh_world_time`, so
//! the value is byte-identical to the sandbox's sim clock (pause / bullet-time
//! feel preserved). A headless host that doesn't insert it gets the
//! `Default` (0.0), which the runtime systems treat as "frozen".

use bevy::prelude::*;

/// Per-frame scaled simulation dt (seconds) the runtime's generic systems read.
///
/// Mirror of the host's sim clock (`WorldTime.sim_dt()` in the sandbox). Scales
/// with bullet-time / hitstop / pause exactly because the host writes the
/// already-scaled value. Default `0.0` so a host that never mirrors it freezes
/// runtime motion rather than running at an unscaled rate.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct SimDt {
    /// Scaled simulation dt for this frame, in seconds.
    pub dt: f32,
}

impl SimDt {
    /// The scaled simulation dt for this frame (seconds).
    #[inline]
    pub fn get(&self) -> f32 {
        self.dt
    }
}
