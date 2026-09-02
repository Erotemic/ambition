//! A measurement knob: widen every `Sighted` body's viewport at fixed population.
//!
//! ⭐ THIS EXISTS BECAUSE THE HALL CANNOT ASK THE QUESTION.
//! `bounded-perception-and-attention.md` measures `kept` saturating at ~14.4
//! from 65 to 130 bodies and concludes, in its own words, that *"the hall CANNOT
//! demonstrate why attention is needed — its geometry already solves it"*. The
//! regime an attention budget is FOR is density: fighters packed inside one
//! another's viewports, where `kept` keeps rising with population. Population is
//! swept by [`crate::population_cap`]; this is the other axis.
//!
//! ⛔⛔ IT CHANGES THE SIMULATION, AND THAT IS THE POINT — the same warning the
//! population cap carries. A widened run is not the shipped hall and no number
//! taken under it describes the shipped hall.
//!
//! ⛔ HOLD ONE AXIS AT A TIME. Population and extent both move `kept`, so a run
//! that varies both cannot attribute the result to either. The sweep script
//! pins one and varies the other, and records which.
//!
//! ⚠ NOT A GAMEPLAY FEATURE. Read from the environment exactly once, absent
//! costs nothing, and it must never become a `UserSettings` knob: a per-body
//! perception override is a legitimate GAME capability (the type already has a
//! field for one) and is a different thing from this.

use ambition_characters::perception::PerceptionExtentOverride;

/// Override the half-extent of every `Sighted` viewport, as `WIDTHxHEIGHT` or a
/// single number applied to both axes. Unset means the shipped default.
pub const PERCEPTION_EXTENT_ENV: &str = "AMBITION_PERCEPTION_VIEWPORT_HALF";

/// The value the environment asks for, read ONCE at plugin build and published
/// as a resource the sim reads; nothing in the simulation names this crate.
///
/// ⭐ THE SAME INVERSION AS `population_cap::from_env` and
/// `brain_override::from_env`. D33 removed the actor kernel's three developer
/// reads; a knob that read the environment from inside `ensure_perception`
/// would add a fourth and undo it.
///
/// ⛔ An unparsable value is NO OVERRIDE, not a panic — a measurement
/// convenience should not stop a run — but it is logged, because a density
/// curve taken under a silently ignored knob is a wrong curve, and every point
/// on it would look like the shipped default while claiming not to be.
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
/// ⛔ A NON-POSITIVE EXTENT IS REFUSED rather than clamped. Zero would give
/// every body an empty viewport and a `kept` of zero, which reads exactly like
/// a working budget of zero — a plausible measurement of nothing.
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

    /// ⛔ THE VALUES THAT WOULD MEASURE SOMETHING FALSE.
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

    /// The knob is INERT unless the environment asks, and absence is the
    /// shipped default rather than any particular number.
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
