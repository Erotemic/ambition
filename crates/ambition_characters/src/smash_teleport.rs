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
    /// authored before this field meant) is AIMED, which is the recovery.
    /// `true` puts the fighter on the far side of the nearest foe, an ambush
    /// rather than an escape.
    ///
    /// ⭐⭐ AIMED MEANS THE WINDOW, AND THE WINDOW IS THE MOVE'S OWN STARTUP.
    /// Jon, on the style: *"a small window to input any direction and the user
    /// can aim the teleport like that but it defaults to up."* Any direction
    /// the player gives between the press and the transit aims it — a flick
    /// they have already let go of counts, which is the whole reason the move
    /// carries a latch (`MovePlayback::aimed_stick`) instead of reading the
    /// stick at the transit. A player who gives NONE goes straight up.
    ///
    /// ⛔⛔ AND NEVER FORWARD. The aim used to come back through the held-item
    /// helper, whose neutral answer is the body's FACING — so an unaimed
    /// recovery fired horizontally off whichever side of the stage the fighter
    /// happened to be looking at, and on stage the ledge assist then caught it
    /// and read as *"it just blinks me to the ledge"*. A recovery's honest
    /// default is up; there is no reading of "asked for nothing" that means
    /// "throw me sideways".
    ///
    /// ⛔⛔ A BOOL AND AN f32, NOT THE ENUM THIS OBVIOUSLY WANTS TO BE. Params
    /// travel as [`ParamValue`], which is a `ron::Value`, and a `ron::Value`
    /// CANNOT CARRY AN ENUM: a struct variant round-trips out of one as a map
    /// and a unit variant as a unit, so `hydrate` fails either way — at the
    /// moment the move fires, as a logged warning and a special that silently
    /// does nothing. Every other technique's params in this crate are primitives
    /// and tuples for the same reason; this comment is that rule written down.
    ///
    /// ⚠ WITH NO FOE IN REACH IT DOES NOTHING AT ALL — not "goes somewhere
    /// default", and not "goes as far as it can toward him". A teleport that
    /// fires into empty space because there was nobody to get behind spends the
    /// move and puts the fighter somewhere nobody asked for; standing still is
    /// the honest failure. [`Self::distance`] is the range that decides it.
    #[serde(default)]
    pub behind_nearest_foe: bool,
    /// World px between the foe's EDGE and the arriving fighter's, when
    /// [`Self::behind_nearest_foe`]. Ignored by an aimed teleport.
    ///
    /// ⛔ FROM THE EDGE, NOT THE CENTRE, so a fighter arrives the same distance
    /// behind a small body and a large one. Arriving inside a Bowser is not the
    /// same move as arriving behind him. The other axis follows the same rule:
    /// the arriving fighter's FEET are placed at the foe's feet, not her centre
    /// at his centre, so a height difference does not bury her or stand her on
    /// his shoulders.
    #[serde(default)]
    pub behind_gap: f32,
    /// How far the teleport carries, walls permitting, in world px.
    ///
    /// ⛔⛔ FOR AN AMBUSH THIS IS A RANGE, NOT A LEASH, and the difference is
    /// the whole move. A foe further away than this is NOT A TARGET and the
    /// teleport refuses; it does not carry the fighter this far along the line
    /// toward him, which would land her in front of him — or inside him — having
    /// spent the move to reach the worst position on the stage. Within the
    /// range she travels however far the far side of him actually is.
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
    /// How long the fighter is INTANGIBLE from the transit, in seconds of the
    /// move's own clock. `0.0` (the default, and what every teleport authored
    /// before this field meant) is no intangibility at all.
    ///
    /// ⭐⭐ THE GENRE'S ANSWER, AND IT IS A KNOB RATHER THAN A RULE. A teleport
    /// recovery that can be struck while it is nowhere is a coin flip: the
    /// fighter is off-stage, committed, and the one frame that decides the stock
    /// is the one where the body has no honest position. Every teleport recovery
    /// in the genre answers this the same way, so this is research rather than a
    /// design question — but it ships as a number a move sets, because a teleport
    /// that is an AMBUSH ([`Self::behind_nearest_foe`]) wants `0.0` and would be
    /// a different move with it.
    ///
    /// ⛔ IT DELIBERATELY ENDS BEFORE THE MOVE DOES. The tail of a recovery is
    /// what makes it punishable, and a window that ran to the end would hand
    /// back the commitment the move is supposed to cost.
    #[serde(default)]
    pub intangible_s: f32,
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
    // ⭐⭐ EVERY AUTHORED TELEPORT IS A WAY HOME, and it is stated HERE rather
    // than by each author, because the fact is a property of teleporting and not
    // of whose move it is. A teleport commands no impulse, so `lift_speed` reads
    // `0.0` and the recovery planner — which modelled every route as one thrown
    // velocity — could not see it (D250). ⛔ Not by fabricating a lift: a burst
    // the move never throws would have the search certify a rise that does not
    // happen. What it offers is a DISCONTINUITY of a stated size.
    // ⭐ THE INTANGIBILITY IS STATED HERE for the same reason the route above is:
    // it is a property of TELEPORTING, not of whose move it is, so the two
    // fighters that share this technique cannot drift apart on it. The mechanism
    // already existed and nothing authored one — `WindowTag::Invuln` becomes
    // `Invulnerability::MOVE` in `project_move_defense_windows`, written every
    // tick for every combat body, which is what makes the grant RETRACT when the
    // window closes.
    //
    // ⛔ CLAMPED TO THE MOVE. An `end_s` past `duration_s` is a window that never
    // closes on this timeline, and the clamp is silent because the honest reading
    // of "intangible longer than the move lasts" is "intangible for the move".
    // ⛔ THROUGH THE SHARED HELPER, not a hand-pushed window. The Actress's
    // trapdoor already authors one this way, and two spellings of "the owner
    // cannot be hit here" is how the two drift.
    let mut spec = spec;
    if params.intangible_s > 0.0 {
        // ⛔ CLAMPED TO THE MOVE. An `end_s` past `duration_s` is a window that
        // never closes on this timeline, and the honest reading of "intangible
        // longer than the move lasts" is "intangible for the move".
        let ends = (at_s + params.intangible_s).min(spec.duration_s);
        spec = crate::moveset_authoring::invuln(spec, at_s, ends);
    }
    spec.gates.recovery_route = Some(ambition_entity_catalog::AuthoredRecoveryRoute::Teleport {
        distance: params.distance,
    });
    spec
}
