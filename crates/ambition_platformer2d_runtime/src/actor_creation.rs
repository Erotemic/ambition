//! Native character construction with declared script-state initialization.

use bevy::{ecs::system::SystemState, prelude::*};
use ambition_extension_host::{AdmittedExtensions, BodyRecords, ExtensionOutbox, LowersIn};
use ambition_extension_sdk::action::ActionStage;
use ambition_platformer2d_construction::runtime::{ActorCreationLimits, RuntimeActor, RuntimeLink, ComposedInitialization};
use ambition_platformer2d_shared_tangle::action_results::publish_action_result;
use ambition_platformer2d_world::governing_rules::RulesOf;
use ambition_world_port::creation::*;

pub const CREATE_DOMAIN: &str = "world.create_actors";

pub fn install(app: &mut App) {
    ambition_platformer2d_shared_tangle::action_results::register_action_domain(app, CREATE_DOMAIN,
        ambition_platformer2d_shared_tangle::action_results::ActionCancellationPolicy::Unsupported);
    ambition_extension_host::install_submitted_request_in_body_phases!(app, CreateActorsPort,
        "ambition_platformer2d_construction", lower);
}

type CreationRules = SystemState<RulesOf<'static, 'static, ActorCreationLimits>>;
fn lower<L: LowersIn>(world: &mut World, mut rules: Local<Option<CreationRules>>) {
    let submitted = world.resource_mut::<ExtensionOutbox>().drain::<CreateActorsPort>(&L::PHASE);
    if submitted.is_empty() { return; }
    let rules = rules.get_or_insert_with(|| SystemState::new(world));
    for submitted in submitted {
        let request = submitted.value;
        let dispatch = ambition_platformer2d_shared_tangle::action_results::begin_action_request::<CreateActorsPort>(
            world, submitted.state_scope, &submitted.entry, &request);
        if !dispatch.execute { continue; }
        let limits = rules.get(world).ok().and_then(|rules| rules.of(submitted.scope).copied()).unwrap_or_default();
        let result = prepare_state(world, &submitted.entry, &request, limits).and_then(|states| {
            let inherit = world.get::<ambition_characters::actor::ActorFaction>(submitted.scope)
                .copied().unwrap_or(ambition_characters::actor::ActorFaction::Neutral);
            let bindings = crate::object_behaviors::prepare_creation(world, submitted.scope,
                &request.actors.iter().map(|actor| actor.bindings.as_slice()).collect::<Vec<_>>())?;
            let actors = request.actors.iter().zip(states).zip(bindings).map(|((actor, states), bindings)| RuntimeActor {
                key: actor.key.clone(), character: actor.character.clone(),
                aabb: ambition_platformer2d_core::Aabb::new(actor.at.into(), actor.half_extents.into()),
                faction: faction(actor.allegiance, inherit), face_left: actor.face_left,
                starting_wallet: actor.starting_wallet, holds: actor.holds.clone(),
                initialization: {
                    let mut records = BodyRecords::default();
                    for (key, shape, record) in states { records.put(key, shape, record); }
                    ComposedInitialization {root: records, owned: bindings}
                },
            }).collect::<Vec<_>>();
            let links = request.links.iter().map(|link| RuntimeLink {
                from: link.from.clone(), to: link.to.clone(), relation: match link.kind {
                    ActorLinkKind::Grudge => ambition_platformer2d_construction::ActorRelation::Grudge,
                    ActorLinkKind::Mount => ambition_platformer2d_construction::ActorRelation::Mount,
                },
            }).collect::<Vec<_>>();
            let created = ambition_platformer2d_construction::runtime::create_actors(world,
                submitted.scope, &actors, &links, limits)?;
            Ok(created.into_iter().map(|actor|
                ambition_platformer2d_shared_tangle::action_results::object_action_subject(world, actor.entity)
                    .expect("a published actor has a live occurrence")).collect())
        });
        match result {
            Ok(ids) => { publish_action_result(world, dispatch.reply.as_ref(), ActionStage::Applied, true,
                "actors_created", "", ids); }
            Err(reason) => { publish_action_result(world, dispatch.reply.as_ref(), ActionStage::Rejected, true,
                "actor_creation_refused", reason, Vec::new()); }
        }
    }
}

fn prepare_state(world: &World, entry: &str, request: &CreateActors, limits: ActorCreationLimits)
    -> Result<Vec<Vec<crate::record_seeds::PreparedSeed>>, String> {
    if request.actors.len() > limits.max_actors as usize { return Err("Too many requested actors.".into()); }
    let admitted = &world.resource::<AdmittedExtensions>().0;
    let entry = admitted.entries.iter().find(|active| active.path == entry).ok_or("The source entry is no longer installed.")?;
    // The byte bound covers the whole request, not each actor.
    let mut bytes = 0usize;
    request.actors.iter().map(|actor| {
        if actor.at.iter().chain(&actor.half_extents).any(|n| !n.is_finite())
            || actor.half_extents.iter().any(|n| *n <= 0.0) {
            return Err(format!("Actor {} has an invalid placement box.", actor.key));
        }
        if actor.state.len() > limits.max_state_records as usize { return Err("Too many initial state records.".into()); }
        crate::record_seeds::prepare_record_seeds(admitted, &entry.descriptor.writes, &actor.state,
            &mut bytes, limits.max_state_bytes, &format!("Actor {}", actor.key))
    }).collect()
}

fn faction(value: SpawnAllegiance, inherit: ambition_characters::actor::ActorFaction)
    -> ambition_characters::actor::ActorFaction {
    use ambition_characters::actor::ActorFaction as F;
    match value { SpawnAllegiance::Inherit => inherit, SpawnAllegiance::Player => F::Player,
        SpawnAllegiance::Enemy => F::Enemy, SpawnAllegiance::Npc => F::Npc,
        SpawnAllegiance::Boss => F::Boss, SpawnAllegiance::Neutral => F::Neutral }
}
