use ambition_engine as ae;
use bevy::prelude::*;

use crate::content::quest::QuestRegistry;
use crate::presentation::cutscene::CutsceneTriggerQueue;

use super::{default_boss_profiles, events::publish_events, BossEncounterRegistry, BossProfile};

pub fn populate_boss_encounter_registry(
    mut registry: ResMut<BossEncounterRegistry>,
    save: Res<crate::persistence::save::SandboxSave>,
) {
    if registry.specs_loaded {
        return;
    }
    // Per ADR 0017: boss-encounter numeric fields can come from
    // `assets/data/boss_encounters/<id>.ron` (override) or the
    // hardcoded `crate::boss_encounter::BossEncounterSpec::<id>()` constructor
    // (fallback). Log a one-time startup census so a regression where
    // a RON file silently fails to parse (loader returns empty) is
    // visible in dev logs without paging through every spec field.
    let ron_ids: std::collections::BTreeSet<String> = super::specs::load_boss_specs_from_disk()
        .into_iter()
        .map(|s| s.id)
        .collect();
    let profiles = default_boss_profiles();
    let ron_count = profiles.iter().filter(|p| ron_ids.contains(&p.id)).count();
    let total = profiles.len();
    bevy::log::info!(
        target: "ambition::boss_encounter",
        "boss_encounter registry: {total} profile(s) loaded ({ron_count} RON-overridden, {} constructor-only)",
        total - ron_count
    );
    for profile in profiles {
        registry.ensure_profile(profile);
    }
    let save_data = save.data();
    for (id, state) in registry.encounters.iter_mut() {
        let persisted = save_data.boss(id);
        if matches!(persisted, crate::save::PersistedEncounterState::Cleared) {
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
    mut music_request: ResMut<crate::encounter::BossEncounterMusicRequest>,
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
    // can wake up encounters when the player walks in. `behavior_id`
    // is the canonical encounter id resolved by `BossRuntime::new`
    // via the brain's `PhaseScript:` payload — preferred over the
    // raw LDtk name so e.g. a BossSpawn named "System Boss" with
    // brain `PhaseScript:clockwork_warden` correctly resolves to
    // the clockwork_warden profile (with its phase music tracks)
    // rather than a generic stub.
    let bosses_in_room: Vec<(String, String, String, ae::Vec2, ae::Vec2, i32, i32)> = bosses
        .iter()
        .map(|(_feature_id, feature)| {
            let b = &feature.boss;
            (
                b.id.clone(),
                b.name.clone(),
                b.behavior.id.clone(),
                b.pos,
                b.spawn,
                b.health.current,
                b.health.max,
            )
        })
        .collect();

    // Lazy registration: use the boss runtime's `behavior.id` as
    // the canonical encounter id. The LDtk iid (`BossSpawn-0158`)
    // lives on as the runtime_id link so combat damage still
    // reaches the right `BossRuntime`. Authored specs (registered
    // before this system runs) take precedence; only bosses without
    // a spec fall through to the auto-registered defaults.
    for (boss_runtime_id, boss_name, encounter_id, _pos, _spawn, _hp, max_hp) in &bosses_in_room {
        registry.link_runtime(encounter_id, boss_runtime_id);
        if !registry.encounters.contains_key(encounter_id) {
            let profile =
                BossProfile::for_encounter_id_or_name(encounter_id).unwrap_or_else(|| {
                    BossProfile::generic(encounter_id.clone(), boss_name.clone(), *max_hp)
                });
            registry.ensure_profile(profile);
        }
    }

    // Wake up an encounter whose boss is now visible in the room.
    // Only the wake-up transition logs (Dormant → Intro); the
    // per-frame "encounter is in phase X" line is gated to debug!
    // so the steady-state doesn't flood the log every frame.
    for (_runtime_id, boss_name, encounter_id, _pos, _spawn, _hp, _max) in &bosses_in_room {
        match registry.encounters.get_mut(encounter_id) {
            Some(state) => {
                if matches!(state.phase, crate::boss_encounter::BossEncounterPhase::Dormant) && state.hp > 0 {
                    bevy::log::info!(
                        target: "ambition::boss_encounter",
                        "wakeup: encounter={encounter_id} (boss={boss_name:?}) phase=Dormant hp={} → enter_intro",
                        state.hp,
                    );
                    let evs = state.enter_intro();
                    publish_events(
                        encounter_id,
                        &evs,
                        &mut music_request,
                        &mut cutscene_queue,
                        &mut banner,
                    );
                    bevy::log::info!(
                        target: "ambition::boss_encounter",
                        "wakeup: {encounter_id} emitted {} event(s); music_request.desired_track = {:?}",
                        evs.len(),
                        music_request.desired_track,
                    );
                }
            }
            None => {
                bevy::log::warn!(
                    target: "ambition::boss_encounter",
                    "wakeup: encounter_id={encounter_id} (boss={boss_name:?}) NOT FOUND in registry; available={:?}",
                    registry.encounters.keys().collect::<Vec<_>>()
                );
            }
        }
    }

    // Tick all in-flight encounters. Unrolled because we need to
    // mutate the runtime with the boss reference based on each
    // encounter's HP, and the borrow checker prefers a copy-out then
    // route style.
    let mut deferred_events: Vec<(String, Vec<crate::boss_encounter::BossEncounterEvent>)> = Vec::new();
    for (id, state) in registry.encounters.iter_mut() {
        if matches!(state.phase, crate::boss_encounter::BossEncounterPhase::Dormant) {
            continue;
        }
        let evs = state.tick(dt);
        if !evs.is_empty() {
            deferred_events.push((id.clone(), evs));
        }
    }
    for (id, evs) in deferred_events {
        if !evs.is_empty() {
            bevy::log::info!(
                target: "ambition::boss_encounter",
                "encounter {id} ticked → {} event(s); music_request before={:?}",
                evs.len(),
                music_request.desired_track,
            );
        }
        publish_events(
            &id,
            &evs,
            &mut music_request,
            &mut cutscene_queue,
            &mut banner,
        );
        if !evs.is_empty() {
            bevy::log::info!(
                target: "ambition::boss_encounter",
                "encounter {id} → music_request after={:?}",
                music_request.desired_track,
            );
        }
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
            if matches!(save.data().boss(id), crate::save::PersistedEncounterState::Cleared) {
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
            if matches!(state.phase, crate::boss_encounter::BossEncounterPhase::Death) && state.death_complete() {
                if boss.alive {
                    boss.alive = false;
                }
                let prior = save.data().boss(id);
                if !matches!(prior, crate::save::PersistedEncounterState::Cleared) {
                    save.data_mut()
                        .set_boss(id, crate::save::PersistedEncounterState::Cleared);
                    quests.push_event(crate::quest::QuestAdvanceEvent::BossDefeated(id.clone()));
                }
            }
            break;
        }
    }

    // Music-request lifetime: `publish_events` only ever SETS the
    // desired_track (via the engine's `MusicRequested` events on
    // phase transitions); nothing on the engine side clears it.
    // That's correct for phase-to-phase transitions inside a
    // fight, but it traps two cases:
    //
    // 1. **Player leaves the arena**: the boss runtime despawns
    //    when the room changes, but `boss_music.desired_track`
    //    stays at the last phase's track. The next room would
    //    keep playing violin instead of its own room music.
    //
    // 2. **Boss defeated**: the encounter transitions to Death.
    //    The fight is over; we want the room's default music
    //    back, not the violin loop playing over the dying boss.
    //
    // So at the end of each tick, check whether ANY of the
    // bosses currently in the room has an encounter in an
    // "active fight" phase. If none does, clear the boss-music
    // request so the priority resolver falls back to the regular
    // encounter music / room default.
    //
    // "Active fight" includes Stagger (still in combat, briefly
    // unable to act) but excludes Dormant + Death. An empty
    // bosses_in_room (player left the room) is treated the same
    // as "no active fight" → clear.
    let any_boss_in_active_fight = bosses_in_room
        .iter()
        .any(|(_, _, encounter_id, _, _, _, _)| {
            registry
                .encounters
                .get(encounter_id)
                .map(|state| {
                    matches!(
                        state.phase,
                        crate::boss_encounter::BossEncounterPhase::Intro
                            | crate::boss_encounter::BossEncounterPhase::Phase1
                            | crate::boss_encounter::BossEncounterPhase::Transition
                            | crate::boss_encounter::BossEncounterPhase::Phase2
                            | crate::boss_encounter::BossEncounterPhase::Enrage
                            | crate::boss_encounter::BossEncounterPhase::Stagger
                    )
                })
                .unwrap_or(false)
        });
    if !any_boss_in_active_fight && music_request.desired_track.is_some() {
        bevy::log::info!(
            target: "ambition::boss_encounter",
            "clearing boss music (no boss in active phase) — prior track={:?}",
            music_request.desired_track,
        );
        music_request.desired_track = None;
    }

    let boss_anchors: Vec<(String, ae::Vec2)> = bosses_in_room
        .iter()
        .map(|(runtime_id, _name, _enc_id, _pos, spawn, _hp, _max_hp)| (runtime_id.clone(), *spawn))
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
