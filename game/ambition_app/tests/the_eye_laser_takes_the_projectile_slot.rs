//! An inventory ability is a variant the body wears into an action slot.
//!
//! The robot's own Projectile action is its fireball. Equipping the Eye Laser
//! from the inventory takes that slot: holding it burns a straight ray from
//! the visor, spends mana for as long as it burns and births no fireball; the
//! body is drawn in its laser row while it burns. Equipping the Fireball gives
//! the slot back. The equips go through the menu's road: a host intent that the
//! simulation applies on its tick.

#![cfg(feature = "rl_sim")]

use crate::common::{base, fixed_60hz_sim};
use ambition_app::Platformer2dSimHarness;
use ambition_platformer2d::combat::strike::{Hitbox, RayStrike};
use ambition_platformer2d::engine_core::resources::ActorResources;
use ambition_platformer2d::items::{Item, ItemEquipRequested};
use ambition_platformer2d::projectiles::ProjectileSpawnRequest;
use ambition_platformer2d::sprite_sheet::character::PinnedRow;
use bevy::prelude::{Entity, Messages, With};

fn primary(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    let mut query = world.query_filtered::<Entity, With<ambition_platformer2d::platformer::body::PrimaryBody>>();
    query.single(world).expect("one primary body")
}

fn equip(sim: &mut Platformer2dSimHarness, item: Item) {
    ambition_platformer2d::platformer::host_intents::write_host_intent(sim.world_mut(), ItemEquipRequested { item });
    sim.step_n(base(), 3);
}

/// The rays the body holds now: (direction, length).
fn rays(sim: &mut Platformer2dSimHarness, owner: Entity) -> Vec<(bevy::math::Vec2, f32)> {
    let world = sim.world_mut();
    let mut query = world.query::<(&Hitbox, &RayStrike)>();
    query
        .iter(world)
        .filter(|(hitbox, _)| hitbox.owner == owner)
        .map(|(_, ray)| (ray.dir, ray.length))
        .collect()
}

fn shots(sim: &Platformer2dSimHarness, owner: Entity) -> usize {
    sim.world()
        .resource::<Messages<ProjectileSpawnRequest>>()
        .iter_current_update_messages()
        .filter(|request| request.owner == owner)
        .count()
}

fn mana(sim: &Platformer2dSimHarness, body: Entity) -> f32 {
    sim.world()
        .get::<ActorResources>(body)
        .and_then(|bank| bank.level_of(&ambition_platformer2d::abilities::mana::MANA))
        .expect("the robot has a mana pool")
        .current
}

fn pinned(sim: &Platformer2dSimHarness, body: Entity) -> Option<Vec<String>> {
    sim.world().get::<PinnedRow>(body).filter(|row| row.is_pinned()).map(|row| row.rows.clone())
}

/// What holding the Projectile slot for `ticks` and letting go did: the most
/// rays held at once, the shots asked for, the mana spent, and the pin seen
/// while it was held.
struct Hold {
    rays: Vec<(bevy::math::Vec2, f32)>,
    shots: usize,
    spent: f32,
    pin: Option<Vec<String>>,
    pin_after: Option<Vec<String>>,
    rays_after: usize,
}

fn hold_the_projectile_slot(sim: &mut Platformer2dSimHarness, body: Entity, ticks: usize) -> Hold {
    let before = mana(sim, body);
    let mut held = Hold { rays: Vec::new(), shots: 0, spent: 0.0, pin: None, pin_after: None, rays_after: 0 };
    for tick in 0..ticks {
        sim.step(ambition_app::AgentAction { projectile: tick == 0, projectile_held: true, ..base() });
        held.shots += shots(sim, body);
        let now = rays(sim, body);
        if now.len() > held.rays.len() {
            held.rays = now;
        }
        held.pin = held.pin.take().or_else(|| pinned(sim, body));
    }
    held.spent = before - mana(sim, body);
    sim.step(ambition_app::AgentAction { projectile_released: true, ..base() });
    held.shots += shots(sim, body);
    for _ in 0..3 {
        sim.step(base());
        held.shots += shots(sim, body);
    }
    held.rays_after = rays(sim, body).len();
    held.pin_after = pinned(sim, body);
    held
}

#[test]
fn the_eye_laser_takes_the_projectile_slot_and_the_fireball_takes_it_back() {
    let mut sim = fixed_60hz_sim();
    sim.step_n(base(), 120);
    let body = primary(&mut sim);
    let facing = sim
        .world()
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(body)
        .expect("a live body")
        .facing
        .signum();

    // Control: the robot's own projectile is a fireball, and no ray.
    let fireball = hold_the_projectile_slot(&mut sim, body, 20);
    assert!(fireball.shots >= 1, "control: the robot's own Projectile slot throws its fireball");
    assert!(fireball.rays.is_empty(), "control: the fireball holds no ray");

    equip(&mut sim, Item::EyeLaser);
    // Let the bank fill again after the fireball.
    sim.step_n(base(), 240);
    let laser = hold_the_projectile_slot(&mut sim, body, 30);
    assert_eq!(laser.shots, 0, "the laser took the slot: no fireball is born");
    assert_eq!(laser.rays.len(), 1, "holding the slot holds one ray: {:?}", laser.rays);
    let (dir, length) = laser.rays[0];
    assert!(
        (dir.x - facing).abs() < 1e-3 && dir.y.abs() < 1e-3,
        "the laser goes straight along the facing ({facing}): {dir:?}"
    );
    assert!(length > 0.0, "the laser reaches out: {length}");
    assert!(laser.spent > 1.0, "half a second of burn spends mana: spent {}", laser.spent);
    assert_eq!(
        laser.pin.as_deref().and_then(|rows| rows.first()).map(String::as_str),
        Some("eye_laser"),
        "the body is drawn in its laser row while it burns"
    );
    assert_eq!(laser.rays_after, 0, "letting go ends the ray");
    assert_eq!(laser.pin_after, None, "letting go gives the body its own row back");

    equip(&mut sim, Item::Fireball);
    sim.step_n(base(), 240);
    let back = hold_the_projectile_slot(&mut sim, body, 20);
    assert!(back.shots >= 1, "the fireball took the slot back");
    assert!(back.rays.is_empty(), "and the laser is gone");
}
