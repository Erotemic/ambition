//! ECS-native feature runtime systems.
//!
//! This is the Phase 3/4/5 strangler path for simple feature families. Static
//! authored pickups, chests, and breakables are spawned as Bevy entities at room
//! load and updated by the systems in this module. `FeatureRuntime` remains the
//! compatibility shell for hazards/enemies/bosses/NPCs/switches and for a few
//! dynamic reward-chest paths that still originate in runtime-owned encounter
//! code.

use super::*;
use crate::audio::SfxMessage;
use crate::fx::{ParticleKind, VfxMessage};
use crate::physics::{DebrisBurstMessage, PhysicsDebrisCue};
use crate::rendering::RoomVisual;
use bevy::prelude::{Commands, Component, Entity, MessageWriter, Query, ResMut, Resource, Time, With};

/// Marker for simulation-side feature entities spawned from the active room.
/// They are deliberately separate from presentation `FeatureVisual` sprites;
/// visible builds keep using the existing visual entities and look up live ECS
/// state by `FeatureId`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeatureSimEntity;

/// Per-frame damage/pogo work that still originates inside the legacy
/// `sandbox_update` loop. Systems after `sandbox_update` drain this into ECS
/// feature components.
#[derive(Resource, Default, Debug)]
pub struct FeatureEcsQueues {
    pub damage_events: Vec<DamageEvent>,
    pub pogo_bounces: Vec<(ae::Aabb, i32)>,
    pub reset_room_features: bool,
}

/// Collision contribution from ECS-owned breakables. Rebuilt before the main
/// sandbox tick and consumed by `world_with_sandbox_solids` anywhere the engine
/// needs the augmented collision world.
#[derive(Resource, Default, Clone, Debug)]
pub struct FeatureEcsWorldOverlay {
    pub blocks: Vec<ae::Block>,
}

/// Spawn ECS-native feature entities for every static feature object in a room.
pub fn spawn_room_feature_entities(commands: &mut Commands, world: &ae::World) {
    for object in &world.objects {
        spawn_room_feature_entity(commands, object);
    }
}

fn spawn_room_feature_entity(commands: &mut Commands, object: &ae::RoomObject) {
    let feature_aabb = FeatureAabb::from_aabb(object.aabb);
    match &object.kind {
        ae::RoomObjectKind::Pickup(pickup) => {
            commands.spawn((
                Name::new(format!("Feature pickup: {}", object.name)),
                FeatureSimEntity,
                RoomVisual,
                FeatureId::new(object.id.clone()),
                FeatureName::new(object.name.clone()),
                feature_aabb,
                PickupFeature::new(pickup.clone()),
            ));
        }
        ae::RoomObjectKind::Chest(chest) => {
            commands.spawn((
                Name::new(format!("Feature chest: {}", object.name)),
                FeatureSimEntity,
                RoomVisual,
                FeatureId::new(object.id.clone()),
                FeatureName::new(object.name.clone()),
                feature_aabb,
                ChestFeature::new(chest.clone()),
            ));
        }
        ae::RoomObjectKind::Breakable(breakable) => {
            let mut entity = commands.spawn((
                Name::new(format!("Feature breakable: {}", object.name)),
                FeatureSimEntity,
                RoomVisual,
                FeatureId::new(object.id.clone()),
                FeatureName::new(object.name.clone()),
                feature_aabb,
                BreakableFeature::new(breakable.clone()),
                StandTimer(0.0),
            ));
            if breakable.collision.blocks_movement() {
                entity.insert(SandboxSolidContributor);
            }
            if breakable.pogo_refresh || (breakable.collision.blocks_movement() && breakable.trigger.allows_stand()) {
                entity.insert(PogoTargetContributor);
            }
        }
        _ => {}
    }
}


/// Reset ECS-owned static feature state after a same-room sandbox reset.
pub fn reset_ecs_room_features(
    mut commands: Commands,
    mut queues: ResMut<FeatureEcsQueues>,
    collected_pickups: Query<Entity, (With<FeatureSimEntity>, With<Collected>)>,
    opened_chests: Query<Entity, (With<FeatureSimEntity>, With<Opened>)>,
    mut breakables: Query<(Entity, &mut BreakableFeature, Option<&mut StandTimer>), With<FeatureSimEntity>>,
) {
    if !queues.reset_room_features {
        return;
    }
    queues.reset_room_features = false;
    queues.damage_events.clear();
    queues.pogo_bounces.clear();

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
}

/// Rebuild the transient collision blocks contributed by ECS-owned breakables.
pub fn rebuild_feature_ecs_world_overlay(
    mut overlay: ResMut<FeatureEcsWorldOverlay>,
    breakables: Query<(&FeatureId, &FeatureName, &FeatureAabb, &BreakableFeature), With<FeatureSimEntity>>,
) {
    overlay.blocks.clear();
    for (id, name, aabb, feature) in &breakables {
        if feature.broken() {
            continue;
        }
        if feature.breakable.pogo_refresh {
            overlay.blocks.push(ae::Block {
                name: format!("ecs-breakable-pogo {}", name.0.as_str()),
                aabb: aabb.aabb(),
                kind: ae::BlockKind::PogoOrb,
            });
            continue;
        }
        let kind = match feature.breakable.collision {
            ae::BreakableCollision::None => continue,
            ae::BreakableCollision::Solid => ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Hard,
            },
            ae::BreakableCollision::OneWayUp => ae::BlockKind::OneWay,
        };
        overlay.blocks.push(ae::Block {
            name: format!("ecs-breakable {}", name.0.as_str()),
            aabb: aabb.aabb(),
            kind,
        });
        if feature.breakable.collision.blocks_movement() && feature.breakable.trigger.allows_stand() {
            overlay.blocks.push(ae::Block {
                name: format!("ecs-breakable-pogo-target {}", id.as_str()),
                aabb: aabb.aabb(),
                kind: ae::BlockKind::PogoOrb,
            });
        }
    }
}

/// Collect ECS-owned pickups after the player simulation has advanced.
pub fn collect_ecs_pickups(
    mut commands: Commands,
    mut runtime: ResMut<crate::SandboxRuntime>,
    pickups: Query<(Entity, &FeatureName, &FeatureAabb, &PickupFeature, Option<&Collected>), With<FeatureSimEntity>>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    let player_body = runtime.player.aabb();
    for (entity, name, aabb, pickup, collected) in &pickups {
        if collected.is_some() || !aabb.aabb().strict_intersects(player_body) {
            continue;
        }
        commands.entity(entity).insert(Collected);
        runtime.features.banner = format!("picked up {}", name.0.as_str());
        runtime.features.banner_timer = 2.6;
        if let ae::PickupKind::Health { amount } = &pickup.pickup.kind {
            runtime.player_health.heal(*amount);
        }
        let pos = aabb.center;
        vfx.write(VfxMessage::Burst {
            pos,
            count: 16,
            speed: 230.0,
            color: [0.84, 0.95, 1.0, 0.82],
            kind: ParticleKind::Spark,
        });
        let id = match &pickup.pickup.kind {
            ae::PickupKind::Health { .. } => ambition_sfx::ids::WORLD_HEALTH_COLLECT,
            ae::PickupKind::Currency { .. } => ambition_sfx::ids::WORLD_COIN_PICKUP,
            _ => ambition_sfx::ids::WORLD_PICKUP_GENERIC,
        };
        sfx.write(SfxMessage::Play { id, pos });
    }
}

/// Open ECS-owned static chests from the same interaction buffer used by doors
/// and legacy NPCs/switches.
pub fn open_ecs_chests(
    mut commands: Commands,
    mut runtime: ResMut<crate::SandboxRuntime>,
    chests: Query<(Entity, &FeatureId, &FeatureName, &FeatureAabb, Option<&Opened>), (With<FeatureSimEntity>, With<ChestFeature>)>,
    mut gameplay_effects: MessageWriter<GameplayEffect>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    if runtime.interact_buffer_timer <= 0.0 {
        return;
    }
    let player_body = runtime.player.aabb();
    for (entity, id, name, aabb, opened) in &chests {
        if opened.is_some() || !aabb.aabb().strict_intersects(player_body) {
            continue;
        }
        commands.entity(entity).insert(Opened);
        runtime.clear_interact_buffer();
        runtime.features.banner = format!("opened {}", name.0.as_str());
        runtime.features.banner_timer = 2.6;
        let pos = aabb.center;
        vfx.write(VfxMessage::Burst {
            pos,
            count: 16,
            speed: 230.0,
            color: [0.84, 0.95, 1.0, 0.82],
            kind: ParticleKind::Spark,
        });
        sfx.write(SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_TREASURE_CHEST_OPEN,
            pos,
        });
        if let Some(encounter_id) = id.as_str().strip_prefix("encounter_chest_") {
            gameplay_effects.write(GameplayEffect::SetFlag {
                id: format!("encounter_{encounter_id}_reward_dropped"),
                on: true,
            });
        }
        break;
    }
}

/// Tick ECS-owned breakable timers and stand-to-break triggers.
pub fn update_ecs_breakables(
    mut commands: Commands,
    time: Res<Time>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    mut breakables: Query<(
        Entity,
        &FeatureName,
        &FeatureAabb,
        &mut BreakableFeature,
        Option<&mut RespawnTimer>,
        Option<&mut StandTimer>,
    ), With<FeatureSimEntity>>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
) {
    let dt = time.delta_secs();
    let player_body = runtime.player.aabb();
    for (entity, name, aabb, mut feature, respawn_timer, stand_timer) in &mut breakables {
        if feature.broken() {
            if let Some(mut timer) = respawn_timer {
                timer.0 = (timer.0 - dt).max(0.0);
                if timer.0 <= 0.0 {
                    feature.breakable.state = ae::BreakableState::Intact;
                    feature.breakable.health.reset();
                    commands.entity(entity).remove::<RespawnTimer>();
                    runtime.features.banner = format!("{} respawned", name.0.as_str());
                    runtime.features.banner_timer = 2.6;
                    vfx.write(VfxMessage::Burst {
                        pos: aabb.center,
                        count: 16,
                        speed: 230.0,
                        color: [0.84, 0.95, 1.0, 0.82],
                        kind: ParticleKind::Spark,
                    });
                }
            }
            continue;
        }

        let breaks_on_stand = feature.breakable.collision.blocks_movement()
            && feature.breakable.trigger.allows_stand();
        let Some(mut stand_timer) = stand_timer else {
            continue;
        };
        if breaks_on_stand && player_is_standing_on(player_body, aabb.aabb()) {
            stand_timer.0 += dt;
            if stand_timer.0 >= BREAK_ON_STAND_SECONDS {
                let damage = feature.breakable.health.current.max(1);
                let broke = feature.breakable.apply_damage(damage);
                if broke {
                    begin_ecs_breakable_respawn(&mut commands, entity, &feature.breakable);
                    stand_timer.0 = 0.0;
                    runtime.features.banner = format!("{} collapsed under weight", name.0.as_str());
                    runtime.features.banner_timer = 2.6;
                    emit_breakable_destroyed(aabb.center, &mut sfx, &mut vfx, &mut debris);
                }
            }
        } else {
            stand_timer.0 = (stand_timer.0 - dt * 2.0).max(0.0);
        }
    }
}

/// Drain queued slash/projectile/pogo damage into ECS breakables.
pub fn apply_ecs_breakable_damage_queue(
    mut commands: Commands,
    mut queues: ResMut<FeatureEcsQueues>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    mut breakables: Query<(Entity, &FeatureId, &FeatureName, &FeatureAabb, &mut BreakableFeature), With<FeatureSimEntity>>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
) {
    let damage_events = std::mem::take(&mut queues.damage_events);
    let pogo_bounces = std::mem::take(&mut queues.pogo_bounces);

    for event in damage_events {
        for (entity, id, name, aabb, mut feature) in &mut breakables {
            let key = format!("breakable:{}", id.as_str());
            if event.ignored_targets.iter().any(|ignored| ignored == &key) {
                continue;
            }
            if feature.broken() || !feature.breakable.trigger.allows_hit() {
                continue;
            }
            if feature.breakable.pogo_refresh {
                continue;
            }
            if !event.volume.strict_intersects(aabb.aabb()) {
                continue;
            }
            let broke = feature.breakable.apply_damage(event.damage.max(1));
            vfx.write(VfxMessage::Impact { pos: midpoint(event.volume.center(), aabb.center) });
            if broke {
                begin_ecs_breakable_respawn(&mut commands, entity, &feature.breakable);
                runtime.features.banner = format!("broke {}", name.0.as_str());
                runtime.features.banner_timer = 2.6;
                emit_breakable_destroyed(aabb.center, &mut sfx, &mut vfx, &mut debris);
            }
        }
    }

    for (orb_aabb, damage) in pogo_bounces {
        for (entity, _id, name, aabb, mut feature) in &mut breakables {
            if feature.broken() || !feature.breakable.pogo_refresh {
                continue;
            }
            if !approximately_same_aabb(aabb.aabb(), orb_aabb) {
                continue;
            }
            let broke = feature.breakable.apply_damage(damage.max(1));
            vfx.write(VfxMessage::Impact { pos: aabb.center });
            if broke {
                begin_ecs_breakable_respawn(&mut commands, entity, &feature.breakable);
                runtime.features.banner = format!("shattered {}", name.0.as_str());
                runtime.features.banner_timer = 2.6;
                emit_breakable_destroyed(aabb.center, &mut sfx, &mut vfx, &mut debris);
            }
        }
    }
}


/// Read-only hit test used by systems that need immediate projectile / attack
/// feedback while damage application is still drained through
/// `FeatureEcsQueues`.
pub fn ecs_damage_event_hits_breakable(
    event: &DamageEvent,
    breakables: &Query<(&FeatureId, &FeatureAabb, &BreakableFeature), With<FeatureSimEntity>>,
) -> bool {
    breakables.iter().any(|(id, aabb, feature)| {
        let key = format!("breakable:{}", id.as_str());
        !event.ignored_targets.iter().any(|ignored| ignored == &key)
            && !feature.broken()
            && feature.breakable.trigger.allows_hit()
            && !feature.breakable.pogo_refresh
            && event.volume.strict_intersects(aabb.aabb())
    })
}

fn begin_ecs_breakable_respawn(
    commands: &mut Commands,
    entity: Entity,
    breakable: &ae::Breakable,
) {
    if let ae::RespawnPolicy::AfterSeconds(seconds) = breakable.respawn {
        commands.entity(entity).insert(RespawnTimer(seconds));
    }
}

fn emit_breakable_destroyed(
    pos: ae::Vec2,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    debris: &mut MessageWriter<DebrisBurstMessage>,
) {
    vfx.write(VfxMessage::Burst {
        pos,
        count: 16,
        speed: 230.0,
        color: [0.84, 0.95, 1.0, 0.82],
        kind: ParticleKind::Spark,
    });
    debris.write(DebrisBurstMessage {
        pos,
        cue: PhysicsDebrisCue::Breakable,
    });
    sfx.write(SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_CRATE_BREAK,
        pos,
    });
}

/// ECS view lookup for migrated feature visual sync.
pub fn ecs_feature_view(
    id: &str,
    pickups: &Query<(&FeatureId, &FeatureAabb, Option<&Collected>), With<PickupFeature>>,
    chests: &Query<(&FeatureId, &FeatureAabb, Option<&Opened>), With<ChestFeature>>,
    breakables: &Query<(&FeatureId, &FeatureAabb, &BreakableFeature)>,
) -> Option<FeatureView> {
    for (feature_id, aabb, collected) in pickups.iter() {
        if feature_id.as_str() == id {
            return Some(FeatureView {
                pos: aabb.center,
                size: aabb.size(),
                kind: FeatureVisualKind::Pickup,
                visible: collected.is_none(),
                flash: false,
                switch_on: false,
            });
        }
    }
    for (feature_id, aabb, opened) in chests.iter() {
        if feature_id.as_str() == id {
            return Some(FeatureView {
                pos: aabb.center,
                size: aabb.size(),
                kind: FeatureVisualKind::Chest,
                visible: true,
                flash: opened.is_some(),
                switch_on: false,
            });
        }
    }
    for (feature_id, aabb, breakable) in breakables.iter() {
        if feature_id.as_str() == id {
            return Some(FeatureView {
                pos: aabb.center,
                size: aabb.size(),
                kind: FeatureVisualKind::Breakable,
                visible: !breakable.broken(),
                flash: breakable.breakable.state == ae::BreakableState::Cracking,
                switch_on: false,
            });
        }
    }
    None
}

/// ECS chest-opened lookup for sprite swapping.
pub fn ecs_chest_opened(
    id: &str,
    chests: &Query<(&FeatureId, Option<&Opened>), With<ChestFeature>>,
) -> Option<bool> {
    chests
        .iter()
        .find(|(feature_id, _)| feature_id.as_str() == id)
        .map(|(_, opened)| opened.is_some())
}

/// ECS breakable-state lookup for sprite swapping.
pub fn ecs_breakable_state(
    id: &str,
    breakables: &Query<(&FeatureId, &BreakableFeature)>,
) -> Option<ae::BreakableState> {
    breakables
        .iter()
        .find(|(feature_id, _)| feature_id.as_str() == id)
        .map(|(_, breakable)| breakable.breakable.state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::{App, Update};

    #[test]
    fn ecs_overlay_ignores_broken_breakables() {
        let mut breakable = ae::Breakable::new("crate", 1);
        breakable.collision = ae::BreakableCollision::Solid;
        let mut app = App::new();
        app.insert_resource(FeatureEcsWorldOverlay::default());
        app.world_mut().spawn((
            FeatureSimEntity,
            FeatureId::new("crate"),
            FeatureName::new("crate"),
            FeatureAabb::from_center_size(ae::Vec2::ZERO, ae::Vec2::new(16.0, 16.0)),
            BreakableFeature::new(breakable),
        ));
        app.add_systems(Update, rebuild_feature_ecs_world_overlay);
        app.update();
        assert_eq!(app.world().resource::<FeatureEcsWorldOverlay>().blocks.len(), 1);
    }
}
