//! Sandbox-level constants and Bevy/rendering coordinate helpers.
//!
//! These are intentionally kept out of `main.rs` so the playable sandbox can be
//! retuned without digging through system scheduling or gameplay code.

use ambition_engine as ae;
use bevy::prelude::*;

/// Default logical window width for initial windowed mode.
///
/// Bevy can resize the window after startup; camera clamping should use the
/// actual `Window` dimensions, not these defaults.
pub const WINDOW_W: u32 = 1600;

/// Default logical window height for initial windowed mode.
pub const WINDOW_H: u32 = 900;

pub const WORLD_Z_BLOCK: f32 = 0.0;
pub const WORLD_Z_DUMMY: f32 = 10.0;
pub const WORLD_Z_PLAYER: f32 = 20.0;
pub const WORLD_Z_FX: f32 = 30.0;

pub const GRID_STEP: f32 = 80.0;

pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color::srgba(r, g, b, a.clamp(0.0, 1.0))
}

/// Convert Ambition Engine coordinates to Bevy's centered 2D camera space.
///
/// Ambition Engine uses a top-left origin with +Y downward because that keeps
/// collision/math code easy to reason about for platformer rooms. Bevy's 2D
/// camera is centered with +Y upward, so all rendering should go through this
/// adapter rather than duplicating coordinate math throughout the sandbox.
pub fn world_to_bevy(world: &ae::World, p: ae::Vec2, z: f32) -> Vec3 {
    Vec3::new(p.x - world.size.x * 0.5, world.size.y * 0.5 - p.y, z)
}
