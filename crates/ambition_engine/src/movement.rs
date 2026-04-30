//! Player movement simulation.
//!
//! This module contains the code that makes the current prototype feel like a
//! platformer: coyote time, buffered jumps, optional double jumps, optional
//! wall jumps/cling/climb, optional dash/double dash, pogo refreshes, rebound
//! pads, hazards, and a symbolic operation trace.
//!
//! The update function is intentionally renderer-free. It consumes a plain
//! `InputState`, mutates a `Player`, and returns `FrameEvents` that the Bevy
//! layer can turn into particles, hitstop, sound, or debug overlays.

use std::fmt;

use crate::abilities::AbilitySet;
use crate::geometry::Aabb;
use crate::math::{approach, Vec2};
use crate::world::{BlockKind, World};

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
    pub facing: f32,
    pub on_ground: bool,
    pub on_wall: bool,
    pub wall_normal_x: f32,
    /// Back-compat/debug convenience: true when at least one dash charge exists.
    pub dash_available: bool,
    /// Number of dash charges available before the next refresh.
    pub dash_charges_available: u8,
    pub air_jumps_available: u8,
    pub wall_clinging: bool,
    pub wall_climbing: bool,
    pub dash_timer: f32,
    pub dash_cooldown: f32,
    pub jump_buffer_timer: f32,
    pub coyote_timer: f32,
    pub rebound_cooldown: f32,
    pub combo: Vec<ComboMark>,
    pub max_speed: f32,
    pub time_alive: f32,
    pub resets: u32,
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
            facing: 1.0,
            on_ground: false,
            on_wall: false,
            wall_normal_x: 0.0,
            dash_available: dash_charges > 0,
            dash_charges_available: dash_charges,
            air_jumps_available: abilities.air_jump_count(DEFAULT_TUNING.air_jumps),
            wall_clinging: false,
            wall_climbing: false,
            dash_timer: 0.0,
            dash_cooldown: 0.0,
            jump_buffer_timer: 0.0,
            coyote_timer: 0.0,
            rebound_cooldown: 0.0,
            combo: Vec::new(),
            max_speed: 0.0,
            time_alive: 0.0,
            resets: 0,
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
            (MovementOp::Dash, MovementOp::Pogo) => "D o P: dash then pogo converts speed into height",
            (MovementOp::Pogo, MovementOp::Dash) => "P o D: pogo then dash converts height into lateral routing",
            (MovementOp::Jump, MovementOp::DoubleJump) => "J o DJ: save the second jump for route correction",
            (MovementOp::Dash, MovementOp::DoubleJump) => "D o DJ: dash then double jump recovers a bad line",
            (MovementOp::WallJump, MovementOp::Dash) => "WJ o D: wall jump then dash is a fast exit",
            (MovementOp::Dash, MovementOp::WallJump) => "D o WJ: dash into wall to bank momentum",
            (MovementOp::WallCling, MovementOp::WallClimb) => "WC o W^: cling opens vertical routing",
            (MovementOp::Rebound, MovementOp::Dash) => "R o D: launcher into dash preserves the loop",
            (MovementOp::Dash, MovementOp::Slash) => "D o S: dash slash is a commitment",
            (MovementOp::Slash, MovementOp::Dash) => "S o D: slash dash is a correction",
            (MovementOp::DoubleDash, MovementOp::DoubleJump) => "DD o DJ: spend horizontal resources before vertical recovery",
            _ => "order matters: this trace is a movement algebra sketch",
        }
    }
}

/// Backend-neutral input for one simulation frame.
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
    pub attack_pressed: bool,
    /// Dedicated downward/pogo slash action. This is separate from
    /// `attack_pressed` so layouts can expose four main face-button verbs.
    pub pogo_pressed: bool,
    pub reset_pressed: bool,
}

/// Engine events emitted by one player simulation step.
#[derive(Clone, Debug, Default)]
pub struct FrameEvents {
    pub operations: Vec<MovementOp>,
    pub reset: bool,
    pub hazard: bool,
}

impl FrameEvents {
    fn op(&mut self, player: &mut Player, op: MovementOp) {
        self.operations.push(op);
        player.record(op);
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
pub const COYOTE_TIME: f32 = 0.120;
pub const JUMP_BUFFER: f32 = 0.135;
pub const POGO_SPEED: f32 = 810.0;
pub const SLASH_RECOIL: f32 = 130.0;
pub const AIR_JUMPS: u8 = 1;

/// Tunable movement parameters.
#[derive(Clone, Copy, Debug)]
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
    pub coyote_time: f32,
    pub jump_buffer: f32,
    pub pogo_speed: f32,
    pub slash_recoil: f32,
    pub air_jumps: u8,
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
    coyote_time: COYOTE_TIME,
    jump_buffer: JUMP_BUFFER,
    pogo_speed: POGO_SPEED,
    slash_recoil: SLASH_RECOIL,
    air_jumps: AIR_JUMPS,
};

pub fn update_player(world: &World, player: &mut Player, input: InputState, raw_dt: f32) -> FrameEvents {
    update_player_with_tuning(world, player, input, raw_dt, DEFAULT_TUNING)
}

/// Advance the player simulation by one frame.
///
/// `raw_dt` is clamped so a slow or paused frame does not explode collision.
/// The Bevy sandbox handles hitstop/freeze by passing `0.0` when appropriate.
pub fn update_player_with_tuning(
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
    let dt = raw_dt.clamp(1.0 / 240.0, 1.0 / 30.0);

    if input.reset_pressed && player.abilities.reset {
        player.reset_to(world.spawn);
        events.reset = true;
        return events;
    }

    age_player(player, dt);
    update_facing_and_timers(player, input, dt, tuning);
    handle_attacks(world, player, input, tuning, &mut events);
    handle_jump_buffer(player, input, tuning, &mut events);
    handle_dash(player, input, tuning, &mut events);
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
    player.combo.retain(|m| m.age < 4.0 || m.op == MovementOp::Reset);
}

fn update_facing_and_timers(player: &mut Player, input: InputState, dt: f32, tuning: MovementTuning) {
    if input.axis_x.abs() > 0.1 {
        player.facing = input.axis_x.signum();
    }

    player.jump_buffer_timer = dec(player.jump_buffer_timer, dt);
    player.coyote_timer = dec(player.coyote_timer, dt);
    player.dash_cooldown = dec(player.dash_cooldown, dt);
    player.rebound_cooldown = dec(player.rebound_cooldown, dt);

    if player.on_ground {
        player.coyote_timer = tuning.coyote_time;
        player.refresh_movement_resources(tuning);
    }

    if input.jump_pressed && player.abilities.jump {
        player.jump_buffer_timer = tuning.jump_buffer;
    }
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
        if try_pogo(world, player, tuning) {
            events.op(player, MovementOp::Pogo);
        } else {
            // Dedicated pogo whiff still gives a tiny correction so it can be
            // tested as a fourth face-button verb without requiring a target.
            player.vel.x -= player.facing * (tuning.slash_recoil * 0.45);
            events.op(player, MovementOp::Slash);
        }
    } else if input.attack_pressed {
        if can_pogo && input.axis_y > 0.25 && try_pogo(world, player, tuning) {
            events.op(player, MovementOp::Pogo);
        } else {
            // A small generated recoil/correction action. It exists to test
            // cancellability and non-commutative feel.
            player.vel.x -= player.facing * tuning.slash_recoil;
            events.op(player, MovementOp::Slash);
        }
    }
}

fn handle_jump_buffer(
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if player.jump_buffer_timer > 0.0 {
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

    // Variable jump height: releasing jump early clips upward velocity.
    if player.abilities.variable_jump && input.jump_released && player.vel.y < -120.0 {
        player.vel.y *= 0.54;
    }
}

fn handle_dash(player: &mut Player, input: InputState, tuning: MovementTuning, events: &mut FrameEvents) {
    if input.dash_pressed
        && player.abilities.dash
        && player.dash_charges_available > 0
        && player.dash_cooldown <= 0.0
    {
        let fallback = Vec2::new(player.facing, 0.0);
        let aim = Vec2::new(input.axis_x, input.axis_y).normalized_or(fallback);
        player.vel = aim * tuning.dash_speed;
        player.dash_timer = tuning.dash_time;
        player.dash_cooldown = tuning.dash_cooldown;
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
    } else {
        player.vel.y += tuning.gravity * dt;
    }

    if player.abilities.move_horizontal {
        let accel = if player.on_ground { tuning.run_accel } else { tuning.air_accel };
        let target_vx = input.axis_x * tuning.max_run_speed;
        player.vel.x = approach(player.vel.x, target_vx, accel * dt);

        let friction = if player.on_ground { tuning.ground_friction } else { tuning.air_friction };
        if input.axis_x.abs() <= 0.1 {
            player.vel.x = approach(player.vel.x, 0.0, friction * dt);
        }
    }

    player.vel.y = player.vel.y.min(tuning.max_fall_speed);

    // Resolve horizontal motion. This establishes wall contact for wall verbs.
    player.on_wall = false;
    player.wall_normal_x = 0.0;
    player.wall_climbing = false;
    let was_clinging = player.wall_clinging;
    player.wall_clinging = false;
    player.pos.x += player.vel.x * dt;
    resolve_axis(world, player, Axis::X);

    apply_wall_abilities(player, input, tuning, was_clinging, events);

    // Resolve vertical motion. Previous bottom determines one-way behavior.
    let prev_bottom = player.aabb().bottom();
    player.on_ground = false;
    player.pos.y += player.vel.y * dt;
    resolve_vertical(world, player, prev_bottom, input.axis_y > 0.35);

    if player.on_ground {
        player.refresh_movement_resources(tuning);
        player.wall_clinging = false;
        player.wall_climbing = false;
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
    let pressing_into_wall = input.axis_x.abs() > 0.1 && input.axis_x.signum() == -player.wall_normal_x;
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
        BlockKind::Solid => true,
        BlockKind::OneWay => matches!(axis, Axis::Y),
        BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => false,
    }
}

fn resolve_axis(world: &World, player: &mut Player, axis: Axis) {
    let mut aabb = player.aabb();
    for block in &world.blocks {
        if !is_solid_for_axis(block.kind, axis) || !aabb.intersects(block.aabb) {
            continue;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            continue;
        }
        match axis {
            Axis::X => {
                if aabb.center.x < block.aabb.center.x {
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
        if !is_solid_for_axis(block.kind, Axis::Y) || !aabb.intersects(block.aabb) {
            continue;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            let landing_from_above = player.vel.y >= 0.0 && prev_bottom <= block.aabb.top() + 8.0;
            if !landing_from_above || drop_through {
                continue;
            }
        }
        if aabb.center.y < block.aabb.center.y {
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

fn try_pogo(world: &World, player: &mut Player, tuning: MovementTuning) -> bool {
    let feet = player.aabb();
    let hitbox = Aabb::new(
        Vec2::new(feet.center.x, feet.bottom() + 18.0),
        Vec2::new(feet.half.x * 0.76, 22.0),
    );
    let hit = world.blocks.iter().any(|block| {
        let valid_target = matches!(block.kind, BlockKind::PogoOrb | BlockKind::Solid | BlockKind::Rebound { .. });
        valid_target && hitbox.intersects(block.aabb)
    });
    if hit {
        player.vel.y = -tuning.pogo_speed;
        player.refresh_movement_resources(tuning);
        player.on_ground = false;
    }
    hit
}

fn touching_hazard(world: &World, player: &Player) -> bool {
    let aabb = player.aabb();
    world
        .blocks
        .iter()
        .any(|b| matches!(b.kind, BlockKind::Hazard) && aabb.intersects(b.aabb))
}

fn touching_rebound(world: &World, player: &Player) -> Option<Vec2> {
    let aabb = player.aabb();
    world.blocks.iter().find_map(|b| match b.kind {
        BlockKind::Rebound { impulse } if aabb.intersects(b.aabb) => Some(impulse),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::build_endgame_sandbox;

    fn step(world: &World, player: &mut Player, input: InputState) -> FrameEvents {
        update_player_with_tuning(world, player, input, 1.0 / 60.0, DEFAULT_TUNING)
    }

    #[test]
    fn double_jump_ability_controls_air_jump() {
        let world = build_endgame_sandbox();
        let mut abilities = AbilitySet::sandbox_all();
        abilities.double_jump = false;
        let mut player = Player::new_with_abilities(world.spawn, abilities);
        player.on_ground = false;
        player.coyote_timer = 0.0;
        player.air_jumps_available = 0;
        let events = step(&world, &mut player, InputState { jump_pressed: true, ..Default::default() });
        assert!(!events.operations.contains(&MovementOp::DoubleJump));

        abilities.double_jump = true;
        let mut player = Player::new_with_abilities(world.spawn, abilities);
        player.on_ground = false;
        player.coyote_timer = 0.0;
        player.air_jumps_available = 1;
        let events = step(&world, &mut player, InputState { jump_pressed: true, ..Default::default() });
        assert!(events.operations.contains(&MovementOp::DoubleJump));
    }

    #[test]
    fn double_dash_ability_controls_dash_charges() {
        let world = build_endgame_sandbox();
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
        assert!(abilities.compatibility_warnings().iter().any(|w| w.contains("wall_climb")));
    }
}
