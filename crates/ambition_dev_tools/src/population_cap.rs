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
static RESOLVED: AtomicUsize = AtomicUsize::new(0);

/// How many actors this room-lowering transaction has admitted.
///
/// ⛔⛔ **ITS LIFETIME IS A TRANSACTION, NOT A NAME.** Two earlier shapes were
/// wrong. First it was process-global with a `reset()` nobody called, so a
/// second room inherited an exhausted quota. Then it was keyed on the room ID,
/// which fixed *that* and still said "until we see a different room" — so
/// **reloading the same room** (hot reload, reset, rebuild) kept the spent
/// counter and admitted nobody.
///
/// [`begin_room_lowering`] is called from the one place that starts a room's
/// placement transaction, so the lifetime is structural: a quota cannot outlive
/// the lowering that opened it, whatever the room is called.
static ADMITTED: AtomicUsize = AtomicUsize::new(0);

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
pub fn admit_actor() -> bool {
    let cap = cap();
    if cap == usize::MAX {
        return true;
    }
    ADMITTED.fetch_add(1, Ordering::Relaxed) < cap
}

/// Open a room-lowering transaction: this room starts with a full quota.
///
/// Called from the monolith's room construction, beside `plan_room`, which runs
/// exactly once per room build.
pub fn begin_room_lowering() {
    ADMITTED.store(0, Ordering::Relaxed);
}

/// Alias of [`begin_room_lowering`] for tests that read better this way.
pub fn reset() {
    begin_room_lowering();
}

/// The cap in force, for the census row. `None` when uncapped.
pub fn active_cap() -> Option<usize> {
    match cap() {
        usize::MAX => None,
        n => Some(n),
    }
}

/// Force the cap without the environment, for tests in crates that own the
/// admission seam. `None` restores "uncapped".
///
/// ⚠ PROCESS-GLOBAL, like the knob it stands in for. A test using this must not
/// run beside another that reads the cap.
#[doc(hidden)]
pub fn force_cap_for_tests(cap: Option<usize>) {
    CAP.store(cap.unwrap_or(usize::MAX), Ordering::Relaxed);
    RESOLVED.store(if cap.is_some() { 1 } else { 0 }, Ordering::Relaxed);
    begin_room_lowering();
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
            assert!(admit_actor(), "an uncapped build refuses nobody");
        }

        force_cap(2);

        // A transaction spends its quota in placement order.
        begin_room_lowering();
        assert!(admit_actor(), "1st of 2");
        assert!(admit_actor(), "2nd of 2");
        assert!(!admit_actor(), "3rd exceeds the cap");
        assert!(!admit_actor(), "and stays refused");

        // ⭐ DEFECT ONE: the count was process-global with an uncalled reset, so
        // the NEXT room inherited an exhausted quota and lowered none of its cast.
        begin_room_lowering();
        assert!(admit_actor(), "a new lowering opens a fresh quota");
        assert!(admit_actor(), "2nd of 2 in the new transaction");
        assert!(!admit_actor(), "then this transaction's cap binds");

        // ⭐ DEFECT TWO: keying the quota on the ROOM ID fixed the first defect
        // and left this one — reloading the SAME room kept the spent counter, so
        // a hot reload or reset admitted nobody. The lifetime is the transaction,
        // not the name, and this arm is the one that proves it.
        begin_room_lowering();
        assert!(admit_actor(), "RELOADING the same room opens a fresh quota too");
        assert!(admit_actor(), "2nd of 2 on the reload");
        assert!(!admit_actor(), "and the cap still binds");

        release_cap();
        assert!(active_cap().is_none(), "the knob is left inert for other tests");
    }
}
