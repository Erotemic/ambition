//! Confirmed-frame lifecycle commitment (Track B, Piece 1).
//!
//! A room-lifecycle operation — a same-room reset (death / manual / replay), a
//! room transition, or a full sandbox reset — despawns and/or rebuilds the
//! authoritative room. Under a rollback host it must NOT run on a speculative
//! frame: the transition transaction machinery is not rollback-registered, so a
//! reconstruction executed on a predicted frame cannot resimulate identically and
//! the sync-test checksum diverges (see
//! `app_it::rollback_room_transition`).
//!
//! Instead of executing, the lifecycle consumer RECORDS a [`PendingLifecycleCommit`] and
//! returns. This resource is **rollback-registered state** — unlike [`crate`]'s external-effect
//! journal, whose consumers live outside the sim — so:
//!
//! * resimulation reproduces the intent deterministically (the consumer re-reads
//!   the same trigger and re-records the same intent);
//! * a corrected input that erases the trigger (the death never happened) rewinds
//!   the intent away with the rest of the world;
//! * repeated prediction cannot accumulate duplicates — it is idempotent STATE,
//!   not a command stream.
//!
//! A host-side system (Track B, Piece 2, `PreUpdate` after the GGRS advances,
//! gated on `ConfirmedFrameBoundary`) then executes the transaction in an
//! exclusive world once the originating frame can never be simulated again, and
//! **rebases the session** so no earlier snapshot can restore the pre-op room.
//!
//! It is no longer true of a transition: a crossing is described ONCE, here, on every host, and
//! the readiness transaction is its only consumer. What still differs is WHEN the intent may be
//! acted on — an eager host has no speculative frames, so it stamps frame `0` and its intent is
//! confirmed on arrival, while a rollback host stamps the recording frame and waits for it. Two
//! confirmation adapters, one description.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// Which room-lifecycle operation a deferred commit will perform.
///
/// Carries only deterministic, rollback-safe data — a reason discriminant and,
/// for a transition, the authored loading-zone id plus the rollback-stable
/// [`SimId`] of the body that triggered it. Never an `Entity`, a fn-pointer, or
/// anything whose value depends on map/query iteration order, so the enclosing
/// [`PendingLifecycleCommit`] can BE rollback state.
#[derive(Clone, Debug, PartialEq)]
pub enum LifecycleIntent {
    /// In-place same-room reset triggered by a player death.
    DeathReset,
    /// In-place same-room reset triggered by the manual reset input.
    ManualReset,
    /// In-place same-room replay of the current room.
    Replay,
    /// Reconstruction: transition into `target_room` (its authored id), placing
    /// the TRIGGERING body at `arrival`. `edge_exit` selects the transition
    /// cooldown/feel, mirroring `RoomTransitionApplication::apply`.
    ///
    /// `subject` is the rollback-stable [`SimId`] of the body that actually
    /// crossed the exit — NOT re-resolved from live control at commit time,
    /// because possession may have changed, ended, or the body may have died
    /// during the confirmation delay. A body without stable identity cannot
    /// produce a deferred transition intent.
    Transition(RoomTransitionIntent),
    /// Reconstruction: full sandbox reset back to the world's start room.
    FullReset,
}

/// **WHAT A ROOM TRANSITION IS**, stated once and independently of how any host
/// waits for it to be safe.
///
/// Two descriptions of one event that disagreed about the body is what let the eager commit
/// transit whoever happened to be driving several frames later. This is the surviving one; the
/// hosts differ only in WHEN they hand it to the readiness transaction — immediately, or once
/// its originating frame is confirmed.
///
/// **every field must encode deterministically**, because the enclosing
/// [`PendingLifecycleCommit`] is rollback state: the room is named by its authored
/// id and the body by its [`SimId`], never by an index or an `Entity`.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomTransitionIntent {
    /// The body that CROSSED. Never re-resolved from live control at commit
    /// time: possession may have changed, ended, or the body may have died during
    /// the wait. A body without stable identity cannot produce a transition.
    pub subject: SimId,
    /// The destination's authored room id.
    pub target_room: String,
    /// Where in it the subject comes out.
    pub arrival: Vec2,
    /// Selects the transition cooldown/feel, mirroring
    /// `RoomTransitionApplication::apply`. An `EdgeExit` crossing feels different
    /// from a door, and the zone that knew which is long out of reach by commit.
    pub edge_exit: bool,
    /// The door / portal cue this crossing owes, resolved from the zone's
    /// activation at DETECTION time.
    ///
    /// **it has to ride the intent.** The commit runs long after the zone that named it is out
    /// of reach, and the intent deliberately stores no zone.
    ///
    /// a `String`, like `target_room` beside it, because this is rollback state.
    pub zone_sfx: Option<String>,
}

/// One deferred lifecycle op, stamped with the sim frame that produced it.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingIntent {
    /// The sim frame that recorded this intent (`ConfirmedFrameBoundary::current`
    /// at record time). The host-side commit fires once this frame is confirmed.
    pub frame: i32,
    /// The operation to perform.
    pub kind: LifecycleIntent,
}

/// The single pending confirmed-frame lifecycle commit (Track B, Piece 1).
///
/// **Rollback-registered** (`rollback/mod.rs`), so the intent rewinds with the world. One slot,
/// **earliest-sticky**: a consumer records only via [`Self::record`], which keeps the intent
/// already present.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct PendingLifecycleCommit {
    pub pending: Option<PendingIntent>,
}

impl PendingLifecycleCommit {
    /// Record a lifecycle intent for `frame`, keeping any intent already present
    /// (earliest wins). Idempotent under resim: re-recording the same
    /// (frame, kind) is a no-op, and a *different* later intent does not clobber
    /// an earlier unconfirmed one.
    pub fn record(&mut self, frame: i32, kind: LifecycleIntent) {
        if self.pending.is_none() {
            self.pending = Some(PendingIntent { frame, kind });
        }
    }

    /// The pending intent if its recording frame is confirmed (can never be
    /// simulated again). `None` while the intent is still speculative.
    pub fn confirmed(&self, confirmed_frame: i32) -> Option<&PendingIntent> {
        self.pending
            .as_ref()
            .filter(|intent| intent.frame <= confirmed_frame)
    }

    /// Retract the pending room crossing owned by `subject`, if that exact body
    /// is the one waiting to transit.
    ///
    /// The lifecycle owner is the only place allowed to spend this rollback-state slot.
    ///
    /// A different body's transition, or any non-transition lifecycle intent, is untouched.
    pub fn retract_transition_for_subject(&mut self, subject: &SimId) -> bool {
        let owned_by_subject = self.pending.as_ref().is_some_and(|pending| {
            matches!(
                &pending.kind,
                LifecycleIntent::Transition(transition) if &transition.subject == subject
            )
        });
        if owned_by_subject {
            self.pending = None;
        }
        owned_by_subject
    }

    /// Clear the slot after the host commits the op.
    pub fn take(&mut self) -> Option<PendingIntent> {
        self.pending.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_keeps_the_earliest_intent() {
        let mut slot = PendingLifecycleCommit::default();
        slot.record(10, LifecycleIntent::DeathReset);
        // A later PREDICTED op must not overwrite the earlier one before the
        // host can commit it, or the confirmed intent is silently lost.
        slot.record(15, LifecycleIntent::ManualReset);
        assert_eq!(
            slot.pending,
            Some(PendingIntent {
                frame: 10,
                kind: LifecycleIntent::DeathReset
            })
        );
    }

    #[test]
    fn confirmed_only_fires_once_the_frame_is_settled() {
        let mut slot = PendingLifecycleCommit::default();
        slot.record(
            10,
            LifecycleIntent::Transition(RoomTransitionIntent {
                subject: SimId::placement("hero"),
                target_room: "east".into(),
                arrival: Vec2::new(1.0, 2.0),
                edge_exit: true,
                zone_sfx: Some("world.portal.enter".into()),
            }),
        );
        assert!(
            slot.confirmed(9).is_none(),
            "frame 10 is still predicted at confirmed=9"
        );
        assert_eq!(slot.confirmed(10).map(|i| i.frame), Some(10));
        assert_eq!(slot.confirmed(12).map(|i| i.frame), Some(10));
    }

    #[test]
    fn take_empties_the_slot() {
        let mut slot = PendingLifecycleCommit::default();
        slot.record(3, LifecycleIntent::FullReset);
        assert!(slot.take().is_some());
        assert_eq!(slot.pending, None);
    }

    #[test]
    fn retraction_removes_only_the_crossing_owned_by_that_body() {
        let hero = SimId::placement("hero");
        let other = SimId::placement("other");
        let transition = LifecycleIntent::Transition(RoomTransitionIntent {
            subject: hero.clone(),
            target_room: "east".into(),
            arrival: Vec2::new(1.0, 2.0),
            edge_exit: false,
            zone_sfx: None,
        });

        let mut slot = PendingLifecycleCommit::default();
        slot.record(8, transition.clone());
        assert!(!slot.retract_transition_for_subject(&other));
        assert_eq!(slot.pending.as_ref().map(|p| &p.kind), Some(&transition));

        assert!(slot.retract_transition_for_subject(&hero));
        assert!(slot.pending.is_none());

        slot.record(9, LifecycleIntent::Replay);
        assert!(!slot.retract_transition_for_subject(&hero));
        assert!(matches!(
            slot.pending.as_ref().map(|p| &p.kind),
            Some(LifecycleIntent::Replay)
        ));
    }
}
