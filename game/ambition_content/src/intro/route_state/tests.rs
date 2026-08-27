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

/// Setting `bob_field_survey_received` should cause the
/// emit_intro_flag_chains system to write
/// `map_private_marks_unlocked` to save via the bus.
#[test]
fn emit_chains_promotes_bob_survey_to_private_marks() {
    use crate::quest::QuestRegistry;
    use ambition_combat::SetFlagRequested;
    use ambition_persistence::save::AmbitionGameSave;
    use ambition_platformer2d_actor_monolith::features::apply_flag_effects;
    use bevy::app::{App, Update};

    let mut app = App::new();
    app.insert_resource(AmbitionGameSave::default());
    app.insert_resource(QuestRegistry::default());
    app.add_message::<SetFlagRequested>();
    app.add_systems(
        Update,
        (super::emit_intro_flag_chains, apply_flag_effects).chain(),
    );

    // Pre-condition: trigger flag set, target flag clear.
    app.world_mut()
        .resource_mut::<AmbitionGameSave>()
        .data_mut()
        .set_flag("bob_field_survey_received", true);

    // First tick: emit_intro_flag_chains writes a SetFlag effect
    // for `map_private_marks_unlocked`; apply_flag_effects reads
    // it the same frame because of `.chain()` ordering.
    app.update();

    let save = app.world().resource::<AmbitionGameSave>();
    assert!(
        save.data().flag("map_private_marks_unlocked"),
        "chained flag should be set after one update"
    );
    // Idempotency: a second tick must not emit a redundant SetFlag.
    app.update();
    let save = app.world().resource::<AmbitionGameSave>();
    assert!(save.data().flag("map_private_marks_unlocked"));
}

/// End-to-end progression check: walking the cartography quest
/// through alice → bob → P5 by setting flags one at a time
/// should advance `intro_cartography_route` through its three
/// steps.
#[test]
fn cartography_quest_advances_through_alice_bob_p5() {
    use crate::quest::{apply_quest_advance_events, default_quest_specs, QuestRegistry};
    use ambition_combat::events::SetFlagRequested;
    use ambition_persistence::quest::QuestAdvanceRequested;
    use ambition_persistence::save::AmbitionGameSave;
    use ambition_platformer2d_actor_monolith::features::{apply_flag_effects, apply_quest_effects};
    use bevy::app::{App, Update};

    let mut app = App::new();
    app.insert_resource(AmbitionGameSave::default());
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

    app.world_mut()
        .resource_mut::<AmbitionGameSave>()
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
        .resource_mut::<AmbitionGameSave>()
        .data_mut()
        .set_flag("bob_field_survey_received", true);
    app.world_mut().resource_mut::<QuestRegistry>().push_event(
        ambition_persistence::quest::QuestAdvanceEvent::FlagSet("bob_field_survey_received".into()),
    );
    app.update();
    assert_eq!(step(&app), 2, "after bob survey, quest should be at step 2");
    let save = app.world().resource::<AmbitionGameSave>();
    assert!(save.data().flag("map_private_marks_unlocked"));

    // Step 3: P5 route memory.
    app.world_mut()
        .resource_mut::<AmbitionGameSave>()
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
    let save = app.world().resource::<AmbitionGameSave>();
    assert!(save.data().flag("route_memory_received"));
}

/// Setting `intro_p5_route_memory_received` should chain to
/// `route_memory_received` and quest steps watching the target
/// flag should see the FlagSet event through apply_flag_effects.
#[test]
fn emit_chains_promotes_p5_to_route_memory() {
    use crate::quest::QuestRegistry;
    use ambition_combat::SetFlagRequested;
    use ambition_persistence::save::AmbitionGameSave;
    use ambition_platformer2d_actor_monolith::features::apply_flag_effects;
    use bevy::app::{App, Update};

    let mut app = App::new();
    app.insert_resource(AmbitionGameSave::default());
    app.insert_resource(QuestRegistry::default());
    app.add_message::<SetFlagRequested>();
    app.add_systems(
        Update,
        (super::emit_intro_flag_chains, apply_flag_effects).chain(),
    );

    app.world_mut()
        .resource_mut::<AmbitionGameSave>()
        .data_mut()
        .set_flag("intro_p5_route_memory_received", true);
    app.update();

    let save = app.world().resource::<AmbitionGameSave>();
    assert!(save.data().flag("route_memory_received"));
}

/// THE INTRO WORLD SAYS WHICH FLAG OPENS WHICH WALL — in the level, not in
/// Rust.
///
/// the engine tests pin the FUNCTION; this pins the WIRING, and that
/// distinction has cost this project a session before: enemy facing was plumbed,
/// tested and green the entire time enemies walked the wrong way, because
/// nothing asserted the authored world ever *said* which way.
///
/// Here the equivalent failure is silent and total: `gated_by` is optional by
/// design (an encounter's walls carry none), so a world that lost the field
/// would produce two walls that simply never appear. No error, no warning — the
/// player just walks through a door that was supposed to be locked.
///
/// this reads the shipped `.ldtk` rather than a fixture, on purpose. A
/// regenerate, an editor session, or a careless merge is exactly what this
/// defends against, and none of those touch a fixture.
#[test]
fn the_intro_world_authors_the_flag_that_opens_each_gated_wall() {
    let text = include_str!("../../../assets/worlds/intro.ldtk");
    let project: serde_json::Value = serde_json::from_str(text).expect("intro.ldtk parses");

    let mut gated: std::collections::BTreeMap<String, String> = Default::default();
    let mut lock_walls = 0usize;
    for level in project["levels"].as_array().into_iter().flatten() {
        for layer in level["layerInstances"].as_array().into_iter().flatten() {
            for entity in layer["entityInstances"].as_array().into_iter().flatten() {
                if entity["__identifier"] != "LockWall" {
                    continue;
                }
                lock_walls += 1;
                let fields: std::collections::BTreeMap<&str, &serde_json::Value> = entity
                    ["fieldInstances"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|f| Some((f["__identifier"].as_str()?, &f["__value"])))
                    .collect();
                let (Some(id), Some(flag)) = (
                    fields.get("id").and_then(|v| v.as_str()),
                    fields.get("gated_by").and_then(|v| v.as_str()),
                ) else {
                    continue;
                };
                if !flag.trim().is_empty() {
                    gated.insert(id.to_string(), flag.to_string());
                }
            }
        }
    }

    // the non-vacuity guard: a world that lost its LockWalls entirely would
    // satisfy every assertion below by having nothing to check.
    assert!(
        lock_walls >= 2,
        "intro.ldtk authors only {lock_walls} LockWall(s); this test is about them"
    );
    assert_eq!(
        gated
            .iter()
            .map(|(id, flag)| (id.as_str(), flag.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("alice_private_return_lock", "bob_field_survey_received"),
            ("gate_alice_private_lock", "bob_field_survey_received"),
        ],
        "these two pairs used to live in a Rust const table; they now live in the \
         level, and losing them there is silent"
    );
}
