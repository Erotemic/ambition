//! Q152: a collected pickup authored to regrow comes back on the world clock
//! (`GameplayElapsed`), also while its room is not live. It is the second
//! customer of the schedule the breakable respawn uses (`WorldTimeSchedule`).
//!
//! `basement_breakables` authors "regrowing heart" (`AfterSeconds`, 4 s) beside
//! a plain heart that does not regrow. Harvested, then left: a player who comes
//! back before the four seconds are up finds it still gone, for the time that
//! remains; one who comes back later finds it regrown. Before this, a collected
//! pickup had no record at all, so it was back on every rebuild of its room.

#![cfg(feature = "rl_sim")]

use ambition_app::Platformer2dSimHarness;
use ambition_platformer2d::actors::features::GameplayElapsed;
use ambition_platformer2d::combat::components::{CenteredAabb, Collected, FeatureName, RespawnTimer};
use ambition_platformer2d::engine_core::Vec2;

use crate::breakable_respawn_across_rooms::cross_to;
use crate::common::{base, fixed_60hz_room_sim};

const ROOM: &str = "basement_breakables";
const HUB: &str = "central_hub_complex";
const HEART: &str = "regrowing heart";
/// The heart beside it, authored with no regrowth.
const PLAIN: &str = "Pickup";
const REGROW_S: f32 = 4.0;

fn now(sim: &Platformer2dSimHarness) -> f32 {
    sim.world().resource::<GameplayElapsed>().0
}

/// (collected, the time its regrowth still needs) of the pickup `name`, or
/// `None` when its room is not built.
fn pickup(sim: &mut Platformer2dSimHarness, name: &str) -> Option<(bool, Option<f32>)> {
    let world = sim.world_mut();
    let mut q = world.query::<(&FeatureName, Option<&Collected>, Option<&RespawnTimer>)>();
    q.iter(world)
        .find(|(feature, _, _)| feature.0.as_str() == name)
        .map(|(_, collected, timer)| (collected.is_some(), timer.map(|timer| timer.0)))
}

fn records(sim: &Platformer2dSimHarness) -> usize {
    sim.world()
        .resource::<ambition_platformer2d::actors::features::ecs::world_time_schedule::WorldTimeSchedule>()
        .records()
        .count()
}

fn place_the_player(sim: &mut Platformer2dSimHarness, at: Vec2) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<
        &mut ambition_platformer2d::engine_core::BodyKinematics,
        bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
    >();
    let mut kin = q.single_mut(world).expect("the player's body is in the world");
    kin.pos = at;
    kin.vel = Vec2::ZERO;
}

fn player_at(sim: &mut Platformer2dSimHarness) -> Vec2 {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<
        &ambition_platformer2d::engine_core::BodyKinematics,
        bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
    >();
    q.single(world).expect("the player's body is in the world").pos
}

/// Collect the pickup `name` by standing on it, as a player does. Returns the
/// clock at the collection.
fn collect(sim: &mut Platformer2dSimHarness, name: &str) -> f32 {
    let at = {
        let world = sim.world_mut();
        let mut q = world.query::<(&FeatureName, &CenteredAabb)>();
        q.iter(world)
            .find(|(feature, _)| feature.0.as_str() == name)
            .map(|(_, aabb)| aabb.center)
            .unwrap_or_else(|| panic!("the room authors the pickup '{name}'"))
    };
    for _ in 0..30 {
        place_the_player(sim, at);
        sim.step(base());
        if pickup(sim, name).is_some_and(|(collected, _)| collected) {
            return now(sim);
        }
    }
    panic!("stood on '{name}' for 30 frames and it was not collected");
}

fn settle(sim: &mut Platformer2dSimHarness, frames: usize) {
    for _ in 0..frames {
        sim.step(base());
    }
}

/// The room, settled, with the heart collected and the player back where she
/// started, off the heart.
fn harvested_room() -> (Platformer2dSimHarness, f32) {
    let mut sim = fixed_60hz_room_sim(ROOM);
    settle(&mut sim, 30);
    assert_eq!(pickup(&mut sim, HEART), Some((false, None)), "precondition: the heart starts whole");
    let start = player_at(&mut sim);
    let collected_at = collect(&mut sim, HEART);
    place_the_player(&mut sim, start);
    settle(&mut sim, 2);
    (sim, collected_at)
}

/// Back before the four seconds are up: still gone, for what remains.
#[test]
fn a_heart_collected_before_leaving_is_still_gone_on_a_quick_return() {
    let (mut sim, collected_at) = harvested_room();
    assert_eq!(cross_to(&mut sim, HUB), HUB);
    assert_eq!(cross_to(&mut sim, ROOM), ROOM);
    let away = now(&sim) - collected_at;
    assert!(
        away < REGROW_S,
        "precondition: the round trip took {away:.2} s of the session clock, inside the {REGROW_S} s regrowth"
    );
    let (collected, remaining) = pickup(&mut sim, HEART).expect("the heart's room is built again");
    let remaining = remaining.unwrap_or(0.0);
    assert!(
        collected && (remaining - (REGROW_S - away)).abs() <= 2.0 / 60.0,
        "after {away:.3} s away: (collected, remaining) = ({collected}, {remaining:.3}); the regrowth must still need {:.3} s",
        REGROW_S - away
    );
}

/// Harvest, leave, wait, return: it has regrown, and its record is gone.
#[test]
fn a_heart_whose_regrowth_fell_due_while_away_has_regrown_on_return() {
    let (mut sim, collected_at) = harvested_room();
    assert_eq!(cross_to(&mut sim, HUB), HUB);
    settle(&mut sim, 300);
    assert_eq!(cross_to(&mut sim, ROOM), ROOM);
    assert!(now(&sim) - collected_at > REGROW_S, "precondition: the regrowth fell due while away");
    settle(&mut sim, 2);
    assert_eq!(
        (pickup(&mut sim, HEART), records(&sim)),
        (Some((false, None)), 0),
        "(the heart, the schedule's records) after the regrowth fell due away from the room"
    );
}

/// In its live room the heart regrows when its time is up. The control is the
/// plain heart beside it, collected in the same tick range: it does not.
#[test]
fn a_heart_regrows_in_its_live_room_and_a_plain_one_does_not() {
    let (mut sim, _) = harvested_room();
    let start = player_at(&mut sim);
    collect(&mut sim, PLAIN);
    place_the_player(&mut sim, start);
    settle(&mut sim, (REGROW_S * 60.0) as usize + 30);
    assert_eq!(
        (pickup(&mut sim, HEART), pickup(&mut sim, PLAIN), records(&sim)),
        (Some((false, None)), Some((true, None)), 0),
        "(the regrowing heart, the plain heart, the schedule's records) after the regrowth"
    );
}

/// A replay is a fresh attempt at the room: the heart is whole.
#[test]
fn a_replay_rebuilds_a_collected_heart_whole() {
    let (mut sim, _) = harvested_room();
    sim.world_mut().write_message(
        ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual(),
    );
    settle(&mut sim, 30);
    assert_eq!(
        (pickup(&mut sim, HEART), records(&sim)),
        (Some((false, None)), 0),
        "(the heart, the schedule's records) after a replay of its room"
    );
}

/// (collected, the schedule's records) after Alice banks a checkpoint, the
/// heart is collected, and Alice dies. With `bob_holds_it`, Bob holds the
/// heart's room and Alice is in the hub; else Alice is alone in it.
fn heart_after_alices_death(bob_holds_it: bool) -> (bool, usize) {
    // Longer than the death's interlude, so only the death can regrow it.
    const LONG_S: f32 = 30.0;
    let mut sim = if bob_holds_it {
        crate::two_players_two_live_rooms::alice_leaves_bob_in(
            ROOM,
            HUB,
            Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
            cross_to,
        )
        .0
    } else {
        let mut sim = fixed_60hz_room_sim(ROOM);
        settle(&mut sim, 30);
        sim
    };
    crate::death_restores_the_checkpoint::commit_a_checkpoint(&mut sim);
    // Collected as the collection leaves it: `Collected` and the countdown
    // of its regrowth. Alice is not in the room to stand on it.
    {
        let world = sim.world_mut();
        let mut q = world.query::<(bevy::prelude::Entity, &FeatureName)>();
        let heart = q
            .iter(world)
            .find(|(_, feature)| feature.0.as_str() == HEART)
            .map(|(entity, _)| entity)
            .expect("the room authors the regrowing heart");
        world.entity_mut(heart).insert((Collected, RespawnTimer(LONG_S)));
    }
    settle(&mut sim, 2);
    assert_eq!(
        (pickup(&mut sim, HEART).map(|(collected, _)| collected), records(&sim)),
        (Some(true), 1),
        "precondition: the heart is collected and its regrowth is scheduled"
    );
    crate::death_restores_the_checkpoint::die(&mut sim);
    let collected = pickup(&mut sim, HEART).is_some_and(|(collected, _)| collected);
    (collected, records(&sim))
}

/// Q151: a death is local to its participant and room. A heart collected in
/// Bob's live room after the checkpoint stays collected when Alice dies in
/// hers, and its regrowth record stays. The live timer serves it, as it serves
/// a breakable's respawn: the restore forgets every record, and
/// `regrow_pickups` records it again on the next tick from the running timer.
/// The control is Alice alone in the room: her restore rebuilds it, so the
/// heart is whole and the record is gone.
#[test]
fn a_death_keeps_the_regrowth_of_a_heart_in_another_players_room() {
    assert_eq!(
        heart_after_alices_death(false),
        (false, 0),
        "control: a death in the heart's own room rebuilds it whole and forgets its regrowth"
    );
    assert_eq!(
        heart_after_alices_death(true),
        (true, 1),
        "Alice's death in the hub took back the regrowth of the heart collected in Bob's live room"
    );
}
