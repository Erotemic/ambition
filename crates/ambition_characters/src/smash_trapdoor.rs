//! Going under the stage and coming back: the authored vocabulary.
//!
//! ⭐ THE SAME SPLIT `smash_teleport`, `smash_vitality` AND `smash_ride` USE. A
//! key and its params are what a MOVESET authors; putting a body into
//! [`BodyMode::Submerged`](ambition_platformer2d_core::player_state::BodyMode::Submerged)
//! and finding the floor it surfaces through is engine work.
//!
//! ⭐⭐ JON'S DESIGN, 2026-08-27: *"press down-b, a trap door opens, the actor
//! jumps into it, small pause, the exit trapdoor opens, and she jumps / pops out
//! of it… It's not a blink. It's a different kind of mobility move."* And then:
//! *"I do want the player to be able to control where they move."*
//!
//! ⛔⛔ SO IT IS TWO EVENTS, NOT ONE. A teleport is a single beat that computes
//! a destination; this is a beat that takes her OUT of the world and a later
//! beat that puts her back, with the time between them belonging to the player.
//! The pause Jon asked for is not a delay the engine sleeps through — it is the
//! part of the move she is playing.
//!
//! ⛔ AND THE MODE IS NOT AUTHORED HERE. What "under the stage" means — no
//! gravity, no geometry, nothing can hit her, the stick still steers — is a
//! property of the BODY MODE, stated once in the movement kernel. This module
//! only says WHEN.

use serde::{Deserialize, Serialize};

use ambition_entity_catalog::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const TRAPDOOR: &str = "smash.trapdoor";

/// One beat of a trapdoor: going under, or coming back up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrapdoorParams {
    /// `true` drops her through the boards; `false` brings her back up.
    ///
    /// ⭐ ONE KEY WITH A DIRECTION, not `smash.submerge` plus
    /// `smash.surface`. The two beats share everything that matters — the same
    /// mode, the same door, the same wooden report — and a second key would
    /// have duplicated all of it to change which way she is going. It is the
    /// same reasoning `TeleportParams::behind_nearest_foe` is written down for.
    ///
    /// ⛔⛔ AND A MOVE THAT AUTHORS ONLY THE FIRST IS A FIGHTER WHO NEVER COMES
    /// BACK. Nothing in the engine ends the mode on its own; `author_trapdoor`
    /// cannot see the rest of the timeline, so the guard against that lives in
    /// the content test that walks her table.
    pub submerge: bool,
    /// How far above her the engine will look for a floor to surface through,
    /// in world px. Ignored when going under.
    ///
    /// ⛔ SHE COMES UP THROUGH A SURFACE, NOT AT A POINT. She has been steering
    /// under the stage and the player has no way to know where the boards end;
    /// surfacing at her exact position would put her inside the floor she was
    /// travelling beneath, or drop her out of the bottom of the world if she
    /// wandered past its edge. The same rule the teleport's ledge assist uses,
    /// and the same function.
    #[serde(default)]
    pub surface_reach: f32,
    /// How hard she LEAPS out of the boards, against gravity, in px/s.
    /// Ignored when going under.
    ///
    /// ⛔⛔ IT LIVES HERE BECAUSE THE SURFACING BEAT IS THE ONE WRITER OF EXIT
    /// VELOCITY, and for a while it was two. The move ALSO authored a
    /// `MoveEventKind::Impulse` at the same instant — deliberately, so *"the
    /// placement and the launch cannot disagree about where she left from"* —
    /// but an impulse is applied inline in `advance_move_playback` while this
    /// beat is a MESSAGE dispatched to a later system, and the later system's
    /// `TransitVelocity::Zero` overwrote it every time. The two did not
    /// disagree; one of them was deleted, and `LEAP_OUT_SPEED = 430.0` was dead
    /// content nobody could see from either file alone.
    ///
    /// ⇒ so the placement and the launch are ONE write, which is what the
    /// original comment was reaching for. `0.0` surfaces her standing.
    #[serde(default)]
    pub leap_speed: f32,
    /// The effect drawn at the door.
    pub vfx: String,
    /// The cue played at the door.
    pub sfx: String,
}

/// Author one trapdoor beat onto a move's timeline.
///
/// # Panics
///
/// If `at_s` is past the move's own duration — a beat scheduled after the move
/// ends never fires, and for the SURFACING beat that is a fighter left under the
/// stage for the rest of the match.
pub fn author_trapdoor(mut spec: MoveSpec, at_s: f32, params: TrapdoorParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` opens a trapdoor at {at_s}s but only lasts {}s, so the beat \
         would never fire",
        spec.id,
        spec.duration_s,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: TRAPDOOR.to_string(),
            params: ParamValue::from_typed(&params).expect("trapdoor params serialize"),
        }),
    });
    spec
}
