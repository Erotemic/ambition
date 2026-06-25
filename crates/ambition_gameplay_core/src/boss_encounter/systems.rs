//! Boss-encounter Bevy systems — the per-frame driver.
//!
//! `populate_boss_encounter_registry` (startup) loads the read-only profile
//! catalog. `update_boss_encounters` (per sim-tick) seeds + wakes bosses in the
//! active room, ticks each phase machine, publishes events, mirrors phase
//! HP/phase onto the boss ECS clusters, manages the adaptive-music request
//! lifetime, and syncs reward chests.
//! `boss_phase_transition_feedback` diffs each boss's phase against a `Local`
//! snapshot to fire camera shake + a `DamageBox` shockwave + scream VFX on
//! dramatic transitions — decoupled from the event pipeline on purpose.

use ambition_engine_core as ae;
use bevy::prelude::*;

use crate::cutscene_trigger::CutsceneTriggerQueue;
use crate::quest::QuestRegistry;

use super::{default_boss_profiles, events::publish_events, BossEncounterRegistry, BossProfile};

pub fn populate_boss_encounter_registry(mut registry: ResMut<BossEncounterRegistry>) {
    if registry.specs_loaded {
        return;
    }
    // Per ADR 0017: named boss encounter specs are authored in
    // `ambition_content/assets/data/boss_encounters/<id>.ron` and installed
    // before the registry is populated. Log a one-time startup census so a
    // missing content install or empty roster is visible immediately.
    let profiles = default_boss_profiles();
    let total = profiles.len();
    bevy::log::info!(
        target: "ambition::boss_encounter",
        "boss_encounter registry: {total} content-installed profile(s) loaded"
    );
    for profile in profiles {
        registry.ensure_profile(profile);
    }
    // The registry is a read-only DATA CATALOG (profiles only). Persisted
    // "cleared" is applied per-entity in `update_boss_encounters` against the
    // boss's own state, not pre-seeded here.
    registry.specs_loaded = true;
}

/// Drive every boss's entity-local phase mechanism: seed from the profile
/// catalog, wake, tick the `BossPhaseState`, resolve death (save + quest), keep
/// the adaptive-music request live, and sync reward chests.
/// `BossStatus.health` + `BossStatus.encounter` ARE the source of truth.
pub fn update_boss_encounters(
    mut commands: Commands,
    world_time: Res<crate::WorldTime>,
    registry: Res<BossEncounterRegistry>,
    mut banner: ResMut<crate::features::GameplayBanner>,
    mut save: ResMut<crate::persistence::save::SandboxSave>,
    mut music_request: ResMut<crate::encounter::BossEncounterMusicRequest>,
    mut quests: ResMut<QuestRegistry>,
    mut cutscene_queue: ResMut<CutsceneTriggerQueue>,
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
            Option<&crate::features::BossOverrides>,
        ),
        With<crate::features::FeatureSimEntity>,
    >,
) {
    // Sim clock: phase pacing (intro / phase-change timers, death outro,
    // reward grace) freezes alongside the player in bullet-time (ADR 0010); we
    // don't want phase transitions to fire while the sim is stopped.
    let dt = world_time.sim_dt();

    // Active-fight music track (first fighting boss wins) + reward anchors,
    // collected as we drive each boss. Anchors carry (placement_id,
    // archetype_id, spawn): R4 keys "cleared" + rewards by PLACEMENT.
    let mut active_music_track: Option<String> = None;
    let mut boss_anchors: Vec<(String, String, ae::Vec2)> = Vec::new();

    for (_feature_id, mut feature, overrides) in &mut bosses {
        let archetype_id = feature.config.behavior.id.clone();
        let runtime_id = feature.config.id.clone();
        let boss_name = feature.config.name.clone();

        // Resolve the authored profile from the read-only catalog (or a generic
        // stub). `behavior.id` is the canonical archetype id resolved at spawn
        // from the brain's `PhaseScript:` payload.
        let profile = registry
            .profiles
            .get(&archetype_id)
            .cloned()
            .or_else(|| BossProfile::for_encounter_id_or_name(&archetype_id))
            .unwrap_or_else(|| {
                BossProfile::generic(
                    archetype_id.clone(),
                    boss_name.clone(),
                    feature.status.health.max,
                )
            });
        let spec = profile.encounter.clone();

        // Seed entity-local state ONCE from the profile (phase triggers as data
        // + HP + behavior). Two of the same boss seed independent state by
        // construction. The per-spawn `BossOverrides` (hp / combat_size / phase
        // triggers) are applied HERE so the profile application above can't
        // clobber them.
        if feature.status.encounter.is_none() {
            feature
                .as_boss_mut()
                .apply_behavior_profile(profile.behavior.clone());
            if let Some(size) = overrides.and_then(|o| o.combat_size) {
                feature.config.behavior.combat_size = Some(size);
            }
            let max_hp = overrides
                .and_then(|o| o.max_hp)
                .unwrap_or(spec.max_hp)
                .max(1);
            feature.status.health = ambition_characters::actor::Health::new(max_hp);
            let triggers = overrides
                .and_then(|o| o.phase_triggers.clone())
                .unwrap_or_else(|| crate::boss_encounter::PhaseTrigger::intrinsic_from_spec(&spec));
            feature.status.encounter = Some(crate::boss_encounter::BossPhaseState::new(triggers));
        }

        // Persisted "cleared" is keyed to this PLACEMENT, NOT the archetype (R4) —
        // a cleared placement renders defeated and is otherwise inert. Shared
        // predicate (`boss_is_cleared`) with the room-load save-sync so they
        // can't drift.
        if crate::features::boss_is_cleared(&save, &feature.config) {
            feature.status.alive = false;
            feature.status.health.current = 0;
            if let Some(phase) = feature.status.encounter.as_mut() {
                phase.phase = crate::boss_encounter::BossEncounterPhase::Death;
            }
            continue;
        }

        // Wake (Dormant → start) while alive, then advance the phase mechanism.
        // The phase ticks even when not alive so a dead boss's death OUTRO timer
        // advances (so `death_outro_complete` can fire).
        let alive = feature.status.alive;
        let hp_fraction = feature.status.health.ratio();
        let mut phase_events = Vec::new();
        {
            let phase = feature.status.encounter.as_mut().expect("seeded above");
            if alive
                && matches!(
                    phase.phase,
                    crate::boss_encounter::BossEncounterPhase::Dormant
                )
            {
                phase_events.extend(phase.wake());
            }
            phase_events.extend(phase.tick(dt, hp_fraction));
        }
        for ev in &phase_events {
            let encounter_events = phase_event_to_encounter_events(ev, &spec);
            publish_events(
                &archetype_id,
                &encounter_events,
                &mut music_request,
                &mut cutscene_queue,
                &mut banner,
            );
        }

        // Read post-tick state for death resolution + music + invuln.
        let (phase, death_done, invulnerable) = {
            let p = feature.status.encounter.as_ref().expect("seeded");
            (
                p.phase,
                p.death_outro_complete(spec.death_seconds),
                p.boss_invulnerable(),
            )
        };

        // Suppress the death-flash overlay during invulnerable beats.
        if invulnerable && feature.status.alive {
            feature.status.hit_flash = 0.0;
        }

        // Death resolution: once the outro elapses, record this PLACEMENT as
        // Cleared (R4) + fire the quest event (idempotent — only the first time
        // the placement flips to Cleared). The quest event still carries the
        // ARCHETYPE id (quest objectives are about the boss kind, e.g. "defeat
        // the Gradient Sentinel").
        if matches!(phase, crate::boss_encounter::BossEncounterPhase::Death) && death_done {
            if feature.status.alive {
                feature.status.alive = false;
            }
            if !crate::features::boss_is_cleared(&save, &feature.config) {
                save.data_mut().set_boss(
                    &runtime_id,
                    crate::persistence::save_data::PersistedEncounterState::Cleared,
                );
                quests.push_event(crate::quest::QuestAdvanceEvent::BossDefeated(
                    archetype_id.clone(),
                ));
            }
        }

        // Collect the active-fight music + the reward anchor (placement_id,
        // archetype_id, spawn): the reward sync keys the chest + looted flag by
        // PLACEMENT and resolves the DropChest reward via the archetype profile.
        if active_music_track.is_none() {
            if let Some(track) = phase_music_track(&spec, phase) {
                if !track.is_empty() {
                    active_music_track = Some(track.to_string());
                }
            }
        }
        boss_anchors.push((
            runtime_id.clone(),
            archetype_id.clone(),
            feature.config.spawn,
        ));
    }

    // Music-request lifetime: keep the active boss's track up; clear it when no
    // boss is in an active-fight phase (boss defeated, or player left the room
    // so no boss entities exist) so room music resumes. Pinned by
    // `boss_music_plays_during_the_fight` +
    // `defeated_boss_is_recorded_cleared_drops_reward_and_clears_music`.
    match active_music_track {
        Some(track) => {
            if music_request.desired_track.as_deref() != Some(track.as_str()) {
                music_request.desired_track = Some(track);
            }
        }
        None if music_request.desired_track.is_some() => {
            music_request.desired_track = None;
        }
        None => {}
    }

    crate::features::sync_boss_reward_chests_ecs(
        &mut commands,
        save.data(),
        &registry,
        &world.0,
        &boss_anchors,
        &reward_chests,
    );
}

/// The adaptive-music track a boss plays in `phase`, from its authored spec.
/// `None` for `Dormant` / `Death` (no boss music — room music resumes).
fn phase_music_track(
    spec: &crate::boss_encounter::BossEncounterSpec,
    phase: crate::boss_encounter::BossEncounterPhase,
) -> Option<&str> {
    use crate::boss_encounter::BossEncounterPhase as P;
    let track = match phase {
        P::Intro => &spec.music_intro,
        P::Phase1 | P::Transition => &spec.music_phase1,
        P::Phase2 | P::Stagger => &spec.music_phase2,
        P::Enrage => &spec.music_enrage,
        P::Dormant | P::Death => return None,
    };
    (!track.is_empty()).then_some(track.as_str())
}

/// Bridge an entity-local [`BossPhaseEvent`](crate::boss_encounter::BossPhaseEvent)
/// to the existing [`publish_events`] consumers (banner / intro cutscene / music).
/// The brief `TransitionLockStarted` tell carries no banner/music of its own.
fn phase_event_to_encounter_events(
    ev: &crate::boss_encounter::BossPhaseEvent,
    spec: &crate::boss_encounter::BossEncounterSpec,
) -> Vec<crate::boss_encounter::BossEncounterEvent> {
    use crate::boss_encounter::{BossEncounterEvent, BossEncounterPhase, BossPhaseEvent};
    match ev {
        BossPhaseEvent::PhaseChanged { from, to } => {
            let mut out = vec![BossEncounterEvent::PhaseChanged {
                from: *from,
                to: *to,
            }];
            if let Some(track) = phase_music_track(spec, *to) {
                if !track.is_empty() {
                    out.push(BossEncounterEvent::MusicRequested {
                        track: track.to_string(),
                    });
                }
            }
            if matches!(to, BossEncounterPhase::Death) {
                out.push(BossEncounterEvent::Defeated);
            }
            out
        }
        BossPhaseEvent::TransitionLockStarted { .. } => Vec::new(),
    }
}

/// Camera-shake amplitude (px) on a dramatic boss phase change. Capped to 14 by
/// [`CameraShakeState::kick`].
const BOSS_PHASE_SHAKE_PX: f32 = 11.0;

/// Boss phase-transition feedback (Jon: a boss should "scream" / "feel loud" on
/// a phase change "without breaking the player's ears"). On a transition INTO a
/// dramatic phase (Transition / Phase2 / Enrage / Stagger) we kick the camera
/// shake and play a placeholder "cry" SFX — a feel layer that works for every
/// boss. The dedicated per-boss scream SPRITE animation + a bespoke quiet "cry"
/// SFX are follow-ups; this reuses the existing shake + a soft impact sound as
/// placeholders.
///
/// Decoupled from the phase-advance / event pipeline on purpose: it diffs each
/// boss's entity-local phase (`BossStatus.encounter`) against a `Local`
/// snapshot, so it needs no changes to `publish_events`.
pub fn boss_phase_transition_feedback(
    mut last_phase: Local<
        std::collections::HashMap<String, crate::boss_encounter::BossEncounterPhase>,
    >,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    // Optional: a headless / camera-less build may not insert the shake resource.
    mut shake: Option<ResMut<crate::time::camera_ease::CameraShakeState>>,
    // Boss entities — phase read from the entity-local state + the actor that
    // emits the phase-transition shockwave.
    bosses: Query<
        (
            Entity,
            &crate::features::FeatureId,
            &crate::features::BodyKinematics,
            &crate::features::CenteredAabb,
            &crate::combat::boss_clusters::BossStatus,
        ),
        With<crate::features::BossConfig>,
    >,
    mut effects: MessageWriter<crate::effects::EffectRequest>,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
) {
    use crate::boss_encounter::BossEncounterPhase as P;
    for (entity, feature_id, kin, aabb, status) in &bosses {
        let Some(phase) = status.encounter.as_ref().map(|p| p.phase) else {
            continue;
        };
        let id = feature_id.as_str();
        let prev = last_phase.insert(id.to_string(), phase);
        // React only to an actual change, and skip the first observation
        // (`prev == None`) so a freshly-seeded boss doesn't shake on spawn.
        if prev.is_none() || prev == Some(phase) {
            continue;
        }
        if matches!(phase, P::Transition | P::Phase2 | P::Enrage | P::Stagger) {
            if let Some(shake) = shake.as_deref_mut() {
                shake.kick(BOSS_PHASE_SHAKE_PX);
            }
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_ROCK_HIT,
                pos: ae::Vec2::ZERO,
            });
            // The transition is a dodge-able GAMEPLAY beat, not just feel: the
            // boss emits a `DamageBox` effect through the SAME generic
            // `apply_effects` consumer the player's shockwave gauntlet uses.
            // Resolved at the boss's own position + faction (`ActorFaction::Boss`),
            // so the shared `apply_hitbox_damage` lands it on the player — the
            // literal "player and boss fire the same attack" unification, in-game.
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
            // flip (#122 "transitions are not noticeable / too short").
            vfx.write(ambition_vfx::vfx::VfxMessage::Burst {
                pos: kin.pos,
                count: 24,
                speed: 340.0,
                color: [1.0, 0.92, 0.45, 0.95],
                kind: ambition_vfx::vfx::ParticleKind::Spark,
            });
        }
    }
}

#[cfg(test)]
mod phase_feedback_tests {
    use super::*;
    use crate::boss_encounter::BossEncounterPhase;
    use crate::combat::boss_clusters::test_support::{test_boss_config, test_boss_status};
    use crate::combat::boss_clusters::BossStatus;
    use crate::features::{BodyKinematics, CenteredAabb, FeatureId};
    use crate::time::camera_ease::CameraShakeState;

    fn spawn_boss(app: &mut App, phase: BossEncounterPhase) -> Entity {
        let config = test_boss_config("gradient_sentinel", "Gradient Sentinel", "clockwork_warden");
        let status = test_boss_status(100, phase);
        app.world_mut()
            .spawn((
                FeatureId::new("gradient_sentinel"),
                BodyKinematics {
                    pos: ae::Vec2::ZERO,
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::splat(64.0),
                    facing: 1.0,
                },
                CenteredAabb::from_center_size(ae::Vec2::ZERO, ae::Vec2::splat(64.0)),
                config,
                status,
            ))
            .id()
    }

    fn set_phase(app: &mut App, entity: Entity, phase: BossEncounterPhase) {
        let mut entity_mut = app.world_mut().entity_mut(entity);
        let mut status = entity_mut.get_mut::<BossStatus>().unwrap();
        if let Some(p) = status.encounter.as_mut() {
            p.phase = phase;
        }
    }

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.add_message::<crate::effects::EffectRequest>();
        app.add_message::<ambition_vfx::vfx::VfxMessage>();
        app.init_resource::<CameraShakeState>();
        app.add_systems(Update, boss_phase_transition_feedback);
        app
    }

    fn shake_px(app: &App) -> f32 {
        app.world().resource::<CameraShakeState>().amplitude_px
    }

    #[test]
    fn dramatic_phase_change_kicks_the_camera_shake() {
        let mut app = test_app();
        let boss = spawn_boss(&mut app, BossEncounterPhase::Phase1);

        // First observation (Phase1) — no shake.
        app.update();
        assert_eq!(shake_px(&app), 0.0, "no shake on first observation");

        // Phase1 → Enrage (dramatic) → shake kicks.
        set_phase(&mut app, boss, BossEncounterPhase::Enrage);
        app.update();
        assert!(shake_px(&app) > 0.0, "dramatic transition kicks the shake");
    }

    #[test]
    fn non_dramatic_change_does_not_shake() {
        let mut app = test_app();
        let boss = spawn_boss(&mut app, BossEncounterPhase::Intro);
        app.update(); // observe Intro
                      // Intro → Phase1 (not dramatic) → no shake.
        set_phase(&mut app, boss, BossEncounterPhase::Phase1);
        app.update();
        assert_eq!(shake_px(&app), 0.0, "Phase1 is not a dramatic transition");
    }
}
