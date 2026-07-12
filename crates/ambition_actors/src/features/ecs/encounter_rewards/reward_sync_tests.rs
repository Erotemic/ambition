//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod reward_sync_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

//! sync_encounter_reward_chests_ecs drops one reward chest per Cleared
//! encounter and is idempotent (no duplicate on re-tick). Wrapped in a
//! thin system so the minimal App can drive the &Commands/&save/&registry
//! /&Query helper.
use super::*;
use crate::encounter::EncounterSpec;
use ambition_interaction::PickupKind;
use ambition_persistence::save::SandboxSave;
use bevy::prelude::{App, Resource, Update};

/// The cleared encounters' `(id, spec)` pairs the reward sync consumes now that
/// it takes the cleared list directly (E1) instead of the registry.
#[derive(Resource)]
struct ClearedEncounters(Vec<(String, EncounterSpec)>);

fn cleared_encounters() -> ClearedEncounters {
    let spec = EncounterSpec {
        id: "test_enc".into(),
        waves: Vec::new(),
        trigger_min: [100.0, 100.0],
        trigger_size: [200.0, 80.0],
        camera_zoom: 1.0,
        lock_wall: None,
        intro_seconds: 0.0,
        music_track: String::new(),
        reward: PickupKind::Health { amount: 2 },
    };
    ClearedEncounters(vec![("test_enc".into(), spec)])
}

fn run_sync(
    mut commands: Commands,
    save: Res<SandboxSave>,
    cleared: Res<ClearedEncounters>,
    chests: Query<(Entity, &EncounterRewardChest, &FeatureId, Option<&Opened>), With<ChestFeature>>,
) {
    sync_encounter_reward_chests_ecs(
        &mut commands,
        SessionSpawnScope::UNSCOPED,
        save.data(),
        &cleared.0,
        &chests,
    );
}

fn app() -> App {
    let mut app = App::new();
    app.insert_resource(SandboxSave::default());
    app.insert_resource(cleared_encounters());
    app.add_systems(Update, run_sync);
    app
}

#[test]
fn cleared_encounter_spawns_its_reward_chest() {
    let mut app = app();
    app.update();
    let mut q = app.world_mut().query::<&EncounterRewardChest>();
    let ids: Vec<String> = q
        .iter(app.world())
        .map(|r| r.encounter_id.clone())
        .collect();
    assert_eq!(
        ids,
        vec!["test_enc".to_string()],
        "one reward chest for the cleared encounter"
    );
}

#[test]
fn reward_sync_is_idempotent() {
    let mut app = app();
    app.update();
    app.update(); // second tick must not spawn a duplicate chest
    let mut q = app.world_mut().query::<&EncounterRewardChest>();
    assert_eq!(
        q.iter(app.world()).count(),
        1,
        "no duplicate chest on re-tick"
    );
}
