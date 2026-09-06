//! Confirmed-frame lifecycle commitment.
//!
//! Room resets and transitions rebuild authoritative world state and therefore
//! cannot execute on speculative rollback frames. Simulation records a
//! rollback-registered [`PendingLifecycleCommit`]; host code executes it only
//! after its originating frame is confirmed, then rebases the rollback session.
//! Eager and rollback hosts share the same intent and differ only in confirmation.

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
    /// Reconstitution with NOBODY IN IT: rebuild `target_room` from its authored
    /// spec, carrying no body through and placing none on arrival.
    ///
    /// ⭐ THIS VARIANT HAS NO SUBJECT BY CONSTRUCTION, and that is the point. The
    /// obvious alternative — making [`RoomTransitionIntent::subject`] an
    /// `Option` — would also make a bodyless DOOR CROSSING representable, and
    /// *"a body without stable identity cannot produce a transition"* is a rule
    /// that is right for a crossing. Two shapes, each unable to express the
    /// other's mistake.
    ///
    /// The one road that records this is a replay in a composition with no
    /// controlled body (`admit_room_replay`). It arrives WITH its consumer,
    /// which is the condition the deletion note below sets.
    ReconstituteRoom(RoomReconstitutionIntent),
}

/// Deterministic description of a room rebuild that no body takes part in.
///
/// Rollback state, so it names the room by its authored id. It carries no
/// arrival: nobody arrives.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomReconstitutionIntent {
    /// The room to rebuild, by authored id. For a replay this is the room the
    /// session is already in.
    pub target_room: String,
}

impl LifecycleIntent {
    /// The room this operation will leave standing, whichever shape it has.
    pub fn target_room(&self) -> &str {
        match self {
            Self::Transition(intent) => &intent.target_room,
            Self::ReconstituteRoom(intent) => &intent.target_room,
        }
    }

    /// The body that takes part, if one does. `None` is a rebuild with nobody
    /// in it — not a missing subject.
    pub fn subject(&self) -> Option<&SimId> {
        match self {
            Self::Transition(intent) => Some(&intent.subject),
            Self::ReconstituteRoom(_) => None,
        }
    }

    /// Where the subject comes out. `None` when there is no subject to place —
    /// which is why this is not a `Vec2` with a default: a reconstitution has no
    /// arrival, and `Vec2::ZERO` would be a position somebody could validate.
    pub fn arrival(&self) -> Option<Vec2> {
        match self {
            Self::Transition(intent) => Some(intent.arrival),
            Self::ReconstituteRoom(_) => None,
        }
    }

    /// The door cue, if a door was opened. Nobody opens one on a rebuild.
    pub fn zone_sfx(&self) -> Option<&str> {
        match self {
            Self::Transition(intent) => intent.zone_sfx.as_deref(),
            Self::ReconstituteRoom(_) => None,
        }
    }

    /// Whether this is an edge crossing, which selects the cooldown and feel.
    /// A rebuild nobody walks into is never one.
    pub fn edge_exit(&self) -> bool {
        match self {
            Self::Transition(intent) => intent.edge_exit,
            Self::ReconstituteRoom(_) => false,
        }
    }
}

/// Deterministic description of a room transition, independent of host
/// confirmation timing. Because it is rollback state, identities use authored
/// room ids and [`SimId`] rather than transient entities or query positions.
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
    /// Door/portal cue resolved when the crossing is detected, before the
    /// originating zone is no longer available at commit time.
    pub zone_sfx: Option<String>,
}

/// Whether a lifecycle operation got the one pending slot.
///
/// A refusal is ordinary, not an error: two operations asked in the same frame
/// and the earlier one owns the world. What is NOT ordinary is running the
/// refused operation's consequences anyway.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum Admission {
    /// The slot is this operation's. Its consequences may run.
    Admitted,
    /// Another lifecycle operation already owns the slot. Change nothing.
    AlreadyPending,
}

impl Admission {
    pub fn admitted(self) -> bool {
        matches!(self, Self::Admitted)
    }
}

/// ⛔⛔ FOUR VARIANTS WERE DELETED HERE, 2026-08-30: `DeathReset`,
/// `ManualReset`, `Replay` and `FullReset`. Nothing recorded any of them — the
/// commit executor's own comment said so — and a stray one would have returned
/// `CommitOutcome::Retry` forever, which is a silent stall wearing an
/// exhaustive match's clothes. Every road that ends a room now records the one
/// thing a lifecycle boundary actually does: `Transition`. A same-room replay
/// is a transition to the room you are standing in.
///
/// ⭐ ONE VARIANT IS NOT A MISTAKE. The enum is the vocabulary the pending slot
/// is about — "one lifecycle operation at a time" — and a new variant is
/// welcome the day it arrives WITH ITS CONSUMER. It was the four that arrived
/// without one.
///
/// ✔ AND ONE ARRIVED, 2026-08-31: [`LifecycleIntent::ReconstituteRoom`], under
/// exactly that condition. Unlike the deleted four, something records it
/// (`admit_room_replay`'s bodyless arm, which until now admitted a replay
/// without ever taking this slot) and something executes it. It is also not one
/// of the four coming back: those were in-place resets that a same-room
/// transition already covers, whereas this one exists because a room rebuild
/// with NO BODY cannot be described as a crossing at all.
///
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
/// Rollback-registered (`rollback/mod.rs`), so the intent rewinds with the world. One slot,
/// earliest-sticky: a consumer records only via [`Self::record`], which keeps the intent
/// already present.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct PendingLifecycleCommit {
    pub pending: Option<PendingIntent>,
}

impl PendingLifecycleCommit {
    /// Record a lifecycle intent for `frame`, keeping any intent already present
    /// (earliest wins), and say WHICH HAPPENED. Idempotent under resim:
    /// re-recording the same (frame, kind) is a no-op, and a *different* later
    /// intent does not clobber an earlier unconfirmed one.
    ///
    /// ⛔⛔ `#[must_use]`, AND THAT IS THE POINT. This is one slot and it is
    /// earliest-sticky, so recording is a request that can be REFUSED — and a
    /// caller that performs the operation's consequences without checking has
    /// changed the world for an operation that never happened. The same-room
    /// replay did exactly that: it reset the avatar, gravity, content cycles and
    /// the previous attempt's residue, and only then discovered the slot was
    /// taken. Admit first, mutate second.
    #[must_use = "the slot is earliest-sticky: a refused intent must not have its consequences run"]
    pub fn record(&mut self, frame: i32, kind: LifecycleIntent) -> Admission {
        if self.pending.is_some() {
            return Admission::AlreadyPending;
        }
        self.pending = Some(PendingIntent { frame, kind });
        Admission::Admitted
    }

    /// The pending intent WHETHER OR NOT it is confirmed — "is anything waiting".
    ///
    /// ⛔ For REPORTING only. Every consumer that acts on an intent must go
    /// through [`Self::confirmed`]; this exists so a host that is refusing to
    /// promote intents can say whether it is refusing anything, which is the
    /// difference between a stalled game and an idle one.
    pub fn peek(&self) -> Option<&PendingIntent> {
        self.pending.as_ref()
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

    fn crossing_to(room: &str) -> LifecycleIntent {
        LifecycleIntent::Transition(RoomTransitionIntent {
            subject: SimId::placement("hero"),
            target_room: room.into(),
            arrival: Vec2::new(1.0, 2.0),
            edge_exit: false,
            zone_sfx: None,
        })
    }

    #[test]
    fn record_keeps_the_earliest_intent_and_says_which_happened() {
        let mut slot = PendingLifecycleCommit::default();
        assert_eq!(slot.record(10, crossing_to("east")), Admission::Admitted);
        // A later PREDICTED op must not overwrite the earlier one before the
        // host can commit it, or the confirmed intent is silently lost.
        assert_eq!(
            slot.record(15, crossing_to("west")),
            Admission::AlreadyPending,
            "the slot is earliest-sticky, and a caller that runs the refused \
             operation's consequences anyway is the defect this return value \
             exists to make visible"
        );
        assert_eq!(
            slot.pending,
            Some(PendingIntent {
                frame: 10,
                kind: crossing_to("east")
            })
        );
    }

    #[test]
    fn confirmed_only_fires_once_the_frame_is_settled() {
        let mut slot = PendingLifecycleCommit::default();
        let _ = slot.record(
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
        let _ = slot.record(3, crossing_to("east"));
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
        let _ = slot.record(8, transition.clone());
        assert!(!slot.retract_transition_for_subject(&other));
        assert_eq!(slot.pending.as_ref().map(|p| &p.kind), Some(&transition));

        assert!(slot.retract_transition_for_subject(&hero));
        assert!(slot.pending.is_none());

        // Another body's crossing is not this body's to retract.
        let _ = slot.record(
            9,
            LifecycleIntent::Transition(RoomTransitionIntent {
                subject: other.clone(),
                target_room: "west".into(),
                arrival: Vec2::ZERO,
                edge_exit: false,
                zone_sfx: None,
            }),
        );
        assert!(!slot.retract_transition_for_subject(&hero));
        assert!(slot.pending.is_some());
    }
}
