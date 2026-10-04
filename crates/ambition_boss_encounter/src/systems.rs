//! Boss-encounter Bevy systems: the per-frame driver.
//!
//! `populate_boss_encounter_registry` (startup) loads the read-only profile
//! catalog. `update_boss_encounters` (per sim tick) seeds and wakes bosses in
//! every live room, ticks each phase machine, publishes events, mirrors phase
//! HP/phase onto the boss ECS clusters, manages the adaptive-music request, and
//! syncs reward chests. `boss_phase_transition_feedback` consumes the
//! `BossPhaseChanged` edge that driver announces and fires camera shake, a
//! `DamageBox` shockwave and scream VFX on dramatic transitions.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_cutscene::CutsceneTriggerQueue;
use ambition_persistence::quest::QuestRegistry;

use super::{
    default_boss_profiles, events::publish_events, BossCatalog, BossEncounterRegistry,
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

/// Drive every boss's entity-local phase mechanism: seed from the encounter
/// the boss was built with (`BossConfig::seed`), wake, tick the `ActorPhaseState`, resolve death (save + quest), keep
/// the adaptive-music request live, and sync reward chests.
/// The body's `BodyHealth` and `BossEncounter.encounter` are the source of truth.
pub fn update_boss_encounters(
    mut commands: Commands,
    world_time: Res<ambition_time::WorldTime>,
    mut banner: ResMut<ambition_combat::GameplayBanner>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    mut music_request: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldMut<
        ambition_encounter::EncounterMusicRequest,
    >,
    mut quests: ResMut<QuestRegistry>,
    mut cutscene_queue: ResMut<CutsceneTriggerQueue>,
    // The geometry of each boss's own live room, where its reward chest
    // settles. A sole-room read here stopped every boss, in every room, while
    // two rooms were live (OW1 cut 7e).
    geometry: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
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
    // The defeats since the last checkpoint, which a replay of their room
    // retracts (BOSS-REPLAY-RETRACTION), and the live room each fell in.
    // And the driven bodies, to say who won a defeat.
    (mut since_checkpoint, rooms, drivers): (
        ResMut<crate::retraction::BossDefeatsSinceCheckpoint>,
        ambition_platformer2d_world::rooms::LiveRoomSpecs,
        Query<(Entity, &ambition_characters::control::DrivingParticipant)>,
    ),
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
            // The parent the boss's mints name.
            Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
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
    // collected per boss and grouped by the boss's live room. Anchors are
    // (placement_id, archetype_id, spawn): "cleared" and rewards are keyed by
    // placement. The music is still one track for the session: a view per
    // player is P5.
    // The active fight's track of each live room: a boss claims the music of
    // the room it fights in (customer 2).
    let mut active_music_tracks: std::collections::BTreeMap<
        Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
        String,
    > = std::collections::BTreeMap::new();
    let mut boss_anchors: std::collections::BTreeMap<
        ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
        Vec<crate::BossRewardAnchor>,
    > = std::collections::BTreeMap::new();

    for (boss_entity, _feature_id, mut feature, mut health, mut combat, overrides, boss_sim_id) in &mut bosses {
        let archetype_id = feature.config.behavior.id.clone();
        let runtime_id = feature.config.id.clone();

        // The encounter this boss was built with (`BossConfig::seed`, from the
        // generation's catalog).
        let (spec, reward) = (
            feature.config.seed.encounter.clone(),
            feature.config.seed.reward.clone(),
        );

        // Seed entity-local state once from the profile (phase triggers, HP),
        // so two of the same boss have independent state. The per-spawn
        // `BossOverrides` (hp / phase triggers) are applied here, so the
        // profile cannot overwrite them. The body size is not seeded here:
        // construction resolved `kin.size`.
        //
        // ⛔ NOR IS THE BEHAVIOUR (2026-10-01). Construction resolved it from
        // the catalog the session's generation froze, and captured the brain's
        // pattern and movement from that same value. Seeding it again from
        // this system's App-global catalog could only repeat that value, or,
        // when the App holds another generation, give the boss a config its
        // own brain does not run. MEASURED: a reload whose session froze N
        // showed N+1 here on the activation frame.
        if feature.status.encounter.is_none() {
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
                // A defeat after the last checkpoint: a replay of this room
                // retracts it (Q56).
                if let Some(definition) = rooms.definition_of(boss_entity) {
                    let room = rooms.live().of(boss_entity);
                    let mut present: Vec<_> = drivers
                        .iter()
                        .filter(|(body, _)| room.is_some() && rooms.live().of(*body) == room)
                        .map(|(_, driver)| driver.0)
                        .collect();
                    present.sort_unstable();
                    present.dedup();
                    since_checkpoint.record(
                        runtime_id.clone(),
                        crate::retraction::BossDefeatSinceCheckpoint {
                            room,
                            definition: rooms.rooms().spec(definition).id.clone(),
                            boss: boss_sim_id.cloned(),
                            present,
                        },
                    );
                }
                // Caused by the placement, so a replay that retracts this
                // defeat retracts the quest step it advanced.
                quests.push_event_caused_by(
                    ambition_persistence::quest::QuestAdvanceEvent::BossDefeated(
                        archetype_id.clone(),
                    ),
                    runtime_id.clone(),
                );
            }
        }

        // Collect the active-fight music and the reward anchor
        // (placement_id, archetype_id, spawn): the reward sync keys the chest
        // and looted flag by placement and resolves the DropChest reward via
        // the archetype profile.
        if let Some(track) = phase_music_track(&spec, phase) {
            if !track.is_empty() {
                active_music_tracks
                    .entry(rooms.live().of(boss_entity))
                    .or_insert_with(|| track.to_string());
            }
        }
        // A boss in no live room drops nothing: no room is simulated there.
        if let Some(room) = geometry.room_of(boss_entity) {
            boss_anchors.entry(room).or_default().push(crate::BossRewardAnchor {
                placement_id: runtime_id.clone(),
                spawn: feature.config.spawn,
                reward: reward.clone(),
            });
        }
    }

    // Music-request lifetime: keep each room's active boss track up; clear it
    // in each room where no boss is in an active-fight phase (defeated, or the player left the
    // room), so room music resumes. Guarded by
    // `boss_music_plays_during_the_fight` and
    // `defeated_boss_is_recorded_cleared_drops_reward_and_clears_music`.
    //
    // Release only this system's own claim. It has no run condition, so the
    // "no boss is fighting" arm runs every frame of every game; clearing the
    // whole tier would silence every other music claimant.
    music_request.release_priority_where(BOSS_MUSIC_OWNER, |room| {
        !active_music_tracks.contains_key(&room)
    });
    for (room, track) in active_music_tracks {
        music_request.claim_priority(room, BOSS_MUSIC_OWNER, track);
    }

    // Each live room's chests, in that room and on its floor.
    for (room, anchors) in &boss_anchors {
        let Some(world) = geometry.in_room(*room) else {
            continue;
        };
        crate::sync_boss_reward_chests_ecs(
            &mut commands,
            session_scope.in_room(Some(*room)),
            save.data(),
            &world.0,
            anchors,
            &reward_chests,
        );
    }
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
    mut commands: Commands,
    mut phase_changes: MessageReader<super::events::BossPhaseChanged>,
    mut sfx: ambition_sfx::SfxWriter,
    // An intent, not a write. The kick is applied after the confirmed-frame
    // boundary, so a phase change on a predicted frame that a correction
    // erases leaves no shake.
    mut shake: MessageWriter<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest>,
    // Boss geometry — the actor that emits the phase-transition shockwave.
    bosses: Query<
        (
            &ambition_platformer2d_core::BodyKinematics,
            &ambition_combat::CenteredAabb,
        ),
        With<crate::BossConfig>,
    >,
    mut vfx: ambition_vfx::vfx::VfxWriter,
    // The burst is drawn in the boss's own live room.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
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
            // boss puts a damage box at its centre, on its side
            // (`HitSide::Boss`), so the shared `apply_hitbox_damage` lands it
            // on the player.
            //
            // ⛔ SPAWNED HERE, NOT REQUESTED. This system runs in Progression,
            // AFTER the combat phase's effect executor, so an `EffectRequest`
            // written here was only executed on the NEXT tick. A rollback that
            // restored the snapshot between the two ticks cleared the waiting
            // request (a message buffer is cleared on load), and the replay had
            // no shockwave: a GGRS sync test of an HP-triggered phase change
            // mismatched on that frame (2026-10-01). The box is the same
            // `spawn_damage_box` the executor would have called, made in the
            // tick that decides it, so the snapshot of that tick holds it.
            ambition_combat::strike::spawn_damage_box(
                &mut commands,
                entity,
                ambition_vfx::HitSide::Boss,
                aabb.center,
                ambition_combat::strike::DamageBox {
                    half_extent: ae::Vec2::new(170.0, 80.0),
                    shape: None,
                    damage: 2,
                    knockback: 1.6,
                    lifetime_s: 0.30,
                    name: Some("Shockwave AOE"),
                },
            );
            // "Scream lines": a sharp radial spark burst from the boss, so the
            // phase change is noticeable and not a silent state flip.
            vfx.for_room(rooms.of(entity)).write(ambition_vfx::vfx::VfxMessage::Burst {
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
    use ambition_platformer2d_core::BodyKinematics;
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
        app.add_message::<ambition_vfx::vfx::VfxInRoom>();
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

    /// What the transition asked the world for this frame: the shake it
    /// requested and the shockwave boxes standing. The shockwave is the
    /// gameplay half, which makes correctness a rollback question.
    fn requested(app: &mut App) -> (usize, usize) {
        let shakes = app.world().resource::<Messages<CameraShakeRequest>>().len();
        let boxes = app
            .world_mut()
            .query::<&ambition_combat::strike::Hitbox>()
            .iter(app.world())
            .count();
        (shakes, boxes)
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
            requested(&mut app),
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
            requested(&mut app),
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
            requested(&mut app),
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
        assert_eq!(requested(&mut app), (1, 1), "the predicted pass fired");

        // The rewind: everything the abandoned pass produced is gone. The
        // shake channel is what presentation would have seen, and the
        // shockwave box is a rollback entity the restore removes.
        app.world_mut()
            .resource_mut::<Messages<CameraShakeRequest>>()
            .clear();
        let boxes: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<ambition_combat::strike::Hitbox>>()
            .iter(app.world())
            .collect();
        for entity in boxes {
            app.world_mut().despawn(entity);
        }

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
            requested(&mut app),
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
