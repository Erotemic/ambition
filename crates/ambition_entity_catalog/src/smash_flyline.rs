//! Authored payload for a flyline recovery.
//! The movement kernel owns wire motion and release velocity. This module defines when the wire starts and the rope parameters.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const FLYLINE: &str = "smash.flyline";

/// One catch of a flyline: the rope, the lift, and what the swing is allowed to
/// buy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlylineParams {
    /// Initial rope length in world pixels. It sets the swing radius and must remain large enough for the authored rise.
    pub rope_length: f32,
    /// Vertical distance lifted during the beat, in world pixels.
    pub rise: f32,
    /// Lift duration in seconds. The move timeline must outlast this interval.
    pub lift_s: f32,
    /// Maximum swing angle from vertical, in degrees. The movement kernel converts it to radians.
    pub max_swing_deg: f32,
    /// What a held stick contributes, in radians per second squared.
    pub swing_accel: f32,
    /// Upward release speed in world pixels per second. The movement kernel is the sole writer of the exit velocity.
    pub release_rise: f32,
    /// Optional one-shot effect at the catch point. The persistent wire is rendered from the wire state, not from this effect.
    #[serde(default)]
    pub vfx: Option<String>,
    /// Sound cue played when the wire catches.
    pub sfx: String,
}

/// Author a flyline catch on a move timeline.
///
/// # Panics
///
/// Panics if `at_s` is after the move duration.
pub fn author_flyline(mut spec: MoveSpec, at_s: f32, params: FlylineParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` catches a flyline at {at_s}s but only lasts {}s, so the beat \
         would never fire",
        spec.id,
        spec.duration_s,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: FLYLINE.to_string(),
            params: ParamValue::from_typed(&params).expect("flyline params serialize"),
        }),
    });
    spec
}
