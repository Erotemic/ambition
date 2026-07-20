//! Movement parameter architecture.
//!
//! Three layers with different owners:
//!
//! - [`MovementTuning`] — the flat AUTHORING/CONTROL-BOUNDARY aggregate content
//!   and dev tools hydrate (RON files, editable tuning). It also carries the
//!   authored gravity RESPONSE magnitude, which is an input to the environment's
//!   frame resolver — never to a movement policy.
//! - [`AxisSweptParams`] — the axis-swept POLICY's authored parameters, grouped
//!   by ownership: [`AxisLocomotion`] (the locomotion law itself),
//!   [`TraversalAbilityTuning`] (optional ability verbs the axis control phase
//!   executes), and [`FlightTuning`] (the free-flight limb).
//! - The environment's current frame (gravity direction, acceleration,
//!   reference orientation) and the controller's input-frame preference are
//!   deliberately NOT here. They enter the kernel through
//!   [`crate::MotionFrame`] and the typed input seam respectively, so a policy
//!   can be swapped or snapshotted without freezing the frame that happened to
//!   be active.

use serde::{Deserialize, Serialize};

use crate::Vec2;

/// `serde(default)` for tuning files baked before `air_stop_assist` existed.
fn default_air_stop_assist() -> f32 {
    AIR_STOP_ASSIST
}

// First-pass movement constants. These remain constants for easy grep/tuning,
// but the simulation accepts a `MovementTuning` so experiments can override
// them without recompiling every assumption into the update function.
pub const GRAVITY: f32 = 2250.0;
/// THE default gravity DIRECTION (`+Y` is screen-down). Single source of truth
/// for "down" — `DEFAULT_TUNING` and the world `GravityField`/`BaseGravity`
/// resources (in `ambition_platformer_primitives`) all resolve here so a flip
/// of the convention is a one-line change.
pub const DEFAULT_GRAVITY_DIR: Vec2 = Vec2::new(0.0, 1.0);
pub const RUN_ACCEL: f32 = 5200.0;
pub const AIR_ACCEL: f32 = 3100.0;
pub const GROUND_FRICTION: f32 = 7600.0;
pub const AIR_FRICTION: f32 = 650.0;
/// Hands-off airborne stop assist: matches the pre-carried-momentum feel of
/// the zero-target approach (`AIR_ACCEL`) + `AIR_FRICTION` stacking.
pub const AIR_STOP_ASSIST: f32 = 3750.0;
pub const MAX_RUN_SPEED: f32 = 270.0;
// Raised for momentum-preserving portal play (Portal-style flings): you
// can build and carry much more speed before the fall cap clips it. The
// fast-fall cap scales with it.
pub const MAX_FALL_SPEED: f32 = 1900.0;
pub const JUMP_SPEED: f32 = 630.0;
pub const DOUBLE_JUMP_SPEED: f32 = 520.0;
pub const WALL_JUMP_X: f32 = 430.0;
pub const WALL_SLIDE_SPEED: f32 = 145.0;
pub const WALL_CLIMB_SPEED: f32 = 210.0;
pub const DASH_SPEED: f32 = 760.0;
pub const DASH_TIME: f32 = 0.115;
pub const DASH_COOLDOWN: f32 = 0.160;
/// Grace window for a dash press that happens just before dash becomes legal.
pub const DASH_BUFFER: f32 = 0.100;
pub const BLINK_DISTANCE: f32 = 190.0;
pub const PRECISION_BLINK_DISTANCE: f32 = 430.0;
pub const PRECISION_BLINK_AIM_SPEED: f32 = 1_650.0;
/// Hold duration before blink switches from quick 8-direction release to precision aim.
///
/// Keep this short so the player can deliberately enter granular blink control
/// without waiting through the snappy quick-blink window.
pub const BLINK_HOLD_THRESHOLD: f32 = 0.100;
pub const BLINK_COOLDOWN: f32 = 0.180;
/// Brief post-blink hang window that prevents repeated blinks from inheriting
/// runaway downward velocity. This is deliberately short: blink should feel
/// controlled, not like a full hover.
pub const BLINK_GRACE_TIME: f32 = 0.090;
/// Maximum downward velocity immediately after a quick blink.
pub const BLINK_MAX_DOWNWARD_SPEED: f32 = 55.0;
/// Maximum downward velocity immediately after a precision blink.
pub const PRECISION_BLINK_MAX_DOWNWARD_SPEED: f32 = 18.0;
pub const FAST_FALL_ACCEL: f32 = 1850.0;
pub const FAST_FALL_SPEED: f32 = 2400.0;
/// Glide / slow-fall vertical cap. Roughly 1/5 of `MAX_FALL_SPEED` so
/// the held-jump glide feels distinctly hover-y without becoming
/// effectively-flying. Pair with `glide_air_accel` for the increased
/// horizontal authority while gliding.
pub const GLIDE_FALL_SPEED: f32 = 220.0;
/// Horizontal acceleration while gliding. Higher than ordinary
/// `air_accel` (4700) so the player can steer mid-glide; lower than
/// `run_accel` (7600) so ground feel still beats air feel.
pub const GLIDE_AIR_ACCEL: f32 = 6200.0;
pub const FLIGHT_ACCEL: f32 = 3200.0;
pub const FLIGHT_DRAG: f32 = 2400.0;
pub const FLIGHT_TERMINAL_SPEED: f32 = 760.0;
pub const FLIGHT_HOVER_SPEED: f32 = 35.0;
pub const FLIGHT_HOVER_HZ: f32 = 0.85;
pub const COYOTE_TIME: f32 = 0.120;
pub const JUMP_BUFFER: f32 = 0.135;
/// Window during which the vertical sweep continues to ignore one-way
/// platforms after a drop-through gesture. Long enough to clear the 8px
/// landing tolerance under typical gravity, short enough that the player can
/// still re-land on a one-way they jump back up onto.
pub const ONE_WAY_DROP_THROUGH_GRACE: f32 = 0.18;
pub const POGO_SPEED: f32 = 720.0;
pub const SLASH_RECOIL: f32 = 110.0;
pub const AIR_JUMPS: u8 = 1;
/// Duration of the dodge-roll invulnerability window (seconds).
pub const DODGE_ROLL_TIME: f32 = 0.22;
/// Dodge-roll velocity: roughly 70 % of dash speed in the facing direction.
pub const DODGE_ROLL_SPEED: f32 = 530.0;
/// Cooldown after a dodge roll before the next one may start.
pub const DODGE_ROLL_COOLDOWN: f32 = 0.42;
/// Parry window: full invulnerability during the first moments of shield activation.
pub const PARRY_WINDOW_TIME: f32 = 0.15;

/// Ledge momentum-carry defaults. See [`LedgeMomentumTuning`] for the
/// per-field semantics. Tuned for Jon's "moving → grab → quick getup
/// gives a boost; sitting still on the ledge does not" feel:
/// - 250 ms window matches the existing regrab cooldown so the
///   "fresh grab" feel window is symmetric.
/// - x_gain = 0.85 carries most of the incoming run speed; the
///   previous 0.65 left too little kick once the cap clipped a
///   typical 270 px/s approach down to ~175 px/s.
/// - y_gain = 0.45 — only meaningful for ledge-jump (vertical hop);
///   climb / roll / attack finish zero this out entirely so they
///   don't launch the player off the platform they just landed on.
/// - Caps pin the boost so an extreme dash → ledge approach doesn't
///   exit at dash speed; ~jump_speed feels like the right ceiling.
/// - getup_speedup_gain shortens the climb/roll/attack transition
///   when momentum was carried, so the animation itself feels
///   continuous instead of "stop and go." 1.0 = full momentum
///   roughly halves the transition; 0.0 disables the speedup.
pub const LEDGE_BOOST_WINDOW: f32 = 0.25;
pub const LEDGE_BOOST_X_GAIN: f32 = 0.85;
pub const LEDGE_BOOST_Y_GAIN: f32 = 0.45;
pub const LEDGE_BOOST_X_CAP: f32 = 420.0;
pub const LEDGE_BOOST_Y_CAP: f32 = 540.0;
pub const LEDGE_GETUP_SPEEDUP_GAIN: f32 = 1.0;

/// Tunable momentum-carry parameters for ledge getups.
///
/// When the player grabs a ledge with non-trivial momentum and then
/// commits to a getup option (climb / roll / attack / jump) within
/// the boost window, the carried-over velocity is folded into the
/// getup so the player exits with a leftover horizontal/vertical
/// kick. The drop / outward-release options never get the boost —
/// those are deliberate disengage actions.
///
/// Set [`Self::window`] to `0.0` to disable the mechanic entirely.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LedgeMomentumTuning {
    /// Seconds after grab during which the incoming momentum is
    /// still claimable by a getup. The boost is weighted linearly
    /// across the window so an immediate action gets the full carry
    /// and an action right at the edge gets near-zero.
    pub window: f32,
    /// Fraction of horizontal momentum carried into the getup, when
    /// the player was moving INTO the platform at grab time. Momentum
    /// opposite the into-platform axis is discarded — the player
    /// wasn't "carrying forward speed," they were sliding backward.
    pub x_gain: f32,
    /// Fraction of upward (sim +Y-down → negative) vertical momentum
    /// carried into the getup. Downward momentum is discarded — the
    /// player was falling, not climbing.
    pub y_gain: f32,
    /// Per-axis cap on the carried boost so extreme approaches don't
    /// catapult the player. Compared against the post-gain magnitude.
    pub x_cap: f32,
    pub y_cap: f32,
    /// Shortens the climb / roll / getup-attack transition duration
    /// when momentum was carried into the getup. Full incoming
    /// momentum (boost weight = 1.0) divides the base duration by
    /// `1.0 + getup_speedup_gain`, so `gain = 1.0` halves the
    /// animation; `gain = 0.0` leaves it untouched.
    ///
    /// This is the fix for "the getup animation doesn't feel any
    /// faster, you stop and are sluggish, and then the boost doesn't
    /// compensate for that initial sluggish feeling" — the boost is
    /// applied across the duration, not just at the end.
    pub getup_speedup_gain: f32,
}

impl Default for LedgeMomentumTuning {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl LedgeMomentumTuning {
    pub const DEFAULT: Self = Self {
        window: LEDGE_BOOST_WINDOW,
        x_gain: LEDGE_BOOST_X_GAIN,
        y_gain: LEDGE_BOOST_Y_GAIN,
        x_cap: LEDGE_BOOST_X_CAP,
        y_cap: LEDGE_BOOST_Y_CAP,
        getup_speedup_gain: LEDGE_GETUP_SPEEDUP_GAIN,
    };

    /// Boost mechanic fully disabled. Set
    /// `MovementTuning::ledge_momentum = LedgeMomentumTuning::OFF`
    /// to fall back to the original "vel zeroed on grab" feel.
    pub const OFF: Self = Self {
        window: 0.0,
        x_gain: 0.0,
        y_gain: 0.0,
        x_cap: 0.0,
        y_cap: 0.0,
        getup_speedup_gain: 0.0,
    };
}

/// Authored movement/control profile used at the ECS and content boundary.
///
/// This flat aggregate is what content hydrates and dev tools edit. It is NOT
/// stored in [`super::MotionModel`]: the trusted axis-swept policy receives only
/// the grouped [`AxisSweptParams`] projection, and the `gravity` RESPONSE
/// magnitude below feeds the environment's per-body frame resolver, never a
/// policy. Current gravity direction, reference orientation, and input-frame
/// preference deliberately have no fields here.
/// The session's ACTIVE movement tuning: the one authority every simulation
/// system reads.
///
/// Neutral by construction. Content hydrates it from authored data; a developer
/// build lets the F3 inspector edit it through
/// `ambition_dev_tools`'s adapter. The simulation does not know which of those
/// happened, which is the point — before this existed, sim systems read the
/// inspector's mirror directly, so a shipping build still depended on the
/// editor.
///
/// A body may still override the session default by carrying its own
/// [`super::super::AuthoredMovementTuning`]; this is the fallback every body
/// shares.
#[derive(bevy_ecs::resource::Resource, Clone, Copy, Debug, Default)]
pub struct ActiveMovementTuning(pub MovementTuning);

impl core::ops::Deref for ActiveMovementTuning {
    type Target = MovementTuning;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<MovementTuning> for ActiveMovementTuning {
    fn from(tuning: MovementTuning) -> Self {
        Self(tuning)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MovementTuning {
    /// Authored gravity RESPONSE magnitude (px/s²) — an input the environment's
    /// frame resolver composes with the live gravity direction and any per-body
    /// response scale. Not a policy parameter.
    pub gravity: f32,
    pub run_accel: f32,
    pub air_accel: f32,
    pub ground_friction: f32,
    pub air_friction: f32,
    /// Hands-off airborne run deceleration (px/s²) toward the CARRIED floor
    /// (`BodyFlightState::carried_run`) — the tight "release the stick and
    /// fall straight down" feel, without ever bleeding momentum the world
    /// imparted (portal flings, knockback). `serde(default)` for tuning files
    /// baked before it existed.
    #[serde(default = "default_air_stop_assist")]
    pub air_stop_assist: f32,
    /// Passive bleed (px/s²) of the carried-momentum floor itself. 0 (the
    /// default) conserves a fling until input, a wall, or landing consumes
    /// it; positive values make the world slowly forget imparted momentum.
    #[serde(default)]
    pub carried_decay: f32,
    pub max_run_speed: f32,
    pub max_fall_speed: f32,
    pub jump_speed: f32,
    pub double_jump_speed: f32,
    pub wall_jump_x: f32,
    pub wall_slide_speed: f32,
    pub wall_climb_speed: f32,
    pub dash_speed: f32,
    pub dash_time: f32,
    pub dash_cooldown: f32,
    pub dash_buffer: f32,
    pub blink_distance: f32,
    pub precision_blink_distance: f32,
    pub precision_blink_aim_speed: f32,
    pub blink_hold_threshold: f32,
    pub blink_cooldown: f32,
    pub blink_grace_time: f32,
    pub blink_max_downward_speed: f32,
    pub precision_blink_max_downward_speed: f32,
    pub fast_fall_accel: f32,
    pub fast_fall_speed: f32,
    /// Vertical fall speed cap while gliding. See [`GLIDE_FALL_SPEED`].
    pub glide_fall_speed: f32,
    /// Horizontal acceleration applied while gliding, replacing `air_accel`.
    /// See [`GLIDE_AIR_ACCEL`].
    pub glide_air_accel: f32,
    pub flight_accel: f32,
    pub flight_drag: f32,
    pub flight_terminal_speed: f32,
    pub flight_hover_speed: f32,
    pub flight_hover_hz: f32,
    /// Direct-velocity free-mover: the controller commands an EXACT velocity each
    /// tick (a boss pattern's `desired_vel`), so the flight limb takes
    /// `stick × flight_terminal_speed` verbatim — no accel ramp, drag, hover-bob,
    /// or deadzone. `#[serde(default)]` (false) so pre-existing tuning files +
    /// every ordinary flyer (parrot, hover-drone) keep the smoothed accel/drag
    /// flight unchanged.
    #[serde(default)]
    pub flight_direct_velocity: bool,
    pub coyote_time: f32,
    pub jump_buffer: f32,
    pub pogo_speed: f32,
    pub slash_recoil: f32,
    pub air_jumps: u8,
    pub dodge_roll_time: f32,
    pub dodge_roll_speed: f32,
    pub dodge_roll_cooldown: f32,
    pub parry_window_time: f32,
    /// Momentum-carry parameters for ledge getups. Set to
    /// `LedgeMomentumTuning::OFF` to disable the mechanic.
    ///
    /// `#[serde(default)]` so any tuning files serialized before this
    /// field existed (e.g. `assets/ambition/sandbox.ron` baked at
    /// boot) deserialize with `LedgeMomentumTuning::DEFAULT` instead
    /// of panicking on `MissingStructField`.
    #[serde(default)]
    pub ledge_momentum: LedgeMomentumTuning,
}

/// The axis-swept LOCOMOTION law: ground/air run, jumps, walls, falling.
///
/// These parameters define how the body moves; ability verbs and the flight
/// limb are separate groups. No field here may describe the live environment.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AxisLocomotion {
    pub run_accel: f32,
    pub air_accel: f32,
    pub ground_friction: f32,
    pub air_friction: f32,
    /// See [`MovementTuning::air_stop_assist`].
    #[serde(default = "default_air_stop_assist")]
    pub air_stop_assist: f32,
    /// See [`MovementTuning::carried_decay`].
    #[serde(default)]
    pub carried_decay: f32,
    pub max_run_speed: f32,
    pub max_fall_speed: f32,
    pub jump_speed: f32,
    pub double_jump_speed: f32,
    pub wall_jump_x: f32,
    pub wall_slide_speed: f32,
    pub wall_climb_speed: f32,
    pub coyote_time: f32,
    pub jump_buffer: f32,
    pub air_jumps: u8,
    pub fast_fall_accel: f32,
    pub fast_fall_speed: f32,
    pub glide_fall_speed: f32,
    pub glide_air_accel: f32,
}

/// Optional traversal/combat ability tuning executed by the axis-swept control
/// phase (dash, blink, dodge, shield/parry, pogo, slash recoil, ledge getups).
/// Ability AVAILABILITY is the body's `AbilitySet`; these are the verbs' knobs.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraversalAbilityTuning {
    pub dash_speed: f32,
    pub dash_time: f32,
    pub dash_cooldown: f32,
    pub dash_buffer: f32,
    pub blink_distance: f32,
    pub precision_blink_distance: f32,
    pub precision_blink_aim_speed: f32,
    pub blink_hold_threshold: f32,
    pub blink_cooldown: f32,
    pub blink_grace_time: f32,
    pub blink_max_downward_speed: f32,
    pub precision_blink_max_downward_speed: f32,
    pub pogo_speed: f32,
    pub slash_recoil: f32,
    pub dodge_roll_time: f32,
    pub dodge_roll_speed: f32,
    pub dodge_roll_cooldown: f32,
    pub parry_window_time: f32,
    #[serde(default)]
    pub ledge_momentum: LedgeMomentumTuning,
}

/// The free-flight limb's tuning (hover, glide-steer, direct-velocity movers).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlightTuning {
    pub accel: f32,
    pub drag: f32,
    pub terminal_speed: f32,
    pub hover_speed: f32,
    pub hover_hz: f32,
    /// See [`MovementTuning::flight_direct_velocity`].
    #[serde(default)]
    pub direct_velocity: bool,
}

/// Parameters owned by the axis-swept movement policy, grouped by ownership.
///
/// This type intentionally contains no gravity vector, acceleration magnitude,
/// reference orientation, or input-frame preference. Those are current
/// environmental/control facts and enter the kernel through
/// [`crate::MotionFrame`] and already-resolved typed input, respectively. A
/// model can therefore be swapped or snapshotted without freezing the reference
/// frame that happened to be active.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AxisSweptParams {
    pub locomotion: AxisLocomotion,
    pub abilities: TraversalAbilityTuning,
    pub flight: FlightTuning,
}

impl Default for MovementTuning {
    fn default() -> Self {
        DEFAULT_TUNING
    }
}

impl MovementTuning {
    /// Project the frame-independent policy parameters consumed by the trusted
    /// axis-swept solver. Environment and input mapping remain outside the model.
    pub const fn axis_swept_params(self) -> AxisSweptParams {
        AxisSweptParams {
            locomotion: AxisLocomotion {
                run_accel: self.run_accel,
                air_accel: self.air_accel,
                ground_friction: self.ground_friction,
                air_friction: self.air_friction,
                air_stop_assist: self.air_stop_assist,
                carried_decay: self.carried_decay,
                max_run_speed: self.max_run_speed,
                max_fall_speed: self.max_fall_speed,
                jump_speed: self.jump_speed,
                double_jump_speed: self.double_jump_speed,
                wall_jump_x: self.wall_jump_x,
                wall_slide_speed: self.wall_slide_speed,
                wall_climb_speed: self.wall_climb_speed,
                coyote_time: self.coyote_time,
                jump_buffer: self.jump_buffer,
                air_jumps: self.air_jumps,
                fast_fall_accel: self.fast_fall_accel,
                fast_fall_speed: self.fast_fall_speed,
                glide_fall_speed: self.glide_fall_speed,
                glide_air_accel: self.glide_air_accel,
            },
            abilities: TraversalAbilityTuning {
                dash_speed: self.dash_speed,
                dash_time: self.dash_time,
                dash_cooldown: self.dash_cooldown,
                dash_buffer: self.dash_buffer,
                blink_distance: self.blink_distance,
                precision_blink_distance: self.precision_blink_distance,
                precision_blink_aim_speed: self.precision_blink_aim_speed,
                blink_hold_threshold: self.blink_hold_threshold,
                blink_cooldown: self.blink_cooldown,
                blink_grace_time: self.blink_grace_time,
                blink_max_downward_speed: self.blink_max_downward_speed,
                precision_blink_max_downward_speed: self.precision_blink_max_downward_speed,
                pogo_speed: self.pogo_speed,
                slash_recoil: self.slash_recoil,
                dodge_roll_time: self.dodge_roll_time,
                dodge_roll_speed: self.dodge_roll_speed,
                dodge_roll_cooldown: self.dodge_roll_cooldown,
                parry_window_time: self.parry_window_time,
                ledge_momentum: self.ledge_momentum,
            },
            flight: FlightTuning {
                accel: self.flight_accel,
                drag: self.flight_drag,
                terminal_speed: self.flight_terminal_speed,
                hover_speed: self.flight_hover_speed,
                hover_hz: self.flight_hover_hz,
                direct_velocity: self.flight_direct_velocity,
            },
        }
    }
}

impl Default for AxisSweptParams {
    fn default() -> Self {
        DEFAULT_AXIS_SWEPT_PARAMS
    }
}

pub const DEFAULT_AXIS_SWEPT_PARAMS: AxisSweptParams = DEFAULT_TUNING.axis_swept_params();

pub const DEFAULT_TUNING: MovementTuning = MovementTuning {
    gravity: GRAVITY,
    run_accel: RUN_ACCEL,
    air_accel: AIR_ACCEL,
    ground_friction: GROUND_FRICTION,
    air_friction: AIR_FRICTION,
    air_stop_assist: AIR_STOP_ASSIST,
    carried_decay: 0.0,
    max_run_speed: MAX_RUN_SPEED,
    max_fall_speed: MAX_FALL_SPEED,
    jump_speed: JUMP_SPEED,
    double_jump_speed: DOUBLE_JUMP_SPEED,
    wall_jump_x: WALL_JUMP_X,
    wall_slide_speed: WALL_SLIDE_SPEED,
    wall_climb_speed: WALL_CLIMB_SPEED,
    dash_speed: DASH_SPEED,
    dash_time: DASH_TIME,
    dash_cooldown: DASH_COOLDOWN,
    dash_buffer: DASH_BUFFER,
    blink_distance: BLINK_DISTANCE,
    precision_blink_distance: PRECISION_BLINK_DISTANCE,
    precision_blink_aim_speed: PRECISION_BLINK_AIM_SPEED,
    blink_hold_threshold: BLINK_HOLD_THRESHOLD,
    blink_cooldown: BLINK_COOLDOWN,
    blink_grace_time: BLINK_GRACE_TIME,
    blink_max_downward_speed: BLINK_MAX_DOWNWARD_SPEED,
    precision_blink_max_downward_speed: PRECISION_BLINK_MAX_DOWNWARD_SPEED,
    fast_fall_accel: FAST_FALL_ACCEL,
    fast_fall_speed: FAST_FALL_SPEED,
    glide_fall_speed: GLIDE_FALL_SPEED,
    glide_air_accel: GLIDE_AIR_ACCEL,
    flight_accel: FLIGHT_ACCEL,
    flight_drag: FLIGHT_DRAG,
    flight_terminal_speed: FLIGHT_TERMINAL_SPEED,
    flight_hover_speed: FLIGHT_HOVER_SPEED,
    flight_hover_hz: FLIGHT_HOVER_HZ,
    // Smoothed accel/drag flight is the default; direct-velocity is opt-in per body.
    flight_direct_velocity: false,
    coyote_time: COYOTE_TIME,
    jump_buffer: JUMP_BUFFER,
    pogo_speed: POGO_SPEED,
    slash_recoil: SLASH_RECOIL,
    air_jumps: AIR_JUMPS,
    dodge_roll_time: DODGE_ROLL_TIME,
    dodge_roll_speed: DODGE_ROLL_SPEED,
    dodge_roll_cooldown: DODGE_ROLL_COOLDOWN,
    parry_window_time: PARRY_WINDOW_TIME,
    ledge_momentum: LedgeMomentumTuning::DEFAULT,
};
