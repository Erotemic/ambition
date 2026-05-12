use ambition_engine as ae;

use super::{BossEncounterRegistry, BossRewardProfile, MOCKINGBIRD_ENCOUNTER_ID};

/// Idempotent sync: when a boss encounter is `Cleared`, make sure any declared
/// death reward exists in the live arena.
///
/// This is now profile-driven: every boss can declare `BossRewardProfile`, and
/// this function applies the same spawn/reload/looted-flag rules for each one.
/// The old mockingbird helper remains as a compatibility wrapper because tests
/// and existing scheduling still use that name.
pub fn sync_boss_reward_chests(
    features: &mut crate::features::FeatureRuntime,
    save: &ae::SandboxSaveData,
    registry: &BossEncounterRegistry,
    world: &ae::World,
) {
    for (encounter_id, profile) in &registry.profiles {
        let BossRewardProfile::DropChest {
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
        let Some(boss) = features.bosses.iter().find(|b| b.id == runtime_id) else {
            continue;
        };
        let chest_pos = boss.spawn + *offset;
        let chest_id = format!("encounter_chest_{encounter_id}");
        let just_spawned = features.chests.iter().all(|c| c.id != chest_id);
        features.spawn_chest(chest_id.clone(), Some(pickup.clone()), chest_pos, *size);
        let looted = save.flag(&crate::encounter::encounter_reward_looted_flag(
            encounter_id,
        ));
        if let Some(chest) = features.chests.iter_mut().find(|c| c.id == chest_id) {
            chest.opened = looted;
            if just_spawned {
                chest.falling = true;
                chest.vel_y = 0.0;
                if looted {
                    let virtual_dt = 1.0 / 60.0;
                    for _ in 0..240 {
                        if !chest.falling {
                            break;
                        }
                        crate::features::tick_chest_fall(chest, world, virtual_dt);
                    }
                }
            }
        }
    }
}

/// Backward-compatible wrapper for the existing mockingbird reward sync entry
/// point. The implementation is profile-driven now, so future bosses should add
/// a `BossRewardProfile` instead of adding another named sync function.
pub fn sync_mockingbird_treasure_chest(
    features: &mut crate::features::FeatureRuntime,
    save: &ae::SandboxSaveData,
    registry: &BossEncounterRegistry,
    world: &ae::World,
) {
    let _ = MOCKINGBIRD_ENCOUNTER_ID;
    sync_boss_reward_chests(features, save, registry, world);
}
