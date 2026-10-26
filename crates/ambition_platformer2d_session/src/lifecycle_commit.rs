//! Confirmed-frame lifecycle commitment.
//!
//! Room resets and transitions rebuild authoritative world state and therefore
//! cannot execute on speculative rollback frames. Simulation records a
//! rollback-registered [`PendingLifecycleCommit`]; host code executes it only
//! after its originating frame is confirmed, then rebases the rollback session.
//! Eager and rollback hosts share the same intent and differ only in confirmation.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId;

/// Which room-lifecycle operation a deferred commit will perform.
///
/// Carries only deterministic, rollback-safe data — a reason discriminant and,
/// for a transition, the authored loading-zone id plus the rollback-stable
/// [`LiveBodyId`] of the body that triggered it. Never an `Entity`, a fn-pointer, or
/// anything whose value depends on map/query iteration order, so the enclosing
/// [`PendingLifecycleCommit`] can BE rollback state.
#[derive(Clone, Debug, PartialEq)]
pub enum LifecycleIntent {
    /// Reconstruction: transition into `target_room` (its authored id), placing
    /// the TRIGGERING body at `arrival`. `edge_exit` selects the transition
    /// cooldown/feel, mirroring `RoomTransitionApplication::apply`.
    ///
    /// `subject` is the rollback-stable [`LiveBodyId`] of the body that
    /// actually crossed the exit (its `SimId` in its live room, because a
    /// duplicate in another live room has the same `SimId`) — NOT re-resolved from live control at commit time,
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
    /// A value-complete desync projection of this intent.
    ///
    /// ⛔⛔ EXHAUSTIVE BY CONSTRUCTION, and that is the only reason it can be
    /// trusted. Every field is destructured by name, so a new one — or a new
    /// variant — stops this compiling instead of quietly falling outside the
    /// hash. A projection that covers PART of a value is worse than none: it
    /// reports agreement between two peers holding different operations.
    ///
    /// ⚠ It hashes the intent alone. A caller that stores an intent beside other
    /// state (see `AcceptedCheckpointRestore`) owes that state its own legs.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{
            checksum_bytes, put_bool, put_opt_str, put_str, put_u32, put_u8, put_vec2,
        };
        let mut bytes = Vec::new();
        match self {
            Self::Transition(RoomTransitionIntent {
                subject,
                target_room,
                arrival,
                edge_exit,
                zone_sfx,
                participant,
            }) => {
                put_u8(&mut bytes, 0);
                put_str(&mut bytes, subject.sim_id.as_str());
                put_bool(&mut bytes, subject.room.is_some());
                put_u32(&mut bytes, subject.room.map_or(0, |room| room.ordinal()));
                put_str(&mut bytes, target_room);
                put_vec2(&mut bytes, *arrival);
                put_bool(&mut bytes, *edge_exit);
                put_opt_str(&mut bytes, zone_sfx.as_deref());
                put_bool(&mut bytes, participant.is_some());
                put_u8(&mut bytes, participant.map_or(0, |slot| slot.0));
            }
            Self::ReconstituteRoom(RoomReconstitutionIntent { target_room }) => {
                put_u8(&mut bytes, 1);
                put_str(&mut bytes, target_room);
            }
        }
        checksum_bytes(&bytes)
    }

    /// The room this operation will leave standing, whichever shape it has.
    pub fn target_room(&self) -> &str {
        match self {
            Self::Transition(intent) => &intent.target_room,
            Self::ReconstituteRoom(intent) => &intent.target_room,
        }
    }

    /// The body that takes part, if one does. `None` is a rebuild with nobody
    /// in it — not a missing subject.
    pub fn subject(&self) -> Option<&LiveBodyId> {
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

    /// The participant whose crossing this is, as recorded when it was
    /// accepted. A rebuild has none.
    pub fn participant(&self) -> Option<ambition_characters::control::PlayerSlot> {
        match self {
            Self::Transition(intent) => intent.participant,
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
/// room ids and [`LiveBodyId`] rather than transient entities or query positions.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomTransitionIntent {
    /// The body that CROSSED. Never re-resolved from live control at commit
    /// time: possession may have changed, ended, or the body may have died during
    /// the wait. A body without stable identity cannot produce a transition.
    ///
    /// Its live identity: the stable id in the live room it crosses from. Two
    /// instances of one room hold the same stable ids, so the commit must find
    /// the body that crossed and not the first body with its id.
    pub subject: LiveBodyId,
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
    /// The participant whose crossing this is: the slot that drove the
    /// subject when the crossing was accepted. `None`: a body no slot drove.
    ///
    /// ⛔ RECORDED, NEVER RE-DERIVED AT COMMIT. The commit decides whether the
    /// room left stays live for another participant and whether the target is
    /// a room another participant holds, and "another" is relative to this
    /// one. Between detection and commit, possession, death or a handoff can
    /// move the slot off the subject; asking the subject's current
    /// `DrivingParticipant` then reads the crossing as nobody's, and the
    /// room another participant holds is retired.
    pub participant: Option<ambition_characters::control::PlayerSlot>,
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

/// Lifecycle operation recorded at a simulation frame for later commit.
///
/// A same-room replay with a body is a `Transition`; a bodyless room
/// rebuild is `ReconstituteRoom`. Every intent variant must have an
/// admitting producer and a commit executor.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingIntent {
    /// The sim frame that recorded this intent (`ConfirmedFrameBoundary::current`
    /// at record time). The host-side commit fires once this frame is confirmed.
    pub frame: i32,
    /// The operation to perform.
    pub kind: LifecycleIntent,
}

/// A lifecycle intent that left the slot without a commit, and the frame it
/// left on.
///
/// The host may have opened a transaction for the intent, and `recorded_on`
/// names that transaction. A rollback host may not read the absence of an
/// intent as its end, because the absence can be speculative. It retires the
/// transaction when `withdrawn_on` is confirmed.
#[derive(Clone, Debug, PartialEq)]
pub struct WithdrawnIntent {
    /// The frame that recorded the intent ([`PendingIntent::frame`]).
    pub recorded_on: i32,
    /// The frame on which the intent left the slot.
    pub withdrawn_on: i32,
    /// The intent that left.
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
    /// The last intent that left the slot without a commit. Each retraction
    /// writes it, so the host can end the transaction it opened for that
    /// intent. An admitted record clears it: the host replaces or reuses its
    /// transaction for the new intent, so the old withdrawal names nothing.
    pub withdrawn: Option<WithdrawnIntent>,
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
        self.withdrawn = None;
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
    /// is the one waiting to transit, on frame `now`.
    ///
    /// The lifecycle owner is the only place allowed to spend this rollback-state slot.
    ///
    /// A different body's transition, or any non-transition lifecycle intent, is untouched.
    pub fn retract_transition_for_subject(&mut self, subject: &LiveBodyId, now: i32) -> bool {
        self.withdraw_if(now, |pending| {
            matches!(
                &pending.kind,
                LifecycleIntent::Transition(transition) if &transition.subject == subject
            )
        })
    }

    /// Retract the pending intent if it is `kind`, on frame `now`. For a
    /// producer that withdraws its own request; a different intent is
    /// untouched.
    pub fn retract(&mut self, kind: &LifecycleIntent, now: i32) -> bool {
        self.withdraw_if(now, |pending| pending.kind == *kind)
    }

    /// Retract the pending intent recorded on `frame`, whatever it is, on
    /// frame `now`. For an operation that a peer could not prepare: it ends
    /// on each peer, on one frame of the shared timeline.
    pub fn retract_recorded_on(&mut self, frame: i32, now: i32) -> bool {
        self.withdraw_if(now, |pending| pending.frame == frame)
    }

    /// Each retraction goes through here, so each one leaves the record that
    /// lets the host end its transaction ([`Self::withdrawn`]).
    fn withdraw_if(&mut self, now: i32, matches: impl FnOnce(&PendingIntent) -> bool) -> bool {
        let Some(pending) = self.pending.take_if(|pending| matches(pending)) else {
            return false;
        };
        self.withdrawn = Some(WithdrawnIntent {
            recorded_on: pending.frame,
            withdrawn_on: now,
            kind: pending.kind,
        });
        true
    }

    /// The last withdrawal, if the frame it happened on is confirmed. `None`
    /// while it is still speculative: a rewind can undo it.
    pub fn confirmed_withdrawal(&self, confirmed_frame: i32) -> Option<&WithdrawnIntent> {
        self.withdrawn
            .as_ref()
            .filter(|withdrawn| withdrawn.withdrawn_on <= confirmed_frame)
    }

    /// Clear the slot after the host commits the op.
    pub fn take(&mut self) -> Option<PendingIntent> {
        self.pending.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    fn crossing_to(room: &str) -> LifecycleIntent {
        LifecycleIntent::Transition(RoomTransitionIntent {
            subject: ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(SimId::placement("hero"), None),
            target_room: room.into(),
            arrival: Vec2::new(1.0, 2.0),
            edge_exit: false,
            zone_sfx: None,
            participant: None,
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
                subject: ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(SimId::placement("hero"), None),
                target_room: "east".into(),
                arrival: Vec2::new(1.0, 2.0),
                edge_exit: true,
                zone_sfx: Some("world.portal.enter".into()),
                participant: None,
            }),
        );
        assert!(
            slot.confirmed(9).is_none(),
            "frame 10 is still predicted at confirmed=9"
        );
        assert_eq!(slot.confirmed(10).map(|i| i.frame), Some(10));
        assert_eq!(slot.confirmed(12).map(|i| i.frame), Some(10));
    }

    /// The participant a crossing was accepted for is part of the pending
    /// crossing: it moves the checksum, so two peers who disagree about whose
    /// crossing it is desync, and it survives a rewind. Slot 0 and "no slot"
    /// are told apart, as the codec writes presence beside the slot.
    #[test]
    fn the_participant_of_a_crossing_is_checksummed_and_restored() {
        use ambition_characters::control::PlayerSlot;
        use ambition_platformer2d_core::snapshot::{Reader, SnapshotState as _};
        let by = |participant| match crossing_to("east") {
            LifecycleIntent::Transition(intent) => {
                LifecycleIntent::Transition(RoomTransitionIntent { participant, ..intent })
            }
            other => other,
        };
        let sums: Vec<_> = [None, Some(PlayerSlot(0)), Some(PlayerSlot(1))]
            .into_iter()
            .map(|participant| by(participant).checksum())
            .collect();
        assert!(
            sums[0] != sums[1] && sums[1] != sums[2] && sums[0] != sums[2],
            "the participant does not move the pending crossing's checksum: {sums:?}"
        );
        for participant in [None, Some(PlayerSlot(0)), Some(PlayerSlot(1))] {
            let mut slot = PendingLifecycleCommit::default();
            let _ = slot.record(4, by(participant));
            let mut bytes = Vec::new();
            slot.encode(&mut bytes);
            let restored = PendingLifecycleCommit::decode(&mut Reader::new(&bytes));
            assert_eq!(
                restored.and_then(|restored| restored.pending).map(|pending| pending.kind.participant()),
                Some(participant),
                "a rewind lost whose crossing it is"
            );
        }
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
        // The crossing is owned by `hero` in live room #1. `other` is the
        // same authored body in live room #0: a duplicate, not the owner, so
        // its death must not retract the crossing.
        let hero = LiveBodyId::new(SimId::placement("hero"), Some(LiveRoomInstance::ACTIVATION.next()));
        let other = LiveBodyId::new(SimId::placement("hero"), Some(LiveRoomInstance::ACTIVATION));
        let transition = LifecycleIntent::Transition(RoomTransitionIntent {
            subject: hero.clone(),
            target_room: "east".into(),
            arrival: Vec2::new(1.0, 2.0),
            edge_exit: false,
            zone_sfx: None,
            participant: None,
        });

        let mut slot = PendingLifecycleCommit::default();
        let _ = slot.record(8, transition.clone());
        assert!(!slot.retract_transition_for_subject(&other, 9));
        assert_eq!(slot.pending.as_ref().map(|p| &p.kind), Some(&transition));

        assert!(slot.retract_transition_for_subject(&hero, 9));
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
                participant: None,
            }),
        );
        assert!(!slot.retract_transition_for_subject(&hero, 9));
        assert!(slot.pending.is_some());
    }

    /// A retraction leaves a record of what left and when, and the record is
    /// confirmed only with the frame it happened on. A refused retraction
    /// leaves none, and the next admitted intent clears it.
    #[test]
    fn a_retraction_records_the_withdrawal_for_the_host() {
        let mut slot = PendingLifecycleCommit::default();
        let _ = slot.record(10, crossing_to("east"));
        assert!(!slot.retract(&crossing_to("west"), 14), "another intent is not withdrawn");
        assert_eq!(slot.withdrawn, None);

        assert!(slot.retract(&crossing_to("east"), 14));
        let withdrawn = WithdrawnIntent { recorded_on: 10, withdrawn_on: 14, kind: crossing_to("east") };
        assert_eq!(slot.withdrawn.as_ref(), Some(&withdrawn));
        assert_eq!(slot.confirmed_withdrawal(13), None, "frame 14 is still speculative");
        assert_eq!(slot.confirmed_withdrawal(14), Some(&withdrawn));

        assert_eq!(slot.record(20, crossing_to("west")), Admission::Admitted);
        assert_eq!(slot.withdrawn, None, "a new intent supersedes the old transaction");
        assert!(slot.retract_recorded_on(20, 21));
        assert_eq!(slot.withdrawn.as_ref().map(|w| (w.recorded_on, w.withdrawn_on)), Some((20, 21)));

        // The withdrawal is rollback state: a rewind keeps it.
        use ambition_platformer2d_core::snapshot::{Reader, SnapshotState as _};
        let _ = slot.record(30, crossing_to("north"));
        let _ = slot.retract(&crossing_to("north"), 31);
        let _ = slot.record(32, crossing_to("south"));
        slot.withdrawn = Some(withdrawn);
        let mut bytes = Vec::new();
        slot.encode(&mut bytes);
        assert_eq!(PendingLifecycleCommit::decode(&mut Reader::new(&bytes)), Some(slot));
    }
}
