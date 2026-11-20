//! Authored payload for a paralysis applied by a damaging hit.
//! The engine holds the struck body (`BodyCombat::paralysis_timer`); the
//! ruleset owns how a paralyzed body buys time back.

use serde::{Deserialize, Serialize};

use crate::ParamValue;

/// The authored on-hit effect key.
pub const PARALYZE: &str = "smash.paralyze";

/// The longest paralysis a move may author, in seconds. A paralyzed body
/// cannot act, so a longer one is a stock lost with no counterplay but the
/// mash.
pub const PARALYZE_MAX_S: f32 = 2.0;

/// Authored parameters of one paralysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParalyzeParams {
    /// How long the struck body is held, in seconds.
    pub duration_s: f32,
}

/// Admission's check of a paralysis's params: they hydrate, and the time is
/// seconds above 0 and at most [`PARALYZE_MAX_S`].
pub fn check_paralyze_params(params: &ParamValue) -> Result<(), String> {
    let typed: ParalyzeParams = params.hydrate().map_err(|error| error.to_string())?;
    if typed.duration_s.is_finite() && typed.duration_s > 0.0 && typed.duration_s <= PARALYZE_MAX_S {
        Ok(())
    } else {
        Err(format!(
            "`duration_s` is {}; a paralysis is seconds, more than 0 and at most {PARALYZE_MAX_S}",
            typed.duration_s
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_refuses_a_paralysis_of_no_time_or_too_long() {
        let check = |duration_s: f32| check_paralyze_params(&ParamValue::from_typed(&ParalyzeParams { duration_s }).unwrap());
        assert_eq!(check(0.5), Ok(()));
        assert!(check(0.0).is_err());
        assert!(check(f32::NAN).is_err());
        assert!(check(PARALYZE_MAX_S + 0.1).is_err());
    }
}
