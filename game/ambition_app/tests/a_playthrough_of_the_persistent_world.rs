//! WORLD-ACCEPTANCE: one headless playthrough of the shipped game.
//!
//! The player takes the Blink in the hub's basement (a held item: Attack
//! becomes a blink), walks from the hub to Alice through the real exits of each room
//! (a door is entered with Interact, an edge exit by standing in it), takes her
//! note, carries it to Bob, and hands it over. Each step asserts its fact
//! against the authority that owns it: the live room, the bag, the save's
//! flags, the quest registry, the collision overlay of the live room, and the
//! custody of the one Blink.
//!
//! A second participant joins on a second pad, stays at Alice while the
//! player carries the note to Bob, and sees Alice's return lock open in its
//! own live room. A death with the Blink in hand sends it back to where it
//! lay.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::{App, Entity, KeyCode};
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

/// The Blink lying in the hub's basement: a held item, so it has one identity
/// and one custody wherever it goes.
const BLINK: &str = "ground_blink";

type Custody = ambition_platformer2d::held_items::ItemCustody;
type Ground = ambition_platformer2d::held_items::GroundItem;

/// Every live occurrence of `id`, and its custody. A count, not a lookup: a
/// copy and a loss are both failures.
fn occurrences(app: &mut App, id: &SimId) -> Vec<(Entity, Custody)> {
    let world = app.world_mut();
    world
        .query::<(Entity, &SimId, &Custody)>()
        .iter(world)
        .filter(|(_, sim_id, _)| *sim_id == id)
        .map(|(entity, _, custody)| (entity, *custody))
        .collect()
}

/// Does the player hold the one occurrence of `id`?
fn the_player_holds(app: &mut App, id: &SimId) -> bool {
    let player = primary_body(app);
    matches!(occurrences(app, id).as_slice(), [(_, Custody::Held { holder })] if *holder == player)
}

/// Stand on the Blink and press Attack until it is in hand. Answers where it
/// lay.
fn take(app: &mut App, id: &SimId) -> ae::Vec2 {
    let at = {
        let world = app.world_mut();
        let found: Vec<ae::Vec2> = world
            .query::<(&SimId, &Ground, &Custody)>()
            .iter(world)
            .filter(|(sim_id, _, custody)| *sim_id == id && custody.in_world())
            .map(|(_, ground, _)| ground.pos)
            .collect();
        assert_eq!(found.len(), 1, "exactly one `{}` lies in the world", id.as_str());
        found[0]
    };
    let body = primary_body(app);
    put_body_at(app, body, at);
    for _ in 0..40 {
        crate::the_note_travels_from_alice_to_bob::tap(app, KeyCode::KeyX);
        if the_player_holds(app, id) {
            return at;
        }
    }
    panic!("pressed Attack on `{}` for 40 taps and never held it", id.as_str());
}

/// How far one Attack, aimed along the facing, carries the standing player.
fn attack_travel(app: &mut App) -> f32 {
    let body = primary_body(app);
    for _ in 0..30 {
        app.update();
    }
    let before = app.world().get::<ae::BodyKinematics>(body).expect("a body").pos;
    crate::the_note_travels_from_alice_to_bob::tap(app, KeyCode::KeyX);
    for _ in 0..20 {
        app.update();
    }
    let after = app.world().get::<ae::BodyKinematics>(body).expect("a body").pos;
    (after - before).length()
}

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

    // A capability: the Blink, from the hub's basement, into the hand. With
    // it, Attack is a blink; without it, Attack is a strike that does not
    // carry the body.
    let blink = SimId::placement(BLINK);
    let control = attack_travel(&mut app);
    take(&mut app, &blink);
    let travel = attack_travel(&mut app);
    assert!(
        control < 24.0 && travel > 96.0,
        "Attack carried the body {control:.0} px before the Blink and {travel:.0} px with it"
    );

    for room in ROUTE_TO_ALICE {
        go_through(&mut app, room);
    }
    assert!(gate_stands(&app, RETURN_LOCK), "before the survey, Alice's return lock does not stand");

    assert!(the_player_holds(&mut app, &blink), "the Blink did not come along to Alice");
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
    assert!(the_player_holds(&mut app, &blink), "after the load, the Blink is not in the player's hand");
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
    crate::common::step_until_route_is_active_and_settled(&mut app, "ambition_gameplay");
    app
}

/// The seat a second participant joins on.
const SECOND_SEAT: ambition_platformer2d::characters::control::PlayerSlot =
    ambition_platformer2d::characters::control::PlayerSlot(1);

/// The body seat `SECOND_SEAT` drives, if it has one.
fn the_second_body(app: &mut App) -> Option<Entity> {
    use ambition_platformer2d::characters::control::DrivingParticipant;
    let world = app.world_mut();
    world
        .query_filtered::<(Entity, &DrivingParticipant), bevy::prelude::With<ambition_platformer2d::platformer::markers::PlayerEntity>>()
        .iter(world)
        .find(|(_, driver)| driver.0 == SECOND_SEAT)
        .map(|(entity, _)| entity)
}

/// The second seat presses Jump until it has a body (the Q153 join road).
fn join_the_second_seat(app: &mut App) -> Entity {
    let jump = ae::ControlFrame {
        jump_pressed: true,
        jump_held: true,
        ..ae::ControlFrame::default()
    };
    for frame in 0..120 {
        let press = if frame % 4 == 0 { jump } else { ae::ControlFrame::default() };
        ambition_platformer2d::rollback::drive_slot_frame(app.world_mut(), SECOND_SEAT, press);
        app.update();
        if let Some(body) = the_second_body(app) {
            for _ in 0..30 {
                app.update();
            }
            return body;
        }
    }
    panic!("the second seat pressed Jump for 120 frames and got no body");
}

/// The live rooms by id, each with whether `block` stands as a gate solid in
/// it. One row per live room, so a room live twice is visible.
fn gates_by_live_room(app: &mut App, block: &str) -> Vec<(String, bool)> {
    use ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot;
    use ambition_platformer2d::world::rooms::LiveRoomDefinition;
    let world = app.world_mut();
    let rooms: Vec<(LiveRoomDefinition, bool)> = world
        .query_filtered::<(&LiveRoomDefinition, &ambition_platformer2d::world::FeatureEcsWorldOverlay), bevy::prelude::With<RoomInstanceRoot>>()
        .iter(world)
        .map(|(definition, overlay)| (*definition, overlay.gate_solids.iter().any(|solid| solid.name.contains(block))))
        .collect();
    let set = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(world)
    .expect("the session keeps its room set");
    let mut named: Vec<(String, bool)> =
        rooms.into_iter().map(|(definition, stands)| (set.spec(definition).id.clone(), stands)).collect();
    named.sort();
    named
}

/// ⭐ TWO PARTICIPANTS, APART: a second seat joins beside the player at Alice
/// and stays there while the player carries the note to Bob in the next room.
/// The hand-over opens Alice's return lock in the second participant's live
/// room, while the two rooms are live at once and neither body moves between
/// them.
#[test]
fn the_hand_over_opens_the_wall_in_the_other_participants_room() {
    // Two pads, so the session holds a seat for a second player.
    let mut app = crate::the_note_travels_from_alice_to_bob::a_returning_players_session_with_pads(2);
    for room in ROUTE_TO_ALICE {
        go_through(&mut app, room);
    }
    talk_and_choose(&mut app, "npc_alice", "Take the note.");

    let second = join_the_second_seat(&mut app);
    assert_eq!(room_of(&app, second).as_deref(), Some("alice_relay"), "the second seat joins in the player's room");

    go_through(&mut app, "bob_relay");
    assert_eq!(room_of(&app, second).as_deref(), Some("alice_relay"), "the second participant stayed at Alice");
    let apart = gates_by_live_room(&mut app, RETURN_LOCK);
    assert_eq!(
        apart,
        vec![("alice_relay".to_string(), true), ("bob_relay".to_string(), false)],
        "(live room, return lock stands) with the two participants apart, before the hand-over"
    );

    talk_and_choose(&mut app, "npc_bob", "Hand him the sealed note.");
    assert_eq!(flag(&app, SURVEY_FLAG), true, "Bob took the note");
    for _ in 0..10 {
        app.update();
    }
    let player = primary_body(&mut app);
    assert_eq!(
        (room_of(&app, player).as_deref(), room_of(&app, second).as_deref()),
        (Some("bob_relay"), Some("alice_relay")),
        "the two participants are still apart"
    );
    assert_eq!(
        gates_by_live_room(&mut app, RETURN_LOCK),
        vec![("alice_relay".to_string(), false), ("bob_relay".to_string(), false)],
        "(live room, return lock stands) after the hand-over in the other room"
    );
}

/// The player dies (a hazard), and the death beat runs until the body is back
/// in play.
fn die(app: &mut App) {
    use ambition_platformer2d::combat::death_rules::{ActorDiedMessage, DeathCause, OutOfPlay};
    let body = primary_body(app);
    let pos = app.world().get::<ae::BodyKinematics>(body).expect("a body").pos;
    app.world_mut().write_message(ActorDiedMessage {
        victim: body,
        pos,
        cause: DeathCause {
            source: ambition_platformer2d::combat::HitSource::Hazard,
            attacker: None,
        },
    });
    app.update();
    assert!(app.world().entity(body).contains::<OutOfPlay>(), "control: the dead player is out of play");
    for _ in 0..600 {
        app.update();
        let body = primary_body(app);
        if !app.world().entity(body).contains::<OutOfPlay>() {
            for _ in 0..60 {
                app.update();
            }
            return;
        }
    }
    panic!("the player never came back into play after the death");
}

/// ⭐ THE DEATH STEP, the part Q164 does not decide. The player carries the
/// Blink to Alice and Bob and dies with no checkpoint taken. The Blink was
/// acquired after the checkpoint, so it goes back (Q124): it is not in the
/// hand, not dropped where the player died, and not copied; it lies where it
/// was found, and it can be taken again. Alice's return lock agrees with the
/// survey flag whichever way Q164 rules on the flag.
#[test]
fn a_death_sends_the_blink_back_to_where_it_lay() {
    let mut app = a_returning_players_session();
    let blink = SimId::placement(BLINK);
    let found_at = take(&mut app, &blink);
    for room in ROUTE_TO_ALICE {
        go_through(&mut app, room);
    }
    talk_and_choose(&mut app, "npc_alice", "Take the note.");
    go_through(&mut app, "bob_relay");
    talk_and_choose(&mut app, "npc_bob", "Hand him the sealed note.");
    go_through(&mut app, "alice_relay");
    assert!(the_player_holds(&mut app, &blink), "control: the Blink is in hand when the player dies");

    die(&mut app);
    assert_eq!(live_room(&mut app), "alice_relay", "with no checkpoint, the player comes back in the room of the death");
    assert_eq!(
        occurrences(&mut app, &blink),
        vec![],
        "after the death, a Blink is in the live world (in hand, or dropped where the player died)"
    );
    assert_eq!(
        gate_stands(&app, RETURN_LOCK),
        !flag(&app, SURVEY_FLAG),
        "after the death, Alice's return lock disagrees with the survey flag"
    );

    for room in ["under_town_pipes", "drain_alley", "intro_escape_shaft", "intro_raid_corridor", "intro_wake_room", "central_hub_complex"] {
        go_through(&mut app, room);
    }
    let lying: Vec<ae::Vec2> = {
        let world = app.world_mut();
        world
            .query::<(&SimId, &Ground, &Custody)>()
            .iter(world)
            .filter(|(id, _, custody)| *id == &blink && custody.in_world())
            .map(|(_, ground, _)| ground.pos)
            .collect()
    };
    assert_eq!(lying, vec![found_at], "back in the hub, the Blinks lying in the world");
    take(&mut app, &blink);
}
