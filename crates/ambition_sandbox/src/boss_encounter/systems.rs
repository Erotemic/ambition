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

    // Per-frame mirror: engine `BossEncounterState` is the source of
    // truth for boss HP (OVERNIGHT-TODO #8). The ECS damage system
    // calls `record_boss_damage` directly and writes the post-hit HP
    // onto `boss.health.current` on the same tick, so this pass is
    // mostly idempotent — it covers the edge cases where engine state
    // changed without a player damage event (boss revival on retry,
    // save-driven Cleared mark, max_hp spec change).
    //
    // Disjoint field borrows: split `registry` into a mutable borrow
    // of `encounters` and immutable borrows of `runtime_ids` /
    // `profiles`. Previously this cloned both maps to avoid a
    // borrow conflict; with destructuring the compiler can prove the
    // borrows are disjoint and the clones go away.
    let registry_mut = &mut *registry;
    let BossEncounterRegistry {
        encounters,
        profiles,
        runtime_ids,
        ..
    } = registry_mut;
    for (id, state) in encounters.iter_mut() {
        let runtime_id = runtime_ids
            .get(id)
            .map(String::as_str)
            .unwrap_or(id.as_str());
        for (feature_id, mut feature) in &mut bosses {
            if feature_id.as_str() != runtime_id {
                continue;
            }
            let boss = &mut feature.boss;
            if let Some(profile) = profiles.get(id) {
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

    // Per-phase music tracks are emitted as `MusicRequested` events
    // from `publish_events` above when the engine state machine fires
    // a phase transition, so the runtime mirror doesn't need to do a
    // post-tick `registry.active_phase()` re-read here. (The previous
    // implementation had a `let _ = phase; let _ = music_request;`
    // pair to acknowledge that the values exist; with the inversion
    // landed, those acknowledgements are no longer load-bearing.)
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
