//! A measurement knob: widen every `Sighted` body's viewport at fixed population.
//!
//! `bounded-perception-and-attention.md` finds `kept` saturating at about 14.4
//! from 65 to 130 bodies: the hall's geometry already bounds attention. An
//! attention budget matters at high density, where `kept` keeps rising.
//! [`crate::population_cap`] sweeps population; this sweeps the other axis.
//!
//! A widened run is not the shipped hall. Vary one axis at a time: population
//! and extent both move `kept`. The sweep script pins one and records which.
//!
//! Not a gameplay feature. Read from the environment once; absent costs
//! nothing. Do not make it a `UserSettings` knob; a per-body perception
//! override is a separate game capability.

use ambition_characters::perception::PerceptionExtentOverride;

/// Override the half-extent of every `Sighted` viewport, as `WIDTHxHEIGHT` or a
/// single number applied to both axes. Unset means the shipped default.
pub const PERCEPTION_EXTENT_ENV: &str = "AMBITION_PERCEPTION_VIEWPORT_HALF";

/// The value the environment asks for, read once at plugin build and published
/// as a resource the sim reads; nothing in the simulation names this crate.
///
/// Like `population_cap::from_env` and `brain_override::from_env`, this keeps
/// environment reads out of the actor kernel (D33).
///
/// An unparsable value gives no override, not a panic, but it is logged so a
/// curve is not taken under a silently ignored knob.
pub fn from_env() -> PerceptionExtentOverride {
    let Ok(raw) = std::env::var(PERCEPTION_EXTENT_ENV) else {
        return PerceptionExtentOverride::NONE;
    };
    match parse(raw.trim()) {
        Some(half) => PerceptionExtentOverride(Some(half)),
        None => {
            eprintln!(
                "[perception-extent] {PERCEPTION_EXTENT_ENV}={raw:?} is not \
                 `WIDTHxHEIGHT` or a single number; running with the shipped \
                 viewport"
            );
            PerceptionExtentOverride::NONE
        }
    }
}

/// `"960x640"` → both axes; `"960"` → a square half-extent.
///
/// A non-positive extent is refused, not clamped. Zero gives every body an
/// empty viewport and `kept` of zero, which looks like a working budget.
fn parse(raw: &str) -> Option<ambition_platformer2d_core::Vec2> {
    let (w, h) = match raw.split_once(['x', 'X']) {
        Some((w, h)) => (w.trim().parse::<f32>().ok()?, h.trim().parse::<f32>().ok()?),
        None => {
            let both = raw.parse::<f32>().ok()?;
            (both, both)
        }
    };
    if !(w.is_finite() && h.is_finite()) || w <= 0.0 || h <= 0.0 {
        return None;
    }
    Some(ambition_platformer2d_core::Vec2::new(w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pair_sets_both_axes_and_a_single_number_is_square() {
        assert_eq!(
            parse("960x640"),
            Some(ambition_platformer2d_core::Vec2::new(960.0, 640.0))
        );
        assert_eq!(
            parse("960"),
            Some(ambition_platformer2d_core::Vec2::new(960.0, 960.0))
        );
        assert_eq!(
            parse(" 960 x 640 "),
            Some(ambition_platformer2d_core::Vec2::new(960.0, 640.0))
        );
    }

    /// Values that would measure something false.
    #[test]
    fn a_degenerate_extent_is_refused_rather_than_clamped() {
        for raw in [
            "0", "0x640", "-480x320", "nan", "inf", "", "480x", "x320", "wide",
        ] {
            assert_eq!(
                parse(raw),
                None,
                "{raw:?} must not become a viewport: a zero or negative extent \
                 gives every body an empty view and a `kept` of zero, which \
                 reads like a working budget rather than a broken knob"
            );
        }
    }

    /// The knob is inert unless set, and absence means the shipped default.
    #[test]
    fn an_unset_environment_publishes_no_override() {
        assert_eq!(PerceptionExtentOverride::NONE.half_extent(), None);
        let shipped = ambition_platformer2d_core::Vec2::new(480.0, 320.0);
        assert_eq!(PerceptionExtentOverride::NONE.or_default(shipped), shipped);
        assert_eq!(
            PerceptionExtentOverride(Some(ambition_platformer2d_core::Vec2::new(1.0, 2.0)))
                .or_default(shipped),
            ambition_platformer2d_core::Vec2::new(1.0, 2.0)
        );
    }
}
