//! Boss-encounter Bevy systems: the per-frame driver.
//!
//! `populate_boss_encounter_registry` (startup) loads the read-only profile
//! catalog. `update_boss_encounters` (per sim tick) seeds and wakes bosses in
//! the active room, ticks each phase machine, publishes events, mirrors phase
//! HP/phase onto the boss ECS clusters, manages the adaptive-music request, and
//! syncs reward chests. `boss_phase_transition_feedback` consumes the
//! `BossPhaseChanged` edge that driver announces and fires camera shake, a
//! `DamageBox` shockwave and scream VFX on dramatic transitions.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_cutscene::CutsceneTriggerQueue;
use ambition_persistence::quest::QuestRegistry;

use super::{
    default_boss_profiles, events::publish_events, BossCatalog, BossEncounterRegistry, BossProfile,
};

/// This system's claim on the encounter layer's priority music tier.
pub const BOSS_MUSIC_OWNER: &str = "boss_encounter";

pub fn populate_boss_encounter_registry(
    catalog: Res<BossCatalog>,
    mut registry: ResMut<BossEncounterRegistry>,
) {
    if registry.specs_loaded {
        return;
    }
    if catalog.is_empty() {
        bevy::log::info!(
            target: "ambition_boss_encounter",
            "boss_encounter registry: App has no boss catalog fragments"
        );
        registry.specs_loaded = true;
        return;
    }
    // Per ADR 0017: named boss encounter specs are authored in
    // `ambition_content/assets/data/boss_encounters/<id>.ron` and assembled
    // into the App-local catalog before this runs. Log a one-time startup
    // census, so a missing provider or an empty catalog is visible.
    let profiles = default_boss_profiles(&catalog);
    let total = profiles.len();
    bevy::log::info!(
        target: "ambition_boss_encounter",
        "boss_encounter registry: {total} App-local profile(s) loaded"
    );
    for profile in profiles {
        registry.ensure_profile(profile);
    }
    // The registry is a read-only data catalog (profiles only). Persisted
    // "cleared" is applied per entity in `update_boss_encounters`.
    registry.specs_loaded = true;
}

/// Drive every boss's entity-local phase mechanism: seed from the profile
/// catalog, wake, tick the `ActorPhaseState`, resolve death (save + quest), keep
/// the adaptive-music request live, and sync reward chests.
/// The body's `BodyHealth` and `BossEncounter.encounter` are the source of truth.
pub fn update_boss_encounters(
    mut commands: Commands,
    catalog: Res<BossCatalog>,
    world_time: Res<ambition_time::WorldTime>,
    registry: Res<BossEncounterRegistry>,
    mut banner: ResMut<ambition_combat::GameplayBanner>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    mut music_request: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldMut<
        ambition_encounter::EncounterMusicRequest,
    >,
    mut quests: ResMut<QuestRegistry>,
    mut cutscene_queue: ResMut<CutsceneTriggerQueue>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    active_session: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
    reward_chests: Query<
        (
            Entity,
            &ambition_combat::BossRewardChest,
            &ambition_combat::FeatureId,
            Option<&ambition_combat::Opened>,
            Option<&ambition_combat::FallingChest>,
        ),
        With<ambition_combat::ChestFeature>,
    >,
    // P0.2: the phase machine's own edge, announced where it is committed.
    mut phase_changes: MessageWriter<super::events::BossPhaseChanged>,
    mut bosses: Query<
        (
            Entity,
            &ambition_combat::FeatureId,
            crate::BossClusterQueryData,
            // The boss's shared body components: HP authority and the
            // hit-flash/reaction timers.
            &mut ambition_characters::actor::BodyHealth,
            &mut ambition_characters::actor::BodyCombat,
            Option<&crate::BossOverrides>,
        ),
        With<ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity>,
    >,
) {
    let Some(session_scope) =
        ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::for_optional_active_session(
            active_session.as_deref(),
        )
    else {
        return;
    };

    // Sim clock: phase pacing (intro/phase-change timers, death outro, reward
    // grace) freezes with the player in bullet-time (ADR 0010), so phase
    // transitions do not fire while the sim is stopped.
    let dt = world_time.sim_dt();

    // Active-fight music track (first fighting boss wins) and reward anchors,
    // collected per boss. Anchors are (placement_id, archetype_id, spawn):
    // "cleared" and rewards are keyed by placement.
    let mut active_music_track: Option<String> = None;
    let mut boss_anchors: Vec<(String, String, ae::Vec2)> = Vec::new();

    for (boss_entity, _feature_id, mut feature, mut health, mut combat, overrides) in &mut bosses {
        let archetype_id = feature.config.behavior.id.clone();
        let runtime_id = feature.config.id.clone();
        let boss_name = feature.config.name.clone();

        // Resolve the authored profile from the read-only catalog (or a
        // generic stub). `behavior.id` is the canonical archetype id resolved
        // at spawn from the brain's `PhaseScript:` payload.
        let profile = registry
            .profiles
            .get(&archetype_id)
            .cloned()
            .or_else(|| BossProfile::for_encounter_id_or_name(&catalog, &archetype_id))
            .unwrap_or_else(|| {
                BossProfile::generic(
                    &catalog,
                    archetype_id.clone(),
                    boss_name.clone(),
                    health.max(),
                )
            });
        let spec = profile.encounter.clone();

        // Seed entity-local state once from the profile (phase triggers, HP,
        // behavior), so two of the same boss have independent state. The
        // per-spawn `BossOverrides` (hp / phase triggers) are applied here, so
        // the profile cannot overwrite them. The body size is not seeded here:
        // construction resolved `kin.size`.
        if feature.status.encounter.is_none() {
            feature
                .as_boss_mut()
                .apply_behavior_profile(profile.behavior.clone());
            let max_hp = overrides
                .and_then(|o| o.max_hp)
                .unwrap_or(spec.max_hp)
                .max(1);
            *health = ambition_characters::actor::BodyHealth::new(
                ambition_characters::actor::Health::new(max_hp),
            );
            let triggers = overrides
                .and_then(|o| o.phase_triggers.clone())
                .unwrap_or_else(|| crate::PhaseTrigger::intrinsic_from_spec(&spec));
            feature.status.encounter = Some(crate::ActorPhaseState::new(triggers));
        }

        // Persisted "cleared" is keyed to this placement, not the archetype. A
        // cleared placement renders defeated and is otherwise inert. The
        // predicate (`boss_is_cleared`) is shared with construction, so they
        // agree.
        if crate::boss_is_cleared(&save, &feature.config) {
            health.health.current = 0;
            if let Some(phase) = feature.status.encounter.as_mut() {
                phase.phase = crate::BossEncounterPhase::Death;
            }
            continue;
        }

        // Was the death already settled before this tick? Read before the
        // tick, because the record below is written on the event's edge.
        //
        // Do not re-derive it from the corpse each frame: a room replay that
        // retracts the record (so the boss can be fought again) would have its
        // retraction overwritten on the next frame.
        let death_was_already_settled = feature
            .status
            .encounter
            .as_ref()
            .is_some_and(|phase| phase.death_outro_complete(spec.death_seconds));

        // Wake (Dormant → start) while alive, then advance the phase
        // mechanism. The phase also ticks when dead, so the death outro timer
        // advances (and `death_outro_complete` can fire).
        let alive = health.alive();
        let hp_fraction = health.health.ratio();
        let mut phase_events = Vec::new();
        {
            let phase = feature.status.encounter.as_mut().expect("seeded above");
            if alive && matches!(phase.phase, crate::BossEncounterPhase::Dormant) {
                phase_events.extend(phase.wake());
            }
            phase_events.extend(phase.tick(dt, hp_fraction));
        }
        for ev in &phase_events {
            publish_events(&archetype_id, ev, &mut cutscene_queue, &mut banner);
            // the transition edge, from the authority that commits it. Every
            // consumer of "this boss just changed phase" reads this, not a
            // diff against its own memory (see `BossPhaseChanged`).
            if let crate::BossPhaseEvent::PhaseChanged { from, to } = ev {
                phase_changes.write(super::events::BossPhaseChanged {
                    boss: boss_entity,
                    from: *from,
                    to: *to,
                });
            }
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
        if invulnerable && health.alive() {
            combat.hit_flash = 0.0;
        }

        // Death resolution: once the outro ends, record this placement as
        // Cleared and fire the quest event (once, when the placement first
        // becomes Cleared). The quest event carries the archetype id (quest
        // objectives are about the boss kind, e.g. "defeat the Gradient
        // Sentinel").
        if matches!(phase, crate::BossEncounterPhase::Death) && death_done {
            // A scripted or environmental kill can reach Death with HP left;
            // zero it so `alive()`, the liveness authority, agrees.
            if health.alive() {
                health.health.current = 0;
            }
            // The edge, not the resting state: recorded on the frame the outro
            // completes and never again, so the record can be retracted while
            // the corpse stands. `boss_is_cleared` still guards the quest
            // event, which fires once per placement.
            if !death_was_already_settled && !crate::boss_is_cleared(&save, &feature.config) {
                save.data_mut().set_boss(
                    &runtime_id,
                    ambition_persistence::save_data::PersistedEncounterState::Cleared,
                );
                quests.push_event(
                    ambition_persistence::quest::QuestAdvanceEvent::BossDefeated(
                        archetype_id.clone(),
                    ),
                );
            }
        }

        // Collect the active-fight music and the reward anchor
        // (placement_id, archetype_id, spawn): the reward sync keys the chest
        // and looted flag by placement and resolves the DropChest reward via
        // the archetype profile.
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

    // Music-request lifetime: keep the active boss's track up; clear it when
    // no boss is in an active-fight phase (defeated, or the player left the
    // room), so room music resumes. Guarded by
    // `boss_music_plays_during_the_fight` and
    // `defeated_boss_is_recorded_cleared_drops_reward_and_clears_music`.
    //
    // Release only this system's own claim. It has no run condition, so the
    // "no boss is fighting" arm runs every frame of every game; clearing the
    // whole tier would silence every other music claimant.
    match active_music_track {
        Some(track) => music_request.claim_priority(BOSS_MUSIC_OWNER, track),
        None => music_request.release_priority(BOSS_MUSIC_OWNER),
    }

    crate::sync_boss_reward_chests_ecs(
        &mut commands,
        session_scope,
        save.data(),
        &registry,
        &world.0,
        &boss_anchors,
        &reward_chests,
    );
}

/// Feed [`MountDied`](ambition_platformer2d_shared_tangle::body::MountDied)
/// directly into a boss rider's entity-local `External("mount_died")` phase
/// trigger. This is a body-to-phase fact, not script vocabulary. It runs before
/// [`update_boss_encounters`] so phase-derived music and edge events see the
/// change in the same frame.
pub fn notify_bosses_on_mount_death(
    mut mount_deaths: MessageReader<ambition_platformer2d_shared_tangle::body::MountDied>,
    mut riders: Query<&mut crate::BossEncounter, With<crate::BossConfig>>,
) {
    for ev in mount_deaths.read() {
        let Ok(mut encounter) = riders.get_mut(ev.rider) else {
            // A non-boss rider (a pirate) has no phase state to notify.
            continue;
        };
        if let Some(phase) = encounter.encounter.as_mut() {
            let _ = phase.notify_external("mount_died");
        }
    }
}

/// The adaptive-music track a boss plays in `phase`, from its authored spec.
/// `None` for `Dormant` / `Death` (no boss music — room music resumes).
fn phase_music_track(
    spec: &crate::BossEncounterSpec,
    phase: crate::BossEncounterPhase,
) -> Option<&str> {
    use crate::BossEncounterPhase as P;
    let track = match phase {
        P::Intro => &spec.music_intro,
        P::Phase1 | P::Transition => &spec.music_phase1,
        P::Phase2 | P::Stagger => &spec.music_phase2,
        P::Enrage => &spec.music_enrage,
        P::Dormant | P::Death => return None,
    };
    (!track.is_empty()).then_some(track.as_str())
}

/// Camera-shake amplitude (px) on a dramatic boss phase change. Capped to 14 by
/// [`CameraShakeState::kick`].
const BOSS_PHASE_SHAKE_PX: f32 = 11.0;

/// Consume authoritative same-frame [`BossPhaseChanged`](super::events::BossPhaseChanged)
/// edges and materialize their gameplay/presentation feedback. The edge comes
/// from the rollback-owned phase machine rather than being re-derived here.
pub fn boss_phase_transition_feedback(
    mut phase_changes: MessageReader<super::events::BossPhaseChanged>,
    mut sfx: ambition_sfx::SfxWriter,
    // An intent, not a write. The kick is applied after the confirmed-frame
    // boundary, so a phase change on a predicted frame that a correction
    // erases leaves no shake.
    mut shake: MessageWriter<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest>,
    // Boss geometry — the actor that emits the phase-transition shockwave.
    bosses: Query<
        (
            &ambition_platformer2d_shared_tangle::body::BodyKinematics,
            &ambition_combat::CenteredAabb,
        ),
        With<crate::BossConfig>,
    >,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
) {
    use crate::BossEncounterPhase as P;
    for change in phase_changes.read() {
        let entity = change.boss;
        let Ok((kin, aabb)) = bosses.get(entity) else {
            continue;
        };
        let phase = change.to;
        if matches!(phase, P::Transition | P::Phase2 | P::Enrage | P::Stagger) {
            shake.write(
                ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest {
                    amplitude_px: BOSS_PHASE_SHAKE_PX,
                },
            );
            sfx.write(ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_ROCK_HIT,
                pos: ae::Vec2::ZERO,
            });
            // The transition is a dodgeable gameplay beat, not only feel: the
            // boss emits a `DamageBox` effect through the same generic
            // `apply_effects` consumer the player's shockwave gauntlet uses.
            // It is resolved at the boss's position and side
            // (`HitSide::Boss`), so the shared `apply_hitbox_damage` lands it
            // on the player.
            effects.write(ambition_vfx::EffectRequest {
                owner: entity,
                effect: ambition_vfx::Effect::DamageBox(ambition_vfx::DamageBoxEffect {
                    center: aabb.center,
                    faction: ambition_vfx::HitSide::Boss,
                    half_extent: ae::Vec2::new(170.0, 80.0),
                    damage: 2,
                    knockback: 1.6,
                    lifetime_s: 0.30,
                    name: Some("Shockwave AOE"),
                }),
            });
            // "Scream lines": a sharp radial spark burst from the boss, so the
            // phase change is noticeable and not a silent state flip.
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
    //! The feedback fires from the announced edge, not from its own memory.
    use super::*;
    use crate::test_support::{test_boss_config, test_boss_status};
    use crate::BossEncounterPhase;
    use ambition_combat::{CenteredAabb, FeatureId};
    use ambition_platformer2d_shared_tangle::body::BodyKinematics;
    use ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest;

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

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<ambition_vfx::EffectRequest>();
        app.add_message::<ambition_vfx::vfx::VfxMessage>();
        app.add_message::<CameraShakeRequest>();
        app.add_message::<super::super::events::BossPhaseChanged>();
        app.add_systems(Update, boss_phase_transition_feedback);
        app
    }

    /// Announce a phase change the way `update_boss_encounters` does when the
    /// phase machine commits one.
    fn announce(app: &mut App, boss: Entity, from: BossEncounterPhase, to: BossEncounterPhase) {
        app.world_mut()
            .resource_mut::<Messages<super::super::events::BossPhaseChanged>>()
            .write(super::super::events::BossPhaseChanged { boss, from, to });
    }

    /// What the transition asked the world for this frame. The shockwave is
    /// the gameplay half, which makes correctness a rollback question.
    fn requested(app: &App) -> (usize, usize) {
        (
            app.world().resource::<Messages<CameraShakeRequest>>().len(),
            app.world()
                .resource::<Messages<ambition_vfx::EffectRequest>>()
                .len(),
        )
    }

    #[test]
    fn a_dramatic_transition_asks_for_a_shake_and_a_shockwave() {
        let mut app = test_app();
        let boss = spawn_boss(&mut app, BossEncounterPhase::Enrage);
        announce(
            &mut app,
            boss,
            BossEncounterPhase::Phase1,
            BossEncounterPhase::Enrage,
        );
        app.update();
        assert_eq!(
            requested(&app),
            (1, 1),
            "a dramatic phase change produced no shake and no shockwave"
        );
    }

    #[test]
    fn a_non_dramatic_transition_asks_for_nothing() {
        let mut app = test_app();
        let boss = spawn_boss(&mut app, BossEncounterPhase::Phase1);
        announce(
            &mut app,
            boss,
            BossEncounterPhase::Intro,
            BossEncounterPhase::Phase1,
        );
        app.update();
        assert_eq!(
            requested(&app),
            (0, 0),
            "Phase1 is not a dramatic transition and must be silent"
        );
    }

    /// A boss standing in a dramatic phase, with nothing announced, does
    /// nothing.
    ///
    /// Level versus edge: the system cannot see `Enrage`, only the
    /// announcement of entering it.
    #[test]
    fn a_boss_already_standing_in_a_dramatic_phase_fires_nothing() {
        let mut app = test_app();
        let _boss = spawn_boss(&mut app, BossEncounterPhase::Enrage);
        app.update();
        app.update();
        assert_eq!(
            requested(&app),
            (0, 0),
            "a boss that has been enraged for two frames re-fired its entry"
        );
    }

    /// A re-simulated transition still fires on the corrected timeline.
    ///
    /// Rollback case: a predicted frame enters `Enrage` and the shockwave
    /// spawns. The host rewinds: `BossEncounter` is rollback-registered and
    /// returns to `Phase1`, and the `DamageBox` is removed. A system with
    /// non-rollback memory (such as a `Local` map) would remember `Enrage`,
    /// see no change on the corrected pass, and produce nothing. The shockwave
    /// is a `DamageBox` the player must dodge, so this is a gameplay loss.
    ///
    /// The fixture reproduces the rewind: the same system instance (with any
    /// memory it has) sees the same transition announced twice, as a
    /// re-simulated frame does.
    #[test]
    fn a_resimulated_transition_still_fires_on_the_corrected_timeline() {
        let mut app = test_app();
        let boss = spawn_boss(&mut app, BossEncounterPhase::Enrage);

        // The predicted pass.
        announce(
            &mut app,
            boss,
            BossEncounterPhase::Phase1,
            BossEncounterPhase::Enrage,
        );
        app.update();
        assert_eq!(requested(&app), (1, 1), "the predicted pass fired");

        // The rewind: everything the abandoned pass produced is gone. These
        // channels are what presentation and the effect consumer would have
        // seen, so clearing them models the pass being taken back.
        app.world_mut()
            .resource_mut::<Messages<CameraShakeRequest>>()
            .clear();
        app.world_mut()
            .resource_mut::<Messages<ambition_vfx::EffectRequest>>()
            .clear();

        // The corrected pass re-runs the phase machine, which announces the
        // same change again because the corrected timeline crosses it.
        announce(
            &mut app,
            boss,
            BossEncounterPhase::Phase1,
            BossEncounterPhase::Enrage,
        );
        app.update();

        assert_eq!(
            requested(&app),
            (1, 1),
            "the re-simulated transition produced nothing. Under the old `Local` \
             diff this is exactly what happened: the map still held `Enrage` from \
             the pass that was thrown away, so the corrected timeline lost its \
             shockwave — a `DamageBox` the player was meant to dodge, deleted by \
             a network hiccup"
        );
    }
}

#[cfg(test)]
mod mount_death_bridge_tests {
    //! `MountDied` → the rider boss's `External("mount_died")` phase trigger.
    //! `notify_bosses_on_mount_death` is the first production caller of
    //! `PhaseTriggerCondition::External`.
    use super::*;
    use crate::test_support::{test_boss_config, test_boss_status_with};
    use crate::BossEncounter;
    use crate::{BossEncounterPhase, PhaseTrigger};
    use ambition_platformer2d_shared_tangle::body::MountDied;

    fn bridge_app() -> App {
        let mut app = App::new();
        app.add_message::<MountDied>();
        app.add_systems(Update, notify_bosses_on_mount_death);
        app
    }

    /// Spawn a boss carrying a `mount_died` external trigger from `Phase1`, at
    /// `Phase1`. Returns its entity.
    fn spawn_mounted_boss(app: &mut App) -> Entity {
        let config = test_boss_config("gnu_ton_rider", "GNU-ton", "gnu_ton_rider");
        let (status, health) = test_boss_status_with(
            100,
            BossEncounterPhase::Phase1,
            vec![PhaseTrigger::external(
                "mount_died",
                BossEncounterPhase::Phase1,
                BossEncounterPhase::Enrage,
                0.0,
            )],
        );
        app.world_mut().spawn((config, status, health)).id()
    }

    fn phase_of(app: &App, e: Entity) -> BossEncounterPhase {
        app.world()
            .entity(e)
            .get::<BossEncounter>()
            .unwrap()
            .encounter
            .as_ref()
            .unwrap()
            .phase
    }

    /// A `MountDied` naming the boss rider fires its `mount_died` trigger,
    /// flipping it into the authored on-foot phase.
    #[test]
    fn mount_death_flips_the_rider_boss_into_its_on_foot_phase() {
        let mut app = bridge_app();
        let boss = spawn_mounted_boss(&mut app);
        assert_eq!(phase_of(&app, boss), BossEncounterPhase::Phase1);

        app.world_mut().write_message(MountDied {
            mount: Entity::PLACEHOLDER,
            rider: boss,
        });
        app.update();

        assert_eq!(
            phase_of(&app, boss),
            BossEncounterPhase::Enrage,
            "the dismounted boss should advance to its authored on-foot phase",
        );
    }

    /// A `MountDied` for an unrelated entity leaves the boss's phase alone (no
    /// spurious external fire).
    #[test]
    fn mount_death_for_another_entity_does_not_move_the_boss() {
        let mut app = bridge_app();
        let boss = spawn_mounted_boss(&mut app);

        // A non-boss rider entity — the bridge's `riders.get_mut` misses it.
        let bystander = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(MountDied {
            mount: Entity::PLACEHOLDER,
            rider: bystander,
        });
        app.update();

        assert_eq!(
            phase_of(&app, boss),
            BossEncounterPhase::Phase1,
            "an unrelated mount death must not phase this boss",
        );
    }
}
