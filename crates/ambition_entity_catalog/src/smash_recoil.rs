//! Authored payload for recoil: a strike that costs the body that struck.
//! The engine charges the striker as a price, not an injury
//! (`ambition.combat.recoil`, `BodyHealth::spend`): the meter advances, the
//! pool never drops below 1, nothing is attributed and nobody is stunned.

use serde::{Deserialize, Serialize};

use crate::ParamValue;

/// The authored on-hit effect key.
pub const RECOIL: &str = "smash.recoil";

/// The most a single strike may cost its striker, in health.
pub const RECOIL_MAX: i32 = 30;

/// Authored parameters of one recoil.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoilParams {
    /// What the striker pays each time the strike lands.
    pub amount: i32,
}

/// Admission's check of a recoil's params: they hydrate, and the amount is
/// more than 0 and at most [`RECOIL_MAX`].
pub fn check_recoil_params(params: &ParamValue) -> Result<(), String> {
    let typed: RecoilParams = params.hydrate().map_err(|error| error.to_string())?;
    if typed.amount > 0 && typed.amount <= RECOIL_MAX {
        Ok(())
    } else {
        Err(format!("`amount` is {}; a recoil costs more than 0 and at most {RECOIL_MAX}", typed.amount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_refuses_a_recoil_of_nothing_or_too_much() {
        let check = |amount: i32| check_recoil_params(&ParamValue::from_typed(&RecoilParams { amount }).unwrap());
        assert_eq!(check(3), Ok(()));
        assert!(check(0).is_err());
        assert!(check(RECOIL_MAX + 1).is_err());
    }
}
