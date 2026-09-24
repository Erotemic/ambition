//! Match-seat binding and the rollback-safe receipt for a live prepared match.

use bevy::prelude::*;

/// Stable roster seat carried by the fighter body. Driving participant,
/// character id, and entity order cannot replace seat identity.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchSeat(pub usize);

// A ruleset can spawn a stand-in entity to own a delayed attack (for example a
// fused mark) after its author lost the last stock. The stand-in has no
// [`MatchSeat`], so [`match_participants`] does not count it. Attribution uses
// `Entity` (`HitEvent::attacker`), not a seat label.

/// Live fighter entities from rollback-restored [`MatchSeat`] components,
/// sorted by seat. No resource stores live entity handles for the cast.
pub fn match_participants(seated: &Query<(Entity, &MatchSeat)>) -> Vec<Entity> {
    let mut by_seat: Vec<(usize, Entity)> = seated
        .iter()
        .map(|(entity, seat)| (seat.0, entity))
        .collect();
    by_seat.sort_by_key(|(seat, _)| *seat);
    by_seat.into_iter().map(|(_, entity)| entity).collect()
}

/// Receipt for a fully activated match. Stores activation facts only; the live
/// cast is derived from [`MatchSeat`] components.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ActiveMatch {
    /// How many seats this match activated with. Compare it against
    /// [`match_participants`] to ask whether the cast is still whole.
    seats: usize,
    /// The frozen seat topology this match was activated against, copied from
    /// the roster so the two can be compared.
    seat_topology: Option<u64>,
    /// Whose plan this is a receipt for. `None` in a composition with no
    /// session lifecycle, the same value `PreparedMatch` stamps there.
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    /// Sim tick of activation, used to derive the opening-ceremony phase
    /// without a mutable timer. `None` when there is no sim clock.
    activated_on: Option<u64>,
    /// Which match of this session this is: the one activation fact two peers
    /// agree on. It starts at zero for everyone who joins a session together
    /// and counts up as they activate matches, so App uptime and history do
    /// not change it. `session` and `activated_on` are per-App counts; see
    /// `MatchInstance::activation_tick`.
    ///
    /// `None` when there is no ordinal authority (answers `CONTEXT_UNSEEDED`).
    ordinal: Option<u64>,
}

/// Mints the peer-agreed ordinal above, one per match activation, restarting
/// at zero when the session changes.
///
/// The reset on session change makes it peer-agreed: two Apps that join one
/// session start at zero and increment on the same activations. It is not a
/// global match counter.
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

    /// What a peer compares: how many matches this session has activated. Not
    /// the session id, which is a per-App count.
    ///
    /// Only `next` is in the checksum; the resource is registered with this
    /// projection, not as a whole-value checksum.
    ///
    /// `take` resets lazily. Alone, that leaves a window between joining a
    /// session and its first activation where the mint still holds the
    /// previous session's `next`, so peers with different histories disagree.
    /// `two_peers_who_played_different_prior_matches_disagree_before_the_first_activation`
    /// covers this type alone. Shell-host compositions close the window by
    /// resetting this resource eagerly at the activation edge, with the three
    /// match-stamped mirrors. A single-session composition has no previous
    /// count.
    pub fn peer_stable_checksum(&self) -> u64 {
        ambition_platformer2d_core::snapshot::PeerDigest::in_domain("match.ordinal_mint")
            .u64(self.next)
            .finish()
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

/// This entity belongs to the match that created it, and dies with it.
///
/// Objects a ruleset spawns (bomb, bolt, mine, portal, spring) otherwise end
/// only by their own rule (fuse, trigger, lifetime), so they could outlive
/// the match. Stamp once at spawn and sweep once by the match owner, instead
/// of a despawn in each spawner or a sweep that knows every component type.
/// Same idea as `StocksMatchSettled` and `SuddenDeathEntered`, on an entity.
///
/// The sweep belongs to the ruleset. `ambition_match` only records ownership.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MatchScoped(pub MatchInstance);

impl MatchScoped {
    /// Whether this object is part of the running match.
    ///
    /// No active match answers `false`: between matches an object has nothing
    /// to belong to.
    pub fn belongs_to(&self, active: Option<&ActiveMatch>) -> bool {
        active.is_some_and(|active| active.instance() == self.0)
    }
}

/// Stable activation identity used by ruleset-local per-match state.
///
/// It has two halves. The local half (`session`, `activated_on`) tells one
/// match from the next on one machine, for `belongs_to` and the settlement
/// staleness checks. The peer half (`ordinal`) is which match of the agreed
/// session this is, and only it may enter a checksum. Without the peer half,
/// two peers could hold the same verdict for different matches and checksum
/// the same while simulating differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MatchInstance {
    /// The gameplay session the cast was built in. Local.
    session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    /// The sim tick it was built on. Local; see [`MatchInstance::activation_tick`].
    activated_on: Option<u64>,
    /// Which match of the agreed session: the term two peers share. From
    /// `SessionMatchOrdinal`.
    ///
    /// `None` with no ordinal authority. Two `None`s compare equal; a fixture
    /// with no identity cannot tell its matches apart.
    ordinal: Option<u64>,
}

impl MatchInstance {
    /// The three facts, for the wire format. See `snapshot_impls`.
    #[doc(hidden)]
    pub fn parts(
        &self,
    ) -> (
        Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
        Option<u64>,
        Option<u64>,
    ) {
        (self.session, self.activated_on, self.ordinal)
    }

    /// Which match of the agreed session this is: the only term a peer
    /// checksum may read. Every projection over a `MatchInstance` uses only
    /// this.
    pub fn peer_match_id(&self) -> Option<u64> {
        self.ordinal
    }

    /// The peer half as checksum bytes: a presence tag and the ordinal.
    ///
    /// Tagged, so "no ordinal authority" and "ordinal 0" differ. Otherwise a
    /// bare fixture would agree with the first match of a real session.
    pub fn peer_match_digest(&self) -> u64 {
        ambition_platformer2d_core::snapshot::PeerDigest::in_domain("match.instance")
            .opt_u64(self.ordinal)
            .finish()
    }

    /// Local activation stamp. Not peer-stable.
    ///
    /// `SimTick` has one writer (`ambition_time::advance_sim_tick`), is never
    /// rebased, and advances in menus and while gameplay is suspended. So this
    /// is the number of sim steps the App has run, and two hosts disagree on
    /// it. Use it only to tell matches apart on one machine (`belongs_to`,
    /// settlement staleness). Nothing compared between peers may read it.
    pub fn activation_tick(&self) -> Option<u64> {
        self.activated_on
    }

    /// Rebuild a present activation from rollback state. Resource snapshotting
    /// restores absence separately.
    #[doc(hidden)]
    pub fn from_snapshot(
        session: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
        activated_on: Option<u64>,
        ordinal: Option<u64>,
    ) -> Self {
        Self {
            session,
            activated_on,
            ordinal,
        }
    }
}

#[cfg(test)]
mod match_context_tests {

    /// The projection makes `ActiveMatch` peer-safe. The registration kind
    /// only says a projection exists; this test says what it excludes.
    #[test]
    fn the_peer_stable_checksum_ignores_session_and_seat_topology() {
        use ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId;

        // The ordinal is fixed at match 3 while the local halves vary: two
        // peers describing one match of one session.
        let receipt = |session: u64, topology: Option<u64>| {
            ActiveMatch::activated(
                2,
                topology,
                Some(SessionScopeId(session)),
                Some(4_200),
                Some(3),
            )
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
        // It must see which match of the session: seat count alone is
        // peer-stable but not identifying.
        assert_ne!(
            receipt(1, None).peer_stable_checksum(),
            ActiveMatch::activated(2, None, Some(SessionScopeId(1)), Some(4_200), Some(4))
                .peer_stable_checksum(),
            "a receipt for match 3 and one for match 4 with the same seating \
             share one checksum"
        );
        // An absent ordinal is not match zero.
        assert_ne!(
            ActiveMatch::activated(2, None, Some(SessionScopeId(1)), Some(4_200), Some(0))
                .peer_stable_checksum(),
            ActiveMatch::activated(2, None, Some(SessionScopeId(1)), Some(4_200), None)
                .peer_stable_checksum(),
            "an absent ordinal projects as ordinal 0"
        );
        // It must still see mechanical facts, or a constant would pass.
        assert_ne!(
            receipt(1, None).peer_stable_checksum(),
            ActiveMatch::activated(3, None, Some(SessionScopeId(1)), Some(4_200), Some(3))
                .peer_stable_checksum(),
            "a two-seat and a three-seat match share one checksum, so the seat \
             count is not reaching the projection"
        );
        // The activation tick must not reach the projection: it counts this
        // App's sim steps, menus included, so hosts on different routes differ.
        assert_eq!(
            receipt(1, None).peer_stable_checksum(),
            ActiveMatch::activated(2, None, Some(SessionScopeId(1)), Some(9_900), Some(3))
                .peer_stable_checksum(),
            "the receipt's checksum moves with the ABSOLUTE sim tick the match \
             activated on"
        );
    }
    use super::*;
    use ambition_platformer2d_core::sim_random::{sim_random, CONTEXT_UNSEEDED, DOMAIN_ITEM_SPAWN};
    use ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId;

    /// The ordinal is peer-agreed because it restarts on the session. A host
    /// that already played matches must produce the same ordinals as a fresh
    /// host once they share a session.
    #[test]
    fn a_host_with_prior_matches_still_starts_a_new_session_at_zero() {
        // The two hosts name the agreed session with different local ids, as
        // real peers do. A shared id could not catch a mint keyed on the id's
        // value.
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
        // They must still count, or a constant would pass the check above.
        assert_eq!(
            fresh_draws,
            vec![0, 1, 2],
            "consecutive matches in one session share an ordinal, so the second \
             replays the first's items"
        );
    }

    /// The peer projection agrees once both hosts have activated, with
    /// different local session ids and different histories. A whole-value
    /// checksum that included `session.0` fails here.
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
        // The projection must still count; ignoring `next` would compare
        // nothing.
        let mut second = fresh;
        second.take(Some(SessionScopeId(41)));
        assert_ne!(
            fresh.peer_stable_checksum(),
            second.peer_stable_checksum(),
            "activating a second match does not change the projection, so a peer \
             that missed an activation agrees with one that did not"
        );
    }

    /// The one divergence window the projection does not close.
    ///
    /// The mint resets lazily inside `take`. Between joining a session and its
    /// first activation it holds the previous session's count, so peers with
    /// different histories disagree, and rollback compares checksums every
    /// frame. The composition closes this with an eager reset at the
    /// activation edge.
    ///
    /// If this flips to `assert_eq`, the mint has become session-owned state,
    /// and `SessionMatchOrdinal` should leave the ID-PEER audit's
    /// `RECORDED_DIVERGENCE` list. That list lives in the composition crate;
    /// `engine.ambition_match-source-purity` forbids naming it here, so search
    /// for it by name.
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
            "this type's lazy reset no longer leaves a window, so the eager \
             reset at the session activation edge is no longer load-bearing and \
             the composition may stop performing it"
        );
    }

    /// The reset is on session change, not on every call.
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

    /// Two matches are two runs of the world and must not draw alike. Every
    /// consumer keys on a match clock that restarts at zero, so without a
    /// per-match context match two would repeat match one's items.
    ///
    /// This covers the seam only. `PreparedMatch` has no constructor outside
    /// `prepare_match`, so no test here can drive `spawn_match_items`. It
    /// checks that two production activations give different contexts and
    /// that those contexts separate the draws.
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

        // The raw draws separate too. A reduced index could hide a shared
        // context behind a modulus.
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

        // The local session scope must not change the draw. `SessionScopeId`
        // is a per-App counter, and the draws choose which item spawns where,
        // so mixing it in would make peers build different worlds. Cross-run
        // variety between sessions is given up for this (see
        // `random_context`).
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

        // The draws agree too, not only the context number; a constant
        // context would pass that check alone.
        assert_eq!(
            drew(a),
            drew(next_session.random_context()),
            "the contexts compare equal and the draws do not, so the comparison \
             is not measuring what the item spawner actually consumes"
        );
        // Control: the draws are not constant, or the checks above prove
        // nothing.
        assert!(
            drew(a).iter().collect::<std::collections::BTreeSet<_>>().len() > 1,
            "the draw sequence is constant, so every equality assertion in this \
             arm holds for a context that seeds nothing"
        );

        // A match with no identity has no context and says so.
        let bare = ActiveMatch::activated(2, None, None, None, None);
        assert_eq!(bare.random_context(), CONTEXT_UNSEEDED);

        // The activation tick must not reach the draw: it counts this App's
        // sim steps, menus included.
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
    /// A new session is a new match even at the same activation tick. A fresh
    /// session starts its clock at zero, so a tick-only comparison would keep
    /// the previous session's objects. `MatchScoped::belongs_to` checks both.
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

        // No active match means nothing belongs (the select screen).
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

    /// The draw context for this match.
    ///
    /// Uses the ordinal, the first term both peers agree on. The activation
    /// tick and `SessionScopeId` are per-App counts, so peers would draw
    /// different items from the same state. Consecutive matches in a session
    /// still draw differently because the ordinal increments.
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
            ordinal: self.ordinal,
        }
    }

    /// What two peers may compare about this receipt: the agreed seat count and
    /// which match of the agreed session it is.
    ///
    /// `session` is a per-App count, and `seat_topology` is a local device
    /// generation that moves when a host re-captures the same seats. Neither
    /// is mechanical identity, so neither enters a checksum. The seat count
    /// alone is not identifying; the match digest is required.
    pub fn peer_stable_checksum(&self) -> u64 {
        ambition_platformer2d_core::snapshot::PeerDigest::in_domain("match.active_receipt")
            .u64(self.seats as u64)
            .u64(self.instance().peer_match_digest())
            .finish()
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
    /// Registering this is correct because `bevy_ggrs` restores absence:
    /// `ResourceSnapshotPlugin::load` maps `(Some(_), None)` to
    /// `remove_resource`.
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
