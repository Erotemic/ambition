//! Authored payload for teleport movement.
//! The ruleset resolves the destination and collision response. This module also declares the recovery route and optional move intangibility.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const TELEPORT: &str = "smash.teleport";

/// Authored parameters for one teleport. The same technique supports an aimed recovery and a nearest-foe ambush by changing destination selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeleportParams {
    /// If false, use the latched aim from the move startup and default to straight up. If true, target the far side of the nearest foe within `distance`.
    /// A boolean is used because `ParamValue` cannot round-trip enum variants reliably.
    #[serde(default)]
    pub behind_nearest_foe: bool,
    /// Gap between the target edge and arrival edge for a nearest-foe teleport. Ignored for aimed teleports.
    #[serde(default)]
    pub behind_gap: f32,
    /// Travel distance for aimed teleports and maximum acquisition range for nearest-foe teleports.
    pub distance: f32,
    /// Maximum upward snap distance to a supported ledge near the resolved destination. `0.0` disables ledge assist.
    pub ledge_assist: f32,
    /// Move-clock seconds of intangibility starting at transit. `0.0` disables it. The helper clamps the window to the move duration.
    #[serde(default)]
    pub intangible_s: f32,
    /// The effect drawn where the fighter left.
    pub depart_vfx: String,
    /// The effect drawn where the fighter arrived.
    pub arrive_vfx: String,
}

/// Author a teleport event, optional intangibility, and a teleport recovery route.
///
/// # Panics
///
/// Panics if `at_s` is after the move duration.
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
    // Teleporting is a recovery route even though it has no launch impulse. The optional invulnerability window uses the shared move-defense helper.
    let mut spec = spec;
    if params.intangible_s > 0.0 {
        // Do not extend the defense window past the move timeline.
        let ends = (at_s + params.intangible_s).min(spec.duration_s);
        spec = crate::authoring::invuln(spec, at_s, ends);
    }
    spec.gates.recovery_route = Some(crate::AuthoredRecoveryRoute::Teleport {
        distance: params.distance,
    });
    spec
}
