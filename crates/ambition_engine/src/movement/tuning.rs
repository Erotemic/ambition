use serde::{Deserialize, Serialize};

// First-pass movement constants. These remain constants for easy grep/tuning,
// but the simulation accepts a `MovementTuning` so experiments can override
// them without recompiling every assumption into the update function.
pub const GRAVITY: f32 = 2250.0;
pub const RUN_ACCEL: f32 = 5200.0;
pub const AIR_ACCEL: f32 = 3100.0;
pub const GROUND_FRICTION: f32 = 7600.0;
pub const AIR_FRICTION: f32 = 650.0;
pub const MAX_RUN_SPEED: f32 = 270.0;
pub const MAX_FALL_SPEED: f32 = 950.0;
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
pub const FAST_FALL_SPEED: f32 = 1380.0;
/// Glide / slow-fall vertical cap. Roughly 1/5 of `MAX_FALL_SPEED` so
/// the held-jump glide feels distinctly hover-y without becoming
/// effectively-flying. Pair with `MovementTuning::glide_air_accel` for
/// the increased horizontal authority while gliding.
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
/// - x_gain = 0.65 carries about two-thirds of incoming run speed.
/// - y_gain = 0.45 keeps recovery boosts noticeable without
///   catapulting double-jumped recoveries.
/// - Caps pin the boost so an extreme dash → ledge approach doesn't
///   exit at dash speed; ~jump_speed feels like the right ceiling.
pub const LEDGE_BOOST_WINDOW: f32 = 0.25;
pub const LEDGE_BOOST_X_GAIN: f32 = 0.65;
pub const LEDGE_BOOST_Y_GAIN: f32 = 0.45;
pub const LEDGE_BOOST_X_CAP: f32 = 320.0;
pub const LEDGE_BOOST_Y_CAP: f32 = 540.0;

/// Tunable momentum-carry parameters for ledge getups.
///
/// When the player grabs a ledge with non-trivial momentum and then
/// commits to a getup option (climb / roll / attack / jump) within
/// the boost window, the carried-over velocity is folded into the
/// getup so the player exits with a leftover horizontal/vertical
/// kick. The drop / outward-release options never get the boost —
/// those are deliberate disengage actions.
///
/// Set [`window`] to `0.0` to disable the mechanic entirely.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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
    };
}

/// Tunable movement parameters.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MovementTuning {
    pub gravity: f32,
    pub run_accel: f32,
    pub air_accel: f32,
    pub ground_friction: f32,
    pub air_friction: f32,
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
    /// Vertical fall speed cap while `Player::gliding` is true. See
    /// [`GLIDE_FALL_SPEED`].
    pub glide_fall_speed: f32,
    /// Horizontal acceleration applied while `Player::gliding` is
    /// true, replacing `air_accel`. See [`GLIDE_AIR_ACCEL`].
    pub glide_air_accel: f32,
    pub flight_accel: f32,
    pub flight_drag: f32,
    pub flight_terminal_speed: f32,
    pub flight_hover_speed: f32,
    pub flight_hover_hz: f32,
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

impl Default for MovementTuning {
    fn default() -> Self {
        DEFAULT_TUNING
    }
}

pub const DEFAULT_TUNING: MovementTuning = MovementTuning {
    gravity: GRAVITY,
    run_accel: RUN_ACCEL,
    air_accel: AIR_ACCEL,
    ground_friction: GROUND_FRICTION,
    air_friction: AIR_FRICTION,
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
