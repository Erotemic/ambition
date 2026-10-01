//! Volley: Attack while holding the volley fires a fan of bolts along the aim
//! from the body's edge. Driven bodies only. Migrated from the native system
//! (fast-iteration I7).

use ambition_combat_port::{WieldedUsePort, Wielder};
use ambition_extension_sdk::{Fault, Invocation, ModuleDescriptor, Port};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

use crate::wielded;

/// The held item's id.
pub const ITEM: &str = "volley";

const MANA_COST: f32 = 18.0;
const SHOT_COUNT: usize = 5;
const SPREAD_DEG: f32 = 40.0;
const SPEED: f32 = 460.0;
const DAMAGE: i32 = 2;
const LIFETIME: f32 = 1.6;
const HALF: Vec2 = Vec2::new(8.0, 8.0);

pub fn module() -> ModuleDescriptor {
    wielded::module("volley", ITEM, wielded::requests(ProjectileSpawnPort::KEY), 2 + SHOT_COUNT as u32, fire)
}

/// The fan's origin, body-local: just outside the body along the aim.
pub fn origin_local(aim_local: Vec2, body_size: Vec2) -> Vec2 {
    let dir = aim_local.normalize_or_zero();
    if dir == Vec2::ZERO {
        return Vec2::ZERO;
    }
    let half = body_size * 0.5;
    let extent = half.x * dir.x.abs() + half.y * dir.y.abs();
    dir * (extent + 8.0)
}

/// One bolt of the volley, as the game fires it (speed, damage, lifetime,
/// size), from `origin` along `dir`. For a fixture that wants the game's own
/// bolt rather than copied numbers.
pub fn authored_bolt(origin: Vec2, dir: Vec2) -> ProjectileSpawn {
    ProjectileSpawn {
        origin,
        dir,
        speed: SPEED,
        damage: DAMAGE,
        max_lifetime: LIFETIME,
        half_extent: HALF,
        gravity: 0.0,
        visual_id: String::new(),
        bounces: 0,
        bounce_on_world_contact: false,
        splash_half_extent: 0.0,
        boomerang_return_s: None,
    }
}

fn fire(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let w: Wielder = inv.trigger::<WieldedUsePort>()?.clone();
    if !w.driven || !wielded::pay(inv, &w, MANA_COST)? {
        return Ok(());
    }
    let aim = Vec2::from(w.to_world(w.aim_local)).normalize_or_zero();
    // Paid, and no direction to fire in: as the native volley, the mana is
    // spent and nothing flies.
    if aim == Vec2::ZERO {
        return Ok(());
    }
    let base_angle = aim.y.atan2(aim.x);
    let origin = Vec2::from(w.position)
        + Vec2::from(w.to_world(origin_local(Vec2::from(w.aim_local), Vec2::from(w.size)).into()));
    let spread = SPREAD_DEG.to_radians();
    for i in 0..SHOT_COUNT {
        let t = if SHOT_COUNT > 1 { i as f32 / (SHOT_COUNT - 1) as f32 - 0.5 } else { 0.0 };
        let angle = base_angle + t * spread;
        inv.submit::<ProjectileSpawnPort>(authored_bolt(origin, Vec2::new(angle.cos(), angle.sin())))?;
    }
    wielded::rock_hit(inv, &w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fan_starts_just_outside_the_body_along_the_aim() {
        let size = Vec2::new(24.0, 40.0);
        assert_eq!(origin_local(Vec2::new(1.0, 0.0), size), Vec2::new(20.0, 0.0));
        assert_eq!(origin_local(Vec2::new(0.0, -1.0), size), Vec2::new(0.0, -28.0));
        assert_eq!(origin_local(Vec2::ZERO, size), Vec2::ZERO);
    }
}
