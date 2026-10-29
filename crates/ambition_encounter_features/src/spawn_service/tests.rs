//! Encounter spawn requests produce a body only in a live generation.

use bevy::prelude::*;

/// A spawn REQUEST from the encounter domain must produce a real body.
///
/// ⛔ THE GAP THIS CLOSES. Until 2026-09-03 the wave driver served its own
/// `SpawnCommand`s inline, and the encounter tests assert only that the EVENT is
/// emitted — so the whole request→body path had no test, and 1213 tests stayed
/// green through a change that moved it. The ordering guard in
/// `encounter::spawn_request_service_order` pins the WIRING (server after
/// driver); this pins the BEHAVIOUR.
#[test]
fn a_spawn_request_on_the_bus_becomes_a_body() {
    let mut app = spawn_request_app();
    app.init_resource::<ambition_platformer2d_construction::mechanics::SessionMechanics>();
    app.update();
    assert_eq!(
        bodies_built(&mut app),
        1,
        "one SpawnCommand on the bus must build exactly one body — the encounter \
         domain asks and this crate constructs"
    );
}


/// With no generation, no session runs (`SessionMechanics`), and the service
/// builds no body.
#[test]
fn a_spawn_request_with_no_generation_builds_no_body() {
    let mut app = spawn_request_app();
    app.update();
    assert_eq!(bodies_built(&mut app), 0, "a body was built with no generation");
}


/// The spawn service alone, with one `SpawnCommand` on the bus and no
/// generation.
fn spawn_request_app() -> App {
    let mut app = App::new();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.insert_resource(ambition_characters::prepared_fixtures::fixture_cast(&["fixture_striker"]));
    app.add_message::<ambition_encounter::EncounterEventMsg>();
    app.add_systems(Update, super::serve_encounter_spawn_commands);

    app.world_mut()
        .resource_mut::<bevy::prelude::Messages<ambition_encounter::EncounterEventMsg>>()
        .write(ambition_encounter::EncounterEventMsg::new(
            "test_encounter",
            ambition_encounter::EncounterEvent::SpawnCommand {
                id: "wave_mob_1".to_string(),
                character: Some("fixture_striker".to_string()),
                kind: "fixture_striker".to_string(),
                pos: [100.0, 100.0],
                size: [20.0, 30.0],
            },
        ));
    app
}


fn bodies_built(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query::<&ambition_characters::brain::Brain>();
    q.iter(app.world()).count()
}
