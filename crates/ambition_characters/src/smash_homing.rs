//! Carry the fighter at whoever they were pointing at: the authored vocabulary.
//!
//! ⭐⭐ THE TARGET QUERY IS NOT NEW AND THIS TECHNIQUE DOES NOT OWN IT.
//! `ambition_combat::targeting::assisted_fire_direction` already answers "which
//! foe was I pointing at" deterministically — inside an authored cone, out to an
//! authored range, and **tie-broken on the stable `SimId` rather than the
//! `Entity`**, because bevy_ggrs destroys and recreates rollback entities and a
//! tie decided by a raw id picks a different target mid-resimulation than the
//! confirmed timeline did. ⇒ A homing move ASKS that question and steers on the
//! answer; the targeting domain keeps it.
//!
//! ⛔ SO WHAT IS ACTUALLY AUTHORED HERE IS THE MOTION, and only that: how fast,
//! for how long, and how wide a cone still counts as "the way I was pointing".
//!
//! ⭐ THE DAMAGE IS NOT HERE EITHER. A homing move is an ordinary strike whose
//! ACTIVE window happens to arrive where somebody is standing — so the hitbox,
//! the launch and the recovery are authored the way every other move authors
//! them, and this technique only decides where the fighter goes.
//!
//! The ruleset half is `ambition_demo_smash::homing`.

use serde::{Deserialize, Serialize};

use ambition_entity_catalog::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key.
pub const HOMING_DASH: &str = "smash.homing_dash";

/// Authored parameters of one homing dash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HomingDashParams {
    /// How fast the fighter travels while homing, in world px per second.
    pub speed: f32,
    /// How long the homing lasts. ⚠ It should END BEFORE THE MOVE DOES, or the
    /// fighter is still being carried through his own recovery and cannot be
    /// punished for missing.
    pub duration_s: f32,
    /// The widest angle from the commanded direction that still counts as "the
    /// way I was pointing", in degrees.
    ///
    /// ⭐⭐ THIS IS WHAT KEEPS IT A READ RATHER THAN A GUARANTEE. `90.0` is the
    /// half-plane — anybody in front of you. Narrow it and the move demands you
    /// point at them; widen it past 90 and it starts finding people behind you,
    /// which is a homing move nobody has to aim.
    pub cone_degrees: f32,
    /// How far a foe may be and still attract the dash, in world px. Past this
    /// the fighter goes where they were pointing and nothing more.
    pub max_range: f32,
}

/// Author a homing dash onto a move's timeline.
///
/// # Panics
///
/// If `at_s` is past the move's duration; if the cone is not positive (a dash
/// that can find nobody is a plain impulse wearing a technique's name); or if
/// the cone reaches behind the fighter, which makes the move unaimable.
pub fn author_homing_dash(mut spec: MoveSpec, at_s: f32, params: HomingDashParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` homes at {at_s}s but only lasts {}s",
        spec.id,
        spec.duration_s,
    );
    assert!(
        params.cone_degrees > 0.0,
        "move `{}` authors a {}° cone, so it can never find a target and is a \
         plain impulse wearing a technique's name",
        spec.id,
        params.cone_degrees,
    );
    assert!(
        params.cone_degrees <= 90.0,
        "move `{}` authors a {}° cone, which reaches BEHIND the fighter — a \
         homing move nobody has to aim is a tracking move, and this is not one",
        spec.id,
        params.cone_degrees,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: HOMING_DASH.to_string(),
            params: ParamValue::from_typed(&params).expect("homing-dash params serialize"),
        }),
    });
    spec
}
