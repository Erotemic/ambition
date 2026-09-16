//! Rollback-safe match settlement state.
//!
//! These facts describe one `MatchInstance`, not a particular stocks ruleset.
//! Rules decide and enter sudden death from above; match identity owns the stamped
//! state so clocks, presentation, and other consumers do not depend on the actor
//! monolith's rules module.

use bevy::prelude::{Resource, World};
use ambition_combat::stocks::MatchVerdict;
use crate::{ActiveMatch, MatchInstance};

/// THE STOCKS OUTCOME FOR ONE MATCH: which match has been settled.
///
/// Set once a stocks ruleset has decided the live match, so the
/// outcome is announced once rather than every tick after it becomes true.
///
/// this was a bare `bool` about the PROCESS, and is what that costs. A match that ended set
/// it true; nothing on this stage set it back, because the only retraction was
/// `decide_stocks_match` observing NO active match and there is no tick between two matches on
/// which the receipt is absent.
///
/// it is not a timeless global. It is the outcome for match X, and saying
/// so is the whole fix: a verdict stamped with the match it is about goes stale
/// BY CONSTRUCTION when a different match activates. Nobody retracts it, nothing
/// has to be ordered against activation, and a composition that never installed
/// this ruleset is not mentioned anywhere on the activation road.
///
/// still a resource rather than a `Local`, and still rollback state: a `Local`
/// does not rewind, and this gates a message the ruleset acts on, so a rewind
/// across the deciding frame must be able to un-decide the match. It rewinds
/// alongside [`ActiveMatch`], which is what makes the comparison below correct
/// after a rewind rather than merely plausible.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct StocksMatchSettled(Option<(MatchInstance, MatchVerdict)>);

/// Hash a mechanical fact plus a discriminant, for the checksum projections
/// below.
///
/// ⛔ ONLY `MatchInstance`'s PEER HALF REACHES THIS, through
/// `peer_match_digest`. Its local terms count something per-App — activations
/// for `session`, sim steps for `activated_on` — so neither can be compared
/// between peers. See `MatchInstance::activation_tick`.
///
/// ⛔⛤ AND `which` IS NOT OPTIONAL. These projections excluded the instance
/// ENTIRELY for a day, which made them false-negative: the same verdict stamped
/// for a different match checksummed identically while `settled(active)`
/// disagreed. A projection has to say WHICH match it describes, in the peer's
/// vocabulary.
fn peer_stable_digest(which: u64, tag: u64, extra: u64) -> u64 {
    let mut bytes = Vec::with_capacity(24);
    bytes.extend_from_slice(&which.to_le_bytes());
    bytes.extend_from_slice(&tag.to_le_bytes());
    bytes.extend_from_slice(&extra.to_le_bytes());
    ambition_platformer2d_core::snapshot::checksum_bytes(&bytes)
}

impl StocksMatchSettled {
    /// What two PEERS may compare about this verdict: WHICH match was decided,
    /// in the peer's vocabulary, and HOW.
    ///
    /// ⛔⛤ IT HASHED THE VERDICT ALONE for a day. A `Winner("left")` stamped for
    /// the PREVIOUS match then checksummed identically to one stamped for the
    /// current match — while `settled(active)` answered `false` on the first and
    /// `true` on the second, so the two peers ran different simulations from
    /// identical checksums.
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
        peer_stable_digest(which, 0x5700_0000_0000_0001, verdict)
    }

    /// Has THIS match been decided? A verdict for a different match is not
    /// this match's, which is the whole reason the stamp is here.
    pub fn settled(&self, active: &ActiveMatch) -> bool {
        self.decided_match() == Some(active.instance())
    }

    /// Record that this match has been decided, and HOW.
    ///
    /// ⭐⭐ THE VERDICT LIVES HERE BECAUSE PRESENTATION MAY NOT READ A MESSAGE.
    /// The winner card and the return countdown both reacted to
    /// `StocksMatchDecided`, which a SPECULATIVE frame can write — and neither
    /// is retractable. The countdown was fixed by reading this latch, which
    /// rewinds; the CARD could not follow because the latch said only WHETHER,
    /// and the outcome it needs was in the message.
    ///
    /// ⛔ AND WAITING FOR CONFIRMATION IS NOT ENOUGH ON A MESSAGE. A reader that
    /// declines to consume until the frame is confirmed keeps its cursor, and a
    /// message channel is two frames deep — so a confirmation arriving later
    /// than that loses the announcement rather than delaying it. State has no
    /// cursor.
    pub fn settle(&mut self, active: &ActiveMatch, verdict: MatchVerdict) {
        self.0 = Some((active.instance(), verdict));
    }

    /// How THIS match ended, or `None` for a match that has not been decided.
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

    /// The match this verdict is about, for the wire format. not a
    /// "has anything been decided" predicate — that question needs the live
    /// match to compare against, which is [`Self::settled`].
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

/// Has the LIVE match been decided? — both halves of the question, for a
/// caller holding a world rather than a system's parameters.
///
/// A latch with a verdict in it says nothing on its own; it has to be the verdict for the match
/// that is running.
pub fn the_live_match_is_settled(world: &World) -> bool {
    match (
        world.get_resource::<ActiveMatch>(),
        world.get_resource::<StocksMatchSettled>(),
    ) {
        (Some(active), Some(settled)) => settled.settled(active),
        _ => false,
    }
}

/// THE MATCH ENTERED SUDDEN DEATH, and WHICH match it is about.
///
/// ⭐ THE SAME STAMPED SHAPE AS [`StocksMatchSettled`], for the same reason and
/// with the same payoff: a fact about match X goes stale BY CONSTRUCTION when
/// match Y activates, so nobody has to retract it and nothing has to be ordered
/// against activation.
///
/// ⛔⛔ AND IT IS WHAT KEEPS THE CLOCK FROM RE-FIRING. Sudden death is entered by
/// NOT settling the match, so `time_expired` stays true for every tick that
/// follows — without this latch the tie would be re-entered sixty times a second
/// and every fighter would be reset to the starting damage forever.
///
/// Rollback state for the reason its sibling is: this gates a message the
/// ruleset acts on, so a rewind across the entering frame must be able to
/// un-enter it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SuddenDeathEntered(Option<MatchInstance>);

impl SuddenDeathEntered {
    /// What two PEERS may compare: WHICH match entered sudden death, in the
    /// peer's vocabulary, and whether anything is latched at all.
    ///
    /// ⛔⛤ IT HASHED ONLY `is_some()` for a day — one BIT — so a latch belonging
    /// to the previous match agreed with a latch belonging to this one.
    pub fn peer_stable_checksum(&self) -> u64 {
        let which = match &self.0 {
            None => 0,
            Some(instance) => instance.peer_match_digest(),
        };
        peer_stable_digest(which, 0x5D00_0000_0000_0002, u64::from(self.0.is_some()))
    }

    /// Is THIS match in sudden death?
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

    /// A stamp whose LOCAL halves vary and whose PEER half is fixed at match 3.
    /// Two peers describing the same match of the same agreed session look like
    /// this: the same ordinal, different session counts, different ticks.
    fn stamp(session: u64, tick: u64) -> MatchInstance {
        MatchInstance::from_snapshot(Some(SessionScopeId(session)), Some(tick), Some(3))
    }

    /// The same match, named by its peer ordinal, with local halves a peer could
    /// never share.
    fn peer_match(ordinal: u64) -> MatchInstance {
        MatchInstance::from_snapshot(Some(SessionScopeId(77)), Some(123_456), Some(ordinal))
    }

    /// ⛔⛤ **THE FALSE-NEGATIVE ARM: IDENTICAL PAYLOAD, DIFFERENT MATCH.**
    ///
    /// This is the case that made removing the local stamp insufficient. Two
    /// peers can hold the same verdict value where one's stamp belongs to the
    /// CURRENT match and the other's to the PREVIOUS one. `settled(active)` then
    /// answers `true` on one and `false` on the other — different simulation —
    /// and before 2026-09-15 their checksums were equal, because the projection
    /// hashed the verdict and nothing about which match it was for.
    ///
    /// ⇒ A checksum that agrees while the simulation diverges is worse than one
    /// that disagrees while it does not: the first hides a desync, the second
    /// only reports one.
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
        // ⚠ AND "NO ORDINAL AUTHORITY" IS NOT MATCH ZERO. A bare fixture with no
        // ordinal must not agree with the first match of a real session.
        assert_ne!(
            verdict_for(0),
            StocksMatchSettled::from_snapshot(Some((
                MatchInstance::from_snapshot(Some(SessionScopeId(77)), Some(123_456), None),
                MatchVerdict::Winner("left".to_string()),
            )))
            .peer_stable_checksum(),
            "an absent ordinal projects as ordinal 0"
        );
        // ⛔ AND THE SUDDEN-DEATH LATCH HAS THE SAME SHAPE, one bit wide, so it
        // was even easier to collide: a latch for match 2 agreed with a latch
        // for match 3.
        assert_ne!(
            SuddenDeathEntered::from_snapshot(Some(peer_match(3))).peer_stable_checksum(),
            SuddenDeathEntered::from_snapshot(Some(peer_match(2))).peer_stable_checksum(),
            "sudden death latched for match 3 and for match 2 checksum \
             identically"
        );
        // ⚠ AND THE LOCAL HALVES MUST STILL BE EXCLUDED, or this arm's fix is
        // the old defect returning: the SAME match named by two peers with
        // different session counts and different activation ticks must agree.
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
        // ⛔⛤ AND THE ACTIVATION TICK IS THE SAME KIND OF TERM. It counts this
        // App's sim steps, menus included, so two hosts that idled on the select
        // screen for different numbers of frames stamp the same match
        // differently. This arm FAILED before 2026-09-15.
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
        // ⛔ AND IT MUST STILL SEE THE OUTCOME, or the exclusion above would be
        // satisfied by a constant.
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

    // ⛔ The two projections must not collide: they are checksummed into the same
    // frame, and a swap between them would then be invisible.
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
