//! Module-owned entities (fast-iteration I7): the adapters of
//! `ambition.world.spawn_module_entity` and `ambition.world.module_entity_tick`.
//! See the port cards in `ambition_combat_port::module_entity`.
//!
//! The world owns a module entity's identity, side, team, session,
//! presentation source, position and lifetime. What the entity does each tick
//! is the module's: the sentry turret is the first one
//! (`ambition_content_modules::sentry`).

use ambition_combat::components::CenteredAabb;
use ambition_characters::actor::ActorFaction;
use ambition_combat_port::{
    ContactBody, EndModuleEntityPort, EndOwnedEntityPort, ItemState, ModuleEntitySpawn, ModuleEntityTick,
    ModuleEntityTickPort, ModuleItemSpawn, MoveModuleEntityPort, OwnedEntities, OwnedEntity, SpawnModuleEntityPort,
    SpawnModuleItemPort, SpawnerBody, SpecialParam,
};
use ambition_extension_host::{AdmittedExtensions, ExtensionInvocations, ExtensionOutbox, LowersIn};
use ambition_extension_sdk::phases::MODULE_ENTITY_TICK;
use ambition_extension_sdk::Port;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{
    FeatureSimEntity, SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_shared_tangle::sim_id::{SimId, SimIdCounter};
use ambition_platformer2d_shared_tangle::sim_selection::winner_by;
use bevy::prelude::*;

/// The pull port, named here for the composition that places its lowering in
/// the body-path `Carry` set.
pub use ambition_combat_port::PullBodiesPort;

/// An entity a module asked for. The module that ticks it is the one bound to
/// its `kind`.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct ModuleEntity {
    pub kind: String,
    pub pos: ae::Vec2,
    pub remaining_s: f32,
    /// The box that bodies touch; zero is a point, which no body touches.
    pub half_extents: ae::Vec2,
    /// The spawn's authored values, given back to the module at each tick.
    /// They do not change, so the rollback probe leaves them out.
    pub params: Vec<(String, SpecialParam)>,
    /// The identity of the body that spawned it, which owns it however it
    /// changes hands (`ambition.world.owned_entities`). Constant.
    pub spawner: Option<SimId>,
    /// Gameplay seconds it has lived, counted up as it ages: not derived from
    /// its lifetime, which may be too long for a step to register on.
    pub age_s: f32,
}

impl ModuleEntity {
    /// An entity that no body touches and that has no params (a turret, a
    /// well).
    pub fn point(kind: impl Into<String>, pos: ae::Vec2, remaining_s: f32) -> Self {
        Self {
            kind: kind.into(),
            pos,
            remaining_s,
            half_extents: ae::Vec2::ZERO,
            params: Vec::new(),
            spawner: None,
            age_s: 0.0,
        }
    }
}

/// The touchers of each module entity of this tick, in the order its
/// invocation gave them: a launch names a toucher by its index here.
#[derive(Resource, Default, Debug)]
pub struct ModuleEntityTouchers(pub Vec<(Entity, Vec<Entity>)>);

/// What a module entity takes from its spawner, frozen at the spawn: the
/// entity outlives the spawner.
pub struct Spawner {
    pub scope: SessionSpawnScope,
    /// The spawner's EFFECTIVE side (`targeting::effective_faction`): a
    /// possessed NPC keeps `ActorFaction::Enemy` and fights for its driver.
    pub side: ActorFaction,
    pub team: Option<ambition_combat::targeting::MatchTeam>,
    pub presentation: Option<ambition_sfx::PresentationSourceId>,
    /// The entity's identity, minted from the spawner's. Things the entity
    /// spawns (a bolt) mint under it.
    pub id: SimId,
}

/// Put one module entity in the world. The only way one enters the world, so
/// a fixture builds the entity production builds.
///
/// The entity carries its spawner's side: a projectile it owns takes its
/// allegiance from the owner, and with no `ActorFaction` it hits nothing.
pub fn spawn_module_entity(commands: &mut Commands, entity: ModuleEntity, from: Spawner) -> Entity {
    let name = Name::new(format!("Module entity {}", entity.kind));
    // ROOM-OWNED AS WELL AS SESSION-OWNED. The spawner's stamp says which
    // live room the entity is in; `RoomScopedEntity` says that room's
    // retirement owns its lifetime. Without it, a retired room left the
    // entity ticking until its timer ran out. The spawner is not the owner:
    // the entity outlives it while its room is live.
    let mut spawned = commands.spawn_room_in_session(from.scope, (entity, name, from.side, from.id));
    if let Some(team) = from.team {
        spawned.insert(team);
    }
    if let Some(source) = from.presentation {
        spawned.insert(ambition_sfx::BodyPresentationSource(source));
    }
    spawned.id()
}

/// Lower `ambition.world.spawn_module_entity` in the phase `L`: mint the
/// entity's identity from the spawner, and spawn it with the spawner's side.
pub fn lower_module_entity_spawns<L: LowersIn>(
    mut outbox: ResMut<ExtensionOutbox>,
    mut spawners: Query<(
        Option<&SimId>,
        Option<&mut SimIdCounter>,
        Option<&SessionScopedEntity>,
        Option<&ActorFaction>,
        Option<&ambition_characters::control::DrivingParticipant>,
        Option<&ambition_combat::targeting::MatchTeam>,
    )>,
    sfx: ambition_sfx::BodySfxWriter,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mut commands: Commands,
) {
    for submitted in outbox.drain::<SpawnModuleEntityPort>(&L::PHASE) {
        let Ok((id, counter, session, side, driver, team)) = spawners.get_mut(submitted.scope) else {
            continue;
        };
        // ADR 0030: a dynamic entity that cannot name its spawner does not
        // spawn, and the things it spawns could not mint under it.
        let (Some(id), Some(mut counter)) = (id, counter) else {
            warn!(
                "extension entry {} asked {:?}, which has no SimId or no SimIdCounter, to \
                 spawn a {:?}; refused",
                submitted.entry, submitted.scope, submitted.value.kind
            );
            continue;
        };
        let ModuleEntitySpawn { kind, at, lifetime_s, half_extents, params } = submitted.value;
        spawn_module_entity(
            &mut commands,
            ModuleEntity {
                kind,
                pos: ae::Vec2::from(at),
                remaining_s: lifetime_s,
                half_extents: ae::Vec2::from(half_extents),
                params,
                spawner: Some(id.clone()),
                age_s: 0.0,
            },
            Spawner {
                // In its spawner's live room, as a shot is in its owner's.
                scope: SessionSpawnScope::new(session.map(|s| s.0)).in_room(rooms.stamped(submitted.scope)),
                side: ambition_combat::targeting::effective_faction(
                    side.copied().unwrap_or(ActorFaction::Player),
                    driver,
                ),
                team: team.cloned(),
                presentation: sfx.source_of(submitted.scope),
                id: SimId::spawned(id, counter.next()),
            },
        );
    }
}

/// Age every module entity by this tick's gameplay step, remove the ones whose
/// lifetime is over, and queue one tick for each living entity of a bound
/// kind.
///
/// Every module entity ages, bound or not: an entity whose module was taken
/// away by a reload still goes at the end of its lifetime.
///
/// The queue order is a gameplay decision: two entities that fire on one tick
/// write two projectile requests, and the projectile domain gives identities
/// in request order. So it is the entities' own state, then identity.
pub fn queue_module_entity_ticks(
    admitted: Res<AdmittedExtensions>,
    world_time: Res<ambition_time::WorldTime>,
    mut invocations: ResMut<ExtensionInvocations>,
    mut touchers_table: ResMut<ModuleEntityTouchers>,
    mut commands: Commands,
    mut entities: Query<(Entity, &mut ModuleEntity, Option<&SimId>)>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    bodies: Query<
        (
            Entity,
            &CenteredAabb,
            &ActorFaction,
            Option<&ambition_characters::actor::BodyHealth>,
            (
                Has<ambition_combat::death_rules::OutOfPlay>,
                Option<&ambition_platformer2d_core::DepthPlane>,
            ),
            Option<&SimId>,
            Option<&ambition_characters::control::DrivingParticipant>,
            Option<&ae::BodyKinematics>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    touchers_table.0.clear();
    let dt = world_time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    let bound: Vec<&str> = admitted
        .0
        .entries
        .iter()
        .filter(|e| e.descriptor.trigger.port == ModuleEntityTickPort::KEY)
        .map(|e| e.descriptor.trigger.selector.as_ref())
        .collect();
    let mut living = Vec::new();
    for (entity, mut module_entity, id) in &mut entities {
        module_entity.remaining_s -= dt;
        module_entity.age_s += dt;
        if module_entity.remaining_s <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        if bound.contains(&module_entity.kind.as_str()) {
            living.push((module_entity.pos, module_entity.remaining_s, id.cloned(), entity));
        }
    }
    if living.is_empty() {
        return;
    }
    living.sort_by(|a, b| {
        a.0.x
            .total_cmp(&b.0.x)
            .then_with(|| a.0.y.total_cmp(&b.0.y))
            .then_with(|| a.1.total_cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| a.3.cmp(&b.3))
    });
    // The bodies that can be targeted: effective side `Enemy` (a possessed
    // NPC keeps `ActorFaction::Enemy` and fights for its driver), and not
    // untouchable (a corpse, a body out of play or behind the playable plane).
    let enemies: Vec<_> = bodies
        .iter()
        .filter(|(_, _, side, health, (out_of_play, plane), _, driver, _)| {
            ambition_combat::targeting::effective_faction(**side, *driver) == ActorFaction::Enemy
                && !ambition_combat::util::body_is_untouchable(*health, *out_of_play, *plane)
        })
        .map(|(body, aabb, _, _, _, id, _, _)| (aabb.center, id, rooms.of(body)))
        .collect();
    // The bodies that can touch an entity: any side, and not untouchable. In
    // identity order, so a rewind gives a module the same first toucher.
    let mut touchable: Vec<_> = bodies
        .iter()
        .filter(|(_, _, _, health, (out_of_play, plane), _, _, _)| {
            !ambition_combat::util::body_is_untouchable(*health, *out_of_play, *plane)
        })
        .map(|(body, aabb, _, _, _, id, _, kin)| (id.cloned(), body, aabb.center, aabb.half_size, kin.map(|k| k.vel), rooms.of(body)))
        .collect();
    touchable.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    for (pos, remaining_s, _, entity) in living {
        let Ok((_, module_entity, _)) = entities.get(entity) else {
            continue;
        };
        let (kind, half_extents, params) =
            (module_entity.kind.clone(), module_entity.half_extents, module_entity.params.clone());
        // Nearest, with a named tie-break: equidistant bodies are common, and
        // query order must not decide. Only a body in the entity's own live
        // room: two entities meet only when they are in one room.
        let room = rooms.of(entity);
        let nearest_enemy = winner_by(
            enemies.iter().filter(|(_, _, body_room)| *body_room == room),
            |(center, _, _)| center.distance_squared(pos),
            |(_, id, _)| *id,
        )
        .map(|(center, _, _)| [center.x, center.y]);
        // A body touches the entity's box when the boxes meet, edges included:
        // a body that stands on a plate meets it.
        let mut touched_by = Vec::new();
        let mut touchers = Vec::new();
        if half_extents != ae::Vec2::ZERO {
            for (_, body, center, half, vel, body_room) in &touchable {
                let reach = half_extents + *half;
                let offset = (*center - pos).abs();
                if *body_room != room || offset.x > reach.x || offset.y > reach.y {
                    continue;
                }
                touched_by.push(*body);
                touchers.push(ContactBody {
                    position: [center.x, center.y],
                    size: [half.x * 2.0, half.y * 2.0],
                    velocity: vel.map_or([0.0; 2], |v| [v.x, v.y]),
                });
            }
        }
        touchers_table.0.push((entity, touched_by));
        invocations.trigger::<ModuleEntityTickPort>(
            &MODULE_ENTITY_TICK,
            kind,
            entity,
            None,
            false,
            ModuleEntityTick {
                position: [pos.x, pos.y],
                remaining_s,
                nearest_enemy,
                params,
                touchers,
            },
        );
    }
}

/// Lower `ambition.world.spawn_module_item` in `technique_execution`: a
/// module entity, as `lower_module_entity_spawns` spawns one, that is also a
/// held item at rest on the stage.
pub fn lower_module_item_spawns(
    mut outbox: ResMut<ExtensionOutbox>,
    mut spawners: Query<(
        Option<&SimId>,
        Option<&mut SimIdCounter>,
        Option<&SessionScopedEntity>,
        Option<&ActorFaction>,
        Option<&ambition_characters::control::DrivingParticipant>,
        Option<&ambition_combat::targeting::MatchTeam>,
    )>,
    sfx: ambition_sfx::BodySfxWriter,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mut commands: Commands,
) {
    for submitted in outbox.drain::<SpawnModuleItemPort>(&ambition_extension_sdk::phases::TECHNIQUE_EXECUTION) {
        let ModuleItemSpawn { kind, item, at, lifetime_s, half_extents, params } = submitted.value;
        let Some(held) = ambition_characters::brain::held_item_by_id(&item) else {
            warn!(
                "extension entry {} asked for a {kind:?} that is the item {item:?}, which this game does not \
                 register; refused",
                submitted.entry
            );
            continue;
        };
        let Ok((id, counter, session, side, driver, team)) = spawners.get_mut(submitted.scope) else {
            continue;
        };
        let (Some(id), Some(mut counter)) = (id, counter) else {
            warn!(
                "extension entry {} asked {:?}, which has no SimId or no SimIdCounter, to spawn a {kind:?}; refused",
                submitted.entry, submitted.scope
            );
            continue;
        };
        let pos = ae::Vec2::from(at);
        let spawned = spawn_module_entity(
            &mut commands,
            ModuleEntity {
                kind,
                pos,
                remaining_s: lifetime_s,
                // No body touches it as a module entity; it is touched as an
                // item.
                half_extents: ae::Vec2::ZERO,
                params,
                spawner: Some(id.clone()),
                age_s: 0.0,
            },
            Spawner {
                scope: SessionSpawnScope::new(session.map(|s| s.0)).in_room(rooms.stamped(submitted.scope)),
                side: ambition_combat::targeting::effective_faction(
                    side.copied().unwrap_or(ActorFaction::Player),
                    driver,
                ),
                team: team.cloned(),
                presentation: sfx.source_of(submitted.scope),
                id: SimId::spawned(id, counter.next()),
            },
        );
        commands
            .entity(spawned)
            .insert(ambition_held_items::GroundItem::at_rest(held, pos, ae::Vec2::from(half_extents)));
    }
}

/// Keep each module entity that is an item where the item is, whoever holds
/// it. Runs before the entities are ticked, after the items moved.
pub fn follow_module_items(
    mut entities: Query<(&mut ModuleEntity, &ambition_held_items::GroundItem, &ambition_held_items::ItemCustody)>,
    where_it_is: ambition_held_items::ItemWorldPos,
) {
    for (mut entity, item, custody) in &mut entities {
        let at = where_it_is.of(custody, item);
        if entity.pos != at {
            entity.pos = at;
        }
    }
}

/// The item `scope` is: whether it is loose, and the hardest it struck.
/// Absent for a scope that is not an item.
pub fn item_state_of(world: &World, scope: Entity) -> Option<ItemState> {
    let custody = world.get::<ambition_held_items::ItemCustody>(scope)?;
    world.get::<ambition_held_items::GroundItem>(scope)?;
    let settled = world.get::<ambition_held_items::SettledItem>(scope).map_or(0.0, |settled| settled.impact_speed);
    let struck = world.get::<ambition_held_items::ItemStruckBody>(scope).map_or(0.0, |hit| hit.impact_speed);
    Some(ItemState { in_world: custody.in_world(), impact_speed: settled.max(struck) })
}

/// The module entities `scope` spawned, in identity order.
pub fn owned_entities_of(world: &World, scope: Entity) -> Option<OwnedEntities> {
    let Some(spawner) = world.get::<SimId>(scope) else {
        return Some(OwnedEntities::default());
    };
    let mut owned: Vec<(&SimId, OwnedEntity)> = world
        .iter_entities()
        .filter_map(|entity| {
            let module_entity = entity.get::<ModuleEntity>()?;
            let id = entity.get::<SimId>()?;
            (module_entity.spawner.as_ref() == Some(spawner)).then(|| {
                (
                    id,
                    OwnedEntity {
                        kind: module_entity.kind.clone(),
                        position: [module_entity.pos.x, module_entity.pos.y],
                        age_s: module_entity.age_s,
                        handle: ambition_combat::extension::blast::credit_of(id),
                    },
                )
            })
        })
        .collect();
    owned.sort_by(|a, b| a.0.cmp(b.0));
    Some(OwnedEntities { entities: owned.into_iter().map(|(_, owned)| owned).collect() })
}

/// The body that spawned `scope`: its credit and its player's stick, or
/// zeros when it is not in the world. Absent for a scope that is not a
/// module entity.
pub fn spawner_body_of(world: &World, scope: Entity) -> Option<SpawnerBody> {
    let module_entity = world.get::<ModuleEntity>(scope)?;
    let Some(spawner) = module_entity.spawner.as_ref() else {
        return Some(SpawnerBody::default());
    };
    let body = world.iter_entities().find(|entity| entity.get::<SimId>() == Some(spawner));
    Some(body.map_or_else(SpawnerBody::default, |body| SpawnerBody {
        credit: ambition_combat::extension::blast::credit_of(spawner),
        stick: body
            .get::<ambition_characters::control::ActorControl>()
            .map_or([0.0; 2], |control| {
                let stick = control.0.steer_axis().vec();
                [stick.x, stick.y]
            }),
    }))
}

/// The credit of each toucher of `scope`'s tick, in its tick's order.
/// Absent for a scope that was not ticked.
pub fn toucher_credits_of(world: &World, scope: Entity) -> Option<Vec<u64>> {
    let (_, touchers) = world.resource::<ModuleEntityTouchers>().0.iter().find(|(entity, _)| *entity == scope)?;
    Some(
        touchers
            .iter()
            .map(|toucher| world.get::<SimId>(*toucher).map_or(0, ambition_combat::extension::blast::credit_of))
            .collect(),
    )
}

/// Lower `ambition.world.move_module_entity`: the entity is where its module
/// moved it. An item entity is where its item is, so it is not moved.
pub fn lower_module_entity_moves(
    mut outbox: ResMut<ExtensionOutbox>,
    mut entities: Query<&mut ModuleEntity, Without<ambition_held_items::GroundItem>>,
) {
    for submitted in outbox.drain::<MoveModuleEntityPort>(&MODULE_ENTITY_TICK) {
        let to = ae::Vec2::from(submitted.value.to);
        if !to.is_finite() {
            warn!("extension entry {} asked to move {:?} to {to:?}; refused", submitted.entry, submitted.scope);
            continue;
        }
        match entities.get_mut(submitted.scope) {
            Ok(mut entity) => entity.pos = to,
            Err(_) => warn!(
                "extension entry {} asked to move {:?}, which is not a module entity that is not an item; refused",
                submitted.entry, submitted.scope
            ),
        }
    }
}

/// Lower `ambition.world.end_owned_entity`: remove the module entity a
/// handle names, when the scope spawned it.
pub fn lower_owned_entity_ends(
    mut outbox: ResMut<ExtensionOutbox>,
    spawners: Query<&SimId>,
    entities: Query<(Entity, &ModuleEntity, &SimId)>,
    mut commands: Commands,
) {
    for submitted in outbox.drain::<EndOwnedEntityPort>(&ambition_extension_sdk::phases::TECHNIQUE_EXECUTION) {
        let spawner = spawners.get(submitted.scope).ok();
        let named = entities.iter().find(|(_, entity, id)| {
            spawner.is_some()
                && entity.spawner.as_ref() == spawner
                && ambition_combat::extension::blast::credit_of(id) == submitted.value.handle
        });
        match named {
            Some((entity, ..)) => {
                commands.entity(entity).despawn();
            }
            None => warn!(
                "extension entry {} asked to end the entity {:#x}, which {:?} did not spawn; refused",
                submitted.entry, submitted.value.handle, submitted.scope
            ),
        }
    }
}

/// Lower `ambition.world.end_module_entity`: remove the module entity the
/// invocation ran for.
pub fn lower_module_entity_ends(
    mut outbox: ResMut<ExtensionOutbox>,
    entities: Query<(), With<ModuleEntity>>,
    mut commands: Commands,
) {
    for submitted in outbox.drain::<EndModuleEntityPort>(&ambition_extension_sdk::phases::MODULE_ENTITY_TICK) {
        if entities.contains(submitted.scope) {
            commands.entity(submitted.scope).despawn();
        } else {
            warn!(
                "extension entry {} asked to end {:?}, which is not a module entity; refused",
                submitted.entry, submitted.scope
            );
        }
    }
}

/// Lower `ambition.world.pull_bodies`: each pull, in request order, moves
/// every reached body toward its centre (see the port card). Runs on the
/// gameplay step, so bullet-time slows the pull with everything else.
///
/// Two pulls do not commute (each closes a fraction of ITS gap), so the
/// request order decides; it is the module-entity tick's order.
pub fn lower_body_pulls(
    mut outbox: ResMut<ExtensionOutbox>,
    world_time: Res<ambition_time::WorldTime>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mut bodies: Query<
        (
            Entity,
            &mut ambition_platformer2d_core::BodyKinematics,
            Option<&mut ae::SweepSample>,
            &ActorFaction,
            Option<&ambition_characters::actor::BodyHealth>,
            (
                Has<ambition_combat::death_rules::OutOfPlay>,
                Option<&ambition_platformer2d_core::DepthPlane>,
            ),
            Option<&ambition_characters::control::DrivingParticipant>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let pulls = outbox.drain::<PullBodiesPort>(&ambition_extension_sdk::phases::MODULE_ENTITY_TICK);
    let dt = world_time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for submitted in pulls {
        let pull = submitted.value;
        let center = ae::Vec2::from(pull.center);
        let factor = (pull.rate * dt).min(1.0);
        // Only bodies in the puller's own live room.
        let room = rooms.of(submitted.scope);
        for (body, mut kin, mut sweep, side, health, (out_of_play, plane), driver) in &mut bodies {
            if rooms.of(body) != room {
                continue;
            }
            // The effective side: a possessed NPC keeps `ActorFaction::Enemy`
            // and fights for its driver, so its authored side would pull the
            // player's own body. A corpse is not pulled.
            if ambition_combat::targeting::effective_faction(*side, driver) != ActorFaction::Enemy
                || ambition_combat::util::body_is_untouchable(health, out_of_play, plane)
            {
                continue;
            }
            if kin.pos.distance(center) <= pull.radius {
                let delta = kin.pos.lerp(center, factor) - kin.pos;
                ae::movement::carry_body(&mut kin, sweep.as_deref_mut(), delta);
            }
        }
    }
}

#[cfg(test)]
mod tests;
