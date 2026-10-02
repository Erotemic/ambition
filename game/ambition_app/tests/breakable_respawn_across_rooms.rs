//! OW5: a broken breakable's respawn is a scheduled logical event on the
//! session clock (`GameplayElapsed`), kept when its room retires.
//!
//! `basement_breakables` authors "respawning breakable platform"
//! (`AfterSeconds:3.0`). Broken, then left: a player who comes back before its
//! three seconds are up finds it still broken, with the time that remains; one
//! who comes back later finds it whole. Before this, the countdown died with
//! the room and the platform was whole on any return. A replay of the room is
//! a fresh attempt: the platform is whole.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::actors::features::GameplayElapsed;
use ambition_platformer2d::combat::components::{BreakableFeature, FeatureName, RespawnTimer};

use crate::common::{base, fixed_60hz_room_sim, zone_to};

const ROOM: &str = "basement_breakables";
const HUB: &str = "central_hub_complex";
const PLATFORM: &str = "respawning breakable platform";
const RESPAWN_S: f32 = 3.0;

fn now(sim: &Platformer2dSimHarness) -> f32 {
    sim.world().resource::<GameplayElapsed>().0
}

/// (broken, the time its respawn still needs) of the platform, or `None` when
/// its room is not built.
fn platform(sim: &mut Platformer2dSimHarness) -> Option<(bool, Option<f32>)> {
    let world = sim.world_mut();
    let mut q = world.query::<(&FeatureName, &BreakableFeature, Option<&RespawnTimer>)>();
    q.iter(world)
        .find(|(name, _, _)| name.0.as_str() == PLATFORM)
        .map(|(_, feature, timer)| (feature.broken(), timer.map(|timer| timer.0)))
}

/// Break the platform as its own break road does: zero health, and the
/// respawn timer its `AfterSeconds` asks for. Returns the clock at the break.
fn break_the_platform(sim: &mut Platformer2dSimHarness) -> f32 {
    let world = sim.world_mut();
    let mut q = world.query::<(bevy::prelude::Entity, &FeatureName, &mut BreakableFeature)>();
    let entity = q
        .iter_mut(world)
        .find(|(_, name, _)| name.0.as_str() == PLATFORM)
        .map(|(entity, _, mut feature)| {
            let max = feature.breakable.health.max;
            assert!(feature.breakable.apply_damage(max), "the platform breaks");
            entity
        })
        .expect("the room authors the respawning platform");
    world.entity_mut(entity).insert(RespawnTimer(RESPAWN_S));
    now(sim)
}

/// Stand in the zone to `target` (holding interact, for a door) until the
/// room changes. Returns the room arrived in.
fn cross_to(sim: &mut Platformer2dSimHarness, target: &str) -> String {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let before = sim.observation().active_room.clone();
    let at = zone_to(sim, target).aabb.center();
    for _ in 0..240 {
        {
            let world = sim.world_mut();
            let mut q = world.query_filtered::<
                &mut ambition_platformer2d::engine_core::BodyKinematics,
                bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
            >();
            if let Ok(mut kin) = q.single_mut(world) {
                kin.pos = at;
                kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
            }
        }
        let room = sim
            .step(AgentAction {
                interact: true,
                interact_held: true,
                ..base()
            })
            .active_room;
        if room != before {
            return room;
        }
    }
    panic!(
        "stood in the zone of '{before}' to '{target}' for 240 frames and the room never changed"
    );
}

fn settle(sim: &mut Platformer2dSimHarness, frames: usize) {
    for _ in 0..frames {
        sim.step(base());
    }
}

fn broken_room() -> (Platformer2dSimHarness, f32) {
    let mut sim = fixed_60hz_room_sim(ROOM);
    settle(&mut sim, 30);
    assert_eq!(
        platform(&mut sim),
        Some((false, None)),
        "precondition: the platform starts whole"
    );
    let broke_at = break_the_platform(&mut sim);
    settle(&mut sim, 2);
    (sim, broke_at)
}

/// Back before the three seconds are up: still broken, for what remains.
#[test]
fn a_platform_broken_before_leaving_is_still_broken_on_a_quick_return() {
    let (mut sim, broke_at) = broken_room();
    assert_eq!(cross_to(&mut sim, HUB), HUB);
    assert_eq!(cross_to(&mut sim, ROOM), ROOM);
    let away = now(&sim) - broke_at;
    assert!(
        away < RESPAWN_S,
        "precondition: the round trip took {away:.2} s of the session clock, inside the {RESPAWN_S} s respawn"
    );
    let (broken, remaining) = platform(&mut sim).expect("the platform's room is built again");
    let remaining = remaining.unwrap_or(0.0);
    assert!(
        broken && (remaining - (RESPAWN_S - away)).abs() <= 2.0 / 60.0,
        "after {away:.3} s away: (broken, remaining) = ({broken}, {remaining:.3}); the respawn must still need {:.3} s",
        RESPAWN_S - away
    );
}

/// Back after the three seconds: whole, and its record is gone.
#[test]
fn a_platform_whose_respawn_fell_due_while_away_is_whole_on_return() {
    let (mut sim, broke_at) = broken_room();
    assert_eq!(cross_to(&mut sim, HUB), HUB);
    settle(&mut sim, 240);
    assert_eq!(cross_to(&mut sim, ROOM), ROOM);
    assert!(
        now(&sim) - broke_at > RESPAWN_S,
        "precondition: the respawn fell due while away"
    );
    settle(&mut sim, 2);
    let schedule = sim
        .world()
        .resource::<ambition_platformer2d::actors::features::ecs::breakable_respawns::BreakableRespawnSchedule>();
    let records = schedule.records().count();
    assert_eq!(
        (platform(&mut sim), records),
        (Some((false, None)), 0),
        "(the platform, the schedule's records) after the respawn fell due away from the room"
    );
}

/// A replay is a fresh attempt at the room: the platform is whole.
#[test]
fn a_replay_rebuilds_a_broken_platform_whole() {
    let (mut sim, _) = broken_room();
    sim.world_mut().write_message(
        ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual(),
    );
    settle(&mut sim, 30);
    assert_eq!(
        platform(&mut sim),
        Some((false, None)),
        "the platform after a replay of its room"
    );
}
