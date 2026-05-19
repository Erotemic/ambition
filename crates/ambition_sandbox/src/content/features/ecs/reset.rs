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
        ),
        With<FeatureSimEntity>,
    >,
    mut switches: Query<&mut SwitchOn, With<SwitchFeature>>,
    mut bosses: Query<&mut BossFeature, With<FeatureSimEntity>>,
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
    for (entity, mut feature, stand_timer) in &mut breakables {
        feature.breakable.state = ae::BreakableState::Intact;
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
    ) in &mut actors
    {
        match &mut *actor {
            ActorRuntime::Peaceful(npc) => {
                npc.pos = npc.spawn;
                aabb.center = npc.spawn;
                npc.vel = ae::Vec2::ZERO;
                npc.on_ground = false;
                npc.hostile = false;
                npc.strikes = 0;
                npc.hit_flash = 0.0;
            }
            ActorRuntime::Hostile(enemy) => {
                // Restore authored spawn state so morphed actors
                // (PirateOnShark → PirateRaider / BurningFlyingShark)
                // return as their original fused archetype with
                // matching size, gravity, choreography, and rider
                // health. Non-morphing enemies are reset to a clean
                // baseline by the same call.
                enemy.reset_to_spawn();
                aabb.center = enemy.pos;
                aabb.half_size = enemy.size * 0.5;
            }
        }
        sync_actor_components_from_runtime(
            &*actor,
            &mut *identity,
            &mut *disposition,
            &mut *health,
            &mut *combat,
            &mut *intent,
            &mut *cooldowns,
        );
    }
    for mut boss_feature in &mut bosses {
        let boss = &mut boss_feature.boss;
        boss.pos = boss.spawn;
        boss.alive = true;
        boss.health.reset();
        boss.pattern_timer = 0.0;
        boss.movement_timer = 0.0;
        boss.attack_windup_timer = 0.0;
        boss.attack_timer = 0.0;
        boss.attack_cooldown = 0.35;
        boss.hit_flash = 0.0;
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
