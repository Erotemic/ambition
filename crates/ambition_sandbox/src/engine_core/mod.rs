//! Reusable Bevy-native simulation primitives.
//!
//! Formerly the `ambition_engine` crate; collapsed into the sandbox
//! 2026-05-28. This module owns the testable gameplay rules a future
//! sibling crate could reuse: kinematic player movement, AABB
//! collision semantics, ability gates, ledge-grab probes, world
//! collision/water/climbable region data, and the cluster components
//! (`PlayerKinematics`, `PlayerGroundState`, …, `PlayerComboTrace`)
//! that make up the player ECS entity.
//!
//! Sandbox-side concerns (LDtk entities, per-room authored Vecs,
//! sandbox dispatch tables, presentation) stay outside this module.
//!
//! As of 2026-05-28 there are zero `to_player`/`write_from_player`
//! round-trips in production simulation code: every cluster-ref entry
//! point (`update_player_{control,simulation}_with_clusters`) and
//! every inner helper (`tick_active_ledge_grab_clusters`,
//! `try_start_ledge_grab_clusters`, `integrate_velocity_clusters`,
//! the cluster-native sweep helpers in `movement/collision`) operates
//! on cluster refs natively. `ae::Player` survives only as a read-only
//! snapshot for `to_player`-shaped callers (debug overlay, trace
//! recorder, headless reporting); the eventual deletion is documented
//! in `dev/journals/player-cluster-native-push-2026-05-28.md`.

pub mod abilities;
pub mod player_clusters;
pub mod geometry;
pub mod ledge_grab;
pub mod movement;
pub mod player_state;
pub mod world;

// Re-export the public surface so story/sandbox crates can treat the engine as
// the main mechanics API while the internals stay organized by concern.
pub use abilities::AbilitySet;
pub use bevy_math::Vec2;
pub use geometry::{aabb_from_min_size, Aabb, AabbExt};
pub use ledge_grab::{
    probe_ledge_grab, LedgeContact, LedgeGetupKind, LedgeGrabState, LEDGE_CLIMB_TIME,
    LEDGE_GRAB_INVULN_TIME, LEDGE_MIN_CLIMB_DELAY, LEDGE_ROLL_OVERSHOOT, LEDGE_ROLL_TIME,
    LEDGE_TOWARD_CLIMB_DELAY,
};
pub use movement::{
    blink_destination, blink_destination_clusters, blink_destination_to_point,
    blink_destination_to_point_clusters, default_player_body_size, update_player,
    update_player_control_with_clusters, update_player_simulation_with_clusters,
    update_player_control, update_player_control_with_tuning, update_player_simulation,
    update_player_simulation_with_tuning, update_player_with_tuning, BlinkEvent, ComboMark,
    FrameEvents, InputState, LedgeMomentumTuning, MovementOp, MovementTuning, Player, AIR_ACCEL,
    AIR_FRICTION, AIR_JUMPS, BLINK_COOLDOWN, BLINK_DISTANCE, BLINK_HOLD_THRESHOLD, COYOTE_TIME,
    DASH_BUFFER, DASH_COOLDOWN, DASH_SPEED, DASH_TIME, DEFAULT_PLAYER_BODY_HEIGHT,
    DEFAULT_PLAYER_BODY_WIDTH, DEFAULT_TUNING, DODGE_ROLL_COOLDOWN, DODGE_ROLL_SPEED,
    DODGE_ROLL_TIME, DOUBLE_JUMP_SPEED, FAST_FALL_ACCEL, FAST_FALL_SPEED, FLIGHT_ACCEL,
    FLIGHT_DRAG, FLIGHT_HOVER_HZ, FLIGHT_HOVER_SPEED, FLIGHT_TERMINAL_SPEED, GRAVITY,
    GROUND_FRICTION, JUMP_BUFFER, JUMP_SPEED, MAX_FALL_SPEED, MAX_RUN_SPEED, PARRY_WINDOW_TIME,
    POGO_SPEED, PRECISION_BLINK_AIM_SPEED, PRECISION_BLINK_DISTANCE, RUN_ACCEL, SLASH_RECOIL,
    WALL_CLIMB_SPEED, WALL_JUMP_X, WALL_SLIDE_SPEED,
};
pub use player_state::{
    classify_player_safety, classify_safety_from_kinematics, try_change_body_mode,
    try_change_body_mode_clusters, BodyMode,
    BodyShape, LocomotionState, PlayerSafetyVerdict, ResourceMeter,
};
pub use player_clusters::{
    refresh_movement_resources_clusters, reset_player_clusters,
    PlayerAbilities as EnginePlayerAbilities, PlayerActionBuffer as EnginePlayerActionBuffer,
    PlayerBlinkState as EnginePlayerBlinkState,
    PlayerBodyModeState as EnginePlayerBodyModeState, PlayerClusterQueryData,
    PlayerClusterQueryDataItem, PlayerClustersMut,
    PlayerComboTrace as EnginePlayerComboTrace, PlayerDashState as EnginePlayerDashState,
    PlayerDodgeState as EnginePlayerDodgeState,
    PlayerEnvironmentContact as EnginePlayerEnvironmentContact,
    PlayerFlightState as EnginePlayerFlightState,
    PlayerGroundState as EnginePlayerGroundState, PlayerJumpState as EnginePlayerJumpState,
    PlayerKinematics as EnginePlayerKinematics, PlayerLedgeState as EnginePlayerLedgeState,
    PlayerLifetime as EnginePlayerLifetime, PlayerMana as EnginePlayerMana,
    PlayerOffense as EnginePlayerOffense, PlayerShieldState as EnginePlayerShieldState,
    PlayerWallState as EnginePlayerWallState,
};
pub use world::{
    BlinkWallTier, Block, BlockKind, ClimbableContact, ClimbableKind, ClimbableRegion,
    ClimbableSpec, WaterContact, WaterKind, WaterRegion, WaterVolumeSpec, World,
};
