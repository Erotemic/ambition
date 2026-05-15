use crate::abilities::AbilitySet;
use crate::geometry::Aabb;
use crate::Vec2;

use super::{ComboMark, MovementOp, MovementTuning, BLINK_DISTANCE, DEFAULT_TUNING};

/// Default standing movement collider width in world pixels.
///
/// Keep this authoritative for gameplay; presentation code may render a
/// larger placeholder sprite around this body while art is still temporary.
pub const DEFAULT_PLAYER_BODY_WIDTH: f32 = 30.0;
/// Default standing movement collider height in world pixels.
pub const DEFAULT_PLAYER_BODY_HEIGHT: f32 = 48.0;

/// Default standing movement collider size.
pub fn default_player_body_size() -> Vec2 {
    Vec2::new(DEFAULT_PLAYER_BODY_WIDTH, DEFAULT_PLAYER_BODY_HEIGHT)
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
    /// Sandbox-side scratch flag: was the player riding the
    /// moving-platform last frame? Used by the diagnostic log in
    /// `app.rs` that prints riding-state transitions for chasing the
    /// "glitchy platform behavior" repro. Engine itself doesn't read
    /// or write this; it lives on `Player` so it survives the
    /// `runtime.reset` field-by-field copy without needing a parallel
    /// store on `SandboxRuntime`.
    pub was_riding_platform: bool,
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
    /// Cached climbable-surface contact for this frame. Set by sandbox
    /// systems from `World::climbable_at(player.aabb)` immediately
    /// before / inside the gameplay loop (mirroring how
    /// `water_contact` is populated). Movement does not yet consume
    /// this -- the contact is exposed for sandbox-side gameplay
    /// systems (input gestures that toggle climbing, sprite swaps,
    /// HUD readouts) and for the RL/headless adapter's
    /// `AgentObservation`. Full `BodyMode::Climbing` integration is a
    /// follow-up.
    pub climbable_contact: Option<crate::world::ClimbableContact>,
    /// Engine-owned ledge hang / pull-up state. `update_player_simulation` owns
    /// this state so ledge grabs participate in the same collision pipeline as
    /// wall contact, water, and gravity instead of being corrected by a later
    /// sandbox system.
    pub ledge_grab: Option<crate::ledge_grab::LedgeGrabState>,
    /// Remaining seconds of dodge-roll invulnerability. > 0 means the
    /// player is currently rolling and should not take contact damage.
    pub dodge_roll_timer: f32,
    /// Cooldown before the next dodge roll may begin.
    pub dodge_roll_cooldown: f32,
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
            size: default_player_body_size(),
            base_size: default_player_body_size(),
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
            was_riding_platform: false,
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
            climbable_contact: None,
            ledge_grab: None,
            dodge_roll_timer: 0.0,
            dodge_roll_cooldown: 0.0,
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

    pub(super) fn spend_dash_charge(&mut self) -> MovementOp {
        let before = self.dash_charges_available;
        self.dash_charges_available = self.dash_charges_available.saturating_sub(1);
        self.dash_available = self.dash_charges_available > 0;
        if before >= 2 {
            MovementOp::DoubleDash
        } else {
            MovementOp::Dash
        }
    }

    pub(super) fn record(&mut self, op: MovementOp) {
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
        let Some(b) = self.combo.iter().next_back().map(|m| m.op) else {
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
