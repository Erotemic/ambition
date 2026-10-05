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

fn jump() -> ControlFrame {
    ControlFrame {
        jump_pressed: true,
        jump_held: true,
        ..ControlFrame::default()
    }
}

fn the_primary(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    world
        .query_filtered::<Entity, With<PrimaryPlayer>>()
        .single(world)
        .expect("exactly one body of the session is primary")
}

/// Every player body that seat `SEAT` drives.
fn bodies_of_the_seat(sim: &mut Platformer2dSimHarness) -> Vec<Entity> {
    let world = sim.world_mut();
    world
        .query_filtered::<(Entity, &DrivingParticipant), With<PlayerEntity>>()
        .iter(world)
        .filter(|(_, driver)| driver.0 == PlayerSlot(SEAT))
        .map(|(entity, _)| entity)
        .collect()
}

fn place_of(sim: &Platformer2dSimHarness, body: Entity) -> (i32, i32) {
    let kin = sim.world().get::<BodyKinematics>(body).expect("the body has kinematics");
    (kin.pos.x.round() as i32, kin.pos.y.round() as i32)
}

fn room_of(sim: &Platformer2dSimHarness, body: Entity) -> Option<InRoomInstance> {
    sim.world().get::<InRoomInstance>(body).copied()
}

/// Press Jump on seat `SEAT` for one frame, then step neutral until its body
/// exists (at most `frames`). Returns the body and the primary's place on the
/// frame the body first exists.
fn join(sim: &mut Platformer2dSimHarness, frames: usize) -> Option<(Entity, (i32, i32))> {
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
fn die(sim: &mut Platformer2dSimHarness, body: Entity) {
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
    die(&mut sim, body);
    let at = place_of(&sim, body);
    let primary_at = place_of(&sim, primary);
    assert_eq!(room_of(&sim, body), room_of(&sim, primary), "seat 1 came back in another room");
    assert!(
        (at.0 - primary_at.0).abs() <= 8 && (at.1 - primary_at.1).abs() <= 24,
        "seat 1 came back at {at:?}, not beside the primary at {primary_at:?}"
    );
}
