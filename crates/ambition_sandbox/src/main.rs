use ambition_engine::{
    build_endgame_sandbox, update_player, Aabb, BlockKind, InputState, Player, Vec2, POGO_SPEED,
};
use macroquad::prelude as mq;

fn window_conf() -> mq::Conf {
    mq::Conf {
        window_title: "Ambition - Tangent Space Sandbox".to_string(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        sample_count: 1,
        window_resizable: true,
        ..Default::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PresetId {
    HollowKnight,
    WasdJkl,
    ArrowsQwer,
    WasdUipo,
}

#[derive(Clone, Copy, Debug)]
struct MovementKeys {
    left: mq::KeyCode,
    right: mq::KeyCode,
    up: mq::KeyCode,
    down: mq::KeyCode,
}

#[derive(Clone, Copy, Debug)]
struct ActionKeys {
    jump: mq::KeyCode,
    attack: mq::KeyCode,
    dash: mq::KeyCode,
    focus_cast: Option<mq::KeyCode>,
    quick_cast: Option<mq::KeyCode>,
    super_dash: Option<mq::KeyCode>,
    dream_nail: Option<mq::KeyCode>,
    quick_map: Option<mq::KeyCode>,
    inventory: Option<mq::KeyCode>,
    pause: mq::KeyCode,
    select_reset: mq::KeyCode,
    /// Optional sandbox-only convenience action. Hollow Knight style layouts
    /// use Down+Attack for pogo, but the chirality test layouts keep a fourth
    /// face-button verb exposed for future experiments.
    dedicated_pogo: Option<mq::KeyCode>,
}

#[derive(Clone, Copy, Debug)]
struct KeyboardPreset {
    id: PresetId,
    name: &'static str,
    movement: MovementKeys,
    actions: ActionKeys,
}

impl KeyboardPreset {
    fn presets() -> [Self; 4] {
        [
            Self::hollow_knight(),
            Self::wasd_jkl(),
            Self::arrows_qwer(),
            Self::wasd_uipo(),
        ]
    }

    fn hollow_knight() -> Self {
        Self {
            id: PresetId::HollowKnight,
            name: "Hollow Knight: arrows + Z/X/C/A/E/S/D",
            movement: MovementKeys {
                left: mq::KeyCode::Left,
                right: mq::KeyCode::Right,
                up: mq::KeyCode::Up,
                down: mq::KeyCode::Down,
            },
            actions: ActionKeys {
                jump: mq::KeyCode::Z,
                attack: mq::KeyCode::X,
                dash: mq::KeyCode::C,
                focus_cast: Some(mq::KeyCode::A),
                quick_cast: Some(mq::KeyCode::E),
                super_dash: Some(mq::KeyCode::S),
                dream_nail: Some(mq::KeyCode::D),
                quick_map: Some(mq::KeyCode::Tab),
                inventory: Some(mq::KeyCode::I),
                pause: mq::KeyCode::Escape,
                select_reset: mq::KeyCode::Delete,
                dedicated_pogo: None,
            },
        }
    }

    fn wasd_jkl() -> Self {
        Self {
            id: PresetId::WasdJkl,
            name: "Custom PC: WASD + Space/J/K/L/I/U",
            movement: MovementKeys {
                left: mq::KeyCode::A,
                right: mq::KeyCode::D,
                up: mq::KeyCode::W,
                down: mq::KeyCode::S,
            },
            actions: ActionKeys {
                jump: mq::KeyCode::Space,
                attack: mq::KeyCode::J,
                dash: mq::KeyCode::K,
                focus_cast: Some(mq::KeyCode::L),
                quick_cast: Some(mq::KeyCode::I),
                super_dash: Some(mq::KeyCode::LeftShift),
                dream_nail: Some(mq::KeyCode::U),
                quick_map: Some(mq::KeyCode::Tab),
                inventory: Some(mq::KeyCode::V),
                pause: mq::KeyCode::Escape,
                select_reset: mq::KeyCode::Delete,
                dedicated_pogo: None,
            },
        }
    }

    fn arrows_qwer() -> Self {
        Self {
            id: PresetId::ArrowsQwer,
            name: "chirality A: arrows + QWER",
            movement: MovementKeys {
                left: mq::KeyCode::Left,
                right: mq::KeyCode::Right,
                up: mq::KeyCode::Up,
                down: mq::KeyCode::Down,
            },
            actions: ActionKeys {
                jump: mq::KeyCode::Q,
                dash: mq::KeyCode::W,
                attack: mq::KeyCode::E,
                focus_cast: None,
                quick_cast: None,
                super_dash: None,
                dream_nail: None,
                quick_map: Some(mq::KeyCode::Tab),
                inventory: Some(mq::KeyCode::I),
                pause: mq::KeyCode::Escape,
                select_reset: mq::KeyCode::Delete,
                dedicated_pogo: Some(mq::KeyCode::R),
            },
        }
    }

    fn wasd_uipo() -> Self {
        Self {
            id: PresetId::WasdUipo,
            name: "chirality B: WASD + UIPO",
            movement: MovementKeys {
                left: mq::KeyCode::A,
                right: mq::KeyCode::D,
                up: mq::KeyCode::W,
                down: mq::KeyCode::S,
            },
            actions: ActionKeys {
                jump: mq::KeyCode::U,
                dash: mq::KeyCode::I,
                attack: mq::KeyCode::P,
                focus_cast: None,
                quick_cast: None,
                super_dash: None,
                dream_nail: None,
                quick_map: Some(mq::KeyCode::Tab),
                inventory: Some(mq::KeyCode::V),
                pause: mq::KeyCode::Escape,
                select_reset: mq::KeyCode::Delete,
                dedicated_pogo: Some(mq::KeyCode::O),
            },
        }
    }

    fn movement_label(&self) -> &'static str {
        match self.id {
            PresetId::HollowKnight | PresetId::ArrowsQwer => "Arrow keys",
            PresetId::WasdJkl | PresetId::WasdUipo => "WASD",
        }
    }

    fn action_label(&self) -> String {
        let mut parts = vec![
            format!("Jump {}", key_name(self.actions.jump)),
            format!("Attack {}", key_name(self.actions.attack)),
            format!("Dash {}", key_name(self.actions.dash)),
        ];
        if let Some(k) = self.actions.dedicated_pogo {
            parts.push(format!("Pogo {}", key_name(k)));
        } else {
            parts.push("Pogo Down+Attack".to_string());
        }

        let optional = [
            ("Focus", self.actions.focus_cast),
            ("QuickCast", self.actions.quick_cast),
            ("SuperDash", self.actions.super_dash),
            ("Dream", self.actions.dream_nail),
            ("Map", self.actions.quick_map),
            ("Inv", self.actions.inventory),
            ("Select", Some(self.actions.select_reset)),
        ];
        for (label, key) in optional {
            if let Some(k) = key {
                parts.push(format!("{} {}", label, key_name(k)));
            }
        }
        parts.join("  |  ")
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ControlFrame {
    axis_x: f32,
    axis_y: f32,
    jump_pressed: bool,
    jump_held: bool,
    jump_released: bool,
    dash_pressed: bool,
    attack_pressed: bool,
    pogo_pressed: bool,
    reset_pressed: bool,
    start_pressed: bool,
}

impl ControlFrame {
    fn read(preset: KeyboardPreset) -> Self {
        let mut axis_x = 0.0;
        let mut axis_y = 0.0;
        if mq::is_key_down(preset.movement.left) {
            axis_x -= 1.0;
        }
        if mq::is_key_down(preset.movement.right) {
            axis_x += 1.0;
        }
        if mq::is_key_down(preset.movement.up) {
            axis_y -= 1.0;
        }
        if mq::is_key_down(preset.movement.down) {
            axis_y += 1.0;
        }

        let select_pressed = mq::is_key_pressed(preset.actions.select_reset)
            || mq::is_key_pressed(mq::KeyCode::Delete);
        let reset_pressed = select_pressed || mq::is_key_pressed(mq::KeyCode::Backspace);

        Self {
            axis_x,
            axis_y,
            jump_pressed: mq::is_key_pressed(preset.actions.jump),
            jump_held: mq::is_key_down(preset.actions.jump),
            jump_released: mq::is_key_released(preset.actions.jump),
            dash_pressed: mq::is_key_pressed(preset.actions.dash),
            attack_pressed: mq::is_key_pressed(preset.actions.attack),
            pogo_pressed: key_pressed_opt(preset.actions.dedicated_pogo),
            reset_pressed,
            start_pressed: mq::is_key_pressed(preset.actions.pause),
        }
    }

    fn engine_input(self) -> InputState {
        InputState {
            axis_x: self.axis_x,
            axis_y: self.axis_y,
            jump_pressed: self.jump_pressed,
            jump_held: self.jump_held,
            jump_released: self.jump_released,
            dash_pressed: self.dash_pressed,
            attack_pressed: self.attack_pressed,
            pogo_pressed: self.pogo_pressed,
            reset_pressed: self.reset_pressed,
        }
    }
}

fn key_pressed_opt(key: Option<mq::KeyCode>) -> bool {
    key.map(mq::is_key_pressed).unwrap_or(false)
}

fn key_name(key: mq::KeyCode) -> &'static str {
    match key {
        mq::KeyCode::A => "A",
        mq::KeyCode::B => "B",
        mq::KeyCode::C => "C",
        mq::KeyCode::D => "D",
        mq::KeyCode::E => "E",
        mq::KeyCode::F => "F",
        mq::KeyCode::G => "G",
        mq::KeyCode::H => "H",
        mq::KeyCode::I => "I",
        mq::KeyCode::J => "J",
        mq::KeyCode::K => "K",
        mq::KeyCode::L => "L",
        mq::KeyCode::M => "M",
        mq::KeyCode::N => "N",
        mq::KeyCode::O => "O",
        mq::KeyCode::P => "P",
        mq::KeyCode::Q => "Q",
        mq::KeyCode::R => "R",
        mq::KeyCode::S => "S",
        mq::KeyCode::T => "T",
        mq::KeyCode::U => "U",
        mq::KeyCode::V => "V",
        mq::KeyCode::W => "W",
        mq::KeyCode::X => "X",
        mq::KeyCode::Y => "Y",
        mq::KeyCode::Z => "Z",
        mq::KeyCode::Left => "Left",
        mq::KeyCode::Right => "Right",
        mq::KeyCode::Up => "Up",
        mq::KeyCode::Down => "Down",
        mq::KeyCode::Space => "Space",
        mq::KeyCode::LeftShift => "LShift",
        mq::KeyCode::Tab => "Tab",
        mq::KeyCode::Escape => "Esc",
        mq::KeyCode::Delete => "Delete",
        mq::KeyCode::Backspace => "Backspace",
        _ => "?",
    }
}

/// Canonical gamepad semantic map. The current build starts with keyboard,
/// but this keeps keyboard presets aligned with expected console positions.
const GAMEPAD_MAP: &[(&str, &str)] = &[
    ("L-stick / D-pad", "movement"),
    ("A / Cross", "jump / confirm"),
    ("X / Square", "attack / nail / slash"),
    ("RT / R2", "dash"),
    ("B / Circle", "focus / cast placeholder"),
    ("RB / R1", "quick cast placeholder"),
    ("LT / L2", "super dash placeholder"),
    ("Y / Triangle", "dream nail placeholder"),
    ("LB / L1", "quick map placeholder"),
    ("Back / Touchpad", "inventory or sandbox select/reset"),
    ("Start / Options", "pause / menu"),
];

#[derive(Clone, Copy, Debug)]
struct ImpactFx {
    pos: Vec2,
    age: f32,
    duration: f32,
    radius: f32,
}

impl ImpactFx {
    fn new(pos: Vec2) -> Self {
        Self {
            pos,
            age: 0.0,
            duration: 0.24,
            radius: 12.0,
        }
    }

    fn progress(self) -> f32 {
        (self.age / self.duration).clamp(0.0, 1.0)
    }

    fn alive(self) -> bool {
        self.age < self.duration
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DummyKind {
    InfiniteSandbag,
    FiniteRespawner,
}

#[derive(Clone, Debug)]
struct Dummy {
    name: &'static str,
    kind: DummyKind,
    spawn: Vec2,
    pos: Vec2,
    vel: Vec2,
    size: Vec2,
    hp: i32,
    max_hp: i32,
    alive: bool,
    respawn_timer: f32,
    hit_flash: f32,
    hit_stun: f32,
}

impl Dummy {
    fn infinite(name: &'static str, spawn: Vec2) -> Self {
        Self {
            name,
            kind: DummyKind::InfiniteSandbag,
            spawn,
            pos: spawn,
            vel: Vec2::ZERO,
            size: Vec2::new(38.0, 66.0),
            hp: 9999,
            max_hp: 9999,
            alive: true,
            respawn_timer: 0.0,
            hit_flash: 0.0,
            hit_stun: 0.0,
        }
    }

    fn finite(name: &'static str, spawn: Vec2) -> Self {
        Self {
            name,
            kind: DummyKind::FiniteRespawner,
            spawn,
            pos: spawn,
            vel: Vec2::ZERO,
            size: Vec2::new(34.0, 58.0),
            hp: 6,
            max_hp: 6,
            alive: true,
            respawn_timer: 0.0,
            hit_flash: 0.0,
            hit_stun: 0.0,
        }
    }

    fn aabb(&self) -> Aabb {
        Aabb::new(self.pos, self.size * 0.5)
    }

    fn apply_hit(&mut self, damage: i32, knock_x: f32) {
        if !self.alive {
            return;
        }
        self.hit_flash = 0.18;
        self.hit_stun = 0.075;
        self.vel.x += knock_x;
        self.vel.y = (self.vel.y - 120.0).max(-360.0);
        if self.kind == DummyKind::FiniteRespawner {
            self.hp -= damage;
            if self.hp <= 0 {
                self.alive = false;
                self.respawn_timer = 0.85;
            }
        }
    }

    fn update(&mut self, dt: f32, ground_y: f32) {
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        if !self.alive {
            self.respawn_timer = (self.respawn_timer - dt).max(0.0);
            if self.respawn_timer <= 0.0 {
                self.alive = true;
                self.hp = self.max_hp;
                self.pos = Vec2::new(self.spawn.x, 88.0);
                self.vel = Vec2::ZERO;
                self.hit_flash = 0.24;
                self.hit_stun = 0.0;
            }
            return;
        }

        self.hit_stun = (self.hit_stun - dt).max(0.0);
        if self.hit_stun > 0.0 {
            return;
        }

        self.vel.y += 1600.0 * dt;
        self.vel.x = approach(self.vel.x, 0.0, 820.0 * dt);
        self.vel.y = self.vel.y.min(900.0);
        self.pos += self.vel * dt;

        let half_h = self.size.y * 0.5;
        if self.pos.y + half_h >= ground_y {
            self.pos.y = ground_y - half_h;
            self.vel.y = 0.0;
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let world = build_endgame_sandbox();
    let mut player = Player::new(world.spawn);
    let mut debug = true;
    let mut freeze = false;
    let mut slowmo = false;
    let mut flash_timer = 0.0f32;
    let mut hitstop_timer = 0.0f32;
    let mut impacts: Vec<ImpactFx> = Vec::new();
    let presets = KeyboardPreset::presets();
    let mut preset_index = 0usize;
    let mut preset_flash = 1.2f32;
    let mut slash_preview: Option<(Aabb, f32)> = None;
    let ground_y = world.size.y - 48.0;
    let mut dummies = vec![
        Dummy::infinite("infinite sandbag", Vec2::new(world.spawn.x + 170.0, ground_y - 33.0)),
        Dummy::finite("finite drop dummy", Vec2::new(world.spawn.x + 300.0, ground_y - 29.0)),
    ];

    loop {
        if mq::is_key_pressed(mq::KeyCode::F1) {
            debug = !debug;
        }
        if mq::is_key_pressed(mq::KeyCode::F9) {
            preset_index = (preset_index + presets.len() - 1) % presets.len();
            preset_flash = 1.2;
        }
        if mq::is_key_pressed(mq::KeyCode::F10) {
            preset_index = (preset_index + 1) % presets.len();
            preset_flash = 1.2;
        }
        if mq::is_key_pressed(mq::KeyCode::F2) {
            slowmo = !slowmo;
        }

        let preset = presets[preset_index];
        let controls = ControlFrame::read(preset);
        if controls.start_pressed {
            freeze = !freeze;
        }

        let frame_dt = mq::get_frame_time();
        hitstop_timer = (hitstop_timer - frame_dt).max(0.0);
        for fx in &mut impacts {
            fx.age += frame_dt;
        }
        impacts.retain(|fx| fx.alive());

        let dt = if freeze || hitstop_timer > 0.0 {
            0.0
        } else if slowmo {
            frame_dt * 0.25
        } else {
            frame_dt
        };

        let events = update_player(&world, &mut player, controls.engine_input(), dt);
        if events.reset || !events.operations.is_empty() {
            flash_timer = 0.12;
        }

        if controls.attack_pressed || controls.pogo_pressed {
            let attack = slash_hitbox(&player, controls.axis_y, controls.pogo_pressed);
            slash_preview = Some((attack, 0.10));
            let mut landed = false;
            for dummy in &mut dummies {
                if dummy.alive && attack.intersects(dummy.aabb()) {
                    let hit_pos = Vec2::new(
                        (attack.center.x + dummy.pos.x) * 0.5,
                        (attack.center.y + dummy.pos.y) * 0.5,
                    );
                    impacts.push(ImpactFx::new(hit_pos));
                    dummy.apply_hit(1, player.facing * 300.0);
                    landed = true;
                }
            }
            if landed {
                hitstop_timer = 0.055;
                flash_timer = 0.16;
            }
            if landed && (controls.pogo_pressed || controls.axis_y > 0.25) {
                player.vel.y = -POGO_SPEED;
                player.dash_available = true;
            }
        }

        for dummy in &mut dummies {
            dummy.update(dt, world.size.y - 48.0);
        }
        if let Some((_, timer)) = &mut slash_preview {
            *timer -= frame_dt;
            if *timer <= 0.0 {
                slash_preview = None;
            }
        }

        flash_timer = (flash_timer - frame_dt).max(0.0);
        preset_flash = (preset_flash - frame_dt).max(0.0);

        draw(
            &world,
            &player,
            &dummies,
            slash_preview,
            &impacts,
            preset,
            debug,
            slowmo,
            freeze,
            hitstop_timer,
            flash_timer,
            preset_flash,
        );
        mq::next_frame().await;
    }
}

fn slash_hitbox(player: &Player, axis_y: f32, forced_pogo: bool) -> Aabb {
    let body = player.aabb();
    if forced_pogo || axis_y > 0.25 {
        Aabb::new(
            Vec2::new(body.center.x, body.bottom() + 24.0),
            Vec2::new(body.half.x * 0.95, 26.0),
        )
    } else if axis_y < -0.25 {
        Aabb::new(
            Vec2::new(body.center.x, body.top() - 22.0),
            Vec2::new(body.half.x * 1.10, 24.0),
        )
    } else {
        Aabb::new(
            Vec2::new(body.center.x + player.facing * (body.half.x + 30.0), body.center.y - 2.0),
            Vec2::new(34.0, 24.0),
        )
    }
}

fn draw(
    world: &ambition_engine::World,
    player: &Player,
    dummies: &[Dummy],
    slash_preview: Option<(Aabb, f32)>,
    impacts: &[ImpactFx],
    preset: KeyboardPreset,
    debug: bool,
    slowmo: bool,
    freeze: bool,
    hitstop: f32,
    flash: f32,
    preset_flash: f32,
) {
    let bg = mq::Color::new(0.020, 0.024, 0.035, 1.0);
    mq::clear_background(bg);

    let scale = (mq::screen_width() / world.size.x).min(mq::screen_height() / world.size.y);
    let offset = mq::vec2(
        (mq::screen_width() - world.size.x * scale) * 0.5,
        (mq::screen_height() - world.size.y * scale) * 0.5,
    );

    draw_grid(world, scale, offset);

    for block in &world.blocks {
        draw_block(block, scale, offset);
    }

    for dummy in dummies {
        draw_dummy(dummy, scale, offset);
    }

    if let Some((hitbox, _)) = slash_preview {
        draw_aabb_lines(hitbox, scale, offset, 2.0, mq::Color::new(1.0, 1.0, 0.35, 0.90));
    }

    for fx in impacts {
        draw_impact_fx(*fx, scale, offset);
    }

    draw_player(player, scale, offset, flash);

    if debug {
        draw_debug(world, player, dummies, preset, slowmo, freeze, hitstop, scale, offset);
    } else {
        mq::draw_text("F1 debug", 16.0, 28.0, 20.0, mq::GRAY);
    }

    if preset_flash > 0.0 {
        let alpha = (preset_flash / 1.2).min(1.0);
        let text = format!("preset: {}", preset.name);
        let w = mq::measure_text(&text, None, 28, 1.0).width;
        mq::draw_rectangle(
            mq::screen_width() * 0.5 - w * 0.5 - 20.0,
            42.0,
            w + 40.0,
            44.0,
            mq::Color::new(0.03, 0.04, 0.07, 0.72 * alpha),
        );
        mq::draw_text(
            &text,
            mq::screen_width() * 0.5 - w * 0.5,
            72.0,
            28.0,
            mq::Color::new(0.85, 0.95, 1.0, alpha),
        );
    }
}

fn draw_grid(world: &ambition_engine::World, scale: f32, offset: mq::Vec2) {
    let minor = mq::Color::new(0.09, 0.10, 0.14, 0.55);
    let major = mq::Color::new(0.14, 0.15, 0.21, 0.75);
    let mut x = 0.0;
    while x <= world.size.x {
        let p0 = w2s(Vec2::new(x, 0.0), scale, offset);
        let p1 = w2s(Vec2::new(x, world.size.y), scale, offset);
        let color = if ((x / 128.0).round() - x / 128.0).abs() < 0.01 { major } else { minor };
        mq::draw_line(p0.x, p0.y, p1.x, p1.y, 1.0, color);
        x += 32.0;
    }
    let mut y = 0.0;
    while y <= world.size.y {
        let p0 = w2s(Vec2::new(0.0, y), scale, offset);
        let p1 = w2s(Vec2::new(world.size.x, y), scale, offset);
        let color = if ((y / 128.0).round() - y / 128.0).abs() < 0.01 { major } else { minor };
        mq::draw_line(p0.x, p0.y, p1.x, p1.y, 1.0, color);
        y += 32.0;
    }
}

fn draw_block(block: &ambition_engine::Block, scale: f32, offset: mq::Vec2) {
    let min = block.aabb.min();
    let max = block.aabb.max();
    let p = w2s(min, scale, offset);
    let size = mq::vec2((max.x - min.x) * scale, (max.y - min.y) * scale);

    match block.kind {
        BlockKind::Solid => {
            let fill = mq::Color::new(0.20, 0.24, 0.32, 1.0);
            let line = mq::Color::new(0.46, 0.55, 0.75, 1.0);
            mq::draw_rectangle(p.x, p.y, size.x, size.y, fill);
            mq::draw_rectangle_lines(p.x, p.y, size.x, size.y, 2.0, line);
        }
        BlockKind::OneWay => {
            let fill = mq::Color::new(0.18, 0.28, 0.30, 0.82);
            let line = mq::Color::new(0.35, 0.85, 0.75, 1.0);
            mq::draw_rectangle(p.x, p.y, size.x, size.y, fill);
            mq::draw_line(p.x, p.y, p.x + size.x, p.y, 3.0, line);
        }
        BlockKind::Hazard => {
            let fill = mq::Color::new(0.42, 0.07, 0.11, 1.0);
            let line = mq::Color::new(1.0, 0.22, 0.28, 1.0);
            mq::draw_rectangle(p.x, p.y, size.x, size.y, fill);
            let spikes = ((size.x / 18.0).max(1.0)) as i32;
            for i in 0..spikes {
                let x0 = p.x + i as f32 * size.x / spikes as f32;
                let x1 = p.x + (i + 1) as f32 * size.x / spikes as f32;
                let xm = (x0 + x1) * 0.5;
                mq::draw_triangle(
                    mq::vec2(x0, p.y),
                    mq::vec2(x1, p.y),
                    mq::vec2(xm, p.y - 16.0 * scale),
                    line,
                );
            }
        }
        BlockKind::PogoOrb => {
            let c = w2s(block.aabb.center, scale, offset);
            let r = block.aabb.half.x * scale;
            mq::draw_circle(c.x, c.y, r * 1.08, mq::Color::new(0.06, 0.38, 0.42, 0.80));
            mq::draw_circle_lines(c.x, c.y, r * 1.12, 3.0, mq::Color::new(0.34, 0.96, 1.0, 1.0));
            mq::draw_line(c.x - r * 0.65, c.y, c.x + r * 0.65, c.y, 2.0, mq::WHITE);
            mq::draw_line(c.x, c.y - r * 0.65, c.x, c.y + r * 0.65, 2.0, mq::WHITE);
        }
        BlockKind::Rebound { impulse } => {
            let fill = mq::Color::new(0.40, 0.23, 0.06, 1.0);
            let line = mq::Color::new(1.0, 0.70, 0.22, 1.0);
            mq::draw_rectangle(p.x, p.y, size.x, size.y, fill);
            mq::draw_rectangle_lines(p.x, p.y, size.x, size.y, 2.0, line);
            let center = w2s(block.aabb.center, scale, offset);
            let dir = mq::vec2(impulse.x, impulse.y).normalize_or_zero();
            let end = center + dir * 42.0 * scale;
            mq::draw_line(center.x, center.y, end.x, end.y, 3.0, line);
            let side = mq::vec2(-dir.y, dir.x);
            mq::draw_triangle(end, end - dir * 10.0 * scale + side * 6.0 * scale, end - dir * 10.0 * scale - side * 6.0 * scale, line);
        }
    }
}

fn draw_impact_fx(fx: ImpactFx, scale: f32, offset: mq::Vec2) {
    let t = fx.progress();
    let c = w2s(fx.pos, scale, offset);
    let radius = (fx.radius + 32.0 * t) * scale;
    let alpha = 1.0 - t;
    mq::draw_circle_lines(
        c.x,
        c.y,
        radius,
        2.0,
        mq::Color::new(1.0, 0.94, 0.42, 0.85 * alpha),
    );
    mq::draw_line(
        c.x - radius * 0.55,
        c.y,
        c.x + radius * 0.55,
        c.y,
        2.0,
        mq::Color::new(1.0, 1.0, 0.80, 0.70 * alpha),
    );
    mq::draw_line(
        c.x,
        c.y - radius * 0.55,
        c.x,
        c.y + radius * 0.55,
        2.0,
        mq::Color::new(1.0, 1.0, 0.80, 0.70 * alpha),
    );
}

fn draw_dummy(dummy: &Dummy, scale: f32, offset: mq::Vec2) {
    if !dummy.alive {
        let respawn = format!("{} respawn {:.1}", dummy.name, dummy.respawn_timer);
        let p = w2s(dummy.spawn, scale, offset);
        mq::draw_text(&respawn, p.x - 52.0 * scale, p.y - 84.0 * scale, 16.0 * scale, mq::GRAY);
        return;
    }

    let aabb = dummy.aabb();
    let min = w2s(aabb.min(), scale, offset);
    let max = w2s(aabb.max(), scale, offset);
    let w = max.x - min.x;
    let h = max.y - min.y;
    let fill = match dummy.kind {
        DummyKind::InfiniteSandbag => mq::Color::new(0.46, 0.33, 0.19, 1.0),
        DummyKind::FiniteRespawner => mq::Color::new(0.42, 0.25, 0.42, 1.0),
    };
    let flash = mq::Color::new(1.0, 0.96, 0.70, 1.0);
    mq::draw_rectangle(min.x, min.y, w, h, if dummy.hit_flash > 0.0 { flash } else { fill });
    mq::draw_rectangle_lines(min.x, min.y, w, h, 2.0, mq::Color::new(0.05, 0.04, 0.03, 1.0));
    mq::draw_line(min.x, min.y + h * 0.36, max.x, min.y + h * 0.36, 1.5, mq::Color::new(0.08, 0.06, 0.05, 1.0));

    let label = match dummy.kind {
        DummyKind::InfiniteSandbag => "sandbag INF",
        DummyKind::FiniteRespawner => "dummy",
    };
    mq::draw_text(label, min.x - 12.0, min.y - 10.0, 15.0 * scale, mq::Color::new(0.86, 0.88, 0.98, 1.0));

    if dummy.kind == DummyKind::FiniteRespawner {
        let ratio = (dummy.hp.max(0) as f32 / dummy.max_hp as f32).clamp(0.0, 1.0);
        mq::draw_rectangle(min.x, min.y - 23.0 * scale, w, 5.0 * scale, mq::Color::new(0.12, 0.07, 0.10, 1.0));
        mq::draw_rectangle(min.x, min.y - 23.0 * scale, w * ratio, 5.0 * scale, mq::Color::new(0.88, 0.36, 0.72, 1.0));
    }
}

fn draw_player(player: &Player, scale: f32, offset: mq::Vec2, flash: f32) {
    let aabb = player.aabb();
    let min = w2s(aabb.min(), scale, offset);
    let max = w2s(aabb.max(), scale, offset);
    let w = max.x - min.x;
    let h = max.y - min.y;
    let body = if player.dash_timer > 0.0 {
        mq::Color::new(0.98, 0.93, 0.42, 1.0)
    } else if flash > 0.0 {
        mq::Color::new(1.0, 1.0, 1.0, 1.0)
    } else if player.dash_available {
        mq::Color::new(0.74, 0.82, 1.0, 1.0)
    } else if player.air_jumps_available > 0 {
        mq::Color::new(0.66, 0.80, 0.72, 1.0)
    } else {
        mq::Color::new(0.52, 0.58, 0.72, 1.0)
    };
    mq::draw_rectangle(min.x, min.y, w, h, body);
    mq::draw_rectangle_lines(min.x, min.y, w, h, 2.0, mq::Color::new(0.05, 0.08, 0.14, 1.0));

    let eye = if player.facing >= 0.0 { max.x - 8.0 * scale } else { min.x + 8.0 * scale };
    mq::draw_circle(eye, min.y + 13.0 * scale, 3.0 * scale, mq::BLACK);

    let center = w2s(player.pos, scale, offset);
    let v = mq::vec2(player.vel.x, player.vel.y) * 0.10 * scale;
    mq::draw_line(center.x, center.y, center.x + v.x, center.y + v.y, 2.0, mq::Color::new(1.0, 0.72, 0.25, 1.0));
}

fn draw_debug(
    world: &ambition_engine::World,
    player: &Player,
    dummies: &[Dummy],
    preset: KeyboardPreset,
    slowmo: bool,
    freeze: bool,
    hitstop: f32,
    scale: f32,
    offset: mq::Vec2,
) {
    let panel = mq::Color::new(0.02, 0.03, 0.05, 0.78);
    mq::draw_rectangle(10.0, 10.0, 960.0, 264.0, panel);
    mq::draw_rectangle_lines(10.0, 10.0, 960.0, 264.0, 1.0, mq::Color::new(0.3, 0.4, 0.55, 1.0));

    let speed = player.vel.length();
    let mode = if freeze {
        "PAUSED"
    } else if hitstop > 0.0 {
        "HITSTOP"
    } else if slowmo {
        "SLOWMO"
    } else {
        "LIVE"
    };
    let finite_hp = dummies
        .iter()
        .find(|d| d.kind == DummyKind::FiniteRespawner)
        .map(|d| if d.alive { format!("{}/{}", d.hp.max(0), d.max_hp) } else { format!("respawn {:.1}", d.respawn_timer) })
        .unwrap_or_else(|| "-".to_string());
    let lines = [
        format!("{}  |  {}  |  {}", world.name, mode, preset.name),
        format!("Move: {}  |  Actions: {}", preset.movement_label(), preset.action_label()),
        "F9 previous preset, F10 next preset, Esc/Start pause, Delete/Select reset".to_string(),
        "Gamepad plan: A jump, X attack, RT dash; B/RB/LT/Y/LB placeholders".to_string(),
        "Toggles: F1 debug, F2 slow motion".to_string(),
        format!("pos=({:.1}, {:.1}) vel=({:.1}, {:.1}) speed={:.1} max={:.1}", player.pos.x, player.pos.y, player.vel.x, player.vel.y, speed, player.max_speed),
        format!(
            "ground={} wall={} dash={} air_jumps={} coyote={:.2} buffer={:.2} resets={} finite_dummy={}",
            player.on_ground,
            player.on_wall,
            player.dash_available,
            player.air_jumps_available,
            player.coyote_timer,
            player.jump_buffer_timer,
            player.resets,
            finite_hp
        ),
        format!("combo: {}", player.combo_symbols()),
        format!("hint: {}", player.current_combo_hint()),
    ];

    for (i, line) in lines.iter().enumerate() {
        mq::draw_text(line, 22.0, 34.0 + i as f32 * 24.0, 18.0, mq::Color::new(0.85, 0.90, 1.0, 1.0));
    }

    // Pogo hitbox preview.
    let feet = player.aabb();
    let hit_center = Vec2::new(feet.center.x, feet.bottom() + 18.0);
    let hit_half = Vec2::new(feet.half.x * 0.76, 22.0);
    let hit_min = w2s(hit_center - hit_half, scale, offset);
    mq::draw_rectangle_lines(hit_min.x, hit_min.y, hit_half.x * 2.0 * scale, hit_half.y * 2.0 * scale, 1.0, mq::Color::new(0.32, 0.90, 1.0, 0.65));

    let mut y = 296.0;
    mq::draw_text("Control semantics:", 18.0, y, 17.0, mq::Color::new(0.68, 0.78, 0.95, 1.0));
    y += 20.0;
    for (button, action) in GAMEPAD_MAP.iter().take(9) {
        mq::draw_text(&format!("{} = {}", button, action), 18.0, y, 15.0, mq::Color::new(0.58, 0.64, 0.76, 1.0));
        y += 18.0;
    }
}

fn draw_aabb_lines(aabb: Aabb, scale: f32, offset: mq::Vec2, thickness: f32, color: mq::Color) {
    let min = w2s(aabb.min(), scale, offset);
    let max = w2s(aabb.max(), scale, offset);
    mq::draw_rectangle_lines(min.x, min.y, max.x - min.x, max.y - min.y, thickness, color);
}

fn w2s(v: Vec2, scale: f32, offset: mq::Vec2) -> mq::Vec2 {
    mq::vec2(offset.x + v.x * scale, offset.y + v.y * scale)
}

fn approach(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}
