//! Sandbox-level constants and Bevy/rendering coordinate helpers.
//!
//! The coordinate transform (`world_to_bevy`) plus the z-layer / grid /
//! default-window constants moved down into `ambition_engine_core::config`
//! (foundation crate) so reusable mechanics can name them without a
//! sandbox-internal path. This module re-exports them so the many
//! `crate::config::{world_to_bevy, WORLD_Z_*, …}` callers across the
//! sandbox keep resolving unchanged, and keeps the render-only `rgba`
//! helper (which needs `bevy::Color`, not available in the foundation
//! crate) here.

use bevy::prelude::*;

pub use ambition_engine_core::config::{
    world_to_bevy, GRID_STEP, WINDOW_H, WINDOW_W, WORLD_Z_BLOCK, WORLD_Z_DUMMY, WORLD_Z_FX,
    WORLD_Z_PLAYER,
};

pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color::srgba(r, g, b, a.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

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
