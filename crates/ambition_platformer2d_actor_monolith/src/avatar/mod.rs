//! Historical slot-0/home-body and protagonist integration.
//!
//! This module is not a permanent actor-domain boundary. `ControlledSubject`
//! answers which body currently has control authority; `PrimaryPlayer` answers a
//! different, narrower question about slot 0's own body. The engine may eventually
//! retain little or no privileged home-body policy at all.
//!
//! During the active actor-monolith decomposition, move each responsibility to
//! its real owner instead of extracting an `avatar` crate: body mechanics and
//! intrinsic capabilities go to actor/body capability owners; starting-character
//! identity and preparation go to provider/session owners; retained slot/home-body
//! lifecycle policy goes to session/control; camera reactions and trails go to
//! presentation.
//!
//! A body's motion, melee, damage, inventory, and abilities follow body-owned
//! simulation seams regardless of which participant currently controls it.
//!
//! Submodules:
//! - [`components`] — the home avatar's policy state (respawn safety, blink camera).
//! - [`movement_components`] — re-exports of the 18 body cluster components from
//!   [`ambition_platformer2d_core::body_clusters`].
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
    advance_moving_platforms, integrate_home_body, surface_skidding, BodyReset,
    PlayerBodyFrameOutput,
};
pub use bundles::{PlayerIdentityBundle, PlayerSimulationBundle};
pub use starting_character::{
    apply_worn_character_gameplay, apply_worn_character_overlay, apply_worn_motion_model,
    gate_worn_player_control, motion_model_spec_for_character, motion_model_spec_for_character_id,
    movement_tuning_for_character, sustain_bubble_shield, sync_charge_projectile_capability,
    InitialBodyPolicy, PersonaBaseline, StartingCharacter, WornControlGateSet,
};
// NOTE: the body vocabulary — `PlayerEntity` / `PrimaryPlayer` (markers),
// `PrimaryPlayerOnly` (filter), `BodyKinematics` + the 18 movement clusters,
// `BodyWallet` (economy — players AND currency-dropping NPCs), `BodyAnimFacts`,
// `BodyMelee` — is NOT re-exported here. None of it is avatar-specific; its single
// home is `crate::actor`. Keeping it off this surface enforces the dependency
// direction (non-avatar code imports body state from `crate::actor`, never through
// here). That direction is what makes `crate::avatar`'s importer sink shrink
// instead of grow.
pub use components::PlayerSafetyState;
pub use events::PlayerHealRequested;
pub use systems::{
    apply_player_heal_requests, blank_scripted_control_frames, regen_player_mana,
    sync_player_actor_poses, tick_controlled_brains, ControlledBrainTick,
};

/// Build a `BodyClusterScratch` for the primary player at `spawn`
/// with the given `AbilitySet`. Single place that production code
/// uses; switching the underlying constructor (or deleting
/// `ae::Player`) only needs to touch this helper.
pub fn primary_player_scratch(
    spawn: ambition_platformer2d_core::Vec2,
    abilities: ambition_platformer2d_core::AbilitySet,
) -> ambition_platformer2d_core::BodyClusterScratch {
    ambition_platformer2d_core::BodyClusterScratch::new_with_abilities(spawn, abilities)
}
