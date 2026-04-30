//! Sandbox enemy/test target simulation.
//!
//! These fixtures moved from the Bevy sandbox into the engine because their
//! behavior is game logic, not rendering logic: health, stun, knockback, death,
//! respawn, and gravity should be testable without Bevy.
//!
//! They are still intentionally simple. The current goal is to test attack
//! feel and feedback, not to design final enemy AI.

use crate::geometry::Aabb;
use crate::math::{approach, Vec2};
use crate::world::World;

/// First-pass target archetypes used by the movement sandbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DummyKind {
    /// Infinite health target for testing hit rhythm, recoil, pogo, and sound.
    InfiniteSandbag,
    /// Finite target that dies and drops back in after a short delay.
    FiniteRespawner,
}

/// Minimal enemy state.
#[derive(Clone, Debug)]
pub struct Dummy {
    pub name: &'static str,
    pub kind: DummyKind,
    pub spawn: Vec2,
    pub pos: Vec2,
    pub vel: Vec2,
    pub size: Vec2,
    pub hp: i32,
    pub max_hp: i32,
    pub alive: bool,
    pub respawn_timer: f32,
    pub hit_flash: f32,
    pub hit_stun: f32,
}

impl Dummy {
    pub fn infinite(name: &'static str, spawn: Vec2) -> Self {
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

    pub fn finite(name: &'static str, spawn: Vec2) -> Self {
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

    pub fn aabb(&self) -> Aabb {
        Aabb::new(self.pos, self.size * 0.5)
    }

    /// Apply attack feedback and damage.
    ///
    /// Returns `true` when this hit killed the dummy. The Bevy layer uses that
    /// to trigger death particles and sounds.
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

    /// Advance dummy physics and respawn timers.
    ///
    /// Returns `true` when a finite dummy respawned this frame.
    pub fn update(&mut self, dt: f32, ground_y: f32) -> bool {
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

/// Default sandbox dummy layout near the player spawn.
pub fn spawn_dummies(world: &World) -> Vec<Dummy> {
    let ground_y = world.size.y - 48.0;
    vec![
        Dummy::infinite("infinite sandbag", Vec2::new(world.spawn.x + 170.0, ground_y - 33.0)),
        Dummy::finite("finite drop dummy", Vec2::new(world.spawn.x + 300.0, ground_y - 29.0)),
    ]
}
