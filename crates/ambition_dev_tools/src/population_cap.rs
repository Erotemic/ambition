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

use std::sync::atomic::{AtomicUsize, Ordering};

/// Cap the number of authored actors admitted per room. Unset means no cap.
pub const POPULATION_CAP_ENV: &str = "AMBITION_ACTOR_POPULATION_CAP";

/// `usize::MAX` means "uncapped", so the hot path is one relaxed load and a
/// compare rather than an `Option` and a branch on a lock.
static CAP: AtomicUsize = AtomicUsize::new(usize::MAX);
static ADMITTED: AtomicUsize = AtomicUsize::new(0);
static RESOLVED: AtomicUsize = AtomicUsize::new(0);

fn cap() -> usize {
    // ⛔ READ ONCE. `std::env::var` on every placement would make the knob's own
    // cost part of what it is measuring, and an env read is not free.
    if RESOLVED.load(Ordering::Relaxed) == 0 {
        let resolved = std::env::var(POPULATION_CAP_ENV)
            .ok()
            .and_then(|raw| raw.trim().parse::<usize>().ok())
            .unwrap_or(usize::MAX);
        CAP.store(resolved, Ordering::Relaxed);
        RESOLVED.store(1, Ordering::Relaxed);
    }
    CAP.load(Ordering::Relaxed)
}

/// Whether one more authored actor may be admitted, counting it if so.
///
/// ⚠ THE ORDER IS THE PLACEMENT ORDER, which is the room's authored order and is
/// therefore deterministic — the same cap admits the same cast every run. It is
/// NOT a spatial or salience choice: a capped hall is the FIRST n placements, not
/// the n nearest anything.
pub fn admit_actor() -> bool {
    let cap = cap();
    if cap == usize::MAX {
        return true;
    }
    ADMITTED.fetch_add(1, Ordering::Relaxed) < cap
}

/// Forget the running count, so a second room load starts over.
pub fn reset() {
    ADMITTED.store(0, Ordering::Relaxed);
}

/// The cap in force, for the census row. `None` when uncapped.
pub fn active_cap() -> Option<usize> {
    match cap() {
        usize::MAX => None,
        n => Some(n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An uncapped build admits everything and never counts.
    ///
    /// ⛔ THE PREMISE GUARD FOR EVERY MEASUREMENT TAKEN WITHOUT THE KNOB. If the
    /// default were anything but "admit all", every uncapped run in the journal
    /// would silently describe a different room than the one that ships.
    #[test]
    fn no_environment_variable_means_no_cap() {
        // The static is resolved from an env this test does not set.
        assert!(
            std::env::var(POPULATION_CAP_ENV).is_err(),
            "this test asserts the DEFAULT; setting the variable would test \
             something else"
        );
        assert_eq!(active_cap(), None);
        for _ in 0..1000 {
            assert!(admit_actor(), "an uncapped build refuses nobody");
        }
    }
}
