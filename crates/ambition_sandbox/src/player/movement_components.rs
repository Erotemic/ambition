//! Re-export facade for the player's authoritative ECS movement-state
//! components, under their original (un-prefixed) names.
//!
//! The 18 cluster types live in
//! [`crate::engine_core::player_clusters`] and `#[derive(Component)]`
//! directly (engine is Bevy-native per ADR 0002). The sandbox just
//! re-exports them under their original names — every consumer that
//! imports `crate::player::BodyKinematics` etc. keeps working.
//!
//! NOTE: [`BodyKinematics`] is re-exported here for player convenience
//! but is the UNIVERSAL actor body (pos/size/vel/facing) — enemies,
//! NPCs, bosses, and projectiles all use the same type. It is NOT
//! player-specific; only the other 17 `Player*` clusters here are.
//!
//! Every engine `update_player_*` helper consumes these cluster refs
//! directly through [`crate::engine_core::PlayerClustersMut`]; no
//! `ae::Player` scratchpad or `engine_player_bridge` shim remains
//! (both deleted 2026-05-28).

pub use crate::engine_core::{
    BodyKinematics, EnginePlayerAbilities as PlayerAbilities,
    EnginePlayerActionBuffer as PlayerActionBuffer, EnginePlayerBaseSize as PlayerBaseSize,
    EnginePlayerBlinkState as PlayerBlinkState, EnginePlayerBodyModeState as PlayerBodyModeState,
    EnginePlayerComboTrace as PlayerComboTrace, EnginePlayerDashState as PlayerDashState,
    EnginePlayerDodgeState as PlayerDodgeState,
    EnginePlayerEnvironmentContact as PlayerEnvironmentContact,
    EnginePlayerFlightState as PlayerFlightState, EnginePlayerGroundState as PlayerGroundState,
    EnginePlayerJumpState as PlayerJumpState, EnginePlayerLedgeState as PlayerLedgeState,
    EnginePlayerLifetime as PlayerLifetime, EnginePlayerMana as PlayerMana,
    EnginePlayerOffense as PlayerOffense, EnginePlayerShieldState as PlayerShieldState,
    EnginePlayerWallState as PlayerWallState,
};
