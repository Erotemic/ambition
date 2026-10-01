//! Boss reward-chest sync: the ECS mirror of "this boss placement is cleared,
//! so its authored `DropChest` reward exists in the room".
//!
//! It lives with the boss domain: the only caller is
//! `boss_encounter::systems::update_boss_encounters`, and the reward shape it
//! reads (`BossRewardProfile::DropChest`) is boss vocabulary. The
//! mob-encounter sibling (`sync_encounter_reward_chests_ecs`) stays in
//! `features::ecs` with the `EncounterMob` wave vocabulary.

use super::BossRewardProfile;
use ambition_combat::falling_chest::settled_chest_center;
use ambition_combat::{
    BossRewardChest, CenteredAabb, ChestFeature, FallingChest, FeatureId, FeatureName, Opened,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{
    FeatureSimEntity, RoomVisual, SessionSpawnScope, SpawnSessionScopedExt,
};
use bevy::prelude::{Commands, Entity, Name, Query, With};

/// One boss placement's reward, where it would drop it.
#[derive(Clone, Debug)]
pub struct BossRewardAnchor {
    /// The chest and its looted flag are keyed by placement, so a cleared
    /// placement drops its own chest.
    pub placement_id: String,
    pub spawn: ae::Vec2,
    /// The boss's own reward (`BossConfig::seed`), not a lookup by archetype.
    pub reward: BossRewardProfile,
}

/// Idempotently ensure cleared boss encounters have ECS reward chests. This
/// helper receives boss spawn anchors from the boss encounter system and owns
/// the reward chest entity and state.
pub fn sync_boss_reward_chests_ecs(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    save: &ambition_persistence::save_data::AmbitionGameSaveData,
    world: &ae::World,
    boss_placements: &[BossRewardAnchor],
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
    for BossRewardAnchor { placement_id, spawn: boss_spawn, reward } in boss_placements {
        let BossRewardProfile::DropChest {
            pickup,
            offset,
            size,
        } = reward
        else {
            continue;
        };
        if !matches!(
            save.boss(placement_id),
            ambition_persistence::save_data::PersistedEncounterState::Cleared
        ) {
            continue;
        }
        let chest_id = ambition_encounter::encounter_chest_feature_id(placement_id);
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
