//! Reward-chest sync for cleared MOB encounters (the wave/`EncounterMob` kind).
//!
//! When an encounter clears, the matching reward chest entity should
//! exist in the room; when the save says the chest has been looted,
//! it should carry the `Opened` marker; when it has not been looted
//! yet, the chest should not carry that marker. These helpers are
//! the single point that mirrors that contract from the encounter
//! registry to the ECS.
//!
//! These two never shared anything but this file.

use super::*;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_combat::components::{
    CenteredAabb, ChestFeature, EncounterRewardChest, FeatureId, FeatureName, Opened,
};
use ambition_platformer2d_shared_tangle::lifecycle::{SessionSpawnScope, SpawnSessionScopedExt};
use bevy::prelude::Name;

/// Drop the encounter's ECS reward chest, if any, and clear its looted flag.
pub fn clear_encounter_reward_ecs(
    commands: &mut Commands,
    save: &mut ambition_persistence::save_data::AmbitionGameSaveData,
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
        ambition_encounter::encounter_reward_looted_flag(encounter_id),
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
    session_scope: SessionSpawnScope,
    save: &ambition_persistence::save_data::AmbitionGameSaveData,
    cleared: &[(String, ambition_encounter::EncounterSpec)],
    chests: &Query<
        (Entity, &EncounterRewardChest, &FeatureId, Option<&Opened>),
        With<ChestFeature>,
    >,
) {
    let chest_size = ae::Vec2::new(28.0, 28.0);
    for (encounter_id, spec) in cleared.iter() {
        let chest_id = format!("encounter_chest_{encounter_id}");
        let looted = save.flag(&ambition_encounter::encounter_reward_looted_flag(
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
        let chest_pos = ambition_encounter::encounter_reward_chest_pos(spec, chest_size);
        let mut entity = commands.spawn_session_scoped(
            session_scope,
            (
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
            ),
        );
        if looted {
            entity.insert(Opened);
        }
    }
}

#[cfg(test)]
mod reward_sync_tests;
