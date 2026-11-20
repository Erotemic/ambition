//! Authored payload for a burial applied by a damaging hit.
//! The engine buries a grounded struck body (`BodyCombat::bury_timer`); the
//! ruleset owns how a buried body digs out.

use serde::{Deserialize, Serialize};

use crate::ParamValue;

/// The authored on-hit effect key.
pub const BURY: &str = "smash.bury";

/// The longest burial a move may author, in seconds. A buried body
/// cannot act, so a longer one is a stock lost with no counterplay but the
/// mash.
pub const BURY_MAX_S: f32 = 2.5;

/// Authored parameters of one burial.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuryParams {
    /// How long the struck body stays buried, in seconds.
    pub duration_s: f32,
}

/// Admission's check of a burial's params: they hydrate, and the time is
/// seconds above 0 and at most [`BURY_MAX_S`].
pub fn check_bury_params(params: &ParamValue) -> Result<(), String> {
    let typed: BuryParams = params.hydrate().map_err(|error| error.to_string())?;
    if typed.duration_s.is_finite() && typed.duration_s > 0.0 && typed.duration_s <= BURY_MAX_S {
        Ok(())
    } else {
        Err(format!(
            "`duration_s` is {}; a burial is seconds, more than 0 and at most {BURY_MAX_S}",
            typed.duration_s
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_refuses_a_burial_of_no_time_or_too_long() {
        let check = |duration_s: f32| check_bury_params(&ParamValue::from_typed(&BuryParams { duration_s }).unwrap());
        assert_eq!(check(0.5), Ok(()));
        assert!(check(0.0).is_err());
        assert!(check(f32::NAN).is_err());
        assert!(check(BURY_MAX_S + 0.1).is_err());
    }
}
