//! Authoritative ECS movement-state components for the player entity.
//!
//! The 18 cluster types live in
//! [`crate::engine_core::player_clusters`] and `#[derive(Component)]`
//! directly (engine is Bevy-native per ADR 0002). The sandbox just
//! re-exports them under their original names — every consumer that
//! imports `crate::player::PlayerKinematics` etc. keeps working.
//!
//! Every engine `update_player_*` helper consumes these cluster refs
//! directly through [`crate::engine_core::PlayerClustersMut`]; no
//! `ae::Player` scratchpad or `engine_player_bridge` shim remains
//! (both deleted 2026-05-28).

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
