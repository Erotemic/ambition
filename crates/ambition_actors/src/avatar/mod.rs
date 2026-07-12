//! **The HOME AVATAR** — the body slot 0 owns and returns to, and the policy that
//! belongs to the local human rather than to any body.
//!
//! This module was `player/`. Its name was the last structural claim that the
//! player is a KIND OF THING rather than a brain and a slot. Everything in it that
//! was not actually the home avatar's has left (the S5/S6 fold,
//! `docs/planning/engine/refactor-chain.md` R6):
//!
//! | What left | Where it went | Why |
//! |---|---|---|
//! | `BodyAnimFacts`, `BodyMelee` | `crate::actor` (→ `ambition_characters`) | body vocabulary; every actor has it |
//! | `LocalPlayer`, `PlayerInputFrame`, the slot gestures, the two input bridges | [`crate::control`] | the device→slot→body seam is a wire, not a kind |
//! | the affordance table | [`crate::affordances`] | a BRIDGE: input × body × world → verb |
//! | `movement_fx`, `swim`, `ledge_grab` | [`crate::features`] | body mechanics; two of them named no `crate::` type at all |
//!
//! **What is left is named correctly.** The home avatar is a real concept — during
//! possession it is precisely the body that is NOT the controlled subject, so
//! nothing else can find it. Its identity bundle, its respawn safety and blink
//! camera, its starting character, its emitted trail, and the tick that integrates
//! it all belong to slot 0 by design, not by omission.
//!
//! **Nothing here is the ONLY path for anything.** A body's motion, melee, damage,
//! and abilities run the same seams every actor runs; the avatar differs in its
//! INPUT FRAME and its RESPAWN POLICY, and that is all.
//!
//! Submodules:
//! - [`components`] — the home avatar's policy state (respawn safety, blink camera).
//! - [`movement_components`] — re-exports of the 18 body cluster components from
//!   [`ambition_engine_core::body_clusters`].
//! - [`bundles`]    — [`PlayerSimulationBundle`] for spawning a sim-side avatar.
//! - [`events`]     — avatar-domain message types.
//! - [`systems`]    — frame systems that read or sync the avatar's components.
//! - [`body_integration`] — the home body's tick (the SAME body tick every actor
//!   runs; it differs only in input frame and respawn policy).
//! - [`trail`]      — the breadcrumb slot 0 chooses to emit.

pub mod body_integration;
pub mod bundles;
#[cfg(test)]
mod clone_probe_tests;
pub mod components;
pub mod events;
pub mod movement_components;
pub mod starting_character;
pub mod systems;
pub mod trail;

pub use body_integration::{
    advance_moving_platforms, integrate_home_body, ledge_platform_carry, LedgePlatformCarry,
    PlayerBodyFrameOutput,
};
pub use bundles::{PlayerIdentityBundle, PlayerSimulationBundle};
pub use starting_character::{
    apply_worn_character_gameplay, apply_worn_character_overlay, apply_worn_motion_model,
    gate_worn_player_control, motion_model_spec_for_character_id, StartingCharacter,
};
// NOTE: the body vocabulary — `PlayerEntity` / `PrimaryPlayer` (markers),
// `PrimaryPlayerOnly` (filter), `BodyKinematics` + the 18 movement clusters,
// `BodyWallet` (economy — players AND currency-dropping NPCs), `BodyAnimFacts`,
// `BodyMelee` — is NOT re-exported here. None of it is avatar-specific; its single
// home is `crate::actor`. Keeping it off this surface enforces the dependency
// direction (non-avatar code imports body state from `crate::actor`, never through
// here). That direction is what makes `crate::avatar`'s importer sink shrink
// instead of grow.
pub use components::{PlayerBlinkCameraState, PlayerSafetyState};
pub use events::PlayerHealRequested;
pub use systems::{
    apply_player_heal_requests, regen_player_mana, sync_player_actor_poses, tick_player_brains,
    write_player_ecs_components,
};

/// Build a `BodyClusterScratch` for the primary player at `spawn`
/// with the given `AbilitySet`. Single place that production code
/// uses; switching the underlying constructor (or deleting
/// `ae::Player`) only needs to touch this helper.
pub fn primary_player_scratch(
    spawn: ambition_engine_core::Vec2,
    abilities: ambition_engine_core::AbilitySet,
) -> ambition_engine_core::BodyClusterScratch {
    ambition_engine_core::BodyClusterScratch::new_with_abilities(spawn, abilities)
}
