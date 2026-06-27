//! Player ECS state.
//!
//! The ECS player entity is the frame-to-frame authority for player movement,
//! health, combat timers, and interaction buffering. All player state lives on
//! ECS components; do not reintroduce a god-object runtime resource.
//!
//! Submodules:
//! - [`components`] — the per-frame ECS components on the player entity.
//! - [`movement_components`] — re-exports of the 18 player cluster
//!   components from [`ambition_engine_core::player_clusters`]. These
//!   replaced the (now-deleted) `PlayerMovementAuthority` /
//!   `PlayerBody` aggregates in the cluster-native migration
//!   (finalized 2026-05-28).
//! - [`bundles`]    — [`PlayerSimulationBundle`] for spawning a sim-side player.
//! - [`events`]     — player-domain message types.
//! - [`systems`]    — frame systems that read or sync the player components.
//! - [`affordances`] — "what would each button do right now?" table +
//!   HUD/input-glyph support.
//! - [`queries`]    — singleton-vs-multiplayer-explicit player query helpers.
//! - [`ledge_grab`] / [`swim`] — thin sandbox shims over engine-owned
//!   ledge-grab and water mechanics (constants + end-to-end test fixtures).

pub mod affordances;
pub mod bundles;
#[cfg(test)]
mod clone_probe_tests;
pub mod components;
pub mod events;
pub mod ledge_grab;
pub mod movement_components;
pub mod movement_fx;
pub mod queries;
pub mod swim;
pub mod systems;
pub mod trail;

pub use bundles::{PlayerIdentityBundle, PlayerSimulationBundle};
pub use components::{
    ActivePlayerAttack, LocalPlayer, PlayerAnimState, PlayerBlinkCameraState,
    PlayerEntity, PlayerInputFrame, PlayerInteractionState, PlayerSafetyState,
    PlayerSlot, PlayerWallet, PrimaryPlayer,
};
pub use events::PlayerHealRequested;
pub use movement_fx::handle_player_events;
pub use movement_components::{
    BodyKinematics, BodyAbilities, BodyActionBuffer, BodyBaseSize, BodyBlinkState,
    BodyModeState, BodyComboTrace, BodyDashState, BodyDodgeState,
    BodyEnvironmentContact, BodyFlightState, BodyGroundState, BodyJumpState,
    BodyLedgeState, BodyLifetime, BodyMana, BodyOffense, BodyShieldState,
    BodyWallState,
};
pub use queries::{primary_player_entity, sort_players_by_slot, PrimaryPlayerOnly};
pub use systems::{
    apply_player_heal_requests, sync_local_player_input_frame, sync_player_actor_poses,
    tick_player_brains, write_player_ecs_components,
};

/// Build a `PlayerClusterScratch` for the primary player at `spawn`
/// with the given `AbilitySet`. Single place that production code
/// uses; switching the underlying constructor (or deleting
/// `ae::Player`) only needs to touch this helper.
pub fn primary_player_scratch(
    spawn: ambition_engine_core::Vec2,
    abilities: ambition_engine_core::AbilitySet,
) -> ambition_engine_core::PlayerClusterScratch {
    ambition_engine_core::PlayerClusterScratch::new_with_abilities(spawn, abilities)
}
