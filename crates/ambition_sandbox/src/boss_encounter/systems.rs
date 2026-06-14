use crate::engine_core as ae;
use bevy::prelude::*;

use crate::presentation::cutscene::CutsceneTriggerQueue;
use crate::quest::QuestRegistry;

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
    let ron_ids: std::collections::BTreeSet<String> = super::specs::boss_encounter_specs()
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
        if matches!(
            persisted,
            crate::persistence::save_data::PersistedEncounterState::Cleared
        ) {
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
            crate::features::BossClusterQueryData,
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
    // is the canonical encounter id resolved by `BossClusterScratch::new`
    // via the brain's `PhaseScript:` payload — preferred over the
    // raw LDtk name so e.g. a BossSpawn named "System Boss" with
    // brain `PhaseScript:clockwork_warden` correctly resolves to
    // the clockwork_warden profile (with its phase music tracks)
    // rather than a generic stub.
    let bosses_in_room: Vec<(String, String, String, ae::Vec2, ae::Vec2, i32, i32)> = bosses
        .iter()
        .map(|(_feature_id, feature)| {
            (
                feature.config.id.clone(),
                feature.config.name.clone(),
                feature.config.behavior.id.clone(),
                feature.kin.pos,
                feature.config.spawn,
                feature.status.health.current,
                feature.status.health.max,
            )
        })
        .collect();

    // Lazy registration: use the boss runtime's `behavior.id` as
    // the canonical encounter id. The LDtk iid (`BossSpawn-0158`)
    // lives on as the runtime_id link so combat damage still
    // reaches the right boss entity. Authored specs (registered
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
                if matches!(
                    state.phase,
                    crate::boss_encounter::BossEncounterPhase::Dormant
                ) && state.hp > 0
                {
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
    let mut deferred_events: Vec<(String, Vec<crate::boss_encounter::BossEncounterEvent>)> =
        Vec::new();
    for (id, state) in registry.encounters.iter_mut() {
        if matches!(
            state.phase,
            crate::boss_encounter::BossEncounterPhase::Dormant
        ) {
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
            if let Some(profile) = profiles.get(id) {
                feature
                    .as_boss_mut()
                    .apply_behavior_profile(profile.behavior.clone());
            }
            if matches!(
                save.data().boss(id),
                crate::persistence::save_data::PersistedEncounterState::Cleared
            ) {
                feature.status.alive = false;
                feature.status.health.current = 0;
                break;
            }
            // Sync max_hp on first link (the clusters default to 18,
            // the engine spec might say more). The engine spec wins
            // because it carries the design intent.
            if feature.status.health.max != state.spec.max_hp.max(1) {
                feature.status.health = crate::actor::Health::new(state.spec.max_hp.max(1));
            }
            // Mirror engine HP into the runtime so combat reads a
            // single number.
            if feature.status.health.current != state.hp && state.hp > 0 {
                feature.status.health.current = state.hp;
            }
            // Suppress runtime-side death animation while boss is in an
            // invulnerable phase (Intro/Transition/Stagger).
            if state.phase.boss_invulnerable() && feature.status.alive {
                feature.status.hit_flash = 0.0;
            }
            // Death resolution: when engine state reports Death and the
            // outro is over, mark the boss dead and update the save.
            if matches!(
                state.phase,
                crate::boss_encounter::BossEncounterPhase::Death
            ) && state.death_complete()
            {
                if feature.status.alive {
                    feature.status.alive = false;
                }
                let prior = save.data().boss(id);
                if !matches!(
                    prior,
                    crate::persistence::save_data::PersistedEncounterState::Cleared
                ) {
                    save.data_mut().set_boss(
                        id,
                        crate::persistence::save_data::PersistedEncounterState::Cleared,
                    );
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
    let active_boss_music_track =
        bosses_in_room
            .iter()
            .find_map(|(_, _, encounter_id, _, _, _, _)| {
                registry
                    .encounters
                    .get(encounter_id)
                    .and_then(active_phase_music_track)
                    .map(str::to_owned)
            });
    match active_boss_music_track {
        Some(track) => {
            if music_request.desired_track.as_deref() != Some(track.as_str()) {
                bevy::log::info!(
                    target: "ambition::boss_encounter",
                    "restoring boss music for active encounter — track={track:?}",
                );
                music_request.desired_track = Some(track);
            }
        }
        None if music_request.desired_track.is_some() => {
            bevy::log::info!(
                target: "ambition::boss_encounter",
                "clearing boss music (no boss in active phase) — prior track={:?}",
                music_request.desired_track,
            );
            music_request.desired_track = None;
        }
        None => {}
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

fn active_phase_music_track(state: &crate::boss_encounter::BossEncounterState) -> Option<&str> {
    let track = match state.phase {
        crate::boss_encounter::BossEncounterPhase::Intro => &state.spec.music_intro,
        crate::boss_encounter::BossEncounterPhase::Phase1
        | crate::boss_encounter::BossEncounterPhase::Transition => &state.spec.music_phase1,
        crate::boss_encounter::BossEncounterPhase::Phase2
        | crate::boss_encounter::BossEncounterPhase::Stagger => &state.spec.music_phase2,
        crate::boss_encounter::BossEncounterPhase::Enrage => &state.spec.music_enrage,
        crate::boss_encounter::BossEncounterPhase::Dormant
        | crate::boss_encounter::BossEncounterPhase::Death => return None,
    };
    (!track.is_empty()).then_some(track.as_str())
}

/// Camera-shake amplitude (px) on a dramatic boss phase change. Capped to 14 by
/// [`CameraShakeState::kick`].
const BOSS_PHASE_SHAKE_PX: f32 = 11.0;

/// Boss phase-transition feedback (Jon: a boss should "scream" / "feel loud" on
/// a phase change "without breaking the player's ears"). On a transition INTO a
/// dramatic phase (Transition / Phase2 / Enrage / Stagger) we kick the camera
/// shake and play a placeholder "cry" SFX — a feel layer that works for every
/// boss, including the new FSM + T-rex. The dedicated per-boss scream SPRITE
/// animation + a bespoke quiet "cry" SFX are follow-ups; this reuses the existing
/// shake + a soft impact sound as placeholders.
///
/// Decoupled from the phase-advance / event pipeline on purpose: it diffs each
/// boss's phase in the registry against a `Local` snapshot, so it needs no
/// changes to `publish_events` or its four callers.
pub fn boss_phase_transition_feedback(
    registry: Res<BossEncounterRegistry>,
    mut last_phase: Local<
        std::collections::HashMap<String, crate::boss_encounter::BossEncounterPhase>,
    >,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    // Optional: a headless / camera-less build may not insert the shake resource.
    mut shake: Option<ResMut<crate::time::camera_ease::CameraShakeState>>,
    // Boss entities, to resolve the encounter id back to the actor that emits
    // the phase-transition shockwave.
    bosses: Query<
        (
            Entity,
            &crate::features::FeatureId,
            &crate::features::BodyKinematics,
            &crate::features::FeatureAabb,
        ),
        With<crate::features::BossConfig>,
    >,
    mut effects: MessageWriter<crate::effects::EffectRequest>,
    mut vfx: MessageWriter<ambition_effects::vfx::VfxMessage>,
) {
    use crate::boss_encounter::BossEncounterPhase as P;
    for (id, state) in &registry.encounters {
        let prev = last_phase.insert(id.clone(), state.phase);
        // React only to an actual change, and skip the first observation
        // (`prev == None`) so a freshly-registered boss doesn't shake on spawn.
        if prev.is_none() || prev == Some(state.phase) {
            continue;
        }
        if matches!(
            state.phase,
            P::Transition | P::Phase2 | P::Enrage | P::Stagger
        ) {
            if let Some(shake) = shake.as_deref_mut() {
                shake.kick(BOSS_PHASE_SHAKE_PX);
            }
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_ROCK_HIT,
                pos: ae::Vec2::ZERO,
            });
            // The transition is now a dodge-able GAMEPLAY beat, not just feel: the
            // boss emits a `DamageBox` effect through the SAME generic
            // `apply_effects` consumer the player's shockwave gauntlet uses.
            // Resolved at the boss's own position + faction (`ActorFaction::Boss`),
            // so the shared `apply_hitbox_damage` lands it on the player — the
            // literal "player and boss fire the same attack" unification, in-game.
            if let Some((entity, _, kin, aabb)) =
                bosses.iter().find(|(_, fid, _, _)| fid.as_str() == id)
            {
                effects.write(crate::effects::EffectRequest {
                    owner: entity,
                    effect: crate::effects::Effect::DamageBox(crate::effects::DamageBoxEffect {
                        center: aabb.center,
                        faction: crate::features::ActorFaction::Boss,
                        half_extent: ae::Vec2::new(170.0, 80.0),
                        damage: 2,
                        knockback: 1.6,
                        lifetime_s: 0.30,
                        name: Some("Shockwave AOE"),
                    }),
                });
                // "Scream lines": a sharp radial spark burst FROM the boss, so the
                // phase change reads as a dramatic beat instead of a silent state
                // flip (#122 "transitions are not noticeable / too short"). The
                // radial Spark burst is the placeholder; a bespoke scream-line
                // sprite is later polish (TODO §A boss transitions).
                vfx.write(ambition_effects::vfx::VfxMessage::Burst {
                    pos: kin.pos,
                    count: 24,
                    speed: 340.0,
                    color: [1.0, 0.92, 0.45, 0.95],
                    kind: ambition_effects::vfx::ParticleKind::Spark,
                });
            }
        }
    }
}

#[cfg(test)]
mod phase_feedback_tests {
    use super::*;
    use crate::boss_encounter::roster::BossSpecRoster;
    use crate::boss_encounter::{BossEncounterPhase, BossEncounterState};
    use crate::time::camera_ease::CameraShakeState;

    fn registry_with_boss(phase: BossEncounterPhase) -> BossEncounterRegistry {
        let mut state =
            BossEncounterState::new(crate::boss_encounter::BossEncounterSpec::gradient_sentinel());
        state.phase = phase;
        let mut reg = BossEncounterRegistry::default();
        reg.encounters
            .insert("gradient_sentinel".to_string(), state);
        reg
    }

    fn shake_px(app: &App) -> f32 {
        app.world().resource::<CameraShakeState>().amplitude_px
    }

    #[test]
    fn dramatic_phase_change_kicks_the_camera_shake() {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_message::<crate::effects::EffectRequest>();
        app.add_message::<ambition_effects::vfx::VfxMessage>();
        app.init_resource::<CameraShakeState>();
        app.insert_resource(registry_with_boss(BossEncounterPhase::Phase1));
        app.add_systems(Update, boss_phase_transition_feedback);

        // First observation (Phase1) — no shake.
        app.update();
        assert_eq!(shake_px(&app), 0.0, "no shake on first observation");

        // Phase1 → Enrage (dramatic) → shake kicks.
        app.world_mut()
            .resource_mut::<BossEncounterRegistry>()
            .encounters
            .get_mut("gradient_sentinel")
            .unwrap()
            .phase = BossEncounterPhase::Enrage;
        app.update();
        assert!(shake_px(&app) > 0.0, "dramatic transition kicks the shake");
    }

    #[test]
    fn non_dramatic_change_does_not_shake() {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_message::<crate::effects::EffectRequest>();
        app.add_message::<ambition_effects::vfx::VfxMessage>();
        app.init_resource::<CameraShakeState>();
        app.insert_resource(registry_with_boss(BossEncounterPhase::Intro));
        app.add_systems(Update, boss_phase_transition_feedback);
        app.update(); // observe Intro
                      // Intro → Phase1 (not dramatic) → no shake.
        app.world_mut()
            .resource_mut::<BossEncounterRegistry>()
            .encounters
            .get_mut("gradient_sentinel")
            .unwrap()
            .phase = BossEncounterPhase::Phase1;
        app.update();
        assert_eq!(shake_px(&app), 0.0, "Phase1 is not a dramatic transition");
    }
}
