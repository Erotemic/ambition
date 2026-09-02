//! A measurement knob: cap how many authored actors a room admits.
//!
//! ⭐ THIS EXISTS TO MAKE A SCALING CURVE POSSIBLE. `hall_of_characters` has its
//! cast authored in LDtk at a fixed 130, and one population cannot separate O(n)
//! from O(n²) — the question the actor-decision campaign is now on. Different
//! ROOMS are not a scaling experiment: brain mix, geometry, factions and assets
//! all change with the room, so the only honest curve varies population inside
//! ONE workload.
//!
//! ⛔⛔ IT CHANGES THE ROOM, AND THAT IS THE POINT. A capped run is not the
//! shipped hall and no number taken under it describes the shipped hall. The cap
//! is reported on the census row so a reader cannot mistake one for the other.
//!
//! ⚠ NOT A GAMEPLAY FEATURE, and deliberately not a `UserSettings` knob or a CLI
//! flag: it is read from the environment exactly once, like
//! `AMBITION_PROFILE_CENSUS` beside it, and absent means "no cap" at zero cost.
//! Making distant actors dormant is a separate and legitimate GAME policy; this
//! is not that, and must not become it.

use ambition_characters::actor::AuthoredPopulationCap;

/// Cap the number of authored actors admitted per room. Unset means no cap.
pub const POPULATION_CAP_ENV: &str = "AMBITION_ACTOR_POPULATION_CAP";

/// The value the environment asks for, read ONCE at plugin build and published
/// as a resource ([`AuthoredPopulationCap`]) that the sim reads; nothing in the
/// simulation names this crate.
///
/// ⭐ THE SAME INVERSION AS `brain_override::from_env`, and the last of the
/// actor kernel's three developer reads (D33). This module used to hold the
/// quota itself — a process-global `AtomicUsize` the kernel `fetch_add`ed
/// while lowering, re-opened by hand (`begin_room_lowering`) at the start of
/// every room build, wrong twice before that (a quota that outlived its room;
/// one keyed on the room's NAME that outlived a reload). The quota now lives in
/// the placement context built once per construction plan
/// (`ambition_characters::actor::ActorAdmission`), so its lifetime is the
/// plan's by construction, and this crate keeps only the knob: the name, the
/// parse, and the reason.
///
/// ⛔ An unparsable value is UNCAPPED, not a panic: the knob is a measurement
/// convenience and "I typed it wrong" should not stop a run — but it is logged,
/// because a curve taken under a silently ignored cap is a wrong curve.
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

    /// ⛔ THE PREMISE GUARD FOR EVERY MEASUREMENT TAKEN WITHOUT THE KNOB. If the
    /// default were anything but "admit all", every uncapped run in the journal
    /// would silently describe a different room than the one that ships. The
    /// quota's own behaviour (spend in placement order; a new lowering starts
    /// full; a reload of the same room starts full) is tested where the quota
    /// lives now, on `ActorAdmission`, without any process-global to race.
    #[test]
    fn an_unset_environment_publishes_no_cap() {
        assert!(
            std::env::var(POPULATION_CAP_ENV).is_err(),
            "this asserts the DEFAULT; setting the variable would test something else"
        );
        assert_eq!(from_env(), AuthoredPopulationCap::UNCAPPED);
    }
}
