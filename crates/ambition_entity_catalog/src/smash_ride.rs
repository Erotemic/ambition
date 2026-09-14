//! Authored payload for summoning and riding a mount.
//! The move names the mount and the recovery route. Mount compatibility and live riding behavior remain owned by the mount system.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const SUMMON_RIDE: &str = "smash.summon_ride";

/// The mount class the pirate is licensed for. ADR 0020's compatibility check
/// reads this off the summoned body's `Mountable`.
pub const SHARK_CLASS: &str = "shark";

/// Authored parameters for one summon-and-ride technique. The mount character id remains content data rather than a hardcoded mechanic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummonRideParams {
    /// Which character to summon as the mount.
    pub character_id: String,
    /// The mount body's half-extents.
    pub half_extents: (f32, f32),
    /// How long the rider may stay aboard, in seconds of sim time.
    pub seconds: f32,
    /// Planner-facing recovery reach in world pixels. Runtime steering does not read this value.
    pub reach: f32,
}

/// Return the character id referenced by a summon-and-ride payload.
/// Malformed params return no references because parameter hydration is validated separately.
pub fn summon_ride_character_refs(effect: &crate::EffectRef) -> Vec<String> {
    effect
        .params
        .hydrate::<SummonRideParams>()
        .map(|params| vec![params.character_id])
        .unwrap_or_default()
}

/// Author a summon-and-ride event. The move cannot begin while the rider is already held, and its recovery route is sustained movement authority.
///
/// # Panics
///
/// Panics if `at_s` is after the move duration.
pub fn author_summon_ride(mut spec: MoveSpec, at_s: f32, params: SummonRideParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` summons its mount at {at_s}s but only lasts {}s, so the \
         summon would never fire and the move would spend a recovery to do nothing",
        spec.id,
        spec.duration_s,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: SUMMON_RIDE.to_string(),
            params: ParamValue::from_typed(&params).expect("summon-ride params serialize"),
        }),
    });
    // A rider cannot summon another mount while already held.
    spec.gates.forbidden_while_held = true;
    // The planner models this as sustained movement authority, not as a fabricated launch impulse.
    spec.gates.recovery_route = Some(
        crate::AuthoredRecoveryRoute::SustainedAuthority {
            seconds: params.seconds,
            reach: params.reach,
        },
    );
    spec
}
