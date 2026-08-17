//! Boss reward-chest sync — the ECS mirror of "this boss placement is cleared,
//! so its authored `DropChest` reward exists in the room".
//!
//! Lives with the boss domain that owns the contract: the only caller is
//! `boss_encounter::systems::update_boss_encounters`, and the reward shape it
//! reads (`BossRewardProfile::DropChest`) is boss vocabulary. The mob-encounter
//! sibling (`sync_encounter_reward_chests_ecs`) stays in `features::ecs`, whose
//! `EncounterMob` wave vocabulary it shares.

use super::{BossEncounterRegistry, BossRewardProfile};
use ambition_combat::falling_chest::settled_chest_center;
use ambition_combat::{
    BossRewardChest, CenteredAabb, ChestFeature, FallingChest, FeatureId, FeatureName, Opened,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{
    FeatureSimEntity, RoomVisual, SessionSpawnScope, SpawnSessionScopedExt,
};
use bevy::prelude::{Commands, Entity, Name, Query, With};

/// Idempotently ensure cleared boss encounters have ECS reward chests.
/// Boss actors are ECS entities now; this helper receives their spawn anchors
/// from the boss encounter system and owns the reward chest entity/state natively.
pub fn sync_boss_reward_chests_ecs(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    save: &ambition_persistence::save_data::AmbitionGameSaveData,
    registry: &BossEncounterRegistry,
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
        let BossRewardProfile::DropChest {
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
        let looted = save.flag(&ambition_encounter::encounter_reward_looted_flag(
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
        let mut entity = commands.spawn_session_scoped(
            session_scope,
            (
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
            ),
        );
        if looted {
            entity.insert(Opened);
        } else {
            entity.insert(FallingChest::new(0.0));
        }
    }
}

#[cfg(test)]
mod boss_reward_sync_tests;
