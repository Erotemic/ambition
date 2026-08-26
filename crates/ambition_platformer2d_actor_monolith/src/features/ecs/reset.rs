//! Same-room sandbox-reset handling for ECS feature state.
//!
//! Listens for `ResetRoomFeaturesEvent` and clears collected pickups,
//! opened chests, broken breakables, dazed/morphed actors, defeated
//! bosses, hazard positions, and flipped switches so the player can
//! retry a room without having to leave and re-enter.

use super::*;
use ambition_combat::events::{ResetRoomFeaturesEvent};

/// Spawned by THIS attempt at the room, and cleared when the attempt is.
///
/// re-scoping the drop to the ROOM would be the wrong fix. A weapon you
/// drop in one room and find again when you walk back is intended behaviour, and
/// room scope deletes it on an ordinary transition. The two questions are
/// genuinely different — *does this survive leaving the room* and *does this
/// survive REPLAYING it* — and one scope cannot answer both. So the attempt is
/// named explicitly rather than inferred from a lifetime that means something
/// else.
///
/// it marks what the ATTEMPT produced, not everything spawned at runtime.
/// A summon a participant is still commanding or an item a body threw can be
/// somebody's durable live state; loot on the ground is the residue of a fight
/// that is about to be un-fought. In-flight projectiles are different: every
/// shot belongs to the combat timeline being reset, so the reset clears all
/// `LiveProjectile` occurrences explicitly rather than encoding their producer
/// category into this marker.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpawnedThisAttempt;

/// Reset ECS-owned static feature state after a same-room sandbox reset.
pub fn reset_ecs_room_features(
    mut commands: Commands,
    mut reset_requests: MessageReader<ResetRoomFeaturesEvent>,
    collected_pickups: Query<Entity, (With<FeatureSimEntity>, With<Collected>)>,
    opened_chests: Query<Entity, (With<FeatureSimEntity>, With<Opened>)>,
    // ONE query because the two despawn identically, and because this system is
    // at Bevy's 16-parameter ceiling — a post-boss NPC and the loot an attempt
    // dropped are both "spawned by the run, cleared with the run".
    run_spawned: Query<
        Entity,
        bevy::prelude::Or<(With<crate::features::PostBossNpc>, With<SpawnedThisAttempt>)>,
    >,
    mut breakables: Query<
        (Entity, &mut BreakableFeature, Option<&mut StandTimer>),
        With<FeatureSimEntity>,
    >,
    mut actors: Query<
        (
            &mut CenteredAabb,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut ActorAggression,
            Option<&ActorInteraction>,
            &mut ambition_platformer2d_core::movement::MotionModel,
            super::actor_clusters::ActorClusterQueryData,
        ),
        // Bosses are reset by the disjoint `bosses` query below. Both this
        // query (via `ActorClusterQueryData`) and the boss query take
        // `&mut BodyKinematics` — now the unified component — so exclude
        // bosses here so Bevy can prove the two queries never alias.
        (
            With<FeatureSimEntity>,
            Without<ambition_boss_encounter::BossConfig>,
        ),
    >,
    // Content pose PINS. `ActorAnimOverride` is an engine-provided override slot
    // a content state machine writes (a shelled enemy's withdraw cycle), and it
    // is a plain component — so it survived every reset, and a room that came
    // back "fresh" came back with its enemies still wearing whatever pose the
    // last attempt left them in. The engine owns the slot, so the engine clears
    // it: after a reset the shared picker chooses again, which is what a reset
    // MEANS. `Entity`-only fetch, so no aliasing with the actor query above.
    pinned_poses: Query<Entity, With<ambition_sprite_sheet::character::ActorAnimOverride>>,
    mut switches: Query<&mut SwitchOn, With<SwitchFeature>>,
    mut bosses: Query<
        (
            super::actor_clusters::ActorClusterQueryData,
            &mut ambition_boss_encounter::BossConfig,
            &mut ambition_boss_encounter::BossEncounter,
            &mut ambition_platformer2d_core::movement::MotionModel,
            &mut ambition_characters::actor::BodyCombat,
            &mut ambition_characters::brain::Brain,
            &mut ambition_characters::brain::BossAttackState,
            &mut ambition_characters::control::ActorControl,
            &mut ambition_boss_encounter::sprites::BossAnimFrame,
        ),
        With<FeatureSimEntity>,
    >,
    mut hazards: Query<&mut HazardFeature, With<FeatureSimEntity>>,
    // Every in-flight projectile belongs to the combat timeline being reset.
    // `Entity`-only fetch, so no aliasing with the actor/boss `&mut BodyKinematics`
    // queries above. Producer/presentation family is deliberately irrelevant.
    live_projectiles: Query<Entity, With<ambition_projectiles::LiveProjectile>>,
    // R5 encounter orchestration from the previous attempt: the encounter entity
    // (+ its finished `EncounterScript`), in-flight falling hazards, and the lure
    // override on a boss. `Entity`-only fetches → no aliasing with the queries above.
    encounter_entities: Query<Entity, With<ambition_boss_encounter::EncounterDef>>,
    falling_hazards: Query<Entity, With<ambition_boss_encounter::FallingHazard>>,
    commanded_bosses: Query<Entity, With<ambition_boss_encounter::CommandedMove>>,
) {
    let reasons: Vec<_> = reset_requests
        .read()
        .map(|request| request.reason)
        .collect();
    if reasons.is_empty() {
        return;
    }
    bevy::log::info!(
        target: "ambition_platformer2d::room_reset",
        "room features reset: reasons={reasons:?}",
    );
    ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
        "room-reset reasons={reasons:?}"
    ));
    // In-flight shots belong to the previous attempt; clear every live
    // projectile so the reset cannot leave an old combat event sailing through
    // the spawn point. Combat slot reservations are dropped
    // for the same reason — `update_ecs_actors` will rebuild them
    // from the freshly-respawned actor positions.
    for entity in &live_projectiles {
        commands.entity(entity).despawn();
    }

    for entity in &collected_pickups {
        commands.entity(entity).remove::<Collected>();
    }
    for entity in &opened_chests {
        commands.entity(entity).remove::<Opened>();
    }
    // Everything the RUN spawned: post-boss NPCs, and the loot an attempt
    // dropped. Same rule the in-flight volleys above already follow.
    for entity in &run_spawned {
        commands.entity(entity).despawn();
    }
    for entity in &pinned_poses {
        commands
            .entity(entity)
            .remove::<ambition_sprite_sheet::character::ActorAnimOverride>();
    }
    for (entity, mut feature, stand_timer) in &mut breakables {
        feature.breakable.state = ambition_interaction::BreakableState::Intact;
        feature.breakable.health.reset();
        if let Some(mut timer) = stand_timer {
            timer.0 = 0.0;
        }
        commands.entity(entity).remove::<RespawnTimer>();
    }
    for (
        mut aabb,
        mut identity,
        mut disposition,
        mut aggression,
        interaction,
        mut motion_model,
        mut cq,
    ) in &mut actors
    {
        // Restore authored spawn state for EVERY actor through the unified
        // cluster: morphed actors (PirateOnShark → PirateRaider /
        // BurningFlyingShark) return as their fused archetype, non-morphing
        // enemies to a clean baseline, and peaceful NPCs to their spawn pose.
        let mut em = cq.as_actor_mut();
        em.reset_to_spawn(&mut motion_model);
        aabb.center = em.kin.pos;
        aabb.half_size = em.kin.size * 0.5;
        // Restore the SPAWN disposition (it is derived from targeting at runtime, so
        // a stood-down fighter would otherwise stay peaceful after the reset and
        // never re-engage): a talkable NPC resets to Peaceful (a provoked one calms
        // back down), a combatant (enemy / duel fighter) resets to Hostile so a duel
        // winner re-fights the freshly-revived loser. `reset_to_spawn` already
        // restored HP + position; this restores the fight state too.
        *disposition = if interaction.is_some() {
            ActorDisposition::Peaceful
        } else {
            ActorDisposition::Hostile
        };
        // Talkable actors (NPCs): clear the provoke accumulator + last attacker
        // so a struck-but-not-yet-hostile NPC starts the retried room fresh.
        if interaction.is_some() {
            aggression.strikes = 0;
            aggression.target = None;
        }
        sync_actor_components_from_cluster(&em, &mut identity);
    }
    for (
        mut cq,
        config,
        mut status,
        mut motion_model,
        mut combat,
        mut brain,
        mut attack_state,
        mut control,
        mut anim_frame,
    ) in &mut bosses
    {
        // Full revive: the pose snap is a discrete TRANSIT (ADR 0024 authority
        // model) — arrive at rest with departure contacts and any attachment
        // reconciled — plus liveness/HP restore and clearing the entity-local
        // encounter so it re-seeds fresh next frame (keeping last attempt's
        // `Death` phase would re-kill the boss the instant it revives; pinned
        // by `boss_revives_after_a_room_reset`).
        {
            let mut em = cq.as_actor_mut();
            let spawn = config.spawn;
            // A revive is a RESTART. `transit_body` keeps maneuver state on
            // purpose (right for a blink, wrong for coming back from the dead)
            // and announces nothing, so `ae::BodyRestarted` never fired for a
            // boss revive and no provider heard about it.
            //
            // safe to say the stronger thing only since `ActorBody::from_kit` records the
            // identity size: `reset_body_clusters` restores `base_size`, which defaulted to the
            // PLAYER's size for every boss in the game.
            ae::reset_body_clusters(
                &mut motion_model,
                &mut em.clusters_mut(),
                spawn,
                ae::DEFAULT_TUNING.air_jumps,
            );
            em.kin.facing = 1.0;
            em.health.reset();
        }
        // AC3.1.A: liveness is `BodyHealth`'s, and `em.health.reset()` above is
        // what restores it. There is no second answer to set.
        combat.reset();
        status.encounter = None;
        status.encounter_phase = ambition_boss_encounter::BossEncounterPhase::Dormant;
        // Reset the durable brain cursor/clocks and clear both transient
        // control intent and the move-derived `BossAttackState` read-model.
        // A stale `desired_vel` or projected attack from the previous attempt
        // must not survive into the post-reset frame.
        if let ambition_characters::brain::Brain::StateMachine(
            ambition_characters::brain::StateMachineCfg::BossPattern { state, .. },
        ) = &mut *brain
        {
            *state = ambition_characters::brain::BossPatternState::default();
        }
        attack_state.clear();
        control.0 = ambition_characters::actor::control::ActorControlFrame::neutral();
        anim_frame.reset();
    }
    for mut hazard_feature in &mut hazards {
        let spawn = hazard_feature.spawn;
        hazard_feature.hazard.pos = spawn;
        if let Some(motion_start) = hazard_feature
            .hazard
            .motion
            .as_ref()
            .and_then(PathMotion::start_pos)
        {
            hazard_feature.hazard.pos = motion_start;
        }
    }
    for mut switch_on in &mut switches {
        switch_on.0 = false;
    }
    // Retire the previous attempt's encounter orchestration so the replay
    // re-forms it fresh: the encounter entity (its `EncounterScript` cursor is
    // already past its beats) is re-created by `sync_boss_encounter_entities` +
    // `setup_cut_rope_encounter` once the boss re-wakes; any in-flight falling
    // hazard + the lure override are dropped.
    for entity in &encounter_entities {
        commands.entity(entity).despawn();
    }
    for entity in &falling_hazards {
        commands.entity(entity).despawn();
    }
    for entity in &commanded_bosses {
        commands
            .entity(entity)
            .remove::<ambition_boss_encounter::CommandedMove>();
    }
}

#[cfg(test)]
mod reset_tests {
    //! Same-room sandbox reset. A ResetRoomFeaturesEvent clears the
    //! transient feature markers so a room can be retried: collected
    //! pickups un-collect, opened chests un-open, broken breakables
    //! return to Intact. No event -> no change.
    use super::*;
    use ambition_interaction::Breakable;
    use bevy::prelude::{App, Entity, Update};

    fn app() -> App {
        let mut app = App::new();
        app.add_message::<ResetRoomFeaturesEvent>();
        app.add_systems(Update, reset_ecs_room_features);
        app
    }

    fn broken_breakable(app: &mut App) -> Entity {
        let mut b = Breakable::new("brk", 1);
        b.apply_damage(5); // health 1 -> Broken
        app.world_mut()
            .spawn((FeatureSimEntity, BreakableFeature::new(b)))
            .id()
    }

    /// The reset already despawns in-flight enemy volleys, with the reason
    /// stated in its own comment — they "belong to the previous attempt". A coin
    /// dropped by an enemy killed in that attempt is the same class of thing.
    #[test]
    fn a_drop_from_the_previous_attempt_does_not_survive_the_reset() {
        use crate::items::pickup::GroundItem;
        let mut app = app();
        let drop = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                SpawnedThisAttempt,
                GroundItem {
                    spec: Default::default(),
                    pos: ambition_platformer2d_core::Vec2::new(100.0, 100.0),
                    vel: ambition_platformer2d_core::Vec2::ZERO,
                    half_extent: ambition_platformer2d_core::Vec2::splat(8.0),
                },
            ))
            .id();

        app.world_mut()
            .write_message(ResetRoomFeaturesEvent::default());
        app.update();

        let survived = app.world().get_entity(drop).is_ok();
        assert!(
            !survived,
            "a coin dropped in the previous attempt is still lying in the room \
             after the reset — the same class as the in-flight volleys this \
             reset already clears",
        );
    }

    #[test]
    fn reset_clears_every_live_projectile_regardless_of_presentation_family() {
        let mut app = app();
        let open = app.world_mut().spawn(ambition_projectiles::LiveProjectile).id();
        let named = app
            .world_mut()
            .spawn((ambition_projectiles::LiveProjectile, ambition_projectiles::ProjectileKind::Fireball))
            .id();

        app.world_mut()
            .write_message(ResetRoomFeaturesEvent::default());
        app.update();

        assert!(app.world().get_entity(open).is_err(), "open shot survived reset");
        assert!(app.world().get_entity(named).is_err(), "named shot survived reset");
    }

    #[test]
    fn reset_clears_room_feature_markers() {
        let mut app = app();
        let chest = app.world_mut().spawn((FeatureSimEntity, Opened)).id();
        let pickup = app.world_mut().spawn((FeatureSimEntity, Collected)).id();
        let brk = broken_breakable(&mut app);

        app.world_mut()
            .write_message(ResetRoomFeaturesEvent::default());
        app.update();

        assert!(
            app.world().get::<Opened>(chest).is_none(),
            "reset un-opens chests"
        );
        assert!(
            app.world().get::<Collected>(pickup).is_none(),
            "reset un-collects pickups"
        );
        assert!(
            !app.world().get::<BreakableFeature>(brk).unwrap().broken(),
            "reset restores a broken breakable to Intact"
        );
    }

    #[test]
    fn no_event_leaves_state_untouched() {
        let mut app = app();
        let chest = app.world_mut().spawn((FeatureSimEntity, Opened)).id();
        app.update(); // no ResetRoomFeaturesEvent written
        assert!(
            app.world().get::<Opened>(chest).is_some(),
            "without the reset event the markers stay"
        );
    }
}
