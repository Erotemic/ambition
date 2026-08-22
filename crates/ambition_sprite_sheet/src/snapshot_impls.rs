//! `SnapshotCursor` for this crate's own types — the rollback checksum wire.
//!
//! One impl, and it is here only because the orphan rule put it here. It was
//! written in `ambition_platformer2d_runtime` against `ambition_platformer2d_actor_monolith`' RE-EXPORT of the
//! type, which reads as if actors owned it; the definition is in this crate.
//! The doc comment below was already arguing that the type is misfiled — the
//! carve did not discover that, it inherited it.

use ambition_platformer2d_core::snapshot::{put_bool, put_f32, put_u32, put_u8, SnapshotCursor};

/// The boss's animation cursor — sim-owned, and gameplay geometry reads it.
///
/// A cursor projection: `spec` is the authored sheet contract and never changes
/// during a session, while `current` / `drive_phase` / `frame` / `elapsed` /
/// `clip_held` are advanced every tick by `drive_boss_animators` on
/// `world_time.entity_dt`. `BossAnimationFrameSample` — the boss's ACTIVE HURTBOX
/// PARTS — is derived from exactly those fields.
///
/// It was not rollback state, and the coverage sweep could not say so: the type
/// lives in `ambition_sprite_sheet`, and that crate is waived wholesale as
/// "sprite metadata / asset binding". Its own first doc line says *Sim-owned*.
/// A crate-prefix waiver assumes a crate holds one kind of thing, and this one
/// swallowed authoritative combat state — the same shape as the equipment
/// oracle's `ProjectileOwner`, which was registered as a lie rather than waived
/// as one.
impl SnapshotCursor for crate::boss::BossAnimFrame {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        put_u8(out, self.current as u8);
        put_u8(out, self.drive_phase as u8);
        put_u32(out, self.frame as u32);
        put_f32(out, self.elapsed);
        put_bool(out, self.clip_held);
    }
}
