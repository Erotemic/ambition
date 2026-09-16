//! Match-seat binding and the rollback-safe receipt for a live prepared match.

use bevy::prelude::*;

/// Stable roster seat carried by the fighter body.
/// Driving participant, character id, and entity order cannot substitute for seat identity.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchSeat(pub usize);

// ⛔⛔ **`SeatCredit` LIVED HERE AND WAS REMOVED AT v178 (2026-09-10) BECAUSE
// NOTHING EVER READ IT.** It labelled a stand-in entity that a ruleset spawns
// when a delayed attack — a mark with a 1.4s fuse — materialises after the
// fighter who authored it has lost their last stock. That stand-in is REAL and
// still exists: every hitbox names its credited attacker as an `Entity`, and
// without it the blast fell back to the marked VICTIM as its owner, so a
// bystander it KO'd was credited to the fighter who was marked.
//
// ⇒ What the stand-in needs is to BE a valid non-victim `Entity` and to carry no
// [`MatchSeat`], so [`match_participants`] cannot count it and the match decides
// exactly as before. Both of those are load-bearing. The positive seat LABEL was
// not: this type's own doc promised that "a consumer that resolves which seat
// did this asks `MatchSeat` OR `SeatCredit`", and MEASURED 2026-09-10, no such
// consumer was ever written. Attribution runs on `Entity` throughout —
// `ambition_combat::events` says a body past the blast margin is "credited by
// `HitEvent::attacker`, not by geometry", `BodyKnockedOut` carries a `cause` and
// no attacker, and `MatchVerdict` is decided by stocks with no per-seat KO tally
// anywhere in the tree.
//
// ⚠ So it was not an unfinished half of a road; the road was never built and the
// engine made a different choice about what carries credit. It cost a snapshot
// LAYOUT entry for a fact no system reads. ⭐ And the two tests that covered it
// asserted the LABEL rather than a consequence — which they had to, because a
// component with no reader HAS no consequence to assert. That is the tell.

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
    /// ⛔⛔⛔ **THE SESSION TERM WAS REMOVED 2026-09-14, AND IT WAS A DESYNC.**
    /// This mixed in the raw `SessionScopeId.0` — a PER-APP MONOTONIC COUNTER
    /// minted by `ActiveSessionScope::begin` once per local activation. The draws
    /// it seeds choose **which item appears and which spawn point receives it**
    /// (`items/match_spawn.rs`), so two peers whose Apps had activated a
    /// different number of sessions built DIFFERENT AUTHORITATIVE WORLDS from the
    /// same match state. Found by the GPT architecture review of 2026-09-14.
    ///
    /// ⛔⛤ **AND THE PARAGRAPH THAT DEFENDED IT NAMED THE RULE IT BROKE.** It
    /// said *"both halves are already canonical simulation state that a rewind
    /// restores … anything a peer could disagree about would desync the draws."*
    /// Rollback-restored establishes that ONE MACHINE REWINDING ITSELF agrees
    /// with itself. It says nothing about TWO PEERS agreeing, and those are
    /// different properties that need different words. The bark draw three files
    /// away rejects `Entity::to_bits()` as a salt for exactly this reason, in
    /// exactly these words — a defence applied to one argument of a call is not
    /// applied to the call.
    ///
    /// ⇒ **THE CONTEXT IS THE ACTIVATION TICK, WHICH BOTH PEERS SIMULATE.**
    /// Within one deterministic run consecutive matches activate on different
    /// ticks, so they still draw differently — which is the property the first
    /// paragraph is about.
    ///
    /// ⛔⛔ **AND THE REPLACEMENT TERM IS ALSO HOST-LOCAL — RECORDED, NOT
    /// FIXED, 2026-09-15.** `activated_on` is a `SimTick`, and `SimTick` is an
    /// absolute count of every sim step this App has run, MENUS INCLUDED (one
    /// writer, `advance_sim_tick`, unconditional at the head of the schedule,
    /// never rebased — measured). Two peers who reached the same lobby by
    /// different routes therefore draw DIFFERENT ITEMS from the same match
    /// state. This is the same defect as the session term, one layer down, and
    /// the paragraph below understated it.
    ///
    /// ⚠ It stays for now because the alternative available today is no
    /// distinguishing term at all, which makes every match in a run replay the
    /// first match's drops — a visible gameplay regression, pinned by
    /// `two_activations_are_two_draw_contexts`. ⇒ **THE FIX IS THE MATCH'S
    /// ORDINAL WITHIN THE AGREED SESSION**: it starts at zero for everyone who
    /// joins together, is insensitive to menu time and prior sessions, and still
    /// separates consecutive matches. That is the next ID-PEER step.
    ///
    /// ⚠ **The CHECKSUM projections no longer read this term** — see
    /// `ActiveMatch::peer_stable_checksum` and the settlement/clock projections.
    /// A checksum is compared every frame, so a false desync there is fatal,
    /// while a repeated item table is not.
    ///
    /// ⚠ **WHAT THIS COSTS, STATED**: two runs of the world whose matches
    /// activate on the same tick now draw the SAME items. That is a repeat across
    /// a restart, not a divergence — both peers still agree — and it is the
    /// honest trade for removing a desync. Restoring cross-run variety needs a
    /// peer-NEGOTIATED nonce (a lobby-supplied match seed), which is netcode this
    /// repository does not have yet. ⛔ **`seat_topology` IS NOT THAT NONCE**:
    /// measured at HEAD, the one production caller of `activate_if_seatable`
    /// passes `None`.
    ///
    /// ⚠ **`SessionScopeId` STAYS ON THIS TYPE** and still decides EQUALITY, so
    /// `MatchScoped::belongs_to` and the settlement resources' staleness checks
    /// are unchanged. Local ownership is what it is for; a random seed is not.
    ///
    /// A match with no stamp — a bare fixture — answers
    /// [`CONTEXT_UNSEEDED`](ambition_platformer2d_core::sim_random::CONTEXT_UNSEEDED),
    /// which is honest: it has no identity to draw against.
    pub fn random_context(&self) -> ambition_platformer2d_core::sim_random::RandomContext {
        match (self.session, self.activation_tick()) {
            (None, None) => ambition_platformer2d_core::sim_random::CONTEXT_UNSEEDED,
            (_, activated_on) => activated_on
                .unwrap_or(0)
                .wrapping_mul(0xD6E8_FEB8_6659_FD93),
        }
    }

    /// ⛔⛔ **THIS IS NOT A PEER-STABLE TERM, AND CALLING IT ONE WAS THE
    /// MISTAKE.** It was `peer_stable()` for a day, documented as "the
    /// activation tick, which both peers simulate".
    ///
    /// **Measured 2026-09-15:** `SimTick` has exactly one writer
    /// (`ambition_time::advance_sim_tick`, `+1` per step), is `init_resource`'d
    /// once at App build, and NOTHING in the workspace rebases it — and it sits
    /// unconditionally at the head of the sim schedule, so it advances in menus
    /// and while gameplay is suspended. `activated_on` is therefore *the total
    /// number of sim steps this App has ever run*. Two hosts that sat on the
    /// select screen for different numbers of frames disagree about it.
    ///
    /// ⇒ Keep it for what it is: a LOCAL stamp that distinguishes one match from
    /// the next on one machine, which is what `belongs_to` and the settlement
    /// staleness checks need. Nothing compared between peers may read it.
    pub fn activation_tick(&self) -> Option<u64> {
        self.activated_on
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

    /// ⛔ THE PROJECTION IS WHAT MAKES `ActiveMatch` PEER-SAFE, and a
    /// registration kind cannot show that — `resource-clone-custom-checksum`
    /// says a projection exists, not what it excludes. This is the arm that
    /// says what it excludes.
    #[test]
    fn the_peer_stable_checksum_ignores_session_and_seat_topology() {
        use ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId;

        let receipt = |session: u64, topology: Option<u64>| {
            ActiveMatch::activated(2, topology, Some(SessionScopeId(session)), Some(4_200))
        };
        // Two hosts: different prior session counts, different local device
        // topology generations, same match.
        assert_eq!(
            receipt(1, Some(7)).peer_stable_checksum(),
            receipt(9, Some(31)).peer_stable_checksum(),
            "the receipt's checksum moves with the host's session count or its \
             local seat-topology generation, so two peers running one match \
             would disagree"
        );
        // ⛔ AND IT MUST STILL SEE THE MECHANICAL FACTS, or excluding the local
        // ones would be satisfied by a constant.
        assert_ne!(
            receipt(1, None).peer_stable_checksum(),
            ActiveMatch::activated(3, None, Some(SessionScopeId(1)), Some(4_200))
                .peer_stable_checksum(),
            "a two-seat and a three-seat match share one checksum, so the seat \
             count is not reaching the projection"
        );
        // ⛔⛤ AND THE ACTIVATION TICK IS THE SAME KIND OF TERM, which this arm
        // asserted the OPPOSITE of until 2026-09-15. It counts this App's sim
        // steps including menu frames, so two hosts that reached the same lobby
        // by different routes stamp one match differently.
        assert_eq!(
            receipt(1, None).peer_stable_checksum(),
            ActiveMatch::activated(2, None, Some(SessionScopeId(1)), Some(9_900))
                .peer_stable_checksum(),
            "the receipt's checksum moves with the ABSOLUTE sim tick the match \
             activated on"
        );
    }
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

        // ⛔⛔⛔ **THIS ASSERTION WAS `assert_ne!` UNTIL 2026-09-14, AND THE
        // REQUIREMENT IT PINNED WAS A DESYNC.** It read: *"two sessions whose
        // first match activated on the same tick share a context, so every
        // playthrough opens the same way."* True as stated — and the only thing
        // that could satisfy it was mixing in `SessionScopeId`, a PER-APP
        // counter. The draws it seeds choose which item spawns and where, so two
        // peers with different local session histories built different
        // authoritative worlds.
        //
        // ⇒ **THE REQUIREMENT IS RETRACTED, NOT THE TEST.** What replaces it is
        // the property that actually has to hold: **the local session scope must
        // not change the draw at all.** Cross-run variety was real and is a real
        // loss (see `random_context`), but it was being bought with a value no
        // two peers can be relied on to agree about.
        //
        // ⚠ A requirement can be falsified by a RULING rather than by an edit,
        // and the honest response is to say which requirement died.
        let next_session = ActiveMatch::activated(2, None, Some(SessionScopeId(1)), Some(100));
        assert_eq!(
            a,
            next_session.instance().random_context(),
            "the LOCAL session scope changed the draw context. It is minted by a \
             per-App counter, so two peers whose Apps have activated a different \
             number of sessions would spawn different items at different points \
             from the same match state"
        );

        // ⛔⛤ **AND THE DRAWS THEMSELVES AGREE, not only the context number.** A
        // context equality alone would pass if `random_context` returned a
        // constant, which is a different defect wearing the same green.
        assert_eq!(
            drew(a),
            drew(next_session.instance().random_context()),
            "the contexts compare equal and the draws do not, so the comparison \
             is not measuring what the item spawner actually consumes"
        );
        // ⛔ AND THE CONTROL: those draws must not be a constant, or the
        // assertion above holds for a seed that carries no information.
        assert!(
            drew(a).iter().collect::<std::collections::BTreeSet<_>>().len() > 1,
            "the draw sequence is constant, so every equality assertion in this \
             arm holds for a context that seeds nothing"
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
    /// What two PEERS may compare about this receipt.
    ///
    /// `session` is a per-App activation count and `seat_topology` is a LOCAL
    /// device-topology generation that moves when a host re-captures an
    /// identical set of seats — neither is mechanical identity, so neither may
    /// enter a checksum. The seat COUNT and the activation tick are peer-stable.
    pub fn peer_stable_checksum(&self) -> u64 {
        let mut bytes = Vec::with_capacity(8);
        bytes.extend_from_slice(&(self.seats as u64).to_le_bytes());
        ambition_platformer2d_core::snapshot::checksum_bytes(&bytes)
    }

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
