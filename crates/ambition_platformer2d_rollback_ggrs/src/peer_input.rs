//! What one peer sends to the other each frame: the controls of a seat, and
//! what this peer says of its pending lifecycle operation.
//!
//! The commit of a lifecycle operation under a peer session (the peer barrier
//! in [`crate::lifecycle_commit`]) needs a fact that only the other peer has:
//! did it prepare the operation? Room preparation is host-side. One peer can
//! build the room and the other cannot (a missing asset, a different save). A
//! peer that commits alone starts its next session, and the other peer stays
//! in the old one. Then neither session runs again.
//!
//! So each peer puts its verdict in its input, and a peer commits only when
//! each handle of the session said "prepared" for the same operation (Q156).
//! The input is the one channel that the two peers already exchange and
//! confirm, so the verdict needs no second protocol.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use ambition_platformer2d_core::ControlFrame;

/// What a peer says of the lifecycle operation it waits on.
///
/// ⛔ NO DATA IN A VARIANT. Each handle's input of one frame goes into one
/// payload, and the receiver divides the length by the handle count, so each
/// input must have the same width (see
/// `the_bytes_two_peers_exchange::each_input_has_the_same_width`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PreparationVerdict {
    /// No operation waits, or this peer has not prepared it yet.
    #[default]
    NotYet,
    /// This peer's plan for the operation is authorized, and it can commit.
    Prepared,
    /// This peer's preparation of the operation failed. No peer commits it,
    /// and it ends on each peer at the frame of this input
    /// (`lifecycle_commit::end_an_operation_a_peer_could_not_prepare`; the
    /// default in force until Q156 is ruled).
    Failed,
}

/// The input identity of a recorded lifecycle intent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerOperation {
    pub recorded_on: i32,
    pub intent_checksum: u64,
    pub preparation_checksum: u64,
}

impl From<&ambition_platformer2d_session::room_preparation::RoomPreparation> for PeerOperation {
    fn from(preparation: &ambition_platformer2d_session::room_preparation::RoomPreparation) -> Self {
        let intent = preparation.admission();
        Self { recorded_on: intent.frame, intent_checksum: intent.checksum(), preparation_checksum: preparation.checksum() }
    }
}

/// Fixed-width wire projection of a frozen construction plan.
/// Zero content bytes mean that no plan was stated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerPlanIdentity {
    pub incoming_content: [u8; 32],
    pub expected_content: [u8; 32],
    pub data_digest: u64,
}

impl From<ambition_platformer2d_publication::PeerRoomConstructionIdentity> for PeerPlanIdentity {
    fn from(plan: ambition_platformer2d_publication::PeerRoomConstructionIdentity) -> Self {
        Self {
            incoming_content: *plan.incoming_content().as_bytes(),
            expected_content: *plan.expected_content().as_bytes(),
            data_digest: plan.data_digest(),
        }
    }
}

impl PeerPlanIdentity {
    pub fn is_stated(self) -> bool {
        self.incoming_content != [0; 32] && self.expected_content != [0; 32]
    }
}

/// One peer's verdict on one operation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerVerdict {
    /// The frame and intent must match. A rollback can replace an intent
    /// without changing its recording frame.
    pub operation: PeerOperation,
    pub plan: PeerPlanIdentity,
    pub said: PreparationVerdict,
}

/// `GgrsConfig::Input`: what crosses between peers for one handle and one
/// frame.
///
/// The fields are positional on the wire (bincode), so the order is part of
/// the format. `CONTROL_FRAME_WIRE_IDENTITY` names it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PeerInput {
    pub control: ControlFrame,
    pub verdict: PeerVerdict,
}

/// This peer's verdict, decided before GGRS reads the local inputs of a frame
/// and put in the input of each local handle. Host state: it is a fact about
/// this machine, and a rewind does not change it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ThisPeersVerdict(pub PeerVerdict);

/// The last CONFIRMED verdict of each handle of the live session.
///
/// Host state, and never rewound: a confirmed input does not change. A
/// verdict recorded from a predicted input could be the default that GGRS
/// predicts, so only confirmed inputs are recorded. The newest frame wins.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct PeerVerdicts {
    /// The handles of the session, each of which must say `Prepared`. Zero
    /// outside a peer session.
    handles: usize,
    /// By handle: the frame of the input, and its verdict.
    said: Vec<Option<(i32, PeerVerdict)>>,
}

impl PeerVerdicts {
    /// No verdicts yet, from a session of `handles` handles.
    pub fn for_handles(handles: usize) -> Self {
        Self { handles, said: Vec::new() }
    }

    /// Record the verdict that `handle` sent in its confirmed input of `frame`.
    pub fn record(&mut self, handle: usize, frame: i32, verdict: PeerVerdict) {
        if self.said.len() <= handle {
            self.said.resize(handle + 1, None);
        }
        let slot = &mut self.said[handle];
        if slot.is_none_or(|(seen, _)| seen <= frame) {
            *slot = Some((frame, verdict));
        }
    }

    /// What `handle` last said of `operation`. `NotYet` when its last verdict
    /// is of another operation, or when nothing of it is confirmed.
    pub fn said(&self, handle: usize, operation: PeerOperation) -> PreparationVerdict {
        match self.said.get(handle).copied().flatten() {
            Some((_, verdict)) if verdict.operation == operation => verdict.said,
            _ => PreparationVerdict::NotYet,
        }
    }

    /// The plan every handle prepared for this operation.
    pub fn prepared_plan(&self, operation: PeerOperation) -> Option<PeerPlanIdentity> {
        if self.handles == 0 {
            return None;
        }
        let first = self.said.first().copied().flatten()?.1;
        if first.operation != operation || first.said != PreparationVerdict::Prepared || !first.plan.is_stated() {
            return None;
        }
        (0..self.handles).all(|handle| self.said.get(handle).copied().flatten().is_some_and(|(_, verdict)| {
            verdict.operation == operation && verdict.said == PreparationVerdict::Prepared && verdict.plan == first.plan
        })).then_some(first.plan)
    }

    /// Prepared replies that disagree with this host's frozen plan.
    pub fn mismatched_plans(&self, operation: PeerOperation, expected: PeerPlanIdentity) -> Vec<usize> {
        (0..self.handles).filter(|handle| self.said.get(*handle).copied().flatten().is_some_and(|(_, verdict)| {
            verdict.operation == operation && verdict.said == PreparationVerdict::Prepared && verdict.plan != expected
        })).collect()
    }

    /// The handles whose preparation of `operation` failed.
    pub fn failed(&self, operation: PeerOperation) -> Vec<usize> {
        (0..self.handles)
            .filter(|handle| self.said(*handle, operation) == PreparationVerdict::Failed)
            .collect()
    }
}

#[cfg(test)]
mod the_bytes_two_peers_exchange {
    //! The peer input on the wire. `ggrs` encodes `Config::Input` with bincode
    //! 1, which is positional and writes no field names. The bytes of the
    //! `ControlFrame` part are pinned in `ambition_platformer2d_core`'s module
    //! of the same name; this pins the whole input.

    use super::*;

    fn plan() -> PeerPlanIdentity {
        PeerPlanIdentity { incoming_content: [1; 32], expected_content: [2; 32], data_digest: 7 }
    }

    /// A non-default verdict, so the order and the variant index are visible.
    fn an_input() -> PeerInput {
        PeerInput {
            control: ControlFrame {
                axis_x: -0.75,
                jump_pressed: true,
                ..ControlFrame::default()
            },
            verdict: PeerVerdict {
                operation: PeerOperation {
                    recorded_on: 0x0102_0304,
                    intent_checksum: 0x0102_0304_0506_0708,
                    preparation_checksum: 9,
                },
                plan: plan(),
                said: PreparationVerdict::Failed,
            },
        }
    }

    /// The control part (72 bytes), then `operation` as a little-endian
    /// `i32`, the intent and preparation checksums, both content identities, the plan digest,
    /// and `Failed` as a `u32` variant index.
    const RECORDED: &str = "000040bf0000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040302010807060504030201090000000000000001010101010101010101010101010101010101010101010101010101010101010202020202020202020202020202020202020202020202020202020202020202070000000000000002000000";

    const RECORDED_WIRE_BYTES: u64 = 168;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn the_bytes_a_peer_decodes_are_pinned() {
        let bytes = bincode::serialize(&an_input()).expect("a PeerInput serializes");
        assert_eq!(
            hex(&bytes),
            RECORDED,
            "THE PEER INPUT PAYLOAD CHANGED SHAPE. If the change is deliberate, \
             bump `CONTROL_FRAME_WIRE_IDENTITY`, re-record these bytes and the \
             input-payload baseline, and say in the commit message what a peer \
             of the previous shape decodes from the new payload."
        );
        let back: PeerInput = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(back, an_input(), "the input does not survive the round trip ggrs performs");
    }

    /// Each handle's input of one frame goes into one payload with no length,
    /// and the receiver divides the total by the handle count.
    #[test]
    fn each_input_has_the_same_width() {
        for said in [PreparationVerdict::NotYet, PreparationVerdict::Prepared, PreparationVerdict::Failed] {
            for plan in [PeerPlanIdentity::default(), plan()] {
                let mut input = an_input();
                input.verdict.said = said;
                input.verdict.plan = plan;
                for preference in 0..=8 {
                input.control.party_preference = preference.try_into().unwrap();
                assert_eq!(bincode::serialized_size(&input).expect("serialized size"), RECORDED_WIRE_BYTES);
                let bytes = bincode::serialize(&input).unwrap();
                assert_eq!(bincode::deserialize::<PeerInput>(&bytes).unwrap(), input);
                }
            }
        }
        assert_eq!(bincode::serialized_size(&PeerInput::default()).unwrap(), RECORDED_WIRE_BYTES);
    }

    #[test]
    fn a_verdict_is_of_one_operation_and_the_newest_confirmed_frame_wins() {
        let key = |recorded_on| PeerOperation { recorded_on, intent_checksum: 7, preparation_checksum: 9 };
        let prepared = |frame| PeerVerdict { operation: key(frame), plan: plan(), said: PreparationVerdict::Prepared };
        let failed = |frame| PeerVerdict { operation: key(frame), plan: plan(), said: PreparationVerdict::Failed };
        let mut not_a_peer_session = PeerVerdicts::default();
        not_a_peer_session.record(0, 40, prepared(30));
        assert!(not_a_peer_session.prepared_plan(key(30)).is_none(), "a session with no handles");
        let mut verdicts = PeerVerdicts::for_handles(2);
        verdicts.record(0, 40, prepared(30));
        assert!(verdicts.prepared_plan(key(30)).is_none(), "handle 1 said nothing yet");
        verdicts.record(1, 38, failed(30));
        assert_eq!(verdicts.failed(key(30)), vec![1]);
        verdicts.record(1, 41, prepared(30));
        assert!(verdicts.prepared_plan(key(30)).is_some());
        assert!(verdicts.prepared_plan(key(31)).is_none(), "a verdict names its operation");
        // A rollback runs an older confirmed frame again after a newer one.
        verdicts.record(1, 39, failed(30));
        assert!(verdicts.prepared_plan(key(30)).is_some(), "an older frame does not replace a newer one");
    }

    #[test]
    fn verdicts_for_a_replaced_intent_do_not_prepare_or_fail_its_replacement() {
        use ambition_platformer2d_session::lifecycle_commit::{
            LifecycleIntent, PendingIntent, RoomReconstitutionIntent,
        };
        let intent = |room: &str| PendingIntent {
            frame: 30, checkpoint_operation: None,
            kind: LifecycleIntent::ReconstituteRoom(RoomReconstitutionIntent {
                target_room: room.into(),
            }),
        };
        let key = |intent: PendingIntent| PeerOperation { recorded_on: intent.frame, intent_checksum: intent.checksum(), preparation_checksum: 9 };
        let old = key(intent("old"));
        let replacement = key(intent("replacement"));
        let mut verdicts = PeerVerdicts::for_handles(2);
        verdicts.record(0, 40, PeerVerdict { operation: old, plan: plan(), said: PreparationVerdict::Prepared });
        verdicts.record(1, 40, PeerVerdict { operation: old, plan: plan(), said: PreparationVerdict::Failed });
        assert!(verdicts.prepared_plan(replacement).is_none());
        assert!(verdicts.failed(replacement).is_empty());
        verdicts.record(1, 41, PeerVerdict { operation: old, plan: plan(), said: PreparationVerdict::Prepared });
        assert!(verdicts.prepared_plan(old).is_some());
        assert!(verdicts.prepared_plan(replacement).is_none());
    }

    #[test]
    fn verdicts_for_an_older_capture_do_not_prepare_or_fail_the_current_capture() {
        let old = PeerOperation { recorded_on: 30, intent_checksum: 7, preparation_checksum: 9 };
        let current = PeerOperation { preparation_checksum: 10, ..old };
        let mut verdicts = PeerVerdicts::for_handles(2);
        for said in [PreparationVerdict::Prepared, PreparationVerdict::Failed] {
            for handle in 0..2 {
                verdicts.record(handle, 40, PeerVerdict { operation: old, plan: plan(), said });
            }
            assert!(verdicts.prepared_plan(current).is_none());
            assert!(verdicts.failed(current).is_empty());
            assert!(verdicts.mismatched_plans(current, plan()).is_empty());
        }
        for handle in 0..2 {
            verdicts.record(handle, 41, PeerVerdict { operation: current, plan: plan(), said: PreparationVerdict::Prepared });
        }
        assert_eq!(verdicts.prepared_plan(current), Some(plan()));
    }

    #[test]
    fn preparation_requires_the_same_content_and_frozen_plan() {
        let operation = PeerOperation { recorded_on: 30, intent_checksum: 7, preparation_checksum: 9 };
        let expected = plan();
        let mut changed_incoming = expected;
        changed_incoming.incoming_content[0] ^= 1;
        let mut changed_live = expected;
        changed_live.expected_content[0] ^= 1;
        let mut changed_data = expected;
        changed_data.data_digest ^= 1;
        for other in [changed_incoming, changed_live, changed_data, PeerPlanIdentity::default()] {
            let mut verdicts = PeerVerdicts::for_handles(2);
            verdicts.record(0, 40, PeerVerdict { operation, plan: expected, said: PreparationVerdict::Prepared });
            verdicts.record(1, 40, PeerVerdict { operation, plan: other, said: PreparationVerdict::Prepared });
            assert_eq!(verdicts.prepared_plan(operation), None);
            assert_eq!(verdicts.mismatched_plans(operation, expected), vec![1]);
            verdicts.record(1, 41, PeerVerdict { operation, plan: expected, said: PreparationVerdict::Prepared });
            assert_eq!(verdicts.prepared_plan(operation), Some(expected));
        }
        let mut unstated = PeerVerdicts::for_handles(2);
        for handle in 0..2 {
            unstated.record(handle, 40, PeerVerdict { operation, plan: Default::default(), said: PreparationVerdict::Prepared });
        }
        assert_eq!(unstated.prepared_plan(operation), None);
    }

}
