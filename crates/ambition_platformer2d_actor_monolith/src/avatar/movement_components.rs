//! Compatibility re-exports for body movement-state components.
//!
//! TODO(compat-remove): migrate player-internal callers to `crate::actor`, then delete this
//! path-preservation module.
pub use ambition_platformer2d_core::{
    BodyAbilities, BodyActionBuffer, BodyBaseSize, BodyBlinkState, BodyComboTrace, BodyDashState,
    BodyDodgeState, BodyEnvironmentContact, BodyFlightState, BodyGroundState, BodyJumpState,
    BodyKinematics, BodyLedgeState, BodyLifetime, BodyMana, BodyModeState, BodyOffense,
    BodyShieldState, BodyWallState,
};
