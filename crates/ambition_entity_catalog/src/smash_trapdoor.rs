//! Authored payload for entering and leaving the submerged body mode.
//! The move authors separate submerge and surface beats. The movement kernel owns submerged motion and surface placement.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const TRAPDOOR: &str = "smash.trapdoor";

/// One beat of a trapdoor: going under, or coming back up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrapdoorParams {
    /// If true, enter submerged mode; if false, surface. A complete move must author both beats.
    pub submerge: bool,
    /// Maximum upward floor-search distance used when surfacing. Ignored when submerging.
    #[serde(default)]
    pub surface_reach: f32,
    /// Upward exit speed when surfacing, in world pixels per second. The surfacing beat is the sole writer of this exit velocity.
    #[serde(default)]
    pub leap_speed: f32,
    /// The effect drawn at the door.
    pub vfx: String,
    /// The cue played at the door.
    pub sfx: String,
}

/// Author one trapdoor beat on a move timeline.
///
/// # Panics
///
/// Panics if `at_s` is after the move duration.
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
