//! Checked character compositions in an existing live room.

use std::collections::{BTreeMap, BTreeSet};
use bevy::prelude::*;
use ambition_characters::{actor::ActorFaction, prepared::ActiveSessionCast};
use ambition_platformer2d_core as ae;
use ae::AabbExt as _;
use ambition_platformer2d_shared_tangle::{construction::*, lifecycle::*, sim_id::{SimId, SimIdCounter}};
use crate::{ActorConstructionParams, ActorConstructionRequest, ActorConstructionServices,
    ActorConstructionRegistry, ActorRelation, PersistedFates, mechanics::SessionMechanics};

/// A native character occurrence. Behavior state is initialized by its owner.
#[derive(Clone, Debug)]
pub struct RuntimeActor {
    pub key: String,
    pub character: String,
    pub aabb: ae::Aabb,
    pub faction: ActorFaction,
    pub face_left: bool,
    pub holds: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct RuntimeLink { pub from: String, pub to: String, pub relation: ActorRelation }

pub use ambition_platformer2d_world::creation::ActorCreationLimits;

/// Roots in request order. Internal limbs are part of the same committed plan.
pub struct CreatedActor { pub key: String, pub id: SimId, pub entity: Entity }

/// Create the complete batch or refuse before world mutation.
/// The live room owns the new bodies. Their creator need not remain alive.
pub fn create_actors(world: &mut World, caller: Entity, actors: &[RuntimeActor],
    links: &[RuntimeLink], limits: ActorCreationLimits) -> Result<Vec<CreatedActor>, String> {
    if actors.is_empty() || actors.len() > limits.max_actors as usize || links.len() > limits.max_links as usize {
        return Err("The actor or relation count exceeds this room's creation bounds.".into());
    }
    world.flush();
    let owner = world.get::<SessionScopedEntity>(caller).map(|owner| owner.0)
        .ok_or("The creator has no live session.")?;
    if world.get_resource::<ActiveSessionScope>().is_some_and(|active| active.current() != Some(owner)) {
        return Err("The creator's session is not active.".into());
    }
    let room = world.get::<InRoomInstance>(caller).map(|at| at.0)
        .or_else(|| world.get::<LiveRoomInstance>(caller).copied()).ok_or("The creator has no live room.")?;
    let mut roots = world.query_filtered::<(&SessionScopedEntity, &LiveRoomInstance), With<RoomInstanceRoot>>();
    if roots.iter(world).filter(|(scope, live)| scope.0 == owner && **live == room).count() != 1 {
        return Err("The creator's room is no longer live.".into());
    }
    let parent = world.get::<SimId>(caller).cloned().ok_or("The creator has no semantic identity.")?;
    let start = world.get::<SimIdCounter>(caller).ok_or("The creator cannot mint identities.")?.0;
    let next = start.checked_add(actors.len() as u64).ok_or("The creator's identity range is exhausted.")?;
    let mut keys = BTreeSet::new();
    for actor in actors {
        if actor.key.is_empty() || actor.key.len() > 128 || !keys.insert(&actor.key) {
            return Err("Actor keys must be nonempty, unique and at most 128 bytes.".into());
        }
        let center = actor.aabb.center();
        let half = actor.aabb.half_size();
        if !center.x.is_finite() || !center.y.is_finite() || !half.x.is_finite() || !half.y.is_finite()
            || half.x <= 0.0 || half.y <= 0.0 {
            return Err(format!("Actor {} has an invalid placement box.", actor.key));
        }
        if actor.holds.len() > limits.max_holds as usize || actor.holds.iter().any(|kind| kind.is_empty() || kind.len() > 128) {
            return Err(format!("Actor {} has invalid or excessive status selectors.", actor.key));
        }
    }
    let generation = world.get_resource::<SessionMechanics>().cloned().ok_or("The live session has no mechanics.")?;
    let cast = world.get_resource::<ActiveSessionCast>().and_then(|cast| cast.cast())
        .cloned().ok_or("The live session has no prepared cast.")?;
    let catalog = world.get_resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()
        .cloned().ok_or("The host has no character catalog.")?;
    let services = ActorConstructionServices {
        context: crate::placements::ActorPlacementContext::new(&generation.sprite_content, &catalog, &generation.sheets)
            .with_prepared(&cast).with_forced_brains(&generation.forced_brains),
        boss_catalog: generation.bosses.clone(),
    };
    let mut rows = Vec::new();
    let mut ids = BTreeMap::new();
    for (index, actor) in actors.iter().enumerate() {
        let sequence = start + index as u64;
        let id = SimId::spawned(&parent, sequence);
        ids.insert(actor.key.as_str(), id.clone());
        let mut spec = ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Passive, actor.character.as_str());
        spec.facing = if actor.face_left { ambition_platformer2d_world::rooms::SpawnFacing::Left }
            else { ambition_platformer2d_world::rooms::SpawnFacing::Right };
        let authored = ambition_platformer2d_world::rooms::Authored::new(id.as_str(), actor.key.clone(), actor.aabb, spec);
        let origin = SpawnOrigin::Dynamic { parent: parent.clone(), sequence };
        if let Some(hands) = ambition_platformer2d_actor_spawn::limbs_of(cast.get(&actor.character)) {
            let hands = ambition_platformer2d_actor_spawn::giant_hand_plans(id.as_str(), actor.aabb, hands);
            let host = id.clone();
            rows.extend(crate::giant_cluster_rows(id, authored, actor.faction, Vec::new(), hands, None, origin,
                move |hand| SpawnOrigin::Dynamic { parent: host.clone(), sequence: hand.ordinal }));
        } else {
            rows.push(ActorConstructionRequest { sim_id: id, origin,
                parameters: ActorConstructionParams::CharacterBody { authored, faction: actor.faction, paths: Vec::new() },
                relations: Vec::new() });
        }
    }
    if rows.len() > limits.max_rows as usize { return Err("The complete actor clusters exceed the row bound.".into()); }
    if rows.iter().any(|row| row.sim_id.as_str().len() > ambition_extension_sdk::MAX_REF_LEN as usize) {
        return Err("An actor identity exceeds the portable reference bound.".into());
    }
    let mut population = world.query::<(&SessionScopedEntity, &InRoomInstance, &ae::body_clusters::BodyKinematics)>();
    let count = population.iter(world).filter(|(scope, at, _)| scope.0 == owner && at.0 == room).count();
    if count.saturating_add(rows.len()) > limits.max_room_bodies as usize {
        return Err("The room's body population bound is exhausted.".into());
    }
    for link in links {
        let from = ids.get(link.from.as_str()).ok_or_else(|| format!("No actor has key {}.", link.from))?;
        let to = ids.get(link.to.as_str()).ok_or_else(|| format!("No actor has key {}.", link.to))?;
        rows.iter_mut().find(|row| &row.sim_id == from).unwrap().relations.push(RelationRequest {
            to: to.clone(), relation: link.relation.clone(),
        });
    }
    crate::preflight_planned_bodies(&rows, Some(&cast), &catalog).map_err(|e| e.to_string())?;
    crate::preflight_actor_relations(&rows, &generation.bosses, Some(&cast)).map_err(|e| e.to_string())?;
    let mut identities = world.query::<(&SessionScopedEntity, &SimId)>();
    let live = identities.iter(world).filter(|(scope, _)| scope.0 == owner).map(|(_, id)| id.clone()).collect();
    let scope = ConstructionScope::in_generation(ContentBinding::RuntimeDynamic, None);
    let plan = crate::ActorConstructionPlan::prepare(scope.clone(), rows, &live,
        world.get_resource::<ActorConstructionRegistry>().ok_or("The actor construction capability is not installed.")?)
        .map_err(|e| e.to_string())?;
    let receipt = {
        let mut commands = world.commands();
        plan.commit(&mut ConstructionExecCtx { commands: &mut commands, scope: &scope,
            session: SessionSpawnScope::scoped(owner).in_room(Some(room)), services: &services,
            facts: &PersistedFates::unrecorded() })
    };
    world.flush();
    for id in receipt.committed_ids() {
        world.entity_mut(receipt.entity(&id).unwrap()).insert(ambition_combat::components::RuntimeStagedActor);
    }
    let created = actors.iter().map(|actor| {
        let id = ids[actor.key.as_str()].clone();
        let entity = receipt.entity(&id).expect("the complete plan committed its root");
        if !actor.holds.is_empty() {
            let mut holds = actor.holds.clone(); holds.sort(); holds.dedup();
            world.entity_mut(entity).insert(ambition_combat::extension::status::BodyStatuses(holds));
        }
        CreatedActor { key: actor.key.clone(), id, entity }
    }).collect();
    world.get_mut::<SimIdCounter>(caller).unwrap().0 = next;
    Ok(created)
}
