//! Sandbox dummy/enemy targets.
//!
//! These are intentionally simple gameplay test fixtures, not final enemies.
//! Their job is to make attack, hitstop, knockback, death, and respawn feedback
//! easy to test in the endgame movement sandbox.

use ambition_engine as ae;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DummyKind {
    InfiniteSandbag,
    FiniteRespawner,
}

#[derive(Clone, Debug)]
pub struct Dummy {
    pub name: &'static str,
    pub kind: DummyKind,
    pub spawn: ae::Vec2,
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub size: ae::Vec2,
    pub hp: i32,
    pub max_hp: i32,
    pub alive: bool,
    pub respawn_timer: f32,
    pub hit_flash: f32,
    pub hit_stun: f32,
}

impl Dummy {
    pub fn infinite(name: &'static str, spawn: ae::Vec2) -> Self {
        Self {
            name,
            kind: DummyKind::InfiniteSandbag,
            spawn,
            pos: spawn,
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(38.0, 66.0),
            hp: 9999,
            max_hp: 9999,
            alive: true,
            respawn_timer: 0.0,
            hit_flash: 0.0,
            hit_stun: 0.0,
        }
    }

    pub fn finite(name: &'static str, spawn: ae::Vec2) -> Self {
        Self {
            name,
            kind: DummyKind::FiniteRespawner,
            spawn,
            pos: spawn,
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(34.0, 58.0),
            hp: 6,
            max_hp: 6,
            alive: true,
            respawn_timer: 0.0,
            hit_flash: 0.0,
            hit_stun: 0.0,
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }

    pub fn apply_hit(&mut self, damage: i32, knock_x: f32) -> bool {
        if !self.alive {
            return false;
        }
        self.hit_flash = 0.18;
        self.hit_stun = 0.075;
        self.vel.x += knock_x;
        self.vel.y = (self.vel.y - 120.0).max(-360.0);
        let mut killed = false;
        if self.kind == DummyKind::FiniteRespawner {
            self.hp -= damage;
            if self.hp <= 0 {
                self.alive = false;
                self.respawn_timer = 0.85;
                killed = true;
            }
        }
        killed
    }

    pub fn update(&mut self, dt: f32, ground_y: f32) -> bool {
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        if !self.alive {
            self.respawn_timer = (self.respawn_timer - dt).max(0.0);
            if self.respawn_timer <= 0.0 {
                self.alive = true;
                self.hp = self.max_hp;
                self.pos = ae::Vec2::new(self.spawn.x, 88.0);
                self.vel = ae::Vec2::ZERO;
                self.hit_flash = 0.24;
                self.hit_stun = 0.0;
                return true;
            }
            return false;
        }
        self.hit_stun = (self.hit_stun - dt).max(0.0);
        if self.hit_stun > 0.0 {
            return false;
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
        false
    }
}

pub fn spawn_dummies(world: &ae::World) -> Vec<Dummy> {
    let ground_y = world.size.y - 48.0;
    vec![
        Dummy::infinite("infinite sandbag", ae::Vec2::new(world.spawn.x + 170.0, ground_y - 33.0)),
        Dummy::finite("finite drop dummy", ae::Vec2::new(world.spawn.x + 300.0, ground_y - 29.0)),
    ]
}

fn approach(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}
