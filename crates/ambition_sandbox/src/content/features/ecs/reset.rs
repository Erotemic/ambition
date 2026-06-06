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
            &mut FeatureAabb,
            &mut ActorRuntime,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut ActorHealth,
            &mut ActorCombatState,
            &mut ActorIntent,
            &mut ActorCooldowns,
            Option<super::enemy_clusters::EnemyClusterQueryData>,
        ),
        // Bosses are reset by the disjoint `bosses` query below. Both this
        // query (via `EnemyClusterQueryData`) and the boss query take
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
            &mut crate::brain::Brain,
            &mut crate::brain::BossAttackState,
            &mut crate::brain::ActorControl,
        ),
        With<FeatureSimEntity>,
    >,
    mut hazards: Query<&mut HazardFeature, With<FeatureSimEntity>>,
    mut enemy_projectiles: ResMut<crate::enemy_projectile::EnemyProjectileState>,
    mut combat_slots: ResMut<crate::combat_slots::CombatSlotsRes>,
) {
    if reset_requests.read().next().is_none() {
        return;
    }
    // In-flight enemy volleys belong to the previous attempt; clear
    // them so the room reset doesn't leave hostile shots sailing
    // through the spawn point. Combat slot reservations are dropped
    // for the same reason — `update_ecs_actors` will rebuild them
    // from the freshly-respawned actor positions.
    enemy_projectiles.clear();
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
        feature.breakable.state = crate::interaction::BreakableState::Intact;
        feature.breakable.health.reset();
        if let Some(mut timer) = stand_timer {
            timer.0 = 0.0;
        }
        commands.entity(entity).remove::<RespawnTimer>();
    }
    for (
        mut aabb,
        mut actor,
        mut identity,
        mut disposition,
        mut health,
        mut combat,
        mut intent,
        mut cooldowns,
        mut clusters,
    ) in &mut actors
    {
        match &mut *actor {
            // NPCs reset in the sibling `reset_ecs_npc_actors` system —
            // their cluster query borrows the shared kinematics/surface
            // components the enemy query here also holds mutably.
            ActorRuntime::Npc => {}
            ActorRuntime::Enemy => {
                // Restore authored spawn state so morphed actors
                // (PirateOnShark → PirateRaider / BurningFlyingShark)
                // return as their original fused archetype with
                // matching size, gravity, mount/rider links, and
                // rider health. Non-morphing enemies are reset to a clean
                // baseline by the same call.
                let cq = clusters
                    .as_mut()
                    .expect("enemy entity carries cluster components");
                let mut em = cq.as_enemy_mut();
                em.reset_to_spawn();
                aabb.center = em.kin.pos;
                aabb.half_size = em.kin.size * 0.5;
                sync_actor_components_from_enemy(
                    &em,
                    &mut identity,
                    &mut disposition,
                    &mut health,
                    &mut combat,
                    &mut intent,
                    &mut cooldowns,
                );
            }
        }
    }
    for (mut feature, mut brain, mut attack_state, mut control) in &mut bosses {
        feature.kin.pos = feature.config.spawn;
        feature.status.alive = true;
        feature.kin.facing = 1.0;
        feature.status.health.reset();
        feature.status.hit_flash = 0.0;
        // Brain-owned state: zero the per-actor `BossPatternState`
        // (cursor / clocks / cycle phase / last_phase) and the
        // `BossAttackState` mirror (live telegraph + active profile
        // + remaining time). `ActorControl` is cleared too so a
        // stale `desired_vel` from the previous attempt doesn't
        // integrate on the post-reset frame.
        if let crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::BossPattern {
            state,
            ..
        }) = &mut *brain
        {
            *state = crate::brain::BossPatternState::default();
        }
        attack_state.clear();
        control.0 = crate::actor_control::ActorControlFrame::neutral();
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
}

/// Reset peaceful NPC actors to their authored spawn on a same-room
/// reset. Split from [`reset_ecs_room_features`] because the NPC cluster
/// query borrows the shared kinematics/surface components the enemy
/// reset query also holds mutably.
pub fn reset_ecs_npc_actors(
    mut reset_requests: MessageReader<ResetRoomFeaturesEvent>,
    mut npcs: Query<
        (
            &mut FeatureAabb,
            super::npc_clusters::NpcClusterQueryData,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut ActorHealth,
            &mut ActorCombatState,
            &mut ActorIntent,
            &mut ActorCooldowns,
        ),
        With<FeatureSimEntity>,
    >,
) {
    if reset_requests.read().next().is_none() {
        return;
    }
    for (
        mut aabb,
        mut clusters,
        mut identity,
        mut disposition,
        mut health,
        mut combat,
        mut intent,
        mut cooldowns,
    ) in &mut npcs
    {
        let mut npc = clusters.as_npc_mut();
        npc.reset_to_spawn();
        aabb.center = npc.kin.pos;
        aabb.half_size = npc.kin.size * 0.5;
        super::actors::sync_actor_components_from_npc(
            &npc,
            &mut identity,
            &mut disposition,
            &mut health,
            &mut combat,
            &mut intent,
            &mut cooldowns,
        );
    }
}

#[cfg(test)]
mod reset_tests {
    //! Same-room sandbox reset. A ResetRoomFeaturesEvent clears the
    //! transient feature markers so a room can be retried: collected
    //! pickups un-collect, opened chests un-open, broken breakables
    //! return to Intact. No event -> no change.
    use super::*;
    use crate::combat_slots::CombatSlotsRes;
    use crate::enemy_projectile::EnemyProjectileState;
    use crate::interaction::Breakable;
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

        app.world_mut().write_message(ResetRoomFeaturesEvent);
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
