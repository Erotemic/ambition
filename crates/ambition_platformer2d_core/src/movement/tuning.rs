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
/// resources (in `ambition_platformer2d_shared_tangle`) all resolve here so a flip
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
/// **Air dodge** — the aerial evade's invulnerable window (seconds).
///
/// Shorter than the ground roll's: the roll ends on its feet and pays a
/// cooldown, while the air dodge is spent for the whole trip through the air,
/// so its commitment is the airtime rather than the clock.
pub const AIR_DODGE_TIME: f32 = 0.20;
/// Air-dodge travel speed along the stick, px/s. Below the roll's, because the
/// air dodge may aim in any direction — including straight down, where the
/// roll's 530 would read as a dive.
pub const AIR_DODGE_SPEED: f32 = 440.0;
/// Endlag after the air-dodge window closes: airborne, controllable, but
/// evading nothing. This is the punish window that makes the option a choice.
pub const AIR_DODGE_ENDLAG: f32 = 0.16;
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MovementTuning {
    /// Authored gravity RESPONSE magnitude (px/s²) — an input the environment's
    /// frame resolver composes with the live gravity direction and any per-body
    /// response scale. Not a policy parameter.
    pub gravity: f32,
    /// Selects the reusable horizontal controller used by AxisSwept.
    #[serde(default)]
    pub horizontal_law: AxisHorizontalLaw,
    /// Selects the reusable jump-arc controller used by AxisSwept.
    #[serde(default)]
    pub jump_law: AxisJumpLaw,
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
    /// **Top horizontal speed while AIRBORNE.** `0.0` = inherit
    /// [`Self::max_run_speed`], which is what every body did before this field
    /// existed and what every body still does until it authors otherwise.
    ///
    /// ⛔ **air ACCELERATION was authored and air TOP SPEED was not**, so a
    /// body's ground run cap governed its drift — the accidental reuse of
    /// ground locomotion the combat campaign names. In a platform fighter air
    /// speed is a per-character stat and a slow-running heavy can still drift
    /// fast; expressing that was impossible.
    ///
    /// ⚠ **the sentinel is deliberate, not laziness.** `Option<f32>` would cost
    /// a bool in the motion codec's frozen wire layout for a value whose
    /// "unset" case is exactly "the other number"; `0.0` is not a meaningful
    /// air speed (a body that cannot drift at all authors its `air_accel` to
    /// zero), so it is free to mean inherit. Read it through
    /// [`Self::air_speed_cap`], never raw.
    #[serde(default)]
    pub max_air_speed: f32,
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
    /// flight unchanged. The one bound a direct command still answers to is
    /// [`FlightTuning::coordinate_speed_cap`], which is `flight_terminal_speed`
    /// itself unless an invariant speed lowers it.
    #[serde(default)]
    pub flight_direct_velocity: bool,
    /// Optional invariant speed for proper-velocity flight. When present, the
    /// flight limb accelerates in proper-velocity space and converts back to a
    /// coordinate velocity.
    ///
    /// The subluminal guarantee is a POSTCONDITION on the limb's output, not a
    /// property of that one control policy: whichever branch produces the
    /// requested velocity — direct-velocity commands included — the result is
    /// bounded by [`FlightTuning::coordinate_speed_cap`], which is the authored
    /// terminal bounded strictly below this value. Authoring a terminal at or
    /// above the invariant speed therefore yields a subluminal body rather than a
    /// broken guarantee.
    #[serde(default)]
    pub flight_invariant_speed: Option<f32>,
    pub coyote_time: f32,
    pub jump_buffer: f32,
    /// Grounded startup a jump owes before the body leaves the floor —
    /// "jump-squat" in platform-fighter vocabulary. `0.0` (the default) means
    /// the leap happens on the press tick, which is what every classic
    /// platformer does and what Mary-O's SMB1 convergence requires.
    ///
    /// ⭐ this is the number that makes a jump COMMITTAL. A body with a squat
    /// can be struck out of its own takeoff, and its opponent can react to the
    /// crouch; a body without one cannot be. It is authored per body precisely
    /// because those are different games, not two settings of one game.
    #[serde(default)]
    pub jump_squat_time: f32,
    pub pogo_speed: f32,
    pub slash_recoil: f32,
    pub air_jumps: u8,
    pub dodge_roll_time: f32,
    pub dodge_roll_speed: f32,
    pub dodge_roll_cooldown: f32,
    /// **The aerial evade**: how long the i-frames last, how fast the body
    /// travels along the stick, and the endlag it owes on the far side.
    ///
    /// `#[serde(default)]` so tuning files baked before the air dodge existed
    /// keep parsing; a zero `air_dodge_time` means this body has no air dodge,
    /// which is the state every body was in.
    #[serde(default)]
    pub air_dodge_time: f32,
    #[serde(default)]
    pub air_dodge_speed: f32,
    #[serde(default)]
    pub air_dodge_endlag: f32,
    /// **The launch speed at which a hit sends this body into TUMBLE**, px/s.
    /// `0.0` (the default) = this body never tumbles and never gets knocked
    /// down, which is every body until one authors a fighter's floor game.
    #[serde(default)]
    pub tumble_speed: f32,
    /// See [`ShieldTuning`].
    #[serde(default)]
    pub shield: ShieldTuning,
    /// See [`FootstoolTuning`].
    #[serde(default)]
    pub footstool: FootstoolTuning,
    pub parry_window_time: f32,
    /// Momentum-carry parameters for ledge getups. Set to
    /// `LedgeMomentumTuning::OFF` to disable the mechanic.
    ///
    /// `#[serde(default)]` so any tuning files serialized before this
    /// field existed (e.g. `assets/ambition/platformer_defaults.ron` baked at
    /// boot) deserialize with `LedgeMomentumTuning::DEFAULT` instead
    /// of panicking on `MissingStructField`.
    #[serde(default)]
    pub ledge_momentum: LedgeMomentumTuning,
}

impl AxisLocomotion {
    /// **The top horizontal speed this body may reach in the AIR.**
    ///
    /// The one reader of [`Self::max_air_speed`]. Every airborne speed target
    /// goes through here so the inherit sentinel cannot be forgotten at one
    /// call site and honoured at another — which is how a body would drift at
    /// its run speed on one law and its air speed on the other.
    pub fn air_speed_cap(&self) -> f32 {
        if self.max_air_speed > 0.0 {
            self.max_air_speed
        } else {
            self.max_run_speed
        }
    }
}

/// Horizontal response law used by the axis-swept policy.
///
/// `Responsive` is Ambition's tight target-velocity controller. `Momentum`
/// preserves neutral airborne speed and gives acceleration, reversal, and coast
/// distinct rates. Both operate on the scalar velocity along the resolved
/// frame's side axis; neither owns a world direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum AxisHorizontalLaw {
    #[default]
    Responsive,
    Momentum(MomentumHorizontalTuning),
}

/// Additional rates needed by a momentum-preserving horizontal law.
///
/// Forward acceleration and speed caps remain the shared `run_accel`,
/// `air_accel`, and `max_run_speed` fields on [`AxisLocomotion`]. These values
/// define only the cases the responsive law historically conflated.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MomentumHorizontalTuning {
    pub ground_reverse_accel: f32,
    pub ground_coast_decel: f32,
    pub air_reverse_accel: f32,
    pub air_coast_decel: f32,
}

impl Default for MomentumHorizontalTuning {
    fn default() -> Self {
        Self {
            ground_reverse_accel: RUN_ACCEL,
            ground_coast_decel: GROUND_FRICTION,
            air_reverse_accel: AIR_ACCEL,
            air_coast_decel: 0.0,
        }
    }
}

/// Vertical jump-arc law used by the axis-swept policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum AxisJumpLaw {
    /// Historical Ambition behavior: one launch speed, with early release
    /// directly scaling the current upward velocity.
    #[default]
    VelocityCut,
    /// Speed-banded launch plus weak held-ascent gravity and stronger
    /// release/fall gravity.
    PhasedGravity(PhasedGravityJumpTuning),
}

/// Parameters for a classic phased-gravity jump.
///
/// Thresholds and launch speeds are expressed in body-local side speed and
/// world units per second. Gravity values are multipliers on the frame's
/// gravity contribution only; external force-zone acceleration is unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhasedGravityJumpTuning {
    /// Ascending body-local side-speed cuts between the four launch bands. Band
    /// `i` is selected by the first threshold the speed falls under; a speed at
    /// or above the last threshold takes the top band. Authors are expected to
    /// place the last cut BELOW `max_run_speed` when the top band is meant to be
    /// reachable by running (see [`Self::top_band_speed`]).
    pub speed_thresholds: [f32; 3],
    /// Per-band launch OFFSETS applied to [`AxisLocomotion::jump_speed`].
    ///
    /// Deliberately offsets rather than absolute speeds: `jump_speed` stays THE
    /// one ground-jump height knob for every law, so retuning it moves the whole
    /// band family together and a character cannot end up with two disagreeing
    /// launch authorities. `[0.0; 4]` is a flat arc that still latches bands.
    pub launch_offsets: [f32; 4],
    pub held_rise_gravity_scale: f32,
    pub released_rise_gravity_scale: f32,
    pub fall_gravity_scale: f32,
    pub held_phase_min_upward_speed: f32,
}

impl Default for PhasedGravityJumpTuning {
    fn default() -> Self {
        Self {
            speed_thresholds: [f32::INFINITY; 3],
            launch_offsets: [0.0; 4],
            held_rise_gravity_scale: 1.0,
            released_rise_gravity_scale: 1.0,
            fall_gravity_scale: 1.0,
            held_phase_min_upward_speed: 0.0,
        }
    }
}

impl PhasedGravityJumpTuning {
    /// Select the takeoff band from absolute body-local side speed.
    pub fn band_for_side_speed(self, side_speed: f32) -> u8 {
        let speed = side_speed.abs();
        self.speed_thresholds
            .iter()
            .position(|threshold| speed < *threshold)
            .unwrap_or(3) as u8
    }

    /// The launch speed for `band`, resolved against the body's base
    /// `jump_speed`. Clamped at zero so a hostile offset cannot invert the jump.
    pub fn launch_speed_for_band(self, base_speed: f32, band: u8) -> f32 {
        (base_speed + self.launch_offsets[usize::from(band.min(3))]).max(0.0)
    }

    /// The lowest side speed that selects the TOP band. An authored profile
    /// whose `max_run_speed` is below this reserves the top band for externally
    /// supplied overspeed (a fling, a conveyor, knockback) rather than running.
    /// Exposed so a character can assert which of the two it meant.
    pub fn top_band_speed(self) -> f32 {
        self.speed_thresholds[2]
    }
}

impl AxisJumpLaw {
    /// Resolve a ground-jump launch from the body's base `jump_speed`. The
    /// optional band is latched by the phased-gravity runtime state.
    pub fn ground_launch(self, base_speed: f32, side_speed: f32) -> (f32, Option<u8>) {
        match self {
            Self::VelocityCut => (base_speed, None),
            Self::PhasedGravity(params) => {
                let band = params.band_for_side_speed(side_speed);
                (params.launch_speed_for_band(base_speed, band), Some(band))
            }
        }
    }
}

/// The axis-swept LOCOMOTION law: ground/air run, jumps, walls, falling.
///
/// These parameters define how the body moves; ability verbs and the flight
/// limb are separate groups. No field here may describe the live environment.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AxisLocomotion {
    pub horizontal_law: AxisHorizontalLaw,
    pub jump_law: AxisJumpLaw,
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
    /// Top horizontal speed while AIRBORNE; `0.0` inherits
    /// [`Self::max_run_speed`]. See the twin on `MovementTuning` for why the
    /// sentinel rather than an `Option`, and read it through
    /// [`Self::air_speed_cap`].
    pub max_air_speed: f32,
    pub max_fall_speed: f32,
    pub jump_speed: f32,
    pub double_jump_speed: f32,
    pub wall_jump_x: f32,
    pub wall_slide_speed: f32,
    pub wall_climb_speed: f32,
    pub coyote_time: f32,
    pub jump_buffer: f32,
    /// See [`MovementTuning::jump_squat_time`]. `0.0` = the leap is instant.
    pub jump_squat_time: f32,
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
    /// See [`AbilityTuning::air_dodge_time`].
    #[serde(default)]
    pub air_dodge_time: f32,
    #[serde(default)]
    pub air_dodge_speed: f32,
    #[serde(default)]
    pub air_dodge_endlag: f32,
    /// See [`TraversalAbilityTuning::tumble_speed`].
    #[serde(default)]
    pub tumble_speed: f32,
    /// See [`ShieldTuning`].
    #[serde(default)]
    pub shield: ShieldTuning,
    /// See [`FootstoolTuning`].
    #[serde(default)]
    pub footstool: FootstoolTuning,
    pub parry_window_time: f32,
    #[serde(default)]
    pub ledge_momentum: LedgeMomentumTuning,
}

/// **The shield as a RESOURCE** — integrity that drains while held, regenerates
/// while down, is spent by blocked hits, and breaks the guard when exhausted.
///
/// Set [`Self::max_health`] to `0.0` (the default) to leave a body's shield an
/// unlimited on/off guard, which is what every body had before this existed.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShieldTuning {
    /// Total integrity. `0.0` = this body's shield is not a resource.
    pub max_health: f32,
    /// Integrity lost per second while the guard is up.
    pub drain_per_second: f32,
    /// Integrity recovered per second while the guard is down and unbroken.
    pub regen_per_second: f32,
    /// Integrity spent per point of damage the guard absorbs.
    pub damage_scale: f32,
    /// Seconds the body is dizzy and shieldless after a break.
    pub break_stun_time: f32,
    /// Seconds of shieldstun the defender owes per point of damage it blocks.
    /// `0.0` makes blocking free, which is what it was.
    pub stun_per_damage: f32,
    /// Lateral push (px/s) the defender takes per point of damage it blocks.
    /// The half of shield pressure that costs SPACE rather than tempo: hold a
    /// guard near a ledge and the hits themselves move you toward it.
    pub pushback_per_damage: f32,
}

impl Default for ShieldTuning {
    fn default() -> Self {
        Self::OFF
    }
}

impl ShieldTuning {
    /// An unlimited guard: no drain, no break, no regeneration to do.
    pub const OFF: Self = Self {
        max_health: 0.0,
        drain_per_second: 0.0,
        regen_per_second: 0.0,
        damage_scale: 0.0,
        break_stun_time: 0.0,
        stun_per_damage: 0.0,
        pushback_per_damage: 0.0,
    };

    /// Platform-fighter defaults: a guard that survives about six seconds held,
    /// refills in about eight, and costs a point of integrity per point blocked.
    pub const PLATFORM_FIGHTER: Self = Self {
        max_health: 50.0,
        drain_per_second: 8.0,
        regen_per_second: 6.0,
        damage_scale: 1.0,
        break_stun_time: 2.0,
        stun_per_damage: 0.012,
        pushback_per_damage: 6.0,
    };

    /// Whether this body's shield is a spendable resource at all.
    pub fn is_resource(self) -> bool {
        self.max_health > 0.0
    }
}

/// **THE FOOTSTOOL** — jumping off another body's head.
///
/// Set [`Self::rise_speed`] to `0.0` (the default) and no body can be stood on,
/// which is what every body in the game had.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FootstoolTuning {
    /// Upward speed the stomper takes off the head (px/s). `0.0` = no footstool.
    pub rise_speed: f32,
    /// Downward speed the stomped body is driven at (px/s). This is the half
    /// that makes a footstool a KILL move near a blast floor rather than a
    /// mobility trick.
    pub press_speed: f32,
    /// Seconds the stomped body has no control authority.
    pub victim_stun: f32,
    /// Penetration tolerance for "feet on its head" (px). See
    /// [`crate::collision_semantics::feet_on_head`] — reach, not hover.
    pub band: f32,
}

impl Default for FootstoolTuning {
    fn default() -> Self {
        Self::OFF
    }
}

impl FootstoolTuning {
    /// Nobody can be stood on.
    pub const OFF: Self = Self {
        rise_speed: 0.0,
        press_speed: 0.0,
        victim_stun: 0.0,
        band: 0.0,
    };

    /// Platform-fighter defaults: a hop a touch under a full jump, a shove that
    /// costs the stomped body its airspace, and a stun short enough to recover
    /// from over the stage and fatal off it.
    pub const PLATFORM_FIGHTER: Self = Self {
        rise_speed: 330.0,
        press_speed: 220.0,
        victim_stun: 0.28,
        band: 14.0,
    };

    /// Whether any body may be stood on under this tuning.
    pub fn is_enabled(self) -> bool {
        self.rise_speed > 0.0
    }
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
    /// See [`MovementTuning::flight_invariant_speed`].
    #[serde(default)]
    pub invariant_speed: Option<f32>,
}

impl FlightTuning {
    /// The coordinate-speed bound the flight limb enforces on its OUTPUT, for
    /// every control policy.
    ///
    /// `terminal_speed` is the authored game-feel knob. When an `invariant_speed`
    /// is also authored it is a physical law rather than a preference, so the cap
    /// is additionally held strictly below `c`. Reading this instead of the raw
    /// terminal is what makes "an invariant speed is never reached" a property of
    /// the limb rather than of the one branch that integrates in proper velocity —
    /// a direct-velocity command and any future control policy pass through the
    /// same bound.
    pub fn coordinate_speed_cap(self) -> f32 {
        let terminal = self.terminal_speed.abs();
        match self.invariant_speed {
            // Strictly below `c`: a body that exactly reaches the invariant speed
            // has an infinite Lorentz factor, so the margin is part of the law.
            Some(c) => terminal.min(c.abs().max(f32::EPSILON) * (1.0 - 1.0e-5)),
            None => terminal,
        }
    }
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
                horizontal_law: self.horizontal_law,
                jump_law: self.jump_law,
                run_accel: self.run_accel,
                air_accel: self.air_accel,
                ground_friction: self.ground_friction,
                air_friction: self.air_friction,
                air_stop_assist: self.air_stop_assist,
                carried_decay: self.carried_decay,
                max_run_speed: self.max_run_speed,
                max_air_speed: self.max_air_speed,
                max_fall_speed: self.max_fall_speed,
                jump_speed: self.jump_speed,
                double_jump_speed: self.double_jump_speed,
                wall_jump_x: self.wall_jump_x,
                wall_slide_speed: self.wall_slide_speed,
                wall_climb_speed: self.wall_climb_speed,
                coyote_time: self.coyote_time,
                jump_buffer: self.jump_buffer,
                jump_squat_time: self.jump_squat_time,
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
                air_dodge_time: self.air_dodge_time,
                air_dodge_speed: self.air_dodge_speed,
                air_dodge_endlag: self.air_dodge_endlag,
                tumble_speed: self.tumble_speed,
                parry_window_time: self.parry_window_time,
                shield: self.shield,
                footstool: self.footstool,
                ledge_momentum: self.ledge_momentum,
            },
            flight: FlightTuning {
                accel: self.flight_accel,
                drag: self.flight_drag,
                terminal_speed: self.flight_terminal_speed,
                hover_speed: self.flight_hover_speed,
                hover_hz: self.flight_hover_hz,
                direct_velocity: self.flight_direct_velocity,
                invariant_speed: self.flight_invariant_speed,
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
    horizontal_law: AxisHorizontalLaw::Responsive,
    jump_law: AxisJumpLaw::VelocityCut,
    run_accel: RUN_ACCEL,
    air_accel: AIR_ACCEL,
    ground_friction: GROUND_FRICTION,
    air_friction: AIR_FRICTION,
    air_stop_assist: AIR_STOP_ASSIST,
    carried_decay: 0.0,
    max_run_speed: MAX_RUN_SPEED,
    // Inherit the ground cap: every body drifts at its run speed until one
    // authors otherwise, which is byte-parity with before this field existed.
    max_air_speed: 0.0,
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
    flight_invariant_speed: None,
    coyote_time: COYOTE_TIME,
    jump_buffer: JUMP_BUFFER,
    // No squat by default: an unauthored body leaps on the press tick.
    jump_squat_time: 0.0,
    pogo_speed: POGO_SPEED,
    slash_recoil: SLASH_RECOIL,
    air_jumps: AIR_JUMPS,
    dodge_roll_time: DODGE_ROLL_TIME,
    dodge_roll_speed: DODGE_ROLL_SPEED,
    dodge_roll_cooldown: DODGE_ROLL_COOLDOWN,
    // ⛔ **ZERO in the default tuning, and that is the decision, not an
    // oversight.** An airborne dash press already MEANS something for a body
    // with the dash ability — it is the protagonist's air dash, a traversal
    // move — and a default-on air dodge would quietly take that press away from
    // every exploration body in the game. The maneuver is body-generic in the
    // kernel and AUTHORED per character, exactly like the shield, the ledge and
    // the moveset: a fighter says `air_dodge_time: AIR_DODGE_TIME` and gets one.
    air_dodge_time: 0.0,
    air_dodge_speed: AIR_DODGE_SPEED,
    air_dodge_endlag: AIR_DODGE_ENDLAG,
    // ⛔ zero for the same reason the air dodge is: a wandering enemy that got
    // knocked down and had to stand up would be a different game.
    tumble_speed: 0.0,
    parry_window_time: PARRY_WINDOW_TIME,
    shield: ShieldTuning::OFF,
    footstool: FootstoolTuning::OFF,
    ledge_momentum: LedgeMomentumTuning::DEFAULT,
};

#[cfg(test)]
mod air_speed_tests {
    use super::*;

    /// ⭐⭐ **Air acceleration was authored and air TOP SPEED was not**, so a
    /// body's ground run cap governed its drift — accidental reuse of ground
    /// locomotion, and the reason a slow-running heavy that drifts fast could
    /// not be expressed.
    ///
    /// ⛔ **the poison is the inherit case**, and it is the one that matters:
    /// every body in the game authors nothing here, so an accessor that failed
    /// to fall back would silently pin the whole cast to zero air speed.
    #[test]
    fn air_speed_is_authored_and_inherits_the_ground_cap_when_it_is_not() {
        let mut locomotion = DEFAULT_AXIS_SWEPT_PARAMS.locomotion;
        locomotion.max_run_speed = 180.0;

        locomotion.max_air_speed = 0.0;
        assert_eq!(
            locomotion.air_speed_cap(),
            180.0,
            "an unauthored body drifts at its run speed, exactly as before"
        );

        locomotion.max_air_speed = 96.0;
        assert_eq!(
            locomotion.air_speed_cap(),
            96.0,
            "a floatier heavy drifts slower"
        );

        locomotion.max_air_speed = 320.0;
        assert_eq!(
            locomotion.air_speed_cap(),
            320.0,
            "and a glass cannon may drift FASTER than it runs — the case a \
             single shared cap makes unspellable"
        );
    }
}
