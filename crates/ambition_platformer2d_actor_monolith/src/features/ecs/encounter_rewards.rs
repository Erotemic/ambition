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
        // The retire half, on the same plugin. Ordered after the ONE drain,
        // because it reacts to this tick's presses; a reader that ran first
        // would retire on last tick's re-arm.
        app.add_systems(
            sim,
            retire_rewards_for_rearmed_encounters
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::Progression,
                )
                .after(ambition_encounter::switches::SwitchActivationDrained),
        );
    }
}

/// Retire an encounter's reward when its switch is re-armed.
///
/// ⭐ THIS COULD NOT BE A SYSTEM UNTIL THE SWITCH LOOP SPLIT. The trigger used
/// to be a POSITION inside the encounter adapter's drained loop — after a
/// save-mutating toggle and behind three early `continue`s — so nothing outside
/// could observe the edge: run before the drain and neither the queue nor the
/// toggle had happened, run after and both were gone. Now the switch domain
/// publishes `ResolvedSwitchActivations` with the post-toggle value, and the
/// edge is a value anyone can read.
///
/// ⛔ BEHAVIOUR IS UNCHANGED ON PURPOSE, including WHEN the flag clears. The
/// switch is the feature layer's input, the chest is its entity and
/// `reward_looted` is its save fact, so *"a switch-off retires the reward"* is
/// room-feature policy and stays that way. It is NOT keyed on
/// `EncounterEvent::Reset`: the death road resets an encounter with no switch
/// toggle, and clearing there would let a player who died after looting
/// re-clear and be paid twice — see
/// `encounter::tests::a_reset_does_not_retire_the_reward_chest`.
pub fn retire_rewards_for_rearmed_encounters(
    mut commands: bevy::prelude::Commands,
    mut save: bevy::prelude::ResMut<ambition_persistence::save::AmbitionGameSave>,
    switches: bevy::prelude::Res<ambition_encounter::switches::ResolvedSwitchActivations>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_world::rooms::RoomSet,
    >,
    chests: bevy::prelude::Query<
        (Entity, &EncounterRewardChest, &FeatureId, Option<&Opened>),
        With<ChestFeature>,
    >,
) {
    if switches.0.is_empty() {
        return;
    }
    for activation in &switches.0 {
        if !matches!(
            activation.action,
            ambition_encounter::switches::SwitchAction::ResetEncounter
        ) || activation.on
        {
            continue;
        }
        // An empty target means "the active room's own encounter", exactly as
        // the adapter resolved it.
        let target_id = if activation.target_encounter.is_empty() {
            rooms.active_spec().id.clone()
        } else {
            activation.target_encounter.clone()
        };
        clear_encounter_reward_ecs(&mut commands, save.data_mut(), &chests, &target_id);
    }
}

#[cfg(test)]
mod retire_on_rearm_tests {
    use super::*;
    use ambition_encounter::switches::{
        ResolvedSwitchActivation, ResolvedSwitchActivations, SwitchAction,
    };
    use ambition_persistence::save::AmbitionGameSave;
    use bevy::prelude::*;

    fn chest_app(activations: Vec<ResolvedSwitchActivation>) -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(AmbitionGameSave::default());
        app.insert_resource(ResolvedSwitchActivations(activations));
        // ⚠ THE SESSION ROOT IS NOT TEST SCAFFOLDING, it is the system's gate.
        // `SessionWorldRef` is a `Single<Ref<T>, With<SessionRoot>>`, so without
        // one the system is SKIPPED and every assertion below passes for the
        // wrong reason. The first version of this test had no root: the "off
        // retires the reward" case failed, and it failed because the system
        // never ran. That requirement is inherited, not new — the adapter this
        // logic left took the same param.
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            app.world_mut(),
            ambition_platformer2d_world::rooms::RoomSet::from_parts(
                "test_room",
                Vec::new(),
                Vec::new(),
            ),
        );
        let chest = app
            .world_mut()
            .spawn((
                ChestFeature::new(ambition_interaction::Chest::new(
                    "encounter_chest_goblin_encounter",
                    None,
                )),
                EncounterRewardChest {
                    encounter_id: "goblin_encounter".into(),
                },
                FeatureId("encounter_chest_goblin_encounter".into()),
            ))
            .id();
        app.add_systems(Update, retire_rewards_for_rearmed_encounters);
        (app, chest)
    }

    fn rearm(on: bool) -> ResolvedSwitchActivation {
        ResolvedSwitchActivation {
            id: "gate".into(),
            action: SwitchAction::ResetEncounter,
            target_encounter: "goblin_encounter".into(),
            on,
        }
    }

    /// A switch turned OFF retires the reward — the behaviour that used to live
    /// inside the encounter adapter's drained loop, unchanged.
    #[test]
    fn a_switch_turned_off_retires_the_reward() {
        let (mut app, chest) = chest_app(vec![rearm(false)]);
        app.update();
        assert!(
            app.world().get_entity(chest).is_err(),
            "re-arming an encounter must drop its chest so the next clear pays \
             out fresh"
        );
    }

    /// ⛔ A switch turned ON must NOT. The activation is the same kind; only
    /// `on` differs, and reacting to the kind alone would retire the reward on
    /// every press instead of every re-arm.
    #[test]
    fn a_switch_turned_on_leaves_the_reward_alone() {
        let (mut app, chest) = chest_app(vec![rearm(true)]);
        app.update();
        assert!(
            app.world().get_entity(chest).is_ok(),
            "only the OFF edge retires the reward; a press that arms the \
             encounter must leave a standing chest alone"
        );
    }

    /// ⛔ And an activation of another KIND must not, however it is toggled.
    #[test]
    fn a_gravity_switch_never_retires_a_reward() {
        let (mut app, chest) = chest_app(vec![ResolvedSwitchActivation {
            id: "gate".into(),
            action: SwitchAction::FlipGravity,
            target_encounter: "goblin_encounter".into(),
            on: false,
        }]);
        app.update();
        assert!(
            app.world().get_entity(chest).is_ok(),
            "a gravity switch shares the queue with the encounter reset and \
             nothing else; it must not reach an encounter's reward"
        );
    }
}
