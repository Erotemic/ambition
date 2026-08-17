//! **What a seated fighter IS, and which match is live.**
//!
//! ⚠ **the verb moved.** Turning a roster into bodies used to live here, and it
//! resolved each seat against live authority tables in the middle of building
//! it. That shape is gone: [`crate::character_runtime::prepared_match`] answers
//! every question first and then builds the whole cast from the answer. What is
//! left here is the seat BINDING — the component a body wears, the derivation
//! that reads the cast off the world, and the latch that says a match is live.
//!
//! ⛔ **and the fork it removed is worth naming, because it generated every bug
//! in the 2026-08-06 report.** A local player's seat used to ADOPT the session's
//! existing body while a CPU's seat SPAWNED a new one, so a fighter's
//! construction depended on who happened to drive it. From that one asymmetry
//! came: a costume handshake that could deadlock the whole match, seat 0 being
//! privileged, health/box/mass/ability divergences unified one at a time across
//! three weeks, and a match with nobody local in it being inexpressible.
//! Realization is now one path for every fighter and control is attached
//! afterwards.

use bevy::prelude::*;

/// Which seat of the match this body is.
///
/// The roster's index, on the body. Match RULES need to name a fighter — whose
/// health bar is on the left, who won the round, where to put them back — and
/// every other way to identify one is a guess: `Brain::Player(slot)` misses the
/// CPU seat, the worn character id collides in a mirror match, and entity order
/// is not an order. Activation is the only place that knows, so it says so.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchSeat(pub usize);

/// **The bodies in a live match, in seat order, DERIVED from the world.**
///
/// The seat binding lives on the fighters as [`MatchSeat`], which is where it
/// belongs: a body knows which seat it is, and a body that no longer exists
/// cannot claim one. Anything that needs the cast asks the world through this,
/// rather than reading a list somebody remembered.
///
/// ⚠ **this used to be a `Vec<Entity>` on [`ActiveMatch`], and that was the bug**
/// (GPT 5.6, 2026-07-29). A resource holding live `Entity` values, mutated from
/// inside the rollback schedule and not registered as rollback state, keeps its
/// future contents across a rewind: the bodies are restored to an earlier state —
/// or to not existing — while the list still names them. Deriving costs one
/// query and cannot go stale, because there is nothing to keep in step.
///
/// Sorted by seat, so `participants[i]` is seat `i` however the entities were
/// spawned. Entity order is not an order; a set that arrives in spawn order makes
/// indexing it mean nothing.
pub fn match_participants(seated: &Query<(Entity, &MatchSeat)>) -> Vec<Entity> {
    let mut by_seat: Vec<(usize, Entity)> = seated
        .iter()
        .map(|(entity, seat)| (seat.0, entity))
        .collect();
    by_seat.sort_by_key(|(seat, _)| *seat);
    by_seat.into_iter().map(|(_, entity)| entity).collect()
}

/// **The match that is LIVE.**
///
/// Present means every seat in the prepared match has a body. Absent means no
/// match is running — either none was prepared, or activation has not run yet.
///
/// ⚠ this replaced a `MatchSeated(bool)`, and the difference is the whole point.
/// A bool said seating had FINISHED and never said WHO, so nothing could ask
/// whether the live fighters are still the set the match was built from.
///
/// ⚠ **and it says how MANY, not WHICH** (GPT 5.6, 2026-07-29). Naming the
/// bodies meant holding `Vec<Entity>` in a resource written from inside the
/// rollback schedule and not registered as rollback state, so a rewind across
/// activation would restore the fighters and leave the list pointing at the
/// future. [`match_participants`] derives the cast from [`MatchSeat`] on the
/// bodies themselves, which rewinds because the bodies do.
///
/// What is left is plain data: a count, a generation number, and **the identity
/// of the activation it receipts** — all facts about the DECISION to activate.
///
/// ⛔ **the identity is load-bearing and it used to be a presence proxy.**
/// Activation asked *"is there an `ActiveMatch` AND are there `MatchSeat`
/// bodies"*, reading a receipt with no bodies as a dead session's paperwork.
/// That is false for a platform fighter: `take_eliminated_fighters_out_of_play`
/// despawns an eliminated fighter's body, and a simultaneous final-stock ring-out
/// is a supported DRAW — so a legitimately finished match sits at
/// `ActiveMatch = current`, `MatchSeat count = 0` for the whole 4.5s the winner
/// card is up. Activation would have fallen straight through and rebuilt the
/// cast with fresh stocks, underneath the announcement. Found by review
/// (GPT 5.6, 2026-08-07) before anybody had to watch it happen.
///
/// ⭐ so identity is stated, not inferred: this receipt names the session whose
/// plan it built, and fighter presence is irrelevant to it.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ActiveMatch {
    /// How many seats this match activated with. Compare it against
    /// [`match_participants`] to ask whether the cast is still whole.
    seats: usize,
    /// The frozen seat topology this match was activated against, copied from
    /// the roster so the two can be COMPARED rather than assumed equal.
    seat_topology: Option<u64>,
    /// **Whose plan this is a receipt for.** `None` in a composition with no
    /// session lifecycle at all, which is the same answer `PreparedMatch` stamps
    /// there, so the two still compare equal.
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    /// **The sim tick the cast was built on**, so the opening ceremony can be
    /// DERIVED rather than ticked.
    ///
    /// ⭐ this is what lets a 3–2–1–GO countdown exist without any new mutable
    /// state in the rollback window: the phase is `now - activated_on` against
    /// the ruleset's declared length, which a rewind recomputes identically.
    ///
    /// `None` in a composition with no sim clock at all (a bare fixture), where
    /// the honest answer is that there is no ceremony to time — the hold is
    /// released immediately, exactly as it was before countdowns existed.
    activated_on: Option<u64>,
}

/// **WHICH ACTIVATION OF WHICH MATCH** — the identity a ruleset keys its own
/// per-match state on. (D147)
///
/// ⛔⛔ **the alternative is a process-global latch somebody has to remember to
/// clear, and forgetting is what D140 was.** `StocksMatchSettled` was a bare
/// `bool` about the process rather than about a match: it went true when a match
/// ended and stayed true, so the next match on the same stage opened wearing the
/// previous one's verdict and could never be decided. The repair at the time was
/// to retract it from `activate_the_prepared_match` — which works, and which
/// made the GENERIC activation road know that one particular ruleset keeps a
/// private boolean.
///
/// ⭐ **keyed to this, a ruleset's per-match state goes stale BY
/// CONSTRUCTION.** *"Has this match been decided"* is `settled == Some(live
/// instance)`, and a different match is not this one — so there is no retraction
/// to schedule, no ordering to get right, and nothing for activation to know
/// about a ruleset it may not even be composed with.
///
/// ⚠ **it rewinds because both halves do.** This is derived from
/// [`ActiveMatch`], which is rollback state, and the ruleset's latch stores a
/// copy — so a rewind across the deciding frame restores a latch stamped with a
/// match and a receipt that agree again, or restores the receipt's ABSENCE and
/// the question stops being asked. No change detection is involved, which is the
/// property that makes it safe inside the rollback window.
///
/// ⚠ **a composition with neither a session nor a clock cannot tell two
/// activations apart**, and that is the honest answer rather than a hole: those
/// are the two facts that distinguish one activation from the next, and a
/// composition with no clock has no tick on which to run a second match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MatchInstance {
    /// The gameplay session the cast was built in.
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    /// The sim tick it was built on.
    activated_on: Option<u64>,
}

impl MatchInstance {
    /// The two facts, for the wire format. See `snapshot_impls`.
    #[doc(hidden)]
    pub fn parts(
        &self,
    ) -> (
        Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
        Option<u64>,
    ) {
        (self.session, self.activated_on)
    }

    /// Rebuild from a rollback snapshot. The fields stay private so
    /// [`ActiveMatch::instance`] is the only place one is MINTED; this is the
    /// hatch, and it is named for what it is.
    #[doc(hidden)]
    pub fn from_snapshot(
        session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
        activated_on: Option<u64>,
    ) -> Self {
        Self {
            session,
            activated_on,
        }
    }
}

impl ActiveMatch {
    /// **Publish an activation.** The one production constructor.
    ///
    /// ⚠ called only by `activate_the_prepared_match`, which is infallible — so
    /// unlike every earlier version of this latch there is no path on which it
    /// can be published over a partially built cast.
    pub fn activated(
        seats: usize,
        seat_topology: Option<u64>,
        session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
        activated_on: Option<u64>,
    ) -> Self {
        Self {
            seats,
            seat_topology,
            session,
            activated_on,
        }
    }

    /// How many ticks the match has been live, or `None` when the composition
    /// has no clock to measure against.
    pub fn ticks_since_activation(&self, now: u64) -> Option<u64> {
        self.activated_on.map(|then| now.saturating_sub(then))
    }

    /// **The session whose prepared plan this receipts.**
    ///
    /// Activation compares this against the plan's own stamp to ask "have I
    /// already built THIS match", which is a question a despawned cast cannot
    /// change the answer to.
    pub fn session(
        &self,
    ) -> Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId> {
        self.session
    }

    /// How many fighters this match activated with.
    ///
    /// Deliberately not "how many are alive now" — that is a question for the
    /// world, and [`match_participants`] answers it. The difference between the
    /// two is exactly the signal a rules layer wants.
    pub fn seats(&self) -> usize {
        self.seats
    }

    /// **WHICH MATCH THIS IS**, as something a ruleset can key its own state on.
    ///
    /// See [`MatchInstance`] for why a ruleset wants one.
    pub fn instance(&self) -> MatchInstance {
        MatchInstance {
            session: self.session,
            activated_on: self.activated_on,
        }
    }

    /// Which frozen topology decided this match's seating, if a session had
    /// frozen one when the roster was built.
    pub fn seat_topology(&self) -> Option<u64> {
        self.seat_topology
    }

    /// **Adopt a frozen topology this match already agrees with.**
    ///
    /// The ONLY legitimate mutation of a live activation, and the narrowness is
    /// the point. It records which topology decided a seating that has not
    /// changed — it cannot move a body, add a seat or drop one.
    pub fn adopt_seat_topology(&mut self, generation: u64) {
        self.seat_topology = Some(generation);
    }

    /// Build an activation directly, for a test that needs a LIVE match without
    /// standing up preparation to produce one.
    ///
    /// The fields stay private so production has exactly one publisher; this is
    /// the hatch, and it is named for what it is.
    #[doc(hidden)]
    pub fn for_test(seats: usize, seat_topology: Option<u64>) -> Self {
        Self {
            seats,
            seat_topology,
            session: None,
            // No clock, so no ceremony: a fixture that wants a LIVE match gets
            // one, which is what this hatch is named for.
            activated_on: None,
        }
    }

    /// The sim tick the cast was built on, when the composition had a clock.
    pub fn activated_on(&self) -> Option<u64> {
        self.activated_on
    }

    /// Rebuild an activation from a rollback snapshot.
    ///
    /// ⚠ **what makes registering this correct is that `bevy_ggrs` restores
    /// ABSENCE**: `ResourceSnapshotPlugin::load` maps `(Some(_), None)` to
    /// `remove_resource`. So a rewind to a frame before activation does not
    /// merely stale the latch, it deletes it — activation sees no active match,
    /// re-runs, and rebuilds the cast from the prepared plan, which is NOT
    /// rollback state and is therefore still there to rebuild from. Registration
    /// would have been decorative if the plugin only overwrote a present value,
    /// which is worth stating because that is the assumption the fix rests on.
    #[doc(hidden)]
    pub fn from_snapshot(
        seats: usize,
        seat_topology: Option<u64>,
        session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
        activated_on: Option<u64>,
    ) -> Self {
        Self {
            seats,
            seat_topology,
            session,
            activated_on,
        }
    }
}

// The tests that used to live here moved with the verb they exercise, to
// `prepared_match`. What remains in this file is a component, a derivation and
// a latch, each covered where it is used.
