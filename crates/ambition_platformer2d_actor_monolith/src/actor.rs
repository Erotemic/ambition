//! THE ONE THING THIS MODULE ACTUALLY OWNS.
//!
//! ⛔⛔ IT WAS A FACADE: six `pub use` lines re-exporting `_core`'s body
//! clusters, `shared_tangle`'s player markers, `ambition_characters`'
//! `BodyAnimFacts` and `ambition_combat`'s `BodyMelee` under the monolith's
//! address, plus one real bundle. About 230 sites inside this crate named those
//! types through here, and every coupling census that counted them read the
//! monolith as their owner. Deleted 2026-08-27 (D33); callers name the crate
//! that owns the thing.
//!
//! ⭐ `platformer_runtime::body` went with it — its own doc carried a
//! `TODO(compat-remove)` saying exactly this, and nobody had taken it.

use ambition_platformer2d_core::{
    BodyAbilities, BodyActionBuffer, BodyBaseSize, BodyBlinkState, BodyComboTrace, BodyDashState,
    BodyDodgeState, BodyEnvironmentContact, BodyFlightState, BodyGroundState, BodyJumpState,
    BodyLedgeState, BodyLifetime, BodyMana, BodyModeState, BodyOffense, BodyShieldState,
    BodyWallState,
};

// Both surface here, on the neutral actor vocabulary, in the S5/S6 fold (R6).

/// Shared movement-cluster components that, with [`BodyKinematics`], form the
/// authoritative movement aggregate consumed by the common movement kernel.

/// Ancillary movement components spawned on every body. [`BodyKinematics`] is
/// separate so rendering, gravity, and targeting can read kinematics without
/// borrowing the full movement aggregate. Player and non-player construction use
/// this same bundle and the same `BodyClusterQueryData` path.
#[derive(bevy::prelude::Bundle)]
pub struct AncillaryMovementBundle {
    pub abilities: BodyAbilities,
    /// The body's intrinsic capability set, captured at spawn from the same
    /// `AbilitySet` the effective [`BodyAbilities`] starts at. Held constant so a
    /// session mask (the F3 dev editable) can gate verbs off `abilities` without
    /// erasing what the character was authored to do. Carried by EVERY body
    /// (player + actors) so it is an inseparable companion of `BodyAbilities`,
    /// never a component a spawn path can forget.
    pub ability_base: ambition_platformer2d_core::AbilityBase,
    /// §3.1 motion record — spawned default (zero-length at origin) and
    /// overwritten by the body's first simulation step.
    pub sweep: ambition_platformer2d_core::SweepSample,
    pub base_size: BodyBaseSize,
    pub ground: BodyGroundState,
    pub wall: BodyWallState,
    pub jump: BodyJumpState,
    pub dash: BodyDashState,
    pub flight: BodyFlightState,
    pub blink: BodyBlinkState,
    pub ledge: BodyLedgeState,
    pub dodge: BodyDodgeState,
    pub shield: BodyShieldState,
    pub body_mode: BodyModeState,
    pub env_contact: BodyEnvironmentContact,
    pub mana: BodyMana,
    pub offense: BodyOffense,
    pub action_buffer: BodyActionBuffer,
    pub lifetime: BodyLifetime,
    pub combo_trace: BodyComboTrace,
    /// The per-tick environment-resolved frame artifact (ADR 0024): spawned at
    /// its default and published by the frame resolution phase each sim tick.
    pub frame: ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
    /// The published semantic movement facts (ADR 0024): rewritten from the
    /// body's policy after every movement step; THE read surface for
    /// animation/combat/affordances/HUD instead of policy internals.
    pub motion_facts: ambition_platformer2d_core::BodyMotionFacts,
}

impl AncillaryMovementBundle {
    /// Split the ancillary clusters out of a [`BodyClusterScratch`],
    /// dropping its vestigial `kinematics` field (the body's authoritative
    /// [`BodyKinematics`] is spawned separately).
    pub fn from_scratch(scratch: ambition_platformer2d_core::BodyClusterScratch) -> Self {
        let ambition_platformer2d_core::BodyClusterScratch {
            abilities,
            kinematics: _,
            // The movement policy is its own component in ECS; callers spawn it
            // separately (ADR 0024).
            model: _,
            base_size,
            mut ground,
            wall,
            jump,
            dash,
            flight,
            blink,
            ledge,
            dodge,
            shield,
            body_mode,
            env_contact,
            mana,
            offense,
            action_buffer,
            lifetime,
            combo_trace,
        } = scratch;
        // A scratch body is an explicit state fixture, but an ECS spawn has no
        // prior world-contact sample. The movement kernel will establish the
        // gravity-relative baseline at the authored pose before its first
        // control/integration step.
        ground.invalidate();
        Self {
            ability_base: ambition_platformer2d_core::AbilityBase::new(abilities.abilities),
            abilities,
            sweep: Default::default(),
            base_size,
            ground,
            wall,
            jump,
            dash,
            flight,
            blink,
            ledge,
            dodge,
            shield,
            body_mode,
            env_contact,
            mana,
            offense,
            action_buffer,
            lifetime,
            combo_trace,
            frame: Default::default(),
            motion_facts: Default::default(),
        }
    }
}
