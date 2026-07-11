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
    save: &mut ambition_persistence::save_data::SandboxSaveData,
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
///
/// Takes the cleared encounters' `(id, spec)` pairs (gathered from the live
/// encounter entities by the caller) rather than the registry, so it stays
/// decoupled from the encounter state representation (E1).
pub fn sync_encounter_reward_chests_ecs(
    commands: &mut Commands,
    save: &ambition_persistence::save_data::SandboxSaveData,
    cleared: &[(String, crate::encounter::EncounterSpec)],
    chests: &Query<
        (Entity, &EncounterRewardChest, &FeatureId, Option<&Opened>),
        With<ChestFeature>,
    >,
) {
    let chest_size = ae::Vec2::new(28.0, 28.0);
    for (encounter_id, spec) in cleared.iter() {
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
            CenteredAabb::from_center_size(chest_pos, chest_size),
            ChestFeature::new(ambition_interaction::Chest::new(
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
    save: &ambition_persistence::save_data::SandboxSaveData,
    registry: &crate::boss_encounter::BossEncounterRegistry,
    world: &ae::World,
    // (placement_id, archetype_id, spawn) for each boss in the room. R4 keys the
    // chest + looted flag by PLACEMENT (so a cleared placement drops its own
    // chest) and resolves the DropChest reward via the archetype profile.
    boss_placements: &[(String, String, ae::Vec2)],
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
    for (placement_id, archetype_id, boss_spawn) in boss_placements {
        let Some(profile) = registry.profiles.get(archetype_id) else {
            continue;
        };
        let crate::boss_encounter::BossRewardProfile::DropChest {
            pickup,
            offset,
            size,
        } = &profile.reward
        else {
            continue;
        };
        if !matches!(
            save.boss(placement_id),
            ambition_persistence::save_data::PersistedEncounterState::Cleared
        ) {
            continue;
        }
        let chest_id = format!("encounter_chest_{placement_id}");
        let looted = save.flag(&crate::encounter::encounter_reward_looted_flag(
            placement_id,
        ));
        let existing = chests
            .iter()
            .find(|(_, reward, _, _, _)| reward.encounter_id == *placement_id);
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
            Name::new(format!("Boss reward chest: {placement_id}")),
            FeatureSimEntity,
            RoomVisual,
            FeatureId::new(chest_id.clone()),
            FeatureName::new(chest_id.clone()),
            CenteredAabb::from_center_size(chest_pos, *size),
            ChestFeature::new(ambition_interaction::Chest::new(
                chest_id,
                Some(pickup.clone()),
            )),
            BossRewardChest::new(placement_id.clone()),
        ));
        if looted {
            entity.insert(Opened);
        } else {
            entity.insert(FallingChest::new(0.0));
        }
    }
}

#[cfg(test)]
mod boss_reward_sync_tests;
#[cfg(test)]
mod reward_sync_tests;
