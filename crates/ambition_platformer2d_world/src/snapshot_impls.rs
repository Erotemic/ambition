//! `SnapshotState` for this crate's own types — the rollback wire format.
//!
//! ⚠ These impls live HERE, beside the types they encode, because
//! `ambition_platformer2d_core::snapshot` owns the trait and the orphan rule binds an
//! impl to the crate owning the trait OR the type. Until 2026-07-30 the trait
//! sat in `ambition_platformer2d_runtime`, above every domain crate, so the only place all
//! ~100 of them could compile was one 2688-line file in `ambition_platformer2d_runtime`. The
//! orphan rule is what proves this file is in the right crate: if a type moves,
//! this stops compiling rather than drifting.
//!
//! ⚠ A field added to an encoded type is a WIRE FORMAT change. Encode and
//! decode must stay in the same order, and `snapshot_unit_enum!` codes are
//! authored per variant so inserting one never renumbers the rest.

use ambition_platformer2d_core::snapshot::{
    put_str,
    Reader, SnapshotState,
};

/// **The active room's live moving platforms.** Each platform's `pos` and motion
/// cursor are advanced every tick by `advance_moving_platforms`, and the state
/// lives only in this resource (the visual entities carry an index into it), so
/// a within-room rollback must restore it or the platforms resume from the tick
/// we rewound FROM. The codec defers to `ambition_platformer2d_world`'s RON round-trip, which
/// keeps the private `MovingPlatformMotion` cursor encapsulated where it is owned.
impl SnapshotState for crate::collision::MovingPlatformSet {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, &self.to_snapshot_ron());
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Self::from_snapshot_ron(r.str()?)
    }
}
