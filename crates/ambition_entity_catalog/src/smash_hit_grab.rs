//! Authored payload for a hit-grab: a strike that seizes the body it struck.
//! The engine holds the struck body as a grab would (`ambition.combat.seize_struck`,
//! the capture relation); the hold replaces the hit's launch and stun.

use serde::{Deserialize, Serialize};

use crate::ParamValue;

/// The authored on-hit effect key.
pub const HIT_GRAB: &str = "smash.hit_grab";

/// The farthest a hit-grab may hold its captive from the captor's position,
/// on either axis, in world px. A hold farther than a long grab's reach puts
/// the captive somewhere no throw was authored for.
pub const HIT_GRAB_MAX_HOLD: f32 = 160.0;

/// Authored parameters of one hit-grab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HitGrabParams {
    /// Where the seized body is held, in the striker's local frame (`+x` the
    /// way it faces, `+y` toward its feet), as a grab's `hold_offset`.
    pub hold_offset: (f32, f32),
}

/// Admission's check of a hit-grab's params: they hydrate, and the hold is a
/// finite point within [`HIT_GRAB_MAX_HOLD`] of the captor.
pub fn check_hit_grab_params(params: &ParamValue) -> Result<(), String> {
    let typed: HitGrabParams = params.hydrate().map_err(|error| error.to_string())?;
    let (x, y) = typed.hold_offset;
    if x.is_finite() && y.is_finite() && x.abs() <= HIT_GRAB_MAX_HOLD && y.abs() <= HIT_GRAB_MAX_HOLD {
        Ok(())
    } else {
        Err(format!("`hold_offset` is ({x}, {y}); a hold is a finite point within {HIT_GRAB_MAX_HOLD} px of the captor"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_refuses_a_hold_out_of_reach_or_not_a_number() {
        let check = |hold_offset: (f32, f32)| check_hit_grab_params(&ParamValue::from_typed(&HitGrabParams { hold_offset }).unwrap());
        assert_eq!(check((18.0, -4.0)), Ok(()));
        assert!(check((f32::NAN, 0.0)).is_err());
        assert!(check((HIT_GRAB_MAX_HOLD + 1.0, 0.0)).is_err());
    }
}
