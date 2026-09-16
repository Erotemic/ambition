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
    /// ⭐⭐ **WHICH MATCH OF THIS SESSION THIS IS — THE ONE ACTIVATION FACT TWO
    /// PEERS AGREE ON.** It starts at zero for everyone who joins a session
    /// together and counts up as they activate matches together, so it is
    /// insensitive to how long either App has been running, how many menus it
    /// sat in, and how many sessions it played before. `session` and
    /// `activated_on` are both per-App counts and cannot do this job; see
    /// `MatchInstance::activation_tick`.
    ///
    /// `None` in a composition with no ordinal authority, which answers
    /// `CONTEXT_UNSEEDED` — honest for a bare fixture with no identity to draw
    /// against.
    ordinal: Option<u64>,
}

/// Mints the peer-agreed ordinal above, one per match activation, restarting at
/// zero whenever the session changes.
///
/// ⭐ RESETTING ON THE SESSION IS WHAT MAKES IT PEER-AGREED. Two Apps that join
/// one session both begin at zero regardless of what they did before, and both
/// increment on the same activations. ⛔ It is NOT a global match counter: a
/// host that played five matches alone and then joins you must start at zero,
/// or its ordinals are its own history again.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionMatchOrdinal {
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    next: u64,
}

impl SessionMatchOrdinal {
    /// The ordinal for a match activating now in `session`, consuming it.
    pub fn take(
        &mut self,
        session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    ) -> u64 {
        if self.session != session {
            self.session = session;
            self.next = 0;
        }
        let ordinal = self.next;
        self.next += 1;
        ordinal
    }

    /// ⭐⭐ **WHAT A PEER COMPARES: HOW MANY MATCHES THIS SESSION HAS ACTIVATED.**
    /// Not the session id — that is a per-App activation count, and two peers in
    /// one agreed session hold different ones by construction.
    ///
    /// ⛔⛤ THIS RESOURCE WAS REGISTERED `rollback_resource_canonical` — a
    /// WHOLE-VALUE checksum — while the comment beside the registration claimed
    /// the session half "is compared only against ITSELF". The comment described
    /// `take`'s behaviour; the registration compared the field. Both halves were
    /// in the peer checksum. Found by the GPT architecture review of 2026-09-15,
    /// in the same commit that introduced the type.
    ///
    /// ⚠ ONE DIVERGENCE WINDOW SURVIVES THIS PROJECTION, and it is recorded
    /// rather than papered over: the mint resets LAZILY, inside `take`, so
    /// between joining a session and activating that session's first match it
    /// still holds the PREVIOUS session's `next`. Two peers with different prior
    /// match counts disagree for exactly that window.
    /// `two_peers_who_played_different_prior_matches_disagree_before_the_first_activation`
    /// holds it. Closing it means making the mint session-OWNED state — a
    /// `MatchOrdinalMint` under the session root, which starts at zero because a
    /// new session's state is new — instead of an App-global resource carrying an
    /// owner tag. That is the review's recommendation and the right shape; it is
    /// a carve, not a checksum change.
    pub fn peer_stable_checksum(&self) -> u64 {
        self.next
    }

    /// The two facts, for the wire format.
    #[doc(hidden)]
    pub fn parts(
        &self,
    ) -> (
        Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
        u64,
    ) {
        (self.session, self.next)
    }

    #[doc(hidden)]
    pub fn from_snapshot(
        session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
        next: u64,
    ) -> Self {
        Self { session, next }
    }
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
            ActiveMatch::activated(2, topology, Some(SessionScopeId(session)), Some(4_200), None)
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
            ActiveMatch::activated(3, None, Some(SessionScopeId(1)), Some(4_200), None)
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
            ActiveMatch::activated(2, None, Some(SessionScopeId(1)), Some(9_900), None)
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
    /// ⭐⭐ **THE ORDINAL IS PEER-AGREED BECAUSE IT RESTARTS ON THE SESSION.**
    ///
    /// This is the property the whole ID-PEER substitution rests on, so it is
    /// asserted rather than argued: a host that has already played matches must
    /// produce the SAME ordinals as a fresh host once they are in one session
    /// together.
    #[test]
    fn a_host_with_prior_matches_still_starts_a_new_session_at_zero() {
        // ⛔⛤ THE TWO HOSTS NAME THE AGREED SESSION WITH DIFFERENT LOCAL IDS,
        // and that is the entire ID-PEER premise. This arm used to hand both of
        // them `SessionScopeId(9)` — a shared local id, which is the one thing
        // two peers never have — so it could not have caught a mint that keyed
        // on the scope's VALUE. Found by the GPT architecture review of
        // 2026-09-15.
        let agreed_on_a = Some(SessionScopeId(9));
        let agreed_on_b = Some(SessionScopeId(41));

        // Host A: played three matches in an earlier session, then joins.
        let mut veteran = SessionMatchOrdinal::default();
        let earlier = Some(SessionScopeId(2));
        for _ in 0..3 {
            veteran.take(earlier);
        }
        let veteran_draws: Vec<u64> = (0..3).map(|_| veteran.take(agreed_on_a)).collect();

        // Host B: fresh App, joins the same session under its OWN local id.
        let mut fresh = SessionMatchOrdinal::default();
        let fresh_draws: Vec<u64> = (0..3).map(|_| fresh.take(agreed_on_b)).collect();

        assert_eq!(
            veteran_draws, fresh_draws,
            "a host with prior matches mints different ordinals in an agreed \
             session, so the two peers draw different items from the same match"
        );
        // ⛔ AND THEY MUST STILL COUNT, or the equality above is satisfied by a
        // constant — which is the defect this replaced, one layer over.
        assert_eq!(
            fresh_draws,
            vec![0, 1, 2],
            "consecutive matches in one session share an ordinal, so the second \
             replays the first's items"
        );
    }

    /// ⭐⭐ **THE PEER PROJECTION AGREES ONCE BOTH HOSTS HAVE ACTIVATED, WITH
    /// DIFFERENT LOCAL SESSION IDS AND DIFFERENT PRIOR HISTORIES.**
    ///
    /// This is the arm that would have caught the whole-value registration: under
    /// `rollback_resource_canonical` the checksum included `session.0`, so 9 and
    /// 41 disagreed forever no matter what the ordinals did.
    #[test]
    fn the_peer_projection_ignores_the_local_session_id_once_a_match_has_activated() {
        let mut veteran = SessionMatchOrdinal::default();
        for _ in 0..3 {
            veteran.take(Some(SessionScopeId(2)));
        }
        veteran.take(Some(SessionScopeId(9)));

        let mut fresh = SessionMatchOrdinal::default();
        fresh.take(Some(SessionScopeId(41)));

        assert_eq!(
            veteran.peer_stable_checksum(),
            fresh.peer_stable_checksum(),
            "two peers who activated the same first match of an agreed session \
             still checksum differently, so they desync on a value they agree \
             about"
        );
        // ⛔ AND THE PROJECTION MUST STILL COUNT. A checksum that ignored `next`
        // too would satisfy the arm above and compare nothing.
        let mut second = fresh;
        second.take(Some(SessionScopeId(41)));
        assert_ne!(
            fresh.peer_stable_checksum(),
            second.peer_stable_checksum(),
            "activating a second match does not change the projection, so a peer \
             that missed an activation agrees with one that did not"
        );
    }

    /// ⛔⛤ **THE ONE DIVERGENCE WINDOW THE PROJECTION DOES NOT CLOSE, HELD BY A
    /// TEST RATHER THAN BY PROSE.**
    ///
    /// The mint resets LAZILY, inside `take`. Between joining a session and
    /// activating that session's first match it still holds the PREVIOUS
    /// session's count, so two peers with different prior match counts disagree
    /// for exactly that window — and rollback checksums are compared every frame,
    /// not only after an activation.
    ///
    /// ⇒ WHEN THIS ARM FLIPS TO `assert_eq`, the mint has become session-OWNED
    /// state (a `MatchOrdinalMint` under the session root, which starts at zero
    /// because a new session's state is new) and `SessionMatchOrdinal` should
    /// leave `RECORDED_DIVERGENCE` in `game/ambition_app/tests/id_peer_audit.rs`.
    #[test]
    fn two_peers_who_played_different_prior_matches_disagree_before_the_first_activation() {
        let mut veteran = SessionMatchOrdinal::default();
        for _ in 0..3 {
            veteran.take(Some(SessionScopeId(2)));
        }
        let fresh = SessionMatchOrdinal::default();

        assert_ne!(
            veteran.peer_stable_checksum(),
            fresh.peer_stable_checksum(),
            "the lazy-reset window has closed — if that is deliberate, flip this \
             arm to assert_eq and drop SessionMatchOrdinal from \
             RECORDED_DIVERGENCE in game/ambition_app/tests/id_peer_audit.rs"
        );
    }

    /// ⛔ AND THE RESET IS ON THE SESSION, NOT ON EVERY CALL.
    #[test]
    fn the_ordinal_only_restarts_when_the_session_changes() {
        let mut mint = SessionMatchOrdinal::default();
        let one = Some(SessionScopeId(1));
        assert_eq!(mint.take(one), 0);
        assert_eq!(mint.take(one), 1);
        assert_eq!(mint.take(Some(SessionScopeId(2))), 0, "a new session restarts");
        assert_eq!(mint.take(Some(SessionScopeId(2))), 1);
        assert_eq!(mint.take(one), 0, "returning to an earlier session restarts too");
        // A composition with no session lifecycle still counts its matches.
        let mut sessionless = SessionMatchOrdinal::default();
        assert_eq!(sessionless.take(None), 0);
        assert_eq!(sessionless.take(None), 1);
    }

    #[test]
    fn two_activations_are_two_draw_contexts() {
        // Built the way activation builds them, not by hand: consecutive matches
        // in one session take consecutive ordinals.
        let first = ActiveMatch::activated(2, None, Some(SessionScopeId(0)), Some(100), Some(0));
        let second = ActiveMatch::activated(2, None, Some(SessionScopeId(0)), Some(900), Some(1));

        let a = first.random_context();
        let b = second.random_context();
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
        let next_session =
            ActiveMatch::activated(2, None, Some(SessionScopeId(1)), Some(100), Some(0));
        assert_eq!(
            a,
            next_session.random_context(),
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
            drew(next_session.random_context()),
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
        let bare = ActiveMatch::activated(2, None, None, None, None);
        assert_eq!(bare.random_context(), CONTEXT_UNSEEDED);

        // ⛔⛤ AND THE ACTIVATION TICK MUST NOT REACH THE DRAW AT ALL. It counts
        // this App's sim steps, menus included, so two peers who reached one
        // lobby by different routes would draw different items from the same
        // match. This assertion FAILED before 2026-09-15.
        let same_match_later_host =
            ActiveMatch::activated(2, None, Some(SessionScopeId(0)), Some(999_999), Some(0));
        assert_eq!(
            a,
            same_match_later_host.random_context(),
            "the draw context moves with the ABSOLUTE activation tick, so a host \
             that sat longer in menus spawns a different item table"
        );
        assert_eq!(
            drew(a),
            drew(same_match_later_host.random_context()),
            "the contexts compare equal and the draws do not"
        );
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
        let here = ActiveMatch::activated(2, None, Some(SessionScopeId(0)), Some(100), None);
        let same = MatchScoped(here.instance());
        assert!(
            same.belongs_to(Some(&here)),
            "an object stamped by the running match did not belong to it"
        );

        let elsewhere = ActiveMatch::activated(2, None, Some(SessionScopeId(1)), Some(100), None);
        assert!(
            !same.belongs_to(Some(&elsewhere)),
            "an object from another SESSION belonged to this match because the \
             activation ticks matched"
        );

        let later = ActiveMatch::activated(2, None, Some(SessionScopeId(0)), Some(900), None);
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
        ordinal: Option<u64>,
    ) -> Self {
        Self {
            seats,
            seat_topology,
            session,
            activated_on,
            ordinal,
        }
    }

    /// Which match of this session this is. See the field.
    pub fn ordinal(&self) -> Option<u64> {
        self.ordinal
    }

    /// ⭐⭐ THE DRAW CONTEXT FOR THIS MATCH, and the reason the ordinal exists.
    ///
    /// ⛔⛤ IT USED TO BE THE ACTIVATION TICK, AND THAT WAS A DESYNC. `SimTick`
    /// counts every sim step the App has run, menus included, and is never
    /// rebased — so two peers who reached one lobby by different routes drew
    /// DIFFERENT ITEMS from the same match state. Before that it was the raw
    /// `SessionScopeId`, which was the same defect one layer up. The ordinal is
    /// the first term here that both peers actually agree on.
    ///
    /// ⚠ Consecutive matches in a session still draw differently, which is the
    /// property the tick was there for: the ordinal increments.
    pub fn random_context(&self) -> ambition_platformer2d_core::sim_random::RandomContext {
        match self.ordinal {
            None => ambition_platformer2d_core::sim_random::CONTEXT_UNSEEDED,
            Some(ordinal) => ordinal
                .wrapping_add(1)
                .wrapping_mul(0xD6E8_FEB8_6659_FD93),
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
            // No ordinal authority: a bare fixture draws from CONTEXT_UNSEEDED.
            ordinal: None,
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
        ordinal: Option<u64>,
    ) -> Self {
        Self {
            seats,
            seat_topology,
            session,
            activated_on,
            ordinal,
        }
    }
}
