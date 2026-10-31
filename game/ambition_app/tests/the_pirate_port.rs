//! The Pirate Port (gate address SAILING): the gate plaza, the waterfront,
//! the tavern and the docks (`tools/ambition_ldtk_tools/specs/pirate_port/`).
//!
//! The shipped session puts the player in the gate plaza, where the gate
//! arrives. From there the player walks by keys (`common::walk`): the low road
//! east to the docks, the brig's yards to the crow's nest, and the high road
//! back west over the roofs to the plaza. It talks to each resident on the
//! way, takes the clerk's page and gives it back, and goes through the door
//! to the pirate cove and back. No step puts the body anywhere after the
//! arrival.

use ambition_platformer2d::characters::control::PlayerSlot;
use ambition_platformer2d::engine_core as ae;
use bevy::prelude::{App, KeyCode};

use crate::common::walk::{Keys, walk_through, walk_through_zone, walk_to};
use crate::neighbor_prefetch_prepares_rooms::{alice as primary_body, cross, room_of};
use crate::the_note_travels_from_alice_to_bob::{
    dialog_active, first_choice, flag, npc, tap, the_shipped_session,
};

const PLAZA: &str = "pirate_port_gate_plaza";
const WATERFRONT: &str = "pirate_port_waterfront";
const TAVERN: &str = "pirate_port_tavern";
const DOCKS: &str = "pirate_port_docks";
const COVE: &str = "pirate_cove";

pub(crate) fn live_room(app: &mut App) -> String {
    let body = primary_body(app);
    room_of(app, body).expect("the player is in a live room")
}

fn settle(app: &mut App) {
    for _ in 0..30 {
        app.update();
    }
}

/// The session, with the player in `room` (where a gate arrives).
pub(crate) fn a_session_in(room: &str) -> App {
    let mut app = the_shipped_session();
    let body = primary_body(&mut app);
    cross(&mut app, body, PlayerSlot(0), room);
    settle(&mut app);
    assert_eq!(live_room(&mut app), room);
    app
}

/// Walk by keys through the exit of the live room to `target`; `zone` names
/// the exit when two lead there.
pub(crate) fn go(app: &mut App, target: &str, zone: Option<&str>) {
    let from = live_room(app);
    let walked = match zone {
        Some(zone) => walk_through_zone(&mut Keys::new(app), target, zone),
        None => walk_through(&mut Keys::new(app), target),
    };
    if let Err(why) = walked {
        panic!("the player did not walk from `{from}` to `{target}` ({zone:?}): {why}");
    }
    settle(app);
    assert_eq!(live_room(app), target);
}

/// Whether the breakable `id` of the live room is broken (`None`: no live
/// room builds it).
pub(crate) fn wall_broken(app: &mut App, id: &str) -> Option<bool> {
    use ambition_platformer2d::breakables::BreakableFeature;
    use ambition_platformer2d::combat::components::FeatureId;
    let world = app.world_mut();
    let mut walls = world.query::<(&FeatureId, &BreakableFeature)>();
    walls.iter(world).find(|(wall, _)| wall.as_str() == id).map(|(_, wall)| wall.broken())
}

/// Walk by keys to `at`, face the breakable wall `id` (`facing` -1 west, 1
/// east) and hit it until it breaks, as a player does.
pub(crate) fn break_the_wall(app: &mut App, id: &str, at: ae::Vec2, facing: f32) {
    use crate::common::walk::{Press, Walker};
    assert_eq!(wall_broken(app, id), Some(false), "control: the wall {id} stands");
    go_to(app, at, id);
    let mut swings = 0;
    while wall_broken(app, id) == Some(false) {
        assert!(swings < 12, "the wall {id} did not break after {swings} swings");
        let mut keys = Keys::new(app);
        keys.step(Press { axis: facing, ..Press::default() });
        keys.step(Press { attack: true, ..Press::default() });
        for _ in 0..24 {
            keys.step(Press::default());
        }
        drop(keys);
        swings += 1;
    }
}

/// Walk by keys to stand at `at` in the live room.
pub(crate) fn go_to(app: &mut App, at: ae::Vec2, what: &str) {
    let room = live_room(app);
    if let Err(why) = walk_to(&mut Keys::new(app), at) {
        panic!("the player did not walk to {what} in `{room}`: {why}");
    }
    for _ in 0..10 {
        app.update();
    }
}

/// Walk by keys to the resident `character`, press Interact, take the first
/// choice (it must read `choice`), and press through the rest. Answers what
/// the resident said first.
///
/// A resident can move while the player walks: in a dangerous room it joins a
/// fight the walk starts (`body::rally_allies_to_the_fight`). So the walk goes
/// again to where it stands now, up to three times, before the talk fails.
pub(crate) fn talk(app: &mut App, character: &str, choice: &str) -> String {
    let mut opened = false;
    for _ in 0..3 {
        let at = {
            let body = npc(app, character);
            app.world().get::<ae::BodyKinematics>(body).expect("a live resident").pos
        };
        go_to(app, at, character);
        tap(app, KeyCode::KeyF);
        for _ in 0..30 {
            app.update();
            if dialog_active(app) {
                opened = true;
                break;
            }
        }
        if opened {
            break;
        }
    }
    assert!(opened, "Interact beside `{character}` in `{}` did not open its talk", live_room(app));
    let mut offered = None;
    for _ in 0..20 {
        if let Some(label) = first_choice(app) {
            offered = Some(label);
            break;
        }
        tap(app, KeyCode::Enter);
    }
    assert_eq!(offered.as_deref(), Some(choice), "`{character}` did not offer `{choice}` first");
    tap(app, KeyCode::Enter);
    for _ in 0..20 {
        if !dialog_active(app) {
            break;
        }
        tap(app, KeyCode::Enter);
    }
    assert!(!dialog_active(app), "the talk with `{character}` did not end");
    settle(app);
    offered.unwrap()
}

pub(crate) fn quest_completed(app: &App, id: &str) -> bool {
    app.world()
        .resource::<ambition_content::quest::QuestRegistry>()
        .get(id)
        .is_some_and(|state| {
            state.progression == ambition_platformer2d::persistence::save_data::PersistedQuestState::Completed
        })
}

/// Where the pickup that sets `flag` lies in the live room.
pub(crate) fn pickup_at(app: &mut App, flag_id: &str) -> ae::Vec2 {
    let world = app.world_mut();
    world
        .query::<(&ambition_platformer2d::item_runtime::pickups::PickupFeature, &ae::geometry::CenteredAabb)>()
        .iter(world)
        .find(|(feature, _)| {
            matches!(&feature.pickup.kind, ambition_platformer2d::interaction::PickupKind::StoryFlag { flag } if flag == flag_id)
        })
        .map(|(_, aabb)| aabb.center)
        .unwrap_or_else(|| panic!("the room has the pickup of `{flag_id}`"))
}

/// ⭐ THE PORT IS A PLACE TO WALK AND TALK. The low road goes east from the
/// gate plaza to the docks; the brig's yards go up to the crow's nest, where
/// the clerk's page is; the high road goes back west over the roofs to the
/// plaza's balcony. Each resident talks. The page goes back to the clerk and
/// the quest is done. The docks' door goes to the pirate cove and back.
#[test]
fn the_pirate_port_is_walked_round_its_loop_and_its_people_talk() {
    let mut app = a_session_in(PLAZA);

    talk(&mut app, "npc_port_ines", "You look worried.");
    assert!(flag(&app, "npc_pirate_port_ines_talked"), "talking to Ines is recorded");
    talk(&mut app, "npc_port_tomas", "Is the gate always like this?");

    // The low road.
    go(&mut app, WATERFRONT, Some("plaza_east_low"));
    talk(&mut app, "npc_port_agnes", "Ginger for the gate-sick?");
    go(&mut app, TAVERN, None);
    talk(&mut app, "npc_port_wren", "What do you serve?");
    talk(&mut app, "npc_pirate_heavy_broadside_bess", "Do you crew from the cove?",
    );
    go(&mut app, WATERFRONT, None);
    go(&mut app, DOCKS, Some("waterfront_east_low"));
    talk(&mut app, "npc_port_jory", "What do you catch?");

    // The way on to the cove, and back.
    talk(&mut app, "npc_port_bram", "What is past that door?");
    go(&mut app, COVE, None);
    go(&mut app, DOCKS, Some("pirate_cove_port_door"));

    // Up the brig to the crow's nest, for the page.
    assert!(!flag(&app, "pirate_port_manifest_found"), "control: the page is not taken before the climb");
    let page = pickup_at(&mut app, "pirate_port_manifest_found");
    go_to(&mut app, page, "the page in the crow's nest");
    assert!(flag(&app, "pirate_port_manifest_found"), "the page is taken in the crow's nest at {page:?}");

    // The high road home.
    go(&mut app, WATERFRONT, Some("docks_west_high"));
    go(&mut app, PLAZA, Some("waterfront_west_high"));
    let at = primary_body(&mut app);
    let y = app.world().get::<ae::BodyKinematics>(at).expect("a body").pos.y;
    assert!(y < 500.0, "the high road arrives on the plaza's balcony (y {y})");

    assert!(!quest_completed(&app, "pirate_port_lost_manifest"), "control: the quest waits for the page to go back");
    talk(&mut app, "npc_port_ines", "Hand her the page.");
    assert!(flag(&app, "pirate_port_manifest_returned"), "Ines has her page");
    assert!(quest_completed(&app, "pirate_port_lost_manifest"), "the quest is done");
}
