//! Ambition Engine
//!
//! A tiny, assetless, code-first movement laboratory.  The current engine is
//! intentionally simple: deterministic AABB collision, handcrafted platformer
//! feel, generated sandbox geometry, and a symbolic combo trace.  The point is
//! to make a gray-box endgame room that is fun before art, story, or assets.

use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const X: Self = Self { x: 1.0, y: 0.0 };
    pub const Y: Self = Self { x: 0.0, y: 1.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn normalized_or(self, fallback: Self) -> Self {
        let len = self.length();
        if len > 1.0e-5 {
            self / len
        } else {
            fallback
        }
    }

    pub fn clamp_length_max(self, max_len: f32) -> Self {
        let len = self.length();
        if len > max_len && len > 1.0e-5 {
            self * (max_len / len)
        } else {
            self
        }
    }

    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y
    }

    pub fn perp(self) -> Self {
        Self::new(-self.y, self.x)
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub center: Vec2,
    pub half: Vec2,
}

impl Aabb {
    pub const fn new(center: Vec2, half: Vec2) -> Self {
        Self { center, half }
    }

    pub fn from_min_size(min: Vec2, size: Vec2) -> Self {
        Self::new(min + size * 0.5, size * 0.5)
    }

    pub fn min(self) -> Vec2 {
        self.center - self.half
    }

    pub fn max(self) -> Vec2 {
        self.center + self.half
    }

    pub fn top(self) -> f32 {
        self.center.y - self.half.y
    }

    pub fn bottom(self) -> f32 {
        self.center.y + self.half.y
    }

    pub fn left(self) -> f32 {
        self.center.x - self.half.x
    }

    pub fn right(self) -> f32 {
        self.center.x + self.half.x
    }

    pub fn intersects(self, rhs: Self) -> bool {
        self.left() < rhs.right()
            && self.right() > rhs.left()
            && self.top() < rhs.bottom()
            && self.bottom() > rhs.top()
    }

    pub fn translated(self, delta: Vec2) -> Self {
        Self::new(self.center + delta, self.half)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlockKind {
    Solid,
    OneWay,
    Hazard,
    PogoOrb,
    Rebound { impulse: Vec2 },
}

#[derive(Clone, Debug)]
pub struct Block {
    pub name: &'static str,
    pub aabb: Aabb,
    pub kind: BlockKind,
}

impl Block {
    pub fn solid(name: &'static str, min: Vec2, size: Vec2) -> Self {
        Self {
            name,
            aabb: Aabb::from_min_size(min, size),
            kind: BlockKind::Solid,
        }
    }

    pub fn one_way(name: &'static str, min: Vec2, size: Vec2) -> Self {
        Self {
            name,
            aabb: Aabb::from_min_size(min, size),
            kind: BlockKind::OneWay,
        }
    }

    pub fn hazard(name: &'static str, min: Vec2, size: Vec2) -> Self {
        Self {
            name,
            aabb: Aabb::from_min_size(min, size),
            kind: BlockKind::Hazard,
        }
    }

    pub fn pogo_orb(name: &'static str, center: Vec2, radius: f32) -> Self {
        Self {
            name,
            aabb: Aabb::new(center, Vec2::new(radius, radius)),
            kind: BlockKind::PogoOrb,
        }
    }

    pub fn rebound(name: &'static str, min: Vec2, size: Vec2, impulse: Vec2) -> Self {
        Self {
            name,
            aabb: Aabb::from_min_size(min, size),
            kind: BlockKind::Rebound { impulse },
        }
    }
}

#[derive(Clone, Debug)]
pub struct World {
    pub name: &'static str,
    pub size: Vec2,
    pub spawn: Vec2,
    pub blocks: Vec<Block>,
}

/// Build the first Ambition endgame lab.  All geometry is procedural/code data;
/// there are no textures, sprites, maps, sounds, or imported assets.
pub fn build_endgame_sandbox() -> World {
    let mut blocks = Vec::new();
    let w = 1280.0;
    let h = 720.0;

    // Shell.
    blocks.push(Block::solid("floor", Vec2::new(0.0, h - 48.0), Vec2::new(w, 48.0)));
    blocks.push(Block::solid("left wall", Vec2::new(0.0, 0.0), Vec2::new(36.0, h)));
    blocks.push(Block::solid("right wall", Vec2::new(w - 36.0, 0.0), Vec2::new(36.0, h)));
    blocks.push(Block::solid("ceiling lip", Vec2::new(0.0, 0.0), Vec2::new(w, 24.0)));

    // A small playground that creates a clockwise flow loop.
    blocks.push(Block::solid("low left step", Vec2::new(145.0, 585.0), Vec2::new(170.0, 30.0)));
    blocks.push(Block::solid("left wall kick column", Vec2::new(115.0, 410.0), Vec2::new(54.0, 170.0)));
    blocks.push(Block::one_way("middle shelf", Vec2::new(375.0, 475.0), Vec2::new(230.0, 18.0)));
    blocks.push(Block::solid("upper left shelf", Vec2::new(230.0, 300.0), Vec2::new(200.0, 24.0)));
    blocks.push(Block::solid("needle pillar", Vec2::new(630.0, 395.0), Vec2::new(54.0, 235.0)));
    blocks.push(Block::one_way("high bridge", Vec2::new(710.0, 245.0), Vec2::new(250.0, 18.0)));
    blocks.push(Block::solid("right catch wall", Vec2::new(1045.0, 330.0), Vec2::new(52.0, 270.0)));
    blocks.push(Block::solid("return shelf", Vec2::new(865.0, 525.0), Vec2::new(180.0, 24.0)));

    // Intentional danger/rest/reset surface: recoverable if you are stylish.
    blocks.push(Block::hazard("spike channel", Vec2::new(465.0, 650.0), Vec2::new(245.0, 22.0)));
    blocks.push(Block::hazard("right spike channel", Vec2::new(770.0, 650.0), Vec2::new(185.0, 22.0)));

    // Pogo orbs act as refresh notes in the movement instrument.
    blocks.push(Block::pogo_orb("pogo alpha", Vec2::new(515.0, 385.0), 18.0));
    blocks.push(Block::pogo_orb("pogo beta", Vec2::new(745.0, 355.0), 18.0));
    blocks.push(Block::pogo_orb("pogo gamma", Vec2::new(975.0, 455.0), 18.0));

    // Rebound pads are explicit momentum converters.
    blocks.push(Block::rebound(
        "left launcher",
        Vec2::new(78.0, 632.0),
        Vec2::new(78.0, 20.0),
        Vec2::new(520.0, -760.0),
    ));
    blocks.push(Block::rebound(
        "right return launcher",
        Vec2::new(1100.0, 615.0),
        Vec2::new(90.0, 22.0),
        Vec2::new(-650.0, -620.0),
    ));
    blocks.push(Block::rebound(
        "ceiling redirect",
        Vec2::new(555.0, 70.0),
        Vec2::new(150.0, 18.0),
        Vec2::new(710.0, 210.0),
    ));

    World {
        name: "Ambition: Tangent Space v0",
        size: Vec2::new(w, h),
        spawn: Vec2::new(210.0, 535.0),
        blocks,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovementOp {
    Jump,
    WallJump,
    Dash,
    Pogo,
    Rebound,
    Slash,
    Reset,
}

impl MovementOp {
    pub fn symbol(self) -> &'static str {
        match self {
            MovementOp::Jump => "J",
            MovementOp::WallJump => "WJ",
            MovementOp::Dash => "D",
            MovementOp::Pogo => "P",
            MovementOp::Rebound => "R",
            MovementOp::Slash => "S",
            MovementOp::Reset => "0",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            MovementOp::Jump => "jump",
            MovementOp::WallJump => "wall jump",
            MovementOp::Dash => "dash",
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

#[derive(Clone, Debug)]
pub struct ComboMark {
    pub op: MovementOp,
    pub age: f32,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub pos: Vec2,
    pub vel: Vec2,
    pub size: Vec2,
    pub facing: f32,
    pub on_ground: bool,
    pub on_wall: bool,
    pub wall_normal_x: f32,
    pub dash_available: bool,
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
    pub fn new(spawn: Vec2) -> Self {
        Self {
            pos: spawn,
            vel: Vec2::ZERO,
            size: Vec2::new(28.0, 46.0),
            facing: 1.0,
            on_ground: false,
            on_wall: false,
            wall_normal_x: 0.0,
            dash_available: true,
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

    pub fn reset_to(&mut self, spawn: Vec2) {
        let resets = self.resets + 1;
        *self = Player::new(spawn);
        self.resets = resets;
        self.record(MovementOp::Reset);
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
            (MovementOp::WallJump, MovementOp::Dash) => "WJ o D: wall jump then dash is a fast exit",
            (MovementOp::Dash, MovementOp::WallJump) => "D o WJ: dash into wall to bank momentum",
            (MovementOp::Rebound, MovementOp::Dash) => "R o D: launcher into dash preserves the loop",
            (MovementOp::Dash, MovementOp::Slash) => "D o S: dash slash is a commitment",
            (MovementOp::Slash, MovementOp::Dash) => "S o D: slash dash is a correction",
            _ => "order matters: this trace is a movement algebra sketch",
        }
    }
}

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
    /// Dedicated downward/pogo slash action. This is separate from attack_pressed
    /// so keyboard/gamepad layouts can expose four main face-button verbs.
    pub pogo_pressed: bool,
    pub reset_pressed: bool,
}

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

pub const GRAVITY: f32 = 2350.0;
pub const RUN_ACCEL: f32 = 6200.0;
pub const AIR_ACCEL: f32 = 3800.0;
pub const GROUND_FRICTION: f32 = 7600.0;
pub const AIR_FRICTION: f32 = 720.0;
pub const MAX_RUN_SPEED: f32 = 310.0;
pub const MAX_FALL_SPEED: f32 = 980.0;
pub const JUMP_SPEED: f32 = 650.0;
pub const WALL_JUMP_X: f32 = 470.0;
pub const DASH_SPEED: f32 = 760.0;
pub const DASH_TIME: f32 = 0.115;
pub const DASH_COOLDOWN: f32 = 0.075;
pub const COYOTE_TIME: f32 = 0.105;
pub const JUMP_BUFFER: f32 = 0.130;
pub const POGO_SPEED: f32 = 780.0;
pub const SLASH_RECOIL: f32 = 115.0;

pub fn update_player(world: &World, player: &mut Player, input: InputState, raw_dt: f32) -> FrameEvents {
    let mut events = FrameEvents::default();
    if raw_dt <= 0.0 {
        return events;
    }
    let dt = raw_dt.clamp(1.0 / 240.0, 1.0 / 30.0);

    if input.reset_pressed {
        player.reset_to(world.spawn);
        events.reset = true;
        return events;
    }

    player.time_alive += dt;
    player.max_speed = player.max_speed.max(player.vel.length());
    for mark in &mut player.combo {
        mark.age += dt;
    }
    player.combo.retain(|m| m.age < 4.0 || m.op == MovementOp::Reset);

    if input.axis_x.abs() > 0.1 {
        player.facing = input.axis_x.signum();
    }

    player.jump_buffer_timer = dec(player.jump_buffer_timer, dt);
    player.coyote_timer = dec(player.coyote_timer, dt);
    player.dash_cooldown = dec(player.dash_cooldown, dt);
    player.rebound_cooldown = dec(player.rebound_cooldown, dt);

    if player.on_ground {
        player.coyote_timer = COYOTE_TIME;
        player.dash_available = true;
    }

    if input.jump_pressed {
        player.jump_buffer_timer = JUMP_BUFFER;
    }

    if input.pogo_pressed {
        if try_pogo(world, player) {
            events.op(player, MovementOp::Pogo);
        } else {
            // Dedicated pogo whiff still gives a tiny correction so it can be
            // tested as a fourth face-button verb without requiring a target.
            player.vel.x -= player.facing * (SLASH_RECOIL * 0.45);
            events.op(player, MovementOp::Slash);
        }
    } else if input.attack_pressed {
        if input.axis_y > 0.25 && try_pogo(world, player) {
            events.op(player, MovementOp::Pogo);
        } else {
            // A small generated recoil/correction action. It does not do damage yet;
            // it exists to test cancellability and non-commutative feel.
            player.vel.x -= player.facing * SLASH_RECOIL;
            events.op(player, MovementOp::Slash);
        }
    }

    if player.jump_buffer_timer > 0.0 {
        if player.on_wall && !player.on_ground {
            player.vel.x = player.wall_normal_x * WALL_JUMP_X;
            player.vel.y = -JUMP_SPEED * 0.94;
            player.on_wall = false;
            player.jump_buffer_timer = 0.0;
            player.coyote_timer = 0.0;
            events.op(player, MovementOp::WallJump);
        } else if player.on_ground || player.coyote_timer > 0.0 {
            player.vel.y = -JUMP_SPEED;
            player.on_ground = false;
            player.jump_buffer_timer = 0.0;
            player.coyote_timer = 0.0;
            events.op(player, MovementOp::Jump);
        }
    }

    if input.jump_released && player.vel.y < -120.0 {
        player.vel.y *= 0.54;
    }

    if input.dash_pressed && player.dash_available && player.dash_cooldown <= 0.0 {
        let fallback = Vec2::new(player.facing, 0.0);
        let aim = Vec2::new(input.axis_x, input.axis_y).normalized_or(fallback);
        player.vel = aim * DASH_SPEED;
        player.dash_available = false;
        player.dash_timer = DASH_TIME;
        player.dash_cooldown = DASH_COOLDOWN;
        events.op(player, MovementOp::Dash);
    }

    if player.dash_timer > 0.0 {
        player.dash_timer = dec(player.dash_timer, dt);
    } else {
        player.vel.y += GRAVITY * dt;
    }

    let accel = if player.on_ground { RUN_ACCEL } else { AIR_ACCEL };
    let target_vx = input.axis_x * MAX_RUN_SPEED;
    player.vel.x = approach(player.vel.x, target_vx, accel * dt);

    let friction = if player.on_ground { GROUND_FRICTION } else { AIR_FRICTION };
    if input.axis_x.abs() <= 0.1 {
        player.vel.x = approach(player.vel.x, 0.0, friction * dt);
    }

    player.vel.y = player.vel.y.min(MAX_FALL_SPEED);

    // Resolve horizontal motion.
    player.on_wall = false;
    player.wall_normal_x = 0.0;
    player.pos.x += player.vel.x * dt;
    resolve_axis(world, player, Axis::X, input.axis_y > 0.35);

    // Resolve vertical motion. Previous bottom determines one-way behavior.
    let prev_bottom = player.aabb().bottom();
    player.on_ground = false;
    player.pos.y += player.vel.y * dt;
    resolve_vertical(world, player, prev_bottom, input.axis_y > 0.35);

    if player.on_ground {
        player.dash_available = true;
    }

    if player.rebound_cooldown <= 0.0 {
        if let Some(impulse) = touching_rebound(world, player) {
            player.vel = impulse;
            player.dash_available = true;
            player.rebound_cooldown = 0.18;
            events.op(player, MovementOp::Rebound);
        }
    }

    if touching_hazard(world, player) || player.pos.y > world.size.y + 200.0 {
        player.reset_to(world.spawn);
        events.hazard = true;
        events.reset = true;
    }

    events
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

fn dec(value: f32, dt: f32) -> f32 {
    (value - dt).max(0.0)
}

fn approach(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}

fn is_solid_for_axis(kind: BlockKind, axis: Axis) -> bool {
    match kind {
        BlockKind::Solid => true,
        BlockKind::OneWay => matches!(axis, Axis::Y),
        BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => false,
    }
}

fn resolve_axis(world: &World, player: &mut Player, axis: Axis, _drop_through: bool) {
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

fn try_pogo(world: &World, player: &mut Player) -> bool {
    let feet = player.aabb();
    let hitbox = Aabb::new(
        Vec2::new(feet.center.x, feet.bottom() + 18.0),
        Vec2::new(feet.half.x * 0.76, 22.0),
    );
    let mut hit = false;
    for block in &world.blocks {
        let valid_target = matches!(block.kind, BlockKind::PogoOrb | BlockKind::Solid | BlockKind::Rebound { .. });
        if valid_target && hitbox.intersects(block.aabb) {
            hit = true;
            break;
        }
    }
    if hit {
        player.vel.y = -POGO_SPEED;
        player.dash_available = true;
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

/// Lightweight symbolic music plan placeholder. The executable does not play
/// music yet; these types are here to make the code-first/no-assets direction
/// explicit and give future procedural audio work a home.
pub mod music {
    #[derive(Clone, Debug)]
    pub struct Motif {
        pub name: &'static str,
        pub scale_degrees: &'static [i32],
        pub rhythm_units: &'static [u8],
    }

    pub const TANGENT_MOTIF: Motif = Motif {
        name: "tangent-space",
        scale_degrees: &[0, 2, 3, 7, 5, 3, 2, 0],
        rhythm_units: &[1, 1, 2, 1, 1, 2, 3, 5],
    };
}
