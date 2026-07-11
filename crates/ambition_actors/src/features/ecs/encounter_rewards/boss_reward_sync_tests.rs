//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod boss_reward_sync_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

//! sync_boss_reward_chests_ecs drops a boss's reward chest once the
//! boss reads Cleared in the save and a spawn anchor is known. The
//! non-ECS world/anchors params are carried in test-only resources so
//! a normal wrapper system can drive the helper.
use super::*;
use crate::boss_encounter::{BossEncounterRegistry, BossProfile};
use ambition_persistence::save::SandboxSave;
use ambition_persistence::save_data::PersistedEncounterState;
use bevy::prelude::{App, Resource, Update};

#[derive(Resource)]
struct TestWorld(ae::World);
#[derive(Resource)]
struct TestAnchors(Vec<(String, String, ae::Vec2)>);

fn run_boss_sync(
    mut commands: Commands,
    save: Res<SandboxSave>,
    registry: Res<BossEncounterRegistry>,
    world: Res<TestWorld>,
    anchors: Res<TestAnchors>,
    chests: Query<
        (
            Entity,
            &BossRewardChest,
            &FeatureId,
            Option<&Opened>,
            Option<&FallingChest>,
        ),
        With<ChestFeature>,
    >,
) {
    sync_boss_reward_chests_ecs(
        &mut commands,
        save.data(),
        &registry,
        &world.0,
        &anchors.0,
        &chests,
    );
}

fn app() -> App {
    let mut app = App::new();
    let mut save = SandboxSave::default();
    save.data_mut()
        .set_boss("test_boss", PersistedEncounterState::Cleared);
    app.insert_resource(save);
    let mut reg = BossEncounterRegistry::default();
    reg.profiles.insert(
        "test_boss".into(),
        BossProfile::from_id("mockingbird").expect("mockingbird is authored"),
    );
    app.insert_resource(reg);
    app.insert_resource(TestWorld(ae::World::new(
        "t",
        ae::Vec2::new(400.0, 400.0),
        ae::Vec2::new(50.0, 50.0),
        Vec::new(),
    )));
    // (placement_id, archetype_id, spawn) — placement == archetype for this
    // single-placement fixture.
    app.insert_resource(TestAnchors(vec![(
        "test_boss".into(),
        "test_boss".into(),
        ae::Vec2::new(200.0, 100.0),
    )]));
    app.add_systems(Update, run_boss_sync);
    app
}

#[test]
fn cleared_boss_drops_its_reward_chest() {
    let mut app = app();
    app.update();
    let mut q = app.world_mut().query::<&BossRewardChest>();
    let ids: Vec<String> = q
        .iter(app.world())
        .map(|r| r.encounter_id.clone())
        .collect();
    assert_eq!(
        ids,
        vec!["test_boss".to_string()],
        "a cleared boss drops one reward chest"
    );
}

#[test]
fn boss_reward_sync_is_idempotent() {
    let mut app = app();
    app.update();
    app.update();
    let mut q = app.world_mut().query::<&BossRewardChest>();
    assert_eq!(
        q.iter(app.world()).count(),
        1,
        "no duplicate boss chest on re-tick"
    );
}
