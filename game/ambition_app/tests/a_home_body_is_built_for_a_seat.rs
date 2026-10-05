//! The home body of a session is built by one recipe for each seat.
//!
//! `session::setup::spawn_home_body` builds the body of seat N at a position
//! from the inputs the experience states once: the worn character, what the
//! experience grants and permits it, and what it holds. The primary body of a
//! session is seat 0 built there, and its caller adds the two primary markers.
//!
//! No production code builds a seat above 0. Where a second seat enters and
//! what it shares are open (`Q153`). This file builds seat 1 through the
//! recipe, in the shipped composition, and reads what that body is.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim as _, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::actors::avatar::{
    HomeBodyAbilities, HomeBodyResources, InitialBodyPolicy, StartingCharacter,
};
use ambition_platformer2d::actors::session::checkpoint::{CheckpointRestoreOutcome, SessionCheckpointOutcomes};
use ambition_platformer2d::actors::session::setup::{spawn_home_body, HomeBody};
use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};
use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
use ambition_platformer2d::engine_core::{BodyKinematics, RoomGeometry};
use ambition_platformer2d::platformer::body::PrimaryBody;
use ambition_platformer2d::platformer::lifecycle::{
    InRoomInstance, RoomInstanceRoot, RoomScopedEntity, SessionCommands, SessionScopedEntity,
};
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::ecs::system::RunSystemOnce as _;
use bevy::prelude::*;

use crate::death_restores_the_checkpoint::commit_a_checkpoint;

/// How far from the primary body the second body is built, in pixels.
const APART: f32 = 64.0;

fn the_primary(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    world
        .query_filtered::<Entity, With<PrimaryPlayer>>()
        .single(world)
        .expect("exactly one body of the session is primary")
}

fn place_of(sim: &Platformer2dSimHarness, body: Entity) -> (i32, i32) {
    let kin = sim.world().get::<BodyKinematics>(body).expect("the body has kinematics");
    (kin.pos.x.round() as i32, kin.pos.y.round() as i32)
}

/// Build the home body of `seat` through the recipe, beside the primary body
/// and in its room, from the inputs of the live session.
fn build_the_home_body_of(sim: &mut Platformer2dSimHarness, seat: PlayerSlot) -> Entity {
    build_a_home_body(sim, seat, ())
}

/// The same, with the markers the caller gives. A second body with the two
/// primary markers is a world no production code builds: it is the fixture of
/// `a_cancelled_restore_changes_nothing`, for a restore that cannot name its
/// subject.
pub(crate) fn build_a_home_body(
    sim: &mut Platformer2dSimHarness,
    seat: PlayerSlot,
    markers: impl Bundle,
) -> Entity {
    let primary = the_primary(sim);
    let mut markers = Some(markers);
    let body = sim
        .world_mut()
        .run_system_once(
            move |mut commands: SessionCommands,
                  tuning: Res<ambition_platformer2d::engine_core::ActiveMovementTuning>,
                  cast: Res<ambition_platformer2d::characters::prepared::ActiveSessionCast>,
                  roots: Query<(&InitialBodyPolicy, &HomeBodyResources, &HomeBodyAbilities)>,
                  rooms: Query<&RoomGeometry, With<RoomInstanceRoot>>,
                  bodies: Query<(&BodyKinematics, &WornCharacter, Option<&InRoomInstance>)>| {
                let (policy, resources, abilities) =
                    roots.single().expect("one session root states the home body");
                assert!(
                    matches!(policy, InitialBodyPolicy::SpawnCharacter(_)),
                    "precondition: this experience builds a home body"
                );
                let (kin, worn, room) = bodies.get(primary).expect("the primary body");
                let character = StartingCharacter::new(worn.id());
                let scope = commands
                    .spawn_scope()
                    .expect("a session is live")
                    .in_room(room.map(|room| room.0));
                spawn_home_body(
                    &mut commands,
                    scope,
                    HomeBody {
                        seat,
                        at: kin.pos + ambition_platformer2d::engine_core::Vec2::new(APART, 0.0),
                        world: rooms.single().expect("one live room"),
                        tuning: &tuning,
                        character: &character,
                        default_character_id: worn.id(),
                        prepared_characters: cast.cast(),
                        resources,
                        abilities,
                    },
                    markers.take().expect("the recipe runs one time"),
                )
            },
        )
        .expect("the recipe runs");
    sim.rebase_rollback_history().expect("the rollback history rebases over the new body");
    body
}

/// What a body says about whose it is.
#[derive(Debug, PartialEq)]
struct Whose {
    identity: String,
    driver: Option<PlayerSlot>,
    player: bool,
    primary_player: bool,
    primary_body: bool,
    session: Option<SessionScopedEntity>,
    room_scoped: bool,
}

fn whose(sim: &Platformer2dSimHarness, body: Entity) -> Whose {
    let entity = sim.world().entity(body);
    Whose {
        identity: entity.get::<SimId>().map(|id| id.as_str().to_string()).unwrap_or_default(),
        driver: entity.get::<DrivingParticipant>().map(|driver| driver.0),
        player: entity.contains::<PlayerEntity>(),
        primary_player: entity.contains::<PrimaryPlayer>(),
        primary_body: entity.contains::<PrimaryBody>(),
        session: entity.get::<SessionScopedEntity>().copied(),
        room_scoped: entity.contains::<RoomScopedEntity>(),
    }
}

/// THE PROPERTY. The body the recipe builds for seat 1 has the identity
/// `slot:1`, is driven by seat 1, is a player body of the live session, and
/// has no primary marker and no room scope. The primary body is as it was:
/// `slot:0`, seat 0, both markers, and the only primary body.
#[test]
fn the_recipe_builds_the_body_of_a_second_seat_that_is_not_primary() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    sim.step_n(AgentAction::default(), 15);
    let primary = the_primary(&mut sim);
    let before = whose(&sim, primary);
    assert_eq!(
        (before.identity.as_str(), before.driver, before.primary_player, before.primary_body, before.room_scoped),
        ("slot:0", Some(PlayerSlot::PRIMARY), true, true, false),
        "control: the primary body of the session"
    );
    assert!(before.session.is_some(), "precondition: the session owns its primary body");

    let second = build_the_home_body_of(&mut sim, PlayerSlot(1));
    assert_eq!(
        whose(&sim, second),
        Whose {
            identity: "slot:1".to_string(),
            driver: Some(PlayerSlot(1)),
            player: true,
            primary_player: false,
            primary_body: false,
            session: before.session,
            room_scoped: false,
        },
        "the home body of seat 1, on the frame it was built"
    );
    sim.step_n(AgentAction::default(), 30);
    assert_eq!(
        whose(&sim, second).identity,
        "slot:1",
        "the identity of the second body after 30 frames of play"
    );
    assert_eq!(the_primary(&mut sim), primary, "the one primary body is the body that was primary");
    assert_eq!(whose(&sim, primary), before, "the primary body after a second body was built");
}

/// A ROOM REPLAY KEEPS THE SECOND BODY. A checkpoint restore builds the room
/// again and sweeps its residents. A home body is owned by the session and is
/// not a resident, so it is the same entity after the restore commits.
///
/// Control: the restore moves the primary body, so the room was replayed.
#[test]
fn a_room_replay_keeps_the_home_body_of_a_second_seat() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    sim.step_n(AgentAction::default(), 15);
    commit_a_checkpoint(&mut sim);
    let primary = the_primary(&mut sim);
    let second = build_the_home_body_of(&mut sim, PlayerSlot(1));
    sim.step_n(AgentAction::default(), 30);
    let health = sim.world().get::<BodyHealth>(second).map(|health| health.health.current);

    let (x, y) = place_of(&sim, primary);
    sim.teleport_player((x as f32 - 2.0 * APART, y as f32));
    sim.step_n(AgentAction::default(), 30);
    let away = place_of(&sim, primary);

    let answered_before = sim.world().resource::<SessionCheckpointOutcomes>().latest().cloned();
    sim.world_mut()
        .write_message(ambition_platformer2d::platformer::lifecycle::ResetToCheckpoint);
    let mut outcome = None;
    for _ in 0..300 {
        sim.step(AgentAction::default());
        let latest = sim.world().resource::<SessionCheckpointOutcomes>().latest().cloned();
        if latest != answered_before {
            outcome = latest;
            break;
        }
    }
    sim.step_n(AgentAction::default(), 30);
    assert!(
        outcome.as_ref().is_some_and(CheckpointRestoreOutcome::committed),
        "the restore with a second body in the session: {outcome:?}"
    );
    assert_ne!(place_of(&sim, primary), away, "control: the restore moved the primary body");
    assert!(
        sim.world().get_entity(second).is_ok(),
        "the home body of seat 1 was despawned by a room replay"
    );
    let after = whose(&sim, second);
    assert_eq!(
        (after.identity.as_str(), after.driver, after.primary_player, after.room_scoped),
        ("slot:1", Some(PlayerSlot(1)), false, false),
        "the home body of seat 1 after a room replay"
    );
    assert_eq!(
        sim.world().get::<BodyHealth>(second).map(|health| health.health.current),
        health,
        "the health of the second body across the restore of the primary"
    );
}

/// THE PRIMARY BODY PLAYS AS BEFORE BESIDE A SECOND BODY. Two runs walk right
/// for the same frames: one with the primary body alone, one with the home
/// body of seat 1 built beside it. The primary body is in the same place in
/// each. The second body has no input, so it does not walk.
#[test]
fn the_primary_body_walks_the_same_beside_a_second_home_body() {
    let walked = |with_a_second: bool| {
        let mut sim =
            Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
        sim.step_n(AgentAction::default(), 15);
        let primary = the_primary(&mut sim);
        let second = with_a_second.then(|| build_the_home_body_of(&mut sim, PlayerSlot(1)));
        let start = place_of(&sim, primary);
        let second_start = second.map(|second| place_of(&sim, second));
        // Away from the second body, so the two do not touch.
        for _ in 0..60 {
            sim.step(AgentAction {
                move_x: -1.0,
                ..AgentAction::default()
            });
        }
        let end = place_of(&sim, primary);
        let second_end = second.map(|second| place_of(&sim, second));
        (start, end, second_start.zip(second_end))
    };
    let (alone_start, alone_end, _) = walked(false);
    let (start, end, second) = walked(true);
    assert_ne!(alone_end.0, alone_start.0, "control: the primary body walks when it is alone");
    assert_eq!(
        (start, end),
        (alone_start, alone_end),
        "(start, end) of the primary body beside a second home body, against the run alone"
    );
    let (second_start, second_end) = second.expect("the second run built a second body");
    assert_eq!(
        second_end.0, second_start.0,
        "the second body walked with no input of its seat: the input of seat 0 drove it"
    );
}
