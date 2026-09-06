//! Content-free deterministic movement and body-state foundation.
//!
//! Actor bodies, including player-controlled bodies, use the same cluster
//! components and [`body_clusters::BodyClustersMut`] movement seam. Tests use
//! [`body_clusters::BodyClusterScratch`] as the non-ECS form of that same state.

pub mod abilities;
pub mod body_clusters;
pub mod cast;
pub mod collision_semantics;
pub mod config;
pub mod confirmed_frame;
pub mod content_epoch;
pub mod control_frame;
pub mod frame;
pub mod geo_id;
pub mod hit_response;
pub mod input_stream;
pub mod kinematic_path;
pub mod ledge_grab;
pub mod motion_codec;
pub mod motion_quality;
pub mod movement;
pub mod player_state;
pub mod rollback_registration;
pub mod sim_random;
pub mod snapshot;
mod snapshot_impls;
#[cfg(test)]
pub(crate) mod test_support;
pub mod world;

// Public mechanics surface.
/// `#[serde(default)]` for a bool whose absent value is `true`.
pub(crate) fn default_true() -> bool {
    true
}

pub use abilities::{AbilityGrant, AbilitySet, MatchAbilities, MatchBody};
pub use bevy_math::Vec2;
pub use body_clusters::{
    announce_body_restarts, refresh_movement_resources_clusters, reset_body_clusters, AbilityBase,
    ActorSurfaceState, AuthoredMovementTuning, BodyAbilities, BodyActionBuffer, BodyBaseSize,
    BodyBlinkState, BodyClusterQueryData, BodyClusterQueryDataItem, BodyClusterScratch,
    BodyClustersMut, BodyComboTrace, BodyDashState, BodyDodgeState, BodyEnvironmentContact,
    BodyFlightState, BodyGroundState, BodyJumpState, BodyKinematics, BodyLedgeState, BodyLifetime,
    BodyMana, BodyModeState, BodyOffense, BodyRestarted, BodyShieldState, BodyWallState,
    RecoveryRefresh, SweepSample, DEFAULT_RECOVERY_CHARGES,
};
pub use rollback_registration::register_rollback_state;
// TODO(compat-remove): migrate geometry/frame callers to `ambition_geometry`, then remove
// these extraction-era re-exports from `ambition_platformer2d_core`.
pub use ambition_geometry::combat_volume::CombatVolume;
pub use ambition_geometry::geometry::{aabb_from_min_size, Aabb, AabbExt, CenteredAabb};
pub use ambition_geometry::reference_frame::{
    AccelerationFrame, CameraReferenceFrame, ControlFrameModes, GameplayFramePolicy,
    InputFrameMode, LocalAxes, MotionFrame, RawDirectionEdges, ResolvedControlFrame, ScreenAxes,
    WorldVec2,
};
pub use ambition_geometry::swing_shape::SwingShape;
pub use ambition_geometry::volume_shape::{VolumeShape, DUMMY_HALF};
pub use ambition_geometry::{combat_volume, geometry, reference_frame, swing_shape, volume_shape};
/// Which sim frames are settled (netcode: the confirmed boundary). Absent on
/// every non-rollback host, where it means "confirm everything".
pub use confirmed_frame::{world_state_is_confirmed, ConfirmedFrameBoundary};
pub use content_epoch::ContentEpoch;
pub use control_frame::AttackStrengthHint;
pub use control_frame::ControlFrame;
/// One seat's frame→tick input latch (netcode N0.1). a VALUE, not a resource:
/// the table that holds one per seat is `SlotControlLatches`, one layer up where
/// `PlayerSlot` exists.
pub use control_frame::ControlFrameLatch;
pub use geo_id::{Face, GeoFaceRef, GeoId, GeoSource, PlacementId};
/// The per-tick input artifact (netcode N0.2): replay, RL, forensics, wire.
pub use input_stream::{InputStream, InputStreamError, InputStreamFrame, INPUT_STREAM_VERSION};
pub use kinematic_path::{resolve_kinematic_path, KinematicPath, KinematicPathMode};
pub use ledge_grab::{
    probe_ledge_grab, LedgeContact, LedgeGetupKind, LedgeGrabState, LEDGE_CLIMB_TIME,
    LEDGE_GRAB_INVULN_TIME, LEDGE_MIN_CLIMB_DELAY, LEDGE_ROLL_OVERSHOOT, LEDGE_ROLL_TIME,
    LEDGE_TOWARD_CLIMB_DELAY,
};
pub use movement::{
    arrive_body_in_room, blink_destination_clusters, blink_destination_to_point_clusters,
    catch_the_wire, cut_the_wire, default_player_body_size, footstool_victim,
    integrate_normal_spine, knock_off_ledge, resolve_burst_maneuver, resolve_shield, step_motion,
    switch_motion_model, ActionEdges, ActionKey, ActiveMovementTuning, AdhesiveCrawlerMotion,
    ArrivalMomentum, AxisHorizontalLaw, AxisJumpLaw, AxisLocomotion, AxisManeuverState,
    AxisSweptMotion, AxisSweptParams, BlinkEvent, BodyContactBlocker, BodyContactField,
    BodyMotionFacts, BurstManeuver, ComboMark, CrawlAttachment, CrawlerParams, CrawlerState,
    DepthOcclusions, Edge, FlightTuning, FootstoolTuning, FrameEvents, GroundContactTransition,
    InputState, LedgeFacts, LedgeMomentumTuning, MomentumHorizontalTuning, MomentumParams,
    MotionModel, MotionModelKind, MotionModelSpec, MotionStepContext, MotionStepResult,
    MovementAction, MovementOp, MovementTuning, NormalSpineCtx, OcclusionSpan, OutOfShield,
    OutOfShieldAction, OutOfShieldGate, ParryTiming, PhasedGravityJumpTuning, PhasedJumpState,
    PoseOwnedExternally, ResetCause, RouteDeparture, ShieldTuning, SurfaceMomentumMotion,
    SurfaceMotion, SurfaceRef, TraversalAbilityTuning, WireState, AIR_ACCEL, AIR_DODGE_ENDLAG,
    AIR_DODGE_SPEED, AIR_DODGE_TIME, AIR_FRICTION, AIR_JUMPS, BLINK_COOLDOWN, BLINK_DISTANCE,
    BLINK_HOLD_THRESHOLD, COYOTE_TIME, DASH_BUFFER, DASH_COOLDOWN, DASH_SPEED, DASH_TIME,
    DEFAULT_AXIS_SWEPT_PARAMS, DEFAULT_GRAVITY_DIR, DEFAULT_PLAYER_BODY_HEIGHT,
    DEFAULT_PLAYER_BODY_WIDTH, DEFAULT_TUNING, DODGE_ROLL_COOLDOWN, DODGE_ROLL_ENDLAG,
    DODGE_ROLL_SPEED, DODGE_ROLL_TIME, DOUBLE_JUMP_SPEED, FAST_FALL_ACCEL, FAST_FALL_SPEED,
    FLIGHT_ACCEL, FLIGHT_DRAG, FLIGHT_HOVER_HZ, FLIGHT_HOVER_SPEED, FLIGHT_TERMINAL_SPEED, GRAVITY,
    GROUND_FRICTION, JUMP_BUFFER, JUMP_SPEED, MAX_FALL_SPEED, MAX_RUN_SPEED, PARRY_WINDOW_TIME,
    POGO_SPEED, PRECISION_BLINK_AIM_SPEED, PRECISION_BLINK_DISTANCE, RUN_ACCEL, SLASH_RECOIL,
    SPOT_DODGE_STICK, SPOT_DODGE_TIME, WALL_CLIMB_SPEED, WALL_JUMP_X, WALL_SLIDE_SPEED,
};
pub use player_state::{
    classify_safety_from_kinematics, resize_feet_planted, try_change_body_mode_clusters, BodyMode,
    BodyShape, LocomotionState, PlayerSafetyVerdict, ResourceMeter,
};
pub use world::{
    BlinkWallTier, Block, BlockKind, ClimbableContact, ClimbableKind, ClimbableRegion,
    ClimbableSpec, RoomGeometry, SurfaceChain, SurfaceFrame, SurfaceJunction, SurfaceKind,
    SurfacePort, WaterContact, WaterKind, WaterRegion, WaterVolumeSpec, World,
};
