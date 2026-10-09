//! WORLD-ACCEPTANCE: one headless playthrough of the shipped game.
//!
//! The player walks from the hub to Alice through the real exits of each room
//! (a door is entered with Interact, an edge exit by standing in it), takes her
//! note, carries it to Bob, and hands it over. Each step asserts its fact
//! against the authority that owns it: the live room, the bag, the save's
//! flags, the quest registry, and the collision overlay of the live room.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use bevy::prelude::{App, KeyCode};
use leafwing_input_manager::prelude::Buttonlike;

use crate::neighbor_prefetch_prepares_rooms::{alice as primary_body, room_of};
use crate::the_note_travels_from_alice_to_bob::{
    a_returning_players_session, bag, flag, put_body_at, quest_step, talk_and_choose, NOTE_FLAG,
    SURVEY_FLAG,
};

/// The rooms from the hub to Alice, each through an exit of the room before.
const ROUTE_TO_ALICE: &[&str] = &[
    "intro_wake_room",
    "intro_raid_corridor",
    "intro_escape_shaft",
    "drain_alley",
    "under_town_pipes",
    "alice_relay",
];

/// The wall in `alice_relay` that Bob's survey opens (`gated_by`).
const RETURN_LOCK: &str = "alice_private_return_lock";

fn live_room(app: &mut App) -> String {
    let body = primary_body(app);
    room_of(app, body).expect("the player is in a live room")
}

/// Leave the live room through its exit to `target`: stand in the exit and,
/// for a door, hold Interact.
fn go_through(app: &mut App, target: &str) {
    let from = live_room(app);
    let (center, door) = {
        let world = app.world_mut();
        let live = ambition_platformer2d::world::rooms::sole_live_room_definition(world).expect("one live room");
        let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
            ambition_platformer2d::world::rooms::RoomSet,
        >(world)
        .expect("the session keeps its room set");
        let zone = rooms
            .spec(live)
            .loading_zones
            .iter()
            .find(|zone| {
                rooms
                    .transition_for_player(live, zone.aabb, ae::Vec2::ZERO, true)
                    .is_some_and(|transition| rooms.rooms[transition.target_room].id == target)
            })
            .unwrap_or_else(|| panic!("`{from}` has no exit to `{target}`"));
        (
            zone.aabb.center(),
            matches!(zone.activation, ambition_platformer2d::world::rooms::LoadingZoneActivation::Door),
        )
    };
    let body = primary_body(app);
    put_body_at(app, body, center);
    // A door takes a fresh press. One held from the first frame can land in
    // the cooldown after the last crossing, so press again every 10 frames.
    let mut arrived = false;
    for frame in 0..600 {
        if door && frame % 10 == 0 {
            Buttonlike::press(&KeyCode::KeyF, app.world_mut());
        }
        if door && frame % 10 == 2 {
            Buttonlike::release(&KeyCode::KeyF, app.world_mut());
        }
        app.update();
        if room_of(app, body).as_deref() == Some(target) {
            arrived = true;
            break;
        }
    }
    Buttonlike::release(&KeyCode::KeyF, app.world_mut());
    assert!(
        arrived,
        "the exit of `{from}` to `{target}` did not take the player there in 600 frames (player at {:?}, room {:?})",
        app.world().get::<ae::BodyKinematics>(body).map(|k| k.pos),
        room_of(app, body)
    );
    for _ in 0..30 {
        app.update();
    }
}

/// Is `block` a standing gate solid of the live room?
fn gate_stands(app: &App, block: &str) -> bool {
    ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<
        ambition_platformer2d::world::FeatureEcsWorldOverlay,
    >(app.world())
    .expect("the live room has a collision overlay")
    .gate_solids
    .iter()
    .any(|solid| solid.name.contains(block))
}

/// ⭐ ONE PLAYTHROUGH: from the hub to Alice through six real exits, the note to
/// Bob, the wall the survey opens is open when the player comes back, and a
/// save taken in another room loads into a new process with all of it: the
/// bag, the flags, the quest step, and the open wall at the end of the walk.
#[test]
fn the_note_reaches_bob_and_opens_alices_return() {
    let mut app = a_returning_players_session();
    let start = live_room(&mut app);
    assert_eq!(start, "central_hub_complex", "the shipped game starts in the hub");
    let step0 = quest_step(&app).expect("the intro quest starts by itself");

    for room in ROUTE_TO_ALICE {
        go_through(&mut app, room);
    }
    assert!(gate_stands(&app, RETURN_LOCK), "before the survey, Alice's return lock does not stand");

    talk_and_choose(&mut app, "npc_alice", "Take the note.");
    assert_eq!(
        (bag(&app, "sealednote"), flag(&app, NOTE_FLAG), quest_step(&app)),
        (1, true, Some(step0 + 1)),
        "(notes, note flag, quest step) after Alice's choice"
    );

    go_through(&mut app, "bob_relay");
    talk_and_choose(&mut app, "npc_bob", "Hand him the sealed note.");
    assert_eq!(
        (bag(&app, "sealednote"), bag(&app, "fieldsurvey"), flag(&app, SURVEY_FLAG), quest_step(&app)),
        (0, 1, true, Some(step0 + 2)),
        "(notes, surveys, survey flag, quest step) after Bob's choice"
    );

    go_through(&mut app, "alice_relay");
    assert!(
        !gate_stands(&app, RETURN_LOCK),
        "the survey was handed over and Alice's return lock still stands"
    );

    // Leave, and save in another room. A new process loads the file.
    go_through(&mut app, "bob_relay");
    go_through(&mut app, "drain_alley");
    let file = app
        .world()
        .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data()
        .clone();
    drop(app);
    let mut app = a_session_loaded_from(&file);
    // With no checkpoint, a load starts in the hub.
    assert_eq!(live_room(&mut app), "central_hub_complex");
    assert_eq!(
        (bag(&app, "sealednote"), bag(&app, "fieldsurvey"), flag(&app, NOTE_FLAG), flag(&app, SURVEY_FLAG), quest_step(&app)),
        (0, 1, true, true, Some(step0 + 2)),
        "(notes, surveys, note flag, survey flag, quest step) after the load"
    );
    for room in ROUTE_TO_ALICE {
        go_through(&mut app, room);
    }
    assert!(!gate_stands(&app, RETURN_LOCK), "after the load, Alice's return lock stands again");
    talk_and_choose(&mut app, "npc_alice", "What now?");

    // The death step waits for Q164: a death takes back the survey and keeps
    // the facts it made (two horizons).
}

/// A new process that loads `file`: the shipped shell session, booted with the
/// save's bytes in `AmbitionGameSave` before the gameplay route starts.
fn a_session_loaded_from(file: &ambition_platformer2d::persistence::save_data::AmbitionGameSaveData) -> App {
    use ambition_app::app::{build_visible_app, VisibleRenderMode};
    use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();
    app.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .0 = file.clone();
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: ShellRouteId::new("ambition_gameplay"),
        request: None,
    });
    for _ in 0..240 {
        app.update();
    }
    app
}
