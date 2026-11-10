//! Fixed-width peer inputs carry seat controls and a compact room-preparation
//! token.
//!
//! Ordinary crossings use the tokens of the simulated frame. Rebuild barriers
//! use confirmed tokens. Both require the exact admission and frozen plan: a
//! peer compares each handle's token with the token of its OWN operation and
//! plan, so the full content identities never cross per frame (they are
//! agreed once, at session handshake, by the content identity).

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

impl PeerOperation {
    /// The operation's 32 bits of a [`PeerVerdictToken`].
    pub fn wire_digest(self) -> u32 {
        let digest = ambition_platformer2d_core::snapshot::PeerDigest::in_domain("peer_input.operation")
            .u64(self.recorded_on as u32 as u64)
            .u64(self.intent_checksum)
            .u64(self.preparation_checksum)
            .finish();
        (digest ^ (digest >> 32)) as u32
    }
}

impl PeerPlanIdentity {
    /// The plan's 30 bits of a [`PeerVerdictToken`]; never zero for a stated
    /// plan, so a `Prepared` token always names one.
    pub fn wire_digest(self) -> u32 {
        if !self.is_stated() {
            return 0;
        }
        let digest = ambition_platformer2d_core::snapshot::PeerDigest::in_domain("peer_input.plan")
            .bytes(&self.incoming_content)
            .bytes(&self.expected_content)
            .u64(self.data_digest)
            .finish();
        let bits = ((digest ^ (digest >> 32)) as u32) & PLAN_MASK;
        bits.max(1)
    }
}

/// One peer's verdict on one operation, as this machine holds it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerVerdict {
    /// The frame and intent must match. A rollback can replace an intent
    /// without changing its recording frame.
    pub operation: PeerOperation,
    pub plan: PeerPlanIdentity,
    pub said: PreparationVerdict,
}

impl PeerVerdict {
    /// What this verdict puts on the wire.
    pub fn token(self) -> PeerVerdictToken {
        let said = match self.said {
            PreparationVerdict::NotYet => return PeerVerdictToken::default(),
            PreparationVerdict::Prepared => 1u64,
            PreparationVerdict::Failed => 2u64,
        };
        let plan = match self.said {
            PreparationVerdict::Prepared => self.plan.wire_digest(),
            _ => 0,
        };
        PeerVerdictToken {
            word: (said << 62) | (u64::from(plan & PLAN_MASK) << 32) | u64::from(self.operation.wire_digest()),
        }
    }
}

const PLAN_MASK: u32 = (1 << 30) - 1;

/// One handle's verdict on the wire: 8 bytes, every frame.
///
/// Bits 0..32 are the operation's digest, 32..62 the frozen plan's (zero
/// unless `Prepared`), 62..64 the verdict (0 `NotYet`, 1 `Prepared`,
/// 2 `Failed`). `NotYet` is the zero word whatever the operation.
///
/// ⚠ THE DIGESTS ARE NOT THE IDENTITIES. A receiver never reads a plan off
/// the wire; it asks whether a handle's token equals the token of its own
/// operation and plan. Two different plans for one operation agree only if
/// their 30-bit digests collide (about one in 10⁹ per mismatch). This check is
/// the second line of defence: peers whose content differs are refused first,
/// at session handshake, by the full content identity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerVerdictToken {
    pub word: u64,
}

impl PeerVerdictToken {
    /// What the handle said, of whatever operation it names.
    pub fn said(self) -> PreparationVerdict {
        match self.word >> 62 {
            0 => PreparationVerdict::NotYet,
            1 => PreparationVerdict::Prepared,
            // 3 is no verdict this build writes; it refuses rather than agrees.
            _ => PreparationVerdict::Failed,
        }
    }

    /// The verdict this handle states of `operation`: `NotYet` when its
    /// token names another operation, or none.
    pub fn of(self, operation: PeerOperation) -> PreparationVerdict {
        match self.said() {
            PreparationVerdict::NotYet => PreparationVerdict::NotYet,
            said if self.word as u32 == operation.wire_digest() => said,
            _ => PreparationVerdict::NotYet,
        }
    }

    /// The plan digest of a `Prepared` token.
    pub fn plan_digest(self) -> u32 {
        ((self.word >> 32) as u32) & PLAN_MASK
    }
}

/// `GgrsConfig::Input`: what crosses between peers for one handle and one
/// frame.
///
/// The fields are positional on the wire (bincode), so the order is part of
/// the format. `CONTROL_FRAME_WIRE_IDENTITY` names it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PeerInput {
    pub control: ControlFrame,
    pub verdict: PeerVerdictToken,
}

/// This peer's verdict, decided before GGRS reads the local inputs of a frame
/// and put in the input of each local handle. Host state: it is a fact about
/// this machine, and a rewind does not change it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ThisPeersVerdict(pub PeerVerdict);

/// The last CONFIRMED token of each handle of the live session.
///
/// Host state, and never rewound: a confirmed input does not change. A
/// token recorded from a predicted input could be the default that GGRS
/// predicts, so only confirmed inputs are recorded. The newest frame wins.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct PeerVerdicts {
    /// The handles of the session, each of which must say `Prepared`. Zero
    /// outside a peer session.
    handles: usize,
    /// By handle: the frame of the input, and its token.
    said: Vec<Option<(i32, PeerVerdictToken)>>,
}

impl PeerVerdicts {
    /// No verdicts yet, from a session of `handles` handles.
    pub fn for_handles(handles: usize) -> Self {
        Self { handles, said: Vec::new() }
    }

    /// Record the token that `handle` sent in its confirmed input of `frame`.
    pub fn record(&mut self, handle: usize, frame: i32, token: PeerVerdictToken) {
        if self.said.len() <= handle {
            self.said.resize(handle + 1, None);
        }
        let slot = &mut self.said[handle];
        if slot.is_none_or(|(seen, _)| seen <= frame) {
            *slot = Some((frame, token));
        }
    }

    fn token(&self, handle: usize) -> Option<PeerVerdictToken> {
        self.said.get(handle).copied().flatten().map(|(_, token)| token)
    }

    /// What `handle` last said of `operation`. `NotYet` when its last token
    /// is of another operation, or when nothing of it is confirmed.
    pub fn said(&self, handle: usize, operation: PeerOperation) -> PreparationVerdict {
        self.token(handle).map_or(PreparationVerdict::NotYet, |token| token.of(operation))
    }

    /// Every handle prepared `operation`, each with the same stated plan. Which
    /// plan is the barrier's question ([`Self::each_prepared`]).
    pub fn agreed(&self, operation: PeerOperation) -> bool {
        let mut tokens = (0..self.handles).map(|handle| self.token(handle));
        let Some(Some(first)) = tokens.next() else {
            return false;
        };
        first.of(operation) == PreparationVerdict::Prepared
            && first.plan_digest() != 0
            && tokens.all(|token| token == Some(first))
    }

    /// Every handle prepared `operation` with this host's frozen `plan`.
    pub fn each_prepared(&self, operation: PeerOperation, plan: PeerPlanIdentity) -> bool {
        each_prepared(operation, plan, (0..self.handles).map(|handle| self.token(handle)))
    }

    /// Prepared replies that disagree with this host's frozen plan.
    pub fn mismatched_plans(&self, operation: PeerOperation, expected: PeerPlanIdentity) -> Vec<usize> {
        (0..self.handles)
            .filter(|handle| self.token(*handle).is_some_and(|token| {
                token.of(operation) == PreparationVerdict::Prepared && token.plan_digest() != expected.wire_digest()
            }))
            .collect()
    }

    /// The handles whose preparation of `operation` failed.
    pub fn failed(&self, operation: PeerOperation) -> Vec<usize> {
        (0..self.handles)
            .filter(|handle| self.said(*handle, operation) == PreparationVerdict::Failed)
            .collect()
    }
}

/// Every one of a complete set of input handles says it prepared `operation`
/// with `plan`. No host history is read.
pub(crate) fn each_prepared(operation: PeerOperation, plan: PeerPlanIdentity,
    tokens: impl IntoIterator<Item = Option<PeerVerdictToken>>,
) -> bool {
    if !plan.is_stated() {
        return false;
    }
    let expected = PeerVerdict { operation, plan, said: PreparationVerdict::Prepared }.token();
    let mut tokens = tokens.into_iter().peekable();
    tokens.peek().is_some() && tokens.all(|token| token == Some(expected))
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

    fn operation() -> PeerOperation {
        PeerOperation { recorded_on: 0x0102_0304, intent_checksum: 0x0102_0304_0506_0708, preparation_checksum: 9 }
    }

    fn verdict(operation: PeerOperation, plan: PeerPlanIdentity, said: PreparationVerdict) -> PeerVerdictToken {
        PeerVerdict { operation, plan, said }.token()
    }

    /// A non-default verdict, so the order and the verdict bits are visible.
    fn an_input() -> PeerInput {
        PeerInput {
            control: ControlFrame {
                axis_x: -0.75,
                jump_pressed: true,
                ..ControlFrame::default()
            },
            verdict: verdict(operation(), plan(), PreparationVerdict::Prepared),
        }
    }

    /// The control part (72 bytes), then the token as a little-endian `u64`:
    /// the operation's digest, the plan's 30 bits, and `Prepared` in the top
    /// two bits.
    const RECORDED: &str = "000040bf0000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000b35d586481ca454c";

    /// NET-INPUT-WIDTH: at most 80 bytes a handle a frame. It was 168, of
    /// which 96 were the verdict with its two 32-byte content identities.
    const RECORDED_WIRE_BYTES: u64 = 80;

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
                input.verdict = verdict(operation(), plan, said);
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

    /// Two local players' inputs of one frame are one payload, divided by the
    /// handle count on the other side.
    #[test]
    fn two_local_players_pack_and_divide_back() {
        let mut second = an_input();
        second.control.axis_x = 0.5;
        second.verdict = verdict(operation(), PeerPlanIdentity::default(), PreparationVerdict::Failed);
        let mut payload = bincode::serialize(&an_input()).unwrap();
        payload.extend(bincode::serialize(&second).unwrap());
        assert_eq!(payload.len() as u64, 2 * RECORDED_WIRE_BYTES);
        let width = payload.len() / 2;
        let back: Vec<PeerInput> =
            payload.chunks(width).map(|chunk| bincode::deserialize(chunk).unwrap()).collect();
        assert_eq!(back, vec![an_input(), second]);
    }

    /// The token says what the verdict said, of the operation it names, and
    /// nothing of another.
    #[test]
    fn a_token_states_its_verdict_of_its_operation_only() {
        let other = PeerOperation { preparation_checksum: 10, ..operation() };
        assert_eq!(PeerVerdictToken::default().said(), PreparationVerdict::NotYet);
        assert_eq!(verdict(operation(), plan(), PreparationVerdict::NotYet), PeerVerdictToken::default());
        for said in [PreparationVerdict::Prepared, PreparationVerdict::Failed] {
            let token = verdict(operation(), plan(), said);
            assert_eq!(token.said(), said);
            assert_eq!(token.of(operation()), said);
            assert_eq!(token.of(other), PreparationVerdict::NotYet, "{said:?} names its operation");
        }
        assert_ne!(
            verdict(operation(), plan(), PreparationVerdict::Prepared).plan_digest(),
            0,
            "a prepared token always names a plan"
        );
        assert_eq!(verdict(operation(), plan(), PreparationVerdict::Failed).plan_digest(), 0);
        assert_eq!(PeerVerdictToken { word: 3 << 62 }.said(), PreparationVerdict::Failed, "an unknown verdict refuses");
    }

    #[test]
    fn a_verdict_is_of_one_operation_and_the_newest_confirmed_frame_wins() {
        let key = |recorded_on| PeerOperation { recorded_on, intent_checksum: 7, preparation_checksum: 9 };
        let prepared = |frame| verdict(key(frame), plan(), PreparationVerdict::Prepared);
        let failed = |frame| verdict(key(frame), plan(), PreparationVerdict::Failed);
        let mut not_a_peer_session = PeerVerdicts::default();
        not_a_peer_session.record(0, 40, prepared(30));
        assert!(!not_a_peer_session.each_prepared(key(30), plan()), "a session with no handles");
        let mut verdicts = PeerVerdicts::for_handles(2);
        verdicts.record(0, 40, prepared(30));
        assert!(!verdicts.each_prepared(key(30), plan()), "handle 1 said nothing yet");
        verdicts.record(1, 38, failed(30));
        assert_eq!(verdicts.failed(key(30)), vec![1]);
        verdicts.record(1, 41, prepared(30));
        assert!(verdicts.each_prepared(key(30), plan()));
        assert!(!verdicts.each_prepared(key(31), plan()), "a verdict names its operation");
        // A rollback runs an older confirmed frame again after a newer one.
        verdicts.record(1, 39, failed(30));
        assert!(verdicts.each_prepared(key(30), plan()), "an older frame does not replace a newer one");
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
        verdicts.record(0, 40, verdict(old, plan(), PreparationVerdict::Prepared));
        verdicts.record(1, 40, verdict(old, plan(), PreparationVerdict::Failed));
        assert!(!verdicts.each_prepared(replacement, plan()));
        assert!(verdicts.failed(replacement).is_empty());
        verdicts.record(1, 41, verdict(old, plan(), PreparationVerdict::Prepared));
        assert!(verdicts.each_prepared(old, plan()));
        assert!(!verdicts.each_prepared(replacement, plan()));
    }

    #[test]
    fn verdicts_for_an_older_capture_do_not_prepare_or_fail_the_current_capture() {
        let old = PeerOperation { recorded_on: 30, intent_checksum: 7, preparation_checksum: 9 };
        let current = PeerOperation { preparation_checksum: 10, ..old };
        let mut verdicts = PeerVerdicts::for_handles(2);
        for said in [PreparationVerdict::Prepared, PreparationVerdict::Failed] {
            for handle in 0..2 {
                verdicts.record(handle, 40, verdict(old, plan(), said));
            }
            assert!(!verdicts.each_prepared(current, plan()));
            assert!(verdicts.failed(current).is_empty());
            assert!(verdicts.mismatched_plans(current, plan()).is_empty());
        }
        for handle in 0..2 {
            verdicts.record(handle, 41, verdict(current, plan(), PreparationVerdict::Prepared));
        }
        assert!(verdicts.each_prepared(current, plan()));
    }

    #[test]
    fn preparation_requires_the_same_content_and_frozen_plan() {
        let operation = PeerOperation { recorded_on: 30, intent_checksum: 7, preparation_checksum: 9 };
        let expected = plan();
        let mut changed_incoming = expected;
        changed_incoming.incoming_content[0] ^= 1;
        let mut changed_live = expected;
        changed_live.expected_content[31] ^= 1;
        let mut changed_data = expected;
        changed_data.data_digest ^= 1;
        for other in [changed_incoming, changed_live, changed_data] {
            let mut verdicts = PeerVerdicts::for_handles(2);
            verdicts.record(0, 40, verdict(operation, expected, PreparationVerdict::Prepared));
            verdicts.record(1, 40, verdict(operation, other, PreparationVerdict::Prepared));
            assert!(!verdicts.each_prepared(operation, expected));
            assert_eq!(verdicts.mismatched_plans(operation, expected), vec![1]);
            verdicts.record(1, 41, verdict(operation, expected, PreparationVerdict::Prepared));
            assert!(verdicts.each_prepared(operation, expected));
        }
        // An unstated plan is never agreed, whatever the handles say.
        let mut unstated = PeerVerdicts::for_handles(2);
        for handle in 0..2 {
            unstated.record(handle, 40, verdict(operation, Default::default(), PreparationVerdict::Prepared));
        }
        assert!(!unstated.each_prepared(operation, Default::default()));
    }
}
