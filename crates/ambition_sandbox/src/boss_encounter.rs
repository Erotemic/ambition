//! Sandbox-side boss encounter coordinator.
//!
//! Bridges `ae::BossEncounterState` (the phase machine) with the
//! existing `BossRuntime` (the in-arena physical actor) and the
//! adaptive music + cutscene + save-state systems.
//!
//! Each `BossSpawn` LDtk entity in the active room maps to one
//! encounter id (defaulting to the boss `name`). When the player
//! enters the room the encounter goes Dormant → Intro and the
//! cutscene queue is asked to play `boss_intro_<id>`. From that point
//! the engine state machine drives transitions; this module mirrors
//! them onto the seldom_state `BossPhase` component, the audio
//! request, and the save resource.

use std::collections::BTreeMap;

use ambition_engine as ae;
use bevy::prelude::*;

use crate::cutscene::CutsceneTriggerQueue;
use crate::quest::QuestRegistry;

#[derive(Resource, Default)]
pub struct BossEncounterRegistry {
    pub encounters: BTreeMap<String, ae::BossEncounterState>,
    /// id -> the boss runtime id we wired to. Used to route damage.
    pub runtime_ids: BTreeMap<String, String>,
    /// True once we've registered the default boss specs.
    pub specs_loaded: bool,
}

impl BossEncounterRegistry {
    pub fn ensure(&mut self, spec: ae::BossEncounterSpec) {
        let id = spec.id.clone();
        self.encounters
            .entry(id)
            .or_insert_with(|| ae::BossEncounterState::new(spec));
    }

    pub fn get(&self, id: &str) -> Option<&ae::BossEncounterState> {
        self.encounters.get(id)
    }

    pub fn link_runtime(&mut self, encounter_id: &str, runtime_id: &str) {
        self.runtime_ids
            .insert(encounter_id.to_string(), runtime_id.to_string());
    }

    pub fn active_phase(&self) -> Option<(&str, ae::BossEncounterPhase)> {
        for (id, state) in &self.encounters {
            if !matches!(state.phase, ae::BossEncounterPhase::Dormant) {
                return Some((id.as_str(), state.phase));
            }
        }
        None
    }
}

/// Default boss specs shipped with the sandbox. Populated lazily so
/// hot reloads of LDtk content don't double-register.
pub fn default_boss_specs() -> Vec<ae::BossEncounterSpec> {
    vec![ae::BossEncounterSpec::gradient_sentinel()]
}

pub fn populate_boss_encounter_registry(
    mut registry: ResMut<BossEncounterRegistry>,
    save: Res<crate::save::SandboxSave>,
) {
    if registry.specs_loaded {
        return;
    }
    for spec in default_boss_specs() {
        registry.ensure(spec);
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
    time: Res<Time>,
    mut registry: ResMut<BossEncounterRegistry>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    mut save: ResMut<crate::save::SandboxSave>,
    mut music_request: ResMut<crate::encounter::EncounterMusicRequest>,
    mut quests: ResMut<QuestRegistry>,
    mut cutscene_queue: ResMut<CutsceneTriggerQueue>,
    room_set: Res<crate::rooms::RoomSet>,
) {
    let dt = time.delta_secs();
    let _active_room = room_set.active_spec().id.clone();

    // Build a list of boss runtime ids alive in the current room so we
    // can wake up encounters when the player walks in.
    let bosses_in_room: Vec<(String, String, ae::Vec2, i32, i32)> = runtime
        .features
        .bosses
        .iter()
        .map(|b| (b.id.clone(), b.name.clone(), b.pos, b.health.current, b.health.max))
        .collect();

    // Lazy registration: any boss runtime in the current room that
    // doesn't yet have an encounter spec gets a generic
    // gradient-sentinel-based spec registered against the boss
    // runtime id. Authored specs take precedence (ensure is no-op
    // if already present); naming the LDtk Bosses with the engine
    // spec id pulls them into a tuned encounter.
    for (boss_id, boss_name, _pos, _hp, max_hp) in &bosses_in_room {
        if registry.encounters.contains_key(boss_id) {
            continue;
        }
        let mut spec = ae::BossEncounterSpec::gradient_sentinel();
        spec.id = boss_id.clone();
        spec.name = boss_name.clone();
        // Pick up the runtime's authored max_hp so the encounter
        // doesn't replace it on first link.
        spec.max_hp = (*max_hp).max(1);
        registry.ensure(spec);
    }

    // Wake up an encounter whose boss is now visible in the room.
    for (boss_id, _name, _pos, _hp, _max) in &bosses_in_room {
        // Match the encounter id by the boss runtime id (sandbox
        // convention: BossSpawn name == encounter id == boss runtime
        // id). Future content can map them through LDtk fields.
        if let Some(state) = registry.encounters.get_mut(boss_id) {
            if matches!(state.phase, ae::BossEncounterPhase::Dormant) && state.hp > 0 {
                let evs = state.enter_intro();
                publish_events(
                    boss_id,
                    &evs,
                    &mut music_request,
                    &mut cutscene_queue,
                    &mut runtime.features,
                );
                registry.link_runtime(boss_id, boss_id);
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
            &mut runtime.features,
        );
    }

    // Damage routing: when the sandbox `BossRuntime.health` decreases,
    // mirror the delta into the engine state and feed it back. The
    // BossRuntime is still the source of truth for HP because
    // existing combat/feature systems already mutate it; the engine
    // state is the *progression machine* fed by the damage delta.
    let runtime_id_lookup: BTreeMap<String, String> = registry.runtime_ids.clone();
    for (id, state) in registry.encounters.iter_mut() {
        let runtime_id = runtime_id_lookup
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.clone());
        let Some(boss) = runtime.features.bosses.iter_mut().find(|b| b.id == runtime_id) else {
            continue;
        };
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
        // invulnerable phase (Intro/Transition/Stagger). We use a
        // hack: writing damage 0 just feeds the tick. Real damage
        // routing happens via `on_boss_damaged` below from the
        // `apply_player_attack` site.
        if state.phase.boss_invulnerable() && boss.alive {
            // Reset hit flash so the arena reads "neutral" during
            // the locked beats — small but readable presentation
            // smoothing.
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
                // Push a quest advance event so any quest watching
                // this boss can progress.
                quests.push_event(ae::QuestAdvanceEvent::BossDefeated(id.clone()));
            }
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
}

fn publish_events(
    encounter_id: &str,
    events: &[ae::BossEncounterEvent],
    music_request: &mut crate::encounter::EncounterMusicRequest,
    cutscene_queue: &mut CutsceneTriggerQueue,
    features: &mut crate::features::FeatureRuntime,
) {
    for event in events {
        match event {
            ae::BossEncounterEvent::PhaseChanged { to, .. } => {
                if matches!(to, ae::BossEncounterPhase::Intro) {
                    cutscene_queue.request(format!("boss_intro_{encounter_id}"));
                }
                features.banner = match to {
                    ae::BossEncounterPhase::Intro => format!("BOSS APPROACHES — {encounter_id}"),
                    ae::BossEncounterPhase::Phase1 => "PHASE 1".to_string(),
                    ae::BossEncounterPhase::Transition => "...".to_string(),
                    ae::BossEncounterPhase::Phase2 => "PHASE 2".to_string(),
                    ae::BossEncounterPhase::Stagger => "STAGGERED — punish".to_string(),
                    ae::BossEncounterPhase::Enrage => "ENRAGED".to_string(),
                    ae::BossEncounterPhase::Death => "DEFEATED".to_string(),
                    ae::BossEncounterPhase::Dormant => String::new(),
                };
                features.banner_timer = 1.4;
            }
            ae::BossEncounterEvent::MusicRequested { track } => {
                if !track.is_empty() {
                    music_request.desired_track = Some(track.clone());
                }
            }
            ae::BossEncounterEvent::DamageApplied { .. } => {}
            ae::BossEncounterEvent::Defeated => {
                // Death cutscene swap could go here in a richer build.
                features.banner = format!("VICTORY: {encounter_id}");
                features.banner_timer = 2.5;
            }
        }
    }
}

/// Helper: feed a damage delta into the encounter machine. Called by
/// `apply_player_attack` after damage hits the BossRuntime.
pub fn record_boss_damage(
    registry: &mut BossEncounterRegistry,
    music_request: &mut crate::encounter::EncounterMusicRequest,
    cutscene_queue: &mut CutsceneTriggerQueue,
    features: &mut crate::features::FeatureRuntime,
    boss_runtime_id: &str,
    damage: i32,
) {
    let Some((id, _)) = registry
        .runtime_ids
        .iter()
        .find(|(_id, runtime_id)| runtime_id.as_str() == boss_runtime_id)
        .map(|(id, runtime_id)| (id.clone(), runtime_id.clone()))
    else {
        return;
    };
    let Some(state) = registry.encounters.get_mut(&id) else {
        return;
    };
    let evs = state.apply_player_damage(damage);
    publish_events(&id, &evs, music_request, cutscene_queue, features);
}
