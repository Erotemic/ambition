//! the App-level test below builds its own world, which is the shape that
//! can pass with production wiring absent. It is here because the invariant it
//! pins is a *cache invalidation* one, and provoking a hot-reload against a live
//! host is a much worse test than provoking it against three resources.

use super::*;

use ambition_platformer2d_ldtk::{
    ActiveLdtkProject, LdtkEntityInstance, LdtkFieldInstance, LdtkLayerInstance, LdtkLevel,
    LdtkProject,
};
use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;
use ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay;
use serde_json::Value;

const FLAG: &str = "bob_field_survey_received";

fn field(identifier: &str, value: &str) -> LdtkFieldInstance {
    LdtkFieldInstance {
        identifier: identifier.into(),
        value: Value::String(value.into()),
        real_editor_values: vec![Value::Null],
    }
}

/// One level whose `activeArea` is `alice_relay`, holding one `LockWall`.
///
/// `gated_by` present by default: the interesting negative case is a wall
/// WITHOUT it, and a fixture whose default is the boring case makes the negative
/// test a one-line edit.
fn project_with_one_wall(gated_by: Option<&str>) -> LdtkProject {
    let mut fields = vec![
        field("id", "alice_private_return_lock"),
        field("name", "wall"),
    ];
    if let Some(gated_by) = gated_by {
        fields.push(field("gated_by", gated_by));
    }
    LdtkProject {
        json_version: "1.5.3".into(),
        levels: vec![LdtkLevel {
            identifier: "alice_relay".into(),
            iid: "level-iid".into(),
            world_x: 0,
            world_y: 0,
            px_wid: 1024,
            px_hei: 768,
            field_instances: vec![field("activeArea", "alice_relay")],
            layer_instances: vec![LdtkLayerInstance {
                identifier: "Ambition".into(),
                layer_type: "Entities".into(),
                c_wid: 64,
                c_hei: 48,
                grid_size: 16,
                entity_instances: vec![
                    // a real level has one and the converter REFUSES an area without it ("no
                    // PlayerStart").
                    LdtkEntityInstance {
                        iid: "PlayerStart-test-alice".into(),
                        identifier: "PlayerStart".into(),
                        pivot: vec![0.0, 0.0],
                        px: [96, 96],
                        width: 16,
                        height: 16,
                        field_instances: Vec::new(),
                    },
                    LdtkEntityInstance {
                        iid: "LockWall-test-alice".into(),
                        identifier: "LockWall".into(),
                        pivot: vec![0.0, 0.0],
                        px: [800, 624],
                        width: 96,
                        height: 112,
                        field_instances: fields,
                    },
                ],
                int_grid_csv: Vec::new(),
                grid_tiles: Vec::new(),
            }],
        }],
    }
}

/// The fixture project, CONVERTED — the road production takes.
///
/// Converting here means a converter that stops emitting either field fails in these tests.
fn room_with_one_wall(
    gated_by: Option<&str>,
    room_id: &str,
) -> ambition_platformer2d_world::rooms::RoomSpec {
    project_with_one_wall(gated_by)
        .to_room_set_with_entry(
            "alice_relay",
            &ambition_platformer2d_ldtk::LdtkVocabulary::engine(),
        )
        .unwrap_or_else(|errors| panic!("fixture converts to rooms: {errors:?}"))
        .rooms
        .into_iter()
        .find(|room| room.id == room_id)
        .unwrap_or_else(|| {
            ambition_platformer2d_world::rooms::RoomSpec::new(
                room_id,
                ambition_platformer2d_core::World::new(
                    room_id,
                    ambition_platformer2d_core::Vec2::new(1024.0, 768.0),
                    ambition_platformer2d_core::Vec2::new(96.0, 96.0),
                    Vec::new(),
                ),
            )
        })
}

/// The walk finds an authored gated wall, with its footprint.
#[test]
fn an_authored_gated_wall_is_found_with_its_footprint() {
    let walls = authored_gated_lock_walls(&room_with_one_wall(Some(FLAG), "alice_relay"));
    assert_eq!(walls.len(), 1);
    assert_eq!(walls[0].id, "alice_private_return_lock");
    assert_eq!(walls[0].gated_by, FLAG);
    assert_eq!(
        walls[0].min,
        ambition_platformer2d_core::Vec2::new(800.0, 624.0)
    );
    assert_eq!(
        walls[0].size,
        ambition_platformer2d_core::Vec2::new(96.0, 112.0)
    );
}

/// A `LockWall` with no `gated_by` is not this system's business.
///
/// Encounter walls — the other consumer of `LockWall` — are exactly the walls that carry no
/// `gated_by`, and they must keep working.
#[test]
fn a_wall_with_no_authored_gate_is_left_to_its_other_consumer() {
    assert!(authored_gated_lock_walls(&room_with_one_wall(None, "alice_relay")).is_empty());
}

/// Only the active room's walls.
#[test]
fn walls_in_another_room_are_not_found() {
    assert!(authored_gated_lock_walls(&room_with_one_wall(Some(FLAG), "drain_alley")).is_empty());
}

/// A world with the system, its inputs, and the one condition it asks.
fn world_with_one_gated_wall() -> App {
    let mut app = App::new();
    app.insert_resource(ActiveLdtkProject(project_with_one_wall(Some(FLAG))));
    app.insert_resource(ambition_persistence::save::AmbitionGameSave::default());
    app.insert_resource(FeatureEcsWorldOverlay::default());
    // the world-fact domain's own condition, published exactly as its plugin
    // publishes it. The system under test never names a flag.
    app.publish_condition(
        crate::world_facts::flag_set_descriptor(),
        crate::world_facts::flag_set,
    );
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "alice_relay",
            vec![room_with_one_wall(Some(FLAG), "alice_relay")],
            Vec::new(),
        ),
    );
    // Mirror the production ordering contract: the overlay rebuild clears
    // `gate_solids` each frame BEFORE this system re-contributes.
    app.add_systems(
        Update,
        (
            |mut overlay: ResMut<FeatureEcsWorldOverlay>| overlay.gate_solids.clear(),
            sync_authored_gated_lock_walls,
        )
            .chain(),
    );
    app
}

fn standing(app: &App) -> usize {
    app.world()
        .resource::<FeatureEcsWorldOverlay>()
        .gate_solids
        .len()
}

/// THE WALL STANDS UNTIL ITS CONDITION IS SATISFIED, AND THEN IT IS GONE.
///
/// nothing here reads a flag. The system asks `world.flag_set`, the world-fact
/// domain answers, and the wall follows — which is the whole reason this stopped
/// being a const table.
#[test]
fn the_wall_stands_until_its_authored_condition_is_satisfied() {
    let mut app = world_with_one_gated_wall();
    app.update();
    assert_eq!(standing(&app), 1, "the flag is clear, so the wall is up");

    app.world_mut()
        .resource_mut::<ambition_persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_flag(FLAG, true);
    app.update();
    assert_eq!(
        standing(&app),
        0,
        "the condition is satisfied; the wall opens"
    );
}

/// A REPLACED ROOM SET INVALIDATES THE CACHE.
///
/// this is the regression the original cache shipped WITHOUT, and it is
/// carried across deliberately: a hot reload that swaps the authored source
/// under an unchanged room id and save state kept serving walls computed from
/// data that is no longer loaded.
#[test]
fn swapping_the_room_set_alone_invalidates_the_cached_walls() {
    let mut app = world_with_one_gated_wall();
    app.update();
    assert_eq!(standing(&app), 1);

    // A quiet frame keeps serving the cached wall — this is what makes the next
    // assertion about invalidation rather than about recomputation.
    app.update();
    assert_eq!(standing(&app), 1);

    // Same room id, same save, different authored content.
    {
        let mut rooms = app.world_mut().query_filtered::<
            &mut ambition_platformer2d_world::rooms::RoomSet,
            bevy::prelude::With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
        >();
        let mut set = rooms
            .iter_mut(app.world_mut())
            .next()
            .expect("the fixture installs a room set");
        for room in &mut set.rooms {
            room.lock_walls.clear();
        }
    }
    app.update();
    assert_eq!(
        standing(&app),
        0,
        "the wall set must track the replaced room set"
    );
}

/// A QUESTION THAT CANNOT BE PREPARED YET LEAVES THE WALL STANDING — AND IS
/// RETRIED.
///
/// ⭐⭐ THE CACHE HOLDS A PREPARED QUESTION NOW, and preparation happens when the
/// room is cached. That buys a wall a `PreparedCondition` instead of a freshly
/// minted argument every frame — and it introduces an ORDER the old code could
/// not have: a provider that registers AFTER the first room is cached would have
/// left its walls holding `None` forever, which is a gate that never opens
/// because of startup sequence rather than because of the world.
///
/// ⛔ BOTH ARMS, because one cannot show the rule: the wall must STAND while the
/// question is unpreparable (the same safe direction an unanswerable question
/// takes) and must OPEN once the provider arrives and the flag is set. A test
/// that only checked the first would pass over a permanent `None`.
#[test]
fn a_wall_whose_question_cannot_be_prepared_yet_stands_and_is_retried() {
    use ambition_platformer2d_shared_tangle::authored_logic::ConditionCatalog;

    let mut app = App::new();
    app.insert_resource(ActiveLdtkProject(project_with_one_wall(Some(FLAG))));
    app.insert_resource(ambition_persistence::save::AmbitionGameSave::default());
    app.insert_resource(FeatureEcsWorldOverlay::default());
    // A catalog that exists and does NOT publish `world.flag_set`. The system
    // returns early when there is no catalog at all, so an empty one is what puts
    // the fixture in the state under test rather than past it.
    app.init_resource::<ConditionCatalog>();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "alice_relay",
            vec![room_with_one_wall(Some(FLAG), "alice_relay")],
            Vec::new(),
        ),
    );
    app.add_systems(
        Update,
        (
            |mut overlay: ResMut<FeatureEcsWorldOverlay>| overlay.gate_solids.clear(),
            sync_authored_gated_lock_walls,
        )
            .chain(),
    );

    app.update();
    assert_eq!(
        standing(&app),
        1,
        "nobody can answer this wall's question yet, so it must stay up — a gate \
         that opened because its provider had not registered would open in \
         exactly the situations where the world is least well understood"
    );

    // The provider arrives, and the flag it answers about is set.
    app.publish_condition(
        crate::world_facts::flag_set_descriptor(),
        crate::world_facts::flag_set,
    );
    app.world_mut()
        .resource_mut::<ambition_persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_flag(FLAG, true);
    app.update();
    assert_eq!(
        standing(&app),
        0,
        "the provider registered after the room was cached and the wall never \
         retried its preparation, so it stands forever on a satisfied condition"
    );
}
