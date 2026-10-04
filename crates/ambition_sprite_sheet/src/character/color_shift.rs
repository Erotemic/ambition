//! A character drawn in other colours, by the renderer: a variant, a buff,
//! a team.
//!
//! Jon, 2026-10-04: an HSV colour shift the engine applies at near zero cost
//! is how many games tell an enemy variant or a buffed enemy apart, so one
//! sheet serves every variant. The shift is applied where a part-drawn body
//! is drawn from its impostor cell (`ambition_render`'s
//! `impostor_unpremultiply.wgsl`): per body, after its parts are composited,
//! in the art's own (sRGB) space.
//!
//! ⚠ ONLY THE PART ROAD shifts colour today. A body drawn from its baked sheet
//! (a flipbook verdict of `Baked`, or rigged sprites not admitted) is drawn by
//! the stock sprite material, which can tint but not turn a hue: it keeps its
//! own colours.

use bevy::prelude::*;

/// Shift a character's colours: hue turned by `hue_degrees`, saturation and
/// value scaled. [`CharacterColorShift::NONE`] (the default) leaves it as
/// drawn.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct CharacterColorShift {
    /// Degrees around the colour wheel, positive towards green from red.
    pub hue_degrees: f32,
    /// Saturation multiplier (0 is grey).
    pub saturation: f32,
    /// Value (brightness) multiplier.
    pub value: f32,
}

impl CharacterColorShift {
    /// No shift.
    pub const NONE: Self = Self {
        hue_degrees: 0.0,
        saturation: 1.0,
        value: 1.0,
    };

    /// A hue turn alone.
    pub fn hue(degrees: f32) -> Self {
        Self {
            hue_degrees: degrees,
            ..Self::NONE
        }
    }

    /// The shift as the shader reads it: (hue in turns, saturation, value, 0).
    pub fn as_uniform(&self) -> Vec4 {
        Vec4::new(self.hue_degrees.rem_euclid(360.0) / 360.0, self.saturation.max(0.0), self.value.max(0.0), 0.0)
    }
}

impl Default for CharacterColorShift {
    fn default() -> Self {
        Self::NONE
    }
}
