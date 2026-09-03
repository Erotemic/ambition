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

// ⭐ NAMED, NOT GLOBBED. This was `use super::*` over the whole
// `features/ecs` module — a channel no `crate::` grep sees. Measured by
// deleting it: bevy's prelude, `ambition_vfx`'s two message types and
// `RoomVisual`, which is `shared_tangle`'s. No monolith vocabulary.
use ambition_combat::components::{
    CenteredAabb, ChestFeature, EncounterRewardChest, FeatureId, FeatureName, Opened,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_platformer2d_shared_tangle::lifecycle::RoomVisual;
use ambition_platformer2d_shared_tangle::lifecycle::{SessionSpawnScope, SpawnSessionScopedExt};
use bevy::prelude::Name;
use bevy::prelude::*;

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

/// The feature layer REACTING to the encounter domain's published fact.
///
/// ⭐ THE DIRECTION IS THE POINT. This used to be a call: an adapter inside the
/// actor kernel read `EncounterLifecycle::phase`, filtered for `Completed`,
/// assembled `(id, spec)` pairs and pushed them into
/// [`sync_encounter_reward_chests_ecs`]. That adapter was the only reason the
/// kernel had to know how an encounter represents "completed". The domain
/// publishes [`ambition_encounter::rewards::ClearedEncounters`] now, and a
/// reward chest is a room FEATURE, which this layer owns — so the feature layer
/// reads the fact and the adapter has nothing to do.
pub fn sync_encounter_reward_chests(
    mut commands: ambition_platformer2d_shared_tangle::lifecycle::SessionCommands<'_, '_>,
    save: bevy::prelude::Res<ambition_persistence::save::AmbitionGameSave>,
    cleared: bevy::prelude::Res<ambition_encounter::rewards::ClearedEncounters>,
    chests: bevy::prelude::Query<
        (Entity, &EncounterRewardChest, &FeatureId, Option<&Opened>),
        With<ChestFeature>,
    >,
) {
    // No live session scope means nothing is spawnable this tick, which is the
    // same gate the adapter used before this system existed.
    let Some(session_scope) = commands.spawn_scope() else {
        return;
    };
    sync_encounter_reward_chests_ecs(&mut commands, session_scope, save.data(), &cleared.0, &chests);
}

/// Composes [`sync_encounter_reward_chests`] on the feature side.
///
/// Ordered `.after(EncounterLifecycleSet)` — the set the domain publishes its
/// cleared list inside — so this reads a list that agrees with the phases the
/// same tick settled. The runtime adds this beside the other feature plugins;
/// no registration for it lands in the encounter adapter.
pub struct EncounterRewardSyncPlugin;

impl bevy::prelude::Plugin for EncounterRewardSyncPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;
        use bevy::prelude::IntoScheduleConfigs as _;
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            sync_encounter_reward_chests
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::Progression,
                )
                .after(ambition_encounter::EncounterLifecycleSet),
        );
    }
}
