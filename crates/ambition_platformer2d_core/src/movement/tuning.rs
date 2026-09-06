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

/// Where a walk becomes a RUN, as a fraction of the body's own top speed.
///
/// Above a half separates a held walk (analog stick part-way, settling near its
/// own lower target) from a committed run, and the genre's running attack is the
/// first thing that had to know the difference.
pub const RUN_COMMIT_FRAC: f32 = 0.55;

/// Serde default for [`LocomotionTuning::crouch_speed_frac`]: FREE, which is
/// exactly what every body did before the field existed.
fn default_crouch_speed_frac() -> f32 {
    1.0
}

fn default_run_commit_frac() -> f32 {
    RUN_COMMIT_FRAC
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
/// Spot dodge — the grounded evade IN PLACE, held down instead of a
/// direction. Shorter than the roll's window because it covers no distance: the
/// roll's commitment is where it takes you, and the spot dodge's is only the
/// time, so a spot dodge that lasted as long would be strictly better.
pub const SPOT_DODGE_TIME: f32 = 0.16;
/// How far down the stick must be held for a grounded evade to read as a spot
/// dodge rather than a roll. Above the roll's own `0.1` sideways threshold so a
/// diagonal reads as the roll it looks like.
pub const SPOT_DODGE_STICK: f32 = 0.5;
/// Air dodge — the aerial evade's invulnerable window (seconds).
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
/// Endlag after a GROUND ROLL comes to rest: standing, controllable, evading
/// nothing — the same punish window as the air dodge's above, and shorter
/// because a roll achieves less. ~5 frames at 60Hz.
pub const DODGE_ROLL_ENDLAG: f32 = 0.08;

/// `#[serde(default)]` for a fraction whose absent value is "unchanged".
fn one_f32() -> f32 {
    1.0
}
/// Parry window: full invulnerability during the first moments of shield activation.
pub const PARRY_WINDOW_TIME: f32 = 0.15;

/// WHEN THE PERFECT-SHIELD WINDOW OPENS — and the two settings are two
/// GAMES, not two candidates.
///
/// ```text
/// OnRaise    Smash 4's, and ours since the parry existed
/// OnRelease  Ultimate's — it moved the perfect shield off the press
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParryTiming {
    /// The default, so no existing body's feel changes.
    #[default]
    OnRaise,
    /// The window opens when the guard COMES DOWN. Makes the shield a
    /// two-decision object — you raise it under pressure and then time the drop
    /// — so a defender inside a multi-hit string is choosing every beat rather
    /// than once at the start. Ultimate's stated reason for moving it.
    OnRelease,
}

/// Ledge momentum-carry defaults. See [`LedgeMomentumTuning`] for per-field semantics.
/// Moving into a ledge grab can boost a quick getup; a stationary grab does not:
/// - 250 ms window matches the existing regrab cooldown so the "fresh grab" feel window is symmetric.
/// - x_gain = 0.85 carries most of the incoming run speed; the previous 0.65 left too little kick once the cap clipped a typical 270 px/s approach down to ~175 px/s.
/// - y_gain = 0.45 — only meaningful for ledge-jump (vertical hop); climb / roll / attack finish zero this out entirely so they don't launch the player off the platform they just landed on.
/// - Caps pin the boost so an extreme dash → ledge approach doesn't exit at dash speed; ~jump_speed feels like the right ceiling.
/// - getup_speedup_gain shortens the climb/roll/attack transition when momentum was carried, so the animation itself feels continuous instead of "stop and go." 1.0 = full momentum roughly halves the transition; 0.0 disables the speedup.
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
/// Neutral by construction. Content hydrates it from authored data; a developer build lets the
/// F3 inspector edit it through `ambition_dev_tools`'s adapter.
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
    /// Air top speed is independent of ground run speed so per-character drift
    /// can differ from ground locomotion.
    ///
    /// the sentinel is deliberate, not laziness. `Option<f32>` would cost
    /// a bool in the motion codec's frozen wire layout for a value whose
    /// "unset" case is exactly "the other number"; `0.0` is not a meaningful
    /// air speed (a body that cannot drift at all authors its `air_accel` to
    /// zero), so it is free to mean inherit. Read it through
    /// [`Self::air_speed_cap`], never raw.
    #[serde(default)]
    pub max_air_speed: f32,
    /// The gait line: what fraction of [`Self::max_run_speed`] counts as a
    /// RUN. Published every tick as `BodyMotionFacts::running`, and read by
    /// the move selector so an Attack press while running is the running attack.
    ///
    /// this is NOT the traversal dash. `AbilitySet::dash` is a discrete
    /// charge-gated burst that REPLACES the velocity vector; a platform
    /// fighter's dash attack comes out of ordinary grounded locomotion and a
    /// fighter kit that switches the burst off still has one.
    ///
    /// a body that wants no gait distinction authors `1.0` and reaches a run
    /// only at full tilt; `0.0` would call a standstill a run.
    #[serde(default = "default_run_commit_frac")]
    pub run_commit_frac: f32,
    /// What a CROUCHING body may do with the stick, as a fraction of its top
    /// speed. `0.0` = a crouch plants you; `1.0` = crouching costs nothing.
    ///
    /// ⭐⭐ THE GENRE'S ANSWER IS "NOT MUCH", and it is research rather than a
    /// feel call: in every Smash, crouching STOPS you outright unless the
    /// character has a crawl, and a crawl is a slow shuffle. The trade is the
    /// point — a crouch shrinks your hurtbox and shortens a launch
    /// (`crouch_cancel_scale`), and what pays for that is your mobility.
    ///
    /// ⛔⛔ IT DEFAULTED TO FREE, and nothing said so. Measured 2026-08-24: the
    /// movement kernel read `BodyMode` only for `Climbing`, so `Crouching` never
    /// reached the locomotion law and a crouching fighter ran at full speed
    /// while keeping both defensive benefits. `1.0` is kept as the ENGINE
    /// default so no Ambition room changes; a platform fighter declares its own.
    #[serde(default = "default_crouch_speed_frac")]
    pub crouch_speed_frac: f32,
    /// THE INITIAL DASH — how long the first phase of a ground move lasts, in
    /// seconds. `0.0` (the default) is what every body in this engine does:
    /// ground speed is one continuum and there is no phase at all.
    ///
    /// ⭐⭐ IT IS THE WINDOW IN WHICH A BODY MAY STILL CHANGE ITS MIND. A fresh
    /// direction sets the body moving at `initial_dash_speed` AT ONCE rather
    /// than accelerating into it, and a direction CHANGE restarts the phase —
    /// which is dash-dancing, and it falls out of the same rule rather than
    /// being a second mechanic. Once the phase ends the ordinary run law
    /// resumes from whatever speed the dash reached, so a held direction flows
    /// into a run without a seam.
    ///
    /// ⛔ THIS IS NOT `AbilitySet::dash`. That one is a charge-gated traversal
    /// burst that REPLACES the velocity vector and is spent; this is the first
    /// phase of ordinary walking, it costs nothing, and a body whose burst is
    /// switched off still has it.
    ///
    /// ⛔⛔ HIGHEST BLAST RADIUS IN THE MOVEMENT KERNEL — ground locomotion is
    /// what every game in this repo walks on. `0.0` keeps every existing world
    /// byte-identical, and a platform fighter declares its own.
    #[serde(default)]
    pub initial_dash_time: f32,
    /// How fast the initial dash moves, in engine units per second. `0.0`
    /// inherits `max_run_speed`, which is the sensible default: the phase is
    /// about WHEN you may turn around, not about being faster.
    #[serde(default)]
    pub initial_dash_speed: f32,
    /// THE TURNAROUND — how long a body reversing out of a COMMITTED RUN takes
    /// to actually change which way it faces. `0.0` (the default) flips facing
    /// the instant the stick does, which is what every body in this engine did.
    ///
    /// ⭐⭐ IT IS WHAT MAKES THE INITIAL DASH'S FREE REVERSAL MEAN ANYTHING.
    /// Reversing inside the dash window costs nothing and reversing out of a
    /// run costs this — the two together are the genre's ground game, and
    /// either one alone is just a speed.
    ///
    /// ⛔ ONLY OUT OF A RUN. A body that has not committed
    /// (`BodyMotionFacts::running`) is still in its dash and turns for free;
    /// charging the phase there would delete dash-dancing.
    ///
    /// ⛔ IT DELAYS THE FACING FLIP, it does not invent a skid. What the body
    /// does with its velocity meanwhile is the ordinary run law's business.
    #[serde(default)]
    pub turnaround_time: f32,
    /// THE TEETER — how much of a body's own width counts as its LEADING FOOT,
    /// as a fraction. A body is on the brink when it is supported but that foot
    /// is over air. `0.0` (the default) means no body ever teeters, which is
    /// what every world in this repo did.
    ///
    /// ⛔ A FRACTION OF THE FOOTPRINT, not a lean distance, and the difference
    /// is measurable: support is decided by ANY lateral overlap, so a body
    /// hanging 14px past a platform with 15px of half-width is still fully
    /// supported. Shifting the whole body sideways therefore finds no edge —
    /// only asking about the outermost slice does.
    ///
    /// ⭐ A FACT, NOT A RULE: it publishes `BodyMotionFacts::teetering` and
    /// changes no collision, no speed and no refusal. Control and animation
    /// read it; the genre draws a wobble and gives the edge a moment of
    /// meaning, which is what makes standing there a decision.
    #[serde(default)]
    pub teeter_margin: f32,
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
    /// this is the number that makes a jump COMMITTAL. A body with a squat
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
    /// Recovery on the far side of a GROUND ROLL (seconds) — the same shape as
    /// [`air_dodge_endlag`](Self::air_dodge_endlag), so the roll is a read
    /// rather than a free reposition. The roll also comes to REST when its
    /// window closes; before both, it kept its speed and stayed safe.
    ///
    /// `#[serde(default)]` — `0.0` is every body authored before this existed,
    /// and it is the honest value for one whose roll is not a fighting option.
    #[serde(default)]
    pub dodge_roll_endlag: f32,
    /// DODGE STALING — how much of its invulnerable window an evade loses per
    /// recent evade, as a fraction. `0.0` (the baseline) is no staling at all,
    /// which is what an exploration body wants: its roll is traversal.
    ///
    /// ⭐ IT WEARS THE I-FRAMES, not the distance or the recovery. The option a
    /// spammed roll is abusing is INVULNERABILITY, so that is what a fighter who
    /// keeps rolling should have less of — a roll that still travelled and still
    /// recovered but stopped being safe is the read the genre wants.
    #[serde(default)]
    pub dodge_stale_step: f32,
    /// The floor [`Self::dodge_stale_step`] cannot take an evade below, as a
    /// fraction of its authored window. `1.0` = staling can never weaken one.
    ///
    /// ⛔ NEVER ZERO IN PRACTICE: an evade with no i-frames at all is not a
    /// worse option, it is a different (and useless) one, and a player cannot
    /// read the difference between "stale" and "broken".
    #[serde(default = "one_f32")]
    pub dodge_stale_floor: f32,
    /// Seconds of body time before one recent evade is forgiven. `0.0` with a
    /// nonzero step means the count never comes down, which is a trap rather
    /// than a mechanic.
    #[serde(default)]
    pub dodge_stale_recovery: f32,
    /// UNTECHABLE LAUNCH — the launch speed at or above which a tumble cannot be
    /// teched out of, engine units/s. `0.0` (the baseline) means every launch is
    /// techable, which is what every body did before the rule existed.
    ///
    /// ⭐ THE GENRE'S RULE: a hit hard enough to kill should not be survivable by
    /// a well-timed press on the wall behind you. It is a rules knob rather than
    /// a per-move flag because it is a property of how HARD the launch was, not
    /// of which move threw it.
    #[serde(default)]
    pub untechable_launch_speed: f32,
    /// EVADE CANCEL TAIL — the last N seconds of an evade during which a MOVE
    /// may start. `0.0` (the baseline) DISABLES the rule entirely: an attack
    /// cancels an evade on its first frame, which is what every body does today.
    ///
    /// ⭐⭐ THE TAIL, NOT A LOCKOUT FROM THE START, and the difference is dodge
    /// staling. A lockout measured from the start needs the evade's TOTAL length
    /// to know where it ends — and staling now SHORTENS that window per body, so
    /// the authored constant is the wrong total. Measuring from the END needs
    /// only the timer that is already there.
    ///
    /// ⇒ committed while `remaining > tail`; actionable once the evade has that
    /// little left. This is the genre's spot-dodge-into-attack.
    #[serde(default)]
    pub evade_cancel_tail: f32,
    /// The aerial evade: how long the i-frames last, how fast the body
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
    /// The launch speed at which a hit sends this body into TUMBLE, px/s.
    /// `0.0` (the default) = this body never tumbles and never gets knocked
    /// down, which is every body until one authors a fighter's floor game.
    #[serde(default)]
    pub tumble_speed: f32,
    /// SPOT DODGE — the grounded evade IN PLACE's invulnerable window, in
    /// seconds. `0.0` (the default) means a grounded evade is always the roll,
    /// which is what every body had before a fighter wanted the other option.
    #[serde(default)]
    pub spot_dodge_time: f32,
    /// When this body's perfect-shield window opens. See [`ParryTiming`] —
    /// the two settings are Smash 4's and Ultimate's.
    #[serde(default)]
    pub parry_timing: ParryTiming,
    /// SMASH DIRECTIONAL INFLUENCE — how far this body may shift itself per
    /// tick of HITLAG, in px. `0.0` (the default) = no SDI, which is every body
    /// until one authors a fighter.
    ///
    /// the defensive half of a mechanic whose offensive half already
    /// ships. DI ([`crate::hit_response::di_adjust`]) bends the launch you are
    /// about to take; SDI moves you out of the NEXT hit's way while the current
    /// one is still frozen, and it is what makes a combo answerable rather than
    /// a sentence.
    ///
    /// The simplification is stated rather than hidden: an edge-counting version needs
    /// per-window state inside the rollback window, and the total is bounded either way by how
    /// long the hitlag lasts.
    #[serde(default)]
    pub sdi_step: f32,
    /// AUTOMATIC SDI — one displacement per HIT, paid when the hitlag ends,
    /// in whatever direction the stick is held at that moment. `0.0` (the
    /// default) is what every body did before this existed.
    ///
    /// ⛔ DISTINCT FROM [`Self::sdi_step`], and the difference is WHAT IT IS
    /// PAID PER. SDI is paid per TICK of hitlag, so a heavy hit with a long
    /// freeze lets a defender travel far and a one-tick multihit gives them
    /// almost nothing. This is paid once per hit whatever the freeze was
    /// worth — which is exactly the case SDI cannot answer, and the reason the
    /// genre has both.
    ///
    /// ⭐ AT THE END, not the start: the defender has the whole freeze to
    /// choose a direction, and the stick they are holding when it lifts is the
    /// one that counts. Paying it at the start would just be one more SDI tick.
    #[serde(default)]
    pub asdi_step: f32,
    /// THE JAB LOCK — how weak a hit has to be to pin a body that is already
    /// PRONE instead of launching it. `0.0` (the default) disables the rule
    /// entirely: a hit on a downed body launches exactly as it always did.
    ///
    /// A launch at or below this speed, landing on a body in knockdown,
    /// re-pins it rather than throwing it, which is what makes a downed
    /// opponent a position to be read rather than a free reset. A hit ABOVE it
    /// launches normally — you cannot pin someone with a smash attack.
    ///
    /// ⛔ IT MUST BE BOUNDED, and [`Self::jab_lock_limit`] is the bound. An
    /// unbounded version is an infinite: weak hit, re-pin, weak hit, forever.
    #[serde(default)]
    pub jab_lock_speed: f32,
    /// How many times in a row a prone body may be pinned before the next hit
    /// launches it whatever its speed — the RESET half of the jab lock.
    ///
    /// `0` with a non-zero [`Self::jab_lock_speed`] would be a rule that never
    /// fires; the pair is authored together or not at all.
    #[serde(default)]
    pub jab_lock_limit: u8,
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
    /// `assets/ambition/platformer_defaults.ron` baked at boot) deserialize with
    /// `LedgeMomentumTuning::DEFAULT` instead of panicking on `MissingStructField`.
    #[serde(default)]
    pub ledge_momentum: LedgeMomentumTuning,
}

impl AxisLocomotion {
    /// The top horizontal speed this body may reach in the AIR.
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

/// Additional reversal and coasting rates for momentum-preserving movement.
/// Forward acceleration and speed caps remain on [`AxisLocomotion`].
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
    /// See [`MovementTuning::run_commit_frac`].
    pub run_commit_frac: f32,
    /// See [`MovementTuning::crouch_speed_frac`]. `1.0` = crouching is free,
    /// which is what every body did before this field existed.
    #[serde(default = "default_crouch_speed_frac")]
    pub crouch_speed_frac: f32,
    /// See [`MovementTuning::initial_dash_time`]. `0.0` = no phase, which is
    /// what every body did before this existed.
    #[serde(default)]
    pub initial_dash_time: f32,
    /// See [`MovementTuning::initial_dash_speed`]. `0.0` inherits the run speed.
    #[serde(default)]
    pub initial_dash_speed: f32,
    /// See [`MovementTuning::turnaround_time`]. `0.0` = facing flips instantly.
    #[serde(default)]
    pub turnaround_time: f32,
    /// See [`MovementTuning::teeter_margin`]. `0.0` = no body teeters.
    #[serde(default)]
    pub teeter_margin: f32,
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
    /// Recovery on the far side of a GROUND ROLL (seconds) — the same shape as
    /// [`air_dodge_endlag`](Self::air_dodge_endlag), so the roll is a read
    /// rather than a free reposition. The roll also comes to REST when its
    /// window closes; before both, it kept its speed and stayed safe.
    ///
    /// `#[serde(default)]` — `0.0` is every body authored before this existed,
    /// and it is the honest value for one whose roll is not a fighting option.
    #[serde(default)]
    pub dodge_roll_endlag: f32,
    /// DODGE STALING — how much of its invulnerable window an evade loses per
    /// recent evade, as a fraction. `0.0` (the baseline) is no staling at all,
    /// which is what an exploration body wants: its roll is traversal.
    ///
    /// ⭐ IT WEARS THE I-FRAMES, not the distance or the recovery. The option a
    /// spammed roll is abusing is INVULNERABILITY, so that is what a fighter who
    /// keeps rolling should have less of — a roll that still travelled and still
    /// recovered but stopped being safe is the read the genre wants.
    #[serde(default)]
    pub dodge_stale_step: f32,
    /// The floor [`Self::dodge_stale_step`] cannot take an evade below, as a
    /// fraction of its authored window. `1.0` = staling can never weaken one.
    ///
    /// ⛔ NEVER ZERO IN PRACTICE: an evade with no i-frames at all is not a
    /// worse option, it is a different (and useless) one, and a player cannot
    /// read the difference between "stale" and "broken".
    #[serde(default = "one_f32")]
    pub dodge_stale_floor: f32,
    /// Seconds of body time before one recent evade is forgiven. `0.0` with a
    /// nonzero step means the count never comes down, which is a trap rather
    /// than a mechanic.
    #[serde(default)]
    pub dodge_stale_recovery: f32,
    /// UNTECHABLE LAUNCH — the launch speed at or above which a tumble cannot be
    /// teched out of, engine units/s. `0.0` (the baseline) means every launch is
    /// techable, which is what every body did before the rule existed.
    ///
    /// ⭐ THE GENRE'S RULE: a hit hard enough to kill should not be survivable by
    /// a well-timed press on the wall behind you. It is a rules knob rather than
    /// a per-move flag because it is a property of how HARD the launch was, not
    /// of which move threw it.
    #[serde(default)]
    pub untechable_launch_speed: f32,
    /// EVADE CANCEL TAIL — the last N seconds of an evade during which a MOVE
    /// may start. `0.0` (the baseline) DISABLES the rule entirely: an attack
    /// cancels an evade on its first frame, which is what every body does today.
    ///
    /// ⭐⭐ THE TAIL, NOT A LOCKOUT FROM THE START, and the difference is dodge
    /// staling. A lockout measured from the start needs the evade's TOTAL length
    /// to know where it ends — and staling now SHORTENS that window per body, so
    /// the authored constant is the wrong total. Measuring from the END needs
    /// only the timer that is already there.
    ///
    /// ⇒ committed while `remaining > tail`; actionable once the evade has that
    /// little left. This is the genre's spot-dodge-into-attack.
    #[serde(default)]
    pub evade_cancel_tail: f32,
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
    /// See [`MovementTuning::spot_dodge_time`].
    #[serde(default)]
    pub spot_dodge_time: f32,
    /// See [`MovementTuning::parry_timing`].
    #[serde(default)]
    pub parry_timing: ParryTiming,
    /// See [`TraversalAbilityTuning::sdi_step`].
    #[serde(default)]
    pub sdi_step: f32,
    /// See [`TraversalAbilityTuning::asdi_step`].
    #[serde(default)]
    pub asdi_step: f32,
    /// See [`TraversalAbilityTuning::jab_lock_speed`].
    #[serde(default)]
    pub jab_lock_speed: f32,
    /// See [`TraversalAbilityTuning::jab_lock_limit`].
    #[serde(default)]
    pub jab_lock_limit: u8,
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

/// Shield integrity that drains while held, regenerates while down, and breaks
/// the guard when exhausted. `max_health == 0.0` selects an unlimited guard.
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
    /// How much of the body a SPENT guard still covers, as a fraction of
    /// its half-height, at zero integrity. `1.0` (the default) means the guard
    /// never shrinks and a body behind it is never poked; Smash's shield sinks
    /// until it exposes the head and the feet, and that is what makes chip
    /// pressure end in a hit rather than in a stalemate.
    pub min_coverage: f32,
    /// HOW FAR A HELD STICK SHIFTS THE GUARD along the body's own gravity
    /// axis, as a fraction of the body's half-height. `0.0` (the default)
    /// pins the guard to the body's centre, which is what every body did
    /// before this existed.
    ///
    /// Shield tilt is the answer to [`Self::min_coverage`], not a second copy
    /// of it: a spent guard exposes the head AND the feet, and tilting chooses
    /// WHICH of the two you are willing to lose. A hit aimed at the part you
    /// shifted toward is blocked; the opposite end is more exposed than it
    /// would have been untilted. That trade is the mechanic — without the
    /// cost it is a free coverage upgrade.
    ///
    /// ⛔ ONE AXIS, and deliberately. Coverage is measured along gravity
    /// ([`ambition_platformer2d::combat::util::guard_covers_hit`]); the lateral question is
    /// already answered by which side the body FACES, so a left/right tilt has
    /// no coverage rule to bias and would be a knob that changes nothing.
    ///
    /// ⛔ IT COMPETES WITH NOTHING. Past [`SPOT_DODGE_STICK`] the same stick
    /// is normally a roll or a spot dodge, so the band tilt gets in practice is
    /// the one that used to be dead input — but the rule itself is just the
    /// stick, so a body whose evade is spent still leans instead of going inert.
    pub tilt_range: f32,
    /// MAY THIS GUARD DROP THROUGH A ONE-WAY PLATFORM? — guard + down, the
    /// genre's platform drop.
    ///
    /// `false` (the default) is what every body did before this existed: the
    /// only way down through a one-way surface is the ordinary down+jump
    /// request, which is unchanged and still works for everyone.
    ///
    /// ⛔ AN EXPLICIT DECLARATION rather than a fallthrough on
    /// [`Self::out_of_shield`], because the two defaults point opposite ways.
    /// The out-of-shield gate reads a game with no policy as "restricts
    /// nothing", which for THIS action would silently hand a platform drop to
    /// every exploration body that has a shield and a one-way surface — waking
    /// a mechanic every one of those worlds was tuned without. Same reason
    /// [`Self::air_guard`] is not an out-of-shield action either: which game a
    /// stage reproduces is a declaration, not a permission.
    ///
    /// ⭐ WHAT IT COSTS THE PLAYER IS THE SPOT DODGE. Guard + down on SOLID
    /// ground is the spot dodge and stays so; on a one-way surface the same
    /// press drops instead. The terrain arbitrates, which is the genre's rule
    /// and the reason this needs no new gesture.
    pub platform_drop: bool,
    /// Lateral push (px/s) the defender takes per point of damage it blocks.
    /// The half of shield pressure that costs SPACE rather than tempo: hold a
    /// guard near a ledge and the hits themselves move you toward it.
    pub pushback_per_damage: f32,
    /// WHAT THIS GUARD IS A PLATFORM FOR — which action classes may start
    /// while it is raised, and `None` for a game that has no such rule.
    ///
    /// `None` is the exploration answer and the engine baseline: a raised guard
    /// restricts nothing and acting does not spend it, which is what every body
    /// did before this existed. `Some` opts into the genre's rule, and with it
    /// into its consequence — an out-of-shield action DROPS the guard, because
    /// a body that could attack from behind a shield it keeps is a body with no
    /// reason to ever lower one.
    ///
    /// ⛔ ONE policy, named in action CLASSES. The failure mode this replaces is
    /// a per-move exception list: "up-smash may, forward-smash may not" spelled
    /// move by move is a table nobody can read and everybody has to extend.
    #[serde(default)]
    pub out_of_shield: Option<OutOfShield>,
    /// May this guard be raised while the body is AIRBORNE?
    ///
    /// `true` (the default) is what every body did before this existed, and it
    /// is right for a game whose shield is a deployable bubble: Ambition's
    /// `bubble_shield` special forces the guard up for its whole duration and is
    /// not grounded-gated, so a body that could not guard in the air would lose
    /// its signature defensive move mid-jump.
    ///
    /// ⛔ A PLATFORM FIGHTER SETS THIS FALSE, and that is the genre's rule
    /// rather than a taste: no Smash title has an airborne shield — pressing it
    /// in the air is the AIR DODGE. Left true, the same press both evades and
    /// guards, which is neither.
    #[serde(default = "crate::default_true")]
    pub air_guard: bool,
    /// Seconds of hard commitment owed for LOWERING the guard by itself —
    /// shield drop lag.
    ///
    /// The other half of what makes holding a guard a decision: an out-of-shield
    /// action is fast, and simply letting go is not. `0.0` (the baseline) makes
    /// dropping free, which is what it was.
    #[serde(default)]
    pub drop_lag: f32,
}

/// Which action classes may START from a raised guard.
///
/// The list is the genre's, not a taste: every Smash title lets you jump, roll
/// or spot-dodge, grab, and rise with an up attack or up special out of shield,
/// and makes everything else wait for the guard to come down. The two "up"
/// entries are not move names — they are the classes that RISE, which is why
/// this genre lets them out of a crouched guard at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutOfShield {
    /// Jump — the universal option every other one is measured against.
    pub jump: bool,
    /// The guard's own evade: the spot dodge and the roll.
    pub burst: bool,
    /// Shield grab — the punish a blocked attack is owed.
    pub grab: bool,
    /// The UP attack, and only the up one.
    pub up_attack: bool,
    /// The UP special, for the same reason.
    pub up_special: bool,
}

impl OutOfShield {
    /// May `action` start from a guard playing by this policy?
    pub fn permits(self, action: crate::movement::abilities::OutOfShieldAction) -> bool {
        use crate::movement::abilities::OutOfShieldAction as A;
        match action {
            A::Jump => self.jump,
            A::Burst => self.burst,
            A::Grab => self.grab,
            A::UpAttack => self.up_attack,
            A::UpSpecial => self.up_special,
        }
    }

    /// The genre's list. See the type's own note for why it is these five.
    pub const PLATFORM_FIGHTER: Self = Self {
        jump: true,
        burst: true,
        grab: true,
        up_attack: true,
        up_special: true,
    };
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
        min_coverage: 1.0,
        tilt_range: 0.0,
        platform_drop: false,
        // No out-of-shield rule and no drop cost: the engine baseline, and what
        // every body did before the policy existed.
        out_of_shield: None,
        // A deployable bubble works wherever the body is. See the field.
        air_guard: true,
        drop_lag: 0.0,
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
        min_coverage: 0.45,
        // A third of a half-height. Enough that a spent guard tilted down
        // still covers the feet a poke was reaching for, and not so much that
        // the guard leaves the body.
        tilt_range: 0.34,
        // Guard + down on a soft platform drops through, in every title.
        platform_drop: true,
        // A guard is a LAUNCHING PLATFORM in this genre, and these five are what
        // it launches. Everything else waits for it to come down.
        out_of_shield: Some(OutOfShield::PLATFORM_FIGHTER),
        // ⛔ NO AIRBORNE SHIELD, which is the genre's rule in every title. The
        // same press in the air is the AIR DODGE.
        air_guard: false,
        // 11 frames, Ultimate's shield-drop. The commitment that makes holding
        // a guard a decision rather than a free stance: an out-of-shield option
        // is fast, and letting go is not.
        drop_lag: 11.0 / 60.0,
    };

    /// Whether this body's shield is a spendable resource at all.
    pub fn is_resource(self) -> bool {
        self.max_health > 0.0
    }

    /// How much of the body the guard covers at `integrity` (1.0 whole, 0.0
    /// about to break), as a fraction of its half-height. Full coverage for a
    /// guard that is not a resource, so an exploration body is never poked.
    pub fn coverage_at(self, integrity: f32) -> f32 {
        if !self.is_resource() {
            return 1.0;
        }
        let t = integrity.clamp(0.0, 1.0);
        self.min_coverage + (1.0 - self.min_coverage) * t
    }

    /// WHERE THIS GUARD IS CENTRED for a stick held `stick_descend`, as a
    /// signed fraction of the body's half-height along gravity.
    ///
    /// ⭐ NO SIGN FLIP, and that is not luck: `LocalAxes` is already
    /// `+y toward-feet`, the same axis and the same sense that
    /// [`ambition_platformer2d::combat::util::guard_covers_hit`] measures a hit on. Holding
    /// DOWN leans the guard toward the feet and hands the head over; holding UP
    /// does the reverse. A negation here would silently invert the mechanic —
    /// and a test written from the wrong assumption would agree with it.
    pub fn tilt_bias(self, stick_descend: f32) -> f32 {
        stick_descend.clamp(-1.0, 1.0) * self.tilt_range
    }
}

/// THE FOOTSTOOL — jumping off another body's head.
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
    /// Seconds the stomped body has no control authority when the footstool
    /// does NOT tumble it — a grounded victim, or a body that never tumbles.
    /// Short on purpose: Ultimate's grounded footstool is a beat you follow up
    /// on, not a punish by itself.
    pub flinch_time: f32,
    /// Seconds an AIRBORNE victim tumbles for. Authored rather than derived
    /// from [`Self::press_speed`] because a footstool "does not produce proper
    /// knockback" — the tumble is the mechanic, the shove is only its distance.
    pub air_tumble_time: f32,
    /// Seconds of intangibility the STOMPER gets for taking the bounce. Four
    /// frames in Ultimate, and the reason a footstool is an escape from
    /// disadvantage rather than only a mobility trick.
    pub stomper_invuln: f32,
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
        flinch_time: 0.0,
        air_tumble_time: 0.0,
        stomper_invuln: 0.0,
        band: 0.0,
    };

    /// Platform-fighter defaults: a hop a touch under a full jump, a shove that
    /// costs the stomped body its airspace, a flinch short enough to be a combo
    /// starter on the ground, and an air tumble long enough to be fatal off it.
    pub const PLATFORM_FIGHTER: Self = Self {
        rise_speed: 330.0,
        press_speed: 220.0,
        flinch_time: 0.12,
        air_tumble_time: 0.40,
        stomper_invuln: 4.0 / 60.0,
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
                run_commit_frac: self.run_commit_frac,
                crouch_speed_frac: self.crouch_speed_frac,
                initial_dash_time: self.initial_dash_time,
                initial_dash_speed: self.initial_dash_speed,
                turnaround_time: self.turnaround_time,
                teeter_margin: self.teeter_margin,
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
                dodge_roll_endlag: self.dodge_roll_endlag,
                dodge_stale_step: self.dodge_stale_step,
                dodge_stale_floor: self.dodge_stale_floor,
                dodge_stale_recovery: self.dodge_stale_recovery,
                untechable_launch_speed: self.untechable_launch_speed,
                evade_cancel_tail: self.evade_cancel_tail,
                air_dodge_time: self.air_dodge_time,
                air_dodge_speed: self.air_dodge_speed,
                air_dodge_endlag: self.air_dodge_endlag,
                tumble_speed: self.tumble_speed,
                spot_dodge_time: self.spot_dodge_time,
                parry_timing: self.parry_timing,
                sdi_step: self.sdi_step,
                asdi_step: self.asdi_step,
                jab_lock_speed: self.jab_lock_speed,
                jab_lock_limit: self.jab_lock_limit,
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
    // Zero inherits the ground run-speed cap.
    max_air_speed: 0.0,
    run_commit_frac: RUN_COMMIT_FRAC,
    crouch_speed_frac: 1.0,
    // ⛔ NO INITIAL-DASH PHASE for the engine default: ground speed stays one
    // continuum, which is what every world in this repo walks on.
    initial_dash_time: 0.0,
    initial_dash_speed: 0.0,
    turnaround_time: 0.0,
    teeter_margin: 0.0,
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
    // ZERO by default, for the same reason the air dodge below is: an
    // exploration body's roll is traversal, and charging it recovery would
    // change every game that never asked for a punish window. A FIGHTER
    // declares one.
    dodge_roll_endlag: 0.0,
    // NO DODGE STALING by default: an exploration body's roll is traversal, and
    // wearing out its i-frames would change every game that never asked.
    dodge_stale_step: 0.0,
    dodge_stale_floor: 1.0,
    dodge_stale_recovery: 0.0,
    // Every launch is techable in a game that declares nothing.
    untechable_launch_speed: 0.0,
    // No lockout: an attack may cancel an evade at any point, which is what
    // every body did before the knob existed.
    evade_cancel_tail: 0.0,
    // ZERO in the default tuning, and that is the decision, not an
    // oversight. An airborne dash press already MEANS something for a body
    // with the dash ability — it is the protagonist's air dash, a traversal
    // move — and a default-on air dodge would quietly take that press away from
    // every exploration body in the game. The maneuver is body-generic in the
    // kernel and AUTHORED per character, exactly like the shield, the ledge and
    // the moveset: a fighter says `air_dodge_time: AIR_DODGE_TIME` and gets one.
    air_dodge_time: 0.0,
    air_dodge_speed: AIR_DODGE_SPEED,
    air_dodge_endlag: AIR_DODGE_ENDLAG,
    // zero for the same reason the air dodge is: a wandering enemy that got
    // knocked down and had to stand up would be a different game.
    tumble_speed: 0.0,
    // zero for the same reason: an exploration body's grounded evade is the
    // roll, and a second one it never asked for would take that press away.
    spot_dodge_time: 0.0,
    // Smash 4's, which is what the parry has always been here — a knob's
    // default is the behaviour that already shipped.
    parry_timing: ParryTiming::OnRaise,
    // zero for the same reason: a body that cannot be launched has nothing to
    // influence its way out of.
    sdi_step: 0.0,
    asdi_step: 0.0,
    jab_lock_speed: 0.0,
    jab_lock_limit: 0,
    parry_window_time: PARRY_WINDOW_TIME,
    shield: ShieldTuning::OFF,
    footstool: FootstoolTuning::OFF,
    ledge_momentum: LedgeMomentumTuning::DEFAULT,
};

#[cfg(test)]
mod air_speed_tests {
    use super::*;

    /// Air acceleration was authored and air TOP SPEED was not, so a
    /// body's ground run cap governed its drift — accidental reuse of ground
    /// locomotion, and the reason a slow-running heavy that drifts fast could
    /// not be expressed.
    ///
    /// the poison is the inherit case, and it is the one that matters:
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
