//! Re-export facade for the player's authoritative ECS movement-state
//! components, under their original (un-prefixed) names.
//!
//! The 18 cluster types live in
//! [`ambition_engine_core::player_clusters`] and `#[derive(Component)]`
//! directly (engine is Bevy-native per ADR 0002). The sandbox just
//! re-exports them under their original names — every consumer that
//! imports `crate::actor::BodyKinematics` etc. keeps working.
//!
//! NOTE: [`BodyKinematics`] is re-exported here for player convenience
//! but is the UNIVERSAL actor body (pos/size/vel/facing) — enemies,
//! NPCs, bosses, and projectiles all use the same type. It is NOT
//! player-specific; only the other 17 `Player*` clusters here are.
//!
//! Every engine `update_player_*` helper consumes these cluster refs
//! directly through [`ambition_engine_core::PlayerClustersMut`]; no
//! `ae::Player` scratchpad or `engine_player_bridge` shim remains
//! (both deleted 2026-05-28).

// The shared body movement-cluster components now live on the neutral actor
// vocabulary (`crate::actor`) under the `Body*` convention — they are not
// player-specific. This facade re-exports them for player-internal convenience;
// non-player code imports them straight from `crate::actor`.
pub use crate::actor::{
    BodyAbilities, BodyActionBuffer, BodyBaseSize, BodyBlinkState, BodyComboTrace, BodyDashState,
    BodyDodgeState, BodyEnvironmentContact, BodyFlightState, BodyGroundState, BodyJumpState,
    BodyKinematics, BodyLedgeState, BodyLifetime, BodyMana, BodyModeState, BodyOffense,
    BodyShieldState, BodyWallState,
};
