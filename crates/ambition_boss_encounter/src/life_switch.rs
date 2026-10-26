//! A boss's life, set by a switch (`SwitchAction::BossLife`).
//!
//! The Hall of Bosses has a switch by each door (Jon, 2026-10-06): green while
//! the boss behind it is alive, red while it is dead. Flipping a red one
//! revives the boss; flipping a green one kills it, "and if the boss is loaded
//! in the simulation ... it brings the boss health to zero and kills it
//! immediately, otherwise if it is not loaded, it just marks the boss as dead,
//! so next time you go into the room its as if you had already killed it."
//!
//! The switch stores nothing of its own: the boss's record in the save
//! (keyed by placement, Q57) IS its state, so a boss killed in a fight turns
//! its switch red with no second write to forget.
//!
//! A succession is one fight (the Inugami rises when Wolfenstein falls,
//! `RoomSpec::boss_successions`): the switch sets the life of its placement
//! and of every boss that wakes after it.

use bevy::prelude::{Commands, Entity, Query, Res, ResMut, With};

/// Carry out this tick's boss-life switch presses.
///
/// - **Kill** (the switch was green): the placement is recorded `Cleared`
///   and its reward marked looted, so a switch kill pays nothing. A live body
///   of it goes to zero health and its phase to `Death`; from the next tick
///   `update_boss_encounters` holds a cleared placement dead, as it does a
///   corpse the room was built with.
/// - **Revive** (it was red): the defeat's records are retracted as a replay
///   retracts them (`retract_defeat_records`: the record back to `Untouched`,
///   the looted flag cleared, the quest steps it advanced put back), its
///   chest is taken away, and a live body of it is re-seeded at its spawn:
///   `update_boss_encounters` seeds health and phase afresh for a boss with no
///   encounter state, as for one just built.
///
/// Ordered after the ONE switch drain (it reacts to this tick's presses) and
/// before `update_boss_encounters` (which then sees the new life the same
/// tick).
/// Run condition of [`apply_boss_life_switches`]: this tick's presses hold a
/// boss-life switch.
pub fn a_boss_life_switch_is_pressed(switches: Res<ambition_encounter::switches::ResolvedSwitchActivations>,
) -> bool {
    switches
        .0
        .iter()
        .any(|activation| {
        matches!(activation.action, ambition_encounter::switches::SwitchAction::BossLife)
    })
}

pub fn apply_boss_life_switches(
    mut commands: Commands,
    switches: Res<ambition_encounter::switches::ResolvedSwitchActivations>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    mut quests: ResMut<ambition_persistence::quest::QuestRegistry>,
    chests: Query<(Entity, &ambition_combat::BossRewardChest), With<ambition_world_items::containers::ChestFeature>,
    >,
    mut bosses: Query<(
        &crate::BossConfig,
        &mut crate::BossEncounter,
        &mut ambition_characters::actor::BodyHealth,
        ambition_platformer2d_core::BodyClusterQueryData,
        &mut ambition_platformer2d_core::MotionModel,
    )>,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
) {
    for activation in &switches.0 {
        if !matches!(activation.action, ambition_encounter::switches::SwitchAction::BossLife) {
            continue;
        }
        if activation.target_encounter.is_empty() {
            continue;
        }
        for target in rooms.rooms().boss_succession_from(&activation.target_encounter) {
            set_boss_life(&mut commands, &mut save, &mut quests, &chests, &mut bosses, &target, activation.on,
            );
        }
    }
}

/// One placement of a switch's fight: alive (`on`) or dead.
#[allow(clippy::type_complexity)]
fn set_boss_life(
    commands: &mut Commands,
    save: &mut ambition_persistence::save::AmbitionGameSave,
    quests: &mut ambition_persistence::quest::QuestRegistry,
    chests: &Query<(Entity, &ambition_combat::BossRewardChest), With<ambition_world_items::containers::ChestFeature>,
    >,
    bosses: &mut Query<(
        &crate::BossConfig,
        &mut crate::BossEncounter,
        &mut ambition_characters::actor::BodyHealth,
        ambition_platformer2d_core::BodyClusterQueryData,
        &mut ambition_platformer2d_core::MotionModel,
    )>,
    target: &str,
    on: bool,
) {
    // The drain published the state the press asks for: ON = alive.
    if on {
        crate::retraction::retract_defeat_records(save.data_mut(), quests, target);
        for (chest, reward) in chests {
            if reward.encounter_id == target {
                commands.entity(chest).despawn();
            }
        }
        for (config, mut status, _health, mut cluster_item, mut model) in bosses.iter_mut() {
            if config.id == target {
                status.encounter = None;
                // A discrete transit (ADR 0024), not a field write: the
                // boss leaves wherever it fell, and contacts and the motion
                // record described that place.
                ambition_platformer2d_core::movement::transit_body(
                    &mut model,
                    &mut cluster_item.as_clusters_mut(),
                    config.spawn,
                    ambition_platformer2d_core::movement::TransitVelocity::Zero,
                );
            }
        }
    } else {
        let data = save.data_mut();
        data.set_boss(target, ambition_persistence::save_data::PersistedEncounterState::Cleared,
        );
        data.set_flag(ambition_encounter::encounter_reward_looted_flag(target), true,
        );
        for (config, mut status, mut health, _clusters, _model) in bosses.iter_mut() {
            if config.id == target {
                health.health.current = 0;
                if let Some(phase) = status.encounter.as_mut() {
                    let _ = phase.kill();
                }
            }
        }
    }
}
