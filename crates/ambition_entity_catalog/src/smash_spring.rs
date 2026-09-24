//! Authored payload for placing a temporary spring on the stage.
//! The spring is a ruleset-owned world actuator that can launch any body that reaches it.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique.
pub const PLACE_SPRING: &str = "smash.place_spring";

/// Authored parameters of one placed spring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceSpringParams {
    /// How hard it throws, in world px per second. Upward is negative `y`, and
    /// the launch is authored as a vector so a plate can be angled.
    pub launch: (f32, f32),
    /// The plate's size on the floor.
    pub half_extents: (f32, f32),
    /// Seconds before the spring is removed. The finite lifetime keeps this move-created actuator separate from stage geometry.
    pub lifetime_s: f32,
    /// Number of launches before the spring is spent. This limit is independent of its lifetime.
    pub uses: u8,
    /// Where it lands, body-local (`+x` toward facing, `+y` gravity-down).
    pub offset: (f32, f32),
    /// Effect row emitted when the spring is placed and when it fires. It is required because the spring has no persistent sprite of its own.
    pub vfx: String,
}

/// Author a spring placement on a move timeline.
///
/// # Panics
///
/// Panics if the event is outside the move, the spring is invisible, has no uses, or has zero launch velocity.
pub fn author_place_spring(mut spec: MoveSpec, at_s: f32, params: PlaceSpringParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` drops its plate at {at_s}s but only lasts {}s, so the plate \
         would never appear and the move would spend a recovery to do nothing",
        spec.id,
        spec.duration_s,
    );
    // `PlacedSpring` has no persistent sprite, so placement needs an authored cue.
    assert!(
        !params.vfx.trim().is_empty(),
        "move `{}` drops a plate that announces nothing — `PlacedSpring` draws no \
         sprite of its own, so a plate with no cue is an object the other player \
         is never told about",
        spec.id,
    );
    assert!(
        params.uses > 0,
        "move `{}` drops a plate with no uses left, which is an invisible object \
         that costs a recovery",
        spec.id,
    );
    let (lx, ly) = params.launch;
    assert!(
        lx.abs() + ly.abs() > 0.0,
        "move `{}` drops a plate that throws nobody anywhere",
        spec.id,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: PLACE_SPRING.to_string(),
            params: ParamValue::from_typed(&params).expect("place-spring params serialize"),
        }),
    });
    spec
}
