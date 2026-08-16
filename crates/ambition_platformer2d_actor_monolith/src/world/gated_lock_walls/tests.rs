//! ⭐ **these came from `ambition_content` with the capability they cover**
//! (2026-08-15). Same invariants, restated against the authored field rather
//! than the Rust const table that used to hold the pairing — which is exactly
//! the change under test.
//!
//! ⚠ **the App-level test below builds its own world, which is the shape that
//! can pass with production wiring absent.** It is here because the invariant it
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
    let mut fields = vec![field("id", "alice_private_return_lock"), field("name", "wall")];
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
                entity_instances: vec![LdtkEntityInstance {
                    iid: "LockWall-test-alice".into(),
                    identifier: "LockWall".into(),
                    pivot: vec![0.0, 0.0],
                    px: [800, 624],
                    width: 96,
                    height: 112,
                    field_instances: fields,
                }],
                int_grid_csv: Vec::new(),
                grid_tiles: Vec::new(),
            }],
        }],
    }
}

/// **The walk finds an authored gated wall, with its footprint.**
#[test]
fn an_authored_gated_wall_is_found_with_its_footprint() {
    let walls = authored_gated_lock_walls(&project_with_one_wall(Some(FLAG)), "alice_relay");
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

/// **A `LockWall` with no `gated_by` is not this system's business.**
///
/// ⭐ this is the old *"ignores unregistered ids"* test, and the migration
/// improved it: membership of a Rust table became the presence of an authored
/// field, so the same invariant now says something an author can see. Encounter
/// walls — the other consumer of `LockWall` — are exactly the walls that carry no
/// `gated_by`, and they must keep working.
#[test]
fn a_wall_with_no_authored_gate_is_left_to_its_other_consumer() {
    assert!(authored_gated_lock_walls(&project_with_one_wall(None), "alice_relay").is_empty());
}

/// **Only the active room's walls.**
#[test]
fn walls_in_another_room_are_not_found() {
    assert!(authored_gated_lock_walls(&project_with_one_wall(Some(FLAG)), "drain_alley").is_empty());
}

/// A world with the system, its inputs, and the one condition it asks.
fn world_with_one_gated_wall() -> App {
    let mut app = App::new();
    app.insert_resource(ActiveLdtkProject(project_with_one_wall(Some(FLAG))));
    app.insert_resource(ambition_persistence::save::AmbitionGameSave::default());
    app.insert_resource(FeatureEcsWorldOverlay::default());
    // ⭐ the world-fact domain's own condition, published exactly as its plugin
    // publishes it. The system under test never names a flag.
    app.publish_condition(
        crate::world_facts::flag_set_descriptor(),
        crate::world_facts::flag_set,
    );
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        crate::rooms::RoomSet::from_parts(
            "alice_relay",
            vec![crate::rooms::RoomSpec::new(
                "alice_relay",
                ambition_platformer2d_core::World::new(
                    "alice_relay",
                    ambition_platformer2d_core::Vec2::new(1024.0, 768.0),
                    ambition_platformer2d_core::Vec2::ZERO,
                    Vec::new(),
                ),
            )],
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

/// **THE WALL STANDS UNTIL ITS CONDITION IS SATISFIED, AND THEN IT IS GONE.**
///
/// ⭐ nothing here reads a flag. The system asks `world.flag_set`, the world-fact
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
    assert_eq!(standing(&app), 0, "the condition is satisfied; the wall opens");
}

/// **A REPLACED PROJECT INVALIDATES THE CACHE.**
///
/// ⛔⛔ **this is the regression the original cache shipped WITHOUT**, and it is
/// carried across deliberately: a hot reload that swaps the LDtk project under an
/// unchanged room id and save state kept serving walls computed from the project
/// that is no longer loaded. The cache has three inputs and the project is the
/// one that is easy to forget, because the other two are the ones you think about
/// while writing the feature.
#[test]
fn swapping_the_project_alone_invalidates_the_cached_walls() {
    let mut app = world_with_one_gated_wall();
    app.update();
    assert_eq!(standing(&app), 1);

    // A quiet frame keeps serving the cached wall — this is what makes the next
    // assertion about invalidation rather than about recomputation.
    app.update();
    assert_eq!(standing(&app), 1);

    // Same room id, same save, different project.
    app.world_mut()
        .resource_mut::<ActiveLdtkProject>()
        .0
        .levels[0]
        .layer_instances[0]
        .entity_instances
        .clear();
    app.update();
    assert_eq!(
        standing(&app),
        0,
        "the wall set must track the replaced project"
    );
}
