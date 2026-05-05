//! Swim mechanic: post-`sandbox_update` buoyancy + swim-controls layer.
//!
//! Reads `FeatureRuntime::water_volumes` (built from
//! `RoomObjectKind::WaterVolume`) and adjusts the player's velocity
//! and gravity contribution while they're submerged. Always slows
//! the player down (so an un-upgraded player splashes through water
//! sluggishly); the active swim impulse only fires when the
//! `swim` ability flag is on.
//!
//! Like ledge grab, this is intentionally a separate sandbox system
//! layered on top of `movement.rs` rather than weaving the new
//! mechanic into the dense simulator.

use ambition_engine::AabbExt;
use bevy::prelude::*;

pub fn update_swim(
    mut runtime: ResMut<crate::SandboxRuntime>,
    controls: Res<crate::input::ControlFrame>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    let player_aabb = runtime.player.aabb();

    let Some(volume) = runtime
        .features
        .water_volumes
        .iter()
        .find(|v| v.aabb.strict_intersects(player_aabb))
        .cloned()
    else {
        return;
    };

    // Buoyancy drag: linear damping per tick. Always applies.
    let drag = volume.spec.drag.clamp(0.0, 1.0);
    runtime.player.vel.x *= 1.0 - drag;
    runtime.player.vel.y *= 1.0 - drag;
    // Cap fall speed.
    if runtime.player.vel.y > volume.spec.max_fall_speed {
        runtime.player.vel.y = volume.spec.max_fall_speed;
    }
    // Active swim impulse — gated on the ability flag.
    if runtime.player.abilities.swim && controls.axis_y < -0.4 {
        runtime.player.vel.y =
            runtime.player.vel.y.min(0.0) - volume.spec.swim_up_impulse * dt;
    }
}
