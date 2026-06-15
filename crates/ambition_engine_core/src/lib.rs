//! Reusable Bevy-native simulation primitives.
//!
//! Formerly the `ambition_engine` crate; collapsed into the sandbox
//! 2026-05-28. This module owns the testable gameplay rules a future
//! sibling crate could reuse: kinematic player movement, AABB
//! collision semantics, ability gates, ledge-grab probes, world
//! collision/water/climbable region data, and the cluster components
//! (`BodyKinematics`, `PlayerGroundState`, …, `PlayerComboTrace`)
//! that make up the player ECS entity.
//!
//! Sandbox-side concerns (LDtk entities, per-room authored Vecs,
//! sandbox dispatch tables, presentation) stay outside this module.
//!
//! As of 2026-05-28 the monolithic `ae::Player` aggregate has been
//! deleted. The 18 cluster components on the player entity are the
//! only player state, and every engine entry point
//! (`update_player_{control,simulation}_with_clusters`,
//! `tick_active_ledge_grab_clusters`, `try_start_ledge_grab_clusters`,
//! `integrate_velocity_clusters`, the sweep helpers in
//! `movement/collision`) operates on cluster refs natively. Tests
//! build a non-ECS scratchpad via
//! [`player_clusters::PlayerClusterScratch::new_with_abilities`]. See
//! `dev/journals/player-cluster-native-push-2026-05-28.md`.

pub mod abilities;
pub mod config;
pub mod geometry;
pub mod ledge_grab;
pub mod movement;
pub mod player_clusters;
pub mod player_state;
pub mod world;

// Re-export the public surface so story/sandbox crates can treat the engine as
// the main mechanics API while the internals stay organized by concern.
pub use abilities::AbilitySet;
pub use bevy_math::Vec2;
pub use geometry::{aabb_from_min_size, Aabb, AabbExt, Bounds};
pub use ledge_grab::{
    probe_ledge_grab, LedgeContact, LedgeGetupKind, LedgeGrabState, LEDGE_CLIMB_TIME,
    LEDGE_GRAB_INVULN_TIME, LEDGE_MIN_CLIMB_DELAY, LEDGE_ROLL_OVERSHOOT, LEDGE_ROLL_TIME,
    LEDGE_TOWARD_CLIMB_DELAY,
};
pub use movement::{
    blink_destination_clusters, blink_destination_to_point_clusters, default_player_body_size,
    update_player_clusters, update_player_control_scratch, update_player_control_with_clusters,
    update_player_control_with_tuning_scratch, update_player_scratch,
    update_player_simulation_scratch, update_player_simulation_with_clusters,
    update_player_simulation_with_tuning_scratch, update_player_with_tuning_clusters,
    update_player_with_tuning_scratch, BlinkEvent, ComboMark, FrameEvents, InputState,
    LedgeMomentumTuning, MovementOp, MovementTuning, AIR_ACCEL, AIR_FRICTION, AIR_JUMPS,
    BLINK_COOLDOWN, BLINK_DISTANCE, BLINK_HOLD_THRESHOLD, COYOTE_TIME, DASH_BUFFER, DASH_COOLDOWN,
    DASH_SPEED, DASH_TIME, DEFAULT_PLAYER_BODY_HEIGHT, DEFAULT_PLAYER_BODY_WIDTH, DEFAULT_TUNING,
    DODGE_ROLL_COOLDOWN, DODGE_ROLL_SPEED, DODGE_ROLL_TIME, DOUBLE_JUMP_SPEED, FAST_FALL_ACCEL,
    FAST_FALL_SPEED, FLIGHT_ACCEL, FLIGHT_DRAG, FLIGHT_HOVER_HZ, FLIGHT_HOVER_SPEED,
    FLIGHT_TERMINAL_SPEED, GRAVITY, GROUND_FRICTION, JUMP_BUFFER, JUMP_SPEED, MAX_FALL_SPEED,
    MAX_RUN_SPEED, PARRY_WINDOW_TIME, POGO_SPEED, PRECISION_BLINK_AIM_SPEED,
    PRECISION_BLINK_DISTANCE, RUN_ACCEL, SLASH_RECOIL, WALL_CLIMB_SPEED, WALL_JUMP_X,
    WALL_SLIDE_SPEED,
};
pub use player_clusters::{
    refresh_movement_resources_clusters, reset_player_clusters, BodyKinematics,
    PlayerAbilities as EnginePlayerAbilities, PlayerActionBuffer as EnginePlayerActionBuffer,
    PlayerBaseSize as EnginePlayerBaseSize, PlayerBlinkState as EnginePlayerBlinkState,
    PlayerBodyModeState as EnginePlayerBodyModeState, PlayerClusterQueryData,
    PlayerClusterQueryDataItem, PlayerClusterScratch, PlayerClustersMut,
    PlayerComboTrace as EnginePlayerComboTrace, PlayerDashState as EnginePlayerDashState,
    PlayerDodgeState as EnginePlayerDodgeState,
    PlayerEnvironmentContact as EnginePlayerEnvironmentContact,
    PlayerFlightState as EnginePlayerFlightState, PlayerGroundState as EnginePlayerGroundState,
    PlayerJumpState as EnginePlayerJumpState, PlayerLedgeState as EnginePlayerLedgeState,
    PlayerLifetime as EnginePlayerLifetime, PlayerMana as EnginePlayerMana,
    PlayerOffense as EnginePlayerOffense, PlayerShieldState as EnginePlayerShieldState,
    PlayerWallState as EnginePlayerWallState,
};
pub use player_state::{
    classify_safety_from_kinematics, try_change_body_mode_clusters, BodyMode, BodyShape,
    LocomotionState, PlayerSafetyVerdict, ResourceMeter,
};
pub use world::{
    BlinkWallTier, Block, BlockKind, ClimbableContact, ClimbableKind, ClimbableRegion,
    ClimbableSpec, WaterContact, WaterKind, WaterRegion, WaterVolumeSpec, World,
};
