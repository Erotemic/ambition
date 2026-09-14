//! Authored payload for a homing dash.
//! Combat targeting chooses the target. This module only defines the motion and targeting limits used by the dash.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key.
pub const HOMING_DASH: &str = "smash.homing_dash";

/// Authored parameters of one homing dash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HomingDashParams {
    /// How fast the fighter travels while homing, in world px per second.
    pub speed: f32,
    /// Homing duration in seconds. It should end before the move recovery finishes.
    pub duration_s: f32,
    /// Maximum target angle from the commanded direction, in degrees. Values above 90 degrees are rejected so the move cannot acquire targets behind the fighter.
    pub cone_degrees: f32,
    /// How far a foe may be and still attract the dash, in world px. Past this
    /// the fighter goes where they were pointing and nothing more.
    pub max_range: f32,
}

/// Author a homing dash on a move timeline.
///
/// # Panics
///
/// Panics if `at_s` is after the move duration or if the targeting cone is invalid.
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
