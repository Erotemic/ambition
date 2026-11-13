//! A second seat joins Ambition and comes back after it dies (Q153).
//!
//! The join: a seat with no body that presses Start gets its home body beside
//! the primary body, in the primary's room, on the frame of the press, inside
//! the simulation. The return: a second seat whose death beat closes comes
//! back beside the primary, and its death sends no room back, because the
//! restore's subject is the primary.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim as _, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::session::checkpoint::SessionCheckpointOutcomes;
use ambition_platformer2d::session::participants::SessionParticipants;
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
use ambition_platformer2d::combat::death_rules::OutOfPlay;
use ambition_platformer2d::engine_core::{AabbExt, BodyKinematics, ControlFrame};
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

/// The join gesture: Start, the press that claims a pad for a seat.
pub(crate) fn start() -> ControlFrame {
    ControlFrame {
        start_pressed: true,
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

/// Press Start on seat `SEAT` for one frame, then step neutral until its body
/// exists (at most `frames`). Returns the body and the primary's place on the
/// frame the body first exists.
pub(crate) fn join(sim: &mut Platformer2dSimHarness, frames: usize) -> Option<(Entity, (i32, i32))> {
    let primary = the_primary(sim);
    sim.drive_seat(SEAT, start());
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
/// frames around the press. Controls: 30 frames with seat 1 neutral build no
/// body, and neither does a Jump press (Jump is play, never the join gesture). The press builds one body of seat 1 (identity `slot:1`, not primary),
/// in the primary's room and clear of its collision box. The resimulated
/// frames agree (no checksum mismatch). A second press builds no second body,
/// and seat 1's input moves its own body and not the primary.
#[test]
fn a_seat_with_no_body_joins_beside_the_primary_on_a_start_press() {
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
    for frame in [jump(), ControlFrame::default(), ControlFrame::default()] {
        sim.drive_seat(SEAT, frame);
        sim.step(AgentAction::default());
    }
    assert!(bodies_of_the_seat(&mut sim).is_empty(), "control: a Jump press is not the join gesture");
    let members = ambition_platformer2d::platformer::lifecycle::session_world_component::<SessionParticipants>(sim.world()).unwrap();
    assert!(members.contains(PlayerSlot::PRIMARY));
    assert!(!members.contains(PlayerSlot(SEAT)));
    let primary = the_primary(&mut sim);

    let (body, primary_at) = join(&mut sim, 4).expect("seat 1 pressed Start and no body was built");
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
    let primary_box = sim.world().get::<BodyKinematics>(primary).unwrap()
        .aabb();
    let joined_box = sim.world().get::<BodyKinematics>(body).unwrap()
        .aabb();
    assert!(
        !primary_box.strict_intersects(joined_box),
        "the joining body at {at:?} overlaps the primary at {primary_at:?}: {joined_box:?}, {primary_box:?}"
    );
    for frame in 0..30 {
        sim.drive_seat(SEAT, if frame == 10 { start() } else { ControlFrame::default() });
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame} after the join: {error}"));
    }
    assert!(ambition_platformer2d::platformer::lifecycle::session_world_component::<SessionParticipants>(sim.world()).unwrap().contains(PlayerSlot(SEAT)));
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

/// THE RETURN (COOP-RECOVERY). Seat 1 dies in the primary's room while the
/// primary plays: its beat closes, no restore is asked for, and it comes back
/// alone at the room's respawn point (the spawn: no checkpoint is in this
/// room), in play, with its health full. The primary has walked away from the
/// spawn, so the spawn is not beside it.
#[test]
fn a_second_seat_that_dies_beside_the_primary_comes_back_at_the_rooms_spawn() {
    let mut sim =
        Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    sim.step_n(AgentAction::default(), 15);
    let primary = the_primary(&mut sim);
    let spawn = place_of(&sim, primary);
    let (body, _) = join(&mut sim, 4).expect("seat 1 pressed Start and no body was built");
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
    // The primary walks the other way, so the spawn is not beside it.
    sim.step_n(AgentAction { move_x: -1.0, ..AgentAction::default() }, 40);
    sim.step_n(AgentAction::default(), 10);
    let away = place_of(&sim, body);
    let primary_at = place_of(&sim, primary);
    assert!((away.0 - spawn.0).abs() > 24, "precondition: seat 1 walked away from the spawn");
    assert!((primary_at.0 - spawn.0).abs() > 24, "precondition: the primary walked away from the spawn");
    if let Some(mut health) = sim.world_mut().get_mut::<BodyHealth>(body) {
        health.health.current = 1;
    }
    let restores_before = ambition_platformer2d::platformer::lifecycle::session_world_component::<SessionCheckpointOutcomes>(sim.world())
        .expect("the live session root carries the checkpoint coordinator").latest().cloned();

    die(&mut sim, body);
    let at = place_of(&sim, body);
    assert!(
        (at.0 - spawn.0).abs() <= 8 && (at.1 - spawn.1).abs() <= 24,
        "seat 1 came back at {at:?}, not at the room's spawn {spawn:?}"
    );
    assert_eq!(place_of(&sim, primary), primary_at, "the primary was touched by seat 1's return");
    assert_eq!(room_of(&sim, body), room_of(&sim, primary), "seat 1 came back in another room");
    let health = sim.world().get::<BodyHealth>(body).map(|health| (health.health.current, health.health.max));
    assert!(
        health.is_some_and(|(current, max)| current == max),
        "seat 1 came back without its health: {health:?}"
    );
    assert_eq!(
        ambition_platformer2d::platformer::lifecycle::session_world_component::<SessionCheckpointOutcomes>(sim.world())
        .expect("the live session root carries the checkpoint coordinator").latest().cloned(),
        restores_before,
        "the death of seat 1 asked for a checkpoint restore of the primary"
    );
}

/// THE LOCAL RECOVERY (COOP-RECOVERY). Seat 1 joins beside Alice in
/// `switch_lab`, Alice walks through the door to the hub, and seat 1 stays
/// and dies alone in the room Alice left. Its own room goes back (a committed
/// restore whose subject is seat 1) and it comes back there, at the room's
/// spawn; Alice is not moved and her room stays live. Control: before its
/// death, seat 1 is in another live room than Alice.
#[test]
fn a_second_seat_that_dies_alone_in_another_room_recovers_in_its_own_room() {
    use crate::common::{fixed_60hz_room_options, walk_through_the_door_to};
    const ROOM: &str = "switch_lab";
    const HUB: &str = "central_hub_complex";
    let outcome = |sim: &Platformer2dSimHarness| {
        ambition_platformer2d::platformer::lifecycle::session_world_component::<SessionCheckpointOutcomes>(sim.world())
            .expect("the live session root carries the checkpoint coordinator")
            .latest()
            .cloned()
    };
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(ROOM),
    )
    .unwrap_or_else(|error| panic!("{ROOM} boots: {error:?}"));
    sim.step_n(AgentAction::default(), 10);
    let alice = the_primary(&mut sim);
    let spawn = place_of(&sim, alice);
    let (body, _) = join(&mut sim, 4).expect("seat 1 pressed Start and no body was built");
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
    let alice_at = place_of(&sim, primary);
    let alice_room = room_of(&sim, primary);
    let before = outcome(&sim);
    die(&mut sim, body);
    sim.step_n(AgentAction::default(), 5);
    let body = bodies_of_the_seat(&mut sim).first().copied().unwrap_or(body);
    assert!(
        outcome(&sim).is_some_and(|outcome| outcome.committed()) && outcome(&sim) != before,
        "seat 1 fell alone in its room and its room did not go back: {:?}",
        outcome(&sim)
    );
    assert_ne!(room_of(&sim, body), room_of(&sim, primary), "seat 1 came back in Alice's room");
    let at = place_of(&sim, body);
    assert!(
        (at.0 - spawn.0).abs() <= 8 && (at.1 - spawn.1).abs() <= 24,
        "seat 1 came back at {at:?}, not at its room's spawn {spawn:?}"
    );
    assert_eq!((place_of(&sim, primary), room_of(&sim, primary)), (alice_at, alice_room), "Alice was moved by seat 1's recovery");
    let rooms: Vec<String> = crate::two_players_two_live_rooms::live_rooms(&mut sim)
        .into_iter()
        .map(|(_, id)| id)
        .collect();
    assert_eq!(rooms.len(), 2, "the live rooms after seat 1's recovery: {rooms:?}");
}

// ── The three arms of the 2026-10-05 review (Namek): each was measured
// before its repair, and the measurement is in its doc. ──────────────────────

const ROOM: &str = "switch_lab";
const HUB: &str = "central_hub_complex";

fn booted_in(room: &str) -> Platformer2dSimHarness {
    use crate::common::fixed_60hz_room_options;
    let mut sim =
        Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(room))
            .unwrap_or_else(|error| panic!("{room} boots: {error:?}"));
    sim.step_n(AgentAction::default(), 10);
    sim
}

fn seat_one_joins(sim: &mut Platformer2dSimHarness) -> Entity {
    let (body, _) = join(sim, 4).expect("seat 1 pressed Start and no body was built");
    sim.drive_seat(SEAT, ControlFrame::default());
    body
}

pub(crate) fn put_body_at(sim: &mut Platformer2dSimHarness, body: Entity, at: ambition_platformer2d::engine_core::Vec2) {
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
pub(crate) fn kill(sim: &mut Platformer2dSimHarness, body: Entity) {
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

pub(crate) fn out_of_play(sim: &Platformer2dSimHarness, body: Entity) -> bool {
    sim.world().entity(body).contains::<OutOfPlay>()
}

/// Seat 1 walks through the door of `room` to `target`. Whether it crossed.
pub(crate) fn seat_one_goes_through_the_door(sim: &mut Platformer2dSimHarness, body: Entity, room: &str, target: &str) -> bool {
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

/// COOP-RECOVERY: A PARTICIPANT WHOSE ROOM A PARTNER HOLDS COMES BACK ALONE.
/// The primary falls beside seat 1: after Ambition's declared wait it comes
/// back in play in its own room, with no checkpoint restore, and seat 1 plays
/// on where it was. Then
/// seat 1 crosses to the hub and the primary falls alone in its room: that
/// room goes back (a committed restore) and the primary is in play.
///
/// Before (Q153's 2026-10-05 default) the primary's room waited while seat 1
/// played in it, and the primary stayed out of play until seat 1 left.
#[test]
fn a_primary_that_falls_beside_a_seat_comes_back_alone_and_its_room_goes_on() {
    let outcome = |sim: &Platformer2dSimHarness| {
        ambition_platformer2d::platformer::lifecycle::session_world_component::<SessionCheckpointOutcomes>(sim.world())
            .expect("the live session root carries the checkpoint coordinator")
            .latest()
            .cloned()
    };
    let mut sim = booted_in(ROOM);
    let primary = the_primary(&mut sim);
    let body = seat_one_joins(&mut sim);
    sim.step_n(AgentAction::default(), 5);
    let seat_at = place_of(&sim, body);
    let before = outcome(&sim);
    kill(&mut sim, primary);
    // Ambition's death beat (1 s) and its declared wait while a partner plays
    // (`recovery_wait_s`, 2 s).
    sim.step_n(AgentAction::default(), 160);
    assert!(out_of_play(&sim, primary), "control: the primary waits while seat 1 plays");
    sim.step_n(AgentAction::default(), 40);
    assert!(!out_of_play(&sim, primary), "the primary fell beside seat 1 and is still out of play");
    assert_eq!(outcome(&sim), before, "the primary's fall beside seat 1 sent its room back");
    let moved = place_of(&sim, body);
    assert!(
        !out_of_play(&sim, body) && (moved.0 - seat_at.0).abs() <= 2 && (moved.1 - seat_at.1).abs() <= 2,
        "seat 1 was touched by the primary's return: {seat_at:?} -> {moved:?}"
    );

    assert!(seat_one_goes_through_the_door(&mut sim, body, ROOM, HUB), "seat 1 did not cross to the hub");
    let before = outcome(&sim);
    kill(&mut sim, primary);
    sim.step_n(AgentAction::default(), 600);
    assert!(!out_of_play(&sim, primary), "the primary fell alone in its room and is still out of play");
    assert!(
        outcome(&sim).is_some_and(|outcome| outcome.committed()) && outcome(&sim) != before,
        "the primary fell alone in its room and its room did not go back: {:?}",
        outcome(&sim)
    );
}

/// COOP-RECOVERY: THE WAIT KEEPS THE WIPE REACHABLE. Seat 1 falls beside the
/// primary and waits (`recovery_wait_s`); the primary falls a second later,
/// inside the wait. Nobody is in play, so the checkpoint restore runs, and it
/// brings both back. Control: a second after its fall, seat 1 is still out of
/// play.
#[test]
fn a_partner_that_falls_inside_the_wait_makes_a_wipe() {
    let outcome = |sim: &Platformer2dSimHarness| {
        ambition_platformer2d::platformer::lifecycle::session_world_component::<SessionCheckpointOutcomes>(sim.world())
            .expect("the live session root carries the checkpoint coordinator")
            .latest()
            .cloned()
    };
    let mut sim = booted_in(ROOM);
    let primary = the_primary(&mut sim);
    let body = seat_one_joins(&mut sim);
    sim.step_n(AgentAction::default(), 5);
    let before = outcome(&sim);
    kill(&mut sim, body);
    sim.step_n(AgentAction::default(), 60);
    assert!(out_of_play(&sim, body), "control: seat 1 waits while the primary plays");
    kill(&mut sim, primary);
    sim.step_n(AgentAction::default(), 600);
    let body = bodies_of_the_seat(&mut sim).first().copied().unwrap_or(body);
    assert!(
        outcome(&sim).is_some_and(|outcome| outcome.committed()) && outcome(&sim) != before,
        "both fell and no restore committed: {:?}",
        outcome(&sim)
    );
    assert_eq!(
        (out_of_play(&sim, primary), out_of_play(&sim, body)),
        (false, false),
        "(primary out of play, seat 1 out of play) after the wipe"
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
            .write_message(ambition_platformer2d::session::reset::NewGameRequested);
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

/// R5, RESTATED FOR COOP-RECOVERY: seat 1 picks up the ground item of
/// `blink_run` (after the session's checkpoint), the primary goes to
/// `portal_bridge`, and seat 1 falls alone in `blink_run`. Its own room goes
/// back: it comes back in `blink_run`, and the item it took since the
/// checkpoint lies in that room again, held by nobody.
#[test]
fn a_seat_that_recovers_alone_gives_back_what_it_took_since_the_checkpoint() {
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
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(item)
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
    let body = bodies_of_the_seat(&mut sim).first().copied().unwrap_or(body);
    let lying = {
        let world = sim.world_mut();
        world
            .query::<&ambition_platformer2d::held_items::ItemCustody>()
            .iter(world)
            .filter(|custody| custody.in_world())
            .count()
    };
    assert_ne!(room_of(&sim, body), room_of(&sim, primary), "seat 1 came back in the primary's room");
    assert_eq!(lying, 1, "the item seat 1 took since the checkpoint does not lie in its room again");
}


/// Q163 (the default in force): a conversation claims only the talker
/// (ruling 2026-08-06), also with two players in one room. Alice talks to the
/// hub's dog with seat 1 beside her; seat 1 keeps playing: its input moves its
/// body, it takes the gun-sword, and it goes through the door to
/// `duel_arena`, all while Alice is still talking. The talker's own input is
/// captured by the dialogue input context, which the harness's actions do
/// not pass through, so it is not measured here.
///
/// Measured before: the conversation put the session in the dialogue mode,
/// which stops every gameplay-gated system, so seat 1 moved (176 px) but could
/// not pick up or take a door. Control: Alice alone still enters the dialogue
/// mode (`a_conversation_in_one_room_does_not_stop_the_other_players_room`).
#[test]
fn a_conversation_holds_only_the_talker_with_two_players_in_one_room() {
    let talking = |sim: &Platformer2dSimHarness| {
        sim.world()
            .get_resource::<ambition_platformer2d::conversation::ActiveConversation>()
            .is_some_and(|conversation| conversation.talker().is_some())
    };
    let mut sim = booted_in("central_hub_complex");
    let body = seat_one_joins(&mut sim);
    sim.step_n(AgentAction::default(), 5);
    assert!(
        crate::two_players_two_live_rooms::alice_talks_to_the_dog(&mut sim),
        "precondition: Alice did not talk to the dog"
    );
    let before = place_of(&sim, body);
    for _ in 0..40 {
        sim.drive_seat(SEAT, ControlFrame { axis_x: 1.0, ..ControlFrame::default() });
        sim.step(AgentAction::default());
    }
    sim.drive_seat(SEAT, ControlFrame::default());
    let moved = (place_of(&sim, body).0 - before.0).abs();
    let gun = SimId::placement("ground_gun_sword");
    let at = {
        let world = sim.world_mut();
        world
            .query::<(&SimId, &ambition_platformer2d::engine_core::BodyKinematics)>()
            .iter(world)
            .find(|(id, _)| **id == gun)
            .map(|(_, ground)| ground.pos)
            .expect("the hub lays out the gun-sword")
    };
    put_body_at(&mut sim, body, at);
    sim.drive_seat(SEAT, ControlFrame { attack_pressed: true, ..ControlFrame::default() });
    sim.step(AgentAction::default());
    sim.drive_seat(SEAT, ControlFrame::default());
    sim.step_n(AgentAction::default(), 3);
    let took = {
        let world = sim.world_mut();
        world
            .query::<(&SimId, &ambition_platformer2d::platformer::lifecycle::InCustodyOf)>()
            .iter(world)
            .any(|(id, custody)| *id == gun && custody.custodian == body)
    };
    assert!(talking(&sim), "precondition: Alice's conversation ended before seat 1 crossed");
    let crossed = seat_one_goes_through_the_door(&mut sim, body, "central_hub_complex", "duel_arena");
    assert_eq!(
        (moved > 24, took, crossed),
        (true, true, true),
        "(seat 1 moved, took the gun-sword, went through the door) while Alice talked; mode {:?}",
        crate::two_players_two_live_rooms::game_mode(&sim)
    );
}

/// A second player sees where its own blink lands. Seat 1 joins beside
/// Alice in the hub and holds Blink: the blink preview has a reticle for seat
/// 1's body, within a blink of it. Control: with nobody holding Blink there is
/// no reticle.
///
/// The preview had one reticle, of the controlled subject, read from the
/// primary seat's device. Poison, the controlled subject's reticle only: no
/// reticle while seat 1 holds Blink.
#[test]
fn a_second_seat_holding_blink_has_its_own_reticle() {
    let reticles = |sim: &Platformer2dSimHarness| {
        sim.world()
            .resource::<ambition_platformer2d::sim_view::BlinkPreviewFact>()
            .reticles
            .iter()
            .map(|reticle| reticle.target)
            .collect::<Vec<_>>()
    };
    let mut sim = booted_in("central_hub_complex");
    let body = seat_one_joins(&mut sim);
    sim.step_n(AgentAction::default(), 5);
    assert_eq!(reticles(&sim), Vec::new(), "control: nobody holds Blink");
    assert!(
        sim.world()
            .get::<ambition_platformer2d::engine_core::BodyAbilities>(body)
            .is_some_and(|abilities| abilities.abilities.blink),
        "precondition: seat 1's body can blink"
    );
    let at = place_of(&sim, body);
    for _ in 0..3 {
        sim.drive_seat(SEAT, ControlFrame { blink_held: true, ..ControlFrame::default() });
        sim.step(AgentAction::default());
    }
    let shown = reticles(&sim);
    sim.drive_seat(SEAT, ControlFrame::default());
    let near = |target: &ambition_platformer2d::engine_core::Vec2| {
        (target.x - at.0 as f32).abs() <= ambition_platformer2d::engine_core::BLINK_DISTANCE + 8.0
            && (target.y - at.1 as f32).abs() <= ambition_platformer2d::engine_core::BLINK_DISTANCE + 8.0
    };
    assert_eq!(
        (shown.len(), shown.iter().all(near)),
        (1, true),
        "the reticles while seat 1 holds Blink, from {at:?}: {shown:?}"
    );
}

#[test]
fn both_seats_release_blink_and_finish_its_cooldown() {
    use ambition_platformer2d::engine_core::BodyBlinkState;
    use ambition_platformer2d::sim_view::BlinkPreviewFact;

    let mut sim = booted_in("central_hub_complex");
    let second = seat_one_joins(&mut sim);
    let first = the_primary(&mut sim);
    for frame in 0..4 {
        let held = ControlFrame {
            axis_x: 1.0,
            blink_pressed: frame == 0,
            blink_held: true,
            ..default()
        };
        sim.drive_seat(SEAT, held);
        sim.step_frame(held);
    }
    assert_eq!(sim.world().resource::<BlinkPreviewFact>().reticles.len(), 2);
    let release = ControlFrame { axis_x: 1.0, blink_released: true, ..default() };
    sim.drive_seat(SEAT, release);
    sim.step_frame(release);
    assert!(sim.world().resource::<BlinkPreviewFact>().reticles.is_empty());
    for body in [first, second] {
        assert!(sim.world().get::<BodyBlinkState>(body).unwrap().cooldown > 0.0,
            "the release did not execute this body's blink");
    }
    sim.step_n(AgentAction::default(), 180);
    for body in [first, second] {
        assert_eq!(sim.world().get::<BodyBlinkState>(body).unwrap().cooldown, 0.0);
    }
}


#[test]
fn start_does_not_readmit_a_participant_without_a_current_body_assignment() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz()),
    ).expect("the session builds");
    let (body, _) = join(&mut sim, 4).expect("the second participant joins");
    sim.world_mut().entity_mut(body).remove::<DrivingParticipant>();
    sim.drive_seat(SEAT, start());
    sim.step(AgentAction::default());
    let world = sim.world_mut();
    let count = world.query::<&SimId>().iter(world).filter(|id| id.as_str() == "slot:1").count();
    assert_eq!(count, 1, "Start must not construct another home body for an admitted participant");
    assert!(ambition_platformer2d::platformer::lifecycle::session_world_component::<SessionParticipants>(world).unwrap().contains(PlayerSlot(SEAT)));
}

/// A session prepared with the co-op setting "Respawn Beside Partner"
/// (PARTNER-RECOVERY), booted in `room`.
fn beside_partner_in(room: &str) -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::build(crate::common::fixed_60hz_room_options(room), |app, options| {
        let mut settings = ambition_platformer2d::persistence::settings::UserSettings::default();
        settings.gameplay.coop_respawn_beside_partner = true;
        app.insert_resource(settings);
        ambition_app::rl_sim::ambition_sim_composition(app, options)
    })
    .expect("the configured session prepares");
    sim.step_n(AgentAction::default(), 10);
    sim
}

fn latest_restore(sim: &Platformer2dSimHarness) -> Option<ambition_platformer2d::session::checkpoint::CheckpointRestoreOutcome> {
    ambition_platformer2d::platformer::lifecycle::session_world_component::<SessionCheckpointOutcomes>(sim.world())
        .expect("the live session root carries the checkpoint coordinator")
        .latest()
        .cloned()
}

/// PARTNER-RECOVERY, ONE ROOM. With the setting on, seat 1 falls in the
/// primary's room after the two walked apart, and comes back where the
/// primary stands, not at the room's spawn (the default's answer,
/// `a_second_seat_that_dies_beside_the_primary_comes_back_at_the_rooms_spawn`).
/// No restore is asked for.
#[test]
fn with_the_setting_a_fallen_seat_comes_back_beside_its_partner() {
    let mut sim = beside_partner_in(ROOM);
    let primary = the_primary(&mut sim);
    let spawn = place_of(&sim, primary);
    let body = seat_one_joins(&mut sim);
    for _ in 0..40 {
        sim.drive_seat(SEAT, ControlFrame { axis_x: 1.0, ..ControlFrame::default() });
        sim.step(AgentAction::default());
    }
    sim.drive_seat(SEAT, ControlFrame::default());
    sim.step_n(AgentAction { move_x: -1.0, ..AgentAction::default() }, 40);
    sim.step_n(AgentAction::default(), 10);
    let primary_at = place_of(&sim, primary);
    assert!((primary_at.0 - spawn.0).abs() > 24, "precondition: the primary walked away from the spawn");
    assert!((place_of(&sim, body).0 - primary_at.0).abs() > 48, "precondition: seat 1 is away from the primary");
    let before = latest_restore(&sim);
    die(&mut sim, body);
    let at = place_of(&sim, body);
    assert!(
        (at.0 - primary_at.0).abs() <= 8 && (at.1 - primary_at.1).abs() <= 24,
        "seat 1 came back at {at:?}, not beside the primary at {primary_at:?}"
    );
    assert_eq!(room_of(&sim, body), room_of(&sim, primary));
    assert_eq!(latest_restore(&sim), before, "seat 1's fall beside a partner sent a room back");
}

/// PARTNER-RECOVERY, TWO ROOMS. With the setting on, seat 1 goes through the
/// door to the hub and the primary falls alone in the room it left. Its room
/// does not go back (the default's answer: it does,
/// `a_primary_that_falls_beside_a_seat_comes_back_alone_and_its_room_goes_on`);
/// the primary crosses into seat 1's room and comes back in play beside it.
#[test]
fn with_the_setting_a_primary_alone_in_its_room_comes_back_beside_its_partner_next_door() {
    let mut sim = beside_partner_in(ROOM);
    let primary = the_primary(&mut sim);
    let body = seat_one_joins(&mut sim);
    sim.step_n(AgentAction::default(), 5);
    assert!(seat_one_goes_through_the_door(&mut sim, body, ROOM, HUB), "seat 1 did not cross to the hub");
    assert_ne!(room_of(&sim, body), room_of(&sim, primary), "precondition: seat 1 is next door");
    let before = latest_restore(&sim);
    kill(&mut sim, primary);
    let mut back = None;
    for frame in 0..600 {
        sim.step(AgentAction::default());
        if !out_of_play(&sim, primary) {
            back = Some(frame);
            break;
        }
    }
    assert!(back.is_some(), "the primary never came back into play");
    assert_eq!(room_of(&sim, primary), room_of(&sim, body), "the primary came back in another room than its partner");
    let (at, partner) = (place_of(&sim, primary), place_of(&sim, body));
    assert!((at.0 - partner.0).abs() <= 96 && (at.1 - partner.1).abs() <= 64, "the primary came back at {at:?}, far from seat 1 at {partner:?}");
    assert_eq!(latest_restore(&sim), before, "the primary's fall sent its room back though a partner plays next door");
}
