//! Module-owned entities (fast-iteration I7): the adapters of
//! `ambition.world.spawn_module_entity` and `ambition.world.module_entity_tick`.
//! See the port cards in `ambition_combat_port::module_entity`.
//!
//! The world owns a module entity's identity, side, team, session,
//! presentation source, position and lifetime. What the entity does each tick
//! is the module's: the sentry turret is the first one
//! (`ambition_content_modules::sentry`).

use ambition_combat::components::{ActorFaction, CenteredAabb};
use ambition_combat_port::{
    EndModuleEntityPort, ModuleEntitySpawn, ModuleEntityTick, ModuleEntityTickPort, SpawnModuleEntityPort,
};
use ambition_extension_host::{AdmittedExtensions, ExtensionInvocations, ExtensionOutbox};
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
}

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
    let mut spawned = commands.spawn_session_scoped(from.scope, (entity, name, from.side, from.id));
    if let Some(team) = from.team {
        spawned.insert(team);
    }
    if let Some(source) = from.presentation {
        spawned.insert(ambition_sfx::BodyPresentationSource(source));
    }
    spawned.id()
}

/// Lower `ambition.world.spawn_module_entity`: mint the entity's identity from
/// the spawner, and spawn it with the spawner's side.
pub fn lower_module_entity_spawns(
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
    for submitted in outbox.drain::<SpawnModuleEntityPort>() {
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
        let ModuleEntitySpawn { kind, at, lifetime_s } = submitted.value;
        spawn_module_entity(
            &mut commands,
            ModuleEntity {
                kind,
                pos: ae::Vec2::from(at),
                remaining_s: lifetime_s,
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
        ),
        With<FeatureSimEntity>,
    >,
) {
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
        .filter(|(_, _, side, health, (out_of_play, plane), _, driver)| {
            ambition_combat::targeting::effective_faction(**side, *driver) == ActorFaction::Enemy
                && !ambition_combat::util::body_is_untouchable(*health, *out_of_play, *plane)
        })
        .map(|(body, aabb, _, _, _, id, _)| (aabb.center, id, rooms.of(body)))
        .collect();
    for (pos, remaining_s, _, entity) in living {
        let kind = entities.get(entity).map(|(_, e, _)| e.kind.clone()).unwrap_or_default();
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
            },
        );
    }
}

/// Lower `ambition.world.end_module_entity`: remove the module entity the
/// invocation ran for.
pub fn lower_module_entity_ends(
    mut outbox: ResMut<ExtensionOutbox>,
    entities: Query<(), With<ModuleEntity>>,
    mut commands: Commands,
) {
    for submitted in outbox.drain::<EndModuleEntityPort>() {
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
    let pulls = outbox.drain::<PullBodiesPort>();
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
