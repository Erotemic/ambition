//! ECS-native feature simulation.
//!
//! Authored and dynamic pickups, chests, breakables, switches, NPCs, enemies,
//! hazards, and bosses are spawned as Bevy entities and updated by the systems
//! in this module. This is the authoritative feature implementation.
//!
//! The damage-event application path (typed slash/projectile/pogo damage,
//! breakable shatter side effects, hit predicates) lives in [`damage`].

use super::*;
use crate::audio::SfxMessage;
use crate::presentation::fx::{ParticleKind, VfxMessage};
use crate::presentation::rendering::RoomVisual;
use crate::world::physics::{DebrisBurstMessage, PhysicsDebrisCue};
use bevy::prelude::{
    Commands, Component, Entity, MessageReader, MessageWriter, NextState, Query, Res, ResMut,
    Resource, With,
};

use crate::WorldTime;

mod damage;
mod view_index;
mod save_sync;
mod anim_helpers;
mod falling_chest;
mod encounter_rewards;

pub use damage::{
    apply_feature_damage_events, ecs_damage_event_hits_actor, ecs_damage_event_hits_boss,
    ecs_damage_event_hits_breakable,
};
pub use view_index::{rebuild_feature_view_index, FeatureViewIndex};
pub use save_sync::{sync_ecs_actors_with_save, sync_ecs_bosses_with_save, sync_ecs_switches_from_save};
pub use anim_helpers::{ecs_boss_anim_state, ecs_boss_name, ecs_breakable_state, ecs_chest_opened, ecs_enemy_anim_state, ecs_enemy_name, ecs_enemy_sprite_override, ecs_npc_anim_state, ecs_npc_name};
pub use falling_chest::update_ecs_falling_chests;
pub use encounter_rewards::{clear_encounter_reward_ecs, sync_boss_reward_chests_ecs, sync_encounter_reward_chests_ecs};
use damage::{begin_ecs_breakable_respawn, emit_breakable_destroyed};

/// Marker for simulation-side feature entities spawned from the active room.
/// They are deliberately separate from presentation `FeatureVisual` sprites;
/// visible builds keep using the existing visual entities and look up live ECS
/// state by `FeatureId`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeatureSimEntity;

#[derive(Component, Clone, Debug)]
pub struct HazardFeature {
    pub hazard: HazardRuntime,
    pub spawn: ae::Vec2,
}

impl HazardFeature {
    pub fn new(hazard: HazardRuntime) -> Self {
        let spawn = hazard.pos;
        Self { hazard, spawn }
    }
}

#[derive(Component, Clone, Debug)]
pub struct BossFeature {
    pub boss: BossRuntime,
}

impl BossFeature {
    pub fn new(boss: BossRuntime) -> Self {
        Self { boss }
    }
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

fn actor_component_snapshot(
    actor: &ActorRuntime,
) -> (
    ActorIdentity,
    ActorDisposition,
    ActorHealth,
    ActorCombatState,
    ActorIntent,
    ActorCooldowns,
) {
    match actor {
        ActorRuntime::Peaceful(npc) => (
            ActorIdentity::new(npc.id.clone(), npc.name.clone()),
            ActorDisposition::Peaceful,
            ActorHealth::new(ae::Health::new(1)),
            ActorCombatState::peaceful(npc.strikes, npc.hit_flash),
            ActorIntent::new(ae::CharacterAiMode::Idle),
            ActorCooldowns::default(),
        ),
        ActorRuntime::Hostile(enemy) => (
            ActorIdentity::new(enemy.id.clone(), enemy.name.clone())
                .with_sprite_override(enemy.sprite_override_npc_name.clone()),
            ActorDisposition::Hostile,
            ActorHealth::new(enemy.health),
            ActorCombatState::hostile(
                enemy.alive,
                enemy.hit_flash,
                enemy.attack_windup_timer,
                enemy.attack_timer,
                enemy.archetype.is_sandbag(),
            ),
            ActorIntent::new(enemy.ai_mode),
            ActorCooldowns {
                attack_cooldown: enemy.attack_cooldown,
                respawn_timer: enemy.respawn_timer,
            },
        ),
    }
}

pub(crate) fn sync_actor_components_from_runtime(
    actor: &ActorRuntime,
    identity: &mut ActorIdentity,
    disposition: &mut ActorDisposition,
    health: &mut ActorHealth,
    combat: &mut ActorCombatState,
    intent: &mut ActorIntent,
    cooldowns: &mut ActorCooldowns,
) {
    let (next_identity, next_disposition, next_health, next_combat, next_intent, next_cooldowns) =
        actor_component_snapshot(actor);
    *identity = next_identity;
    *disposition = next_disposition;
    *health = next_health;
    *combat = next_combat;
    *intent = next_intent;
    *cooldowns = next_cooldowns;
}

/// Collision contribution from ECS-owned breakables. Rebuilt before the main
/// sandbox tick and consumed by `world_with_sandbox_solids` anywhere the engine
/// needs the augmented collision world.
#[derive(Resource, Default, Clone, Debug)]
pub struct FeatureEcsWorldOverlay {
    pub blocks: Vec<ae::Block>,
}

/// Tick the gameplay banner resource once per frame.
pub fn tick_gameplay_banner(world_time: Res<WorldTime>, mut banner: ResMut<GameplayBanner>) {
    // Sim clock: the gameplay banner displays gameplay-driven
    // messages (quest hints, encounter intros) so its dismissal
    // timer should pause alongside the sim — otherwise the banner
    // burns its display window during bullet-time / pause.
    banner.tick(world_time.sim_dt());
}

/// Apply deferred banner requests from high-param systems.
pub fn apply_gameplay_banner_requests(
    mut banner: ResMut<GameplayBanner>,
    mut requests: MessageReader<GameplayBannerRequested>,
) {
    for request in requests.read() {
        banner.show(request.text.clone(), request.duration);
    }
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
        ae::RoomObjectKind::DamageVolume(volume) => {
            let hazard = HazardRuntime::new_with_paths(object, volume.clone(), paths);
            commands.spawn((
                Name::new(format!("Feature hazard: {}", object.name)),
                FeatureSimEntity,
                RoomVisual,
                FeatureId::new(object.id.clone()),
                FeatureName::new(object.name.clone()),
                FeatureAabb::from_center_size(hazard.pos, hazard.size),
                HazardFeature::new(hazard),
            ));
        }
        ae::RoomObjectKind::BossSpawn(brain) => {
            let boss = BossRuntime::new(object, brain.clone());
            let initial_phase = BossPhase::from_alive(boss.alive);
            commands.spawn((
                Name::new(format!("Feature boss: {}", object.name)),
                FeatureSimEntity,
                RoomVisual,
                FeatureId::new(object.id.clone()),
                FeatureName::new(object.name.clone()),
                FeatureAabb::from_center_size(boss.pos, boss.render_size()),
                BossPatternTimer(boss.pattern_timer),
                initial_phase,
                BossFeature::new(boss),
            ));
        }
        ae::RoomObjectKind::Pickup(pickup) => {
            commands.spawn((
                Name::new(format!("Feature pickup: {}", object.name)),
                PickupBundle::new(&object.id, &object.name, feature_aabb, pickup.clone()),
            ));
        }
        ae::RoomObjectKind::Chest(chest) => {
            commands.spawn((
                Name::new(format!("Feature chest: {}", object.name)),
                ChestBundle::new(&object.id, &object.name, feature_aabb, chest.clone()),
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
            if breakable.pogo_refresh
                || (breakable.collision.blocks_movement() && breakable.trigger.allows_stand())
            {
                entity.insert(PogoTargetContributor);
            }
        }
        ae::RoomObjectKind::EnemySpawn(brain) => {
            let actor = ActorRuntime::Hostile(EnemyRuntime::new(object, brain.clone(), paths));
            let (identity, disposition, health, combat, intent, cooldowns) =
                actor_component_snapshot(&actor);
            commands.spawn((
                Name::new(format!("Feature actor enemy: {}", object.name)),
                EnemyActorBundle {
                    base: FeatureBaseBundle::new(&object.id, &object.name, feature_aabb),
                    identity,
                    disposition,
                    health,
                    combat,
                    intent,
                    cooldowns,
                },
                actor,
            ));
        }
        ae::RoomObjectKind::Interactable(interactable) => {
            if matches!(interactable.kind, ae::InteractionKind::Npc { .. }) {
                let actor = ActorRuntime::Peaceful(NpcRuntime::new_with_paths(
                    object,
                    interactable.clone(),
                    paths,
                ));
                let (identity, disposition, health, combat, intent, cooldowns) =
                    actor_component_snapshot(&actor);
                commands.spawn((
                    Name::new(format!("Feature actor npc: {}", object.name)),
                    EnemyActorBundle {
                        base: FeatureBaseBundle::new(&object.id, &object.name, feature_aabb),
                        identity,
                        disposition,
                        health,
                        combat,
                        intent,
                        cooldowns,
                    },
                    actor,
                ));
            } else if let ae::InteractionKind::Custom(payload) = &interactable.kind {
                if let Some(activation) = crate::encounter::SwitchActivation::parse_custom(payload)
                {
                    commands.spawn((
                        Name::new(format!("Feature switch: {}", object.name)),
                        FeatureSimEntity,
                        RoomVisual,
                        FeatureId::new(object.id.clone()),
                        FeatureName::new(object.name.clone()),
                        feature_aabb,
                        SwitchFeature::new(activation),
                        SwitchOn(false),
                    ));
                }
            }
        }
        _ => {}
    }
}

/// Spawn one hostile actor for an encounter wave.
///
/// The encounter system still owns wave timing, but the mob itself is a normal
/// feature entity queried by actor, projectile, rendering, and health systems.
pub fn spawn_encounter_mob(
    commands: &mut Commands,
    encounter_id: impl Into<String>,
    id: String,
    brain: ae::EnemyBrain,
    pos: ae::Vec2,
    size: ae::Vec2,
) {
    let encounter_id = encounter_id.into();
    let archetype = EnemyArchetype::from_brain(&brain);
    let aabb = ae::Aabb::new(pos, size * 0.5);
    let object = ae::RoomObject::new(
        id.clone(),
        id.clone(),
        aabb,
        ae::RoomObjectKind::EnemySpawn(brain.clone()),
    );
    let mut enemy = EnemyRuntime::new(&object, brain, &[]);
    enemy.archetype = archetype;
    enemy.health = ae::Health::new(archetype.max_health());
    // Encounter mobs should not auto-respawn like training sandbags.
    enemy.respawn_timer = 999_999.0;
    let actor = ActorRuntime::Hostile(enemy);
    let (identity, disposition, health, combat, intent, cooldowns) =
        actor_component_snapshot(&actor);
    commands.spawn((
        Name::new(format!("Encounter mob: {id}")),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(id.clone()),
        FeatureName::new(id),
        FeatureAabb::from_center_size(pos, size),
        identity,
        disposition,
        health,
        combat,
        intent,
        cooldowns,
        actor,
        EncounterMob::new(encounter_id),
    ));
}

/// Despawn all ECS mobs owned by an encounter attempt.
pub fn despawn_encounter_mobs(
    commands: &mut Commands,
    mobs: &Query<(Entity, &EncounterMob, &FeatureId, &ActorCombatState)>,
    encounter_id: &str,
) {
    for (entity, mob, _, _) in mobs.iter() {
        if mob.encounter_id == encounter_id {
            commands.entity(entity).despawn();
        }
    }
}


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

/// Rebuild the transient collision blocks contributed by ECS-owned breakables.
pub fn rebuild_feature_ecs_world_overlay(
    mut overlay: ResMut<FeatureEcsWorldOverlay>,
    breakables: Query<
        (&FeatureId, &FeatureName, &FeatureAabb, &BreakableFeature),
        With<FeatureSimEntity>,
    >,
    actors: Query<(&FeatureId, &FeatureAabb, &ActorRuntime), With<FeatureSimEntity>>,
    bosses: Query<(&FeatureId, &FeatureAabb, &BossFeature), With<FeatureSimEntity>>,
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
        if feature.breakable.collision.blocks_movement() && feature.breakable.trigger.allows_stand()
        {
            overlay.blocks.push(ae::Block {
                name: format!("ecs-breakable-pogo-target {}", id.as_str()),
                aabb: aabb.aabb(),
                kind: ae::BlockKind::PogoOrb,
            });
        }
    }

    // Expose alive enemy and boss bodies as PogoOrb ghost-blocks so the
    // pogo-attack advance code can bounce off them without requiring the
    // damage queue to resolve first. PogoOrb blocks do not block player
    // movement or blink traversal, so this cannot cause collision regressions.
    for (id, aabb, actor) in &actors {
        let ActorRuntime::Hostile(enemy) = actor else {
            continue;
        };
        if !enemy.alive {
            continue;
        }
        overlay.blocks.push(ae::Block {
            name: format!("ecs-enemy-body {}", id.as_str()),
            aabb: aabb.aabb(),
            kind: ae::BlockKind::PogoOrb,
        });
    }
    for (id, aabb, feature) in &bosses {
        if !feature.boss.alive {
            continue;
        }
        overlay.blocks.push(ae::Block {
            name: format!("ecs-boss-body {}", id.as_str()),
            aabb: aabb.aabb(),
            kind: ae::BlockKind::PogoOrb,
        });
    }
}

/// Collect ECS-owned pickups after the player simulation has advanced.
pub fn collect_ecs_pickups(
    mut commands: Commands,
    mut banner: ResMut<GameplayBanner>,
    player: Query<&crate::player::PlayerBody, With<crate::player::PlayerEntity>>,
    pickups: Query<
        (
            Entity,
            &FeatureName,
            &FeatureAabb,
            &PickupFeature,
            Option<&Collected>,
        ),
        With<FeatureSimEntity>,
    >,
    mut heals: MessageWriter<crate::player::PlayerHealRequested>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    let Ok(player_body) = player.single() else {
        return;
    };
    let player_body = player_body.aabb();
    for (entity, name, aabb, pickup, collected) in &pickups {
        if collected.is_some() || !aabb.aabb().strict_intersects(player_body) {
            continue;
        }
        commands.entity(entity).insert(Collected);
        banner.show(format!("picked up {}", name.0.as_str()), 2.6);
        if let ae::PickupKind::Health { amount } = &pickup.pickup.kind {
            heals.write(crate::player::PlayerHealRequested::new(*amount));
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
    mut banner: ResMut<GameplayBanner>,
    mut player: Query<
        (
            &crate::player::PlayerBody,
            &mut crate::player::PlayerInteractionState,
        ),
        With<crate::player::PlayerEntity>,
    >,
    chests: Query<
        (
            Entity,
            &FeatureId,
            &FeatureName,
            &FeatureAabb,
            Option<&Opened>,
            Option<&FallingChest>,
        ),
        (With<FeatureSimEntity>, With<ChestFeature>),
    >,
    mut gameplay_effects: MessageWriter<GameplayEffect>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    let Ok((player_body, mut interaction)) = player.single_mut() else {
        return;
    };
    if !interaction.buffered() {
        return;
    }
    let player_body = player_body.aabb();
    for (entity, id, name, aabb, opened, falling) in &chests {
        if falling.is_some() || opened.is_some() || !aabb.aabb().strict_intersects(player_body) {
            continue;
        }
        commands.entity(entity).insert(Opened);
        interaction.clear();
        banner.show(format!("opened {}", name.0.as_str()), 2.6);
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
    world_time: Res<WorldTime>,
    player_body_q: Query<&crate::player::PlayerBody, With<crate::player::PlayerEntity>>,
    mut banner: ResMut<GameplayBanner>,
    mut breakables: Query<
        (
            Entity,
            &FeatureName,
            &FeatureAabb,
            &mut BreakableFeature,
            Option<&mut RespawnTimer>,
            Option<&mut StandTimer>,
        ),
        With<FeatureSimEntity>,
    >,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
) {
    // Sim clock: breakable respawn / stand-to-break should freeze in
    // bullet-time alongside the player and enemies (ADR 0010).
    let dt = world_time.sim_dt();
    let Ok(pb) = player_body_q.single() else {
        return;
    };
    let player_body = pb.aabb();
    for (entity, name, aabb, mut feature, respawn_timer, stand_timer) in &mut breakables {
        if feature.broken() {
            if let Some(mut timer) = respawn_timer {
                timer.0 = (timer.0 - dt).max(0.0);
                if timer.0 <= 0.0 {
                    feature.breakable.state = ae::BreakableState::Intact;
                    feature.breakable.health.reset();
                    commands.entity(entity).remove::<RespawnTimer>();
                    banner.show(format!("{} respawned", name.0.as_str()), 2.6);
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
                    banner.show(format!("{} collapsed under weight", name.0.as_str()), 2.6);
                    emit_breakable_destroyed(aabb.center, &mut sfx, &mut vfx, &mut debris);
                }
            }
        } else {
            stand_timer.0 = (stand_timer.0 - dt * 2.0).max(0.0);
        }
    }
}

/// Tick ECS-authored hazards and publish player damage through Bevy messages.
pub fn update_ecs_hazards(
    world_time: Res<WorldTime>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<crate::presentation::fx::VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut player_damage: MessageWriter<PlayerDamageEvent>,
    player: Query<
        (
            &crate::player::PlayerBody,
            &crate::player::PlayerCombatState,
        ),
        With<crate::player::PlayerEntity>,
    >,
    mut hazards: Query<
        (&FeatureName, &mut FeatureAabb, &mut HazardFeature),
        With<FeatureSimEntity>,
    >,
) {
    // Sim clock: patrolling damage volumes must slow in bullet-time
    // so the player can route around them. ADR 0010.
    let dt = world_time.sim_dt();
    let Ok((pb, combat)) = player.single() else {
        return;
    };
    let player_body = pb.aabb();
    let player_pos = pb.pos;
    let player_vulnerable =
        !pb.invincible && !pb.dodge_rolling && !pb.parrying && combat.vulnerable();
    for (_name, mut aabb, mut feature) in &mut hazards {
        let hazard = &mut feature.hazard;
        hazard.update(dt);
        aabb.center = hazard.pos;
        aabb.half_size = hazard.size * 0.5;
        if !player_vulnerable || !hazard.active() || !hazard.aabb().strict_intersects(player_body) {
            continue;
        }
        let pos = player_pos;
        let knockback_dir = (pos.x - hazard.pos.x).signum();
        vfx.write(VfxMessage::Impact { pos });
        vfx.write(VfxMessage::Burst {
            pos,
            count: 14,
            speed: 300.0,
            color: [1.0, 0.34, 0.28, 0.88],
            kind: ParticleKind::Shard,
        });
        debris.write(DebrisBurstMessage {
            pos,
            cue: PhysicsDebrisCue::Impact,
        });
        sfx.write(crate::audio::SfxMessage::Play {
            id: hazard_sfx_id(&hazard.name),
            pos,
        });
        player_damage.write(PlayerDamageEvent {
            mode: hazard.mode,
            source: PlayerDamageSource::Hazard,
            source_pos: hazard.pos,
            impact_pos: pos,
            knockback_dir,
            strength: 1.0,
            amount: hazard.volume.damage.amount.max(1),
        });
    }
}

/// Tick ECS-authored bosses and publish player damage through Bevy messages.
pub fn update_ecs_bosses(
    world_time: Res<WorldTime>,
    world: Res<crate::GameWorld>,
    platform_set: Res<crate::MovingPlatformSet>,
    feel_tuning: Res<crate::time::feel::SandboxFeelTuning>,
    overlay: Res<FeatureEcsWorldOverlay>,
    encounter_registry: Res<crate::boss_encounter::BossEncounterRegistry>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<crate::presentation::fx::VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut player_damage: MessageWriter<PlayerDamageEvent>,
    player_query: Query<
        (
            &crate::player::PlayerBody,
            &crate::player::PlayerCombatState,
            &crate::player::PlayerMovementAuthority,
        ),
        With<crate::player::PlayerEntity>,
    >,
    mut bosses: Query<
        (
            &mut FeatureAabb,
            &mut BossFeature,
            &mut BossPatternTimer,
            &mut BossPhase,
        ),
        With<FeatureSimEntity>,
    >,
) {
    // Sim clock: bosses must slow with bullet-time (ADR 0010); a
    // boss locked-on to the player should not get free hits when
    // the player triggers bullet-time mid-pattern.
    let dt = world_time.sim_dt();
    let feature_world = world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    let Ok((pb, combat, authority)) = player_query.single() else {
        return;
    };
    let player = authority.player.clone();
    let player_body = pb.aabb();
    let player_vulnerable =
        !pb.invincible && !pb.dodge_rolling && !pb.parrying && combat.vulnerable();
    for (mut aabb, mut feature, mut pattern_timer, mut phase) in &mut bosses {
        let boss = &mut feature.boss;
        // Forward this boss's current encounter phase into the runtime
        // so `Scripted` attack patterns can pick the right phase
        // timeline. Look up by the semantic encounter id derived from
        // the boss display name (matches the lazy-register path in
        // `boss_encounter::systems::update_boss_encounters`). If the
        // encounter hasn't been registered yet, leave the previous
        // phase value alone — defaults to `Dormant` from `new()`.
        let encounter_id = crate::boss_encounter::encounter_id_from_name(&boss.name);
        if let Some(state) = encounter_registry.get(&encounter_id) {
            if boss.encounter_phase != state.phase {
                boss.encounter_phase = state.phase;
                // Reset the scripted cursor on phase change so each phase's
                // timeline begins at step 0 rather than mid-step.
                boss.scripted_step_index = 0;
                boss.scripted_step_elapsed = 0.0;
            }
        }
        boss.update(
            &feature_world,
            &player,
            feel_tuning.feature_combat_tuning(),
            dt,
        );
        aabb.center = boss.pos;
        aabb.half_size = boss.render_size() * 0.5;
        pattern_timer.0 = boss.pattern_timer;
        *phase = BossPhase::from_alive(boss.alive);
        if player_vulnerable && boss.alive {
            if let Some(damage) = boss.player_damage(player_body) {
                let pos = damage.impact_pos;
                sfx.write(crate::audio::SfxMessage::Play {
                    id: ambition_sfx::ids::PLAYER_DAMAGE,
                    pos,
                });
                vfx.write(VfxMessage::Impact { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 14,
                    speed: 300.0,
                    color: [1.0, 0.34, 0.28, 0.88],
                    kind: ParticleKind::Shard,
                });
                debris.write(DebrisBurstMessage {
                    pos,
                    cue: PhysicsDebrisCue::Impact,
                });
                player_damage.write(damage);
            }
        }
    }
}

/// Tick ECS actors. Peaceful and hostile actors share the same entity identity
/// and can switch disposition in-place; dynamic encounter-spawned mobs use the
/// same `ActorRuntime::Hostile` path with an `EncounterMob` marker.
pub fn update_ecs_actors(
    world_time: Res<WorldTime>,
    world: Res<crate::GameWorld>,
    platform_set: Res<crate::MovingPlatformSet>,
    feel_tuning: Res<crate::time::feel::SandboxFeelTuning>,
    overlay: Res<FeatureEcsWorldOverlay>,
    mut slot_board: ResMut<crate::combat_slots::CombatSlotsRes>,
    mut enemy_projectiles: ResMut<crate::enemy_projectile::EnemyProjectileState>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<crate::presentation::fx::VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut player_damage: MessageWriter<PlayerDamageEvent>,
    player_query: Query<
        (
            &crate::player::PlayerBody,
            &crate::player::PlayerCombatState,
            &crate::player::PlayerMovementAuthority,
        ),
        With<crate::player::PlayerEntity>,
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
) {
    // Sim clock: enemies, NPCs, encounter mobs all advance on the
    // gameplay clock so bullet-time / pause / hitstop freeze them
    // alongside the player. ADR 0010 + reference_lessons_learned.
    let dt = world_time.sim_dt();
    let feature_world = world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    let Ok((pb, combat, authority)) = player_query.single() else {
        return;
    };
    let player = authority.player.clone();
    let player_body = pb.aabb();
    let player_vulnerable =
        !pb.invincible && !pb.dodge_rolling && !pb.parrying && combat.vulnerable();

    // Pass 1: collect slot requests from every live hostile enemy.
    // The slot board is per-target (player) and arbitrates which
    // enemies are allowed to commit to an attack this tick; the
    // others hold at the outer ring. This is the anti-clump layer.
    let mut requests: Vec<(String, ae::Vec2, ae::SlotKind)> = Vec::new();
    for (_, actor, _, _, _, _, _, _) in &actors {
        if let ActorRuntime::Hostile(enemy) = &*actor {
            if enemy.alive {
                requests.push((enemy.id.clone(), enemy.pos, enemy.archetype.slot_kind()));
            }
        }
    }
    let slot_requests: Vec<ae::SlotRequest> = requests
        .iter()
        .map(|(id, pos, kind)| ae::SlotRequest {
            actor_id: id.as_str(),
            actor_pos: *pos,
            kind: *kind,
        })
        .collect();
    ae::assign_slots(&mut slot_board.0, player.pos, &slot_requests);

    // Per-kind holding-position fallback: when an actor doesn't win
    // a slot, distribute the leftover actors across the holding
    // positions of all slots of their kind. Stable, deterministic
    // ordering by actor id so the assignment doesn't flicker
    // between frames.
    //
    // Without this, multiple unassigned actors of the same kind all
    // picked `slots.iter().find()`'s FIRST matching slot's
    // `holding_pos` — i.e. they shared a single fallback point and
    // visually clumped.
    let mut unassigned_by_kind: std::collections::HashMap<ae::SlotKind, Vec<&str>> =
        std::collections::HashMap::new();
    for (id, _pos, kind) in &requests {
        if slot_board.0.slot_for(id).is_none() {
            unassigned_by_kind
                .entry(*kind)
                .or_default()
                .push(id.as_str());
        }
    }
    let mut holding_pos_by_id: std::collections::HashMap<String, ae::Vec2> =
        std::collections::HashMap::new();
    for (kind, mut ids) in unassigned_by_kind {
        let kind_slots: Vec<usize> = slot_board
            .0
            .slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.kind == kind)
            .map(|(i, _)| i)
            .collect();
        if kind_slots.is_empty() {
            continue;
        }
        ids.sort_unstable(); // stable round-robin order
        for (rank, id) in ids.into_iter().enumerate() {
            let slot_idx = kind_slots[rank % kind_slots.len()];
            holding_pos_by_id.insert(
                id.to_string(),
                slot_board.0.slots[slot_idx].holding_pos(player.pos),
            );
        }
    }

    // Per-actor nearest-same-kind-neighbor index (O(N²), N ≤ a few).
    // Used by the choreography for "personal space" steering so two
    // aerial actors close to each other push apart even when their
    // slot anchors are far apart.
    let mut neighbor_by_id: std::collections::HashMap<String, ae::Vec2> =
        std::collections::HashMap::new();
    for (id_a, pos_a, kind_a) in &requests {
        let mut nearest: Option<(f32, ae::Vec2)> = None;
        for (id_b, pos_b, kind_b) in &requests {
            if id_a == id_b || kind_a != kind_b {
                continue;
            }
            let d = (*pos_a - *pos_b).length_squared();
            if nearest.map(|(best, _)| d < best).unwrap_or(true) {
                nearest = Some((d, *pos_b));
            }
        }
        if let Some((_, pos)) = nearest {
            neighbor_by_id.insert(id_a.clone(), pos);
        }
    }

    // Pass 2: tick each actor with its assigned slot position. Falls
    // back to the slot's holding-ring position when this actor didn't
    // win a slot so it still has a sensible steering target.
    let combat_tuning = feel_tuning.feature_combat_tuning();
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
                npc.update(&feature_world, &player, dt);
                aabb.center = npc.pos;
                aabb.half_size = npc.size * 0.5;
            }
            ActorRuntime::Hostile(enemy) => {
                let slot_pos = if let Some(slot) = slot_board.0.slot_for(&enemy.id) {
                    Some(slot.world_pos(player.pos))
                } else if enemy.alive {
                    // No slot assigned — fall back to the per-actor
                    // holding-ring position computed above. Multiple
                    // unassigned actors of the same kind are spread
                    // round-robin across all holding positions of
                    // that kind rather than sharing slot 0.
                    holding_pos_by_id.get(&enemy.id).copied()
                } else {
                    None
                };
                let nearest_neighbor = neighbor_by_id.get(&enemy.id).copied();
                let mut outputs = super::enemies::EnemyTickOutputs::default();
                enemy.update(
                    &feature_world,
                    &player,
                    combat_tuning,
                    slot_pos,
                    nearest_neighbor,
                    &mut outputs,
                    dt,
                );
                aabb.center = enemy.pos;
                aabb.half_size = enemy.size * 0.5;
                // Flush projectile spawns this enemy emitted this tick.
                for spawn in outputs.projectile_spawns {
                    enemy_projectiles.spawn(spawn);
                }
                if player_vulnerable && enemy.alive {
                    if let Some(damage) = enemy.player_damage(player_body) {
                        let pos = damage.impact_pos;
                        sfx.write(crate::audio::SfxMessage::Play {
                            id: ambition_sfx::ids::PLAYER_DAMAGE,
                            pos,
                        });
                        vfx.write(VfxMessage::Impact { pos });
                        vfx.write(VfxMessage::Burst {
                            pos,
                            count: 14,
                            speed: 300.0,
                            color: [1.0, 0.34, 0.28, 0.88],
                            kind: ParticleKind::Shard,
                        });
                        debris.write(DebrisBurstMessage {
                            pos,
                            cue: PhysicsDebrisCue::Impact,
                        });
                        player_damage.write(damage);
                    }
                }
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
}

/// Handle interactions with ECS switches and peaceful NPCs. Chests stay in
/// `open_ecs_chests` because they have their own reward/persistence path.
pub fn interact_ecs_actors_and_switches(
    mut dialogue: ResMut<crate::dialog::DialogState>,
    mut next_mode: ResMut<NextState<crate::GameMode>>,
    mut banner: ResMut<GameplayBanner>,
    mut player: Query<
        (
            &crate::player::PlayerBody,
            &mut crate::player::PlayerInteractionState,
        ),
        With<crate::player::PlayerEntity>,
    >,
    actors: Query<(&FeatureAabb, &ActorRuntime), With<FeatureSimEntity>>,
    mut switches: Query<
        (
            &FeatureId,
            &FeatureName,
            &FeatureAabb,
            &SwitchFeature,
            &mut SwitchOn,
        ),
        With<FeatureSimEntity>,
    >,
    mut gameplay_effects: MessageWriter<GameplayEffect>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    let Ok((player_body, mut interaction)) = player.single_mut() else {
        return;
    };
    if !interaction.buffered() {
        return;
    }
    let player_body = player_body.aabb();
    for (aabb, actor) in &actors {
        let ActorRuntime::Peaceful(npc) = actor else {
            continue;
        };
        if !aabb.aabb().strict_intersects(player_body) {
            continue;
        }
        interaction.clear();
        banner.show(npc.message(), 2.6);
        let request = npc.dialogue_request();
        dialogue.start(&request.dialogue_id, &request.npc_name);
        next_mode.set(crate::GameMode::Dialogue);
        gameplay_effects.write(GameplayEffect::AdvanceQuest(
            ae::QuestAdvanceEvent::NpcTalked(npc.id.clone()),
        ));
        gameplay_effects.write(GameplayEffect::SetFlag {
            id: "met_any_hub_npc".into(),
            on: true,
        });
        gameplay_effects.write(GameplayEffect::SetFlag {
            id: format!("npc_{}_talked", request.dialogue_id),
            on: true,
        });
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
        interaction.clear();
        banner.show(format!("activated {}", name.0.as_str()), 2.6);
        on.0 = true;
        gameplay_effects.write(GameplayEffect::ActivateSwitch {
            activation: switch.activation.clone(),
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


#[cfg(test)]
mod tests;
