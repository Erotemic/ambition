//! Player movement simulation.
//!
//! This module contains the code that makes the current prototype feel like a
//! platformer: coyote time, buffered jumps, optional double jumps, optional
//! wall jumps/cling/climb, optional dash/double dash, blink/precision blink,
//! pogo refreshes, rebound pads, hazards, and a symbolic operation trace.
//!
//! The update function is intentionally renderer-free. It consumes a plain
//! `InputState`, mutates a `Player`, and returns `FrameEvents` that the Bevy
//! layer can turn into particles, hitstop, sound, or debug overlays.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::abilities::AbilitySet;
use crate::geometry::{Aabb, AabbExt};
use crate::world::{BlinkWallTier, BlockKind, World};
use crate::{approach, Vec2};

/// A symbolic movement operation that can be shown in the debug HUD.
///
/// These are the first seeds of the "movement algebra" concept: order matters,
/// and the game can explain advanced movement as compositions of simple verbs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovementOp {
    Jump,
    DoubleJump,
    WallJump,
    WallCling,
    WallClimb,
    Dash,
    DoubleDash,
    FlyToggle,
    Blink,
    PrecisionBlink,
    Pogo,
    Rebound,
    Slash,
    Reset,
}

impl MovementOp {
    pub fn symbol(self) -> &'static str {
        match self {
            MovementOp::Jump => "J",
            MovementOp::DoubleJump => "DJ",
            MovementOp::WallJump => "WJ",
            MovementOp::WallCling => "WC",
            MovementOp::WallClimb => "W^",
            MovementOp::Dash => "D",
            MovementOp::DoubleDash => "DD",
            MovementOp::FlyToggle => "F",
            MovementOp::Blink => "B",
            MovementOp::PrecisionBlink => "PB",
            MovementOp::Pogo => "P",
            MovementOp::Rebound => "R",
            MovementOp::Slash => "S",
            MovementOp::Reset => "0",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            MovementOp::Jump => "jump",
            MovementOp::DoubleJump => "double jump",
            MovementOp::WallJump => "wall jump",
            MovementOp::WallCling => "wall cling",
            MovementOp::WallClimb => "wall climb",
            MovementOp::Dash => "dash",
            MovementOp::DoubleDash => "double dash",
            MovementOp::FlyToggle => "fly toggle",
            MovementOp::Blink => "blink",
            MovementOp::PrecisionBlink => "precision blink",
            MovementOp::Pogo => "pogo",
            MovementOp::Rebound => "rebound",
            MovementOp::Slash => "slash",
            MovementOp::Reset => "reset",
        }
    }
}

impl fmt::Display for MovementOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.symbol())
    }
}

/// A timestamped combo entry for debug display and future scoring/teaching.
#[derive(Clone, Debug)]
pub struct ComboMark {
    pub op: MovementOp,
    pub age: f32,
}

/// Kinematic player state.
///
/// The player is represented by a single AABB and hand-authored movement
/// timers. This gives us the tight, custom feel we want for a platformer and
/// avoids delegating core character motion to a generic rigid-body solver.
#[derive(Clone, Debug)]
pub struct Player {
    /// Active ability/upgrades for this player. The sandbox enables all current
    /// abilities by default; tests and future story states can use smaller sets.
    pub abilities: AbilitySet,
    pub pos: Vec2,
    pub vel: Vec2,
    pub size: Vec2,
    /// Standing-stance AABB size. `size` mirrors `base_size` while
    /// `body_mode == BodyMode::Standing`; alternate stances (Crouching /
    /// Crawling / MorphBall) shrink `size` while leaving `base_size`
    /// untouched so transitions back to Standing always use the
    /// canonical shape from this field.
    pub base_size: Vec2,
    pub facing: f32,
    pub on_ground: bool,
    pub on_wall: bool,
    pub wall_normal_x: f32,
    /// Back-compat/debug convenience: true when at least one dash charge exists.
    pub dash_available: bool,
    /// Number of dash charges available before the next refresh.
    pub dash_charges_available: u8,
    pub air_jumps_available: u8,
    /// True while free-flight mode is toggled on. Flight is intentionally
    /// floaty/accelerative rather than pixel-precise movement.
    pub fly_enabled: bool,
    /// Phase accumulator for the subtle idle hover bob while flying.
    pub flight_phase: f32,
    /// Time until blink can be started again.
    pub blink_cooldown: f32,
    /// True while the blink/special button is being held for quick/precision blink.
    pub blink_hold_active: bool,
    /// Current hold duration for the blink button.
    pub blink_hold_timer: f32,
    /// True after the hold crosses `blink_hold_threshold`; the sandbox uses
    /// this to enter bullet-time/aim-preview mode.
    pub blink_aiming: bool,
    /// Precision-blink aim cursor relative to the player position. Quick blink
    /// ignores this, but long-hold precision blink updates it gradually.
    pub blink_aim_offset: Vec2,
    /// Short post-blink grace window. While positive, ordinary falling is
    /// suspended/clamped so repeated blinks feel like controlled teleports
    /// instead of preserving accumulated fall speed.
    pub blink_grace_timer: f32,
    /// True after a double-tap-down has committed to fast-fall. Holding down
    /// alone does not set this, preserving down+attack as a natural pogo input.
    pub fast_falling: bool,
    /// True the frames the player is held-jump gliding with
    /// `abilities.glide` enabled. Set inside `integrate_velocity` from
    /// the live input + airborne + falling test; cleared on landing,
    /// dash start, blink, fly toggle, water contact, fast-fall.
    /// Sandbox / sprite / sfx hooks read this for glide cape vfx.
    pub gliding: bool,
    pub wall_clinging: bool,
    pub wall_climbing: bool,
    pub dash_timer: f32,
    pub dash_cooldown: f32,
    /// Buffered dash input. This lets a dash pressed a few frames before
    /// cooldown/charge availability still fire once the dash becomes legal.
    pub dash_buffer_timer: f32,
    pub jump_buffer_timer: f32,
    pub coyote_timer: f32,
    pub rebound_cooldown: f32,
    /// Brief window after a one-way drop-through gesture during which the
    /// vertical sweep continues to ignore one-way platforms. Without this the
    /// player would be snapped back onto the platform on the next frame, while
    /// still inside the landing-tolerance band.
    pub drop_through_timer: f32,
    pub combo: Vec<ComboMark>,
    pub max_speed: f32,
    pub time_alive: f32,
    pub resets: u32,
    /// Multiplier on outgoing player melee/projectile damage. The
    /// sandbox F3 stats editor and any future power-up writes this;
    /// damage-emitting code reads it. Default 1; clamped to >=1 by
    /// callers that want a "no zero-damage hits" floor.
    ///
    /// Lives on `Player` (not `Health`) because it scales the player's
    /// outgoing damage, not their incoming damage. Promoted from
    /// `SandboxRuntime::slash_damage` so per-player tuning is engine
    /// state, not sandbox-only state.
    pub damage_multiplier: i32,
    /// Generic resource meter the player spends on charge attacks /
    /// special abilities. Defaults to a full 100/100 meter with no
    /// regen / decay. Surfaced through `crate::ResourceMeter` so any
    /// future ability can wire `try_spend` / `tick_regen`. Promoted
    /// from `SandboxRuntime::mana_current` / `mana_max` (kept as i32
    /// in the sandbox for the F3 inspector — this struct's f32
    /// internals are converted at the editor boundary).
    pub mana: crate::ResourceMeter,
    /// True → all incoming damage to this player is dropped before HP
    /// math runs. Used by the F3 stats editor's "invincible" toggle and
    /// any future invuln-frame mechanics.
    ///
    /// Lives on `Player` (not `Health`) so the Player aggregate carries
    /// both gameplay flags AND health together for save/load and
    /// per-player multiplayer state. Promoted from
    /// `SandboxRuntime::invincible`.
    pub invincible: bool,
    /// Authoritative body-shape stance. Default is `Standing`. Sandbox
    /// systems writing crouch / morph / slide should set this directly,
    /// gated on `BodyShape::fits_at` for collision-safe resize.
    /// Trace/HUD readers consult this field instead of inferring.
    pub body_mode: crate::player_state::BodyMode,
    /// Cached water contact for this frame. Set at the top of
    /// `update_player_simulation_with_tuning` from
    /// `World::water_at(player.aabb)`. Movement uses this to:
    /// - drown when `!abilities.swim`,
    /// - convert buffered jump presses into swim impulses,
    /// - apply buoyancy / drag / fall-cap during integration.
    pub water_contact: Option<crate::world::WaterContact>,
}

impl Player {
    /// Create an endgame-sandbox player with all currently implemented verbs.
    pub fn new(spawn: Vec2) -> Self {
        Self::new_with_abilities(spawn, AbilitySet::sandbox_all())
    }

    /// Create a player with a specific ability set.
    ///
    /// This constructor is important for automated tests and future story
    /// progression, where we need to check the game with only a subset of
    /// abilities unlocked.
    pub fn new_with_abilities(spawn: Vec2, abilities: AbilitySet) -> Self {
        let dash_charges = abilities.dash_charge_count();
        Self {
            abilities,
            pos: spawn,
            vel: Vec2::ZERO,
            size: Vec2::new(28.0, 46.0),
            base_size: Vec2::new(28.0, 46.0),
            facing: 1.0,
            on_ground: false,
            on_wall: false,
            wall_normal_x: 0.0,
            dash_available: dash_charges > 0,
            dash_charges_available: dash_charges,
            air_jumps_available: abilities.air_jump_count(DEFAULT_TUNING.air_jumps),
            fly_enabled: false,
            flight_phase: 0.0,
            blink_cooldown: 0.0,
            blink_hold_active: false,
            blink_hold_timer: 0.0,
            blink_aiming: false,
            blink_aim_offset: Vec2::new(BLINK_DISTANCE, 0.0),
            blink_grace_timer: 0.0,
            fast_falling: false,
            gliding: false,
            wall_clinging: false,
            wall_climbing: false,
            dash_timer: 0.0,
            dash_cooldown: 0.0,
            dash_buffer_timer: 0.0,
            jump_buffer_timer: 0.0,
            coyote_timer: 0.0,
            rebound_cooldown: 0.0,
            drop_through_timer: 0.0,
            combo: Vec::new(),
            max_speed: 0.0,
            time_alive: 0.0,
            resets: 0,
            damage_multiplier: 1,
            invincible: false,
            mana: crate::ResourceMeter::new(100.0, 0.0, 0.0),
            body_mode: crate::player_state::BodyMode::Standing,
            water_contact: None,
        }
    }

    pub fn aabb(&self) -> Aabb {
        Aabb::new(self.pos, self.size * 0.5)
    }

    /// Reset position/resources while preserving the active ability set.
    pub fn reset_to(&mut self, spawn: Vec2) {
        let resets = self.resets + 1;
        let abilities = self.abilities;
        *self = Player::new_with_abilities(spawn, abilities);
        self.resets = resets;
        self.record(MovementOp::Reset);
    }

    /// Refill movement resources that are refreshed by touching safe surfaces
    /// or pogo/rebound targets.
    pub fn refresh_movement_resources(&mut self, tuning: MovementTuning) {
        self.dash_charges_available = self.abilities.dash_charge_count();
        self.dash_available = self.dash_charges_available > 0;
        self.air_jumps_available = self.abilities.air_jump_count(tuning.air_jumps);
    }

    fn spend_dash_charge(&mut self) -> MovementOp {
        let before = self.dash_charges_available;
        self.dash_charges_available = self.dash_charges_available.saturating_sub(1);
        self.dash_available = self.dash_charges_available > 0;
        if before >= 2 {
            MovementOp::DoubleDash
        } else {
            MovementOp::Dash
        }
    }

    fn record(&mut self, op: MovementOp) {
        self.combo.push(ComboMark { op, age: 0.0 });
        if self.combo.len() > 18 {
            let excess = self.combo.len() - 18;
            self.combo.drain(0..excess);
        }
    }

    pub fn combo_symbols(&self) -> String {
        if self.combo.is_empty() {
            return "-".to_string();
        }
        self.combo
            .iter()
            .map(|m| m.op.symbol())
            .collect::<Vec<_>>()
            .join(" o ")
    }

    /// Temporary teaching/debug hint for the most recent operation pair.
    ///
    /// This is not meant to be final UI copy. It exists so early playtests can
    /// see that the engine is already thinking in terms of ordered operations.
    pub fn current_combo_hint(&self) -> &'static str {
        let Some(a) = self.combo.iter().rev().nth(1).map(|m| m.op) else {
            return "build a chain: jump, dash, pogo, rebound";
        };
        let Some(b) = self.combo.iter().rev().next().map(|m| m.op) else {
            return "build a chain: jump, dash, pogo, rebound";
        };
        match (a, b) {
            (MovementOp::Dash, MovementOp::Pogo) => {
                "D o P: dash then pogo converts speed into height"
            }
            (MovementOp::Pogo, MovementOp::Dash) => {
                "P o D: pogo then dash converts height into lateral routing"
            }
            (MovementOp::Jump, MovementOp::DoubleJump) => {
                "J o DJ: save the second jump for route correction"
            }
            (MovementOp::Dash, MovementOp::DoubleJump) => {
                "D o DJ: dash then double jump recovers a bad line"
            }
            (MovementOp::WallJump, MovementOp::Dash) => {
                "WJ o D: wall jump then dash is a fast exit"
            }
            (MovementOp::Dash, MovementOp::WallJump) => "D o WJ: dash into wall to bank momentum",
            (MovementOp::WallCling, MovementOp::WallClimb) => {
                "WC o W^: cling opens vertical routing"
            }
            (MovementOp::Rebound, MovementOp::Dash) => {
                "R o D: launcher into dash preserves the loop"
            }
            (MovementOp::Dash, MovementOp::Slash) => "D o S: dash slash is a commitment",
            (MovementOp::Slash, MovementOp::Dash) => "S o D: slash dash is a correction",
            (MovementOp::DoubleDash, MovementOp::DoubleJump) => {
                "DD o DJ: spend horizontal resources before vertical recovery"
            }
            (MovementOp::Blink, MovementOp::Dash) => {
                "B o D: blink then dash extends a chosen vector"
            }
            (MovementOp::Dash, MovementOp::Blink) => {
                "D o B: dash then blink preserves intent but changes topology"
            }
            (MovementOp::PrecisionBlink, MovementOp::Slash) => {
                "PB o S: aim blink into an exact hit"
            }
            _ => "order matters: this trace is a movement algebra sketch",
        }
    }
}

/// Game-action input for one simulation frame.
///
/// Keyboard/gamepad remapping belongs in the presentation layer. Once those
/// devices are interpreted, the engine only needs a small set of actions.
#[derive(Clone, Copy, Debug, Default)]
pub struct InputState {
    /// -1 left, +1 right.
    pub axis_x: f32,
    /// -1 up, +1 down.
    pub axis_y: f32,
    pub jump_pressed: bool,
    pub jump_held: bool,
    pub jump_released: bool,
    pub dash_pressed: bool,
    /// Toggle free-flight mode when the ability is enabled.
    pub fly_toggle_pressed: bool,
    /// Blink/special button pressed this frame.
    pub blink_pressed: bool,
    /// Blink/special button held this frame.
    pub blink_held: bool,
    /// Blink/special button released this frame.
    pub blink_released: bool,
    /// Double-tap-down gesture recognized by the input layer. This is separate
    /// from `axis_y` so down+attack can mean pogo without forcing fast-fall.
    pub fast_fall_pressed: bool,
    /// Down-held + jump-pressed gesture: drop through one-way platforms.
    /// The presentation layer composes this from raw inputs so the engine
    /// does not have to reason about jump-vs-drop disambiguation itself.
    pub drop_through_pressed: bool,
    pub attack_pressed: bool,
    /// Dedicated downward/pogo slash action. This is separate from
    /// `attack_pressed` so layouts can expose four main face-button verbs.
    pub pogo_pressed: bool,
    pub reset_pressed: bool,
    /// Real, unscaled frame duration supplied by the presentation layer.
    ///
    /// Most simulation uses the scaled `raw_dt`, but precision-blink aiming is
    /// a control/UI gesture: the cursor should stay responsive even when game
    /// time is nearly frozen. If zero, the engine falls back to scaled dt.
    pub control_dt: f32,
}

/// Engine event emitted when a blink teleports the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlinkEvent {
    pub from: Vec2,
    pub to: Vec2,
    pub precision: bool,
}

/// Engine events emitted by one player simulation step.
#[derive(Clone, Debug, Default)]
pub struct FrameEvents {
    pub operations: Vec<MovementOp>,
    pub blinks: Vec<BlinkEvent>,
    pub reset: bool,
    pub hazard: bool,
    /// AABBs of pogo-orb-like blocks the player bounced off this frame.
    /// The sandbox uses this to damage breakable pogo orbs whose runtime
    /// AABB matches; non-breakable pogo orbs are ignored.
    pub pogo_hits: Vec<Aabb>,
}

impl FrameEvents {
    fn op(&mut self, player: &mut Player, op: MovementOp) {
        self.operations.push(op);
        player.record(op);
    }

    /// Merge another event bundle into this frame.
    ///
    /// This is used by the two-clock update path: control/intent is processed
    /// in real time, then physical evolution is processed in scaled game time.
    pub fn extend(&mut self, other: FrameEvents) {
        self.operations.extend(other.operations);
        self.blinks.extend(other.blinks);
        self.reset |= other.reset;
        self.hazard |= other.hazard;
        self.pogo_hits.extend(other.pogo_hits);
    }
}

// First-pass movement constants. These remain constants for easy grep/tuning,
// but the simulation accepts a `MovementTuning` so experiments can override
// them without recompiling every assumption into the update function.
pub const GRAVITY: f32 = 2250.0;
pub const RUN_ACCEL: f32 = 7600.0;
pub const AIR_ACCEL: f32 = 4700.0;
pub const GROUND_FRICTION: f32 = 9200.0;
pub const AIR_FRICTION: f32 = 860.0;
pub const MAX_RUN_SPEED: f32 = 330.0;
pub const MAX_FALL_SPEED: f32 = 1040.0;
pub const JUMP_SPEED: f32 = 660.0;
pub const DOUBLE_JUMP_SPEED: f32 = 610.0;
pub const WALL_JUMP_X: f32 = 500.0;
pub const WALL_SLIDE_SPEED: f32 = 170.0;
pub const WALL_CLIMB_SPEED: f32 = 250.0;
pub const DASH_SPEED: f32 = 820.0;
pub const DASH_TIME: f32 = 0.105;
pub const DASH_COOLDOWN: f32 = 0.060;
/// Grace window for a dash press that happens just before dash becomes legal.
pub const DASH_BUFFER: f32 = 0.110;
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
pub const FLIGHT_ACCEL: f32 = 900.0;
pub const FLIGHT_DRAG: f32 = 520.0;
pub const FLIGHT_TERMINAL_SPEED: f32 = 430.0;
pub const FLIGHT_HOVER_SPEED: f32 = 42.0;
pub const FLIGHT_HOVER_HZ: f32 = 0.85;
pub const COYOTE_TIME: f32 = 0.120;
pub const JUMP_BUFFER: f32 = 0.135;
/// Window during which the vertical sweep continues to ignore one-way
/// platforms after a drop-through gesture. Long enough to clear the 8px
/// landing tolerance under typical gravity, short enough that the player can
/// still re-land on a one-way they jump back up onto.
pub const ONE_WAY_DROP_THROUGH_GRACE: f32 = 0.18;
pub const POGO_SPEED: f32 = 810.0;
pub const SLASH_RECOIL: f32 = 130.0;
pub const AIR_JUMPS: u8 = 1;

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
};

pub fn update_player(
    world: &World,
    player: &mut Player,
    input: InputState,
    raw_dt: f32,
) -> FrameEvents {
    update_player_with_tuning(world, player, input, raw_dt, DEFAULT_TUNING)
}

/// Advance the player for callers that do not care about separate clocks.
///
/// This compatibility wrapper uses the same duration for control and simulation.
/// The Bevy sandbox uses the split functions below so bullet-time can freeze
/// physical evolution while keeping input/aim control responsive.
pub fn update_player_with_tuning(
    world: &World,
    player: &mut Player,
    input: InputState,
    raw_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    let control_dt = if input.control_dt > 0.0 {
        input.control_dt
    } else {
        raw_dt
    };
    let mut events = update_player_control_with_tuning(world, player, input, control_dt, tuning);
    let sim_events = update_player_simulation_with_tuning(world, player, input, raw_dt, tuning);
    events.extend(sim_events);
    events
}

/// Process player intent and instantaneous actions using real, unscaled time.
///
/// Input should remain responsive during bullet-time: the blink aim cursor,
/// button-hold thresholds, toggles, dash presses, attack presses, and jump
/// buffering are control-layer concepts. They advance from real frame time,
/// not from slowed simulation time.
pub fn update_player_control(
    world: &World,
    player: &mut Player,
    input: InputState,
    control_dt: f32,
) -> FrameEvents {
    update_player_control_with_tuning(world, player, input, control_dt, DEFAULT_TUNING)
}

pub fn update_player_control_with_tuning(
    world: &World,
    player: &mut Player,
    input: InputState,
    control_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    let mut events = FrameEvents::default();

    if input.reset_pressed && player.abilities.reset {
        player.reset_to(world.spawn);
        events.reset = true;
        return events;
    }

    update_facing_and_control_intent(player, input, tuning);
    handle_mode_toggles(player, input, &mut events);
    handle_blink(world, player, input, control_dt, tuning, &mut events);
    handle_attacks(world, player, input, tuning, &mut events);
    handle_dash(player, input, tuning, &mut events);
    handle_jump_release(player, input);

    events
}

/// Advance physical world evolution using scaled game time.
///
/// Gravity, velocity integration, timers, coyote time, cooldowns, enemies,
/// platforms, and particles should all consume this same scaled timestep. Tiny
/// positive values are preserved so near-frozen bullet-time is honored; only
/// large frame spikes are capped.
pub fn update_player_simulation(
    world: &World,
    player: &mut Player,
    input: InputState,
    raw_dt: f32,
) -> FrameEvents {
    update_player_simulation_with_tuning(world, player, input, raw_dt, DEFAULT_TUNING)
}

pub fn update_player_simulation_with_tuning(
    world: &World,
    player: &mut Player,
    input: InputState,
    raw_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    let mut events = FrameEvents::default();
    if raw_dt <= 0.0 {
        return events;
    }
    let dt = raw_dt.min(1.0 / 30.0);

    // Water contact is queried once per tick and cached on the
    // player so jump-buffer handling, gravity integration, and the
    // post-step reset gate all see the same answer. Source-agnostic:
    // `water_at` covers both IntGrid `Water` cells and entity
    // `WaterVolume` regions.
    player.water_contact = world.water_at(player.aabb());

    // Drowning gate: water without the swim ability is a death zone,
    // not a slow-down. Trigger the same reset path the hazard tile
    // uses so the existing flash/sfx/respawn pipeline applies.
    if player.water_contact.is_some() && !player.abilities.swim {
        player.reset_to(world.spawn);
        events.hazard = true;
        events.reset = true;
        return events;
    }

    age_player(player, dt);
    update_simulation_timers(player, dt, tuning);
    handle_jump_buffer(world, player, input, tuning, &mut events);
    integrate_velocity(world, player, input, dt, tuning, &mut events);

    if touching_hazard(world, player) || player.pos.y > world.size.y + 200.0 {
        player.reset_to(world.spawn);
        events.hazard = true;
        events.reset = true;
    }

    events
}

fn age_player(player: &mut Player, dt: f32) {
    player.time_alive += dt;
    player.max_speed = player.max_speed.max(player.vel.length());
    for mark in &mut player.combo {
        mark.age += dt;
    }
    player
        .combo
        .retain(|m| m.age < 4.0 || m.op == MovementOp::Reset);
}

fn update_facing_and_control_intent(
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
) {
    if input.axis_x.abs() > 0.1 {
        player.facing = input.axis_x.signum();
    }

    if input.jump_pressed && player.abilities.jump {
        player.jump_buffer_timer = tuning.jump_buffer;
    }
    if input.dash_pressed && player.abilities.dash {
        player.dash_buffer_timer = tuning.dash_buffer;
    }
}

fn update_simulation_timers(player: &mut Player, dt: f32, tuning: MovementTuning) {
    player.jump_buffer_timer = dec(player.jump_buffer_timer, dt);
    player.dash_buffer_timer = dec(player.dash_buffer_timer, dt);
    player.coyote_timer = dec(player.coyote_timer, dt);
    player.drop_through_timer = dec(player.drop_through_timer, dt);
    player.dash_cooldown = dec(player.dash_cooldown, dt);
    player.blink_cooldown = dec(player.blink_cooldown, dt);
    player.blink_grace_timer = dec(player.blink_grace_timer, dt);
    player.rebound_cooldown = dec(player.rebound_cooldown, dt);

    if player.on_ground {
        player.coyote_timer = tuning.coyote_time;
        player.refresh_movement_resources(tuning);
    }
}

fn handle_mode_toggles(player: &mut Player, input: InputState, events: &mut FrameEvents) {
    if input.fly_toggle_pressed && player.abilities.fly {
        player.fly_enabled = !player.fly_enabled;
        if player.fly_enabled {
            player.fast_falling = false;
            player.wall_clinging = false;
            player.wall_climbing = false;
            player.dash_timer = 0.0;
            player.blink_grace_timer = 0.0;
        }
        events.op(player, MovementOp::FlyToggle);
    }
}

fn handle_blink(
    world: &World,
    player: &mut Player,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if !player.abilities.blink {
        player.blink_hold_active = false;
        player.blink_aiming = false;
        player.blink_hold_timer = 0.0;
        player.blink_aim_offset = Vec2::new(tuning.blink_distance * player.facing, 0.0);
        return;
    }

    if (input.blink_pressed || (input.blink_held && !player.blink_hold_active))
        && player.blink_cooldown <= 0.0
    {
        // Permit a held blink button to arm as soon as cooldown clears. This
        // avoids a bad second-blink case where the user pressed slightly early,
        // the hold was ignored, and bullet-time never engaged.
        player.blink_hold_active = true;
        player.blink_hold_timer = 0.0;
        player.blink_aiming = false;
        player.blink_aim_offset = Vec2::new(tuning.blink_distance * player.facing, 0.0);
    }

    if player.blink_hold_active && input.blink_held {
        // Blink hold/aim uses unscaled control time. During precision blink,
        // physics can be nearly frozen, but the destination cursor should still
        // feel like a responsive UI control.
        let control_dt = dt.min(1.0 / 20.0);
        player.blink_hold_timer += control_dt;
        if player.abilities.precision_blink
            && player.blink_hold_timer >= tuning.blink_hold_threshold
        {
            player.blink_aiming = true;
        }
        if player.blink_aiming {
            let aim_input = Vec2::new(input.axis_x, input.axis_y);
            if aim_input.length_squared() > 0.01 {
                player.blink_aim_offset +=
                    aim_input * (tuning.precision_blink_aim_speed * control_dt);
                player.blink_aim_offset = player
                    .blink_aim_offset
                    .clamp_length_max(tuning.precision_blink_distance);
            }
        }
    }

    if player.blink_hold_active && input.blink_released {
        let fallback = Vec2::new(player.facing, 0.0);
        let aim = Vec2::new(input.axis_x, input.axis_y).normalize_or(fallback);
        let precision = player.blink_aiming && player.abilities.precision_blink;
        let from = player.pos;
        let to = if precision {
            blink_destination_to_point(world, player, player.pos + player.blink_aim_offset)
        } else {
            blink_destination(world, player, aim, tuning.blink_distance)
        };
        complete_blink(player, from, to, precision, tuning, events);
    }

    // Cancel a partially-started blink if the binding disappeared for any
    // reason without a release event. This avoids sticky bullet-time state when
    // focus changes or a future remapper swaps presets mid-hold.
    if player.blink_hold_active && !input.blink_held && !input.blink_released {
        player.blink_hold_active = false;
        player.blink_aiming = false;
        player.blink_hold_timer = 0.0;
        player.blink_aim_offset = Vec2::new(tuning.blink_distance * player.facing, 0.0);
    }
}

/// Finish a blink in one place so every blink variant shares the same
/// post-teleport state policy.
///
/// Blink completion is kept in one place so destination resolution, cooldowns,
/// presentation events, and post-blink state stay consistent across quick and
/// precision variants.
fn complete_blink(
    player: &mut Player,
    from: Vec2,
    to: Vec2,
    precision: bool,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    player.pos = to;
    apply_post_blink_motion(player, precision, tuning);
    player.blink_cooldown = tuning.blink_cooldown;
    player.blink_hold_active = false;
    player.blink_hold_timer = 0.0;
    player.blink_aiming = false;
    player.blink_aim_offset = Vec2::new(tuning.blink_distance * player.facing, 0.0);
    let op = if precision {
        MovementOp::PrecisionBlink
    } else {
        MovementOp::Blink
    };
    events.op(player, op);
    events.blinks.push(BlinkEvent {
        from,
        to,
        precision,
    });
}

/// Apply the movement-state aftermath of a completed blink.
///
/// Blink is a topological reposition, not another gravity-preserving dash. This
/// policy is intentionally small and explicit. The real bullet-time invariant is
/// enforced by the split control/simulation clocks above; this function only
/// defines the immediate feel after teleporting.
fn apply_post_blink_motion(player: &mut Player, precision: bool, tuning: MovementTuning) {
    let damping = if precision { 0.35 } else { 0.55 };
    let max_downward = if precision {
        tuning.precision_blink_max_downward_speed
    } else {
        tuning.blink_max_downward_speed
    };

    player.vel.x *= damping;
    if player.vel.y > max_downward {
        player.vel.y = max_downward;
    } else {
        player.vel.y *= damping;
    }

    player.fast_falling = false;
    player.wall_clinging = false;
    player.wall_climbing = false;
    player.dash_timer = 0.0;
    player.blink_grace_timer = tuning.blink_grace_time;
}

fn handle_attacks(
    world: &World,
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if !player.abilities.attack {
        return;
    }
    let can_pogo = player.abilities.pogo;
    if input.pogo_pressed && can_pogo {
        if let Some(orb_aabb) = try_pogo(world, player, tuning) {
            events.op(player, MovementOp::Pogo);
            events.pogo_hits.push(orb_aabb);
        } else {
            // Dedicated pogo whiff still gives a tiny correction so it can be
            // tested as a fourth face-button verb without requiring a target.
            player.vel.x -= player.facing * (tuning.slash_recoil * 0.45);
            events.op(player, MovementOp::Slash);
        }
    } else if input.attack_pressed {
        if can_pogo && input.axis_y > 0.25 {
            if let Some(orb_aabb) = try_pogo(world, player, tuning) {
                events.op(player, MovementOp::Pogo);
                events.pogo_hits.push(orb_aabb);
            } else {
                player.vel.x -= player.facing * tuning.slash_recoil;
                events.op(player, MovementOp::Slash);
            }
        } else {
            // A small generated recoil/correction action. It exists to test
            // cancellability and non-commutative feel.
            player.vel.x -= player.facing * tuning.slash_recoil;
            events.op(player, MovementOp::Slash);
        }
    }
}

fn handle_jump_buffer(
    world: &World,
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if player.jump_buffer_timer > 0.0 {
        // Underwater swimming wins over every other jump path: while
        // submerged with the swim ability, a buffered jump becomes
        // exactly one upward swim stroke and nothing else. This keeps
        // "underwater jump != normal jump" true on a single press,
        // and the `min(-impulse)` floor makes repeated taps reliably
        // rise even if the previous stroke is still climbing.
        if let Some(contact) = player.water_contact {
            if player.abilities.swim {
                let impulse = contact.spec.swim_up_impulse;
                player.vel.y = (player.vel.y - impulse).min(-impulse);
                player.jump_buffer_timer = 0.0;
                player.coyote_timer = 0.0;
                events.op(player, MovementOp::Jump);
                return;
            }
        }

        // Down + jump while standing on a one-way platform means "drop through",
        // not "jump". Cancel the buffered jump so the vertical sweep can take
        // the player past the platform on the next integration step.
        if input.drop_through_pressed && player.on_ground && standing_on_one_way(world, player) {
            player.jump_buffer_timer = 0.0;
            player.on_ground = false;
            player.coyote_timer = 0.0;
            // Latch the drop-through so subsequent frames keep ignoring the
            // one-way until the player has cleared the landing tolerance band.
            // Without this, the gesture only frees the player for a single
            // frame and the resolve-up step snaps them back onto the platform.
            player.drop_through_timer = ONE_WAY_DROP_THROUGH_GRACE;
            return;
        }
        if player.abilities.wall_jump && player.on_wall && !player.on_ground {
            player.vel.x = player.wall_normal_x * tuning.wall_jump_x;
            player.vel.y = -tuning.jump_speed * 0.94;
            player.on_wall = false;
            player.wall_clinging = false;
            player.wall_climbing = false;
            player.jump_buffer_timer = 0.0;
            player.coyote_timer = 0.0;
            events.op(player, MovementOp::WallJump);
        } else if player.abilities.jump && (player.on_ground || player.coyote_timer > 0.0) {
            player.vel.y = -tuning.jump_speed;
            player.on_ground = false;
            player.jump_buffer_timer = 0.0;
            player.coyote_timer = 0.0;
            player.air_jumps_available = player.abilities.air_jump_count(tuning.air_jumps);
            events.op(player, MovementOp::Jump);
        } else if player.abilities.double_jump && player.air_jumps_available > 0 {
            player.vel.y = -tuning.double_jump_speed;
            player.on_ground = false;
            player.on_wall = false;
            player.wall_clinging = false;
            player.wall_climbing = false;
            player.jump_buffer_timer = 0.0;
            player.air_jumps_available -= 1;
            events.op(player, MovementOp::DoubleJump);
        }
    }
}

fn handle_jump_release(player: &mut Player, input: InputState) {
    // Variable jump height is an input/control gesture. It should react even
    // during bullet-time rather than waiting for scaled simulation time.
    if player.abilities.variable_jump && input.jump_released && player.vel.y < -120.0 {
        player.vel.y *= 0.54;
    }
}

fn handle_dash(
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if player.dash_buffer_timer > 0.0
        && player.abilities.dash
        && player.dash_charges_available > 0
        && player.dash_cooldown <= 0.0
    {
        let fallback = Vec2::new(player.facing, 0.0);
        let aim = Vec2::new(input.axis_x, input.axis_y).normalize_or(fallback);
        player.vel = aim * tuning.dash_speed;
        player.dash_timer = tuning.dash_time;
        player.dash_cooldown = tuning.dash_cooldown;
        player.dash_buffer_timer = 0.0;
        let op = player.spend_dash_charge();
        events.op(player, op);
    }
}

fn integrate_velocity(
    world: &World,
    player: &mut Player,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if player.dash_timer > 0.0 {
        player.dash_timer = dec(player.dash_timer, dt);
    } else if player.fly_enabled && player.abilities.fly {
        integrate_flight(player, input, dt, tuning);
    } else {
        let blink_hang_active = player.blink_grace_timer > 0.0 && player.vel.y >= 0.0;
        // Water makes gravity gentler and adds linear drag. We
        // multiply gravity by the region's `gravity_scale` (Mario-
        // style: still sinks, just slower) and apply per-frame drag
        // to both axes so directional inputs feel more like swimming
        // strokes than running. The fall cap below also gets lowered
        // to the per-region cap so the player doesn't accelerate to
        // dash speeds in deep water.
        let water_gravity_scale = player
            .water_contact
            .map(|c| c.spec.gravity_scale)
            .unwrap_or(1.0);
        if !blink_hang_active {
            player.vel.y += tuning.gravity * water_gravity_scale * dt;
        }
        if input.fast_fall_pressed && player.abilities.fast_fall && !player.on_ground {
            player.fast_falling = true;
        }
        if player.fast_falling && !blink_hang_active && player.water_contact.is_none() {
            player.vel.y += tuning.fast_fall_accel * dt;
        }

        // Glide: hold-jump while airborne and falling. Fast-fall and
        // water/blink-hang preempt it (the player explicitly chose
        // those alternatives), so glide only takes hold when none of
        // those modes are active. The actual fall cap lookup below
        // reads `player.gliding`.
        player.gliding = player.abilities.glide
            && !player.on_ground
            && !player.fast_falling
            && !blink_hang_active
            && player.water_contact.is_none()
            && input.jump_held
            && player.vel.y > 0.0;

        if player.abilities.move_horizontal {
            let accel = if player.on_ground {
                tuning.run_accel
            } else if player.gliding {
                tuning.glide_air_accel
            } else {
                tuning.air_accel
            };
            let target_vx = input.axis_x * tuning.max_run_speed;
            player.vel.x = approach(player.vel.x, target_vx, accel * dt);

            let friction = if player.on_ground {
                tuning.ground_friction
            } else {
                tuning.air_friction
            };
            if input.axis_x.abs() <= 0.1 {
                player.vel.x = approach(player.vel.x, 0.0, friction * dt);
            }
        }

        if let Some(contact) = player.water_contact {
            // Water drag is a linear-per-tick decay applied AFTER the
            // gravity / horizontal accel pass so the gravity-applied
            // velocity also gets damped.
            let drag = contact.spec.drag.clamp(0.0, 1.0);
            player.vel.x *= 1.0 - drag;
            player.vel.y *= 1.0 - drag;
            player.vel.y = player.vel.y.min(contact.spec.max_fall_speed);
        } else {
            let fall_cap = if player.fast_falling {
                tuning.fast_fall_speed
            } else if player.gliding {
                tuning.glide_fall_speed
            } else {
                tuning.max_fall_speed
            };
            player.vel.y = player.vel.y.min(fall_cap);
        }
    }

    // Resolve horizontal motion with a Parry-backed swept AABB. This
    // establishes wall contact for wall verbs without letting high-speed dash
    // or future knockback skip through a thin wall.
    player.on_wall = false;
    player.wall_normal_x = 0.0;
    player.wall_climbing = false;
    let was_clinging = player.wall_clinging;
    player.wall_clinging = false;
    sweep_player_x(world, player, player.vel.x * dt);

    apply_wall_abilities(player, input, tuning, was_clinging, events);

    // Resolve vertical motion. Previous bottom determines one-way behavior.
    let prev_bottom = player.aabb().bottom();
    player.on_ground = false;
    let drop_through = input.drop_through_pressed || player.drop_through_timer > 0.0;
    sweep_player_y(world, player, player.vel.y * dt, prev_bottom, drop_through);

    if player.on_ground {
        player.refresh_movement_resources(tuning);
        player.blink_grace_timer = 0.0;
        player.fast_falling = false;
        player.gliding = false;
        player.wall_clinging = false;
        player.wall_climbing = false;
        player.drop_through_timer = 0.0;
    }

    if player.abilities.rebound && player.rebound_cooldown <= 0.0 {
        if let Some(impulse) = touching_rebound(world, player) {
            player.vel = impulse;
            player.refresh_movement_resources(tuning);
            player.on_ground = false;
            player.rebound_cooldown = 0.18;
            events.op(player, MovementOp::Rebound);
        }
    }
}

fn integrate_flight(player: &mut Player, input: InputState, dt: f32, tuning: MovementTuning) {
    player.fast_falling = false;
    player.wall_clinging = false;
    player.wall_climbing = false;
    player.flight_phase += dt * tuning.flight_hover_hz * std::f32::consts::TAU;

    let target_x = input.axis_x * tuning.flight_terminal_speed;
    let mut target_y = input.axis_y * tuning.flight_terminal_speed;
    if input.axis_y.abs() <= 0.10 {
        target_y = player.flight_phase.sin() * tuning.flight_hover_speed;
    }

    player.vel.x = approach(player.vel.x, target_x, tuning.flight_accel * dt);
    player.vel.y = approach(player.vel.y, target_y, tuning.flight_accel * dt);

    if input.axis_x.abs() <= 0.10 {
        player.vel.x = approach(player.vel.x, 0.0, tuning.flight_drag * dt);
    }
    if input.axis_y.abs() <= 0.10 {
        player.vel.y = approach(player.vel.y, target_y, tuning.flight_drag * dt);
    }

    player.vel.x = player
        .vel
        .x
        .clamp(-tuning.flight_terminal_speed, tuning.flight_terminal_speed);
    player.vel.y = player
        .vel
        .y
        .clamp(-tuning.flight_terminal_speed, tuning.flight_terminal_speed);
}

fn apply_wall_abilities(
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
    was_clinging: bool,
    events: &mut FrameEvents,
) {
    if !player.on_wall || player.on_ground || !player.abilities.wall_cling {
        return;
    }
    // Pressing toward the wall means axis_x is opposite the collision normal.
    let pressing_into_wall =
        input.axis_x.abs() > 0.1 && input.axis_x.signum() == -player.wall_normal_x;
    if !pressing_into_wall {
        return;
    }

    player.wall_clinging = true;
    if player.abilities.wall_climb && input.axis_y.abs() > 0.25 {
        player.wall_climbing = true;
        player.vel.y = input.axis_y * tuning.wall_climb_speed;
        if !was_clinging {
            events.op(player, MovementOp::WallClimb);
        }
    } else {
        if player.vel.y > tuning.wall_slide_speed {
            player.vel.y = tuning.wall_slide_speed;
        }
        if !was_clinging {
            events.op(player, MovementOp::WallCling);
        }
    }
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

fn dec(value: f32, dt: f32) -> f32 {
    (value - dt).max(0.0)
}

fn is_solid_for_axis(kind: BlockKind, axis: Axis) -> bool {
    match kind {
        BlockKind::Solid | BlockKind::BlinkWall { .. } => true,
        BlockKind::OneWay => matches!(axis, Axis::Y),
        BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => false,
    }
}

fn sweep_fraction(time_of_impact: f32) -> f32 {
    time_of_impact.clamp(0.0, 1.0)
}

fn sweep_player_x(world: &World, player: &mut Player, delta_x: f32) {
    let delta = Vec2::new(delta_x, 0.0);
    if delta.x.abs() <= 1.0e-5 {
        resolve_axis(world, player, Axis::X);
        return;
    }

    if let Some(hit) = world.first_body_sweep(player.aabb(), delta, |block| {
        is_solid_for_axis(block.kind, Axis::X) && !matches!(block.kind, BlockKind::OneWay)
    }) {
        let toi_fraction = sweep_fraction(hit.time_of_impact);
        player.pos.x += delta.x * toi_fraction;
        let body = player.aabb();
        let immediate_contact = hit.time_of_impact <= 1.0e-5;
        let overlap_x = (body.right().min(hit.block.aabb.right())
            - body.left().max(hit.block.aabb.left()))
        .max(0.0);
        let overlap_y = (body.bottom().min(hit.block.aabb.bottom())
            - body.top().max(hit.block.aabb.top()))
        .max(0.0);
        // Skip the horizontal snap in two failure-mode cases:
        // 1. The contact is dominantly *vertical* (player's head poking
        //    into a wide ceiling, or feet poking into a wide floor). The
        //    perpendicular `resolve_vertical` pass owns this contact;
        //    pushing horizontally toward the block's far edge would
        //    catapult the player across the entire room.
        // 2. The contact is dominantly horizontal but the player is
        //    *already moving away* from the block (e.g. wall-jump pushed
        //    them off a wall they were sub-pixel-penetrating). The
        //    delta-direction snap logic uses delta.x sign to pick a face;
        //    when the player is on the far side from where delta.x points
        //    that pick is wrong and pushes them through the block.
        let vertical_dominant = immediate_contact && overlap_y > 0.0 && overlap_x > overlap_y;
        let body_to_right_of_block = body.center().x > hit.block.aabb.center().x;
        let moving_away_from_block =
            (body_to_right_of_block && delta.x > 0.0) || (!body_to_right_of_block && delta.x < 0.0);
        let horizontal_overlap_moving_away =
            immediate_contact && overlap_x > 0.0 && moving_away_from_block;
        if vertical_dominant || horizontal_overlap_moving_away {
            player.pos.x += delta.x * (1.0 - toi_fraction);
        } else {
            // Pick the snap face from the player's *position relative to
            // the block*, not from delta.x sign. The two only agree when
            // the player is approaching from the side delta.x implies;
            // for a pre-existing overlap they can disagree, which is the
            // tunneling failure mode addressed above.
            if body_to_right_of_block {
                player.pos.x += hit.block.aabb.right() - body.left();
                player.wall_normal_x = 1.0;
            } else {
                player.pos.x += hit.block.aabb.left() - body.right();
                player.wall_normal_x = -1.0;
            }
            player.vel.x = 0.0;
            player.on_wall = true;
        }
    } else {
        player.pos.x += delta.x;
    }

    // Shape casts catch fast motion; positional resolution remains as a cheap
    // penetration repair for starts inside geometry or stacked contacts.
    resolve_axis(world, player, Axis::X);
}

fn sweep_player_y(
    world: &World,
    player: &mut Player,
    delta_y: f32,
    prev_bottom: f32,
    drop_through: bool,
) {
    let delta = Vec2::new(0.0, delta_y);
    if delta.y.abs() <= 1.0e-5 {
        resolve_vertical(world, player, prev_bottom, drop_through);
        return;
    }

    let start_body = player.aabb();
    if let Some(hit) = world.first_body_sweep(player.aabb(), delta, |block| {
        if !is_solid_for_axis(block.kind, Axis::Y) {
            return false;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            let landing_from_above = delta.y >= 0.0 && prev_bottom <= block.aabb.top() + 8.0;
            return landing_from_above && !drop_through;
        }
        // AMBITION_REVIEW(spatial): reject blocks the body is already
        // overlapping dominantly on the y axis. Concrete repro: a player
        // wall-clinging on a tall left-side wall whose top is at world
        // y=0 used to get a `time_of_impact = 0` hit on the wall during
        // the downward y-sweep, then snap the body's bottom to the wall's
        // top — teleporting the player from `(62, 1678)` to `(62, -23)`
        // (= `0 - half_height`). The fix mirrors `resolve_axis(Axis::X)`'s
        // overlap-shape guard: when the existing x-overlap is non-zero
        // and the y-overlap is larger, this is a *side-wall* contact and
        // belongs to the x-axis sweep/resolve. The vertical sweep should
        // not see it. See `docs/lessons_learned.md` for the trace
        // signature and the regression test.
        if body_is_side_contact(start_body, block.aabb) {
            return false;
        }
        true
    }) {
        player.pos.y += delta.y * sweep_fraction(hit.time_of_impact);
        let body = player.aabb();
        if delta.y > 0.0 || body.center().y < hit.block.aabb.center().y {
            player.pos.y += hit.block.aabb.top() - body.bottom();
            player.on_ground = true;
        } else {
            player.pos.y += hit.block.aabb.bottom() - body.top();
        }
        player.vel.y = 0.0;
    } else {
        player.pos.y += delta.y;
    }

    resolve_vertical(world, player, prev_bottom, drop_through);
}

/// True when `body`'s vertical range fits entirely inside `block`'s
/// vertical range — i.e. the body is *alongside* the block, not above
/// or below it. The y-axis sweep / resolve cannot legitimately produce
/// a contact in that geometry: any landing has the body's bottom
/// approaching the block's top from above, and any ceiling hit has the
/// body's top approaching the block's bottom from below. A body
/// fully nested inside the block's y-range can only be touching the
/// block on its left or right *side*, which the x-axis sweep / resolve
/// owns.
///
/// This is the symmetric counterpart to the `overlap_x > overlap_y`
/// guard `resolve_axis(Axis::X)` already uses. The first revision of
/// this fix required `overlap_x > 0` (strict penetration), which
/// missed the trace's *exact-edge-touching* case (`body.left ==
/// wall.right` to within float precision). The current predicate
/// catches both edge-touching and penetrating side contacts because
/// the y-range test is independent of x-overlap.
///
/// Tolerance: a 1e-4 epsilon on the y-range bounds so a body whose
/// top exactly equals the block's top (e.g. a player standing at the
/// same height as a one-tile-tall ledge corner) is still classified
/// as a side-contact when the bottom is also inside.
fn body_is_side_contact(body: Aabb, block: Aabb) -> bool {
    const Y_NESTED_EPS: f32 = 1.0e-4;
    body.top() >= block.top() - Y_NESTED_EPS && body.bottom() <= block.bottom() + Y_NESTED_EPS
}

// AMBITION_REVIEW(spatial): one-way platform contact test. The 4px vertical
// epsilon mirrors the landing tolerance used by the vertical sweep; if either
// is changed the other should follow.
fn standing_on_one_way(world: &World, player: &Player) -> bool {
    let body = player.aabb();
    for block in &world.blocks {
        if !matches!(block.kind, BlockKind::OneWay) {
            continue;
        }
        let horizontally_overlaps =
            body.right() > block.aabb.left() + 1.0 && body.left() < block.aabb.right() - 1.0;
        let near_top = (body.bottom() - block.aabb.top()).abs() <= 4.0;
        if horizontally_overlaps && near_top {
            return true;
        }
    }
    false
}

fn resolve_axis(world: &World, player: &mut Player, axis: Axis) {
    let mut aabb = player.aabb();
    for block in &world.blocks {
        if !is_solid_for_axis(block.kind, axis) || !aabb.strict_intersects(block.aabb) {
            continue;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            continue;
        }
        match axis {
            Axis::X => {
                // Only resolve as a horizontal contact when the overlap is
                // shallower in x than in y. Otherwise this is a vertical
                // contact (player's head poking into a wide ceiling, or feet
                // poking into a wide floor) and the appropriate axis is the
                // perpendicular `resolve_vertical` pass — pushing
                // horizontally instead can catapult the player across the
                // entire room (the floor/ceiling block spans the whole
                // width, so its near edge is far away). Concrete repro: a
                // wall-jump off the left wall while feet barely overlap the
                // floor used to teleport the player tens of pixels left
                // through the wall.
                let overlap_x = (aabb.right().min(block.aabb.right())
                    - aabb.left().max(block.aabb.left()))
                .max(0.0);
                let overlap_y = (aabb.bottom().min(block.aabb.bottom())
                    - aabb.top().max(block.aabb.top()))
                .max(0.0);
                if overlap_x > overlap_y {
                    continue;
                }
                if aabb.center().x < block.aabb.center().x {
                    let push = block.aabb.left() - aabb.right();
                    player.pos.x += push;
                    player.wall_normal_x = -1.0;
                } else {
                    let push = block.aabb.right() - aabb.left();
                    player.pos.x += push;
                    player.wall_normal_x = 1.0;
                }
                player.vel.x = 0.0;
                player.on_wall = true;
            }
            Axis::Y => {}
        }
        aabb = player.aabb();
    }
}

fn resolve_vertical(world: &World, player: &mut Player, prev_bottom: f32, drop_through: bool) {
    let mut aabb = player.aabb();
    for block in &world.blocks {
        if !is_solid_for_axis(block.kind, Axis::Y) || !aabb.strict_intersects(block.aabb) {
            continue;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            let landing_from_above = player.vel.y >= 0.0 && prev_bottom <= block.aabb.top() + 8.0;
            if !landing_from_above || drop_through {
                continue;
            }
        }
        // AMBITION_REVIEW(spatial): symmetric to `resolve_axis(Axis::X)`.
        // If the body's y-range is entirely nested inside the block's
        // y-range, this is a side-wall contact — the x-axis sweep /
        // resolve owns it. Pushing vertically here can catapult the
        // player to the wall block's top edge if the wall spans the
        // full room height (concrete repro: wall-clinging on a tall
        // left wall whose top is at world y=32 used to teleport the
        // player to y = top - half_height = 9). Skipping OneWay
        // because OneWay is by construction wider than tall.
        if !matches!(block.kind, BlockKind::OneWay) && body_is_side_contact(aabb, block.aabb) {
            continue;
        }
        if aabb.center().y < block.aabb.center().y {
            let push = block.aabb.top() - aabb.bottom();
            player.pos.y += push;
            player.on_ground = true;
        } else {
            let push = block.aabb.bottom() - aabb.top();
            player.pos.y += push;
        }
        player.vel.y = 0.0;
        aabb = player.aabb();
    }
}

/// Attempt a pogo bounce. Returns the AABB of the orb-like block that was
/// hit (for the sandbox to route damage to a breakable pogo orb), or `None`
/// if no valid target was under the player's feet. Non-PogoOrb hits return
/// the AABB too so callers don't need to second-guess the kind, but the
/// sandbox damage path filters for orbs by matching against breakables
/// flagged `pogo_refresh`.
fn try_pogo(world: &World, player: &mut Player, tuning: MovementTuning) -> Option<Aabb> {
    let feet = player.aabb();
    let hitbox = Aabb::new(
        Vec2::new(feet.center().x, feet.bottom() + 18.0),
        Vec2::new(feet.half_size().x * 0.76, 22.0),
    );
    let hit = world.blocks.iter().find(|block| {
        let valid_target = matches!(
            block.kind,
            BlockKind::PogoOrb
                | BlockKind::Solid
                | BlockKind::BlinkWall { .. }
                | BlockKind::Rebound { .. }
        );
        valid_target && hitbox.strict_intersects(block.aabb)
    });
    if let Some(block) = hit {
        let aabb = block.aabb;
        player.vel.y = -tuning.pogo_speed;
        player.refresh_movement_resources(tuning);
        player.on_ground = false;
        Some(aabb)
    } else {
        None
    }
}

fn touching_hazard(world: &World, player: &Player) -> bool {
    let aabb = player.aabb();
    world
        .blocks
        .iter()
        .any(|b| matches!(b.kind, BlockKind::Hazard) && aabb.strict_intersects(b.aabb))
}

fn touching_rebound(world: &World, player: &Player) -> Option<Vec2> {
    let aabb = player.aabb();
    world.blocks.iter().find_map(|b| match b.kind {
        BlockKind::Rebound { impulse } if aabb.strict_intersects(b.aabb) => Some(impulse),
        _ => None,
    })
}

/// Compute the furthest safe blink destination along `aim`.
///
/// Blink should feel like a topological reposition, but it must not place the
/// player inside solid geometry. The implementation uses a Parry-backed shape
/// cast for hard blockers, then samples the remaining path so blink-through
/// walls can be crossed without becoming valid resting positions.
pub fn blink_destination(world: &World, player: &Player, aim: Vec2, max_distance: f32) -> Vec2 {
    let direction = aim.normalize_or(Vec2::new(player.facing, 0.0));
    blink_destination_to_point(world, player, player.pos + direction * max_distance)
}

/// Compute a safe blink destination toward a deliberate target point.
///
/// The path may cross configured blink walls if the player's ability set allows
/// it, but the final resting AABB must be free of solid geometry. This lets
/// blink-through upgrades become meaningful without ever depositing the player
/// inside a wall.
pub fn blink_destination_to_point(world: &World, player: &Player, target: Vec2) -> Vec2 {
    let start = player.pos;
    let half = player.size * 0.5;
    let mut target = target;
    target.x = target.x.clamp(half.x, world.size.x - half.x);
    target.y = target.y.clamp(half.y, world.size.y - half.y);
    let delta = target - start;
    let distance = delta.length();
    if distance <= 1.0e-5 {
        return start;
    }

    let start_body = Aabb::new(start, half);
    let max_t = world
        .first_body_sweep(start_body, delta, |block| {
            blink_path_blocker(player, block.kind)
        })
        .map(|hit| hit.time_of_impact)
        .unwrap_or(1.0);
    let sweep_target = start + delta * max_t;
    last_free_blink_position(world, player, start, sweep_target, half)
}

fn blink_path_blocker(player: &Player, kind: BlockKind) -> bool {
    match kind {
        BlockKind::Solid => true,
        BlockKind::BlinkWall { tier } => !player_can_blink_through(player, tier),
        BlockKind::OneWay | BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => {
            false
        }
    }
}

fn last_free_blink_position(
    world: &World,
    player: &Player,
    start: Vec2,
    target: Vec2,
    half: Vec2,
) -> Vec2 {
    let delta = target - start;
    let distance = delta.length();
    if distance <= 1.0e-5 {
        return start;
    }

    let steps = ((distance / 14.0).ceil() as usize).clamp(8, 64);
    let mut last_safe = start;
    for step in 1..=steps {
        let t = step as f32 / steps as f32;
        let candidate = start + delta * t;
        let candidate_aabb = Aabb::new(candidate, half);
        match blink_collision(world, player, candidate_aabb) {
            BlinkCollision::Free => last_safe = candidate,
            BlinkCollision::PassThrough => {}
            BlinkCollision::Blocked => break,
        }
    }
    last_safe
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlinkCollision {
    Free,
    PassThrough,
    Blocked,
}

fn blink_collision(world: &World, player: &Player, aabb: Aabb) -> BlinkCollision {
    let mut pass_through = false;
    for block in &world.blocks {
        if !aabb.strict_intersects(block.aabb) {
            continue;
        }
        match block.kind {
            BlockKind::Solid => return BlinkCollision::Blocked,
            BlockKind::BlinkWall { tier } => {
                if player_can_blink_through(player, tier) {
                    pass_through = true;
                } else {
                    return BlinkCollision::Blocked;
                }
            }
            BlockKind::OneWay => pass_through = true,
            BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => {}
        }
    }
    if pass_through {
        BlinkCollision::PassThrough
    } else {
        BlinkCollision::Free
    }
}

fn player_can_blink_through(player: &Player, tier: BlinkWallTier) -> bool {
    match tier {
        BlinkWallTier::Soft => player.abilities.blink_through_soft_walls,
        BlinkWallTier::Hard => player.abilities.blink_through_hard_walls,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{BlinkWallTier, Block};

    fn step(world: &World, player: &mut Player, input: InputState) -> FrameEvents {
        update_player_with_tuning(world, player, input, 1.0 / 60.0, DEFAULT_TUNING)
    }

    fn test_world() -> World {
        let w = 1600.0;
        let h = 900.0;
        World {
            name: "movement test world".to_string(),
            size: Vec2::new(w, h),
            spawn: Vec2::new(210.0, h - 95.0),
            blocks: vec![
                Block::solid("floor", Vec2::new(0.0, h - 48.0), Vec2::new(w, 48.0)),
                Block::solid("left wall", Vec2::new(0.0, 0.0), Vec2::new(36.0, h)),
                Block::solid("right wall", Vec2::new(w - 36.0, 0.0), Vec2::new(36.0, h)),
                Block::solid("ceiling", Vec2::new(0.0, 0.0), Vec2::new(w, 24.0)),
            ],
            objects: Vec::new(),
            water_regions: Vec::new(),
        }
    }

    #[test]
    fn tiny_dt_preserves_bullet_time_scale() {
        let world = test_world();
        let mut player = Player::new(world.spawn);
        player.on_ground = false;
        player.coyote_timer = 0.0;
        player.vel = Vec2::ZERO;
        let _ = update_player_with_tuning(
            &world,
            &mut player,
            InputState::default(),
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
        let normal_fall_speed = player.vel.y;

        let mut slow_player = Player::new(world.spawn);
        slow_player.on_ground = false;
        slow_player.coyote_timer = 0.0;
        slow_player.vel = Vec2::ZERO;
        let _ = update_player_with_tuning(
            &world,
            &mut slow_player,
            InputState::default(),
            (1.0 / 60.0) * 0.001,
            DEFAULT_TUNING,
        );

        assert!(slow_player.vel.y > 0.0);
        assert!(
            slow_player.vel.y < normal_fall_speed * 0.01,
            "tiny dt should not be clamped up to normal-ish gravity"
        );
    }

    #[test]
    fn control_clock_can_aim_blink_while_sim_clock_is_nearly_frozen() {
        let world = test_world();
        let mut player = Player::new(world.spawn);
        player.on_ground = false;
        player.coyote_timer = 0.0;
        player.vel = Vec2::ZERO;

        // Real-time control crosses the precision-blink threshold.
        for i in 0..8 {
            let _ = update_player_control_with_tuning(
                &world,
                &mut player,
                InputState {
                    axis_x: 1.0,
                    blink_pressed: i == 0,
                    blink_held: true,
                    ..Default::default()
                },
                1.0 / 60.0,
                DEFAULT_TUNING,
            );
        }
        assert!(
            player.blink_aiming,
            "control time should enter precision aim quickly"
        );

        // Game-time simulation is almost frozen, so gravity should barely change.
        let _ = update_player_simulation_with_tuning(
            &world,
            &mut player,
            InputState::default(),
            (1.0 / 60.0) * 0.000035,
            DEFAULT_TUNING,
        );
        assert!(
            player.vel.y < 0.01,
            "player gravity must use scaled game time while control remains real-time; got {}",
            player.vel.y
        );
    }

    #[test]
    fn held_blink_arms_when_cooldown_clears_without_new_press() {
        let world = test_world();
        let mut player = Player::new(world.spawn);
        player.blink_cooldown = 0.02;

        // Pressing slightly early should not arm yet.
        let _ = update_player_control_with_tuning(
            &world,
            &mut player,
            InputState {
                blink_pressed: true,
                blink_held: true,
                ..Default::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
        assert!(!player.blink_hold_active);

        // Cooldown clears in simulation time.
        let _ = update_player_simulation_with_tuning(
            &world,
            &mut player,
            InputState::default(),
            0.03,
            DEFAULT_TUNING,
        );
        assert_eq!(player.blink_cooldown, 0.0);

        // The user is still holding the button, so control time can arm blink
        // without requiring another just-pressed edge.
        let _ = update_player_control_with_tuning(
            &world,
            &mut player,
            InputState {
                blink_held: true,
                ..Default::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
        assert!(player.blink_hold_active);
    }

    #[test]
    fn double_jump_ability_controls_air_jump() {
        let world = test_world();
        let mut abilities = AbilitySet::sandbox_all();
        abilities.double_jump = false;
        let mut player = Player::new_with_abilities(world.spawn, abilities);
        player.on_ground = false;
        player.coyote_timer = 0.0;
        player.air_jumps_available = 0;
        let events = step(
            &world,
            &mut player,
            InputState {
                jump_pressed: true,
                ..Default::default()
            },
        );
        assert!(!events.operations.contains(&MovementOp::DoubleJump));

        abilities.double_jump = true;
        let mut player = Player::new_with_abilities(world.spawn, abilities);
        player.on_ground = false;
        player.coyote_timer = 0.0;
        player.air_jumps_available = 1;
        let events = step(
            &world,
            &mut player,
            InputState {
                jump_pressed: true,
                ..Default::default()
            },
        );
        assert!(events.operations.contains(&MovementOp::DoubleJump));
    }

    #[test]
    fn double_dash_ability_controls_dash_charges() {
        let world = test_world();
        let mut single_dash = AbilitySet::sandbox_all();
        single_dash.double_dash = false;
        let player = Player::new_with_abilities(world.spawn, single_dash);
        assert_eq!(player.dash_charges_available, 1);

        let player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        assert_eq!(player.dash_charges_available, 2);
    }

    #[test]
    fn wall_climb_requires_wall_cling() {
        let mut abilities = AbilitySet::sandbox_all();
        abilities.wall_cling = false;
        assert!(abilities
            .compatibility_warnings()
            .iter()
            .any(|w| w.contains("wall_climb")));
    }

    #[test]
    fn blink_ability_gates_teleport() {
        let world = test_world();
        let mut abilities = AbilitySet::sandbox_all();
        abilities.blink = false;
        abilities.precision_blink = false;
        let mut player = Player::new_with_abilities(world.spawn, abilities);
        let start = player.pos;
        let input = InputState {
            axis_x: 1.0,
            blink_pressed: true,
            blink_held: true,
            ..Default::default()
        };
        let _ = update_player_control_with_tuning(
            &world,
            &mut player,
            input,
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
        let input = InputState {
            axis_x: 1.0,
            blink_released: true,
            ..Default::default()
        };
        let events = update_player_control_with_tuning(
            &world,
            &mut player,
            input,
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
        assert_eq!(player.pos, start);
        assert!(events.blinks.is_empty());
    }

    #[test]
    fn quick_blink_moves_on_release() {
        let world = test_world();
        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        let start = player.pos;
        step(
            &world,
            &mut player,
            InputState {
                axis_x: 1.0,
                blink_pressed: true,
                blink_held: true,
                ..Default::default()
            },
        );
        let events = step(
            &world,
            &mut player,
            InputState {
                axis_x: 1.0,
                blink_released: true,
                ..Default::default()
            },
        );
        assert!(player.pos.x > start.x + 20.0);
        assert_eq!(events.blinks.len(), 1);
        assert!(!events.blinks[0].precision);
        assert!(events.operations.contains(&MovementOp::Blink));
    }

    #[test]
    fn held_blink_enters_precision_aiming() {
        let world = test_world();
        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        for _ in 0..20 {
            let blink_pressed = !player.blink_hold_active;
            step(
                &world,
                &mut player,
                InputState {
                    axis_x: 1.0,
                    blink_held: true,
                    blink_pressed,
                    ..Default::default()
                },
            );
        }
        assert!(player.blink_aiming);
        let events = step(
            &world,
            &mut player,
            InputState {
                axis_x: 1.0,
                blink_released: true,
                ..Default::default()
            },
        );
        assert_eq!(events.blinks.len(), 1);
        assert!(events.blinks[0].precision);
        assert!(events.operations.contains(&MovementOp::PrecisionBlink));
    }

    #[test]
    fn one_way_platform_requires_down_plus_jump_to_drop_through() {
        let mut world = test_world();
        // One-way platform suspended above the floor. Player will land on it
        // from above and we expect plain "down" alone to keep them resting.
        let plat_top_y = 600.0;
        world.blocks.push(Block::one_way(
            "drop test platform",
            Vec2::new(360.0, plat_top_y),
            Vec2::new(180.0, 12.0),
        ));

        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        player.pos = Vec2::new(450.0, plat_top_y - player.size.y * 0.5);
        player.vel = Vec2::ZERO;
        player.on_ground = false;

        // Settle onto the platform.
        for _ in 0..6 {
            step(&world, &mut player, InputState::default());
        }
        assert!(player.on_ground, "player should land on the one-way");
        let resting_y = player.pos.y;

        // Holding down alone must NOT drop through anymore.
        for _ in 0..6 {
            step(
                &world,
                &mut player,
                InputState {
                    axis_y: 1.0,
                    ..Default::default()
                },
            );
        }
        assert!(
            (player.pos.y - resting_y).abs() < 1.0,
            "down-alone must not drop through one-way (moved {} px)",
            player.pos.y - resting_y
        );

        // Down + jump (with the explicit drop_through_pressed gesture) drops.
        // Critically the gesture only fires for one frame: the presentation
        // layer recomputes drop_through_pressed each frame from
        // `axis_y > 0.35 && jump_pressed`, and `jump_pressed` is just-pressed,
        // so subsequent frames see drop_through_pressed=false. The engine must
        // latch the drop-through internally for long enough to clear the
        // landing-tolerance band.
        step(
            &world,
            &mut player,
            InputState {
                axis_y: 1.0,
                jump_pressed: true,
                drop_through_pressed: true,
                ..Default::default()
            },
        );
        for _ in 0..10 {
            step(
                &world,
                &mut player,
                InputState {
                    axis_y: 1.0,
                    // jump_pressed and drop_through_pressed are NOT held: this
                    // is exactly the input shape the sandbox produces after
                    // the initial press.
                    ..Default::default()
                },
            );
        }
        assert!(
            player.pos.y > resting_y + 12.0,
            "down+jump should drop the player below the one-way (delta {})",
            player.pos.y - resting_y
        );
    }

    #[test]
    fn glide_caps_fall_speed_while_jump_held() {
        let world = test_world();
        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        player.on_ground = false;
        // Drop the player into free fall well above any contact, with
        // velocity already above the glide cap so the cap clamp is the
        // only thing that can pull it back down.
        player.pos = Vec2::new(world.spawn.x, world.spawn.y - 600.0);
        player.vel = Vec2::new(0.0, 800.0);

        let events = step(
            &world,
            &mut player,
            InputState {
                jump_held: true,
                ..Default::default()
            },
        );
        let _ = events; // unused

        assert!(player.gliding, "hold-jump while falling should engage glide");
        assert!(
            player.vel.y <= DEFAULT_TUNING.glide_fall_speed + 1.0,
            "glide cap should clamp fall speed; got {}",
            player.vel.y
        );
        assert!(
            player.vel.y < DEFAULT_TUNING.max_fall_speed * 0.5,
            "glide cap must be markedly below max_fall_speed; got {}",
            player.vel.y
        );
    }

    #[test]
    fn glide_disengages_when_jump_released() {
        let world = test_world();
        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        player.on_ground = false;
        player.pos = Vec2::new(world.spawn.x, world.spawn.y - 600.0);
        player.vel = Vec2::new(0.0, 800.0);

        // Frame 1: held → glide engages
        step(
            &world,
            &mut player,
            InputState {
                jump_held: true,
                ..Default::default()
            },
        );
        assert!(player.gliding);

        // Frame 2: released → glide disengages, fall speed climbs back
        // toward max_fall_speed (gravity reapplied without the glide cap)
        step(
            &world,
            &mut player,
            InputState::default(),
        );
        assert!(!player.gliding);
    }

    #[test]
    fn glide_requires_ability_flag() {
        let world = test_world();
        let mut abilities = AbilitySet::sandbox_all();
        abilities.glide = false;
        let mut player = Player::new_with_abilities(world.spawn, abilities);
        player.on_ground = false;
        player.pos = Vec2::new(world.spawn.x, world.spawn.y - 600.0);
        player.vel = Vec2::new(0.0, 800.0);

        step(
            &world,
            &mut player,
            InputState {
                jump_held: true,
                ..Default::default()
            },
        );
        assert!(
            !player.gliding,
            "glide should not engage when the ability flag is off"
        );
    }

    #[test]
    fn fast_fall_requires_double_tap_signal() {
        let world = test_world();
        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        player.on_ground = false;
        player.vel.y = 0.0;

        // Holding down is still useful for pogo / downward attack intent, but
        // should not automatically trigger fast-fall.
        step(
            &world,
            &mut player,
            InputState {
                axis_y: 1.0,
                ..Default::default()
            },
        );
        assert!(!player.fast_falling);

        // The presentation layer recognizes double-tap-down and sends this
        // explicit event to the engine.
        step(
            &world,
            &mut player,
            InputState {
                axis_y: 1.0,
                fast_fall_pressed: true,
                ..Default::default()
            },
        );
        assert!(player.fast_falling);
    }

    #[test]
    fn repeated_blinks_clamp_downward_velocity_each_time() {
        let world = test_world();
        let mut player = Player::new(world.spawn);
        player.pos = Vec2::new(420.0, 620.0);

        for _ in 0..2 {
            player.vel = Vec2::new(25.0, 900.0);
            player.blink_cooldown = 0.0;
            player.blink_hold_active = true;
            player.blink_aiming = false;
            let events = update_player_with_tuning(
                &world,
                &mut player,
                InputState {
                    axis_x: 1.0,
                    blink_released: true,
                    ..Default::default()
                },
                1.0 / 60.0,
                DEFAULT_TUNING,
            );
            assert_eq!(events.blinks.len(), 1);
            assert!(
                player.vel.y
                    <= DEFAULT_TUNING.blink_max_downward_speed
                        + DEFAULT_TUNING.gravity / 60.0
                        + 1.0,
                "blink should not preserve a large downward fall speed; got {}",
                player.vel.y
            );
            assert!(player.blink_grace_timer > 0.0);
        }
    }

    #[test]
    fn post_blink_grace_suspends_gravity_for_tiny_window() {
        let world = test_world();
        let mut player = Player::new(world.spawn);
        player.pos = Vec2::new(420.0, 620.0);
        player.vel = Vec2::new(0.0, 900.0);
        player.blink_hold_active = true;
        let _events = update_player_with_tuning(
            &world,
            &mut player,
            InputState {
                axis_x: 1.0,
                blink_released: true,
                ..Default::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
        let after_blink_vy = player.vel.y;
        let _events = update_player_with_tuning(
            &world,
            &mut player,
            InputState::default(),
            1.0 / 240.0,
            DEFAULT_TUNING,
        );
        assert!(
            player.vel.y <= after_blink_vy + 0.1,
            "gravity should be suspended during the short post-blink grace window"
        );
    }

    #[test]
    fn blink_walls_can_be_passed_by_upgrade_without_allowing_solid_walls() {
        let mut world = test_world();
        world.blocks.clear();
        world.blocks.push(Block::blink_wall(
            "test soft blink membrane",
            Vec2::new(220.0, 0.0),
            Vec2::new(22.0, 300.0),
            BlinkWallTier::Soft,
        ));

        let mut blocked_abilities = AbilitySet::basic();
        blocked_abilities.blink = true;
        let blocked_player = Player::new_with_abilities(Vec2::new(140.0, 140.0), blocked_abilities);
        let blocked = blink_destination_to_point(&world, &blocked_player, Vec2::new(340.0, 140.0));
        assert!(blocked.x < 220.0);

        let mut pass_abilities = blocked_abilities;
        pass_abilities.blink_through_soft_walls = true;
        let pass_player = Player::new_with_abilities(Vec2::new(140.0, 140.0), pass_abilities);
        let passed = blink_destination_to_point(&world, &pass_player, Vec2::new(340.0, 140.0));
        assert!(passed.x > 300.0);
    }

    #[test]
    fn fly_toggle_switches_mode_and_counters_gravity() {
        let world = test_world();
        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        assert!(!player.fly_enabled);
        let events = step(
            &world,
            &mut player,
            InputState {
                fly_toggle_pressed: true,
                ..Default::default()
            },
        );
        assert!(player.fly_enabled);
        assert!(events.operations.contains(&MovementOp::FlyToggle));
        player.on_ground = false;
        player.vel = Vec2::ZERO;
        step(
            &world,
            &mut player,
            InputState {
                axis_y: -1.0,
                ..Default::default()
            },
        );
        assert!(
            player.vel.y < 0.0,
            "flying upward input should accelerate upward"
        );
    }

    /// A successful pogo bounce records the orb's AABB on `FrameEvents`,
    /// so the sandbox can route damage to a matching breakable pogo orb.
    #[test]
    fn pogo_bounce_records_orb_aabb_on_frame_events() {
        let mut world = test_world();
        let orb_center = Vec2::new(700.0, 600.0);
        world.blocks.push(Block::pogo_orb("orb", orb_center, 18.0));

        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        // Place the player just above the orb so a downward pogo press hits it.
        player.pos = Vec2::new(orb_center.x, orb_center.y - 24.0);
        player.vel = Vec2::ZERO;
        player.on_ground = false;

        let events = update_player_control_with_tuning(
            &world,
            &mut player,
            InputState {
                pogo_pressed: true,
                control_dt: 1.0 / 60.0,
                ..Default::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );
        assert!(
            events.operations.contains(&MovementOp::Pogo),
            "expected MovementOp::Pogo to fire, got {:?}",
            events.operations
        );
        assert_eq!(events.pogo_hits.len(), 1, "{:?}", events.pogo_hits);
        let hit = events.pogo_hits[0];
        let dx = (hit.center().x - orb_center.x).abs();
        let dy = (hit.center().y - orb_center.y).abs();
        assert!(
            dx < 1.0 && dy < 1.0,
            "pogo_hit center {:?} != orb {:?}",
            hit.center(),
            orb_center
        );
    }

    /// Wall-jumping off the left wall while the player's body slightly
    /// overlaps a wide horizontal block (floor/ceiling) must not catapult
    /// the player out the opposite side of the room.
    ///
    /// Reproduction in the square_arena: player is wall-clinging the left
    /// wall low enough that their feet still poke into the floor block.
    /// `resolve_axis(Axis::X)` saw the residual floor overlap and tried to
    /// resolve it *horizontally* — the floor block spans the whole room,
    /// so its left edge is at x=0, which produced a single-frame push
    /// equal to the negative of the player's right edge (~58 pixels left)
    /// and dumped the player at negative x.
    #[test]
    fn wall_jump_does_not_catapult_through_left_wall() {
        let world = test_world();
        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());

        // Park the player against the left wall with a tiny overlap into the
        // floor (1 pixel deep) — the kind of residual penetration the engine
        // tolerates between sweeps.
        let body = player.aabb();
        let left_wall_right = 36.0;
        let floor_top = world.size.y - 48.0;
        player.pos.x = left_wall_right + body.half_size().x; // touching wall on its right edge
        player.pos.y = floor_top - body.half_size().y + 1.0; // bottom 1 px below floor top
        player.vel = Vec2::ZERO;
        player.on_ground = false;
        player.on_wall = true;
        player.wall_normal_x = 1.0;
        player.coyote_timer = 0.0;

        let initial_x = player.pos.x;
        let _ = update_player_with_tuning(
            &world,
            &mut player,
            InputState {
                axis_x: -1.0,
                axis_y: 0.0,
                jump_pressed: true,
                jump_held: true,
                control_dt: 1.0 / 60.0,
                ..Default::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );

        // After one wall-jump frame the player should be drifting *right*
        // (away from the wall) or at worst still touching it — never past
        // the wall's right edge in the negative-x direction by tens of
        // pixels.
        assert!(
            player.pos.x >= initial_x - 1.0,
            "wall jump pushed player to x={} from x={} — expected to stay near or right of starting position",
            player.pos.x,
            initial_x,
        );
        assert!(
            player.pos.x - body.half_size().x >= 0.0,
            "wall jump punched the player through the left wall (body left = {})",
            player.pos.x - body.half_size().x,
        );
    }

    /// Closer match to the actual reported bug: the player has a tiny
    /// residual penetration into the left wall (sub-pixel rounding from
    /// the previous frame's snap) and is moving away from it on
    /// wall-jump. The horizontal sweep finds the wall at ToI=0; the snap
    /// uses delta direction (+x → "block is to my right") and pushes the
    /// player through the wall by `wall.left() - body.right() = -63`.
    #[test]
    fn wall_jump_does_not_catapult_player_off_wall_overlap() {
        let world = test_world();
        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        let body = player.aabb();
        let left_wall_right = 36.0;
        // Body penetrates wall by 1 px on the x-axis, mid-height of the
        // room (no floor/ceiling overlap to confuse the issue).
        player.pos.x = left_wall_right + body.half_size().x - 1.0;
        player.pos.y = world.size.y * 0.5;
        player.vel = Vec2::new(500.0, -650.0); // wall-jump initial velocities
        player.on_ground = false;
        player.on_wall = false;
        player.wall_normal_x = 0.0;

        let initial_x = player.pos.x;
        let _ = update_player_simulation_with_tuning(
            &world,
            &mut player,
            InputState {
                axis_x: -1.0,
                control_dt: 1.0 / 60.0,
                ..Default::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );

        // After one frame the player should be sitting at body.left ≈
        // wall.right (the wall snap aligns them), or at most a few pixels
        // to the right (motion delta). They must not be teleported by
        // tens of pixels in any direction.
        let dx = (player.pos.x - initial_x).abs();
        assert!(
            dx < 30.0,
            "wall overlap caused horizontal teleport: dx={dx}, pos.x went from {initial_x} to {}",
            player.pos.x,
        );
        assert!(
            player.pos.x - body.half_size().x >= 0.0 - 0.5,
            "player was punched through the left wall: body left = {}",
            player.pos.x - body.half_size().x,
        );
    }

    /// Regression: reproduces the wall-cling → Grounded teleport captured
    /// in `debug_traces/ambition_trace_1777903935-558508824-000000_*.json`.
    /// The player wall-clings on a tall left-side wall (top at world y=0,
    /// bottom at world's floor) and slides downward at `wall_slide_speed`.
    /// Before the fix, the y-axis sweep would return `time_of_impact = 0`
    /// on the wall (the body was edge-touching / fractionally penetrating
    /// it), then unconditionally snap the body's bottom to the wall's TOP
    /// edge — teleporting the player ~1700 px upward to
    /// `y = 0 - half_height = -23`.
    ///
    /// The fix filters dominantly-horizontal overlaps out of the y-sweep
    /// and adds the symmetric guard to `resolve_vertical`. After the fix
    /// the player either stays roughly where they were (continuing the
    /// wall slide) or moves by at most one frame's worth of velocity.
    #[test]
    fn wall_cling_does_not_teleport_to_wall_top_on_y_sweep() {
        let world = test_world();
        // Wall-cling pose: edge-touching left wall (wall.right = 36),
        // mid-room vertically, with wall_slide_speed downward.
        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        let half = player.size * 0.5;
        let wall_right = 36.0;
        // 0.05 px penetration into the wall — within the kind of float
        // fuzz that survives between the x-sweep and the y-sweep.
        player.pos.x = wall_right + half.x - 0.05;
        player.pos.y = world.size.y * 0.5; // ~450, well inside the room
        player.vel = Vec2::new(0.0, DEFAULT_TUNING.wall_slide_speed);
        player.on_ground = false;
        player.on_wall = true;
        player.wall_normal_x = 1.0;
        player.wall_clinging = true;

        let initial_y = player.pos.y;
        let _ = update_player_simulation_with_tuning(
            &world,
            &mut player,
            InputState {
                axis_x: -1.0, // pressing into the wall
                control_dt: 1.0 / 60.0,
                ..Default::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );

        // Hard invariant: after one sim step the y position must still be
        // inside the world envelope, and the y delta must be bounded by
        // the velocity-budget plus a small slop. The pre-fix behavior
        // teleported to y ≈ -23 (about 470 px above start); the post-fix
        // behavior should be |dy| < 50 px.
        assert!(
            player.pos.y >= 0.0 && player.pos.y <= world.size.y,
            "wall-cling y-sweep teleported player out of the world envelope: pos.y = {} (world.size.y = {})",
            player.pos.y,
            world.size.y,
        );
        let dy = (player.pos.y - initial_y).abs();
        assert!(
            dy < 50.0,
            "wall-cling y-sweep moved player by {dy} px in one frame; expected at most a few pixels of slide",
        );
        // The player must not have transitioned to Grounded against a
        // surface that doesn't exist at this y. The bug snapped the body
        // bottom to the wall's TOP (y=0) and set on_ground=true.
        assert!(
            !player.on_ground,
            "wall-cling y-sweep falsely set on_ground; player was supposedly grounded at y={}",
            player.pos.y,
        );
    }

    /// Guards against `body_is_side_contact` being too broad. Player
    /// descending onto the *top corner* of a tall solid (a pillar) with
    /// slight x overlap should still resolve as a normal landing —
    /// `on_ground = true`, `pos.y` snaps so `body.bottom = pillar.top`.
    /// If this test ever starts failing, the side-contact filter has
    /// expanded into legitimate vertical-landing geometry.
    #[test]
    fn descending_onto_top_corner_of_tall_block_lands_normally() {
        // World with a tall pillar centered horizontally.
        let world = World {
            name: "pillar".into(),
            size: Vec2::new(800.0, 600.0),
            spawn: Vec2::new(50.0, 50.0),
            blocks: vec![Block::solid(
                "pillar",
                Vec2::new(380.0, 200.0),
                Vec2::new(40.0, 400.0),
            )],
            objects: Vec::new(),
            water_regions: Vec::new(),
        };
        // Pillar AABB: (380, 200) → (420, 600). Top = 200, bottom = 600.
        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        // Position player so body slightly overlaps the pillar on x and is
        // about to land on its top: body x range covers ~[380-14+5, 380+5+14)
        // = [371, 405) with player half-width 14. With pos.x = 391,
        // body.left = 377 < pillar.left = 380, body.right = 405 > 380 →
        // x overlap of 25 px. body.top is well above pillar.top, body.bottom
        // is just above pillar.top.
        player.pos = Vec2::new(391.0, 200.0 - 23.0 - 0.5);
        // Falling straight down at a typical mid-arc speed.
        player.vel = Vec2::new(0.0, 200.0);
        player.on_ground = false;
        player.on_wall = false;

        let _ = update_player_simulation_with_tuning(
            &world,
            &mut player,
            InputState {
                control_dt: 1.0 / 60.0,
                ..Default::default()
            },
            1.0 / 60.0,
            DEFAULT_TUNING,
        );

        let body = player.aabb();
        assert!(
            player.on_ground,
            "descending onto pillar top should land (on_ground = true); got pos={:?}",
            player.pos
        );
        // body.bottom should be at or extremely near the pillar's top.
        assert!(
            (body.bottom() - 200.0).abs() < 1.0,
            "body.bottom should snap to pillar.top = 200; got {} (pos.y = {})",
            body.bottom(),
            player.pos.y,
        );
    }

    /// Direct unit test of `body_is_side_contact`. Both `sweep_player_y`
    /// and `resolve_vertical` consult it to avoid the wall-cling teleport
    /// class. The first revision used `overlap_x > 0` and missed the
    /// exact-edge-touching case captured in
    /// `debug_traces/ambition_trace_1777905256-*.json`; the predicate
    /// now keys on the body's y-range being nested inside the block's
    /// y-range, which catches edge-touching and penetrating side
    /// contacts uniformly.
    #[test]
    fn body_is_side_contact_classifies_walls_vs_floors() {
        // Player about to land on a wide floor: body.top < floor.top,
        // so body's y-range is NOT nested inside floor's y-range. Not
        // a side contact.
        let body = Aabb::new(Vec2::new(50.0, 100.0), Vec2::new(14.0, 23.0));
        let floor = Aabb::new(Vec2::new(80.0, 125.0), Vec2::new(60.0, 6.0));
        assert!(
            !body_is_side_contact(body, floor),
            "player about to land on a wide floor must NOT be classified as a side contact"
        );

        // Tall left wall, body fully alongside it (body's y-range is
        // strictly inside the wall's y-range). Edge-touching on x.
        // Side contact regardless of x-overlap.
        let wall = Aabb::new(Vec2::new(18.0, 450.0), Vec2::new(18.0, 450.0));
        let body_alongside_edge = Aabb::new(Vec2::new(36.0 + 14.0, 450.0), Vec2::new(14.0, 23.0));
        assert!(
            body_is_side_contact(body_alongside_edge, wall),
            "body alongside a tall wall (edge-touching on x) must be a side contact"
        );

        // Same wall, body penetrating by 1 px on x. Still alongside on y.
        let body_inside_wall =
            Aabb::new(Vec2::new(36.0 + 14.0 - 1.0, 450.0), Vec2::new(14.0, 23.0));
        assert!(
            body_is_side_contact(body_inside_wall, wall),
            "body penetrating a tall wall on x is still a side contact"
        );

        // Player landing on the top corner of a tall block (small x
        // overlap, body.bottom near block.top, body.top above block.top).
        // The body's y-range is NOT nested inside the block's y-range
        // (body.top < block.top), so this is a real vertical contact —
        // NOT a side contact. Guards against the predicate becoming too
        // broad.
        let pillar = Aabb::new(Vec2::new(900.0, 800.0), Vec2::new(40.0, 200.0));
        let body_landing_on_pillar = Aabb::new(
            Vec2::new(900.0 - 40.0 + 5.0, 600.0 - 23.0 + 1.0),
            Vec2::new(14.0, 23.0),
        );
        assert!(
            !body_is_side_contact(body_landing_on_pillar, pillar),
            "descending onto the top edge of a tall block (slight x overlap, body.top above block.top) must NOT be classified as a side contact"
        );

        // Player jumping up into a thick ceiling block (body.bottom
        // crossing block.bottom from below). body.bottom > block.bottom
        // → not nested → real vertical contact.
        let ceiling = Aabb::new(Vec2::new(900.0, 200.0), Vec2::new(400.0, 100.0));
        let body_under_ceiling =
            Aabb::new(Vec2::new(900.0, 300.0 + 23.0 - 1.0), Vec2::new(14.0, 23.0));
        assert!(
            !body_is_side_contact(body_under_ceiling, ceiling),
            "rising into a thick ceiling (body.bottom poking past block.bottom) must NOT be classified as a side contact"
        );
    }
}
