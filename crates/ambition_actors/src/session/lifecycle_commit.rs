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
//! The fix is to DEFER. Instead of executing, the lifecycle consumer RECORDS a
//! [`PendingLifecycleCommit`] and returns. This resource is **rollback-registered
//! state** — unlike [`crate`]'s external-effect journal, whose consumers live
//! outside the sim — so:
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
//! Non-rollback hosts (fixed-tick, render-frame, headless — no
//! `ConfirmedFrameBoundary`) never record: the consumers execute eagerly exactly
//! as before, so the shipped games are untouched.

use bevy::prelude::*;

use ambition_platformer_primitives::sim_id::SimId;

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
    /// cooldown/feel, mirroring `commit_room_transition_geometry`.
    ///
    /// `subject` is the rollback-stable [`SimId`] of the body that actually
    /// crossed the exit — NOT re-resolved from live control at commit time,
    /// because possession may have changed, ended, or the body may have died
    /// during the confirmation delay (GPT review finding 2). `None` means the
    /// trigger had no `SimId` (the home avatar on paths that don't stamp one);
    /// the committer then transports the primary player, which is stable for the
    /// unpossessed case.
    Transition {
        subject: Option<SimId>,
        target_room: String,
        arrival: Vec2,
        edge_exit: bool,
    },
    /// Reconstruction: full sandbox reset back to the world's start room.
    FullReset,
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
/// **Rollback-registered** (`rollback/mod.rs`), so the intent rewinds with the
/// world. One slot, **earliest-sticky**: a consumer records only via
/// [`Self::record`], which keeps the intent already present. That guarantees a
/// confirmed intent is never overwritten by a later *predicted* one before the
/// host has a chance to commit it; the host clears the slot and rebases the
/// timeline, after which any still-pending later op is re-derived fresh.
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
            LifecycleIntent::Transition {
                subject: Some(SimId::placement("hero")),
                target_room: "east".into(),
                arrival: Vec2::new(1.0, 2.0),
                edge_exit: true,
            },
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
}
