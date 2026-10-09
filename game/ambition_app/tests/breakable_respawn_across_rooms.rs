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
    ambition_platformer2d::platformer::lifecycle::session_world_component::<GameplayElapsed>(sim.world())
        .expect("the session root carries the clock")
        .0
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
    break_the_platform_for(sim, RESPAWN_S)
}

/// [`break_the_platform`], with a respawn after `respawn_s` seconds.
fn break_the_platform_for(sim: &mut Platformer2dSimHarness, respawn_s: f32) -> f32 {
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
    world.entity_mut(entity).insert(RespawnTimer(respawn_s));
    now(sim)
}

/// Stand in the zone to `target` (holding interact, for a door) until the
/// room changes. Returns the room arrived in.
pub(crate) fn cross_to(sim: &mut Platformer2dSimHarness, target: &str) -> String {
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
    let schedule = ambition_platformer2d::platformer::lifecycle::session_world_component::<ambition_platformer2d::actors::features::ecs::world_time_schedule::WorldTimeSchedule>(sim.world())
        .expect("the session root carries the schedule");
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

/// (broken, the schedule's records) after Alice banks a checkpoint, the
/// platform is broken, and Alice dies. With `bob_holds_it`, Bob holds the
/// platform's room and Alice is in the hub; else Alice is alone in it.
fn platform_after_alices_death(bob_holds_it: bool) -> (bool, usize) {
    // Longer than the death's interlude, so only the death can make it whole.
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
    break_the_platform_for(&mut sim, LONG_S);
    settle(&mut sim, 2);
    let records = |sim: &Platformer2dSimHarness| {
        ambition_platformer2d::platformer::lifecycle::session_world_component::<ambition_platformer2d::actors::features::ecs::world_time_schedule::WorldTimeSchedule>(sim.world())
        .expect("the session root carries the schedule")
            .records()
            .count()
    };
    assert_eq!(
        (platform(&mut sim).map(|(broken, _)| broken), records(&sim)),
        (Some(true), 1),
        "precondition: the platform is broken and its respawn is scheduled"
    );
    crate::death_restores_the_checkpoint::die(&mut sim);
    let broken = platform(&mut sim).is_some_and(|(broken, _)| broken);
    (broken, records(&sim))
}

/// Q151: a death is local to its participant and room. A platform Bob broke
/// in his live room after the checkpoint stays broken when Alice dies in hers,
/// and its respawn record stays, so a rebuild of his room after he leaves
/// still builds it broken for the time that remains. The control is Alice
/// alone in the room: her restore rebuilds it, so the platform is whole and
/// the record is gone.
///
/// ⚠ THE LIVE TIMER SERVES THIS, NOT THE RESTORE. Measured 2026-10-03 with a
/// probe: `forget_scheduled_returns_on_restore` does forget Bob's record, and
/// on the next tick `mirror_breakable_respawns` records it again from his
/// platform's running timer, with the same due time. While a room is live its
/// timers are the authority and the schedule mirrors them. A room Bob has
/// already left keeps his record by its owners
/// (`a_death_keeps_the_respawn_of_a_platform_another_player_broke_in_a_room_he_left`).
#[test]
fn a_death_keeps_the_respawn_of_a_platform_in_another_players_room() {
    assert_eq!(
        platform_after_alices_death(false),
        (false, 0),
        "control: a death in the platform's own room rebuilds it whole and forgets its respawn"
    );
    assert_eq!(
        platform_after_alices_death(true),
        (true, 1),
        "Alice's death in the hub took back the respawn of the platform Bob broke in his live room"
    );
}

/// (the platform's respawn record, live rooms) after Alice dies in the hub,
/// when the platform was broken after the checkpoint in a room that has since
/// retired. With `bobs`, Bob broke it in his room and then walked to the hub;
/// else Alice broke it alone and walked to the hub.
fn a_dormant_record_after_alices_death(bobs: bool) -> (Option<f32>, usize) {
    use crate::two_players_two_live_rooms::{bob_goes_from, live_rooms};
    const LONG_S: f32 = 30.0;
    // The schedule is keyed by the room and the platform's authored id; the
    // room has one record, the platform's.
    let schedule_due = |sim: &Platformer2dSimHarness| {
        ambition_platformer2d::platformer::lifecycle::session_world_component::<ambition_platformer2d::actors::features::ecs::world_time_schedule::WorldTimeSchedule>(sim.world())
        .expect("the session root carries the schedule")
            .records()
            .find(|((room, _), _)| room == ROOM)
            .map(|(_, due)| due)
    };
    let mut sim = if bobs {
        let (mut sim, _) = crate::two_players_two_live_rooms::alice_leaves_bob_in(
            ROOM,
            HUB,
            Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
            cross_to,
        );
        crate::death_restores_the_checkpoint::commit_a_checkpoint(&mut sim);
        break_the_platform_for(&mut sim, LONG_S);
        settle(&mut sim, 2);
        let hub = live_rooms(&mut sim)
            .into_iter()
            .find(|(_, id)| id == HUB)
            .map(|(room, _)| room)
            .expect("precondition: Alice holds the hub");
        bob_goes_from(&mut sim, ROOM, HUB, hub);
        sim
    } else {
        let mut sim = fixed_60hz_room_sim(ROOM);
        settle(&mut sim, 30);
        crate::death_restores_the_checkpoint::commit_a_checkpoint(&mut sim);
        break_the_platform_for(&mut sim, LONG_S);
        settle(&mut sim, 2);
        assert_eq!(cross_to(&mut sim, HUB), HUB);
        settle(&mut sim, 30);
        sim
    };
    let before = schedule_due(&sim);
    assert!(
        before.is_some() && !live_rooms(&mut sim).iter().any(|(_, id)| id == ROOM),
        "precondition: the platform's room retired and its respawn is recorded ({before:?})"
    );
    crate::death_restores_the_checkpoint::die(&mut sim);
    let after = schedule_due(&sim);
    assert!(after.is_none() || after == before, "the record kept a different due time: {before:?} -> {after:?}");
    (after, live_rooms(&mut sim).len())
}

/// Q151 for a room that is no longer live: the respawn of a platform Bob broke
/// after the checkpoint is his consequence, so it stays when Alice dies
/// elsewhere after his room retired, with its due time. The control is the
/// same break by Alice alone: her death takes it back. A record owns the
/// participants who were in its room when it was made, and a restore takes
/// out only the dying one; before, the restore forgot every record of a room
/// that was not live.
#[test]
fn a_death_keeps_the_respawn_of_a_platform_another_player_broke_in_a_room_he_left() {
    assert_eq!(
        a_dormant_record_after_alices_death(false).0,
        None,
        "control: Alice's death takes back the respawn of the platform she broke"
    );
    assert!(
        a_dormant_record_after_alices_death(true).0.is_some(),
        "Alice's death in the hub took back the respawn of the platform Bob broke in a room he left"
    );
}
