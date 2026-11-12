//! Room lifecycle admission and application receipts.
//!
//! Simulation records an admitted operation in rollback state. Ordinary peer
//! crossings apply from shared frame inputs. Restores and local sync-test
//! operations use the confirmed host barrier.

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
            checksum_bytes, put_bool, put_opt_str, put_str, put_u8, put_vec2,
        };
        let mut bytes = Vec::new();
        match self {
            Self::Transition(RoomTransitionIntent {
                disposition, source_zone, initiator, companions, target_room, edge_exit, zone_sfx,
            }) => {
                put_u8(&mut bytes, 0);
                put_u8(&mut bytes, match disposition {
                    RoomTransitionDisposition::Travel => 0,
                    RoomTransitionDisposition::RebuildSource => 1,
                });
                put_str(&mut bytes, target_room);
                put_bool(&mut bytes, *edge_exit);
                put_opt_str(&mut bytes, zone_sfx.as_deref());
                put_opt_str(&mut bytes, source_zone.as_deref());
                ambition_platformer2d_core::snapshot::put_u64(&mut bytes, (1 + companions.len()) as u64);
                for transfer in std::iter::once(initiator).chain(companions) {
                    let RoomBodyTransfer { requirement, subject, arrival, participant } = transfer;
                    put_u8(&mut bytes, match requirement {
                        BodyTransferRequirement::InPlay => 0,
                        BodyTransferRequirement::Present => 1,
                    });
                    subject.write_peer_stable_into(&mut bytes);
                    match arrival {
                        RoomArrival::At(at) => { put_u8(&mut bytes, 0); put_vec2(&mut bytes, *at); }
                        RoomArrival::NearBody(body) => { put_u8(&mut bytes, 1); body.write_peer_stable_into(&mut bytes); }
                        RoomArrival::Pose { at, material } => {
                            put_u8(&mut bytes, 2);
                            put_vec2(&mut bytes, *at);
                            ambition_platformer2d_core::snapshot::SnapshotState::encode(material, &mut bytes);
                        }
                    }
                    put_bool(&mut bytes, participant.is_some());
                    put_u8(&mut bytes, participant.map_or(0, |slot| slot.0));
                }
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

    pub fn rebuilds_source_room(&self) -> bool {
        match self {
            Self::Transition(intent) => intent.disposition == RoomTransitionDisposition::RebuildSource,
            Self::ReconstituteRoom(_) => true,
        }
    }

    pub fn transfers(&self) -> impl Iterator<Item = &RoomBodyTransfer> {
        match self { Self::Transition(intent) => Some(intent), Self::ReconstituteRoom(_) => None }
            .into_iter().flat_map(RoomTransitionIntent::transfers)
    }

    /// The body that initiated the operation. A bodyless rebuild has none.
    pub fn subject(&self) -> Option<&LiveBodyId> {
        match self {
            Self::Transition(intent) => Some(&intent.initiator.subject),
            Self::ReconstituteRoom(_) => None,
        }
    }

    /// Where the subject comes out. `None` when there is no subject to place —
    /// which is why this is not a `Vec2` with a default: a reconstitution has no
    /// arrival, and `Vec2::ZERO` would be a position somebody could validate.
    pub fn arrival(&self) -> Option<&RoomArrival> {
        match self {
            Self::Transition(intent) => Some(&intent.initiator.arrival),
            Self::ReconstituteRoom(_) => None,
        }
    }

    /// The participant whose crossing this is, as recorded when it was
    /// accepted. A rebuild has none.
    pub fn participant(&self) -> Option<ambition_characters::control::PlayerSlot> {
        match self {
            Self::Transition(intent) => intent.initiator.participant,
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

    pub fn source_zone(&self) -> Option<&str> {
        match self {
            Self::Transition(intent) => intent.source_zone.as_deref(),
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
    pub disposition: RoomTransitionDisposition,
    /// The zone that admitted the operation.
    pub source_zone: Option<String>,
    /// The body that activated the crossing.
    pub initiator: RoomBodyTransfer,
    /// Other bodies selected when this operation was admitted.
    pub companions: Vec<RoomBodyTransfer>,
    pub target_room: String,
    pub edge_exit: bool,
    pub zone_sfx: Option<String>,
}

/// The source room's fate is independent of the body's controller.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoomTransitionDisposition {
    #[default]
    Travel,
    RebuildSource,
}

/// Required body state when the transfer is published.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyTransferRequirement {
    /// The body must be alive and in play.
    InPlay,
    /// The body can be out of play. Recovery and restoration use this condition.
    Present,
}

/// One body selected for a room transfer. Control changes do not replace it.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomBodyTransfer {
    pub requirement: BodyTransferRequirement,
    pub subject: LiveBodyId,
    pub arrival: RoomArrival,
    pub participant: Option<ambition_characters::control::PlayerSlot>,
}

/// Resolve a point or a live-body reference on the room commit frame.
#[derive(Clone, Debug, PartialEq)]
pub enum RoomArrival {
    At(Vec2),
    Pose { at: Vec2, material: ambition_platformer2d_core::frame::BodyMaterial },
    NearBody(LiveBodyId),
}

impl RoomArrival {
    pub fn point(&self) -> Option<Vec2> {
        match self { Self::At(at) | Self::Pose { at, .. } => Some(*at), Self::NearBody(_) => None }
    }

    pub fn is_finite(&self) -> bool {
        match self { Self::At(at) | Self::Pose { at, .. } => at.is_finite(), Self::NearBody(_) => true }
    }
}

impl RoomTransitionIntent {
    pub fn transfers(&self) -> impl Iterator<Item = &RoomBodyTransfer> {
        std::slice::from_ref(&self.initiator).iter().chain(self.companions.iter())
    }
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
    /// The admitted restore. `None` means an ordinary room operation.
    pub checkpoint_operation: Option<crate::checkpoint::CheckpointOperationKey>,
}

impl PendingIntent {
    /// Peer identity includes the restore admission, but not its local scope value.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_i32, put_u64};
        let Self { frame, kind, checkpoint_operation } = self;
        let mut bytes = Vec::new();
        put_i32(&mut bytes, *frame);
        put_u64(&mut bytes, kind.checksum());
        write_checkpoint_operation(&mut bytes, *checkpoint_operation);
        checksum_bytes(&bytes)
    }
}

fn write_checkpoint_operation(out: &mut Vec<u8>, key: Option<crate::checkpoint::CheckpointOperationKey>) {
    ambition_platformer2d_core::snapshot::put_bool(out, key.is_some());
    if let Some(key) = key {
        key.write_peer_stable_into(out);
    }
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
    pub checkpoint_operation: Option<crate::checkpoint::CheckpointOperationKey>,
}

/// A room crossing applied on one simulation frame.
#[derive(Clone, Debug, PartialEq)]
pub struct AppliedIntent {
    pub operation: PendingIntent,
    pub applied_on: i32,
    pub preparation_checksum: u64,
    pub arrived_in: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}

/// One pending lifecycle admission and the latest applied crossing.
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
    /// The most recent application on this rollback branch.
    pub applied: Option<AppliedIntent>,
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
        self.pending = Some(PendingIntent { frame, kind, checkpoint_operation: None });
        self.withdrawn = None;
        Admission::Admitted
    }

    /// Admit the restore and its key together. A refusal spends no sequence.
    pub fn record_restore(
        &mut self,
        frame: i32,
        kind: LifecycleIntent,
        operations: &mut crate::checkpoint::SessionCheckpointOperations,
        scope: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId>,
    ) -> Option<crate::checkpoint::CheckpointOperationKey> {
        if self.pending.is_some() {
            return None;
        }
        let key = operations.admit(scope)?;
        self.pending = Some(PendingIntent { frame, kind, checkpoint_operation: Some(key) });
        self.withdrawn = None;
        Some(key)
    }

    /// Compare shared state while retaining local session keys in clone snapshots.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_bool, put_i32, put_u64};
        let Self { pending, withdrawn, applied } = self;
        let mut bytes = Vec::new();
        put_bool(&mut bytes, pending.is_some());
        if let Some(pending) = pending {
            put_u64(&mut bytes, pending.checksum());
        }
        put_bool(&mut bytes, withdrawn.is_some());
        if let Some(WithdrawnIntent { recorded_on, withdrawn_on, kind, checkpoint_operation }) = withdrawn {
            put_i32(&mut bytes, *recorded_on);
            put_i32(&mut bytes, *withdrawn_on);
            put_u64(&mut bytes, kind.checksum());
            write_checkpoint_operation(&mut bytes, *checkpoint_operation);
        }
        put_bool(&mut bytes, applied.is_some());
        if let Some(applied) = applied {
            put_u64(&mut bytes, applied.operation.checksum());
            put_i32(&mut bytes, applied.applied_on);
            put_u64(&mut bytes, applied.preparation_checksum);
            put_bool(&mut bytes, applied.arrived_in.is_some());
            if let Some(room) = applied.arrived_in {
                ambition_platformer2d_core::snapshot::put_u32(&mut bytes, room.ordinal());
            }
        }
        checksum_bytes(&bytes)
    }

    /// Inspect the current branch's pending admission. Application requires
    /// frame input agreement or the confirmed lifecycle barrier.
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
                LifecycleIntent::Transition(transition) if transition.transfers().any(|transfer| &transfer.subject == subject)
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
            checkpoint_operation: pending.checkpoint_operation,
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

    /// Spend the slot and retain the applied operation in rollback state.
    pub fn complete(&mut self, applied_on: i32, preparation_checksum: u64,
        arrived_in: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    ) -> Option<AppliedIntent> {
        let receipt = AppliedIntent { operation: self.pending.take()?, applied_on,
            preparation_checksum, arrived_in };
        self.applied = Some(receipt.clone());
        Some(receipt)
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
            disposition: Default::default(),
            source_zone: None,
                initiator: crate::lifecycle_commit::RoomBodyTransfer {
                    requirement: crate::lifecycle_commit::BodyTransferRequirement::InPlay,
                    subject: ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(SimId::placement("hero"), None),
                    arrival: crate::lifecycle_commit::RoomArrival::At(Vec2::new(1.0, 2.0)),
                    participant: None,
                },
                companions: Vec::new(),
            target_room: room.into(),
            edge_exit: false,
            zone_sfx: None,
        })
    }

    #[test]
    fn restore_admission_is_atomic_and_refusal_spends_no_key() {
        use crate::checkpoint::SessionCheckpointOperations;
        let mut slot = PendingLifecycleCommit::default();
        let mut operations = SessionCheckpointOperations::default();
        assert!(slot.record(10, crossing_to("east")).admitted());
        assert_eq!(slot.record_restore(11, crossing_to("west"), &mut operations, None), None);
        assert_eq!(operations, SessionCheckpointOperations::default());
        slot.take();
        let key = slot.record_restore(12, crossing_to("west"), &mut operations, None).unwrap();
        assert_eq!(key.sequence, 0);
        assert_eq!(slot.peek().unwrap().checkpoint_operation, Some(key));
        assert!(slot.retract_recorded_on(12, 13));
        assert_eq!(slot.withdrawn.as_ref().unwrap().checkpoint_operation, Some(key));

        let before = slot.clone();
        let mut exhausted = SessionCheckpointOperations::exhausted_for_test();
        assert_eq!(slot.record_restore(14, crossing_to("north"), &mut exhausted, None), None);
        assert_eq!(slot, before);
    }

    #[test]
    fn every_transfer_is_part_of_the_admission_checksum_and_cancellation() {
        let LifecycleIntent::Transition(mut crossing) = crossing_to("east") else { unreachable!() };
        let companion = RoomBodyTransfer {
            requirement: crate::lifecycle_commit::BodyTransferRequirement::InPlay,
            subject: LiveBodyId::new(SimId::placement("companion"), None),
            arrival: crate::lifecycle_commit::RoomArrival::At(Vec2::new(20.0, 40.0)), participant: Some(ambition_characters::control::PlayerSlot(1)),
        };
        let single = LifecycleIntent::Transition(crossing.clone()).checksum();
        crossing.initiator.requirement = BodyTransferRequirement::Present;
        assert_ne!(single, LifecycleIntent::Transition(crossing.clone()).checksum());
        crossing.initiator.requirement = BodyTransferRequirement::InPlay;
        crossing.disposition = RoomTransitionDisposition::RebuildSource;
        assert_ne!(single, LifecycleIntent::Transition(crossing.clone()).checksum());
        crossing.disposition = RoomTransitionDisposition::Travel;
        crossing.companions.push(companion.clone());
        let checksum = LifecycleIntent::Transition(crossing.clone()).checksum();
        assert_ne!(single, checksum);
        crossing.companions[0].arrival = RoomArrival::At(Vec2::new(21.0, 40.0));
        assert_ne!(checksum, LifecycleIntent::Transition(crossing.clone()).checksum());
        crossing.companions[0] = companion.clone();
        crossing.companions[0].requirement = BodyTransferRequirement::Present;
        assert_ne!(checksum, LifecycleIntent::Transition(crossing.clone()).checksum());
        crossing.companions[0] = companion.clone();
        crossing.companions[0].participant = None;
        assert_ne!(checksum, LifecycleIntent::Transition(crossing.clone()).checksum());
        let mut pending = PendingLifecycleCommit::default();
        assert!(pending.record(10, LifecycleIntent::Transition(crossing)).admitted());
        assert!(pending.retract_transition_for_subject(&companion.subject, 11));
        assert!(pending.peek().is_none());
    }

    #[test]
    fn relative_arrivals_checksum_the_reference_instead_of_a_point() {
        let LifecycleIntent::Transition(crossing) = crossing_to("east") else { unreachable!() };
        let point = LifecycleIntent::Transition(crossing.clone()).checksum();
        let mut relative = crossing.clone();
        relative.initiator.arrival = RoomArrival::NearBody(crossing.initiator.subject.clone());
        let checksum = LifecycleIntent::Transition(relative.clone()).checksum();
        assert_ne!(point, checksum);
        relative.initiator.arrival = RoomArrival::NearBody(LiveBodyId::new(SimId::placement("other"), None));
        assert_ne!(checksum, LifecycleIntent::Transition(relative).checksum());
    }

    #[test]
    fn different_source_zones_do_not_identify_the_same_crossing() {
        let LifecycleIntent::Transition(mut first) = crossing_to("east") else { unreachable!() };
        first.source_zone = Some("first-ring".into());
        let mut second = first.clone();
        second.source_zone = Some("second-ring".into());
        assert_ne!(LifecycleIntent::Transition(first).checksum(), LifecycleIntent::Transition(second).checksum());
    }

    #[test]
    fn restore_peer_checksums_compare_admissions_without_local_scope_values() {
        use crate::checkpoint::CheckpointOperationKey;
        use ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId;
        let state = |scope, sequence| {
            let mut slot = PendingLifecycleCommit::default();
            assert!(slot.record(10, crossing_to("east")).admitted());
            slot.pending.as_mut().unwrap().checkpoint_operation = Some(CheckpointOperationKey { scope, sequence });
            slot
        };
        let first = state(Some(SessionScopeId(1)), 7);
        let peer = state(Some(SessionScopeId(99)), 7);
        assert_ne!(first, peer);
        assert_eq!(first.checksum(), peer.checksum());
        for different in [state(Some(SessionScopeId(1)), 8), state(None, 7)] {
            assert_ne!(first.checksum(), different.checksum());
            assert_ne!(first.peek().unwrap().checksum(), different.peek().unwrap().checksum());
        }
        let mut ordinary = first.clone();
        ordinary.pending.as_mut().unwrap().checkpoint_operation = None;
        assert_ne!(first.checksum(), ordinary.checksum());
        let mut withdrawn = first;
        assert!(withdrawn.retract_recorded_on(10, 11));
        let mut withdrawn_peer = peer;
        assert!(withdrawn_peer.retract_recorded_on(10, 11));
        assert_eq!(withdrawn.checksum(), withdrawn_peer.checksum());
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
                frame: 10, checkpoint_operation: None,
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
                disposition: Default::default(),
                source_zone: None,
                initiator: crate::lifecycle_commit::RoomBodyTransfer {
                    requirement: crate::lifecycle_commit::BodyTransferRequirement::InPlay,
                    subject: ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(SimId::placement("hero"), None),
                    arrival: crate::lifecycle_commit::RoomArrival::At(Vec2::new(1.0, 2.0)),
                    participant: None,
                },
                companions: Vec::new(),
                target_room: "east".into(),
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

    /// The participant a crossing was accepted for is part of the pending
    /// crossing: it moves the checksum, so two peers cannot agree about
    /// different participants. An absent participant differs from slot zero.
    #[test]
    fn the_participant_of_a_crossing_is_checksummed() {
        use ambition_characters::control::PlayerSlot;
        let by = |participant| match crossing_to("east") {
            LifecycleIntent::Transition(mut intent) => {
                intent.initiator.participant = participant;
                LifecycleIntent::Transition(intent)
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
            disposition: Default::default(),
            source_zone: None,
                initiator: crate::lifecycle_commit::RoomBodyTransfer {
                    requirement: crate::lifecycle_commit::BodyTransferRequirement::InPlay,
                    subject: hero.clone(),
                    arrival: crate::lifecycle_commit::RoomArrival::At(Vec2::new(1.0, 2.0)),
                    participant: None,
                },
                companions: Vec::new(),
            target_room: "east".into(),
            edge_exit: false,
            zone_sfx: None,
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
                disposition: Default::default(),
                source_zone: None,
                initiator: crate::lifecycle_commit::RoomBodyTransfer {
                    requirement: crate::lifecycle_commit::BodyTransferRequirement::InPlay,
                    subject: other.clone(),
                    arrival: crate::lifecycle_commit::RoomArrival::At(Vec2::ZERO),
                    participant: None,
                },
                companions: Vec::new(),
                target_room: "west".into(),
                edge_exit: false,
                zone_sfx: None,
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
        let withdrawn = WithdrawnIntent { recorded_on: 10, withdrawn_on: 14, kind: crossing_to("east"), checkpoint_operation: None };
        assert_eq!(slot.withdrawn.as_ref(), Some(&withdrawn));
        assert_eq!(slot.confirmed_withdrawal(13), None, "frame 14 is still speculative");
        assert_eq!(slot.confirmed_withdrawal(14), Some(&withdrawn));

        assert_eq!(slot.record(20, crossing_to("west")), Admission::Admitted);
        assert_eq!(slot.withdrawn, None, "a new intent supersedes the old transaction");
        assert!(slot.retract_recorded_on(20, 21));
        assert_eq!(slot.withdrawn.as_ref().map(|w| (w.recorded_on, w.withdrawn_on)), Some((20, 21)));


    }
}
