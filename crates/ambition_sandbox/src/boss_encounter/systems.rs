use std::collections::BTreeMap;

use ambition_engine as ae;
use bevy::prelude::*;

use crate::content::quest::QuestRegistry;
use crate::presentation::cutscene::CutsceneTriggerQueue;

use super::{
    default_boss_profiles, encounter_id_from_name, events::publish_events, BossEncounterRegistry,
    BossProfile,
};

pub fn populate_boss_encounter_registry(
    mut registry: ResMut<BossEncounterRegistry>,
    save: Res<crate::persistence::save::SandboxSave>,
) {
    if registry.specs_loaded {
        return;
    }
    for profile in default_boss_profiles() {
        registry.ensure_profile(profile);
    }
    let save_data = save.data();
    for (id, state) in registry.encounters.iter_mut() {
        let persisted = save_data.boss(id);
        if matches!(persisted, ae::PersistedEncounterState::Cleared) {
            // Already-defeated bosses skip straight to Death so the
            // arena renders empty next time the player walks in.
            // `phase = Dormant`, `hp = 0` is the cleanest carry-over
            // shape — the boss runtime won't spawn into the arena
            // and the encounter machinery stays silent.
            state.hp = 0;
        }
    }
    registry.specs_loaded = true;
}

/// Tick all live boss encounters. The single resource read keeps the
/// system param count low so this can be called as a regular Bevy
/// system without splitting.
pub fn update_boss_encounters(
    mut commands: Commands,
    world_time: Res<crate::WorldTime>,
    mut registry: ResMut<BossEncounterRegistry>,
    mut banner: ResMut<crate::features::GameplayBanner>,
    mut save: ResMut<crate::persistence::save::SandboxSave>,
    mut music_request: ResMut<crate::encounter::EncounterMusicRequest>,
    mut quests: ResMut<QuestRegistry>,
    mut cutscene_queue: ResMut<CutsceneTriggerQueue>,
    room_set: Res<crate::rooms::RoomSet>,
    world: Res<crate::GameWorld>,
    reward_chests: Query<
        (
            Entity,
            &crate::features::BossRewardChest,
            &crate::features::FeatureId,
            Option<&crate::features::Opened>,
            Option<&crate::features::FallingChest>,
        ),
        With<crate::features::ChestFeature>,
    >,
    mut bosses: Query<
        (
            &crate::features::FeatureId,
            &mut crate::features::BossFeature,
        ),
        With<crate::features::FeatureSimEntity>,
    >,
) {
    // Sim clock: encounter pacing (intro / phase-change timers,
    // reward grace) freezes alongside the player in bullet-time
    // (ADR 0010); we don't want phase transitions to fire while the
    // sim is stopped.
    let dt = world_time.sim_dt();
    let _active_room = room_set.active_spec().id.clone();

    // Build a list of boss runtime ids alive in the current room so we
    // can wake up encounters when the player walks in.
    let bosses_in_room: Vec<(String, String, ae::Vec2, ae::Vec2, i32, i32)> = bosses
        .iter()
        .map(|(_feature_id, feature)| {
            let b = &feature.boss;
            (
                b.id.clone(),
                b.name.clone(),
                b.pos,
                b.spawn,
                b.health.current,
                b.health.max,
            )
        })
        .collect();

    // Lazy registration: derive a *semantic* encounter id from the
    // boss's authored `name` (e.g. "clockwork warden" →
    // "clockwork_warden"). The LDtk iid (`BossSpawn-0158`) lives on
    // as the runtime_id link so combat damage still reaches the
    // right `BossRuntime`. Authored specs (registered before this
    // system runs) take precedence; only bosses without a spec fall
    // through to the auto-registered defaults.
    for (boss_runtime_id, boss_name, _pos, _spawn, _hp, max_hp) in &bosses_in_room {
        let encounter_id = encounter_id_from_name(boss_name);
        registry.link_runtime(&encounter_id, boss_runtime_id);
        if !registry.encounters.contains_key(&encounter_id) {
            let profile =
                BossProfile::for_encounter_id_or_name(&encounter_id).unwrap_or_else(|| {
                    BossProfile::generic(encounter_id.clone(), boss_name.clone(), *max_hp)
                });
            registry.ensure_profile(profile);
        }
    }

    // Wake up an encounter whose boss is now visible in the room.
    for (_runtime_id, boss_name, _pos, _spawn, _hp, _max) in &bosses_in_room {
        let encounter_id = encounter_id_from_name(boss_name);
        if let Some(state) = registry.encounters.get_mut(&encounter_id) {
            if matches!(state.phase, ae::BossEncounterPhase::Dormant) && state.hp > 0 {
                let evs = state.enter_intro();
                publish_events(
                    &encounter_id,
                    &evs,
                    &mut music_request,
                    &mut cutscene_queue,
                    &mut banner,
                );
            }
        }
    }

    // Tick all in-flight encounters. Unrolled because we need to
    // mutate the runtime with the boss reference based on each
    // encounter's HP, and the borrow checker prefers a copy-out then
    // route style.
    let mut deferred_events: Vec<(String, Vec<ae::BossEncounterEvent>)> = Vec::new();
    for (id, state) in registry.encounters.iter_mut() {
        if matches!(state.phase, ae::BossEncounterPhase::Dormant) {
            continue;
        }
        let evs = state.tick(dt);
        if !evs.is_empty() {
            deferred_events.push((id.clone(), evs));
        }
    }
    for (id, evs) in deferred_events {
        publish_events(
            &id,
            &evs,
            &mut music_request,
            &mut cutscene_queue,
            &mut banner,
        );
    }

    // Damage routing: when the sandbox `BossRuntime.health` decreases,
    // mirror the delta into the engine state and feed it back. The
    // BossRuntime is still the source of truth for HP because
    // existing combat/feature systems already mutate it; the engine
    // state is the *progression machine* fed by the damage delta.
    let runtime_id_lookup: BTreeMap<String, String> = registry.runtime_ids.clone();
    let profile_lookup = registry.profiles.clone();
    for (id, state) in registry.encounters.iter_mut() {
        let runtime_id = runtime_id_lookup
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.clone());
        for (feature_id, mut feature) in &mut bosses {
            if feature_id.as_str() != runtime_id {
                continue;
            }
            let boss = &mut feature.boss;
            if let Some(profile) = profile_lookup.get(id) {
                boss.apply_behavior_profile(profile.behavior.clone());
            }
            if matches!(save.data().boss(id), ae::PersistedEncounterState::Cleared) {
                boss.alive = false;
                boss.health.current = 0;
                break;
            }
            // Sync max_hp on first link (the BossRuntime defaults to 18,
            // the engine spec might say more). The engine spec wins
            // because it carries the design intent.
            if boss.health.max != state.spec.max_hp.max(1) {
                boss.health = ae::Health::new(state.spec.max_hp.max(1));
            }
            // Mirror engine HP into the runtime so combat reads a
            // single number.
            if boss.health.current != state.hp && state.hp > 0 {
                boss.health.current = state.hp;
            }
            // Suppress runtime-side death animation while boss is in an
            // invulnerable phase (Intro/Transition/Stagger).
            if state.phase.boss_invulnerable() && boss.alive {
                boss.hit_flash = 0.0;
            }
            // Death resolution: when engine state reports Death and the
            // outro is over, mark the runtime dead and update the save.
            if matches!(state.phase, ae::BossEncounterPhase::Death) && state.death_complete() {
                if boss.alive {
                    boss.alive = false;
                }
                let prior = save.data().boss(id);
                if !matches!(prior, ae::PersistedEncounterState::Cleared) {
                    save.data_mut()
                        .set_boss(id, ae::PersistedEncounterState::Cleared);
                    quests.push_event(ae::QuestAdvanceEvent::BossDefeated(id.clone()));
                }
            }
            break;
        }
    }

    // While any encounter is in flight, the encounter music request
    // takes precedence over the legacy mob-encounter request. We
    // write the boss's per-phase track as `desired_track`; if both a
    // mob encounter AND a boss are active (shouldn't happen at the
    // same time, but guard) the boss wins because the boss path runs
    // after `update_encounters_from_world`.
    if let Some((_id, phase)) = registry.active_phase() {
        let _ = phase; // Already published as MusicRequested events.
        let _ = music_request; // Already mutated in `publish_events`.
    }

    let boss_anchors: Vec<(String, ae::Vec2)> = bosses_in_room
        .iter()
        .map(|(runtime_id, _name, _pos, spawn, _hp, _max_hp)| (runtime_id.clone(), *spawn))
        .collect();
    crate::features::sync_boss_reward_chests_ecs(
        &mut commands,
        save.data(),
        &registry,
        &world.0,
        &boss_anchors,
        &reward_chests,
    );
}
