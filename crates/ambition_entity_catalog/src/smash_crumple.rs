//! Authored payload for a crumple applied by a damaging hit.
//! The engine sags a grounded struck body and then lays it prone
//! (`BodyCombat::crumple_timer`); the floor game is its escape.

use serde::{Deserialize, Serialize};

use crate::ParamValue;

/// The authored on-hit effect key.
pub const CRUMPLE: &str = "smash.crumple";

/// The longest crumple a move may author, in seconds. A crumpling body
/// cannot act and has no mash (the floor game after it is the escape), so the
/// sag is short.
pub const CRUMPLE_MAX_S: f32 = 1.5;

/// Authored parameters of one crumple.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrumpleParams {
    /// How long the struck body sags before it collapses, in seconds.
    pub duration_s: f32,
}

/// Admission's check of a crumple's params: they hydrate, and the time is
/// seconds above 0 and at most [`CRUMPLE_MAX_S`].
pub fn check_crumple_params(params: &ParamValue) -> Result<(), String> {
    let typed: CrumpleParams = params.hydrate().map_err(|error| error.to_string())?;
    if typed.duration_s.is_finite() && typed.duration_s > 0.0 && typed.duration_s <= CRUMPLE_MAX_S {
        Ok(())
    } else {
        Err(format!(
            "`duration_s` is {}; a crumple is seconds, more than 0 and at most {CRUMPLE_MAX_S}",
            typed.duration_s
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_refuses_a_crumple_of_no_time_or_too_long() {
        let check = |duration_s: f32| check_crumple_params(&ParamValue::from_typed(&CrumpleParams { duration_s }).unwrap());
        assert_eq!(check(0.5), Ok(()));
        assert!(check(0.0).is_err());
        assert!(check(f32::NAN).is_err());
        assert!(check(CRUMPLE_MAX_S + 0.1).is_err());
    }
}
