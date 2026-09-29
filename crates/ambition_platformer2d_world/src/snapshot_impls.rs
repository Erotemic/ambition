//! `SnapshotState` for this crate's own types — the rollback wire format.
//!
//! These impls live beside the types they encode. The orphan rule keeps the
//! snapshot implementation with either the trait or the type, so moving a type
//! forces its codec to move with it.
//!
//! A field added to an encoded type is a wire format change. Encode and
//! decode must stay in the same order, and `snapshot_unit_enum!` codes are
//! authored per variant so inserting one never renumbers the rest.

use ambition_platformer2d_core::snapshot::{put_str, Reader, SnapshotState};

/// A live room's moving platforms. `advance_moving_platforms` moves each
/// platform's `pos` and motion cursor every tick, and the state lives only in
/// this component on the room's root (visual entities carry an index). A within-room rollback
/// must restore it. The codec uses the crate's RON round-trip, which keeps the
/// private `MovingPlatformMotion` cursor encapsulated.
impl SnapshotState for crate::collision::MovingPlatformSet {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, &self.to_snapshot_ron());
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Self::from_snapshot_ron(r.str()?)
    }
}
