//! A measurement knob: cap how many authored actors a room admits.
//!
//! This makes a scaling curve possible. `hall_of_characters` has a fixed cast
//! of 130, and one population cannot separate O(n) from O(n²). Different rooms
//! change brain mix, geometry, factions and assets, so the curve must vary
//! population inside one workload.
//!
//! A capped run is not the shipped hall. The cap is reported on the census row.
//!
//! Not a gameplay feature, `UserSettings` knob or CLI flag. Read from the
//! environment once, like `AMBITION_PROFILE_CENSUS`; absent means no cap.
//! Dormant distant actors are a separate game policy.

use ambition_characters::actor::AuthoredPopulationCap;

/// Cap the number of authored actors admitted per room. Unset means no cap.
pub const POPULATION_CAP_ENV: &str = "AMBITION_ACTOR_POPULATION_CAP";

/// The value the environment asks for, read once at plugin build and published
/// as a resource ([`AuthoredPopulationCap`]) that the sim reads; nothing in the
/// simulation names this crate.
///
/// Like `brain_override::from_env`, this keeps environment reads out of the
/// actor kernel (D33). The quota itself lives in the placement context built
/// once per construction plan (`ambition_characters::actor::ActorAdmission`),
/// so its lifetime is the plan's. This crate keeps only the knob.
///
/// An unparsable value runs uncapped, not a panic, but it is logged.
pub fn from_env() -> AuthoredPopulationCap {
    let Ok(raw) = std::env::var(POPULATION_CAP_ENV) else {
        return AuthoredPopulationCap::UNCAPPED;
    };
    match raw.trim().parse::<usize>() {
        Ok(cap) => AuthoredPopulationCap::capped_at(cap),
        Err(error) => {
            eprintln!(
                "[population-cap] {POPULATION_CAP_ENV}={raw:?} is not a count ({error}); \
                 running UNCAPPED"
            );
            AuthoredPopulationCap::UNCAPPED
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Premise guard for every run without the knob: the default must admit all.
    /// The quota's own behaviour is tested on `ActorAdmission`.
    #[test]
    fn an_unset_environment_publishes_no_cap() {
        assert!(
            std::env::var(POPULATION_CAP_ENV).is_err(),
            "this asserts the DEFAULT; setting the variable would test something else"
        );
        assert_eq!(from_env(), AuthoredPopulationCap::UNCAPPED);
    }
}
