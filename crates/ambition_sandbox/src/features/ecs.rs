//! ECS-native feature runtime systems.
//!
//! This is the Phase 3/4/5 strangler path for simple feature families. Static
//! authored pickups, chests, and breakables are spawned as Bevy entities at room
//! load and updated by the systems in this module. `FeatureRuntime` remains the
//! compatibility shell for hazards/bosses and for dynamic encounter
//! enemies/reward-chest paths that still originate in runtime-owned encounter
//! code. Authored pickups, chests, breakables, switches, and actor NPC/enemy
//! features now live as ECS entities.

use super::*;
use crate::audio::SfxMessage;
use crate::fx::{ParticleKind, VfxMessage};
use crate::physics::{DebrisBurstMessage, PhysicsDebrisCue};
use crate::rendering::RoomVisual;
use bevy::prelude::{Commands, Component, Entity, MessageWriter, NextState, Query, Res, ResMut, Resource, Time, With};

/// Marker for simulation-side feature entities spawned from the active room.
/// They are deliberately separate from presentation `FeatureVisual` sprites;
/// visible builds keep using the existing visual entities and look up live ECS
/// state by `FeatureId`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeatureSimEntity;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorDisposition {
    Peaceful,
    Hostile,
}

/// Unified ECS runtime for authored NPCs and enemies.
///
/// The only meaningful gameplay distinction is disposition: peaceful actors
/// talk / patrol, hostile actors chase / attack. A peaceful NPC can flip into
/// the hostile branch in-place after enough strikes instead of being removed
/// from one runtime vector and reinserted into another.
#[derive(Component, Clone, Debug)]
pub enum ActorRuntime {
    Peaceful(NpcRuntime),
    Hostile(EnemyRuntime),
}

impl ActorRuntime {
    pub fn id(&self) -> &str {
        match self {
            Self::Peaceful(actor) => actor.id.as_str(),
            Self::Hostile(actor) => actor.id.as_str(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Peaceful(actor) => actor.name.as_str(),
            Self::Hostile(actor) => actor.name.as_str(),
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        match self {
            Self::Peaceful(actor) => actor.aabb(),
            Self::Hostile(actor) => actor.aabb(),
        }
    }

    pub fn pos(&self) -> ae::Vec2 {
        match self {
            Self::Peaceful(actor) => actor.pos,
            Self::Hostile(actor) => actor.pos,
        }
    }

    pub fn size(&self) -> ae::Vec2 {
        match self {
            Self::Peaceful(actor) => actor.size,
            Self::Hostile(actor) => actor.size,
        }
    }

    pub fn disposition(&self) -> ActorDisposition {
        match self {
            Self::Peaceful(_) => ActorDisposition::Peaceful,
            Self::Hostile(_) => ActorDisposition::Hostile,
        }
    }

    pub fn visual_kind(&self) -> FeatureVisualKind {
        match self {
            Self::Peaceful(_) => FeatureVisualKind::Npc,
            Self::Hostile(enemy) => enemy.visual_kind(),
        }
    }

    pub fn visible(&self) -> bool {
        match self {
            Self::Peaceful(_) => true,
            Self::Hostile(enemy) => enemy.alive,
        }
    }

    pub fn flash(&self) -> bool {
        match self {
            Self::Peaceful(npc) => npc.hit_flash > 0.0,
            Self::Hostile(enemy) => {
                enemy.hit_flash > 0.0 || enemy.attack_windup_timer > 0.0 || enemy.attack_timer > 0.0
            }
        }
    }

    pub fn feature_view(&self) -> FeatureView {
        FeatureView {
            pos: self.pos(),
            size: self.size(),
            kind: self.visual_kind(),
            visible: self.visible(),
            flash: self.flash(),
            switch_on: false,
        }
    }

    fn hostile_from_npc(npc: &NpcRuntime) -> EnemyRuntime {
        let object = ae::RoomObject::new(
            npc.id.clone(),
            npc.name.clone(),
            npc.aabb(),
            ae::RoomObjectKind::EnemySpawn(ae::EnemyBrain::Custom("medium_striker".into())),
        );
        let mut enemy = EnemyRuntime::new(
            &object,
            ae::EnemyBrain::Custom("medium_striker".into()),
            &[],
        );
        enemy.pos = npc.pos;
        enemy.spawn = npc.spawn;
        enemy.size = ae::Vec2::new(npc.size.x.max(22.0), npc.size.y.max(38.0));
        enemy.vel = npc.vel;
        enemy.facing = npc.facing;
        enemy.on_ground = npc.on_ground;
        if npc.name != "Kernel Guide NPC" {
            enemy.sprite_override_npc_name = Some(npc.name.clone());
        }
        enemy
    }
}

/// Per-frame damage/pogo work that still originates inside the legacy
/// `sandbox_update` loop. Systems after `sandbox_update` drain this into ECS
/// feature components.
#[derive(Resource, Default, Debug)]
pub struct FeatureEcsQueues {
    pub damage_events: Vec<DamageEvent>,
    pub pogo_bounces: Vec<(ae::Aabb, i32)>,
    pub pending_events: FeatureEvents,
    pub reset_room_features: bool,
}

impl FeatureEcsQueues {
    pub fn drain_events(&mut self) -> FeatureEvents {
        std::mem::take(&mut self.pending_events)
    }
}

/// Collision contribution from ECS-owned breakables. Rebuilt before the main
/// sandbox tick and consumed by `world_with_sandbox_solids` anywhere the engine
/// needs the augmented collision world.
#[derive(Resource, Default, Clone, Debug)]
pub struct FeatureEcsWorldOverlay {
    pub blocks: Vec<ae::Block>,
}

/// Spawn ECS-native feature entities for every static feature object in a room.
pub fn spawn_room_feature_entities(commands: &mut Commands, room: &crate::rooms::RoomSpec) {
    let paths = room_spec_paths(room);
    for object in &room.world.objects {
        spawn_room_feature_entity(commands, object, &paths);
    }
}

fn spawn_room_feature_entity(
    commands: &mut Commands,
    object: &ae::RoomObject,
    paths: &[(String, ae::KinematicPath)],
) {
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
        ae::RoomObjectKind::EnemySpawn(brain) => {
            commands.spawn((
                Name::new(format!("Feature actor enemy: {}", object.name)),
                FeatureSimEntity,
                RoomVisual,
                FeatureId::new(object.id.clone()),
                FeatureName::new(object.name.clone()),
                feature_aabb,
                ActorRuntime::Hostile(EnemyRuntime::new(object, brain.clone(), paths)),
            ));
        }
        ae::RoomObjectKind::Interactable(interactable) => {
            if matches!(interactable.kind, ae::InteractionKind::Npc { .. }) {
                commands.spawn((
                    Name::new(format!("Feature actor npc: {}", object.name)),
                    FeatureSimEntity,
                    RoomVisual,
                    FeatureId::new(object.id.clone()),
                    FeatureName::new(object.name.clone()),
                    feature_aabb,
                    ActorRuntime::Peaceful(NpcRuntime::new_with_paths(
                        object,
                        interactable.clone(),
                        paths,
                    )),
                ));
            } else if let ae::InteractionKind::Custom(payload) = &interactable.kind {
                if payload.starts_with("switch:") {
                    commands.spawn((
                        Name::new(format!("Feature switch: {}", object.name)),
                        FeatureSimEntity,
                        RoomVisual,
                        FeatureId::new(object.id.clone()),
                        FeatureName::new(object.name.clone()),
                        feature_aabb,
                        SwitchFeature::new(payload.clone()),
                        SwitchOn(false),
                    ));
                }
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
    mut actors: Query<(&mut FeatureAabb, &mut ActorRuntime), With<FeatureSimEntity>>,
    mut switches: Query<&mut SwitchOn, With<SwitchFeature>>,
) {
    if !queues.reset_room_features {
        return;
    }
    queues.reset_room_features = false;
    queues.damage_events.clear();
    queues.pogo_bounces.clear();
    queues.pending_events = FeatureEvents::default();

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
    for (mut aabb, mut actor) in &mut actors {
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
                enemy.pos = enemy.spawn;
                aabb.center = enemy.spawn;
                enemy.vel = ae::Vec2::ZERO;
                enemy.alive = true;
                enemy.health.reset();
                enemy.hit_flash = 0.0;
                enemy.attack_timer = 0.0;
                enemy.attack_windup_timer = 0.0;
            }
        }
    }
    for mut switch_on in &mut switches {
        switch_on.0 = false;
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
    mut actors: Query<(&FeatureId, &FeatureAabb, &mut ActorRuntime), With<FeatureSimEntity>>,
    mut gameplay_effects: MessageWriter<GameplayEffect>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
) {
    let damage_events = std::mem::take(&mut queues.damage_events);
    let pogo_bounces = std::mem::take(&mut queues.pogo_bounces);

    for event in damage_events {
        let mut actor_hit_this_event = false;
        for (id, aabb, mut actor) in &mut actors {
            let key = match &*actor {
                ActorRuntime::Peaceful(_) => format!("npc:{}", id.as_str()),
                ActorRuntime::Hostile(_) => format!("enemy:{}", id.as_str()),
            };
            if event.ignored_targets.iter().any(|ignored| ignored == &key) {
                continue;
            }
            if !event.volume.strict_intersects(aabb.aabb()) {
                continue;
            }
            match &mut *actor {
                ActorRuntime::Peaceful(npc) => {
                    npc.hit_flash = 0.18;
                    npc.strikes = npc.strikes.saturating_add(1);
                    let impact = midpoint(event.volume.center(), npc.pos);
                    vfx.write(VfxMessage::Impact { pos: impact });
                    gameplay_effects.write(GameplayEffect::StrikeNpc {
                        npc_id: npc.id.clone(),
                        pos: npc.pos,
                    });
                    actor_hit_this_event = true;
                    if npc.strikes >= NPC_HOSTILE_STRIKE_THRESHOLD {
                        let hostile = ActorRuntime::hostile_from_npc(npc);
                        gameplay_effects.write(GameplayEffect::SetFlag {
                            id: npc.flag_id(),
                            on: true,
                        });
                        vfx.write(VfxMessage::SpeechBubble {
                            pos: npc.bark_anchor(),
                            text: npc.hostile_bark().to_string(),
                        });
                        vfx.write(VfxMessage::Burst {
                            pos: npc.pos,
                            count: 16,
                            speed: 230.0,
                            color: [0.84, 0.95, 1.0, 0.82],
                            kind: ParticleKind::Spark,
                        });
                        runtime.features.banner = format!("{} turns hostile", npc.name);
                        runtime.features.banner_timer = 2.6;
                        *actor = ActorRuntime::Hostile(hostile);
                    } else {
                        vfx.write(VfxMessage::SpeechBubble {
                            pos: npc.bark_anchor(),
                            text: npc.hit_bark().to_string(),
                        });
                    }
                }
                ActorRuntime::Hostile(enemy) => {
                    if !enemy.alive {
                        continue;
                    }
                    enemy.hit_flash = 0.16;
                    if let DamageSource::PlayerSlash { knock_x } = &event.source {
                        enemy.vel.x += *knock_x;
                        enemy.vel.y = (enemy.vel.y - 90.0).max(-280.0);
                    }
                    let killed = if enemy.archetype == EnemyArchetype::InfiniteSandbag {
                        false
                    } else {
                        enemy.health.damage(event.damage.max(1))
                    };
                    let impact = midpoint(event.volume.center(), enemy.pos);
                    vfx.write(VfxMessage::Impact { pos: impact });
                    actor_hit_this_event = true;
                    if killed {
                        enemy.alive = false;
                        if enemy.archetype == EnemyArchetype::FiniteSandbag {
                            enemy.respawn_timer = 0.85;
                            runtime.features.banner = format!("{} dropped; respawning", enemy.name);
                        } else {
                            runtime.features.banner = format!("defeated {}", enemy.name);
                            if !enemy.id.starts_with("encounter:")
                                && enemy.archetype != EnemyArchetype::InfiniteSandbag
                                && enemy.archetype != EnemyArchetype::FiniteSandbag
                            {
                                gameplay_effects.write(GameplayEffect::SetFlag {
                                    id: format!("enemy_{}_dead", enemy.id),
                                    on: true,
                                });
                            }
                        }
                        runtime.features.banner_timer = 2.6;
                        vfx.write(VfxMessage::Burst {
                            pos: enemy.pos,
                            count: 16,
                            speed: 230.0,
                            color: [0.84, 0.95, 1.0, 0.82],
                            kind: ParticleKind::Spark,
                        });
                        debris.write(DebrisBurstMessage {
                            pos: enemy.pos,
                            cue: PhysicsDebrisCue::EnemyRagdoll,
                        });
                        sfx.write(SfxMessage::Death { pos: enemy.pos });
                    }
                }
            }
        }
        if actor_hit_this_event {
            runtime.hitstop_timer = runtime.hitstop_timer.max(0.06);
            runtime.flash_timer = runtime.flash_timer.max(0.10);
            sfx.write(SfxMessage::Hit { pos: event.volume.center() });
        }

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


/// Tick authored ECS actors. Peaceful and hostile actors share the same entity
/// identity and can switch disposition in-place; dynamic encounter-spawned mobs
/// still live in `FeatureRuntime` until encounter spawning is migrated.
pub fn update_ecs_actors(
    time: Res<Time>,
    world: Res<crate::GameWorld>,
    runtime: Res<crate::SandboxRuntime>,
    feel_tuning: Res<crate::feel::SandboxFeelTuning>,
    overlay: Res<FeatureEcsWorldOverlay>,
    mut queues: ResMut<FeatureEcsQueues>,
    mut actors: Query<(&mut FeatureAabb, &mut ActorRuntime), With<FeatureSimEntity>>,
) {
    let dt = time.delta_secs();
    let feature_world = world_with_sandbox_solids(
        &world.0,
        &runtime.moving_platforms,
        &runtime.features,
        &overlay,
    );
    let player = runtime.player.clone();
    let player_body = player.aabb();
    let player_vulnerable = !runtime.player.invincible && runtime.damage_invuln_timer <= 0.0;
    for (mut aabb, mut actor) in &mut actors {
        match &mut *actor {
            ActorRuntime::Peaceful(npc) => {
                npc.update(&feature_world, &player, dt);
                aabb.center = npc.pos;
                aabb.half_size = npc.size * 0.5;
            }
            ActorRuntime::Hostile(enemy) => {
                enemy.update(&feature_world, &player, feel_tuning.feature_combat_tuning(), dt);
                aabb.center = enemy.pos;
                aabb.half_size = enemy.size * 0.5;
                if player_vulnerable && enemy.alive {
                    if let Some(damage) = enemy.player_damage(player_body) {
                        queues
                            .pending_events
                            .messages
                            .push(format!("{} hit the player", enemy.name));
                        queues.pending_events.impacts.push(damage.impact_pos);
                        queues
                            .pending_events
                            .play_sfx(ambition_sfx::ids::PLAYER_DAMAGE, damage.impact_pos);
                        queues.pending_events.player_damage.push(damage);
                    }
                }
            }
        }
    }
}

/// Handle interactions with ECS switches and peaceful NPCs. Chests stay in
/// `open_ecs_chests` because they have their own reward/persistence path.
pub fn interact_ecs_actors_and_switches(
    mut runtime: ResMut<crate::SandboxRuntime>,
    mut next_mode: ResMut<NextState<crate::GameMode>>,
    actors: Query<(&FeatureAabb, &ActorRuntime), With<FeatureSimEntity>>,
    mut switches: Query<(&FeatureId, &FeatureName, &FeatureAabb, &SwitchFeature, &mut SwitchOn), With<FeatureSimEntity>>,
    mut gameplay_effects: MessageWriter<GameplayEffect>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    if runtime.interact_buffer_timer <= 0.0 {
        return;
    }
    let player_body = runtime.player.aabb();
    for (aabb, actor) in &actors {
        let ActorRuntime::Peaceful(npc) = actor else {
            continue;
        };
        if !aabb.aabb().strict_intersects(player_body) {
            continue;
        }
        runtime.clear_interact_buffer();
        runtime.features.banner = npc.message();
        runtime.features.banner_timer = 2.6;
        let request = npc.dialogue_request();
        runtime.dialogue.start(&request.dialogue_id, &request.npc_name);
        next_mode.set(crate::GameMode::Dialogue);
        gameplay_effects.write(GameplayEffect::AdvanceQuest(ae::QuestAdvanceEvent::NpcTalked(npc.id.clone())));
        gameplay_effects.write(GameplayEffect::SetFlag { id: "met_any_hub_npc".into(), on: true });
        gameplay_effects.write(GameplayEffect::SetFlag { id: format!("npc_{}_talked", request.dialogue_id), on: true });
        vfx.write(VfxMessage::Burst {
            pos: npc.pos,
            count: 16,
            speed: 230.0,
            color: [0.84, 0.95, 1.0, 0.82],
            kind: ParticleKind::Spark,
        });
        return;
    }

    for (_id, name, aabb, switch, mut on) in &mut switches {
        if !aabb.aabb().strict_intersects(player_body) {
            continue;
        }
        runtime.clear_interact_buffer();
        runtime.features.banner = format!("activated {}", name.0.as_str());
        runtime.features.banner_timer = 2.6;
        on.0 = true;
        gameplay_effects.write(GameplayEffect::ActivateSwitch {
            payload: switch.payload.clone(),
            pos: aabb.center,
        });
        vfx.write(VfxMessage::Burst {
            pos: aabb.center,
            count: 16,
            speed: 230.0,
            color: [0.84, 0.95, 1.0, 0.82],
            kind: ParticleKind::Spark,
        });
        return;
    }
}


/// Mirror save-derived actor state onto ECS-owned authored NPC/enemy actors.
///
/// This is the ECS counterpart to `FeatureRuntime::apply_save`: provoked NPCs
/// load as hostile actors, and persisted non-respawning enemy deaths stay dead
/// across room reloads. Dynamic encounter mobs still live in `FeatureRuntime`
/// and continue to use the legacy save path.
pub fn sync_ecs_actors_with_save(
    save: Res<crate::save::SandboxSave>,
    mut actors: Query<&mut ActorRuntime, With<FeatureSimEntity>>,
) {
    let data = save.data();
    for mut actor in &mut actors {
        match &mut *actor {
            ActorRuntime::Peaceful(npc) => {
                if data.flag(&npc.flag_id()) {
                    let mut hostile = ActorRuntime::hostile_from_npc(npc);
                    if data.flag(&format!("enemy_{}_dead", hostile.id)) {
                        hostile.alive = false;
                        hostile.health.current = 0;
                    }
                    *actor = ActorRuntime::Hostile(hostile);
                }
            }
            ActorRuntime::Hostile(enemy) => {
                if !enemy.id.starts_with("encounter:")
                    && enemy.archetype != EnemyArchetype::InfiniteSandbag
                    && enemy.archetype != EnemyArchetype::FiniteSandbag
                    && data.flag(&format!("enemy_{}_dead", enemy.id))
                {
                    enemy.alive = false;
                    enemy.health.current = 0;
                }
            }
        }
    }
}

/// Mirror the remaining encounter-owned switch latch state from the legacy
/// compatibility list onto ECS switch components. This lets encounter arming
/// continue to use its existing switch helpers while rendering/interactions are
/// driven by ECS entities.
pub fn sync_ecs_switches_from_runtime(
    runtime: Res<crate::SandboxRuntime>,
    mut switches: Query<(&FeatureId, &mut SwitchOn), With<SwitchFeature>>,
) {
    for (id, mut switch_on) in &mut switches {
        if let Some(runtime_switch) = runtime.features.switches.iter().find(|switch| switch.id == id.as_str()) {
            switch_on.0 = runtime_switch.on;
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

pub fn ecs_damage_event_hits_actor(
    event: &DamageEvent,
    actors: &Query<(&FeatureId, &FeatureAabb, &ActorRuntime), With<FeatureSimEntity>>,
) -> bool {
    actors.iter().any(|(id, aabb, actor)| {
        let key = match actor {
            ActorRuntime::Peaceful(_) => format!("npc:{}", id.as_str()),
            ActorRuntime::Hostile(_) => format!("enemy:{}", id.as_str()),
        };
        !event.ignored_targets.iter().any(|ignored| ignored == &key)
            && actor.visible()
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
    switches: &Query<(&FeatureId, &FeatureAabb, &SwitchOn), With<SwitchFeature>>,
    actors: &Query<(&FeatureId, &ActorRuntime)>,
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
    for (feature_id, aabb, switch_on) in switches.iter() {
        if feature_id.as_str() == id {
            return Some(FeatureView {
                pos: aabb.center,
                size: aabb.size(),
                kind: FeatureVisualKind::Switch,
                visible: true,
                flash: false,
                switch_on: switch_on.0,
            });
        }
    }
    for (feature_id, actor) in actors.iter() {
        if feature_id.as_str() == id {
            return Some(actor.feature_view());
        }
    }
    None
}

pub fn ecs_actor_view_compat(
    id: &str,
    actors: &Query<(&FeatureId, &ActorRuntime)>,
) -> Option<FeatureView> {
    actors.iter().find_map(|(feature_id, actor)| {
        (feature_id.as_str() == id).then(|| actor.feature_view())
    })
}

pub fn ecs_npc_name<'a>(id: &str, actors: &'a Query<(&FeatureId, &ActorRuntime)>) -> Option<&'a str> {
    actors.iter().find_map(|(feature_id, actor)| {
        if feature_id.as_str() != id {
            return None;
        }
        match actor {
            ActorRuntime::Peaceful(npc) => Some(npc.name.as_str()),
            ActorRuntime::Hostile(enemy) => enemy.sprite_override_npc_name.as_deref(),
        }
    })
}

pub fn ecs_enemy_sprite_override<'a>(
    id: &str,
    actors: &'a Query<(&FeatureId, &ActorRuntime)>,
) -> Option<&'a str> {
    actors.iter().find_map(|(feature_id, actor)| {
        if feature_id.as_str() != id {
            return None;
        }
        match actor {
            ActorRuntime::Hostile(enemy) => enemy.sprite_override_npc_name.as_deref(),
            _ => None,
        }
    })
}

pub fn ecs_enemy_anim_state(
    id: &str,
    actors: &Query<(&FeatureId, &ActorRuntime)>,
) -> Option<crate::character_sprites::EnemyAnimState> {
    actors.iter().find_map(|(feature_id, actor)| {
        if feature_id.as_str() != id {
            return None;
        }
        match actor {
            ActorRuntime::Hostile(enemy) => Some(crate::character_sprites::EnemyAnimState {
                vel: enemy.vel,
                facing: enemy.facing,
                alive: enemy.alive,
                attack_active: enemy.attack_timer > 0.0,
                attack_windup: enemy.attack_windup_timer > 0.0,
                hit_flash: enemy.hit_flash > 0.0,
            }),
            _ => None,
        }
    })
}

pub fn ecs_npc_anim_state(
    id: &str,
    actors: &Query<(&FeatureId, &ActorRuntime)>,
) -> Option<crate::character_sprites::NpcAnimState> {
    actors.iter().find_map(|(feature_id, actor)| {
        if feature_id.as_str() != id {
            return None;
        }
        match actor {
            ActorRuntime::Peaceful(npc) => Some(crate::character_sprites::NpcAnimState {
                vel: npc.vel,
                facing: npc.facing,
                hit_flash: npc.hit_flash > 0.0,
            }),
            _ => None,
        }
    })
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
