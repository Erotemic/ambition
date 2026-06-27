//! The pure, content-free movement/physics MODEL — the math the rest of
//! the workspace builds on.
//!
//! This is the foundation crate: a deterministic, mostly Bevy-free
//! kinematic platformer simulation with no game-specific content. It owns
//! the testable gameplay rules every sibling crate reuses — kinematic
//! body movement, AABB collision semantics, ability gates, ledge-grab
//! probes, world collision/water/climbable region data, gravity-relative
//! reference frames, and the cluster components (`BodyKinematics`,
//! `BodyGroundState`, …, `BodyComboTrace`) that make up an actor's ECS
//! body (the player included). `ambition_characters` (minds/cast),
//! `ambition_content` (named game content), and `ambition_gameplay_core`
//! (machinery) all sit above it.
//!
//! Top-level modules: [`abilities`] (capability flags), [`config`]
//! (coordinate/layer constants), [`geometry`] (Aabb2d-backed collision
//! primitives), [`ledge_grab`] (ledge probe + state), [`movement`] (the
//! body movement spine + tuning), [`body_clusters`] (the authoritative
//! cluster components + the `BodyClustersMut` view), [`player_state`]
//! (locomotion/body-mode/resource-meter vocabulary), [`reference_frame`]
//! (gravity-relative frame transforms), and [`world`] (room block data).
//!
//! There is no monolithic body aggregate: the cluster components are the
//! only body state, and every entry point in [`movement`] operates on a
//! [`body_clusters::BodyClustersMut`] view natively. Tests build a
//! non-ECS scratchpad via
//! [`body_clusters::BodyClusterScratch::new_with_abilities`].

pub mod abilities;
pub mod collision_semantics;
pub mod combat_volume;
pub mod config;
pub mod volume_shape;
pub mod geometry;
pub mod ledge_grab;
pub mod movement;
pub mod body_clusters;
pub mod player_state;
pub mod reference_frame;
pub mod world;

// Re-export the public surface so story/sandbox crates can treat the engine as
// the main mechanics API while the internals stay organized by concern.
pub use abilities::AbilitySet;
pub use bevy_math::Vec2;
pub use combat_volume::CombatVolume;
pub use geometry::{aabb_from_min_size, Aabb, AabbExt, CenteredAabb};
pub use volume_shape::{VolumeShape, DUMMY_HALF};
pub use ledge_grab::{
    probe_ledge_grab, LedgeContact, LedgeGetupKind, LedgeGrabState, LEDGE_CLIMB_TIME,
    LEDGE_GRAB_INVULN_TIME, LEDGE_MIN_CLIMB_DELAY, LEDGE_ROLL_OVERSHOOT, LEDGE_ROLL_TIME,
    LEDGE_TOWARD_CLIMB_DELAY,
};
pub use movement::{
    blink_destination_clusters, blink_destination_to_point_clusters, default_player_body_size,
    integrate_normal_spine, resolve_shield, update_body_control_with_clusters,
    update_body_simulation_with_clusters, update_body_with_tuning_clusters, update_player_clusters,
    update_player_control_scratch,
    update_player_control_with_clusters, update_player_control_with_tuning_scratch,
    update_player_scratch, update_player_simulation_scratch,
    update_player_simulation_with_clusters, update_player_simulation_with_tuning_scratch,
    update_player_with_tuning_clusters, update_player_with_tuning_scratch, BlinkEvent, ComboMark,
    FrameEvents, InputState, LedgeMomentumTuning, MovementOp, MovementTuning, NormalSpineCtx,
    AIR_ACCEL, AIR_FRICTION, AIR_JUMPS, BLINK_COOLDOWN, BLINK_DISTANCE, BLINK_HOLD_THRESHOLD,
    COYOTE_TIME, DASH_BUFFER, DASH_COOLDOWN, DASH_SPEED, DASH_TIME, DEFAULT_PLAYER_BODY_HEIGHT,
    DEFAULT_GRAVITY_DIR, DEFAULT_GRAVITY_SIGN, DEFAULT_PLAYER_BODY_WIDTH, DEFAULT_TUNING,
    DODGE_ROLL_COOLDOWN, DODGE_ROLL_SPEED,
    DODGE_ROLL_TIME, DOUBLE_JUMP_SPEED, FAST_FALL_ACCEL, FAST_FALL_SPEED, FLIGHT_ACCEL,
    FLIGHT_DRAG, FLIGHT_HOVER_HZ, FLIGHT_HOVER_SPEED, FLIGHT_TERMINAL_SPEED, GRAVITY,
    GROUND_FRICTION, JUMP_BUFFER, JUMP_SPEED, MAX_FALL_SPEED, MAX_RUN_SPEED, PARRY_WINDOW_TIME,
    POGO_SPEED, PRECISION_BLINK_AIM_SPEED, PRECISION_BLINK_DISTANCE, RUN_ACCEL, SLASH_RECOIL,
    WALL_CLIMB_SPEED, WALL_JUMP_X, WALL_SLIDE_SPEED,
};
pub use body_clusters::{
    refresh_movement_resources_clusters, reset_body_clusters, BodyAbilities, BodyActionBuffer,
    BodyBaseSize, BodyBlinkState, BodyComboTrace, BodyDashState, BodyDodgeState,
    BodyEnvironmentContact, BodyFlightState, BodyGroundState, BodyJumpState, BodyKinematics,
    BodyLedgeState, BodyLifetime, BodyMana, BodyModeState, BodyOffense, BodyShieldState,
    BodyWallState, BodyClusterQueryData, BodyClusterQueryDataItem, BodyClusterScratch,
    BodyClustersMut,
};
pub use player_state::{
    classify_safety_from_kinematics, try_change_body_mode_clusters, BodyMode, BodyShape,
    LocomotionState, PlayerSafetyVerdict, ResourceMeter,
};
pub use reference_frame::{
    AccelerationFrame, ControlFrameModes, GameplayFramePolicy, InputFrameMode, RawDirectionEdges,
    ResolvedControlFrame,
};
pub use world::{
    BlinkWallTier, Block, BlockKind, ClimbableContact, ClimbableKind, ClimbableRegion,
    ClimbableSpec, WaterContact, WaterKind, WaterRegion, WaterVolumeSpec, World,
};
