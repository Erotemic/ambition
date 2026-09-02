//! A developer's cap on how many authored actors a room ADMITS — the value,
//! not the knob.
//!
//! ⭐ THE SAME INVERSION AS [`crate::brain::AuthoredBrainOverride`], for the
//! third of the actor kernel's three developer reads. Until 2026-09-02 room
//! lowering called `ambition_dev_tools::population_cap::admit_actor()` — a
//! process-global `AtomicUsize` the simulation `fetch_add`ed while deciding
//! what the world contains, whose lifetime had to be re-opened by hand at the
//! start of every room build (`begin_room_lowering`), and which two earlier
//! shapes had already got wrong (a quota that outlived its room; a quota
//! keyed on the room's NAME that outlived a reload). The value now rides the
//! lowering snapshot, and the quota lives in the placement context that is
//! built once per room construction plan — so its lifetime is the plan's,
//! structurally, and nothing has to remember to reset it.
//!
//! ⛔ `None` means "no cap", which is what an unset environment variable has
//! always meant, at the cost of one compare per authored actor. This is a
//! MEASUREMENT knob (one workload, varied population, for a scaling curve);
//! it is not a gameplay policy and must not become one. The environment name
//! and the reasons stay with `ambition_dev_tools::population_cap`.

/// How many authored actors a room may admit. `None` = uncapped.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AuthoredPopulationCap(pub Option<usize>);

impl AuthoredPopulationCap {
    pub const UNCAPPED: Self = Self(None);

    pub fn capped_at(cap: usize) -> Self {
        Self(Some(cap))
    }

    pub fn cap(self) -> Option<usize> {
        self.0
    }
}

/// One room lowering's admission quota: the cap, and how many actors this
/// transaction has admitted so far.
///
/// Interior mutability because placements are lowered against a SHARED
/// snapshot (`&C`); an atomic rather than a `Cell` because the snapshot must
/// stay `Sync`. `Clone` copies the count, so a context cloned MID-lowering
/// would carry its spend — no road does that: the context is built once per
/// construction plan, before any placement is lowered.
#[derive(Debug, Default)]
pub struct ActorAdmission {
    cap: AuthoredPopulationCap,
    admitted: std::sync::atomic::AtomicUsize,
}

impl Clone for ActorAdmission {
    fn clone(&self) -> Self {
        Self {
            cap: self.cap,
            admitted: std::sync::atomic::AtomicUsize::new(
                self.admitted.load(std::sync::atomic::Ordering::Relaxed),
            ),
        }
    }
}

impl ActorAdmission {
    pub fn new(cap: AuthoredPopulationCap) -> Self {
        Self {
            cap,
            admitted: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn cap(&self) -> AuthoredPopulationCap {
        self.cap
    }

    /// Whether one more authored actor may be admitted, counting it if so.
    ///
    /// ⚠ THE ORDER IS THE PLACEMENT ORDER, which is the room's authored order
    /// and is therefore deterministic — the same cap admits the same cast every
    /// run. It is NOT a spatial or salience choice: a capped hall is the FIRST
    /// n placements, not the n nearest anything.
    pub fn admit_actor(&self) -> bool {
        let Some(cap) = self.cap.0 else {
            return true;
        };
        self.admitted
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            < cap
    }

    /// How many this transaction has admitted (for a census, not a decision).
    pub fn admitted(&self) -> usize {
        self.admitted.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A quota belongs to ONE lowering: two contexts are two quotas, and a
    /// fresh one for the same cap starts full — the property the old
    /// process-global counter got wrong twice.
    #[test]
    fn a_quota_belongs_to_one_lowering_and_a_second_lowering_starts_full() {
        let first = ActorAdmission::new(AuthoredPopulationCap::capped_at(2));
        assert!(first.admit_actor());
        assert!(first.admit_actor());
        assert!(!first.admit_actor(), "the third is over the cap");
        assert_eq!(first.admitted(), 3, "the refusal was counted as an attempt");

        let second = ActorAdmission::new(AuthoredPopulationCap::capped_at(2));
        assert!(second.admit_actor(), "a new lowering starts with a full quota");

        let uncapped = ActorAdmission::new(AuthoredPopulationCap::UNCAPPED);
        for _ in 0..1000 {
            assert!(uncapped.admit_actor());
        }
        assert_eq!(uncapped.admitted(), 0, "uncapped counts nothing");
    }
}
