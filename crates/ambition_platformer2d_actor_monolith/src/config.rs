//! The render-only `rgba` color helper.
//!
//! Only `rgba` remains here, because it needs `bevy:Color`, which the foundation crate can't
//! depend on.

use bevy::prelude::*;

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
