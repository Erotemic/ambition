#![cfg(feature = "rl_sim")]
//! Jon, 2026-10-23: "when flying things die they fall up. They should fall
//! down. at normal gravity."
//!
//! A flyer is authored with no gravity (`ActorSurfaceState::gravity_scale` 0).
//! A dead body falls by the full gravity of its room, so a death blow that
//! sends a flyer up only delays its fall.

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::frame_env::ResolvedMotionFrame;
use bevy::prelude::*;

/// A European swallow of `primeval_run`, killed in the air with a launch up,
/// is lower one second later and moves down.
#[test]
fn a_flyer_killed_in_the_air_falls_down() {
    let mut sim = fixed_60hz_room_sim("primeval_run");
    sim.step_n(base(), 30);
    let world = sim.world_mut();
    let swallow = world
        .query::<(Entity, &WornCharacter, &BodyHealth, &ae::ActorSurfaceState)>()
        .iter(world)
        .find(|(_, worn, health, surface)| {
            worn.id() == "european_swallow" && health.alive() && surface.gravity_scale <= 0.001
        })
        .map(|(entity, ..)| entity)
        .expect("premise: `primeval_run` places a live European swallow that flies");
    let down = world.get::<ResolvedMotionFrame>(swallow).expect("a frame").down();
    // The kill, and the launch up that a death blow can give.
    let mut health = world.get_mut::<BodyHealth>(swallow).expect("health");
    let max = health.max();
    health.damage(max);
    let mut kin = world.get_mut::<ae::BodyKinematics>(swallow).expect("a body");
    kin.vel = -down * 300.0;
    let killed_at = kin.pos;

    sim.step_n(base(), 60);
    let world = sim.world();
    assert!(!world.get::<BodyHealth>(swallow).expect("health").alive(), "premise: the swallow stays dead");
    let kin = world.get::<ae::BodyKinematics>(swallow).expect("the dead swallow is still a body");
    let fell = (kin.pos - killed_at).dot(down);
    assert!(fell > 4.0, "a dead swallow killed with a launch up is {fell} px below where it died, one second later");
    assert!(kin.vel.dot(down) >= 0.0, "a dead swallow still rises one second after it died: vel {:?}", kin.vel);
}
