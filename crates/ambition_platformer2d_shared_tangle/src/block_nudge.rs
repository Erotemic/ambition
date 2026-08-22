//! A struck block flinches — the presentation half of hitting one.
//!
//! distinguishable. They also need a small animation (probably an in-code position
//! nudge up and back into place) when they are hit."*
//!
//! PRESENTATION ONLY, and that is the whole design decision. The nudge must
//! not move the collision box: a body standing on a bonked block would be lifted by
//! it, a body beside it shoved, and a rollback would have to rewind an animation.
//! The block's geometry stays authoritative and static; what moves is the drawn
//! quad. Nothing in this module is sim state and nothing here is rewound.
//!
//! keyed by block NAME, because that is the identity both halves already
//! share. `FeatureEcsWorldOverlay::removed_block_names` and the renderer's
//! `BlockVisual { block_name }` are name-keyed for the same reason — a block is
//! authored geometry, not an entity the sim owns, so the name is the only handle
//! that survives the trip.

use bevy::prelude::Message;

/// A block was struck and should flinch.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct BlockStruck {
    /// Durable geometry identity, matching `BlockVisual::geo_id`.
    pub id: ambition_platformer2d_core::GeoId,
}

impl BlockStruck {
    pub fn new(id: ambition_platformer2d_core::GeoId) -> Self {
        Self { id }
    }
}

/// How far the flinch travels, in world px, and how long it takes.
///
/// against gravity, not "up" — the renderer resolves the direction from the
/// gravity frame, so a block struck in a flipped room flinches the way that room
/// means it. Naming it `RISE` rather than `UP` is the same relativity rule the
/// engine applies to feet and jumps.
pub const NUDGE_RISE_PX: f32 = 5.0;
/// Out and back, total. Short enough to read as an impact rather than a bounce.
pub const NUDGE_SECONDS: f32 = 0.12;

/// The flinch offset at `t` seconds into the animation, as a fraction of
/// [`NUDGE_RISE_PX`]. Out fast, back slower — an impact, not a sine wave.
pub fn nudge_fraction(t: f32) -> f32 {
    if t <= 0.0 || t >= NUDGE_SECONDS {
        return 0.0;
    }
    let phase = t / NUDGE_SECONDS;
    if phase < 0.35 {
        // Out: near-linear, so the first frame already reads.
        phase / 0.35
    } else {
        // Back: eased, so it settles rather than snapping.
        let back = (phase - 0.35) / 0.65;
        1.0 - back * back
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flinch_leaves_and_returns() {
        assert_eq!(nudge_fraction(0.0), 0.0, "no offset before it starts");
        assert_eq!(nudge_fraction(NUDGE_SECONDS), 0.0, "and none after it ends");
        assert!(
            nudge_fraction(NUDGE_SECONDS * 0.35) > 0.99,
            "it reaches full rise at the turn"
        );
        // the property that matters is not the curve's shape but that it is
        // BOUNDED: an offset that overshot would push the drawn block through the
        // one above it, which on a shelf of blocks reads as the row breaking apart.
        for i in 0..=100 {
            let f = nudge_fraction(NUDGE_SECONDS * i as f32 / 100.0);
            assert!(
                (0.0..=1.0).contains(&f),
                "offset fraction stays in range: {f}"
            );
        }
    }
}
