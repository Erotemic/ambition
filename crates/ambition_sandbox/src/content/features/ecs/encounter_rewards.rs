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
    save: &mut ae::SandboxSaveData,
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
    save: &ae::SandboxSaveData,
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
            ChestFeature::new(ae::Chest::new(
                chest_id,
                Some(ae::PickupKind::Health { amount: 2 }),
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
    save: &ae::SandboxSaveData,
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
            ae::PersistedEncounterState::Cleared
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
            ChestFeature::new(ae::Chest::new(chest_id, Some(pickup.clone()))),
            BossRewardChest::new(encounter_id.clone()),
        ));
        if looted {
            entity.insert(Opened);
        } else {
            entity.insert(FallingChest::new(0.0));
        }
    }
}
