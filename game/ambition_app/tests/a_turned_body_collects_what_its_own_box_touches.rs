//! In the shipped game, a body in turned gravity collects the pickup its own
//! collision box touches.
//!
//! A body that is not square lies along its gravity: in gravity toward +x the
//! player's box is as wide as the player is tall. The readers outside the
//! movement kernel (a pickup, a chest, an interact, a door) asked the LEVEL
//! box, so the player collected a heart beside her and not a heart past her
//! end. They ask `BodyKinematics::collision_box` now: the body's present
//! position and size turned to the DOWN of its last step.
//!
//! Most of those readers were converted on the strength of that one function.
//! This arm is the function witnessed in production, through one real reader
//! (`collect_ecs_pickups`), with the kernel writing the record.

#![cfg(feature = "rl_sim")]

use ambition_app::Platformer2dSimHarness;
use ambition_platformer2d::combat::components::{CenteredAabb, Collected, FeatureName};
use ambition_platformer2d::engine_core::{Aabb, BodyKinematics, SweepSample, Vec2};
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::With;

use crate::common::{base, fixed_60hz_room_sim};

const ROOM: &str = "basement_breakables";
/// The two hearts the room authors.
const BESIDE_HER: &str = "regrowing heart";
const PAST_HER_END: &str = "one-time heart";

/// (the box, collected) of the pickup `name`.
fn heart(sim: &mut Platformer2dSimHarness, name: &str) -> (Aabb, bool) {
    let world = sim.world_mut();
    let mut q = world.query::<(&FeatureName, &CenteredAabb, Option<&Collected>)>();
    q.iter(world)
        .find(|(feature, _, _)| feature.0.as_str() == name)
        .map(|(_, aabb, collected)| (Aabb::new(aabb.center, aabb.half_size), collected.is_some()))
        .unwrap_or_else(|| panic!("the room authors the pickup '{name}'"))
}

fn body(sim: &mut Platformer2dSimHarness) -> (BodyKinematics, SweepSample) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(&BodyKinematics, &SweepSample), With<PrimaryPlayer>>();
    let (kin, record) = q.single(world).expect("the player's body is in the world, with its record");
    (*kin, *record)
}

fn place_the_player(sim: &mut Platformer2dSimHarness, at: Vec2) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut BodyKinematics, With<PrimaryPlayer>>();
    let mut kin = q.single_mut(world).expect("the player's body is in the world");
    kin.pos = at;
    kin.vel = Vec2::ZERO;
}

fn overlaps(a: Aabb, b: Aabb) -> bool {
    a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
}

/// Put the player `offset` from the centre of the pickup `name`, step one
/// tick, and say what the two boxes of the body answer and whether the pickup
/// was collected.
struct Outcome {
    turned_box_touches: bool,
    level_box_touches: bool,
    collected: bool,
}

fn one_tick_at(sim: &mut Platformer2dSimHarness, name: &str, offset: Vec2) -> Outcome {
    let (pickup, already) = heart(sim, name);
    assert!(!already, "premise: '{name}' is not collected yet");
    let centre = (pickup.min + pickup.max) * 0.5;
    place_the_player(sim, centre + offset);
    sim.step(base());
    let (kin, record) = body(sim);
    assert!(
        (record.down - Vec2::new(1.0, 0.0)).length() < 1.0e-3,
        "premise: the kernel stepped the player in gravity toward +x: {record:?}"
    );
    let level = kin.size * 0.5;
    let turned = Vec2::new(level.y, level.x);
    Outcome {
        turned_box_touches: overlaps(Aabb::new(kin.pos, turned), pickup),
        level_box_touches: overlaps(Aabb::new(kin.pos, level), pickup),
        collected: heart(sim, name).1,
    }
}

#[test]
fn the_player_in_sideways_gravity_collects_the_heart_her_own_box_touches() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    for _ in 0..30 {
        sim.step(base());
    }
    {
        let world = sim.world_mut();
        let room = ambition_platformer2d::session::sole_live_room_component::<
            ambition_platformer2d::world::rooms::LiveRoomInstance,
        >(world)
        .copied();
        world
            .resource_mut::<ambition_platformer2d::world::BaseGravity>()
            .turn(room, Vec2::new(1.0, 0.0));
    }
    sim.step(base());
    let (kin, _) = body(&mut sim);
    let level = kin.size * 0.5;
    assert!((level.x - level.y).abs() > 4.0, "premise: the player is not square: {:?}", kin.size);
    // Half way between the reach of the short side and the reach of the long
    // side: one box touches the pickup and the other does not.
    let between = (level.x + level.y) * 0.5;

    let (pickup, _) = heart(&mut sim, BESIDE_HER);
    let pickup_half = (pickup.max - pickup.min) * 0.5;
    let beside = one_tick_at(&mut sim, BESIDE_HER, Vec2::new(0.0, -(pickup_half.y + between)));
    assert!(
        beside.level_box_touches && !beside.turned_box_touches,
        "premise: the heart beside her is in her level box and not in her own box"
    );
    assert!(!beside.collected, "the heart beside her is not in her box, and she must not collect it");

    let (pickup, _) = heart(&mut sim, PAST_HER_END);
    let pickup_half = (pickup.max - pickup.min) * 0.5;
    let past_her_end = one_tick_at(&mut sim, PAST_HER_END, Vec2::new(-(pickup_half.x + between), 0.0));
    assert!(
        past_her_end.turned_box_touches && !past_her_end.level_box_touches,
        "premise: the heart past her end is in her own box and not in her level box"
    );
    assert!(past_her_end.collected, "the heart past her end is in her box, and she must collect it");
}
