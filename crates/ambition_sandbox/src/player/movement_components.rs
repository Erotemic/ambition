//! Authoritative ECS movement-state components for the player entity.
//!
//! Phase 3a of the player-ecs-bandaid plan moved these cluster types
//! into the engine ([`crate::engine_core::player_clusters`]). The engine
//! types `#[derive(Component)]` directly (engine is Bevy-native per
//! ADR 0002), so the sandbox now just re-exports them under their
//! original names — every consumer that imports
//! `crate::player::PlayerKinematics` etc. keeps working.
//!
//! Phase 3b refactors engine `update_player_*` helpers to consume
//! these cluster refs directly; Phase 3c deletes the
//! `engine_player_bridge` shim once the helpers no longer need a
//! tick-local `ae::Player` scratchpad.

pub use crate::engine_core::{
    EnginePlayerAbilities as PlayerAbilities, EnginePlayerActionBuffer as PlayerActionBuffer,
    EnginePlayerBlinkState as PlayerBlinkState, EnginePlayerBodyModeState as PlayerBodyModeState,
    EnginePlayerComboTrace as PlayerComboTrace, EnginePlayerDashState as PlayerDashState,
    EnginePlayerDodgeState as PlayerDodgeState,
    EnginePlayerEnvironmentContact as PlayerEnvironmentContact,
    EnginePlayerFlightState as PlayerFlightState, EnginePlayerGroundState as PlayerGroundState,
    EnginePlayerJumpState as PlayerJumpState, EnginePlayerKinematics as PlayerKinematics,
    EnginePlayerLedgeState as PlayerLedgeState, EnginePlayerLifetime as PlayerLifetime,
    EnginePlayerMana as PlayerMana, EnginePlayerOffense as PlayerOffense,
    EnginePlayerShieldState as PlayerShieldState, EnginePlayerWallState as PlayerWallState,
};
