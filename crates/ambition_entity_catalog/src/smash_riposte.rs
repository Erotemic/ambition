//! Authored payload for a body-anchored follow-up strike.
//! The strike uses the normal combat hitbox authority. Counter responses are one consumer, but any move may author this technique.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key.
pub const RIPOSTE_STRIKE: &str = "smash.riposte_strike";

/// Authored parameters of one answering cut.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiposteStrikeParams {
    /// Percent dealt by the cut.
    pub damage: u32,
    /// Hitbox feel multiplier, not launch speed. The validator rejects non-positive values and values above 8.
    pub knockback: f32,
    /// Distance in front of the fighter where the cut is centered, in world pixels.
    pub reach: f32,
    /// Half-extents of the cut, in world px.
    pub half_extents: (f32, f32),
    /// Lifetime of the stationary cut hitbox, in seconds.
    pub lifetime_s: f32,
    /// Optional sound id for a successful cut. `None` leaves the victim's normal hurt sound.
    pub hit_sfx: Option<String>,
}

/// Hydrate and semantically validate a riposte-strike payload.
pub fn check_riposte_strike_params(
    params: &crate::ParamValue,
) -> Result<(), String> {
    let typed: RiposteStrikeParams = params.hydrate().map_err(|error| error.to_string())?;
    let problems = typed.problems();
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

impl RiposteStrikeParams {
    /// Return all semantic validation errors for this strike payload.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.damage == 0 {
            problems.push(
                "deals 0 damage, so the parry is answered by an invisible box \
                 that costs the attacker nothing"
                    .to_string(),
            );
        }
        if !(self.knockback > 0.0) {
            problems.push(format!(
                "authors {} knockback: the field is a FEEL MULTIPLIER (1.1–1.6), \
                 and zero or less is not a weaker hit but an absent one",
                self.knockback,
            ));
        }
        if self.knockback > 8.0 {
            problems.push(format!(
                "authors {} knockback, above `MAX_PLAUSIBLE_FEEL_SCALE` (8.0) — \
                 this reads like a launch SPEED copied off a `Strike`, which is \
                 a different unit and the mistake three shipped moves made",
                self.knockback,
            ));
        }
        if self.reach <= 0.0 {
            problems.push(format!(
                "cuts {}px in front of itself, so the answer lands inside the \
                 fighter who threw it",
                self.reach,
            ));
        }
        if self.half_extents.0 <= 0.0 || self.half_extents.1 <= 0.0 {
            problems.push(format!(
                "authors a {:?} cut, which has no area and can never overlap \
                 anybody",
                self.half_extents,
            ));
        }
        if self.lifetime_s <= 0.0 {
            problems.push(format!(
                "lives {}s, so it is spawned and gone before a frame resolves",
                self.lifetime_s,
            ));
        }
        problems
    }
}

/// Author a body-anchored follow-up strike on a move timeline.
///
/// # Panics
///
/// Panics if `at_s` is after the move duration or the strike payload is invalid.
pub fn author_cut(mut spec: MoveSpec, at_s: f32, params: RiposteStrikeParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` cuts at {at_s}s but only lasts {}s",
        spec.id,
        spec.duration_s,
    );
    let problems = params.problems();
    assert!(
        problems.is_empty(),
        "move `{}` authors an unusable cut: {}",
        spec.id,
        problems.join("; "),
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: RIPOSTE_STRIKE.to_string(),
            params: ParamValue::from_typed(&params).expect("riposte-strike params serialize"),
        }),
    });
    spec
}
