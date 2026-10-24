//! THE CONSIGNMENT ERRAND: one modest reason to travel between dimensions.
//!
//! Customs on the orbital station of the Science dimension holds crate 0447
//! (survey lenses from the Math town) because its manifest counts the lenses
//! as "aleph-null". The robot asks Officer Okafor, goes to the Math town's
//! Surveyor's Office, where Georg Canter signs a finite count, brings it
//! back, and goes home to the hub. The quest `orbital_consignment` records
//! each step; clearing the crate opens the freight lane's gates and the hatch
//! of the damaged spur, and the save keeps all of it.
//!
//! The shipped shell session, driven with real key presses: Interact starts a
//! conversation and Enter takes the first choice. Within a district the
//! robot walks through the real doors. Between dimensions the test crosses by
//! the session's lifecycle transition, where a player dials a gate (the gate
//! network places the rings).

use ambition_platformer2d::characters::control::PlayerSlot;
use bevy::prelude::App;

use crate::neighbor_prefetch_prepares_rooms::{alice as primary_body, cross, room_of};
use crate::the_note_travels_from_alice_to_bob::{the_shipped_session, bag, flag, talk_and_choose};

const QUEST: &str = "orbital_consignment";
const DOCK: &str = "orbital_station_gate_dock";

fn step_of(app: &App) -> Option<(u8, bool)> {
    app.world()
        .resource::<ambition_content::quest::QuestRegistry>()
        .get(QUEST)
        .map(|state| (state.step, state.progression == ambition_platformer2d::persistence::save_data::PersistedQuestState::Completed))
}

fn live_room(app: &mut App) -> String {
    let body = primary_body(app);
    room_of(app, body).expect("the player is in a live room")
}

/// Walk out of the live room to `target`, by keys, through the real exit.
fn go_through(app: &mut App, target: &str) {
    let from = live_room(app);
    if let Err(why) = crate::common::walk::walk_through(&mut crate::common::walk::Keys::new(app), target) {
        panic!("the player did not walk from `{from}` to `{target}`: {why}");
    }
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(live_room(app), target);
}

/// Cross to `target` by the session's lifecycle transition: where a player
/// dials a gate.
fn travel(app: &mut App, target: &str) {
    let body = primary_body(app);
    cross(app, body, PlayerSlot(0), target);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(live_room(app), target);
}

fn gate_stands(app: &App, block: &str) -> bool {
    ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<
        ambition_platformer2d::world::FeatureEcsWorldOverlay,
    >(app.world())
    .expect("the live room has a collision overlay")
    .gate_solids
    .iter()
    .any(|solid| solid.name.contains(block))
}

fn the_save_file(
    app: &mut App,
) -> (ambition_platformer2d::persistence::PersistenceRoot, ambition_platformer2d::persistence::save_data::AmbitionGameSaveData) {
    use ambition_platformer2d::persistence::save;
    for _ in 0..5 {
        app.update();
    }
    let world = app.world();
    let root = world.resource::<ambition_platformer2d::persistence::PersistenceRoot>().clone();
    let owner = world.resource::<save::SaveOwner>().current().to_owned();
    let path = save::save_path_for(&root.0, &owner).expect("the live save's owner has a file of its own");
    let loaded = save::load_save(&path);
    assert!(loaded.present, "the autosave wrote no file at {}", path.display());
    (root, loaded.data)
}

fn a_session_loaded_from(root: &ambition_platformer2d::persistence::PersistenceRoot) -> App {
    use ambition_app::app::{build_visible_app, VisibleRenderMode};
    use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(root.clone());
    app.finish();
    app.update();
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: ShellRouteId::new("ambition_gameplay"),
        request: None,
    });
    crate::common::step_until_route_is_active_and_settled(&mut app, "ambition_gameplay");
    app
}

/// ⭐ CONSIGNMENT 0447, END TO END: station, Math town, station, home; then
/// a new App reads the save and finds the errand done and its consequences
/// in place.
#[test]
fn consignment_0447_is_cleared_across_two_dimensions_and_remembered() {
    let mut app = the_shipped_session();
    assert_eq!(step_of(&app), Some((0, false)), "the errand is open from the start, at its first step");

    // The station: customs holds the crate; the freight lane is shut.
    travel(&mut app, DOCK);
    assert!(gate_stands(&app, "dock_quarantine_gate"), "the quarantine pen is shut before the errand");
    talk_and_choose(&mut app, "npc_station_okafor", "I can ask the sender.");
    assert!(flag(&app, "orbital_consignment_query"));
    assert_eq!(step_of(&app), Some((1, false)));

    // The Math town: down the street to the Surveyor's Office, by its doors.
    travel(&mut app, "math_gate_hall");
    go_through(&mut app, "math_town_street");
    go_through(&mut app, "math_surveyors_office");
    talk_and_choose(&mut app, "npc_georg_canter", "Please count the lenses.");
    assert!(flag(&app, "orbital_manifest_signed"));
    assert_eq!(step_of(&app), Some((2, false)));

    // Back to customs: cleared, the pen opens, and the spur's pass is given.
    travel(&mut app, DOCK);
    talk_and_choose(&mut app, "npc_station_okafor", "Clear the crate.");
    assert!(flag(&app, "orbital_consignment_cleared") && flag(&app, "orbital_spur_pass"));
    assert_eq!(step_of(&app), Some((3, false)));
    for _ in 0..30 {
        app.update();
    }
    assert!(!gate_stands(&app, "dock_quarantine_gate"), "the quarantine pen opens when the crate is cleared");
    go_through(&mut app, "orbital_station_docking_bay");
    assert!(!gate_stands(&app, "bay_quarantine_gate"), "the bay's end of the freight lane is open too");

    // Home: the errand is complete and pays once.
    travel(&mut app, "central_hub_complex");
    for _ in 0..30 {
        app.update();
    }
    let (step, complete) = step_of(&app).expect("the errand is in the registry");
    assert!(complete, "home in the hub, the errand is complete (step {step})");
    assert!(flag(&app, "orbital_consignment_reward_granted"), "the reward is paid");
    let batteries = bag(&app, "sparebattery");
    assert!(batteries >= 1, "the reward's battery is in the bag");

    // Save, and a new App reads it back.
    let (root, written) = the_save_file(&mut app);
    drop(app);
    let mut app = a_session_loaded_from(&root);
    assert_eq!(
        app.world().resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>().data(),
        &written,
        "the new App's save is the file the first App wrote"
    );
    assert!(step_of(&app).is_some_and(|(_, complete)| complete), "after the load, the errand is complete");
    assert_eq!(bag(&app, "sparebattery"), batteries, "after the load, the reward is paid once");
    travel(&mut app, DOCK);
    assert!(!gate_stands(&app, "dock_quarantine_gate"), "after the load, the quarantine pen is open");
    travel(&mut app, "orbital_station_service_deck");
    assert!(!gate_stands(&app, "service_spur_hatch"), "after the load, the spur's hatch is open");
    talk_and_choose(&mut app, "npc_station_rahimi", "What is in there?");
}
