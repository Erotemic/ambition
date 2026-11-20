//! Authored payload for lifesteal: a strike that mends the body that struck.
//! The engine repays the striker's health and its damage meter
//! (`ambition.combat.mend`, `BodyHealth::heal`); a struck body keeps its hit.

use serde::{Deserialize, Serialize};

use crate::ParamValue;

/// The authored on-hit effect key.
pub const LIFESTEAL: &str = "smash.lifesteal";

/// The most a single strike may mend, in health. A lifesteal that repays more
/// than a heavy hit deals is a heal button with a hitbox on it.
pub const LIFESTEAL_MAX: i32 = 30;

/// Authored parameters of one lifesteal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifestealParams {
    /// How much the striker mends each time the strike lands.
    pub amount: i32,
}

/// Admission's check of a lifesteal's params: they hydrate, and the amount is
/// more than 0 and at most [`LIFESTEAL_MAX`].
pub fn check_lifesteal_params(params: &ParamValue) -> Result<(), String> {
    let typed: LifestealParams = params.hydrate().map_err(|error| error.to_string())?;
    if typed.amount > 0 && typed.amount <= LIFESTEAL_MAX {
        Ok(())
    } else {
        Err(format!("`amount` is {}; a lifesteal mends more than 0 and at most {LIFESTEAL_MAX}", typed.amount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_refuses_a_lifesteal_of_nothing_or_too_much() {
        let check = |amount: i32| check_lifesteal_params(&ParamValue::from_typed(&LifestealParams { amount }).unwrap());
        assert_eq!(check(4), Ok(()));
        assert!(check(0).is_err());
        assert!(check(-3).is_err());
        assert!(check(LIFESTEAL_MAX + 1).is_err());
    }
}
