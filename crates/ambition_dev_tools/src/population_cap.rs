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
use std::sync::Mutex;

/// Cap the number of authored actors admitted per room. Unset means no cap.
pub const POPULATION_CAP_ENV: &str = "AMBITION_ACTOR_POPULATION_CAP";

/// `usize::MAX` means "uncapped", so the hot path is one relaxed load and a
/// compare rather than an `Option` and a branch on a lock.
static CAP: AtomicUsize = AtomicUsize::new(usize::MAX);
static RESOLVED: AtomicUsize = AtomicUsize::new(0);

/// The room currently being lowered, and how many actors it has admitted.
///
/// ⛔⛔ **THE COUNT USED TO BE PROCESS-GLOBAL AND NOTHING EVER RESET IT.** There
/// was a `reset()` whose doc said *"so a second room load starts over"*, and it
/// had zero callers — so a capped process that walked into a second room handed
/// that room an already-exhausted quota and lowered none of its cast. The Hall
/// curve was unaffected only because each of its points was a separate process
/// that entered one room.
///
/// Keying on the room id makes the reset structural: the quota belongs to a room
/// lowering, and a different room id IS the new transaction. A `Mutex` is free
/// here because this runs during room lowering, never per frame.
static ROOM: Mutex<Option<(String, usize)>> = Mutex::new(None);

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

/// Whether one more authored actor in `room` may be admitted, counting it if so.
///
/// ⚠ THE ORDER IS THE PLACEMENT ORDER, which is the room's authored order and is
/// therefore deterministic — the same cap admits the same cast every run. It is
/// NOT a spatial or salience choice: a capped hall is the FIRST n placements, not
/// the n nearest anything.
///
/// ⛔ CALL THIS ONLY WHERE AN ACTOR IS BEING BUILT. It used to be called from the
/// top of `lower_interactable_placement`, before the placement's kind was known,
/// so it counted doors, chests, pickups and breakables against an "actor" cap and
/// omitted them once the cap was reached. The Hall happens to contain only
/// `NpcSpawn` placements, so its curve selected what was intended; any other room
/// would have lost furniture instead of cast.
pub fn admit_actor(room: &str) -> bool {
    let cap = cap();
    if cap == usize::MAX {
        return true;
    }
    let mut guard = ROOM.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let admitted = match guard.as_mut() {
        // A different room is a new lowering transaction, and a new quota.
        Some((current, count)) if current == room => count,
        _ => {
            *guard = Some((room.to_owned(), 0));
            &mut guard.as_mut().expect("just set").1
        }
    };
    let allowed = *admitted < cap;
    *admitted += 1;
    allowed
}

/// Forget the running count, so a fresh lowering starts over. Tests only — the
/// room key is what resets this in a real run.
pub fn reset() {
    *ROOM.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
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
    /// Force the cap without touching the environment, so a test can exercise
    /// the capped road. The env read is a `OnceLock`-style latch; this sets the
    /// same cells it would have.
    fn force_cap(n: usize) {
        CAP.store(n, Ordering::Relaxed);
        RESOLVED.store(1, Ordering::Relaxed);
        reset();
    }

    fn release_cap() {
        CAP.store(usize::MAX, Ordering::Relaxed);
        RESOLVED.store(0, Ordering::Relaxed);
        reset();
    }

    /// ⛔⛔ ONE TEST, because the cap is process-global and cargo runs tests in
    /// parallel threads. This was two `#[test]`s for about a minute and they
    /// raced on `CAP` immediately: the uncapped-default assertion ran while this
    /// one had forced a cap of 2. Anything touching that latch belongs here.
    #[test]
    fn a_quota_belongs_to_a_room_lowering_and_a_second_room_gets_a_fresh_one() {
        // ⛔ THE PREMISE GUARD FOR EVERY MEASUREMENT TAKEN WITHOUT THE KNOB. If
        // the default were anything but "admit all", every uncapped run in the
        // journal would silently describe a different room than the one that
        // ships.
        assert!(
            std::env::var(POPULATION_CAP_ENV).is_err(),
            "this asserts the DEFAULT; setting the variable would test something else"
        );
        assert_eq!(active_cap(), None);
        for _ in 0..1000 {
            assert!(admit_actor("any_room"), "an uncapped build refuses nobody");
        }

        force_cap(2);

        // The first room spends its quota in placement order.
        assert!(admit_actor("hall"), "1st of 2");
        assert!(admit_actor("hall"), "2nd of 2");
        assert!(!admit_actor("hall"), "3rd exceeds the cap");
        assert!(!admit_actor("hall"), "and stays refused");

        // ⭐ THE DEFECT: before the room key, this returned false — the second
        // room inherited an exhausted counter and lowered none of its cast.
        assert!(
            admit_actor("goblin_encounter"),
            "a different room is a new lowering transaction and a fresh quota"
        );
        assert!(admit_actor("goblin_encounter"), "2nd of 2 in the new room");
        assert!(!admit_actor("goblin_encounter"), "then the new room's cap binds");

        // Returning to the first room is also a fresh transaction: rooms are
        // lowered one at a time, so "same id again" means a reload.
        assert!(admit_actor("hall"), "a reload starts the quota over");

        release_cap();
        assert!(active_cap().is_none(), "the knob is left inert for other tests");
    }
}
