//! Coordinate transforms and layer/grid constants.
//!
//! These are the engine-coordinate ↔ Bevy-coordinate adapter plus the
//! shared z-layer / grid / default-window constants. They depend only on
//! `bevy_math` + [`crate::World`], so they live in the foundation crate:
//! reusable mechanics (portal, gravity, …) and the sandbox both go
//! through `world_to_bevy` rather than duplicating coordinate math.
//!
//! Render-only helpers that need `bevy_color`/`bevy_render` (e.g. an
//! `rgba` `Color` constructor) intentionally stay in the consuming
//! crate; this module is render-feature-free.

use crate::World;
use bevy_math::{Vec2, Vec3};

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

/// Convert Ambition Engine coordinates to Bevy's centered 2D camera space.
///
/// Ambition Engine uses a top-left origin with +Y downward because that keeps
/// collision/math code easy to reason about for platformer rooms. Bevy's 2D
/// camera is centered with +Y upward, so all rendering should go through this
/// adapter rather than duplicating coordinate math throughout the sandbox.
pub fn world_to_bevy(world: &World, p: Vec2, z: f32) -> Vec3 {
    world_size_to_bevy(world.size, p, z)
}

/// [`world_to_bevy`] from just the world's size — the only field the transform
/// reads. Render crates below the host (e.g. portal presentation) hold a copied
/// size instead of the whole `World`; the centering + y-flip math stays defined
/// here, once.
pub fn world_size_to_bevy(size: Vec2, p: Vec2, z: f32) -> Vec3 {
    Vec3::new(p.x - size.x * 0.5, size.y * 0.5 - p.y, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_world() -> World {
        World::new("test", Vec2::new(1000.0, 600.0), Vec2::ZERO, Vec::new())
    }

    #[test]
    fn world_to_bevy_centers_world_at_origin() {
        let world = test_world();
        // World center maps to Bevy origin (with the y flip).
        let bevy = world_to_bevy(&world, Vec2::new(500.0, 300.0), 1.0);
        assert!(bevy.x.abs() < 1e-3);
        assert!(bevy.y.abs() < 1e-3);
        assert_eq!(bevy.z, 1.0);
    }

    #[test]
    fn world_to_bevy_inverts_y_axis() {
        let world = test_world();
        // Top-left of the world (y=0 in engine coords) maps to top of
        // Bevy's centered camera (y=+world.size.y/2).
        let bevy = world_to_bevy(&world, Vec2::new(500.0, 0.0), 0.0);
        assert!((bevy.y - 300.0).abs() < 1e-3);
        // Bottom of the world maps to negative y in Bevy.
        let bevy = world_to_bevy(&world, Vec2::new(500.0, 600.0), 0.0);
        assert!((bevy.y - (-300.0)).abs() < 1e-3);
    }

    #[test]
    fn world_z_constants_layer_correctly() {
        // Standard back-to-front layering: blocks behind player, fx in front.
        assert!(WORLD_Z_BLOCK < WORLD_Z_DUMMY);
        assert!(WORLD_Z_DUMMY < WORLD_Z_PLAYER);
        assert!(WORLD_Z_PLAYER < WORLD_Z_FX);
    }
}
