//! Intro-v1 route-state chained flags.
//!
//! When the player picks up certain narrative pickups (Bob's field survey,
//! the system boss's P5 reward, etc.) the slice wants secondary flags to
//! flip too — `map_private_marks_unlocked`, `route_memory_received`, and
//! similar map-layer hooks that downstream listeners can subscribe to
//! without watching for the specific source flag.
//!
//! Implemented as a tiny system that runs after [`apply_flag_effects`] each
//! frame: it reads the save layer, walks the static [`INTRO_FLAG_CHAINS`]
//! table, and emits a fresh `GameplayEffect::SetFlag` for any chained flag
//! whose trigger is set but whose target is still missing. The chained
//! emission then flows through `apply_flag_effects` next frame, which
//! writes it to save and pushes a `QuestAdvanceEvent::FlagSet` so quest
//! steps that listen on the chained flag advance automatically.
//!
//! Keeping the chain as a const data table (not a switch arm in
//! `apply_flag_effects`) means new intro chains are one-line edits and the
//! bus stays generic.
//!
//! The system is idempotent: the second time it observes a trigger that
//! has already set its target it sees the target flag present and skips.

use bevy::prelude::*;

use ambition_gameplay_core::features::SetFlagRequested;

/// `(trigger_flag, target_flag)` — when the trigger lands in the save
/// layer, the system emits a SetFlag for the target. Targets are listed
/// in playtest-handoff.md §"What remains placeholder" so the next agent
/// can grep both ends in one read.
pub const INTRO_FLAG_CHAINS: &[(&str, &str)] = &[
    // Bob's field survey reveals private map marks the player can read
    // back. Wired here so Task 04's narrative beat surfaces a concrete
    // downstream flag without the cartography quest having to carry the
    // entire reveal payload.
    ("bob_field_survey_received", "map_private_marks_unlocked"),
    // The P5 reward (collected in first_system_boss) imprints route
    // memory: the world remembers which routes the player cleared,
    // which Task 09+ visualizations / dialogue branches can consume.
    ("intro_p5_route_memory_received", "route_memory_received"),
    // Picking up Alice's sealed route note also turns on basic map
    // awareness so a future minimap layer has a flag to gate on.
    ("alice_route_note_carried", "map_basic_unlocked"),
    // Evil/lawful report route (Script C in playtest-handoff.md).
    // Activating the `gate_official_report` Switch in
    // gate_stack_lower sets `switch_gate_official_report_used` (the
    // standard interact-system pattern). This chain promotes that to
    // the canonical `alice_route_note_reported` and then to
    // `private_routes_compromised` so a single Switch toggle
    // produces a coherent save-state record of the report path.
    (
        "switch_gate_official_report_used",
        "alice_route_note_reported",
    ),
    ("alice_route_note_reported", "private_routes_compromised"),
];

/// Watches the save layer for any chained trigger and emits the target
/// flag through the standard `GameplayEffect::SetFlag` bus. Runs every
/// frame; cost is O(chains × set-flag-lookups) and the chain table is
/// expected to stay under a few dozen entries.
pub fn emit_intro_flag_chains(
    save: Res<ambition_gameplay_core::persistence::save::SandboxSave>,
    mut effects: MessageWriter<SetFlagRequested>,
) {
    let data = save.data();
    for (trigger, target) in INTRO_FLAG_CHAINS.iter().copied() {
        if data.flag(trigger) && !data.flag(target) {
            effects.write(SetFlagRequested {
                id: target.to_string(),
                on: true,
            });
        }
    }
}

/// LockWalls in the intro slice whose collision should be removed
/// once the named flag is set in save. Each entry is
/// `(lock_id_on_LockWall_entity, unlock_flag)`.
///
/// LockWalls without an associated EncounterTrigger are inert in the
/// stock runtime — the entity exists in LDtk but no system inserts a
/// blocking solid into the engine's `world.blocks`. The system below
/// reads from this table to provide that wiring for the cartography
/// route: while the unlock flag is clear, an `intro_lock:<id>` solid
/// block is inserted in the active room; once the flag flips, the
/// block is removed and the player can walk through.
pub const INTRO_FLAG_GATED_LOCK_WALLS: &[(&str, &str)] = &[
    ("alice_private_return_lock", "bob_field_survey_received"),
    ("gate_alice_private_lock", "bob_field_survey_received"),
];

// The intro-specific dialog redirect table moved into the unified
// data-driven registry at `assets/data/dialogue/registry.ron`
// alongside the sandbox redirects. The boss/flag gate predicates are
// the same shape; `dialog::redirect_post_quest_dialog` now walks both
// families in one pass. Adding a new intro flag swap is a one-row
// edit in the RON file.

/// Pure computational core of [`sync_intro_flag_gated_lock_walls`].
/// Given the LDtk project, the active room id, and a save snapshot,
/// returns the (lock_id, min, size) triples that should be present
/// as `intro_lock:<id>` blocks this frame. Extracted so the Bevy
/// system can be tested without spinning up a full ECS world.
pub fn compute_intro_flag_gated_lock_walls(
    project: &ambition_gameplay_core::world::ldtk_world::LdtkProject,
    active_room_id: &str,
    save: &ambition_gameplay_core::persistence::save_data::SandboxSaveData,
) -> Vec<(
    String,
    ambition_engine_core::Vec2,
    ambition_engine_core::Vec2,
)> {
    let mut out: Vec<(
        String,
        ambition_engine_core::Vec2,
        ambition_engine_core::Vec2,
    )> = Vec::new();
    for level in &project.levels {
        if level.active_area() != active_room_id {
            continue;
        }
        for entity in level.all_entity_instances() {
            if entity.identifier != "LockWall" {
                continue;
            }
            let Some(id) = ambition_gameplay_core::world::ldtk_world::field_string(entity, "id")
            else {
                continue;
            };
            let id_trim = id.trim();
            let Some((_, flag)) = INTRO_FLAG_GATED_LOCK_WALLS
                .iter()
                .find(|(lock, _)| *lock == id_trim)
            else {
                continue;
            };
            if save.flag(flag) {
                continue;
            }
            let min = ambition_engine_core::Vec2::new(
                entity.px[0] as f32,
                entity.px[1] as f32,
            );
            let size = ambition_engine_core::Vec2::new(
                entity.width as f32,
                entity.height as f32,
            );
            out.push((id_trim.to_string(), min, size));
        }
    }
    out
}

/// Per-frame sync of the intro flag-gated lock walls. Mirrors the
/// encounter system's `sync_lock_walls` but driven by the save layer
/// rather than encounter phase. Delegates the LDtk-walking logic to
/// [`compute_intro_flag_gated_lock_walls`] so the policy is testable
/// in isolation.
pub fn sync_intro_flag_gated_lock_walls(
    project: Option<Res<ambition_gameplay_core::world::ldtk_world::SandboxLdtkProject>>,
    room_set: Option<Res<ambition_gameplay_core::rooms::RoomSet>>,
    save: Option<Res<ambition_gameplay_core::persistence::save::SandboxSave>>,
    world: Option<ResMut<ambition_gameplay_core::GameWorld>>,
) {
    let (Some(project), Some(room_set), Some(save), Some(mut world)) =
        (project, room_set, save, world)
    else {
        return;
    };
    let active_room_id = room_set.active_spec().id.clone();
    let desired = compute_intro_flag_gated_lock_walls(&project.0, &active_room_id, save.data());
    let desired_ids: std::collections::HashSet<String> =
        desired.iter().map(|(id, _, _)| id.clone()).collect();

    world.0.blocks.retain(|b| {
        if let Some(stripped) = b.name.strip_prefix("intro_lock:") {
            desired_ids.contains(stripped)
        } else {
            true
        }
    });

    for (id, min, size) in desired {
        let name = format!("intro_lock:{id}");
        if !world.0.blocks.iter().any(|b| b.name == name) {
            world
                .0
                .blocks
                .push(ambition_engine_core::Block::solid(
                    name, min, size,
                ));
        }
    }
}

#[cfg(test)]
mod tests {
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
    fn synthetic_alice_relay_project() -> ambition_gameplay_core::world::ldtk_world::LdtkProject {
        use ambition_gameplay_core::world::ldtk_world::{
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
        let save = ambition_gameplay_core::persistence::save_data::SandboxSaveData::default();
        let walls = compute_intro_flag_gated_lock_walls(&project, "alice_relay", &save);
        assert_eq!(walls.len(), 1, "expected one lock wall");
        let (id, min, size) = &walls[0];
        assert_eq!(id, "alice_private_return_lock");
        assert_eq!(
            *min,
            ambition_engine_core::Vec2::new(800.0, 624.0)
        );
        assert_eq!(
            *size,
            ambition_engine_core::Vec2::new(96.0, 112.0)
        );
    }

    /// Once the unlock flag flips, compute should drop the LockWall
    /// from the returned set.
    #[test]
    fn lock_wall_compute_drops_block_when_flag_set() {
        let project = synthetic_alice_relay_project();
        let mut save = ambition_gameplay_core::persistence::save_data::SandboxSaveData::default();
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
        let save = ambition_gameplay_core::persistence::save_data::SandboxSaveData::default();
        let walls = compute_intro_flag_gated_lock_walls(&project, "drain_alley", &save);
        assert!(walls.is_empty(), "expected no lock walls for inactive room");
    }

    /// A LockWall whose id is not in the registry table must be left
    /// alone — the system only manages flag-gated locks, not every
    /// LockWall in the project.
    #[test]
    fn lock_wall_compute_ignores_unregistered_ids() {
        use ambition_gameplay_core::world::ldtk_world::LdtkFieldInstance;
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
        let save = ambition_gameplay_core::persistence::save_data::SandboxSaveData::default();
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
        use ambition_gameplay_core::features::apply_flag_effects;
        use ambition_gameplay_core::features::SetFlagRequested;
        use ambition_gameplay_core::persistence::save::SandboxSave;
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
        use ambition_gameplay_core::features::{apply_flag_effects, apply_quest_effects};
        use ambition_gameplay_core::features::{QuestAdvanceRequested, SetFlagRequested};
        use ambition_gameplay_core::persistence::save::SandboxSave;
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
            ambition_gameplay_core::quest::QuestAdvanceEvent::FlagSet(
                "alice_route_note_carried".into(),
            ),
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
            ambition_gameplay_core::quest::QuestAdvanceEvent::FlagSet(
                "bob_field_survey_received".into(),
            ),
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
            ambition_gameplay_core::quest::QuestAdvanceEvent::FlagSet(
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
        use ambition_gameplay_core::features::apply_flag_effects;
        use ambition_gameplay_core::features::SetFlagRequested;
        use ambition_gameplay_core::persistence::save::SandboxSave;
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
}
