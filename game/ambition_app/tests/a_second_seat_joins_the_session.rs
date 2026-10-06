//! A second seat joins Ambition and comes back after it dies (Q153 defaults).
//!
//! The join: a seat with no body that presses Jump gets its home body beside
//! the primary body, in the primary's room, on the frame of the press, inside
//! the simulation. The return: a second seat whose death beat closes comes
//! back beside the primary, and its death sends no room back, because the
//! restore's subject is the primary.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim as _, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::actors::session::checkpoint::SessionCheckpointOutcomes;
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
use ambition_platformer2d::combat::death_rules::OutOfPlay;
use ambition_platformer2d::engine_core::{BodyKinematics, ControlFrame};
use ambition_platformer2d::platformer::lifecycle::InRoomInstance;
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::*;

const SEAT: u8 = 1;

pub(crate) fn jump() -> ControlFrame {
    ControlFrame {
        jump_pressed: true,
        jump_held: true,
        ..ControlFrame::default()
    }
}

pub(crate) fn the_primary(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    world
        .query_filtered::<Entity, With<PrimaryPlayer>>()
        .single(world)
        .expect("exactly one body of the session is primary")
}

/// Every player body that seat `SEAT` drives.
pub(crate) fn bodies_of_the_seat(sim: &mut Platformer2dSimHarness) -> Vec<Entity> {
    let world = sim.world_mut();
    world
        .query_filtered::<(Entity, &DrivingParticipant), With<PlayerEntity>>()
        .iter(world)
        .filter(|(_, driver)| driver.0 == PlayerSlot(SEAT))
        .map(|(entity, _)| entity)
        .collect()
}

pub(crate) fn place_of(sim: &Platformer2dSimHarness, body: Entity) -> (i32, i32) {
    let kin = sim.world().get::<BodyKinematics>(body).expect("the body has kinematics");
    (kin.pos.x.round() as i32, kin.pos.y.round() as i32)
}

pub(crate) fn room_of(sim: &Platformer2dSimHarness, body: Entity) -> Option<InRoomInstance> {
    sim.world().get::<InRoomInstance>(body).copied()
}

/// Press Jump on seat `SEAT` for one frame, then step neutral until its body
/// exists (at most `frames`). Returns the body and the primary's place on the
/// frame the body first exists.
pub(crate) fn join(sim: &mut Platformer2dSimHarness, frames: usize) -> Option<(Entity, (i32, i32))> {
    let primary = the_primary(sim);
    sim.drive_seat(SEAT, jump());
    sim.step(AgentAction::default());
    for _ in 0..frames {
        if let Some(body) = bodies_of_the_seat(sim).first().copied() {
            return Some((body, place_of(sim, primary)));
        }
        sim.drive_seat(SEAT, ControlFrame::default());
        sim.step(AgentAction::default());
    }
    None
}

/// Seat `SEAT`'s body `body` dies (a hazard), and the beat runs out. Panics
/// unless the body is out of play on the frame after the death (the control)
/// and back in play within 240 frames.
pub(crate) fn die(sim: &mut Platformer2dSimHarness, body: Entity) {
    let at = place_of(sim, body);
    sim.world_mut().write_message(ambition_platformer2d::combat::death_rules::ActorDiedMessage {
        victim: body,
        pos: ambition_platformer2d::engine_core::Vec2::new(at.0 as f32, at.1 as f32),
        cause: ambition_platformer2d::combat::death_rules::DeathCause {
            source: ambition_platformer2d::combat::HitSource::Hazard,
            attacker: None,
        },
    });
    sim.step(AgentAction::default());
    assert!(sim.world().entity(body).contains::<OutOfPlay>(), "control: the dead seat is out of play");
    for _ in 0..240 {
        sim.step(AgentAction::default());
        if !sim.world().entity(body).contains::<OutOfPlay>() {
            return;
        }
    }
    panic!("seat 1 never came back into play after its death");
}

/// THE JOIN, under a two-seat sync test that rewinds and resimulates the
/// frames around the press. Control: 30 frames with seat 1 neutral build no
/// body. The press builds one body of seat 1 (identity `slot:1`, not primary),
/// in the primary's room and where the primary stands. The resimulated
/// frames agree (no checksum mismatch). A second press builds no second body,
/// and seat 1's input moves its own body and not the primary.
#[test]
fn a_seat_with_no_body_joins_beside_the_primary_on_a_jump_press() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10)
            .with_rollback_players(2),
    )
    .expect("a two-seat GGRS sync-test harness builds");
    for _ in 0..30 {
        sim.drive_seat(SEAT, ControlFrame::default());
        sim.step(AgentAction::default());
    }
    assert!(bodies_of_the_seat(&mut sim).is_empty(), "control: a seat that pressed nothing has no body");
    let primary = the_primary(&mut sim);

    let (body, primary_at) = join(&mut sim, 4).expect("seat 1 pressed Jump and no body was built");
    assert_eq!(
        (
            sim.world().get::<SimId>(body).map(|id| id.as_str().to_string()),
            sim.world().entity(body).contains::<PrimaryPlayer>(),
            room_of(&sim, body),
        ),
        (Some("slot:1".to_string()), false, room_of(&sim, primary)),
        "(identity, primary, room) of the body seat 1 joined with"
    );
    let at = place_of(&sim, body);
    assert!(
        (at.0 - primary_at.0).abs() <= 8 && (at.1 - primary_at.1).abs() <= 24,
        "the body of seat 1 is at {at:?}, not beside the primary at {primary_at:?}"
    );
    for frame in 0..30 {
        sim.drive_seat(SEAT, if frame == 10 { jump() } else { ControlFrame::default() });
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame} after the join: {error}"));
    }
    let stats = sim.rollback_execution_stats().expect("GGRS instrumentation is installed");
    assert!(stats.load_runs > 0, "no rewind happened, so the join frame was never resimulated");
    // One body, and not the same `Entity`: a rewind across the join frame
    // despawns the body and the resimulation builds it again.
    let seated = bodies_of_the_seat(&mut sim);
    assert_eq!(seated.len(), 1, "seat 1 does not have one body after the resimulated frames (a second press, or a rewind that lost it): {seated:?}");
    let body = seated[0];
    assert_eq!(
        sim.world().get::<SimId>(body).map(|id| id.as_str().to_string()),
        Some("slot:1".to_string()),
        "the identity of seat 1's body after the resimulated frames"
    );

    let primary_before = place_of(&sim, primary);
    let seat_before = place_of(&sim, body);
    for _ in 0..40 {
        sim.drive_seat(
            SEAT,
            ControlFrame {
                axis_x: 1.0,
                ..ControlFrame::default()
            },
        );
        sim.step(AgentAction::default());
    }
    assert_ne!(place_of(&sim, body).0, seat_before.0, "seat 1's input did not move its body");
    assert_eq!(place_of(&sim, primary), primary_before, "seat 1's input moved the primary");
}

/// THE RETURN. Seat 1 dies while the primary plays: its beat closes, no
/// restore is asked for, and it comes back beside the primary, in play, with
/// its health full. Control: while its beat is open it is out of play.
#[test]
fn a_second_seat_that_dies_comes_back_beside_the_primary() {
    let mut sim =
        Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    sim.step_n(AgentAction::default(), 15);
    let primary = the_primary(&mut sim);
    let (body, _) = join(&mut sim, 4).expect("seat 1 pressed Jump and no body was built");
    // Seat 1 walks away from the primary, so "beside the primary" is a move.
    for _ in 0..40 {
        sim.drive_seat(
            SEAT,
            ControlFrame {
                axis_x: 1.0,
                ..ControlFrame::default()
            },
        );
        sim.step(AgentAction::default());
    }
    sim.drive_seat(SEAT, ControlFrame::default());
    let away = place_of(&sim, body);
    let primary_at = place_of(&sim, primary);
    assert!((away.0 - primary_at.0).abs() > 24, "precondition: seat 1 walked away from the primary");
    if let Some(mut health) = sim.world_mut().get_mut::<BodyHealth>(body) {
        health.health.current = 1;
    }
    let restores_before = sim.world().resource::<SessionCheckpointOutcomes>().latest().cloned();

    die(&mut sim, body);
    let at = place_of(&sim, body);
    let primary_at = place_of(&sim, primary);
    assert!(
        (at.0 - primary_at.0).abs() <= 8 && (at.1 - primary_at.1).abs() <= 24,
        "seat 1 came back at {at:?}, not beside the primary at {primary_at:?}"
    );
    assert_eq!(room_of(&sim, body), room_of(&sim, primary), "seat 1 came back in another room");
    let health = sim.world().get::<BodyHealth>(body).map(|health| (health.health.current, health.health.max));
    assert!(
        health.is_some_and(|(current, max)| current == max),
        "seat 1 came back without its health: {health:?}"
    );
    assert_eq!(
        sim.world().resource::<SessionCheckpointOutcomes>().latest().cloned(),
        restores_before,
        "the death of seat 1 asked for a checkpoint restore of the primary"
    );
}

/// THE RETURN ACROSS ROOMS. Seat 1 joins beside Alice in `switch_lab`, Alice
/// walks through the door to the hub, and seat 1 stays and dies in the room
/// Alice left. It comes back beside Alice, in her live room. Control: before
/// its death, seat 1 is in another live room than Alice.
#[test]
fn a_second_seat_that_dies_in_another_room_comes_back_in_the_primarys_room() {
    use crate::common::{a_save_that_has_seen_the_hub_intro, fixed_60hz_room_options, walk_through_the_door_to};
    const ROOM: &str = "switch_lab";
    const HUB: &str = "central_hub_complex";
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(ROOM).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .unwrap_or_else(|error| panic!("{ROOM} boots: {error:?}"));
    sim.step_n(AgentAction::default(), 10);
    let (body, _) = join(&mut sim, 4).expect("seat 1 pressed Jump and no body was built");
    sim.drive_seat(SEAT, ControlFrame::default());
    assert_eq!(walk_through_the_door_to(&mut sim, HUB), HUB);
    sim.step_n(AgentAction::default(), 30);
    let primary = the_primary(&mut sim);
    let body = bodies_of_the_seat(&mut sim).first().copied().unwrap_or(body);
    assert_ne!(
        room_of(&sim, body),
        room_of(&sim, primary),
        "control: seat 1 stayed in the room Alice left"
    );
    assert_eq!(
        crate::two_players_two_live_rooms::live_rooms(&mut sim).len(),
        2,
        "control: the room seat 1 stayed in is live beside Alice's"
    );
    die(&mut sim, body);
    let body = bodies_of_the_seat(&mut sim).first().copied().unwrap_or(body);
    let at = place_of(&sim, body);
    let primary_at = place_of(&sim, primary);
    assert_eq!(room_of(&sim, body), room_of(&sim, primary), "seat 1 came back in another room");
    assert!(
        (at.0 - primary_at.0).abs() <= 8 && (at.1 - primary_at.1).abs() <= 24,
        "seat 1 came back at {at:?}, not beside the primary at {primary_at:?}"
    );
    // OW4: the room seat 1 left by its return holds nobody, so it retires,
    // as a room a crossing leaves empty does. It stayed live with nobody in
    // it, and a restore of Alice's checkpoint neither spared it nor built it
    // again.
    let rooms: Vec<String> = crate::two_players_two_live_rooms::live_rooms(&mut sim)
        .into_iter()
        .map(|(_, id)| id)
        .collect();
    assert_eq!(rooms, [HUB.to_string()], "the live rooms after seat 1 came back");
}

// ── The three arms of the 2026-10-05 review (Namek): each was measured
// before its repair, and the measurement is in its doc. ──────────────────────

const ROOM: &str = "switch_lab";
const HUB: &str = "central_hub_complex";

fn booted_in(room: &str) -> Platformer2dSimHarness {
    use crate::common::{a_save_that_has_seen_the_hub_intro, fixed_60hz_room_options};
    let mut sim =
        Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(room).with_save(a_save_that_has_seen_the_hub_intro()))
            .unwrap_or_else(|error| panic!("{room} boots: {error:?}"));
    sim.step_n(AgentAction::default(), 10);
    sim
}

fn seat_one_joins(sim: &mut Platformer2dSimHarness) -> Entity {
    let (body, _) = join(sim, 4).expect("seat 1 pressed Jump and no body was built");
    sim.drive_seat(SEAT, ControlFrame::default());
    body
}

fn put_body_at(sim: &mut Platformer2dSimHarness, body: Entity, at: ambition_platformer2d::engine_core::Vec2) {
    let world = sim.world_mut();
    let mut bodies = world.query::<(
        ambition_platformer2d::engine_core::BodyClusterQueryData,
        &mut ambition_platformer2d::engine_core::movement::MotionModel,
    )>();
    let (mut clusters, mut model) = bodies.get_mut(world, body).expect("the body has clusters");
    let mut clusters = clusters.as_clusters_mut();
    ambition_platformer2d::engine_core::movement::transit_body(
        &mut model,
        &mut clusters,
        at,
        ambition_platformer2d::engine_core::movement::TransitVelocity::Zero,
    );
}

/// The death, and one frame. [`die`] also waits for the return; this does not.
fn kill(sim: &mut Platformer2dSimHarness, body: Entity) {
    let at = place_of(sim, body);
    sim.world_mut().write_message(ambition_platformer2d::combat::death_rules::ActorDiedMessage {
        victim: body,
        pos: ambition_platformer2d::engine_core::Vec2::new(at.0 as f32, at.1 as f32),
        cause: ambition_platformer2d::combat::death_rules::DeathCause {
            source: ambition_platformer2d::combat::HitSource::Hazard,
            attacker: None,
        },
    });
    sim.step(AgentAction::default());
}

fn out_of_play(sim: &Platformer2dSimHarness, body: Entity) -> bool {
    sim.world().entity(body).contains::<OutOfPlay>()
}

/// Seat 1 walks through the door of `room` to `target`. Whether it crossed.
fn seat_one_goes_through_the_door(sim: &mut Platformer2dSimHarness, body: Entity, room: &str, target: &str) -> bool {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let before = room_of(sim, body);
    let door = crate::two_players_two_live_rooms::door_of(sim, room, target).aabb.center();
    put_body_at(sim, body, door);
    let mut crossed = false;
    for _ in 0..180 {
        sim.drive_seat(
            SEAT,
            ControlFrame {
                interact_pressed: true,
                interact_held: true,
                ..Default::default()
            },
        );
        sim.step(AgentAction::default());
        if room_of(sim, body) != before {
            crossed = true;
            break;
        }
    }
    sim.drive_seat(SEAT, ControlFrame::default());
    sim.step_n(AgentAction::default(), 30);
    crossed
}

/// R1: NOBODY IS LEFT OUT OF PLAY FOR EVER. The primary falls beside seat 1,
/// so its room waits for seat 1. Then seat 1 walks out of that room: nobody is
/// in play there any more, so the primary's room goes back (a committed
/// restore) and the primary is in play. Then seat 1 falls in the hub and
/// comes back beside the primary.
///
/// Measured before the repair: the primary's question was spent on the tick
/// its window closed, so 600 frames after seat 1 left, the primary was still
/// out of play; and after seat 1 fell too, both were out of play 900 frames
/// later with no restore (each waited for the other).
#[test]
fn a_primary_that_waited_for_a_seat_comes_back_when_the_seat_leaves_its_room() {
    let mut sim = booted_in(ROOM);
    let primary = the_primary(&mut sim);
    let body = seat_one_joins(&mut sim);
    sim.step_n(AgentAction::default(), 5);
    kill(&mut sim, primary);
    sim.step_n(AgentAction::default(), 400);
    assert!(
        out_of_play(&sim, primary),
        "control: the primary fell beside seat 1, in play in its room, and its room waits"
    );
    assert!(seat_one_goes_through_the_door(&mut sim, body, ROOM, HUB), "seat 1 did not cross to the hub");
    let body = bodies_of_the_seat(&mut sim).first().copied().unwrap_or(body);
    sim.step_n(AgentAction::default(), 600);
    assert!(
        !out_of_play(&sim, primary),
        "seat 1 left the primary's room and the primary is still out of play: its room was never asked again"
    );
    assert!(
        sim.world()
            .resource::<SessionCheckpointOutcomes>()
            .latest()
            .is_some_and(|outcome| outcome.committed()),
        "the primary came back without a committed restore: {:?}",
        sim.world().resource::<SessionCheckpointOutcomes>().latest()
    );
    kill(&mut sim, body);
    sim.step_n(AgentAction::default(), 900);
    let body = bodies_of_the_seat(&mut sim).first().copied().unwrap_or(body);
    assert_eq!(
        (out_of_play(&sim, primary), out_of_play(&sim, body), room_of(&sim, body)),
        (false, false, room_of(&sim, primary)),
        "(primary out of play, seat 1 out of play, seat 1's room) after seat 1 fell in the hub"
    );
}

/// R8: A NEW GAME TAKES SEAT 1 WITH IT. A New Game retires every live room but
/// the one it builds. Seat 1's body is the session's, so it stays; it must be
/// in the new live room and answer its input. Control: seat 1 beside the
/// primary.
///
/// Measured before the repair, seat 1 in another live room: its stamp stayed
/// the retired room's (#0, live rooms [#2]) and 40 frames of input moved it
/// 0 px. Control: restamped, it ran 175 px.
#[test]
fn a_new_game_takes_a_seat_in_another_room_into_the_new_room() {
    for apart in [false, true] {
        let mut sim = booted_in(ROOM);
        let body = seat_one_joins(&mut sim);
        if apart {
            assert_eq!(walk_through_the_door_to_hub(&mut sim), HUB);
            sim.step_n(AgentAction::default(), 30);
        }
        sim.world_mut()
            .write_message(ambition_platformer2d::actors::session::reset::NewGameRequested);
        sim.step_n(AgentAction::default(), 400);
        let seated = bodies_of_the_seat(&mut sim);
        assert_eq!(seated, vec![body], "apart={apart}: seat 1 does not have its one body after the New Game");
        let primary = the_primary(&mut sim);
        assert_eq!(
            room_of(&sim, body),
            room_of(&sim, primary),
            "apart={apart}: seat 1 is not in the primary's room after the New Game"
        );
        let before = place_of(&sim, body);
        for _ in 0..40 {
            sim.drive_seat(
                SEAT,
                ControlFrame {
                    axis_x: 1.0,
                    ..ControlFrame::default()
                },
            );
            sim.step(AgentAction::default());
        }
        sim.drive_seat(SEAT, ControlFrame::default());
        assert!(
            (place_of(&sim, body).0 - before.0).abs() > 24,
            "apart={apart}: 40 frames of input moved seat 1 from {before:?} to {:?}",
            place_of(&sim, body)
        );
    }
}

fn walk_through_the_door_to_hub(sim: &mut Platformer2dSimHarness) -> String {
    crate::common::walk_through_the_door_to(sim, HUB)
}

/// R5: THE RETURN TAKES WHAT THE SEAT HOLDS. Seat 1 picks up the ground item
/// of `blink_run`, the primary goes to `portal_bridge`, and seat 1 falls in
/// `blink_run`. It comes back in the primary's room, and the item it holds
/// comes with it, as a crossing carries what a body holds.
///
/// Measured before the repair: the body's stamp was the primary's room and the
/// item's stayed `blink_run`'s, still held.
#[test]
fn a_seat_that_comes_back_into_another_room_brings_what_it_holds() {
    const ITEM_ROOM: &str = "blink_run";
    const NEXT: &str = "portal_bridge";
    let mut sim = booted_in(ITEM_ROOM);
    let body = seat_one_joins(&mut sim);
    sim.step_n(AgentAction::default(), 5);
    let item = {
        let world = sim.world_mut();
        world
            .query::<(Entity, &ambition_platformer2d::held_items::ItemCustody)>()
            .iter(world)
            .filter(|(_, custody)| custody.in_world())
            .map(|(entity, _)| entity)
            .next()
            .expect("blink_run has a ground item")
    };
    let at = sim
        .world()
        .get::<ambition_platformer2d::held_items::GroundItem>(item)
        .expect("the item lies on the ground")
        .pos;
    put_body_at(&mut sim, body, at);
    sim.drive_seat(
        SEAT,
        ControlFrame {
            attack_pressed: true,
            ..Default::default()
        },
    );
    sim.step(AgentAction::default());
    sim.drive_seat(SEAT, ControlFrame::default());
    sim.step_n(AgentAction::default(), 3);
    let custodian = sim
        .world()
        .get::<ambition_platformer2d::platformer::lifecycle::InCustodyOf>(item)
        .map(|custody| custody.custodian);
    assert_eq!(custodian, Some(body), "precondition: seat 1 holds the item");
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, NEXT), NEXT);
    sim.step_n(AgentAction::default(), 30);
    let primary = the_primary(&mut sim);
    assert_ne!(room_of(&sim, body), room_of(&sim, primary), "precondition: seat 1 stayed in blink_run");
    die(&mut sim, body);
    sim.step_n(AgentAction::default(), 5);
    assert_eq!(
        (room_of(&sim, body), room_of(&sim, item)),
        (room_of(&sim, primary), room_of(&sim, primary)),
        "(seat 1's room, the held item's room) after seat 1 came back beside the primary"
    );
}

