//! Player ECS state.
//!
//! The ECS player entity is the frame-to-frame authority for player movement,
//! health, combat timers, and interaction buffering. All player state lives on
//! ECS components; do not reintroduce a god-object runtime resource.
//!
//! Submodules:
//! - [`components`] — the per-frame ECS components on the player entity.
//! - [`bundles`]    — [`PlayerSimulationBundle`] for spawning a sim-side player.
//! - [`events`]     — player-domain message types.
//! - [`systems`]    — frame systems that read or sync the player components.

pub mod bubble_shield;
pub mod bundles;
pub mod components;
pub mod events;
pub mod ledge_grab;
pub mod queries;
pub mod swim;
pub mod systems;

pub use bundles::{PlayerIdentityBundle, PlayerSimulationBundle};
pub use components::{
    LocalPlayer, PlayerAnimState, PlayerBlinkCameraState, PlayerBody, PlayerCombatState,
    PlayerEntity, PlayerHealth, PlayerInteractionState, PlayerMovementAuthority,
    PlayerPlatformRideState, PlayerSlot, PrimaryPlayer,
};
pub use events::{PlayerDamageRequested, PlayerHealRequested};
pub use queries::{primary_player_entity, sort_players_by_slot, PrimaryPlayerOnly};
pub use systems::{apply_player_heal_requests, write_player_ecs_components};
