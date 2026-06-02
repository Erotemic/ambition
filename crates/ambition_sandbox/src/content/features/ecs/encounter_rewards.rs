//! Reward-chest sync for cleared mob and boss encounters.
//!
//! When an encounter clears, the matching reward chest entity should
//! exist in the room; when the save says the chest has been looted,
//! it should carry the `Opened` marker; when it has not been looted
//! yet, the chest should not carry that marker. These helpers are
//! the single point that mirrors that contract from the encounter
//! and boss-encounter registries to the ECS.

use super::falling_chest::settled_chest_center;
use super::*;
use bevy::prelude::Name;

/// Drop the encounter's ECS reward chest, if any, and clear its looted flag.
pub fn clear_encounter_reward_ecs(
    commands: &mut Commands,
    save: &mut crate::save::SandboxSaveData,
    chests: &Query<
        (Entity, &EncounterRewardChest, &FeatureId, Option<&Opened>),
        With<ChestFeature>,
    >,
    encounter_id: &str,
) {
    for (entity, reward, _, _) in chests.iter() {
        if reward.encounter_id == encounter_id {
            commands.entity(entity).despawn();
        }
    }
    save.set_flag(
        crate::encounter::encounter_reward_looted_flag(encounter_id),
        false,
    );
}

/// Idempotently ensure cleared mob encounters have an ECS reward chest.
pub fn sync_encounter_reward_chests_ecs(
    commands: &mut Commands,
    save: &crate::save::SandboxSaveData,
    registry: &crate::encounter::EncounterRegistry,
    chests: &Query<
        (Entity, &EncounterRewardChest, &FeatureId, Option<&Opened>),
        With<ChestFeature>,
    >,
) {
    let chest_size = ae::Vec2::new(28.0, 28.0);
    for (encounter_id, state) in registry.encounters.iter() {
        if !matches!(state.phase, crate::encounter::EncounterPhase::Cleared) {
            continue;
        }
        let Some(spec) = state.spec.as_ref() else {
            continue;
        };
        let chest_id = format!("encounter_chest_{encounter_id}");
        let looted = save.flag(&crate::encounter::encounter_reward_looted_flag(
            encounter_id,
        ));
        let existing = chests
            .iter()
            .find(|(_, reward, _, _)| reward.encounter_id == *encounter_id);
        if let Some((entity, _, _, opened)) = existing {
            match (looted, opened.is_some()) {
                (true, false) => {
                    commands.entity(entity).insert(Opened);
                }
                (false, true) => {
                    commands.entity(entity).remove::<Opened>();
                }
                _ => {}
            }
            continue;
        }
        let chest_pos = crate::encounter::encounter_reward_chest_pos(spec, chest_size);
        let mut entity = commands.spawn((
            Name::new(format!("Encounter reward chest: {encounter_id}")),
            FeatureSimEntity,
            RoomVisual,
            FeatureId::new(chest_id.clone()),
            FeatureName::new(chest_id.clone()),
            FeatureAabb::from_center_size(chest_pos, chest_size),
            ChestFeature::new(crate::interaction::Chest::new(
                chest_id,
                Some(spec.reward.clone()),
            )),
            EncounterRewardChest::new(encounter_id.clone()),
        ));
        if looted {
            entity.insert(Opened);
        }
    }
}

/// Idempotently ensure cleared boss encounters have ECS reward chests.
/// Boss actors are ECS entities now; this helper receives their spawn anchors
/// from the boss encounter system and owns the reward chest entity/state natively.
pub fn sync_boss_reward_chests_ecs(
    commands: &mut Commands,
    save: &crate::save::SandboxSaveData,
    registry: &crate::boss_encounter::BossEncounterRegistry,
    world: &ae::World,
    boss_anchors: &[(String, ae::Vec2)],
    chests: &Query<
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
    for (encounter_id, profile) in &registry.profiles {
        let crate::boss_encounter::BossRewardProfile::DropChest {
            pickup,
            offset,
            size,
        } = &profile.reward
        else {
            continue;
        };
        if !matches!(
            save.boss(encounter_id),
            crate::save::PersistedEncounterState::Cleared
        ) {
            continue;
        }
        let runtime_id = registry
            .runtime_ids
            .get(encounter_id)
            .cloned()
            .unwrap_or_else(|| encounter_id.clone());
        let Some((_, boss_spawn)) = boss_anchors.iter().find(|(id, _)| id == &runtime_id) else {
            continue;
        };
        let chest_id = format!("encounter_chest_{encounter_id}");
        let looted = save.flag(&crate::encounter::encounter_reward_looted_flag(
            encounter_id,
        ));
        let existing = chests
            .iter()
            .find(|(_, reward, _, _, _)| reward.encounter_id == *encounter_id);
        if let Some((entity, _, _, opened, falling)) = existing {
            match (looted, opened.is_some()) {
                (true, false) => {
                    commands.entity(entity).insert(Opened);
                }
                (false, true) => {
                    commands.entity(entity).remove::<Opened>();
                }
                _ => {}
            }
            if looted && falling.is_some() {
                commands.entity(entity).remove::<FallingChest>();
            }
            continue;
        }
        let mut chest_pos = *boss_spawn + *offset;
        if looted {
            chest_pos = settled_chest_center(world, chest_pos, *size);
        }
        let mut entity = commands.spawn((
            Name::new(format!("Boss reward chest: {encounter_id}")),
            FeatureSimEntity,
            RoomVisual,
            FeatureId::new(chest_id.clone()),
            FeatureName::new(chest_id.clone()),
            FeatureAabb::from_center_size(chest_pos, *size),
            ChestFeature::new(crate::interaction::Chest::new(
                chest_id,
                Some(pickup.clone()),
            )),
            BossRewardChest::new(encounter_id.clone()),
        ));
        if looted {
            entity.insert(Opened);
        } else {
            entity.insert(FallingChest::new(0.0));
        }
    }
}

#[cfg(test)]
mod reward_sync_tests {
    //! sync_encounter_reward_chests_ecs drops one reward chest per Cleared
    //! encounter and is idempotent (no duplicate on re-tick). Wrapped in a
    //! thin system so the minimal App can drive the &Commands/&save/&registry
    //! /&Query helper.
    use super::*;
    use crate::encounter::{EncounterPhase, EncounterRegistry, EncounterSpec, EncounterState};
    use crate::interaction::PickupKind;
    use crate::persistence::save::SandboxSave;
    use bevy::prelude::{App, Update};

    fn cleared_registry() -> EncounterRegistry {
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
        let state = EncounterState {
            spec: Some(spec),
            phase: EncounterPhase::Cleared,
            ..Default::default()
        };
        let mut reg = EncounterRegistry::default();
        reg.encounters.insert("test_enc".into(), state);
        reg
    }

    fn run_sync(
        mut commands: Commands,
        save: Res<SandboxSave>,
        registry: Res<EncounterRegistry>,
        chests: Query<
            (Entity, &EncounterRewardChest, &FeatureId, Option<&Opened>),
            With<ChestFeature>,
        >,
    ) {
        sync_encounter_reward_chests_ecs(&mut commands, save.data(), &registry, &chests);
    }

    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(SandboxSave::default());
        app.insert_resource(cleared_registry());
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
        assert_eq!(ids, vec!["test_enc".to_string()], "one reward chest for the cleared encounter");
    }

    #[test]
    fn reward_sync_is_idempotent() {
        let mut app = app();
        app.update();
        app.update(); // second tick must not spawn a duplicate chest
        let mut q = app.world_mut().query::<&EncounterRewardChest>();
        assert_eq!(q.iter(app.world()).count(), 1, "no duplicate chest on re-tick");
    }
}

#[cfg(test)]
mod boss_reward_sync_tests {
    //! sync_boss_reward_chests_ecs drops a boss's reward chest once the
    //! boss reads Cleared in the save and a spawn anchor is known. The
    //! non-ECS world/anchors params are carried in test-only resources so
    //! a normal wrapper system can drive the helper.
    use super::*;
    use crate::boss_encounter::{BossEncounterRegistry, BossProfile};
    use crate::persistence::save::SandboxSave;
    use crate::save::PersistedEncounterState;
    use bevy::prelude::{App, Resource, Update};

    #[derive(Resource)]
    struct TestWorld(ae::World);
    #[derive(Resource)]
    struct TestAnchors(Vec<(String, ae::Vec2)>);

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
        save.data_mut().set_boss("test_boss", PersistedEncounterState::Cleared);
        app.insert_resource(save);
        let mut reg = BossEncounterRegistry::default();
        reg.profiles.insert("test_boss".into(), BossProfile::mockingbird());
        app.insert_resource(reg);
        app.insert_resource(TestWorld(ae::World::new(
            "t",
            ae::Vec2::new(400.0, 400.0),
            ae::Vec2::new(50.0, 50.0),
            Vec::new(),
        )));
        app.insert_resource(TestAnchors(vec![(
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
        let ids: Vec<String> = q.iter(app.world()).map(|r| r.encounter_id.clone()).collect();
        assert_eq!(ids, vec!["test_boss".to_string()], "a cleared boss drops one reward chest");
    }

    #[test]
    fn boss_reward_sync_is_idempotent() {
        let mut app = app();
        app.update();
        app.update();
        let mut q = app.world_mut().query::<&BossRewardChest>();
        assert_eq!(q.iter(app.world()).count(), 1, "no duplicate boss chest on re-tick");
    }
}
