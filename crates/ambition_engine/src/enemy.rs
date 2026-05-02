//! Sandbox enemy/test target simulation.
//!
//! These fixtures moved from the Bevy sandbox into the engine because their
//! behavior is game logic, not rendering logic: health, stun, knockback, death,
//! respawn, and gravity should be testable without Bevy.
//!
//! They are still intentionally simple. The current goal is to test attack
//! feel and feedback, not to design final enemy AI.

use crate::geometry::{Aabb, AabbExt};
use crate::world::{BlockKind, World};
use crate::{approach, Vec2};

const DUMMY_GRAVITY: f32 = 1600.0;
const DUMMY_GROUND_FRICTION: f32 = 820.0;
const DUMMY_MAX_X_SPEED: f32 = 1400.0;
const DUMMY_MAX_FALL_SPEED: f32 = 900.0;

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

    /// Advance dummy physics and collide against a full room.
    ///
    /// This is intentionally small and conservative rather than clever. The
    /// dummy is an AABB body, so we resolve X and Y separately like the player,
    /// using Parry-backed swept casts so high knockback cannot tunnel through
    /// thin walls in a single frame.
    pub fn update_in_world(&mut self, dt: f32, world: &World) -> bool {
        let respawn_pos = Vec2::new(self.spawn.x, 88.0);
        self.update_common_timers_and_respawn(dt, respawn_pos, |dummy, dt| {
            dummy.vel.y += DUMMY_GRAVITY * dt;
            dummy.vel.x = approach(dummy.vel.x, 0.0, DUMMY_GROUND_FRICTION * dt);
            dummy.vel.x = dummy.vel.x.clamp(-DUMMY_MAX_X_SPEED, DUMMY_MAX_X_SPEED);
            dummy.vel.y = dummy.vel.y.min(DUMMY_MAX_FALL_SPEED);

            sweep_dummy_x(world, dummy, dummy.vel.x * dt);

            let prev_bottom = dummy.aabb().bottom();
            sweep_dummy_y(world, dummy, dummy.vel.y * dt, prev_bottom);

            apply_dummy_rebound(world, dummy);
        })
    }

    fn update_common_timers_and_respawn<F>(
        &mut self,
        dt: f32,
        respawn_pos: Vec2,
        mut integrate_alive: F,
    ) -> bool
    where
        F: FnMut(&mut Dummy, f32),
    {
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        if !self.alive {
            self.respawn_timer = (self.respawn_timer - dt).max(0.0);
            if self.respawn_timer <= 0.0 {
                self.alive = true;
                self.hp = self.max_hp;
                self.pos = respawn_pos;
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
        integrate_alive(self, dt);
        false
    }
}

fn dummy_collides_on_x(kind: BlockKind) -> bool {
    matches!(kind, BlockKind::Solid | BlockKind::BlinkWall { .. })
}

fn dummy_collides_on_y(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Solid | BlockKind::BlinkWall { .. } | BlockKind::OneWay
    )
}

fn sweep_fraction(time_of_impact: f32) -> f32 {
    time_of_impact.clamp(0.0, 1.0)
}

fn sweep_dummy_x(world: &World, dummy: &mut Dummy, delta_x: f32) {
    let delta = Vec2::new(delta_x, 0.0);
    if delta.x.abs() <= 1.0e-5 {
        resolve_dummy_x(world, dummy);
        return;
    }

    if let Some(hit) =
        world.first_body_sweep(dummy.aabb(), delta, |block| dummy_collides_on_x(block.kind))
    {
        dummy.pos.x += delta.x * sweep_fraction(hit.time_of_impact);
        let body = dummy.aabb();
        if delta.x > 0.0 {
            dummy.pos.x += hit.block.aabb.left() - body.right();
        } else {
            dummy.pos.x += hit.block.aabb.right() - body.left();
        }
        dummy.vel.x = 0.0;
    } else {
        dummy.pos.x += delta.x;
    }

    // Shape casts catch fast motion; positional resolution remains as a cheap
    // penetration repair for starts inside geometry or stacked contacts.
    resolve_dummy_x(world, dummy);
}

fn sweep_dummy_y(world: &World, dummy: &mut Dummy, delta_y: f32, prev_bottom: f32) {
    let delta = Vec2::new(0.0, delta_y);
    if delta.y.abs() <= 1.0e-5 {
        resolve_dummy_y(world, dummy, prev_bottom);
        return;
    }

    if let Some(hit) = world.first_body_sweep(dummy.aabb(), delta, |block| {
        if !dummy_collides_on_y(block.kind) {
            false
        } else if matches!(block.kind, BlockKind::OneWay) {
            delta.y >= 0.0 && prev_bottom <= block.aabb.top() + 8.0
        } else {
            true
        }
    }) {
        dummy.pos.y += delta.y * sweep_fraction(hit.time_of_impact);
        let body = dummy.aabb();
        if delta.y > 0.0 || body.center().y < hit.block.aabb.center().y {
            dummy.pos.y += hit.block.aabb.top() - body.bottom();
        } else {
            dummy.pos.y += hit.block.aabb.bottom() - body.top();
        }
        dummy.vel.y = 0.0;
    } else {
        dummy.pos.y += delta.y;
    }

    resolve_dummy_y(world, dummy, prev_bottom);
}

fn resolve_dummy_x(world: &World, dummy: &mut Dummy) {
    let mut body = dummy.aabb();
    for block in &world.blocks {
        if !dummy_collides_on_x(block.kind) || !body.strict_intersects(block.aabb) {
            continue;
        }
        if body.center().x < block.aabb.center().x {
            dummy.pos.x += block.aabb.left() - body.right();
        } else {
            dummy.pos.x += block.aabb.right() - body.left();
        }
        dummy.vel.x = 0.0;
        body = dummy.aabb();
    }
}

fn resolve_dummy_y(world: &World, dummy: &mut Dummy, prev_bottom: f32) {
    let mut body = dummy.aabb();
    for block in &world.blocks {
        if !dummy_collides_on_y(block.kind) || !body.strict_intersects(block.aabb) {
            continue;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            let landing_from_above = dummy.vel.y >= 0.0 && prev_bottom <= block.aabb.top() + 8.0;
            if !landing_from_above {
                continue;
            }
        }
        if body.center().y < block.aabb.center().y {
            dummy.pos.y += block.aabb.top() - body.bottom();
        } else {
            dummy.pos.y += block.aabb.bottom() - body.top();
        }
        dummy.vel.y = 0.0;
        body = dummy.aabb();
    }
}

/// Apply rebound pads to dummies as well as to the player.
///
/// Rebound pads are part of the movement sandbox's physics language, not a
/// player-only gimmick. If a launched enemy intersects one, it should read as a
/// real world object by converting the enemy's velocity too.
fn apply_dummy_rebound(world: &World, dummy: &mut Dummy) {
    let body = dummy.aabb();
    for block in &world.blocks {
        if let BlockKind::Rebound { impulse } = block.kind {
            if body.strict_intersects(block.aabb) {
                if body.center().y < block.aabb.center().y {
                    dummy.pos.y += block.aabb.top() - body.bottom();
                } else if body.center().y > block.aabb.center().y {
                    dummy.pos.y += block.aabb.bottom() - body.top();
                }
                dummy.vel = impulse;
                dummy.hit_flash = dummy.hit_flash.max(0.10);
                break;
            }
        }
    }
}

/// Default sandbox dummy layout near the player spawn.
pub fn spawn_dummies(world: &World) -> Vec<Dummy> {
    let ground_y = world.size.y - 48.0;
    vec![
        Dummy::infinite(
            "infinite sandbag",
            Vec2::new(world.spawn.x + 170.0, ground_y - 33.0),
        ),
        Dummy::finite(
            "finite drop dummy",
            Vec2::new(world.spawn.x + 300.0, ground_y - 29.0),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Block, World};

    #[test]
    fn high_speed_dummy_knockback_stops_at_wall() {
        let wall = Block::solid("test wall", Vec2::new(200.0, 0.0), Vec2::new(24.0, 200.0));
        let floor = Block::solid("test floor", Vec2::new(0.0, 130.0), Vec2::new(500.0, 24.0));
        let world = World {
            name: "dummy collision test".to_string(),
            size: Vec2::new(500.0, 200.0),
            spawn: Vec2::new(80.0, 90.0),
            blocks: vec![wall, floor],
            objects: Vec::new(),
        };
        let mut dummy = Dummy::infinite("test dummy", Vec2::new(160.0, 97.0));
        dummy.vel.x = 2500.0;
        dummy.update_in_world(1.0 / 30.0, &world);
        assert!(dummy.aabb().right() <= 200.0 + 0.01);
        assert_eq!(dummy.vel.x, 0.0);
    }

    #[test]
    fn dummy_rebound_pad_converts_enemy_velocity() {
        let pad = Block::rebound(
            "test rebound",
            Vec2::new(80.0, 130.0),
            Vec2::new(120.0, 20.0),
            Vec2::new(0.0, -700.0),
        );
        let floor = Block::solid("test floor", Vec2::new(0.0, 170.0), Vec2::new(500.0, 24.0));
        let world = World {
            name: "dummy rebound test".to_string(),
            size: Vec2::new(500.0, 220.0),
            spawn: Vec2::new(100.0, 90.0),
            blocks: vec![pad, floor],
            objects: Vec::new(),
        };
        let mut dummy = Dummy::infinite("test dummy", Vec2::new(120.0, 96.0));
        dummy.vel.y = 320.0;
        dummy.update_in_world(1.0 / 30.0, &world);
        assert!(
            dummy.vel.y < -500.0,
            "expected rebound impulse, got {:?}",
            dummy.vel
        );
    }
}
