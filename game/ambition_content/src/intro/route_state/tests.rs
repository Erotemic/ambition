//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

#[test]
fn chain_table_has_no_duplicate_triggers() {
    // Two chains with the same trigger would emit redundant SetFlag
    // effects every frame. Forbid that at compile-time-style check.
    let mut triggers = std::collections::BTreeSet::new();
    for (trigger, _target) in INTRO_FLAG_CHAINS.iter().copied() {
        assert!(
            triggers.insert(trigger),
            "duplicate trigger in INTRO_FLAG_CHAINS: {trigger}"
        );
    }
}

#[test]
fn chain_table_has_no_trigger_equals_target() {
    for (trigger, target) in INTRO_FLAG_CHAINS.iter().copied() {
        assert_ne!(trigger, target, "chain trigger == target: {trigger}");
    }
}

/// Hand-build a minimal LdtkProject with a single level whose
/// activeArea = "alice_relay" and one LockWall entity matching a
/// known intro gated lock id.
fn synthetic_alice_relay_project() -> ambition_actors::world::ldtk_world::LdtkProject {
    use ambition_actors::world::ldtk_world::{
        LdtkEntityInstance, LdtkFieldInstance, LdtkLayerInstance, LdtkLevel, LdtkProject,
    };
    use serde_json::Value;

    let lock_wall = LdtkEntityInstance {
        iid: "LockWall-test-alice".into(),
        identifier: "LockWall".into(),
        pivot: vec![0.0, 0.0],
        px: [800, 624],
        width: 96,
        height: 112,
        field_instances: vec![
            LdtkFieldInstance {
                identifier: "id".into(),
                value: Value::String("alice_private_return_lock".into()),
                real_editor_values: vec![Value::Null],
            },
            LdtkFieldInstance {
                identifier: "name".into(),
                value: Value::String("alice_private_return_lock".into()),
                real_editor_values: vec![Value::Null],
            },
        ],
    };
    let area_field = LdtkFieldInstance {
        identifier: "activeArea".into(),
        value: Value::String("alice_relay".into()),
        real_editor_values: vec![Value::Null],
    };
    LdtkProject {
        json_version: "1.5.3".into(),
        levels: vec![LdtkLevel {
            identifier: "alice_relay".into(),
            iid: "level-iid".into(),
            world_x: 0,
            world_y: 0,
            px_wid: 1024,
            px_hei: 768,
            field_instances: vec![area_field],
            layer_instances: vec![LdtkLayerInstance {
                identifier: "Ambition".into(),
                layer_type: "Entities".into(),
                c_wid: 64,
                c_hei: 48,
                grid_size: 16,
                entity_instances: vec![lock_wall],
                int_grid_csv: Vec::new(),
                grid_tiles: Vec::new(),
            }],
        }],
    }
}

/// Without the unlock flag, compute_intro_flag_gated_lock_walls
/// should return the LockWall's footprint.
#[test]
fn lock_wall_compute_returns_block_when_flag_clear() {
    let project = synthetic_alice_relay_project();
    let save = ambition_persistence::save_data::SandboxSaveData::default();
    let walls = compute_intro_flag_gated_lock_walls(&project, "alice_relay", &save);
    assert_eq!(walls.len(), 1, "expected one lock wall");
    let (id, min, size) = &walls[0];
    assert_eq!(id, "alice_private_return_lock");
    assert_eq!(*min, ambition_engine_core::Vec2::new(800.0, 624.0));
    assert_eq!(*size, ambition_engine_core::Vec2::new(96.0, 112.0));
}

/// Once the unlock flag flips, compute should drop the LockWall
/// from the returned set.
#[test]
fn lock_wall_compute_drops_block_when_flag_set() {
    let project = synthetic_alice_relay_project();
    let mut save = ambition_persistence::save_data::SandboxSaveData::default();
    save.set_flag("bob_field_survey_received", true);
    let walls = compute_intro_flag_gated_lock_walls(&project, "alice_relay", &save);
    assert!(walls.is_empty(), "expected no lock walls after unlock");
}

/// A non-active room's lock walls should not appear in the
/// active-room block list — the system only operates on the
/// current room.
#[test]
fn lock_wall_compute_skips_other_rooms() {
    let project = synthetic_alice_relay_project();
    let save = ambition_persistence::save_data::SandboxSaveData::default();
    let walls = compute_intro_flag_gated_lock_walls(&project, "drain_alley", &save);
    assert!(walls.is_empty(), "expected no lock walls for inactive room");
}

/// A LockWall whose id is not in the registry table must be left
/// alone — the system only manages flag-gated locks, not every
/// LockWall in the project.
#[test]
fn lock_wall_compute_ignores_unregistered_ids() {
    use ambition_actors::world::ldtk_world::LdtkFieldInstance;
    let mut project = synthetic_alice_relay_project();
    // Mutate the one entity's `id` field to something not in
    // INTRO_FLAG_GATED_LOCK_WALLS.
    if let Some(entity) = project.levels[0].layer_instances[0]
        .entity_instances
        .first_mut()
    {
        entity.field_instances = vec![LdtkFieldInstance {
            identifier: "id".into(),
            value: serde_json::Value::String("encounter_owned_lock".into()),
            real_editor_values: vec![serde_json::Value::Null],
        }];
    }
    let save = ambition_persistence::save_data::SandboxSaveData::default();
    let walls = compute_intro_flag_gated_lock_walls(&project, "alice_relay", &save);
    assert!(
        walls.is_empty(),
        "registered-id-only filter should exclude this"
    );
}

// The Yarn migration retired `redirect_post_quest_dialog`:
// boss-cleared / flag-set redirects are now inline `<<if>>`
// branches inside the `.yarn` files. The runtime is exercised
// by running the actual dialog; the tests above used to pin
// the per-frame redirect dispatch, which no longer exists.

#[test]
fn flag_gated_lock_walls_have_unique_ids() {
    let mut ids = std::collections::BTreeSet::new();
    for (lock_id, _flag) in INTRO_FLAG_GATED_LOCK_WALLS.iter().copied() {
        assert!(
            ids.insert(lock_id),
            "duplicate LockWall id in INTRO_FLAG_GATED_LOCK_WALLS: {lock_id}"
        );
    }
}

/// Setting `bob_field_survey_received` should cause the
/// emit_intro_flag_chains system to write
/// `map_private_marks_unlocked` to save via the bus.
#[test]
fn emit_chains_promotes_bob_survey_to_private_marks() {
    use crate::quest::QuestRegistry;
    use ambition_actors::features::apply_flag_effects;
    use ambition_combat::SetFlagRequested;
    use ambition_persistence::save::SandboxSave;
    use bevy::app::{App, Update};

    let mut app = App::new();
    app.insert_resource(SandboxSave::default());
    app.insert_resource(QuestRegistry::default());
    app.add_message::<SetFlagRequested>();
    app.add_systems(
        Update,
        (super::emit_intro_flag_chains, apply_flag_effects).chain(),
    );

    // Pre-condition: trigger flag set, target flag clear.
    app.world_mut()
        .resource_mut::<SandboxSave>()
        .data_mut()
        .set_flag("bob_field_survey_received", true);

    // First tick: emit_intro_flag_chains writes a SetFlag effect
    // for `map_private_marks_unlocked`; apply_flag_effects reads
    // it the same frame because of `.chain()` ordering.
    app.update();

    let save = app.world().resource::<SandboxSave>();
    assert!(
        save.data().flag("map_private_marks_unlocked"),
        "chained flag should be set after one update"
    );
    // Idempotency: a second tick must not emit a redundant SetFlag.
    app.update();
    let save = app.world().resource::<SandboxSave>();
    assert!(save.data().flag("map_private_marks_unlocked"));
}

/// End-to-end progression check: walking the cartography quest
/// through alice → bob → P5 by setting flags one at a time
/// should advance `intro_cartography_route` through its three
/// steps.
#[test]
fn cartography_quest_advances_through_alice_bob_p5() {
    use crate::quest::{apply_quest_advance_events, default_quest_specs, QuestRegistry};
    use ambition_actors::features::{apply_flag_effects, apply_quest_effects};
    use ambition_actors::features::{QuestAdvanceRequested, SetFlagRequested};
    use ambition_persistence::save::SandboxSave;
    use bevy::app::{App, Update};

    let mut app = App::new();
    app.insert_resource(SandboxSave::default());
    let mut registry = QuestRegistry::default();
    for spec in default_quest_specs() {
        registry.ensure(spec);
    }
    if let Some(q) = registry.quests.get_mut("intro_cartography_route") {
        let _ = q.start();
    }
    app.insert_resource(registry);
    app.add_message::<SetFlagRequested>();
    app.add_message::<QuestAdvanceRequested>();
    // Order matters: chain emits SetFlag effects, then
    // apply_flag_effects writes them to save + pushes
    // QuestAdvanceEvent::FlagSet into the registry, then
    // apply_quest_advance_events drains those events and
    // advances quest state.
    app.add_systems(
        Update,
        (
            super::emit_intro_flag_chains,
            apply_flag_effects,
            apply_quest_effects,
            apply_quest_advance_events,
        )
            .chain(),
    );

    let step = |app: &App| {
        app.world()
            .resource::<QuestRegistry>()
            .quests
            .get("intro_cartography_route")
            .map(|q| q.step)
            .unwrap_or(0)
    };

    assert_eq!(step(&app), 0, "quest starts at step 0");

    // Step 1: alice's note. Set the source flag directly so the
    // chain promotion landed in save + bus same-frame; the quest
    // step condition watches FlagSet("alice_route_note_carried").
    app.world_mut()
        .resource_mut::<SandboxSave>()
        .data_mut()
        .set_flag("alice_route_note_carried", true);
    app.world_mut().resource_mut::<QuestRegistry>().push_event(
        ambition_persistence::quest::QuestAdvanceEvent::FlagSet("alice_route_note_carried".into()),
    );
    app.update();
    assert_eq!(
        step(&app),
        1,
        "after alice carry, quest should be at step 1"
    );

    // Step 2: bob's field survey.
    app.world_mut()
        .resource_mut::<SandboxSave>()
        .data_mut()
        .set_flag("bob_field_survey_received", true);
    app.world_mut().resource_mut::<QuestRegistry>().push_event(
        ambition_persistence::quest::QuestAdvanceEvent::FlagSet("bob_field_survey_received".into()),
    );
    app.update();
    assert_eq!(step(&app), 2, "after bob survey, quest should be at step 2");
    let save = app.world().resource::<SandboxSave>();
    assert!(save.data().flag("map_private_marks_unlocked"));

    // Step 3: P5 route memory.
    app.world_mut()
        .resource_mut::<SandboxSave>()
        .data_mut()
        .set_flag("intro_p5_route_memory_received", true);
    app.world_mut().resource_mut::<QuestRegistry>().push_event(
        ambition_persistence::quest::QuestAdvanceEvent::FlagSet(
            "intro_p5_route_memory_received".into(),
        ),
    );
    app.update();
    let registry = app.world().resource::<QuestRegistry>();
    let q = registry.quests.get("intro_cartography_route").unwrap();
    assert!(q.is_complete(), "after P5 pickup, quest should be complete");
    let save = app.world().resource::<SandboxSave>();
    assert!(save.data().flag("route_memory_received"));
}

/// Setting `intro_p5_route_memory_received` should chain to
/// `route_memory_received` and quest steps watching the target
/// flag should see the FlagSet event through apply_flag_effects.
#[test]
fn emit_chains_promotes_p5_to_route_memory() {
    use crate::quest::QuestRegistry;
    use ambition_actors::features::apply_flag_effects;
    use ambition_combat::SetFlagRequested;
    use ambition_persistence::save::SandboxSave;
    use bevy::app::{App, Update};

    let mut app = App::new();
    app.insert_resource(SandboxSave::default());
    app.insert_resource(QuestRegistry::default());
    app.add_message::<SetFlagRequested>();
    app.add_systems(
        Update,
        (super::emit_intro_flag_chains, apply_flag_effects).chain(),
    );

    app.world_mut()
        .resource_mut::<SandboxSave>()
        .data_mut()
        .set_flag("intro_p5_route_memory_received", true);
    app.update();

    let save = app.world().resource::<SandboxSave>();
    assert!(save.data().flag("route_memory_received"));
}
