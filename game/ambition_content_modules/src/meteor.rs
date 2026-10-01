//! Meteor: Attack while holding the meteor drops a line of falling rocks on a
//! zone ahead of the body, across its gravity. Driven bodies only. Migrated
//! from the native system (fast-iteration I7).

use ambition_combat_port::{WieldedUsePort, Wielder};
use ambition_extension_sdk::{Fault, Invocation, ModuleDescriptor, Port};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

use crate::wielded;

/// The held item's id.
pub const ITEM: &str = "meteor";

const MANA_COST: f32 = 32.0;
const COUNT: usize = 5;
const RANGE: f32 = 190.0;
const SPREAD: f32 = 220.0;
const DROP_HEIGHT: f32 = 270.0;
const SPEED: f32 = 140.0;
const GRAVITY: f32 = 950.0;
const DAMAGE: i32 = 2;
const LIFETIME: f32 = 2.0;
const HALF: Vec2 = Vec2::new(9.0, 9.0);

pub fn module() -> ModuleDescriptor {
    wielded::module("meteor", ITEM, wielded::requests(ProjectileSpawnPort::KEY), 2 + COUNT as u32, drop)
}

/// Where each rock starts: above a zone `RANGE` ahead of the body (by the aim,
/// else the facing), spread across the gravity.
pub fn origins(position: Vec2, aim_local: Vec2, facing: f32, gravity_down: Vec2) -> [Vec2; COUNT] {
    let down = gravity_down.try_normalize().unwrap_or(Vec2::new(0.0, 1.0));
    let side = Vec2::new(down.y, -down.x);
    let to_world = |local: Vec2| side * local.x + down * local.y;
    let dir_x = if aim_local.x.abs() > 0.001 { aim_local.x.signum() } else { facing.signum() };
    let zone = position + to_world(Vec2::new(dir_x * RANGE, 0.0));
    let spawn_center = zone + to_world(Vec2::new(0.0, -DROP_HEIGHT));
    let mut out = [Vec2::ZERO; COUNT];
    for (i, slot) in out.iter_mut().enumerate() {
        let frac = (i as f32) / ((COUNT - 1) as f32) - 0.5;
        *slot = spawn_center + to_world(Vec2::new(frac * SPREAD, 0.0));
    }
    out
}

fn drop(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let w: Wielder = inv.trigger::<WieldedUsePort>()?.clone();
    if !w.driven || !wielded::pay(inv, &w, MANA_COST)? {
        return Ok(());
    }
    let down = Vec2::from(w.frame_down);
    for origin in origins(Vec2::from(w.position), Vec2::from(w.aim_local), w.facing, down) {
        inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
            origin,
            dir: down,
            speed: SPEED,
            damage: DAMAGE,
            max_lifetime: LIFETIME,
            half_extent: HALF,
            gravity: GRAVITY,
            visual_id: String::new(),
            bounces: 0,
            bounce_on_world_contact: false,
            splash_half_extent: 0.0,
            boomerang_return_s: None,
        })?;
    }
    wielded::rock_hit(inv, &w)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rocks keep their body-local layout whichever way gravity points.
    #[test]
    fn the_drop_is_the_same_in_every_gravity_frame() {
        let at = Vec2::new(100.0, 100.0);
        let local = |origin: Vec2, down: Vec2| {
            let side = Vec2::new(down.y, -down.x);
            let d = origin - at;
            Vec2::new(d.dot(side), d.dot(down))
        };
        let reference = origins(at, Vec2::new(1.0, 0.0), 1.0, Vec2::new(0.0, 1.0));
        for down in [Vec2::new(1.0, 0.0), Vec2::new(0.0, -1.0), Vec2::new(-1.0, 0.0)] {
            for (r, o) in reference.iter().zip(origins(at, Vec2::new(1.0, 0.0), 1.0, down)) {
                assert!((local(*r, Vec2::new(0.0, 1.0)) - local(o, down)).length() < 1e-3);
            }
        }
        // Above the body (against gravity) and ahead of it.
        assert!(reference.iter().all(|o| o.y < at.y && o.x > at.x - SPREAD));
    }
}
