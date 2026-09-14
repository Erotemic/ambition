//! Authored payload for changing the mover's own health.
//! Positive values restore health; negative values pay a move cost through the existing health authority.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const VITALITY: &str = "smash.vitality";

/// What one authored change to the mover's own health costs or gives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VitalityParams {
    /// Signed change to the mover's own health. Positive values restore health; negative values are a move cost. The shared health authority also updates its damage meter.
    pub change: i32,
    /// Minimum health a negative cost may leave. The runtime never allows the cost to reduce the mover below 1.
    #[serde(default)]
    pub floor: i32,
    /// The effect drawn on the mover when the change lands.
    pub vfx: String,
    /// The cue played when the change lands.
    pub sfx: String,
}

/// Author a self-health change on a move timeline.
///
/// # Panics
///
/// Panics if `at_s` is after the move duration or `change` is zero.
pub fn author_vitality(mut spec: MoveSpec, at_s: f32, params: VitalityParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` changes health at {at_s}s but only lasts {}s, so the change \
         would never fire and the move would spend a recovery to do nothing",
        spec.id,
        spec.duration_s,
    );
    assert!(
        params.change != 0,
        "move `{}` authors a health change of zero, which is a move that costs \
         frames and means nothing",
        spec.id,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: VITALITY.to_string(),
            params: ParamValue::from_typed(&params).expect("vitality params serialize"),
        }),
    });
    spec
}
