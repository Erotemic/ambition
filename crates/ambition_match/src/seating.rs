//! Match-seat binding and the rollback-safe receipt for a live prepared match.

use bevy::prelude::*;

/// Stable roster seat carried by the fighter body.
/// Driving participant, character id, and entity order cannot substitute for seat identity.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchSeat(pub usize);

/// Derive live fighter entities from rollback-restored [`MatchSeat`] components, sorted by seat.
/// No resource stores live entity handles for the cast.
pub fn match_participants(seated: &Query<(Entity, &MatchSeat)>) -> Vec<Entity> {
    let mut by_seat: Vec<(usize, Entity)> = seated
        .iter()
        .map(|(entity, seat)| (seat.0, entity))
        .collect();
    by_seat.sort_by_key(|(seat, _)| *seat);
    by_seat.into_iter().map(|(_, entity)| entity).collect()
}

/// Receipt for a fully activated match.
/// Stores activation facts only; the live cast is derived from [`MatchSeat`] components.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ActiveMatch {
    /// How many seats this match activated with. Compare it against
    /// [`match_participants`] to ask whether the cast is still whole.
    seats: usize,
    /// The frozen seat topology this match was activated against, copied from
    /// the roster so the two can be COMPARED rather than assumed equal.
    seat_topology: Option<u64>,
    /// Whose plan this is a receipt for. `None` in a composition with no
    /// session lifecycle at all, which is the same answer `PreparedMatch` stamps
    /// there, so the two still compare equal.
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    /// Simulation tick of activation, used to derive opening-ceremony phase without mutable timer state.
    /// `None` means the composition has no simulation clock.
    activated_on: Option<u64>,
}

/// **This entity belongs to the match that created it, and dies with it.**
///
/// ⛔⛔ IT EXISTS BECAUSE A MINE OUTLIVED ITS MATCH. Jon, 2026-09-05, playing:
/// *"a mine laid in a match still persists into the next match, that sounds like
/// an issue with architecture expression. Ending a match should be cleaning
/// everything up."* Measured the same day: the smash ruleset spawns at five
/// sites — bomb, bolt, mine, portal, spring — and every one ended only by its own
/// rule (a fuse, a trigger, a lifetime). A match ending was not one of those
/// rules, so anything still waiting when a match ended was still waiting when the
/// next one began.
///
/// ⭐ THE SAME IDIOM `StocksMatchSettled` AND `SuddenDeathEntered` ALREADY USE,
/// moved from a resource onto an ENTITY. [`MatchInstance`]'s own doc calls itself
/// *"stable activation identity used by ruleset-local per-match state… so stale
/// state fails identity match"* — that is precisely this, and it was only ever
/// applied to resources.
///
/// ⛔ WHAT THIS REFUSES is a despawn in each of the five systems, or one sweep
/// that knows the five component types. Both put the end of a match's objects in
/// N places that must each remember, which is how the mine came to outlive a
/// match while the fighters did not — and the next technique authored would be
/// the sixth thing to forget. Stamped once at spawn, swept once by whoever owns
/// the match.
///
/// ⚠ THE SWEEP IS THE RULESET'S, not this crate's. `ambition_match` is data: it
/// says what an object belongs to, and a composition decides what to do about it.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MatchScoped(pub MatchInstance);

impl MatchScoped {
    /// Is this object still part of the match now running?
    ///
    /// ⚠ `None` — no active match at all — answers FALSE, deliberately. Between
    /// matches there is nothing for a mine to belong to, and leaving it on the
    /// select screen is the defect wearing a different hat.
    pub fn belongs_to(&self, active: Option<&ActiveMatch>) -> bool {
        active.is_some_and(|active| active.instance() == self.0)
    }
}

/// Stable activation identity used by ruleset-local per-match state.
/// It derives from rollback-restored session and activation tick, so stale state fails identity match.
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

    /// WHICH RUN OF THE WORLD this is, for `sim_random`.
    ///
    /// ⭐⭐ WITHOUT IT EVERY MATCH IS THE SAME MATCH. A draw is a pure function
    /// of its inputs and the match clock restarts at zero, so two matches that
    /// reach the same tick in one domain drew IDENTICALLY — the second match
    /// replayed the first's items, in order, from its first drop.
    ///
    /// ⛔ IT IS THE ACTIVATION STAMP, not a counter and not a wall clock. Both
    /// halves are already canonical simulation state that a rewind restores, so
    /// the context a resimulated tick draws with is the one it drew with the
    /// first time. Anything a peer could disagree about would desync the draws.
    ///
    /// A match with no stamp — a bare fixture — answers
    /// [`CONTEXT_UNSEEDED`](ambition_platformer2d_core::sim_random::CONTEXT_UNSEEDED),
    /// which is honest: it has no identity to draw against.
    pub fn random_context(&self) -> ambition_platformer2d_core::sim_random::RandomContext {
        match (self.session, self.activated_on) {
            (None, None) => ambition_platformer2d_core::sim_random::CONTEXT_UNSEEDED,
            // Mixed rather than concatenated: two sessions whose matches
            // activated on the same tick must not collapse onto one context, and
            // neither must one session's consecutive matches.
            (session, activated_on) => session
                .map_or(0, |session| session.0)
                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                .wrapping_add(
                    activated_on
                        .unwrap_or(0)
                        .wrapping_mul(0xD6E8_FEB8_6659_FD93),
                ),
        }
    }

    /// Rebuild a present activation from rollback state; resource snapshotting separately restores absence.
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

#[cfg(test)]
mod match_context_tests {
    use super::*;
    use ambition_platformer2d_core::sim_random::{sim_random, CONTEXT_UNSEEDED, DOMAIN_ITEM_SPAWN};
    use ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId;

    /// ⭐⭐ TWO MATCHES ARE TWO RUNS OF THE WORLD, and they must not draw alike.
    ///
    /// ⛔⛔ THE DEFECT THIS PINS: `sim_random` had no context axis, and every
    /// consumer keys on a match clock that restarts at zero. So match two drew
    /// match one's items, in order, from its first drop — the property that makes
    /// a resimulated tick reproduce made every playthrough reproduce with it.
    ///
    /// ⚠ THIS IS THE SEAM, NOT THE WHOLE ROAD. `spawn_match_items` passing this
    /// context to its draws is one line no test here can reach: `PreparedMatch`
    /// has no constructor outside `prepare_match`, so a fixture cannot give the
    /// spawner a rules table to read. What is pinned is that two PRODUCTION
    /// activations yield different contexts and that those contexts separate the
    /// draws — the same limit, and the same reason, as the match clock's
    /// ceremony half.
    #[test]
    fn two_activations_are_two_draw_contexts() {
        // Built the way activation builds them, not by hand.
        let first = ActiveMatch::activated(2, None, Some(SessionScopeId(0)), Some(100));
        let second = ActiveMatch::activated(2, None, Some(SessionScopeId(0)), Some(900));

        let a = first.instance().random_context();
        let b = second.instance().random_context();
        assert_ne!(
            a, b,
            "two matches in one session share a draw context, so the second \
             replays the first's items from its first drop"
        );

        // ⛔ AND THE DRAWS THEMSELVES SEPARATE, on the RAW value. A check on a
        // reduced index compares numbers differing only by a modulus and would
        // report a shared context as healthy.
        let drew = |context| {
            (0..32)
                .map(|tick| sim_random(DOMAIN_ITEM_SPAWN, context, tick, 0))
                .collect::<Vec<u64>>()
        };
        assert_eq!(
            drew(a)
                .iter()
                .zip(drew(b))
                .filter(|(x, y)| **x == *y)
                .count(),
            0,
            "the contexts differ and the draws do not, so the axis is inert"
        );

        // ⛔ AND A DIFFERENT SESSION IS A DIFFERENT RUN even at the same
        // activation tick — a fresh session restarts the sim clock, so without
        // this the first match of every session is identical.
        let next_session = ActiveMatch::activated(2, None, Some(SessionScopeId(1)), Some(100));
        assert_ne!(
            a,
            next_session.instance().random_context(),
            "two sessions whose first match activated on the same tick share a \
             context, so every playthrough opens the same way"
        );

        // A match with no identity at all has no context to draw against, and
        // says so rather than inventing one.
        let bare = ActiveMatch::activated(2, None, None, None);
        assert_eq!(bare.instance().random_context(), CONTEXT_UNSEEDED);
    }
    /// ⛔⛔ A NEW SESSION IS A NEW MATCH EVEN AT THE SAME ACTIVATION TICK.
    ///
    /// `MatchInstance` is `(session, activated_on)`, and anything comparing only
    /// the tick would keep the previous session's objects whenever the clocks
    /// lined up — which they do, because a fresh session starts its clock at
    /// zero. `MatchScoped::belongs_to` is what a sweep asks, so it has to be
    /// both facts.
    #[test]
    fn match_scoped_identity_is_session_and_tick_together() {
        let here = ActiveMatch::activated(2, None, Some(SessionScopeId(0)), Some(100));
        let same = MatchScoped(here.instance());
        assert!(
            same.belongs_to(Some(&here)),
            "an object stamped by the running match did not belong to it"
        );

        let elsewhere = ActiveMatch::activated(2, None, Some(SessionScopeId(1)), Some(100));
        assert!(
            !same.belongs_to(Some(&elsewhere)),
            "an object from another SESSION belonged to this match because the \
             activation ticks matched"
        );

        let later = ActiveMatch::activated(2, None, Some(SessionScopeId(0)), Some(900));
        assert!(
            !same.belongs_to(Some(&later)),
            "an object from an earlier match in the SAME session belonged to the \
             later one"
        );

        // ⛔ AND NO MATCH AT ALL MEANS NOTHING BELONGS — the select screen, where
        // a leftover mine is the same defect wearing a different hat.
        assert!(
            !same.belongs_to(None),
            "a match-scoped object belonged to a world with no active match"
        );
    }
}

impl ActiveMatch {
    /// Publish the receipt after the full cast has been activated.
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

    /// Session whose prepared plan this activation receipts.
    pub fn session(
        &self,
    ) -> Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId> {
        self.session
    }

    /// Number of seats activated; live participants are derived from the world.
    pub fn seats(&self) -> usize {
        self.seats
    }

    /// Identity rulesets use to key per-match state.
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

    /// Record the frozen topology that already agrees with this unchanged seating.
    pub fn adopt_seat_topology(&mut self, generation: u64) {
        self.seat_topology = Some(generation);
    }

    /// Test-only constructor for a live match without preparation.
    #[doc(hidden)]
    pub fn for_test(seats: usize, seat_topology: Option<u64>) -> Self {
        Self {
            seats,
            seat_topology,
            session: None,
            // No clock means no opening-ceremony hold.
            activated_on: None,
        }
    }

    /// The sim tick the cast was built on, when the composition had a clock.
    pub fn activated_on(&self) -> Option<u64> {
        self.activated_on
    }

    /// Rebuild an activation from a rollback snapshot.
    ///
    /// what makes registering this correct is that `bevy_ggrs` restores ABSENCE:
    /// `ResourceSnapshotPlugin::load` maps `(Some(_), None)` to `remove_resource`. Registration
    /// would have been decorative if the plugin only overwrote a present value, which is worth
    /// stating because that is the assumption the fix rests on.
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
