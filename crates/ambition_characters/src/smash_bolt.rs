//! A bolt the caster steers with the same stick they walk with.
//!
//! ⭐⭐ JON'S ASSIGNMENT, 2026-09-05: *"I want the author to have side-b be the
//! pk-thunder style 'mind' attack."*
//!
//! ⭐⭐ AND IT NEEDS NO INPUT LEASE, which is what the campaign plan expected to
//! build first. `ActorControlFrame::steer_axis()` already publishes *"what the
//! PLAYER is HOLDING, as opposed to what this body is ALLOWED to move by"* — it
//! exists because `update.rs` republishes the DAMPED frame after integration, so
//! a rooted move reads `locomotion` as zero for its whole duration. ⇒ The caster
//! keeps their own seat; a system reads their live stick and turns the bolt with
//! it, and `steer_axis()` is right whether the move is damping them or not — it
//! returns the value recorded BEFORE damping, or the live one when nothing
//! damped.
//!
//! ⛔⛔ AND THE CASTER IS NOT ROOTED FOR THE BOLT'S WHOLE LIFE — I wrote that it
//! was and it is not. `hitless_special` roots the MOVE, and the bolt is authored
//! to OUTLIVE it deliberately (a whiff must not pin the caster through their own
//! punish window). ⇒ So for most of the flight he is free, and **one stick does
//! both**: walking right also steers the bolt right.
//!
//! ⭐ THAT IS THE MOVE'S REAL COST, and it is better than the commitment I
//! thought I had authored. He is not helpless — he is DIVIDED. Repositioning
//! himself and aiming the thought are the same input, so every step he takes is
//! a turn he did not choose.
//!
//! ⛔ STEERING IS NOT POSSESSION, and only the first is wanted here. Possession
//! is "my input drives another body through ITS OWN action set while my avatar
//! goes inert" — that is what `TemporaryControl` and a per-seat driver are for,
//! and this move asks for none of it.
//!
//! The ruleset half — the object, its flight, and what it does to whoever it
//! reaches — is `ambition_demo_smash::bolt`.

use serde::{Deserialize, Serialize};

use ambition_entity_catalog::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique.
pub const STEERED_BOLT: &str = "smash.steered_bolt";

/// Authored parameters of one steered bolt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteeredBoltParams {
    /// How fast it travels, in world px per second. CONSTANT — the stick turns
    /// it and never speeds it up, which is what makes the move about aim rather
    /// than about mashing.
    pub speed: f32,
    /// How hard the stick can turn it, in degrees per second.
    ///
    /// ⭐ THIS NUMBER IS THE MOVE. Too low and it is a slow straight shot; too
    /// high and it is a homing missile the caster cannot miss with. It is the
    /// only thing separating a bolt you fly from a bolt you fire.
    pub turn_rate_deg: f32,
    /// Seconds before it fades on its own.
    pub lifetime_s: f32,
    /// Damage to whoever it reaches — the caster excepted, who gets
    /// [`Self::self_launch`] instead.
    pub damage: i32,
    /// Half-extent of the bolt's contact box.
    pub radius: f32,
    /// Launch speed applied to whoever it hits.
    pub knockback: f32,
    /// How hard the CASTER is thrown when the bolt comes back to them.
    ///
    /// ⭐⭐ THE WHOLE POINT OF THE MOVE, and the reason it is a recovery as well
    /// as an attack: you fly the bolt into your own back and it carries you.
    /// `0.0` is a bolt that simply fizzles on its owner, which is a different
    /// and much poorer move.
    pub self_launch: f32,
    /// Where it appears, body-local (`+x` toward facing, `+y` gravity-down).
    pub offset: (f32, f32),
}

/// Author a steered bolt onto a move's timeline.
///
/// # Panics
///
/// If `at_s` is past the move's own duration — the bolt would never appear and
/// the move would spend its recovery to do nothing. And if `turn_rate_deg` is
/// not positive, because a bolt nobody can turn is a slow projectile wearing a
/// steering move's startup.
pub fn author_steered_bolt(mut spec: MoveSpec, at_s: f32, params: SteeredBoltParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` fires its bolt at {at_s}s but only lasts {}s, so the bolt \
         would never appear and the move would spend a recovery to do nothing",
        spec.id,
        spec.duration_s,
    );
    assert!(
        params.turn_rate_deg > 0.0,
        "move `{}` authors a bolt that turns at {}°/s — a bolt nobody can steer \
         is a slow projectile wearing a steering move's startup",
        spec.id,
        params.turn_rate_deg,
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
