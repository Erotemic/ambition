//! Player ECS state.
//!
//! The ECS player entity is the frame-to-frame authority for player movement,
//! health, combat timers, and interaction buffering. All player state lives on
//! ECS components; do not reintroduce a god-object runtime resource.
//!
//! Submodules:
//! - [`components`] — the per-frame ECS components on the player entity.
//! - [`movement_components`] — Phase 1 cluster components that will replace
//!   the live `PlayerMovementAuthority` aggregate (see
//!   `docs/planning/player-ecs-bandaid-phase0.md`).
//! - [`bundles`]    — [`PlayerSimulationBundle`] for spawning a sim-side player.
//! - [`events`]     — player-domain message types.
//! - [`systems`]    — frame systems that read or sync the player components.

pub mod affordances;
pub mod bubble_shield;
pub mod bundles;
pub mod components;
pub mod engine_player_bridge;
pub mod events;
pub mod ledge_grab;
pub mod movement_components;
pub mod queries;
pub mod swim;
pub mod systems;

pub use bundles::{PlayerIdentityBundle, PlayerSimulationBundle};
pub use components::{
    ActivePlayerAttack, LocalPlayer, PlayerAnimState, PlayerBlinkCameraState, PlayerCombatState,
    PlayerEntity, PlayerHealth, PlayerInputFrame, PlayerInteractionState,
    PlayerPlatformRideState, PlayerSafetyState, PlayerSlot, PrimaryPlayer,
};
pub use movement_components::{
    PlayerAbilities, PlayerActionBuffer, PlayerBlinkState, PlayerBodyModeState, PlayerComboTrace,
    PlayerDashState, PlayerDodgeState, PlayerEnvironmentContact, PlayerFlightState,
    PlayerGroundState, PlayerJumpState, PlayerKinematics, PlayerLedgeState, PlayerLifetime,
    PlayerMana, PlayerOffense, PlayerShieldState, PlayerWallState,
};
pub use events::{PlayerDamageRequested, PlayerHealRequested};
pub use queries::{primary_player_entity, sort_players_by_slot, PrimaryPlayerOnly};
pub use systems::{
    apply_player_heal_requests, sync_local_player_input_frame, tick_player_brains,
    write_player_ecs_components,
};
