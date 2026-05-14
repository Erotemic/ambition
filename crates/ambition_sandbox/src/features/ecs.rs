//! ECS-native feature simulation.
//!
//! Authored and dynamic pickups, chests, breakables, switches, NPCs, enemies,
//! hazards, and bosses are spawned as Bevy entities and updated by the systems
//! in this module. This is the authoritative feature implementation.

use super::*;
use crate::audio::SfxMessage;
use crate::fx::{ParticleKind, VfxMessage};
use crate::physics::{DebrisBurstMessage, PhysicsDebrisCue};
use crate::rendering::RoomVisual;
use bevy::prelude::{Commands, Component, Entity, MessageReader, MessageWriter, NextState, Query, Res, ResMut, Resource, Time, With};

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

/// Collision contribution from ECS-owned breakables. Rebuilt before the main
/// sandbox tick and consumed by `world_with_sandbox_solids` anywhere the engine
/// needs the augmented collision world.
#[derive(Resource, Default, Clone, Debug)]
pub struct FeatureEcsWorldOverlay {
    pub blocks: Vec<ae::Block>,
}

/// Tick the gameplay banner resource once per frame.
pub fn tick_gameplay_banner(time: Res<Time>, mut banner: ResMut<GameplayBanner>) {
    banner.tick(time.delta_secs());
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
            commands.spawn((
                Name::new(format!("Feature boss: {}", object.name)),
                FeatureSimEntity,
                RoomVisual,
                FeatureId::new(object.id.clone()),
                FeatureName::new(object.name.clone()),
                FeatureAabb::from_center_size(boss.pos, boss.render_size()),
                BossFeature::new(boss),
            ));
        }
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
    commands.spawn((
        Name::new(format!("Encounter mob: {id}")),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(id.clone()),
        FeatureName::new(id),
        FeatureAabb::from_center_size(pos, size),
        ActorRuntime::Hostile(enemy),
        EncounterMob::new(encounter_id),
    ));
}

/// Despawn all ECS mobs owned by an encounter attempt.
pub fn despawn_encounter_mobs(
    commands: &mut Commands,
    mobs: &Query<(Entity, &EncounterMob, &FeatureId, &ActorRuntime)>,
    encounter_id: &str,
) {
    for (entity, mob, _, _) in mobs.iter() {
        if mob.encounter_id == encounter_id {
            commands.entity(entity).despawn();
        }
    }
}

/// Drop the encounter's ECS reward chest, if any, and clear its looted flag.
pub fn clear_encounter_reward_ecs(
    commands: &mut Commands,
    save: &mut ae::SandboxSaveData,
    chests: &Query<(Entity, &EncounterRewardChest, &FeatureId, Option<&Opened>), With<ChestFeature>>,
    encounter_id: &str,
) {
    for (entity, reward, _, _) in chests.iter() {
        if reward.encounter_id == encounter_id {
            commands.entity(entity).despawn();
        }
    }
    save.set_flag(crate::encounter::encounter_reward_looted_flag(encounter_id), false);
}

/// Idempotently ensure cleared mob encounters have an ECS reward chest.
pub fn sync_encounter_reward_chests_ecs(
    commands: &mut Commands,
    save: &ae::SandboxSaveData,
    registry: &crate::encounter::EncounterRegistry,
    chests: &Query<(Entity, &EncounterRewardChest, &FeatureId, Option<&Opened>), With<ChestFeature>>,
) {
    let chest_size = ae::Vec2::new(28.0, 28.0);
    for (encounter_id, state) in registry.encounters.iter() {
        if !matches!(state.phase, crate::encounter::EncounterPhase::Cleared) {
            continue;
        }
        let Some(spec) = state.spec.as_ref() else {
            continue;
        };
        let chest_id = format!("encounter_chest_{encounter_id}");
        let looted = save.flag(&crate::encounter::encounter_reward_looted_flag(encounter_id));
        let existing = chests
            .iter()
            .find(|(_, reward, _, _)| reward.encounter_id == *encounter_id);
        if let Some((entity, _, _, opened)) = existing {
            match (looted, opened.is_some()) {
                (true, false) => {
                    commands.entity(entity).insert(Opened);
                }
                (false, true) => {
                    commands.entity(entity).remove::<Opened>();
                }
                _ => {}
            }
            continue;
        }
        let chest_pos = crate::encounter::encounter_reward_chest_pos(spec, chest_size);
        let mut entity = commands.spawn((
            Name::new(format!("Encounter reward chest: {encounter_id}")),
            FeatureSimEntity,
            RoomVisual,
            FeatureId::new(chest_id.clone()),
            FeatureName::new(chest_id.clone()),
            FeatureAabb::from_center_size(chest_pos, chest_size),
            ChestFeature::new(ae::Chest::new(
                chest_id,
                Some(ae::PickupKind::Health { amount: 2 }),
            )),
            EncounterRewardChest::new(encounter_id.clone()),
        ));
        if looted {
            entity.insert(Opened);
        }
    }
}

/// Idempotently ensure cleared boss encounters have ECS reward chests.
/// Boss actors are ECS entities now; this helper receives their spawn anchors
/// from the boss encounter system and owns the reward chest entity/state natively.
pub fn sync_boss_reward_chests_ecs(
    commands: &mut Commands,
    save: &ae::SandboxSaveData,
    registry: &crate::boss_encounter::BossEncounterRegistry,
    world: &ae::World,
    boss_anchors: &[(String, ae::Vec2)],
    chests: &Query<(Entity, &BossRewardChest, &FeatureId, Option<&Opened>, Option<&FallingChest>), With<ChestFeature>>,
) {
    for (encounter_id, profile) in &registry.profiles {
        let crate::boss_encounter::BossRewardProfile::DropChest { pickup, offset, size } =
            &profile.reward
        else {
            continue;
        };
        if !matches!(save.boss(encounter_id), ae::PersistedEncounterState::Cleared) {
            continue;
        }
        let runtime_id = registry
            .runtime_ids
            .get(encounter_id)
            .cloned()
            .unwrap_or_else(|| encounter_id.clone());
        let Some((_, boss_spawn)) = boss_anchors.iter().find(|(id, _)| id == &runtime_id) else {
            continue;
        };
        let chest_id = format!("encounter_chest_{encounter_id}");
        let looted = save.flag(&crate::encounter::encounter_reward_looted_flag(encounter_id));
        let existing = chests
            .iter()
            .find(|(_, reward, _, _, _)| reward.encounter_id == *encounter_id);
        if let Some((entity, _, _, opened, falling)) = existing {
            match (looted, opened.is_some()) {
                (true, false) => {
                    commands.entity(entity).insert(Opened);
                }
                (false, true) => {
                    commands.entity(entity).remove::<Opened>();
                }
                _ => {}
            }
            if looted && falling.is_some() {
                commands.entity(entity).remove::<FallingChest>();
            }
            continue;
        }
        let mut chest_pos = *boss_spawn + *offset;
        if looted {
            chest_pos = settled_chest_center(world, chest_pos, *size);
        }
        let mut entity = commands.spawn((
            Name::new(format!("Boss reward chest: {encounter_id}")),
            FeatureSimEntity,
            RoomVisual,
            FeatureId::new(chest_id.clone()),
            FeatureName::new(chest_id.clone()),
            FeatureAabb::from_center_size(chest_pos, *size),
            ChestFeature::new(ae::Chest::new(chest_id, Some(pickup.clone()))),
            BossRewardChest::new(encounter_id.clone()),
        ));
        if looted {
            entity.insert(Opened);
        } else {
            entity.insert(FallingChest::new(0.0));
        }
    }
}

/// Tick ECS reward chests that are still falling to the floor.
pub fn update_ecs_falling_chests(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<crate::GameWorld>,
    mut chests: Query<(Entity, &mut FeatureAabb, &mut FallingChest), With<ChestFeature>>,
) {
    let dt = time.delta_secs();
    for (entity, mut aabb, mut falling) in &mut chests {
        falling.vel_y = (falling.vel_y + CHEST_FALL_GRAVITY * dt).min(CHEST_FALL_MAX_SPEED);
        let step = falling.vel_y * dt;
        if step <= 0.0 {
            continue;
        }
        let max_substep = aabb.half_size.y.max(2.0);
        let mut remaining = step;
        while remaining > 0.0 {
            let advance = remaining.min(max_substep);
            let try_center = ae::Vec2::new(aabb.center.x, aabb.center.y + advance);
            let try_aabb = ae::Aabb::new(try_center, aabb.half_size);
            let blocked = world.0.body_overlaps_any(try_aabb, |block| {
                matches!(
                    block.kind,
                    ae::BlockKind::Solid | ae::BlockKind::OneWay | ae::BlockKind::BlinkWall { .. }
                )
            });
            if blocked {
                commands.entity(entity).remove::<FallingChest>();
                break;
            }
            aabb.center = try_center;
            remaining -= advance;
        }
    }
}

fn settled_chest_center(world: &ae::World, start: ae::Vec2, size: ae::Vec2) -> ae::Vec2 {
    let mut center = start;
    let half_size = size * 0.5;
    let mut vel_y: f32 = 0.0;
    let virtual_dt = 1.0 / 60.0;
    for _ in 0..240 {
        vel_y = (vel_y + CHEST_FALL_GRAVITY * virtual_dt).min(CHEST_FALL_MAX_SPEED);
        let step = vel_y * virtual_dt;
        if step <= 0.0 {
            continue;
        }
        let max_substep = half_size.y.max(2.0);
        let mut remaining = step;
        while remaining > 0.0 {
            let advance = remaining.min(max_substep);
            let try_center = ae::Vec2::new(center.x, center.y + advance);
            let try_aabb = ae::Aabb::new(try_center, half_size);
            let blocked = world.body_overlaps_any(try_aabb, |block| {
                matches!(
                    block.kind,
                    ae::BlockKind::Solid | ae::BlockKind::OneWay | ae::BlockKind::BlinkWall { .. }
                )
            });
            if blocked {
                return center;
            }
            center = try_center;
            remaining -= advance;
        }
    }
    center
}


/// Reset ECS-owned static feature state after a same-room sandbox reset.
pub fn reset_ecs_room_features(
    mut commands: Commands,
    mut reset_requests: MessageReader<ResetRoomFeaturesEvent>,
    collected_pickups: Query<Entity, (With<FeatureSimEntity>, With<Collected>)>,
    opened_chests: Query<Entity, (With<FeatureSimEntity>, With<Opened>)>,
    mut breakables: Query<(Entity, &mut BreakableFeature, Option<&mut StandTimer>), With<FeatureSimEntity>>,
    mut actors: Query<(&mut FeatureAabb, &mut ActorRuntime), With<FeatureSimEntity>>,
    mut switches: Query<&mut SwitchOn, With<SwitchFeature>>,
    mut bosses: Query<&mut BossFeature, With<FeatureSimEntity>>,
    mut hazards: Query<&mut HazardFeature, With<FeatureSimEntity>>,
) {
    if reset_requests.read().next().is_none() {
        return;
    }

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
    mut banner: ResMut<GameplayBanner>,
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
        banner.show(format!("picked up {}", name.0.as_str()), 2.6);
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
    mut banner: ResMut<GameplayBanner>,
    chests: Query<(
        Entity,
        &FeatureId,
        &FeatureName,
        &FeatureAabb,
        Option<&Opened>,
        Option<&FallingChest>,
    ), (With<FeatureSimEntity>, With<ChestFeature>)>,
    mut gameplay_effects: MessageWriter<GameplayEffect>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    if runtime.interact_buffer_timer <= 0.0 {
        return;
    }
    let player_body = runtime.player.aabb();
    for (entity, id, name, aabb, opened, falling) in &chests {
        if falling.is_some() || opened.is_some() || !aabb.aabb().strict_intersects(player_body) {
            continue;
        }
        commands.entity(entity).insert(Opened);
        runtime.clear_interact_buffer();
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
    time: Res<Time>,
    runtime: Res<crate::SandboxRuntime>,
    mut banner: ResMut<GameplayBanner>,
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

/// Apply typed slash/projectile/pogo damage messages to ECS feature targets.
pub fn apply_feature_damage_events(
    mut commands: Commands,
    mut damage_events: MessageReader<DamageEvent>,
    mut pogo_bounces: MessageReader<PogoBounceEvent>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    mut banner: ResMut<GameplayBanner>,
    mut breakables: Query<(Entity, &FeatureId, &FeatureName, &FeatureAabb, &mut BreakableFeature), With<FeatureSimEntity>>,
    mut actors: Query<(&FeatureId, &FeatureAabb, &mut ActorRuntime), With<FeatureSimEntity>>,
    mut bosses: Query<(&FeatureId, &FeatureAabb, &mut BossFeature), With<FeatureSimEntity>>,
    mut gameplay_effects: MessageWriter<GameplayEffect>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
) {
    for event in damage_events.read().cloned() {
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
                        banner.show(format!("{} turns hostile", npc.name), 2.6);
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
                            banner.show(format!("{} dropped; respawning", enemy.name), 2.6);
                        } else {
                            banner.show(format!("defeated {}", enemy.name), 2.6);
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
        let mut boss_hit_this_event = false;
        for (id, aabb, mut feature) in &mut bosses {
            let key = format!("boss:{}", id.as_str());
            if event.ignored_targets.iter().any(|ignored| ignored == &key) {
                continue;
            }
            let boss = &mut feature.boss;
            if !boss.alive || !event.volume.strict_intersects(aabb.aabb()) {
                continue;
            }
            boss.hit_flash = 0.18;
            let amount = event.damage.max(1);
            let killed = boss.health.damage(amount);
            let impact = midpoint(event.volume.center(), boss.pos);
            vfx.write(VfxMessage::Impact { pos: impact });
            gameplay_effects.write(GameplayEffect::DamageBoss {
                boss_id: boss.id.clone(),
                amount,
            });
            boss_hit_this_event = true;
            if killed {
                boss.alive = false;
                banner.show(format!("defeated boss {}", boss.name), 2.6);
                vfx.write(VfxMessage::Burst {
                    pos: boss.pos,
                    count: 16,
                    speed: 230.0,
                    color: [0.84, 0.95, 1.0, 0.82],
                    kind: ParticleKind::Spark,
                });
                debris.write(DebrisBurstMessage {
                    pos: boss.pos,
                    cue: PhysicsDebrisCue::BossRagdoll,
                });
                sfx.write(SfxMessage::Death { pos: boss.pos });
            }
        }

        if actor_hit_this_event || boss_hit_this_event {
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
                banner.show(format!("broke {}", name.0.as_str()), 2.6);
                emit_breakable_destroyed(aabb.center, &mut sfx, &mut vfx, &mut debris);
            }
        }
    }

    for event in pogo_bounces.read() {
        let orb_aabb = event.orb_aabb;
        let damage = event.damage;
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
                banner.show(format!("shattered {}", name.0.as_str()), 2.6);
                emit_breakable_destroyed(aabb.center, &mut sfx, &mut vfx, &mut debris);
            }
        }
    }
}


/// Tick ECS-authored hazards and publish player damage through Bevy messages.
pub fn update_ecs_hazards(
    time: Res<Time>,
    runtime: Res<crate::SandboxRuntime>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<crate::fx::VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut player_damage: MessageWriter<PlayerDamageEvent>,
    mut hazards: Query<(&FeatureName, &mut FeatureAabb, &mut HazardFeature), With<FeatureSimEntity>>,
) {
    let dt = time.delta_secs();
    let player_body = runtime.player.aabb();
    let player_vulnerable = !runtime.player.invincible && runtime.damage_invuln_timer <= 0.0;
    for (_name, mut aabb, mut feature) in &mut hazards {
        let hazard = &mut feature.hazard;
        hazard.update(dt);
        aabb.center = hazard.pos;
        aabb.half_size = hazard.size * 0.5;
        if !player_vulnerable || !hazard.active() || !hazard.aabb().strict_intersects(player_body) {
            continue;
        }
        let pos = runtime.player.pos;
        let knockback_dir = (pos.x - hazard.pos.x).signum();
        vfx.write(VfxMessage::Impact { pos });
        vfx.write(VfxMessage::Burst {
            pos,
            count: 14,
            speed: 300.0,
            color: [1.0, 0.34, 0.28, 0.88],
            kind: ParticleKind::Shard,
        });
        debris.write(DebrisBurstMessage { pos, cue: PhysicsDebrisCue::Impact });
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
    time: Res<Time>,
    world: Res<crate::GameWorld>,
    runtime: Res<crate::SandboxRuntime>,
    feel_tuning: Res<crate::feel::SandboxFeelTuning>,
    overlay: Res<FeatureEcsWorldOverlay>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<crate::fx::VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut player_damage: MessageWriter<PlayerDamageEvent>,
    mut bosses: Query<(&mut FeatureAabb, &mut BossFeature), With<FeatureSimEntity>>,
) {
    let dt = time.delta_secs();
    let feature_world = world_with_sandbox_solids(
        &world.0,
        &runtime.moving_platforms,
        &overlay,
    );
    let player = runtime.player.clone();
    let player_body = player.aabb();
    let player_vulnerable = !runtime.player.invincible && runtime.damage_invuln_timer <= 0.0;
    for (mut aabb, mut feature) in &mut bosses {
        let boss = &mut feature.boss;
        boss.update(&feature_world, &player, feel_tuning.feature_combat_tuning(), dt);
        aabb.center = boss.pos;
        aabb.half_size = boss.render_size() * 0.5;
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
                debris.write(DebrisBurstMessage { pos, cue: PhysicsDebrisCue::Impact });
                player_damage.write(damage);
            }
        }
    }
}


/// Tick ECS actors. Peaceful and hostile actors share the same entity identity
/// and can switch disposition in-place; dynamic encounter-spawned mobs use the
/// same `ActorRuntime::Hostile` path with an `EncounterMob` marker.
pub fn update_ecs_actors(
    time: Res<Time>,
    world: Res<crate::GameWorld>,
    runtime: Res<crate::SandboxRuntime>,
    feel_tuning: Res<crate::feel::SandboxFeelTuning>,
    overlay: Res<FeatureEcsWorldOverlay>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<crate::fx::VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut player_damage: MessageWriter<PlayerDamageEvent>,
    mut actors: Query<(&mut FeatureAabb, &mut ActorRuntime), With<FeatureSimEntity>>,
) {
    let dt = time.delta_secs();
    let feature_world = world_with_sandbox_solids(
        &world.0,
        &runtime.moving_platforms,
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
                        debris.write(DebrisBurstMessage { pos, cue: PhysicsDebrisCue::Impact });
                        player_damage.write(damage);
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
    mut banner: ResMut<GameplayBanner>,
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
        banner.show(npc.message(), 2.6);
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
        banner.show(format!("activated {}", name.0.as_str()), 2.6);
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
/// Provoked NPCs load as hostile actors, and persisted non-respawning enemy
/// deaths stay dead across room reloads. Dynamic encounter mobs are ignored
/// because their lifecycle belongs to encounter state.
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


/// Mirror persisted boss-cleared state onto ECS-owned boss actors.
pub fn sync_ecs_bosses_with_save(
    save: Res<crate::save::SandboxSave>,
    mut bosses: Query<&mut BossFeature, With<FeatureSimEntity>>,
) {
    let data = save.data();
    for mut feature in &mut bosses {
        let boss = &mut feature.boss;
        let encounter_id = crate::boss_encounter::encounter_id_from_name(&boss.name);
        if matches!(data.boss(&encounter_id), ae::PersistedEncounterState::Cleared)
            || matches!(data.boss(&boss.id), ae::PersistedEncounterState::Cleared)
        {
            boss.alive = false;
            boss.health.current = 0;
        }
    }
}

/// Mirror persisted save switch state onto ECS switch components.
///
/// Encounter arming now reads `EncounterSwitchIndex`, which is rebuilt from
/// these ECS components.
pub fn sync_ecs_switches_from_save(
    save: Res<crate::save::SandboxSave>,
    mut switches: Query<(&FeatureId, &mut SwitchOn), With<SwitchFeature>>,
) {
    for (id, mut switch_on) in &mut switches {
        switch_on.0 = save.data().switch(id.as_str());
    }
}


/// Read-only hit test used by systems that need immediate projectile / attack
/// feedback while damage application is still drained through
/// typed Bevy messages.
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

pub fn ecs_damage_event_hits_boss(
    event: &DamageEvent,
    bosses: &Query<(&FeatureId, &FeatureAabb, &BossFeature), With<FeatureSimEntity>>,
) -> bool {
    bosses.iter().any(|(id, aabb, feature)| {
        let key = format!("boss:{}", id.as_str());
        !event.ignored_targets.iter().any(|ignored| ignored == &key)
            && feature.boss.alive
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
    hazards: &Query<(&FeatureId, &FeatureAabb, &HazardFeature)>,
    bosses: &Query<(&FeatureId, &BossFeature)>,
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
    for (feature_id, aabb, hazard) in hazards.iter() {
        if feature_id.as_str() == id {
            return Some(FeatureView {
                pos: hazard.hazard.pos,
                size: aabb.size(),
                kind: FeatureVisualKind::Hazard,
                visible: hazard.hazard.active(),
                flash: false,
                switch_on: false,
            });
        }
    }
    for (feature_id, boss) in bosses.iter() {
        if feature_id.as_str() == id {
            let boss = &boss.boss;
            return Some(FeatureView {
                pos: boss.pos,
                size: boss.render_size(),
                kind: FeatureVisualKind::Boss,
                visible: boss.alive,
                flash: boss.hit_flash > 0.0
                    || boss.attack_windup_timer > 0.0
                    || boss.attack_timer > 0.0,
                switch_on: false,
            });
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


pub fn ecs_boss_name<'a>(id: &str, bosses: &'a Query<(&FeatureId, &BossFeature)>) -> Option<&'a str> {
    bosses.iter().find_map(|(feature_id, boss)| {
        (feature_id.as_str() == id).then(|| boss.boss.name.as_str())
    })
}

pub fn ecs_boss_anim_state(
    id: &str,
    bosses: &Query<(&FeatureId, &BossFeature)>,
) -> Option<crate::boss_sprites::BossAnimState> {
    bosses.iter().find_map(|(feature_id, boss)| {
        if feature_id.as_str() != id {
            return None;
        }
        let boss = &boss.boss;
        Some(crate::boss_sprites::BossAnimState {
            alive: boss.alive,
            attack_active: boss.attack_timer > 0.0,
            attack_windup: boss.attack_windup_timer > 0.0,
            hit_flash: boss.hit_flash > 0.0,
            pattern_timer: boss.pattern_timer,
        })
    })
}
