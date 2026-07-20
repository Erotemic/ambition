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
//! `ambition_content` (named game content), and `ambition_actors`
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
pub mod body_clusters;
pub mod cast;
pub mod collision_semantics;
pub mod combat_volume;
pub mod config;
pub mod control_frame;
pub mod frame;
pub mod geo_id;
pub mod geometry;
pub mod input_stream;
pub mod kinematic_path;
pub mod ledge_grab;
pub mod movement;
pub mod player_state;
pub mod reference_frame;
#[cfg(test)]
pub(crate) mod test_support;
pub mod volume_shape;
pub mod world;

// Re-export the public surface so story/sandbox crates can treat the engine as
// the main mechanics API while the internals stay organized by concern.
pub use abilities::{AbilityGrant, AbilitySet};
pub use bevy_math::Vec2;
pub use body_clusters::{
    refresh_movement_resources_clusters, reset_body_clusters, AbilityBase, AuthoredMovementTuning,
    BodyAbilities, BodyActionBuffer, BodyBaseSize, BodyBlinkState, BodyClusterQueryData,
    BodyClusterQueryDataItem, BodyClusterScratch, BodyClustersMut, BodyComboTrace, BodyDashState,
    BodyDodgeState, BodyEnvironmentContact, BodyFlightState, BodyGroundState, BodyJumpState,
    BodyKinematics, BodyLedgeState, BodyLifetime, BodyMana, BodyModeState, BodyOffense,
    BodyShieldState, BodyWallState, SweepSample,
};
pub use combat_volume::CombatVolume;
pub use control_frame::ControlFrame;
/// The frame→tick input latch (netcode N0.1) and its two systems. Separate from
/// the `ControlFrame` vocabulary above because only the DEVICE layer installs it.
pub use control_frame::{
    accumulate_control_frame_latch, publish_latched_control_frame, ControlFrameLatch,
};
pub use geo_id::{Face, GeoFaceRef, GeoId, GeoSource, PlacementId};
pub use geometry::{aabb_from_min_size, Aabb, AabbExt, CenteredAabb};
/// The per-tick input artifact (netcode N0.2): replay, RL, forensics, wire.
pub use input_stream::{InputStream, InputStreamError, InputStreamFrame, INPUT_STREAM_VERSION};
pub use kinematic_path::{KinematicPath, KinematicPathMode};
pub use ledge_grab::{
    probe_ledge_grab, LedgeContact, LedgeGetupKind, LedgeGrabState, LEDGE_CLIMB_TIME,
    LEDGE_GRAB_INVULN_TIME, LEDGE_MIN_CLIMB_DELAY, LEDGE_ROLL_OVERSHOOT, LEDGE_ROLL_TIME,
    LEDGE_TOWARD_CLIMB_DELAY,
};
pub use movement::{
    blink_destination_clusters, blink_destination_to_point_clusters, default_player_body_size,
    integrate_normal_spine, knock_off_ledge, resolve_shield, step_motion, switch_motion_model,
    ActionEdges, ActionKey, ActiveMovementTuning, AdhesiveCrawlerMotion, AxisLocomotion,
    AxisManeuverState, AxisSweptMotion, AxisSweptParams, BlinkEvent, BodyMotionFacts, ComboMark,
    CrawlAttachment, CrawlerParams, CrawlerState, DepthOcclusions, Edge, FlightTuning, FrameEvents,
    GroundContactTransition, InputState, LedgeFacts, LedgeMomentumTuning, MomentumParams,
    MotionModel, MotionModelKind, MotionModelSpec, MotionStepContext, MotionStepResult,
    MovementAction, MovementOp, MovementTuning, NormalSpineCtx, OcclusionSpan, RouteDeparture,
    SurfaceMomentumMotion, SurfaceMotion, SurfaceRef, TraversalAbilityTuning, AIR_ACCEL,
    AIR_FRICTION, AIR_JUMPS, BLINK_COOLDOWN, BLINK_DISTANCE, BLINK_HOLD_THRESHOLD, COYOTE_TIME,
    DASH_BUFFER, DASH_COOLDOWN, DASH_SPEED, DASH_TIME, DEFAULT_AXIS_SWEPT_PARAMS,
    DEFAULT_GRAVITY_DIR, DEFAULT_PLAYER_BODY_HEIGHT, DEFAULT_PLAYER_BODY_WIDTH, DEFAULT_TUNING,
    DODGE_ROLL_COOLDOWN, DODGE_ROLL_SPEED, DODGE_ROLL_TIME, DOUBLE_JUMP_SPEED, FAST_FALL_ACCEL,
    FAST_FALL_SPEED, FLIGHT_ACCEL, FLIGHT_DRAG, FLIGHT_HOVER_HZ, FLIGHT_HOVER_SPEED,
    FLIGHT_TERMINAL_SPEED, GRAVITY, GROUND_FRICTION, JUMP_BUFFER, JUMP_SPEED, MAX_FALL_SPEED,
    MAX_RUN_SPEED, PARRY_WINDOW_TIME, POGO_SPEED, PRECISION_BLINK_AIM_SPEED,
    PRECISION_BLINK_DISTANCE, RUN_ACCEL, SLASH_RECOIL, WALL_CLIMB_SPEED, WALL_JUMP_X,
    WALL_SLIDE_SPEED,
};
pub use player_state::{
    classify_safety_from_kinematics, try_change_body_mode_clusters, BodyMode, BodyShape,
    LocomotionState, PlayerSafetyVerdict, ResourceMeter,
};
pub use reference_frame::{
    AccelerationFrame, ControlFrameModes, GameplayFramePolicy, InputFrameMode, LocalAxes,
    MotionFrame, RawDirectionEdges, ResolvedControlFrame, ScreenAxes, WorldVec2,
};
pub use volume_shape::{VolumeShape, DUMMY_HALF};
pub use world::{
    BlinkWallTier, Block, BlockKind, ClimbableContact, ClimbableKind, ClimbableRegion,
    ClimbableSpec, RoomGeometry, SurfaceChain, SurfaceFrame, SurfaceJunction, SurfaceKind,
    SurfacePort, WaterContact, WaterKind, WaterRegion, WaterVolumeSpec, World,
};
