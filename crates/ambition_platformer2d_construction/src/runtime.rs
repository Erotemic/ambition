//! Checked runtime construction in an existing live room.

use crate::{
    mechanics::SessionMechanics, ActorConstructionParams, ActorConstructionRegistry,
    ActorConstructionRequest, ActorConstructionServices, ActorRelation, PersistedFates,
};
use ae::AabbExt as _;
use ambition_characters::{actor::ActorFaction, prepared::ActiveSessionCast};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::{
    construction::*,
    lifecycle::*,
    sim_id::{SimId, SimIdCounter},
};
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// A checked private state bundle that belongs to one new object occurrence.
#[derive(Clone, Debug)]
pub struct OwnedObjectState<B> { pub key: String, pub bundle: B }

/// Root state and private instances materialize in the same native transaction.
#[derive(Clone, Debug)]
pub struct ComposedInitialization<Root, Owned> {
    pub root: Root,
    pub owned: Vec<OwnedObjectState<Owned>>,
}

/// Native owners validate their bundles before the construction transaction starts.
pub trait RuntimeInitialization: Clone {
    type Owned: Bundle + Clone;
    fn insert(&self, root: &mut RootScope<'_, '_, '_>);
    fn owned(&self) -> &[OwnedObjectState<Self::Owned>];
}
impl<B: Bundle + Clone> RuntimeInitialization for B {
    type Owned = ();
    fn insert(&self, root: &mut RootScope<'_, '_, '_>) { root.insert(self.clone()); }
    fn owned(&self) -> &[OwnedObjectState<()>] { &[] }
}
impl<R: Bundle + Clone, O: Bundle + Clone> RuntimeInitialization for ComposedInitialization<R, O> {
    type Owned = O;
    fn insert(&self, root: &mut RootScope<'_, '_, '_>) { root.insert(self.root.clone()); }
    fn owned(&self) -> &[OwnedObjectState<O>] { &self.owned }
}

/// A native character occurrence. Behavior state is initialized by its owner.
#[derive(Clone, Debug)]
pub struct RuntimeActor<Initialization = ()> {
    pub key: String,
    pub character: String,
    pub aabb: ae::Aabb,
    pub faction: ActorFaction,
    pub face_left: bool,
    pub starting_wallet: Option<i32>,
    pub holds: Vec<String>,
    /// The component owner validates these values before construction.
    pub initialization: Initialization,
}

#[derive(Clone, Debug)]
pub struct RuntimeLink {
    pub from: String,
    pub to: String,
    pub relation: ActorRelation,
}

pub use ambition_platformer2d_world::creation::ActorCreationLimits;

/// Roots in request order. Internal limbs are part of the same committed plan.
pub struct CreatedActor {
    pub key: String,
    pub id: SimId,
    pub entity: Entity,
}

/// Publish the complete batch or leave no new live roots.
/// The live room owns the new bodies. Their creator need not remain alive.
pub fn create_actors<Initialization: RuntimeInitialization>(
    world: &mut World,
    caller: Entity,
    actors: &[RuntimeActor<Initialization>],
    links: &[RuntimeLink],
    limits: ActorCreationLimits,
) -> Result<Vec<CreatedActor>, String> {
    if actors.is_empty()
        || actors.len() > limits.max_actors as usize
        || links.len() > limits.max_links as usize
    {
        return Err("The actor or relation count exceeds this room's creation bounds.".into());
    }
    let context = RuntimeConstruction::prepare(world, caller, actors.len())?;
    let owner = context.session;
    let room = context.room;
    let mut keys = BTreeSet::new();
    for actor in actors {
        if actor.key.is_empty() || actor.key.len() > 128 || !keys.insert(&actor.key) {
            return Err("Actor keys must be nonempty, unique and at most 128 bytes.".into());
        }
        let center = actor.aabb.center();
        let half = actor.aabb.half_size();
        if !center.x.is_finite()
            || !center.y.is_finite()
            || !half.x.is_finite()
            || !half.y.is_finite()
            || half.x <= 0.0
            || half.y <= 0.0
        {
            return Err(format!("Actor {} has an invalid placement box.", actor.key));
        }
        if actor.holds.len() > limits.max_holds as usize
            || actor
                .holds
                .iter()
                .any(|kind| kind.is_empty() || kind.len() > 128)
        {
            return Err(format!(
                "Actor {} has invalid or excessive status selectors.",
                actor.key
            ));
        }
    }
    let cast = world
        .get_resource::<ActiveSessionCast>()
        .and_then(|cast| cast.cast())
        .cloned()
        .ok_or("The live session has no prepared cast.")?;
    let catalog = world
        .get_resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()
        .cloned()
        .ok_or("The host has no character catalog.")?;
    let mut rows = Vec::new();
    let mut ids = BTreeMap::new();
    for (index, actor) in actors.iter().enumerate() {
        let id = context.id(index);
        ids.insert(actor.key.as_str(), id.clone());
        let mut spec = ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Passive,
            actor.character.as_str(),
        );
        spec.facing = if actor.face_left {
            ambition_platformer2d_world::rooms::SpawnFacing::Left
        } else {
            ambition_platformer2d_world::rooms::SpawnFacing::Right
        };
        let authored = ambition_platformer2d_world::rooms::Authored::new(
            id.as_str(),
            actor.key.clone(),
            actor.aabb,
            spec,
        );
        let origin = context.origin(index);
        if let Some(hands) = ambition_platformer2d_actor_spawn::limbs_of(cast.get(&actor.character))
        {
            let hands =
                ambition_platformer2d_actor_spawn::giant_hand_plans(id.as_str(), actor.aabb, hands);
            let host = id.clone();
            rows.extend(crate::giant_cluster_rows(
                id,
                authored,
                actor.faction,
                Vec::new(),
                hands,
                None,
                origin,
                move |hand| SpawnOrigin::Dynamic {
                    parent: host.clone(),
                    sequence: hand.ordinal,
                },
            ));
        } else {
            rows.push(ActorConstructionRequest {
                sim_id: id,
                origin,
                parameters: ActorConstructionParams::CharacterBody {
                    authored,
                    faction: actor.faction,
                    paths: Vec::new(),
                },
                relations: Vec::new(),
            });
        }
    }
    if rows.len() > limits.max_rows as usize {
        return Err("The complete actor clusters exceed the row bound.".into());
    }
    if rows
        .iter()
        .any(|row| row.sim_id.as_str().len() > ambition_extension_sdk::MAX_REF_LEN as usize)
    {
        return Err("An actor identity exceeds the portable reference bound.".into());
    }
    let mut population = world.query::<(
        &SessionScopedEntity,
        &InRoomInstance,
        &ae::body_clusters::BodyKinematics,
    )>();
    let count = population
        .iter(world)
        .filter(|(scope, at, _)| scope.0 == owner && at.0 == room)
        .count();
    if count.saturating_add(rows.len()) > limits.max_room_bodies as usize {
        return Err("The room's body population bound is exhausted.".into());
    }
    for link in links {
        let from = ids
            .get(link.from.as_str())
            .ok_or_else(|| format!("No actor has key {}.", link.from))?;
        let to = ids
            .get(link.to.as_str())
            .ok_or_else(|| format!("No actor has key {}.", link.to))?;
        rows.iter_mut()
            .find(|row| &row.sim_id == from)
            .unwrap()
            .relations
            .push(RelationRequest {
                to: to.clone(),
                relation: link.relation.clone(),
            });
    }
    crate::preflight_planned_bodies(&rows, Some(&cast), &catalog).map_err(|e| e.to_string())?;
    crate::preflight_actor_relations(&rows, &context.services.boss_catalog, Some(&cast))
        .map_err(|e| e.to_string())?;
    let initializers: BTreeMap<_, _> = actors.iter().map(|actor| (ids[actor.key.as_str()].clone(), actor)).collect();
    let states = initializers.iter().map(|(id, actor)| (id.clone(), &actor.initialization)).collect();
    let receipt = context.commit(world, rows, &states, |id, root| {
        root.insert(ambition_combat::components::RuntimeStagedActor);
        if let Some(actor) = initializers.get(id) {
            if let Some(balance) = actor.starting_wallet {
                root.insert(ambition_characters::actor::BodyWallet {balance});
            }
            if !actor.holds.is_empty() {
                let mut holds = actor.holds.clone();
                holds.sort();
                holds.dedup();
                root.insert(ambition_combat::extension::status::BodyStatuses(holds));
            }
        }
    })?;
    let created = actors.iter().map(|actor| {
        let id = ids[actor.key.as_str()].clone();
        let entity = receipt.entity(&id).expect("the complete plan committed its root");
        CreatedActor {key: actor.key.clone(), id, entity}
    }).collect();
    Ok(created)
}

/// Frozen source context for a runtime construction batch.
/// Domain owners validate their data before they commit the common plan.
pub struct RuntimeConstruction {
    caller: Entity,
    session: SessionScopeId,
    room: LiveRoomInstance,
    parent: SimId,
    start: u64,
    next: u64,
    services: ActorConstructionServices,
}
impl RuntimeConstruction {
    pub fn prepare(world: &mut World, caller: Entity, count: usize) -> Result<Self, String> {
        world.flush();
        let (session, room, parent, start) = Self::source(world, caller)?;
        let next = start
            .checked_add(count as u64)
            .ok_or("The creator's identity range is exhausted.")?;
        let generation = world
            .get_resource::<SessionMechanics>()
            .ok_or("The live session has no mechanics.")?;
        let catalog = world
            .get_resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()
            .cloned()
            .unwrap_or_else(ambition_characters::actor::character_catalog::CharacterCatalog::empty);
        let cast = world
            .get_resource::<ActiveSessionCast>()
            .and_then(|cast| cast.cast())
            .cloned()
            .unwrap_or_default();
        let services = ActorConstructionServices {
            context: crate::placements::ActorPlacementContext::new(
                &generation.sprite_content,
                &catalog,
                &generation.sheets,
            )
            .with_prepared(&cast)
            .with_forced_brains(&generation.forced_brains),
            boss_catalog: generation.bosses.clone(),
        };
        Ok(Self {
            caller,
            session,
            room,
            parent,
            start,
            next,
            services,
        })
    }

    fn source(
        world: &mut World,
        caller: Entity,
    ) -> Result<(SessionScopeId, LiveRoomInstance, SimId, u64), String> {
        let session = world
            .get::<SessionScopedEntity>(caller)
            .map(|owner| owner.0)
            .ok_or("The creator has no live session.")?;
        if world
            .get_resource::<ActiveSessionScope>()
            .is_some_and(|active| active.current() != Some(session))
        {
            return Err("The creator's session is not active.".into());
        }
        let is_room = world.get::<RoomInstanceRoot>(caller).is_some();
        let room = world
            .get::<InRoomInstance>(caller)
            .map(|at| at.0)
            .or_else(|| {
                is_room
                    .then(|| world.get::<LiveRoomInstance>(caller).copied())
                    .flatten()
            })
            .ok_or("The creator has no live room.")?;
        let mut roots = world
            .query_filtered::<(&SessionScopedEntity, &LiveRoomInstance), With<RoomInstanceRoot>>();
        if roots
            .iter(world)
            .filter(|(owner, live)| owner.0 == session && **live == room)
            .count()
            != 1
        {
            return Err("The creator's room is no longer live.".into());
        }
        let parent = world
            .get::<SimId>(caller)
            .cloned()
            .ok_or("The creator has no semantic identity.")?;
        let start = world
            .get::<SimIdCounter>(caller)
            .ok_or("The creator cannot mint identities.")?
            .0;
        Ok((session, room, parent, start))
    }

    pub fn session(&self) -> SessionScopeId {
        self.session
    }
    pub fn room(&self) -> LiveRoomInstance {
        self.room
    }
    pub fn origin(&self, index: usize) -> SpawnOrigin {
        let _ = self.id(index);
        SpawnOrigin::Dynamic {
            parent: self.parent.clone(),
            sequence: self.start + index as u64,
        }
    }

    pub fn id(&self, index: usize) -> SimId {
        let sequence = self
            .start
            .checked_add(index as u64)
            .expect("a prepared index fits the reserved range");
        assert!(sequence < self.next);
        SimId::spawned(&self.parent, sequence)
    }

    /// Initialize and verify hidden roots, then publish their complete state.
    pub fn commit<Initialization: RuntimeInitialization>(
        self,
        world: &mut World,
        rows: Vec<ActorConstructionRequest>,
        states: &BTreeMap<SimId, &Initialization>,
        mut initialize: impl FnMut(&SimId, &mut RootScope<'_, '_, '_>),
    ) -> Result<ConstructionReceipt, String> {
        world.flush();
        if Self::source(world, self.caller)?
            != (self.session, self.room, self.parent.clone(), self.start)
        {
            return Err("The construction source changed before commit.".into());
        }
        if rows
            .iter()
            .any(|row| row.sim_id.as_str().len() > ambition_extension_sdk::MAX_REF_LEN as usize)
        {
            return Err("A constructed identity exceeds the portable reference bound.".into());
        }
        let rooms = TransactionRooms::only(self.room);
        let session = SessionSpawnScope::scoped(self.session).in_room(Some(self.room));
        let baseline = TransactionBaseline::capture_for_session(world, session, rooms)
            .map_err(|problem| {
                bevy::log::error!(?problem, "runtime publication baseline is invalid");
                "The runtime publication baseline is invalid.".to_owned()
            })?;
        let live = baseline.entries().keys().cloned().collect();
        if !inactive_candidate_filter_installed(world) {
            return Err("Runtime publication requires the candidate query filter.".into());
        }
        let mut owned_count = 0usize;
        for (id, state) in states {
            if !rows.iter().any(|row| &row.sim_id == id) {
                return Err("Initialization names an object outside the construction batch.".into());
            }
            owned_count = owned_count.checked_add(state.owned().len()).ok_or("Too many owned states.")?;
        }
        let object_count = rows.len().checked_add(owned_count).ok_or("Too many construction objects.")?;
        let occurrences = PreparedObjectOccurrences::prepare(world, self.session, self.room, object_count)?;
        let scope = ConstructionScope::in_generation(ContentBinding::RuntimeDynamic, None);
        let source_key: String = self.parent.as_str().bytes().map(|byte| format!("{byte:02x}")).collect();
        let lane = ConstructionLane::named(format!("runtime:{}:{source_key}:{}", self.room.ordinal(), self.start));
        let plan = crate::ActorConstructionPlan::prepare_in_lane(
            scope.clone(),
            lane,
            rows,
            &live,
            world
                .get_resource::<ActorConstructionRegistry>()
                .ok_or("The construction capability is not installed.")?,
        )
        .map_err(|error| error.to_string())?;
        let mut owned_specs = Vec::with_capacity(owned_count);
        for (index, root) in plan.entities().iter().enumerate() {
            if let Some(state) = states.get(root.sim_id()) {
                for owned in state.owned() {
                    owned_specs.push(OwnedStateSpec {owner: root.sim_id().clone(),
                        reference: occurrences.reference(index), key: owned.key.clone()});
                }
            }
        }
        let plan = plan.with_owned_states(owned_specs).map_err(|error| error.to_string())?;
        let transaction = plan.transaction(session);
        let mut owned_entities = Vec::with_capacity(owned_count);
        let receipt = {
            let mut commands = world.commands();
            let receipt = plan.commit_hidden(&mut ConstructionExecCtx {
                commands: &mut commands, scope: &scope, session,
                services: &self.services, facts: &PersistedFates::unrecorded(),
            });
            for (index, id) in receipt.committed_ids().iter().enumerate() {
                let mut root = RootScope::new(&mut commands, session, receipt.entity(id).unwrap());
                initialize(id, &mut root);
                if let Some(state) = states.get(id) {
                    state.insert(&mut root);
                    let reference = occurrences.reference(index);
                    for owned in state.owned() {
                        let entity = receipt.owned_state(reference, &owned.key).expect("the plan allocated every private state");
                        RootScope::new(&mut commands, session, entity).insert(owned.bundle.clone());
                        owned_entities.push(entity);
                    }
                }
            }
            receipt
        };
        world.flush();
        let actual = AuthoritativeScope::gather_for_session(world, &transaction, session, rooms);
        let mut problems = verify_committed_roster(&plan, &receipt, &baseline, &actual, world)
            .err().unwrap_or_default();
        problems.extend(crate::verify_rig_composition(&plan, &receipt, world));
        if !problems.is_empty() {
            retire_candidate(world, &transaction);
            bevy::log::error!(?problems, "runtime construction did not verify");
            return Err("The runtime batch failed native verification.".into());
        }
        let roots: Vec<_> = receipt.committed_ids().iter().map(|id| receipt.entity(id).unwrap()).collect();
        let mut complete = roots.clone();
        complete.extend(owned_entities);
        let notices = match occurrences.assign(world, &complete) {
            Ok(notices) => notices,
            Err(problem) => {
                retire_candidate(world, &transaction);
                return Err(problem.into());
            }
        };
        world.get_mut::<SimIdCounter>(self.caller).expect("the checked source is live").0 = self.next;
        let published = publish_candidate(world, &transaction);
        assert_eq!(published, complete.len(), "the complete runtime batch must publish together");
        for notice in notices.into_iter().take(roots.len()) { world.trigger(notice); }
        Ok(receipt)
    }
}
