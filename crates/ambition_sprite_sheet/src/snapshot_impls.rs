//! `SnapshotCursor` for this crate's own types — the rollback checksum wire.
//!
//! The orphan rule requires this impl to live in the crate that defines the
//! type.

use ambition_platformer2d_core::snapshot::{put_bool, put_f32, put_u32, put_u8, SnapshotCursor};

/// The boss's animation cursor: sim-owned, and gameplay geometry reads it.
///
/// A cursor projection: `spec` is the authored sheet contract and does not
/// change during a session. `current` / `drive_phase` / `frame` / `elapsed` /
/// `clip_held` advance every tick in `drive_boss_animators` on
/// `world_time.entity_dt`. `BossAnimationFrameSample` (the boss's active
/// hurtbox parts) is derived from those fields, so this is rollback state even
/// though this crate is otherwise sprite metadata.
impl SnapshotCursor for crate::boss::BossAnimFrame {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        put_u8(out, self.current as u8);
        put_u8(out, self.drive_phase as u8);
        put_u32(out, self.frame as u32);
        put_f32(out, self.elapsed);
        put_bool(out, self.clip_held);
    }
}
