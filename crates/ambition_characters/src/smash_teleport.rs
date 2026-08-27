//! Teleport-as-a-recovery: the authored vocabulary.
//!
//! ⭐ THE SAME SPLIT `smash_capture` AND `smash_ride` USE, and for the same
//! reason. A key and its params are what a MOVESET authors, so they live where
//! movesets can name them; resolving a destination against the collision world
//! is the ENGINE's job, and that half sits beside `blink_target` — the one
//! teleport rule every controller already shares.
//!
//! ⭐⭐ JON'S DESIGN, 2026-08-27: *"Mewtwo / Palutena / Zelda style teleports…
//! We need to be sure we have some sort of aim assist when the blinks are aimed
//! at a ledge."* The aim assist is the whole reason this is a technique rather
//! than an authored impulse: a recovery that vanishes and reappears is trivial
//! to write and unusable if it drops you a pixel under the stage.
//!
//! ⛔ THE LOOK IS AUTHORED, NOT BUILT IN. Jon asked for two teleports that
//! differ only in presentation — *"the animation for the author teleport up b is
//! different, instead of a phase-out effect, it is more of a affine transform to
//! a point, with a store of star flash for the blink out, and the opposite of
//! that for the blink in"* — so the effect ids travel in the params and the
//! engine draws whatever the move named.

use serde::{Deserialize, Serialize};

use ambition_entity_catalog::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const TELEPORT: &str = "smash.teleport";

/// WHERE a teleport goes: the thing a fighter's identity actually differs in.
///
/// ⭐ THE RECOVERY AND THE AMBUSH ARE ONE TECHNIQUE. Both resolve a destination
/// against the collision world, both want the wall clamp, both draw a departure
/// and an arrival; the only thing that differs is how the point is chosen. A
/// second technique key would have duplicated `blink_target`, the ledge assist
/// and the VFX plumbing to change one line.
/// Authored parameters of one teleport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeleportParams {
    /// WHERE this teleport goes: `false` (the default, and what every teleport
    /// authored before this field meant) is AIMED — the stick, then straight up,
    /// which is the recovery. `true` puts the fighter on the far side of the
    /// nearest foe, an ambush rather than an escape.
    ///
    /// ⛔⛔ A BOOL AND AN f32, NOT THE ENUM THIS OBVIOUSLY WANTS TO BE. Params
    /// travel as [`ParamValue`], which is a `ron::Value`, and a `ron::Value`
    /// CANNOT CARRY AN ENUM: a struct variant round-trips out of one as a map
    /// and a unit variant as a unit, so `hydrate` fails either way — at the
    /// moment the move fires, as a logged warning and a special that silently
    /// does nothing. Every other technique's params in this crate are primitives
    /// and tuples for the same reason; this comment is that rule written down.
    ///
    /// ⚠ WITH NO FOE ON THE STAGE IT DOES NOTHING AT ALL — not "goes somewhere
    /// default". A teleport that fires into empty space because there was nobody
    /// to get behind spends the move and puts the fighter somewhere nobody asked
    /// for; standing still is the honest failure.
    #[serde(default)]
    pub behind_nearest_foe: bool,
    /// World px between the foe's EDGE and the arriving fighter's, when
    /// [`Self::behind_nearest_foe`]. Ignored by an aimed teleport.
    ///
    /// ⛔ FROM THE EDGE, NOT THE CENTRE, so a fighter arrives the same distance
    /// behind a small body and a large one. Arriving inside a Bowser is not the
    /// same move as arriving behind him.
    #[serde(default)]
    pub behind_gap: f32,
    /// How far the teleport carries, walls permitting, in world px.
    pub distance: f32,
    /// How far from the resolved destination a LEDGE may be and still catch the
    /// arrival, in world px.
    ///
    /// ⭐⭐ THIS IS THE AIM ASSIST, and it is the difference between a recovery
    /// and a coin flip. A teleport aimed at a platform edge either lands on it
    /// or dies just under it, and the margin is a few pixels of stick angle no
    /// player can hold. Within this radius the arrival is placed STANDING on the
    /// ledge instead.
    ///
    /// ⛔ IT ONLY EVER HELPS UPWARD ONTO A SURFACE. It never moves an arrival
    /// that already had support, and it never pulls a fighter DOWN — a teleport
    /// that dragged you onto a platform you had cleared would be the assist
    /// taking the stage away from you.
    ///
    /// `0.0` disables it, which is what a teleport that is not a recovery wants.
    pub ledge_assist: f32,
    /// The effect drawn where the fighter LEFT.
    pub depart_vfx: String,
    /// The effect drawn where the fighter ARRIVED.
    pub arrive_vfx: String,
}

/// Author a teleport onto a move's timeline.
///
/// # Panics
///
/// If `at_s` is past the move's own duration. A teleport scheduled after the
/// move ends never fires, and the move would cost its recovery to do nothing —
/// exactly the failure this helper exists to catch at authoring time.
pub fn author_teleport(mut spec: MoveSpec, at_s: f32, params: TeleportParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` teleports at {at_s}s but only lasts {}s, so the teleport \
         would never fire and the move would spend a recovery to do nothing",
        spec.id,
        spec.duration_s,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: TELEPORT.to_string(),
            params: ParamValue::from_typed(&params).expect("teleport params serialize"),
        }),
    });
    spec
}
