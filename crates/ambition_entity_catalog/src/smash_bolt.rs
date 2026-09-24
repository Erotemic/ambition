//! Authored payload for the steerable bolt technique.
//! The ruleset owns projectile simulation. Steering reads the caster's control frame; this module only defines the move payload and validation.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique.
pub const STEERED_BOLT: &str = "smash.steered_bolt";

/// Authored parameters of one steered bolt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteeredBoltParams {
    /// How fast it travels, in world px per second. Constant: the stick turns
    /// it and never speeds it up, which is what makes the move about aim rather
    /// than about mashing.
    pub speed: f32,
    /// Maximum turn rate, in degrees per second.
    pub turn_rate_deg: f32,
    /// Seconds before it fades on its own.
    pub lifetime_s: f32,
    /// Effect row used to show the bolt path. Empty values are invalid because the steerable projectile must remain visible.
    pub trail_vfx: String,
    /// Seconds between trail marks. Must be positive.
    pub trail_every_s: f32,
    /// Damage to whoever it reaches — the caster excepted, who gets
    /// [`Self::self_launch`] instead.
    pub damage: i32,
    /// Half-extent of the bolt's contact box.
    pub radius: f32,
    /// Hitbox feel multiplier, not launch speed. Values above 8 are rejected as implausible.
    pub knockback: f32,
    /// Feel multiplier applied to the caster when the bolt returns to them. `0.0` disables self-launch.
    pub self_launch: f32,
    /// Where it appears, body-local (`+x` toward facing, `+y` gravity-down).
    pub offset: (f32, f32),
}

impl SteeredBoltParams {
    /// Return all semantic validation errors for this payload.
    /// The Rust authoring helper and runtime declaration both use this validator. Timeline checks such as `at_s` stay in the helper because they need the move.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.trail_vfx.trim().is_empty() {
            out.push(
                "`trail_vfx` is empty, so the bolt draws nothing — the whole move \
                 is flying it, and the caster cannot fly what they cannot see"
                    .to_string(),
            );
        }
        if !(self.trail_every_s > 0.0) {
            out.push(format!(
                "`trail_every_s` is {}, so the trail is redrawn never",
                self.trail_every_s
            ));
        }
        if !(self.turn_rate_deg > 0.0) {
            out.push(format!(
                "`turn_rate_deg` is {}, and a bolt nobody can steer is a slow \
                 projectile wearing a steering move's startup",
                self.turn_rate_deg
            ));
        }
        out
    }
}

/// Hydrate and semantically validate a steered-bolt payload.
pub fn check_steered_bolt_params(
    params: &crate::ParamValue,
) -> Result<(), String> {
    let typed: SteeredBoltParams = params.hydrate().map_err(|error| error.to_string())?;
    let problems = typed.problems();
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// Author a steered bolt on a move timeline.
///
/// # Panics
///
/// Panics if the payload is invalid or `at_s` is after the move duration.
pub fn author_steered_bolt(mut spec: MoveSpec, at_s: f32, params: SteeredBoltParams) -> MoveSpec {
    // Use the same semantic validator as runtime admission.
    let problems = params.problems();
    assert!(
        problems.is_empty(),
        "move `{}` authors an invalid bolt: {}",
        spec.id,
        problems.join("; "),
    );
    assert!(
        at_s <= spec.duration_s,
        "move `{}` fires its bolt at {at_s}s but only lasts {}s, so the bolt \
         would never appear and the move would spend a recovery to do nothing",
        spec.id,
        spec.duration_s,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: STEERED_BOLT.to_string(),
            params: ParamValue::from_typed(&params).expect("steered-bolt params serialize"),
        }),
    });
    spec
}
