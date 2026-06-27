//! Same-room sandbox-reset handling for ECS feature state.
//!
//! Listens for `ResetRoomFeaturesEvent` and clears collected pickups,
//! opened chests, broken breakables, dazed/morphed actors, defeated
//! bosses, hazard positions, and flipped switches so the player can
//! retry a room without having to leave and re-enter.

use super::*;

/// Reset ECS-owned static feature state after a same-room sandbox reset.
pub fn reset_ecs_room_features(
    mut commands: Commands,
    mut reset_requests: MessageReader<ResetRoomFeaturesEvent>,
    collected_pickups: Query<Entity, (With<FeatureSimEntity>, With<Collected>)>,
    opened_chests: Query<Entity, (With<FeatureSimEntity>, With<Opened>)>,
    post_boss_npcs: Query<Entity, With<crate::features::PostBossNpc>>,
    mut breakables: Query<
        (Entity, &mut BreakableFeature, Option<&mut StandTimer>),
        With<FeatureSimEntity>,
    >,
    mut actors: Query<
        (
            &mut CenteredAabb,
            &mut ActorIdentity,
            &ActorDisposition,
            &mut BodyHealth,
            &mut BodyCombat,
            &mut ActorIntent,
            &mut ActorCooldowns,
            &mut ActorAggression,
            Option<&ActorInteraction>,
            super::actor_clusters::ActorClusterQueryData,
        ),
        // Bosses are reset by the disjoint `bosses` query below. Both this
        // query (via `ActorClusterQueryData`) and the boss query take
        // `&mut BodyKinematics` — now the unified component — so exclude
        // bosses here so Bevy can prove the two queries never alias.
        (
            With<FeatureSimEntity>,
            Without<super::boss_clusters::BossConfig>,
        ),
    >,
    mut switches: Query<&mut SwitchOn, With<SwitchFeature>>,
    mut bosses: Query<
        (
            super::boss_clusters::BossClusterQueryData,
            &mut ambition_characters::brain::Brain,
            &mut ambition_characters::brain::BossAttackState,
            &mut ambition_characters::brain::ActorControl,
        ),
        With<FeatureSimEntity>,
    >,
    mut hazards: Query<&mut HazardFeature, With<FeatureSimEntity>>,
    // In-flight enemy projectiles are ECS entities now (Phase 3c-iii); despawn
    // them instead of clearing a Vec. `Entity`-only fetch, so no aliasing with
    // the actor/boss `&mut BodyKinematics` queries above.
    enemy_projectiles: Query<Entity, With<crate::enemy_projectile::EnemyProjectile>>,
    mut combat_slots: ResMut<crate::combat::slots::CombatSlotsRes>,
    // R5 encounter orchestration from the previous attempt: the encounter entity
    // (+ its finished `EncounterScript`), in-flight falling hazards, and the lure
    // override on a boss. `Entity`-only fetches → no aliasing with the queries above.
    encounter_entities: Query<Entity, With<crate::boss_encounter::EncounterDef>>,
    falling_hazards: Query<Entity, With<crate::boss_encounter::FallingHazard>>,
    commanded_bosses: Query<Entity, With<crate::boss_encounter::CommandedMove>>,
) {
    if reset_requests.read().next().is_none() {
        return;
    }
    // In-flight enemy volleys belong to the previous attempt; clear
    // them so the room reset doesn't leave hostile shots sailing
    // through the spawn point. Combat slot reservations are dropped
    // for the same reason — `update_ecs_actors` will rebuild them
    // from the freshly-respawned actor positions.
    for entity in &enemy_projectiles {
        commands.entity(entity).despawn();
    }
    combat_slots.0.clear_assignments();

    for entity in &collected_pickups {
        commands.entity(entity).remove::<Collected>();
    }
    for entity in &opened_chests {
        commands.entity(entity).remove::<Opened>();
    }
    for entity in &post_boss_npcs {
        commands.entity(entity).despawn();
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
        disposition,
        mut health,
        mut combat,
        mut intent,
        mut cooldowns,
        mut aggression,
        interaction,
        mut cq,
    ) in &mut actors
    {
        // Restore authored spawn state for EVERY actor through the unified
        // cluster: morphed actors (PirateOnShark → PirateRaider /
        // BurningFlyingShark) return as their fused archetype, non-morphing
        // enemies to a clean baseline, and peaceful NPCs to their spawn pose.
        let mut em = cq.as_actor_mut();
        em.reset_to_spawn();
        aabb.center = em.kin.pos;
        aabb.half_size = em.kin.size * 0.5;
        // Talkable actors (NPCs): clear the provoke accumulator + last attacker
        // so a struck-but-not-yet-hostile NPC starts the retried room fresh.
        if interaction.is_some() {
            aggression.strikes = 0;
            aggression.target = None;
        }
        sync_actor_components_from_cluster(
            &em,
            *disposition,
            &mut identity,
            &mut health,
            &mut combat,
            &mut intent,
            &mut cooldowns,
        );
    }
    for (mut feature, mut brain, mut attack_state, mut control) in &mut bosses {
        // Full revive (pos / facing / health / hit_flash + clear the entity-local
        // encounter so it re-seeds fresh next frame). One definition on `BossMut`
        // so a new `BossStatus` field can't desync this from the seed/save-skip
        // paths. (Why clearing `encounter` is load-bearing: see the helper docs.)
        feature.as_boss_mut().reset_to_spawn();
        // Brain-owned state: zero the per-actor `BossPatternState`
        // (cursor / clocks / cycle phase / last_phase) and the
        // `BossAttackState` mirror (live telegraph + active profile
        // + remaining time). `ActorControl` is cleared too so a
        // stale `desired_vel` from the previous attempt doesn't
        // integrate on the post-reset frame.
        if let ambition_characters::brain::Brain::StateMachine(ambition_characters::brain::StateMachineCfg::BossPattern {
            state,
            ..
        }) = &mut *brain
        {
            *state = ambition_characters::brain::BossPatternState::default();
        }
        attack_state.clear();
        control.0 = ambition_characters::actor::control::ActorControlFrame::neutral();
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
            .remove::<crate::boss_encounter::CommandedMove>();
    }
}

#[cfg(test)]
mod reset_tests {
    //! Same-room sandbox reset. A ResetRoomFeaturesEvent clears the
    //! transient feature markers so a room can be retried: collected
    //! pickups un-collect, opened chests un-open, broken breakables
    //! return to Intact. No event -> no change.
    use super::*;
    use crate::combat::slots::CombatSlotsRes;
    use crate::enemy_projectile::EnemyProjectileState;
    use ambition_interaction::Breakable;
    use bevy::prelude::{App, Entity, Update};

    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(EnemyProjectileState::default());
        app.insert_resource(CombatSlotsRes::default());
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
