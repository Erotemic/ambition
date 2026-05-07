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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_world() -> ae::World {
        ae::World::new(
            "test",
            ae::Vec2::new(1000.0, 600.0),
            ae::Vec2::ZERO,
            Vec::new(),
        )
    }

    #[test]
    fn world_to_bevy_centers_world_at_origin() {
        let world = test_world();
        // World center maps to Bevy origin (with the y flip).
        let bevy = world_to_bevy(&world, ae::Vec2::new(500.0, 300.0), 1.0);
        assert!(bevy.x.abs() < 1e-3);
        assert!(bevy.y.abs() < 1e-3);
        assert_eq!(bevy.z, 1.0);
    }

    #[test]
    fn world_to_bevy_inverts_y_axis() {
        let world = test_world();
        // Top-left of the world (y=0 in engine coords) maps to top of
        // Bevy's centered camera (y=+world.size.y/2).
        let bevy = world_to_bevy(&world, ae::Vec2::new(500.0, 0.0), 0.0);
        assert!((bevy.y - 300.0).abs() < 1e-3);
        // Bottom of the world maps to negative y in Bevy.
        let bevy = world_to_bevy(&world, ae::Vec2::new(500.0, 600.0), 0.0);
        assert!((bevy.y - (-300.0)).abs() < 1e-3);
    }

    #[test]
    fn world_z_constants_layer_correctly() {
        // Standard back-to-front layering: blocks behind player, fx in front.
        assert!(WORLD_Z_BLOCK < WORLD_Z_DUMMY);
        assert!(WORLD_Z_DUMMY < WORLD_Z_PLAYER);
        assert!(WORLD_Z_PLAYER < WORLD_Z_FX);
    }

    #[test]
    fn rgba_clamps_alpha() {
        // Alpha > 1.0 clamps to 1.0; negative clamps to 0.0.
        let opaque = rgba(1.0, 0.5, 0.0, 5.0);
        let clear = rgba(1.0, 0.5, 0.0, -1.0);
        // Bevy's Color::srgba returns a Color we can re-extract from
        // via the to_srgba helper.
        let opaque_alpha = opaque.to_srgba().alpha;
        let clear_alpha = clear.to_srgba().alpha;
        assert!((opaque_alpha - 1.0).abs() < 1e-3);
        assert!(clear_alpha.abs() < 1e-3);
    }
}
