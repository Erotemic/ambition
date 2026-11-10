//! LOCAL-PLAY-MODES slice 3: a joined seat leaves, through the stamped host
//! intent the System menu writes (`ParticipantLeaveRequested`).
//!
//! The lifetime policy under test: membership ends, the seat's body is
//! despawned, what it holds is put down where it stood (one copy, not two and
//! not none), its view and HUD row go, no room is reset, and the seat can join
//! again. The primary seat cannot leave this way (the control).
//!
//! The last arm runs a join, a leave and a second join under a two-seat sync
//! test, which rewinds and resimulates the frames around each, so a leave that
//! does not replay to the same state fails. The arms with the object run
//! without it: they place seat 1's body from the test, outside the
//! simulation, and a rewind would undo that.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{AgentAction, AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::characters::control::PlayerSlot;
use ambition_platformer2d::engine_core::{BodyKinematics, ControlFrame};
use ambition_platformer2d::held_items::ItemCustody;
use ambition_platformer2d::platformer::host_intents::write_host_intent;
use ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences;
use ambition_platformer2d::platformer::sim_id::SimId;
use ambition_platformer2d::session::leave::ParticipantLeaveRequested;
use ambition_platformer2d::session::participants::SessionParticipants;
use ambition_platformer2d::sim_view::{LocalView, SharedViewHudFacts, ViewHudSeat};
use bevy::prelude::*;

use crate::a_second_seat_joins_the_session::{
    bodies_of_the_seat, join, put_body_at, room_of, seat_one_goes_through_the_door, the_primary,
};
use crate::common::{fixed_60hz_room_options, fixed_60hz_room_sim};
use crate::two_players_two_live_rooms::the_views;

const ROOM: &str = "central_hub_complex";
const NEXT_DOOR: &str = "duel_arena";
const OBJECT: &str = "ground_gun_sword";

fn booted() -> Platformer2dSimHarness {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(AgentAction::default(), 10);
    sim
}

fn members(sim: &mut Platformer2dSimHarness) -> Vec<u8> {
    let world = sim.world_mut();
    world
        .query::<&SessionParticipants>()
        .iter(world)
        .flat_map(|members| members.iter().map(|slot| slot.0).collect::<Vec<_>>())
        .collect()
}

fn leave(sim: &mut Platformer2dSimHarness, seat: u8) {
    write_host_intent(sim.world_mut(), ParticipantLeaveRequested { participant: PlayerSlot(seat) });
    sim.step_n(AgentAction::default(), 3);
}

/// Every live occurrence of the object: its custody and where it is.
fn the_object(sim: &mut Platformer2dSimHarness) -> Vec<(ItemCustody, (i32, i32))> {
    let id = SimId::placement(OBJECT);
    let world = sim.world_mut();
    world
        .query::<(&SimId, &ItemCustody, &BodyKinematics)>()
        .iter(world)
        .filter(|(sim_id, ..)| **sim_id == id)
        .map(|(_, custody, kin)| (*custody, (kin.pos.x.round() as i32, kin.pos.y.round() as i32)))
        .collect()
}

/// The object's whereabouts row, by kind (`Placed`, `InCustody`, ...).
fn whereabouts(sim: &Platformer2dSimHarness) -> String {
    match sim.world().resource::<AuthoredOccurrences>().whereabouts(&SimId::placement(OBJECT)) {
        Some(row) => format!("{row:?}").split([' ', '{', '(']).next().unwrap_or_default().to_owned(),
        None => "none".to_owned(),
    }
}

/// Every seat a HUD shows, on any view.
fn hud_seats(sim: &mut Platformer2dSimHarness) -> Vec<u8> {
    let world = sim.world_mut();
    let mut seats: Vec<u8> = world
        .query_filtered::<(&ViewHudSeat, &SharedViewHudFacts), With<LocalView>>()
        .iter(world)
        .flat_map(|(own, shared)| own.0.iter().chain(shared.0.iter().map(|(slot, _)| slot)).map(|slot| slot.0).collect::<Vec<_>>())
        .collect();
    seats.sort();
    seats.dedup();
    seats
}

/// Seat 1's body now. A rewind across the join rebuilds the body as a new
/// entity, so the sync-test arm reads it again after each step.
fn seat_one_body(sim: &mut Platformer2dSimHarness) -> Entity {
    let bodies = bodies_of_the_seat(sim);
    assert_eq!(bodies.len(), 1, "seat 1 drives one body: {bodies:?}");
    bodies[0]
}

/// Seat 1 joins and takes the object off the hub's shelf.
fn seat_one_holds_the_object(sim: &mut Platformer2dSimHarness) -> Entity {
    join(sim, 4).expect("seat 1 pressed Start and no body was built");
    sim.drive_seat(1, ControlFrame::default());
    sim.step_n(AgentAction::default(), 15);
    let body = seat_one_body(sim);
    let (_, at) = the_object(sim).into_iter().find(|(custody, _)| custody.in_world()).expect("the object lies on its shelf");
    put_body_at(sim, body, ambition_platformer2d::engine_core::Vec2::new(at.0 as f32, at.1 as f32));
    sim.drive_seat(1, ControlFrame { attack_pressed: true, ..ControlFrame::default() });
    sim.step(AgentAction::default());
    sim.drive_seat(1, ControlFrame::default());
    sim.step_n(AgentAction::default(), 15);
    let bob = seat_one_body(sim);
    assert_eq!(
        the_object(sim).iter().map(|(custody, _)| *custody).collect::<Vec<_>>(),
        vec![ItemCustody::Held { holder: bob }],
        "control: seat 1 holds the one object"
    );
    bob
}

/// Seat 1 leaves beside the primary, holding the object; the primary's leave
/// is refused first; and seat 1 joins again afterwards.
#[test]
fn a_seat_that_leaves_puts_down_what_it_holds_and_can_join_again() {
    let mut sim = booted();
    let primary = the_primary(&mut sim);
    seat_one_holds_the_object(&mut sim);
    let room = room_of(&sim, primary);
    assert_eq!(hud_seats(&mut sim), vec![0, 1], "control: a HUD shows both seats");

    leave(&mut sim, 0);
    assert_eq!(members(&mut sim), vec![0, 1], "the primary seat does not leave this way");
    let bob = seat_one_body(&mut sim);
    assert_eq!(
        the_object(&mut sim).iter().map(|(custody, _)| *custody).collect::<Vec<_>>(),
        vec![ItemCustody::Held { holder: bob }],
        "and seat 1 still holds the object"
    );

    let stood = sim.world().get::<BodyKinematics>(bob).expect("seat 1's body").pos;
    leave(&mut sim, 1);
    assert_eq!(members(&mut sim), vec![0], "seat 1 is no longer a member");
    assert!(bodies_of_the_seat(&mut sim).is_empty(), "seat 1's body retired");
    let object = the_object(&mut sim);
    assert_eq!(object.len(), 1, "one object, not two and not none: {object:?}");
    assert!(object[0].0.in_world(), "the object lies in the world: {object:?}");
    assert!(
        (object[0].1 .0 - stood.x.round() as i32).abs() <= 2,
        "the object was put down where seat 1 stood ({stood:?}): {object:?}"
    );
    assert_eq!(hud_seats(&mut sim), vec![0], "no HUD shows the seat that left");
    assert_eq!(room_of(&sim, primary), room, "the primary's room was not rebuilt");
    assert_eq!(the_primary(&mut sim), primary, "nor its body");

    let (again, _) = join(&mut sim, 4).expect("seat 1 joins again after it left");
    sim.drive_seat(1, ControlFrame::default());
    sim.step_n(AgentAction::default(), 15);
    assert_ne!(again, bob, "a new body");
    assert_eq!(members(&mut sim), vec![0, 1]);
    assert_eq!(the_object(&mut sim).len(), 1, "the join minted no object");
}

/// Seat 1 carries the object next door and leaves there: the view that
/// followed it closes, and the object is still accounted for once.
#[test]
fn a_seat_that_leaves_another_room_closes_its_view() {
    let mut sim = booted();
    let body = seat_one_holds_the_object(&mut sim);
    assert!(seat_one_goes_through_the_door(&mut sim, body, ROOM, NEXT_DOOR), "precondition: seat 1 crossed to {NEXT_DOOR}");
    let views = the_views(&mut sim);
    assert_eq!(views.len(), 2, "control: a second view follows seat 1 next door: {views:?}");

    leave(&mut sim, 1);
    sim.step_n(AgentAction::default(), 30);
    assert!(bodies_of_the_seat(&mut sim).is_empty(), "seat 1's body retired");
    let views = the_views(&mut sim);
    assert_eq!(views.len(), 1, "seat 1's view closed: {views:?}");
    assert_eq!(hud_seats(&mut sim), vec![0], "no HUD shows the seat that left");
    let object = the_object(&mut sim);
    assert_eq!(object.len(), 1, "one object, not two and not none: {object:?}");
    assert!(object[0].0.in_world(), "nobody holds the object: {object:?}");
    assert_eq!(whereabouts(&sim), "Placed", "it is recorded as put down next door");
}

/// A join, a leave and a second join, each rewound and resimulated by a
/// two-seat sync test: every pass of the leave's tick retires the same seat.
#[test]
fn a_leave_and_a_second_join_replay_under_the_sync_test() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(ROOM).with_sync_test_rollback_settings(4, 10).with_rollback_players(2),
    )
    .expect("the hub boots under a two-seat sync test");
    sim.step_n(AgentAction::default(), 10);
    join(&mut sim, 4).expect("seat 1 joins");
    sim.drive_seat(1, ControlFrame::default());
    sim.step_n(AgentAction::default(), 15);
    seat_one_body(&mut sim);
    leave(&mut sim, 1);
    sim.step_n(AgentAction::default(), 15);
    assert_eq!(sim.rollback_health(), Ok(()), "the leave resimulated to the same state");
    assert_eq!(members(&mut sim), vec![0], "seat 1 left");
    assert!(bodies_of_the_seat(&mut sim).is_empty(), "seat 1's body retired");
    join(&mut sim, 4).expect("seat 1 joins again");
    sim.drive_seat(1, ControlFrame::default());
    sim.step_n(AgentAction::default(), 15);
    seat_one_body(&mut sim);
    assert_eq!(members(&mut sim), vec![0, 1], "seat 1 is a member again");
    assert_eq!(sim.rollback_health(), Ok(()), "the second join resimulated to the same state");
}
