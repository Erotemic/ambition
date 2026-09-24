//! Rollback-safe match settlement state.
//!
//! These facts describe one `MatchInstance`, not a particular stocks ruleset.
//! Rules decide and enter sudden death from above. Match identity owns the
//! stamped state, so clocks, presentation, and other consumers do not depend
//! on the actor monolith's rules module.

use bevy::prelude::{Resource, World};
use ambition_combat::stocks::MatchVerdict;
use crate::{ActiveMatch, MatchInstance};

/// The stocks outcome for one match: which match was settled, and how.
///
/// Set once a stocks ruleset decides the live match, so the outcome is
/// announced once.
///
/// Stamped with the match it is about, so it goes stale automatically when a
/// different match activates. Nobody retracts it, and nothing is ordered
/// against activation.
///
/// A rollback resource, not a `Local`: it gates a message the ruleset acts
/// on, so a rewind across the deciding frame must un-decide the match. It
/// rewinds with [`ActiveMatch`], so the comparison stays correct.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct StocksMatchSettled(Option<(MatchInstance, MatchVerdict)>);

/// Hash a mechanical fact plus a match discriminant, for the checksum
/// projections below.
///
/// Only `MatchInstance`'s peer half reaches this, through
/// `peer_match_digest`. Its local terms are per-App counts (see
/// `MatchInstance::activation_tick`). `which` is required: without it, the
/// same verdict for a different match checksums the same while
/// `settled(active)` differs.
fn peer_stable_digest(domain: &str, which: u64, extra: u64) -> u64 {
    ambition_platformer2d_core::snapshot::PeerDigest::in_domain(domain)
        .u64(which)
        .u64(extra)
        .finish()
}

impl StocksMatchSettled {
    /// What two peers may compare about this verdict: which match was decided
    /// (peer ordinal) and how. The verdict alone is not enough; a verdict for
    /// the previous match would checksum the same as one for this match.
    pub fn peer_stable_checksum(&self) -> u64 {
        let which = match &self.0 {
            None => 0,
            Some((instance, _)) => instance.peer_match_digest(),
        };
        let verdict = match &self.0 {
            None => 0,
            Some((_local_stamp, verdict)) => {
                match verdict {
                    ambition_combat::stocks::MatchVerdict::Draw => 1,
                    ambition_combat::stocks::MatchVerdict::NoContest => 2,
                    ambition_combat::stocks::MatchVerdict::Winner(side) => {
                        3 ^ (ambition_platformer2d_core::snapshot::checksum_bytes(side.as_bytes())
                            << 8)
                    }
                }
            }
        };
        peer_stable_digest("match.stocks_verdict", which, verdict)
    }

    /// Whether this match was decided. A verdict for a different match does
    /// not count.
    pub fn settled(&self, active: &ActiveMatch) -> bool {
        self.decided_match() == Some(active.instance())
    }

    /// Record that this match was decided, and how.
    ///
    /// The verdict is state here because presentation must not read a
    /// message: a speculative frame can write `StocksMatchDecided`, and the
    /// winner card cannot retract. This latch rewinds. Waiting for
    /// confirmation on a message also fails: the channel is two frames deep,
    /// so a late confirmation loses the message. State has no cursor.
    pub fn settle(&mut self, active: &ActiveMatch, verdict: MatchVerdict) {
        self.0 = Some((active.instance(), verdict));
    }

    /// How this match ended, or `None` if it is not decided.
    pub fn verdict(&self, active: &ActiveMatch) -> Option<&MatchVerdict> {
        self.0
            .as_ref()
            .filter(|(instance, _)| *instance == active.instance())
            .map(|(_, verdict)| verdict)
    }

    /// Rebuild from a rollback snapshot. See `snapshot_impls`.
    #[doc(hidden)]
    pub fn from_snapshot(decided: Option<(MatchInstance, MatchVerdict)>) -> Self {
        Self(decided)
    }

    /// The match this verdict is about, for the wire format. Not a "was
    /// anything decided" check; use [`Self::settled`] with the live match.
    #[doc(hidden)]
    pub fn decided_match(&self) -> Option<MatchInstance> {
        self.0.as_ref().map(|(instance, _)| instance.clone())
    }

    /// The verdict this latch holds, for the wire format.
    #[doc(hidden)]
    pub fn decided_verdict(&self) -> Option<&MatchVerdict> {
        self.0.as_ref().map(|(_, verdict)| verdict)
    }
}

/// Whether the live match was decided, for a caller that holds a world
/// instead of system parameters. A verdict means nothing unless it is for the
/// running match.
pub fn the_live_match_is_settled(world: &World) -> bool {
    match (
        world.get_resource::<ActiveMatch>(),
        world.get_resource::<StocksMatchSettled>(),
    ) {
        (Some(active), Some(settled)) => settled.settled(active),
        _ => false,
    }
}

/// The match entered sudden death, and which match.
///
/// Same stamped shape as [`StocksMatchSettled`]: a fact about match X goes
/// stale automatically when match Y activates.
///
/// This latch stops the clock from re-firing. Sudden death is entered by not
/// settling the match, so `time_expired` stays true. Without the latch, the
/// tie would re-enter every tick and reset every fighter's damage.
///
/// Rollback state for the same reason as its sibling: a rewind across the
/// entering frame must un-enter it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SuddenDeathEntered(Option<MatchInstance>);

impl SuddenDeathEntered {
    /// What two peers may compare: which match entered sudden death (peer
    /// ordinal) and whether anything is latched. One bit alone would let a
    /// latch for the previous match agree with one for this match.
    pub fn peer_stable_checksum(&self) -> u64 {
        let which = match &self.0 {
            None => 0,
            Some(instance) => instance.peer_match_digest(),
        };
        peer_stable_digest("match.sudden_death", which, u64::from(self.0.is_some()))
    }

    /// Whether this match is in sudden death.
    pub fn entered(&self, active: &ActiveMatch) -> bool {
        self.0 == Some(active.instance())
    }

    /// Record that this match has entered it.
    pub fn enter(&mut self, active: &ActiveMatch) {
        self.0 = Some(active.instance());
    }

    /// Which match, for the wire format. See `snapshot_impls`.
    #[doc(hidden)]
    pub fn entered_match(&self) -> Option<MatchInstance> {
        self.0
    }

    /// Rebuild from a rollback snapshot. See `snapshot_impls`.
    #[doc(hidden)]
    pub fn from_snapshot(entered: Option<MatchInstance>) -> Self {
        Self(entered)
    }
}

#[cfg(test)]
mod peer_stable_projection_tests {
    use super::*;
    use ambition_combat::stocks::MatchVerdict;
    use ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId;

    /// A stamp whose local halves vary and whose peer half is fixed at match
    /// 3: two peers describing the same match of one agreed session.
    fn stamp(session: u64, tick: u64) -> MatchInstance {
        MatchInstance::from_snapshot(Some(SessionScopeId(session)), Some(tick), Some(3))
    }

    /// The same match, named by its peer ordinal, with local halves a peer could
    /// never share.
    fn peer_match(ordinal: u64) -> MatchInstance {
        MatchInstance::from_snapshot(Some(SessionScopeId(77)), Some(123_456), Some(ordinal))
    }

    /// Same payload, different match. Two peers can hold the same verdict
    /// where one stamp is for the current match and the other for the
    /// previous. `settled(active)` then differs, so the checksums must differ.
    /// A checksum that agrees while the simulation diverges hides a desync.
    #[test]
    fn the_same_verdict_for_a_different_match_is_a_different_checksum() {
        let verdict_for = |ordinal: u64| {
            StocksMatchSettled::from_snapshot(Some((
                peer_match(ordinal),
                MatchVerdict::Winner("left".to_string()),
            )))
            .peer_stable_checksum()
        };
        assert_ne!(
            verdict_for(3),
            verdict_for(2),
            "the same verdict stamped for match 3 and for match 2 checksums \
             identically, so a peer holding a STALE verdict agrees with one \
             holding the live one while `settled(active)` disagrees"
        );
        // "No ordinal authority" is not match zero.
        assert_ne!(
            verdict_for(0),
            StocksMatchSettled::from_snapshot(Some((
                MatchInstance::from_snapshot(Some(SessionScopeId(77)), Some(123_456), None),
                MatchVerdict::Winner("left".to_string()),
            )))
            .peer_stable_checksum(),
            "an absent ordinal projects as ordinal 0"
        );
        // The sudden-death latch has the same shape.
        assert_ne!(
            SuddenDeathEntered::from_snapshot(Some(peer_match(3))).peer_stable_checksum(),
            SuddenDeathEntered::from_snapshot(Some(peer_match(2))).peer_stable_checksum(),
            "sudden death latched for match 3 and for match 2 checksum \
             identically"
        );
        // The local halves must still be excluded: the same match, named by
        // peers with different session counts and ticks, must agree.
        assert_eq!(
            SuddenDeathEntered::from_snapshot(Some(MatchInstance::from_snapshot(
                Some(SessionScopeId(1)),
                Some(400),
                Some(3),
            )))
            .peer_stable_checksum(),
            SuddenDeathEntered::from_snapshot(Some(MatchInstance::from_snapshot(
                Some(SessionScopeId(9)),
                Some(999_999),
                Some(3),
            )))
            .peer_stable_checksum(),
            "two peers naming the SAME match of the agreed session disagree, so \
             the peer term was widened by re-admitting a local one"
        );
    }

    #[test]
    fn the_verdict_checksum_ignores_the_session_count() {
        let settled = |session: u64| {
            StocksMatchSettled::from_snapshot(Some((
                stamp(session, 4_200),
                MatchVerdict::Winner("left".to_string()),
            )))
        };
        assert_eq!(
            settled(1).peer_stable_checksum(),
            settled(9).peer_stable_checksum(),
            "the verdict's checksum moves with the host's prior session count, so \
             two peers who agree on the outcome would desync"
        );
        // The activation tick is also local: it counts this App's sim steps,
        // menus included.
        assert_eq!(
            settled(1).peer_stable_checksum(),
            StocksMatchSettled::from_snapshot(Some((
                MatchInstance::from_snapshot(Some(SessionScopeId(1)), Some(999_999), Some(3)),
                MatchVerdict::Winner("left".to_string()),
            )))
            .peer_stable_checksum(),
            "the verdict's checksum moves with the ABSOLUTE sim tick the match \
             activated on, which counts menu frames"
        );
        // It must still see the outcome, or a constant would pass.
        assert_ne!(
            settled(1).peer_stable_checksum(),
            StocksMatchSettled::from_snapshot(Some((
                stamp(1, 4_200),
                MatchVerdict::Winner("right".to_string()),
            )))
            .peer_stable_checksum(),
            "two different winners share one checksum, so the verdict is not \
             reaching the projection"
        );
        assert_ne!(
            settled(1).peer_stable_checksum(),
            StocksMatchSettled::from_snapshot(Some((
                stamp(1, 4_200),
                MatchVerdict::Draw,
            )))
            .peer_stable_checksum(),
            "a win and a draw share one checksum"
        );
        assert_ne!(
            settled(1).peer_stable_checksum(),
            StocksMatchSettled::from_snapshot(None).peer_stable_checksum(),
            "an undecided match and a decided one share one checksum"
        );
    }

    #[test]
    fn the_sudden_death_checksum_ignores_the_session_count() {
        let latched = |session: u64| SuddenDeathEntered::from_snapshot(Some(stamp(session, 4_200)));
        assert_eq!(
            latched(1).peer_stable_checksum(),
            latched(9).peer_stable_checksum(),
            "the sudden-death latch's checksum moves with the host's prior \
             session count"
        );
        assert_eq!(
            latched(1).peer_stable_checksum(),
            SuddenDeathEntered::from_snapshot(Some(stamp(1, 9_900))).peer_stable_checksum(),
            "the latch's checksum moves with the absolute activation tick"
        );
        assert_ne!(
            latched(1).peer_stable_checksum(),
            SuddenDeathEntered::from_snapshot(None).peer_stable_checksum(),
            "a latched match and an unlatched one share one checksum"
        );
    }

    // The two projections are checksummed into the same frame and must not
    // collide, or a swap between them would be invisible.
    #[test]
    fn the_two_settlement_projections_do_not_share_a_digest() {
        assert_ne!(
            StocksMatchSettled::from_snapshot(None).peer_stable_checksum(),
            SuddenDeathEntered::from_snapshot(None).peer_stable_checksum(),
            "the empty verdict and the empty latch hash identically"
        );
        assert_ne!(
            SuddenDeathEntered::from_snapshot(Some(stamp(1, 4_200))).peer_stable_checksum(),
            StocksMatchSettled::from_snapshot(Some((stamp(1, 4_200), MatchVerdict::Draw)))
                .peer_stable_checksum(),
            "a latch and a draw on the same match hash identically"
        );
    }
}
