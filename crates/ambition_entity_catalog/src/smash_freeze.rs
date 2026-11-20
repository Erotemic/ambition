//! Authored payload for a freeze applied by a damaging hit.
//! The engine encases the struck body (`BodyCombat::freeze_timer`); the
//! ruleset owns how a frozen body thaws.

use serde::{Deserialize, Serialize};

use crate::ParamValue;

/// The authored on-hit effect key.
pub const FREEZE: &str = "smash.freeze";

/// The longest freeze a move may author, in seconds. A frozen body
/// cannot act, so a longer one is a stock lost with no counterplay but the
/// mash.
pub const FREEZE_MAX_S: f32 = 2.5;

/// Authored parameters of one freeze.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreezeParams {
    /// How long the struck body stays encased, in seconds.
    pub duration_s: f32,
}

/// Admission's check of a freeze's params: they hydrate, and the time is
/// seconds above 0 and at most [`FREEZE_MAX_S`].
pub fn check_freeze_params(params: &ParamValue) -> Result<(), String> {
    let typed: FreezeParams = params.hydrate().map_err(|error| error.to_string())?;
    if typed.duration_s.is_finite() && typed.duration_s > 0.0 && typed.duration_s <= FREEZE_MAX_S {
        Ok(())
    } else {
        Err(format!(
            "`duration_s` is {}; a freeze is seconds, more than 0 and at most {FREEZE_MAX_S}",
            typed.duration_s
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_refuses_a_freeze_of_no_time_or_too_long() {
        let check = |duration_s: f32| check_freeze_params(&ParamValue::from_typed(&FreezeParams { duration_s }).unwrap());
        assert_eq!(check(0.5), Ok(()));
        assert!(check(0.0).is_err());
        assert!(check(f32::NAN).is_err());
        assert!(check(FREEZE_MAX_S + 0.1).is_err());
    }
}
