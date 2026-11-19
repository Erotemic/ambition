//! Bounded rule queries use the shared body participation and targeting owners.

use std::collections::BTreeSet;

use ambition_characters::{actor::ActorFaction, control::DrivingParticipant};
use ambition_combat_port::body_query::*;
use ambition_extension_host::{
    AdmittedExtensions, ExtensionAppExt, ExtensionOutbox, LowersIn, ObservationContext,
};
use ambition_extension_sdk::{
    Port,
    action::{ActionResultPort, ActionStage},
    phases::ACTOR_THINK,
};
use ambition_platformer2d_core::{self as ae, snapshot::SnapshotState};
use ambition_platformer2d_shared_tangle::{
    action_results::{ActionMailbox, begin_action, publish_action_result},
    lifecycle::{InRoomInstance, SessionScopedEntity},
    sim_id::SimId,
};
use ambition_platformer2d_world::governing_rules::RulesOf;
use bevy::{ecs::system::SystemState, prelude::*};

pub const QUERY_DOMAIN: &str = "combat.body_query";

/// A game can set these limits in its room rules. Requests choose smaller bounds.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BodyQueryLimits {
    pub max_targets: u32,
    pub max_radius: f32,
}
impl Default for BodyQueryLimits {
    fn default() -> Self {
        Self {
            max_targets: 32,
            max_radius: 8192.0,
        }
    }
}
impl BodyQueryLimits {
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.max_targets == 0 {
            problems.push("body_queries.max_targets must be positive".into());
        }
        if !self.max_radius.is_finite() || self.max_radius <= 0.0 {
            problems.push("body_queries.max_radius must be finite and positive".into());
        }
        problems
    }
}

#[derive(Component, Clone, Debug, Default)]
pub struct BodyQueryHistory {
    results: Vec<BodyQueryResult>,
}
ambition_extension_sdk::port_record! { BodyQueryHistory { results: Vec<BodyQueryResult> } }
impl SnapshotState for BodyQueryHistory {
    fn encode(&self, out: &mut Vec<u8>) {
        let mut bytes = Vec::new();
        ambition_extension_sdk::structure::put(self, &mut bytes);
        ae::snapshot::put_bytes(out, &bytes);
    }
    fn decode(reader: &mut ae::snapshot::Reader<'_>) -> Option<Self> {
        ambition_extension_sdk::wire::decode_all(
            reader.bytes()?,
            ambition_extension_sdk::structure::read::<Self>,
        )
        .ok()
    }
}

pub fn install(app: &mut App) {
    ambition_platformer2d_shared_tangle::action_results::register_action_domain(app, QUERY_DOMAIN,
        ambition_platformer2d_shared_tangle::action_results::ActionCancellationPolicy::Unsupported);
    use ambition_extension_host::{
        InActorThink, InBodyContact, InBossConduct, InHitReaction, InModuleEntityTick,
        InTechniqueExecution, InWieldedUse,
    };
    macro_rules! phase {
        ($marker:ty) => {
            app.install_extension_submitted_request::<BodyQueryPort, _>(
                <$marker>::PHASE,
                "ambition_combat",
                lower::<$marker>,
            );
        };
    }
    phase!(InActorThink);
    phase!(InTechniqueExecution);
    phase!(InWieldedUse);
    phase!(InBossConduct);
    phase!(InModuleEntityTick);
    phase!(InBodyContact);
    phase!(InHitReaction);
    app.install_extension_context_observation::<BodyQueryResultPort>(
        ACTOR_THINK,
        "ambition_combat",
        result_of_reply,
    );
}

fn result_of_reply(
    world: &World,
    context: ObservationContext<'_>,
) -> Option<Option<BodyQueryResult>> {
    let Some(result) = context.trigger::<ActionResultPort>() else { return Some(None); };
    Some(
        (result.ticket.domain == QUERY_DOMAIN && result.stage == ActionStage::Applied)
            .then(|| {
                world
                    .get::<BodyQueryHistory>(context.scope)?
                    .results
                    .iter()
                    .find(|query| query.ticket == result.ticket)
                    .cloned()
            })
            .flatten(),
    )
}

type QueryRules = SystemState<RulesOf<'static, 'static, BodyQueryLimits>>;
fn lower<L: LowersIn>(world: &mut World, mut rules: Local<Option<QueryRules>>) {
    let submitted = world
        .resource_mut::<ExtensionOutbox>()
        .drain::<BodyQueryPort>(&L::PHASE);
    if submitted.is_empty() {
        return;
    }
    let rules = rules.get_or_insert_with(|| SystemState::new(world));
    for submitted in submitted {
        let request = submitted.value;
        let mut bytes = Vec::new();
        BodyQueryPort::encode(&request, &mut bytes);
        let dispatch = begin_action(
            world,
            submitted.scope,
            &submitted.entry,
            QUERY_DOMAIN,
            Some(&request.reply),
            ambition_extension_sdk::digest::digest_bytes(&bytes),
        );
        if !dispatch.execute {
            continue;
        }
        let valid = ambition_extension_host::reply_entry::<ActionResultPort>(
            &world.resource::<AdmittedExtensions>().0,
            &submitted.entry,
            &request.reply.selector,
            &ACTOR_THINK,
        )
        .is_some();
        let limits = rules
            .get(world)
            .ok()
            .and_then(|rules| rules.of(submitted.scope).copied())
            .unwrap_or_default();
        let selected = if valid {
            select(world, submitted.scope, &request, limits)
        } else {
            Err("invalid_reply_entry")
        };
        match selected {
            Err(code) => {
                publish_action_result(
                    world,
                    dispatch.reply.as_ref(),
                    ActionStage::Rejected,
                    true,
                    code,
                    "",
                    Vec::new(),
                );
            }
            Ok((targets, truncated)) => {
                let ticket = dispatch.reply.as_ref().unwrap().ticket.clone();
                let result = BodyQueryResult {
                    room: scope_room(world, submitted.scope).unwrap().ordinal(),
                    ticket,
                    observed_at: world
                        .get_resource::<ambition_time::SimTick>()
                        .map_or(0, |tick| tick.0),
                    phase: L::PHASE.0.to_string(),
                    truncated,
                    targets,
                };
                let identities = result
                    .targets
                    .iter()
                    .map(|target| target.id.clone())
                    .collect();
                if world.get::<BodyQueryHistory>(submitted.scope).is_none() {
                    world
                        .entity_mut(submitted.scope)
                        .insert(BodyQueryHistory::default());
                }
                let mut history = world.get_mut::<BodyQueryHistory>(submitted.scope).unwrap();
                if history.results.len() == ActionMailbox::HISTORY_CAPACITY {
                    history.results.remove(0);
                }
                history.results.push(result);
                publish_action_result(
                    world,
                    dispatch.reply.as_ref(),
                    ActionStage::Applied,
                    true,
                    "body_query_ready",
                    "",
                    identities,
                );
            }
        }
    }
}


fn scope_room(world: &World, scope: Entity) -> Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance> {
    use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, RoomInstanceRoot};
    world.get::<InRoomInstance>(scope).map(|at| at.0)
        .or_else(|| world.get::<RoomInstanceRoot>(scope).and(world.get::<LiveRoomInstance>(scope).copied()))
}

fn select(
    world: &mut World,
    scope: Entity,
    request: &BodyQuery,
    limits: BodyQueryLimits,
) -> Result<(Vec<BodyTarget>, bool), &'static str> {
    if !limits.problems().is_empty() {
        return Err("invalid_query_limits");
    }
    if request.origin.iter().any(|value| !value.is_finite())
        || !request.radius.is_finite()
        || request.radius < 0.0
    {
        return Err("invalid_query_geometry");
    }
    if request.radius > limits.max_radius
        || request.limit == 0
        || request.limit > limits.max_targets
        || request.identities.len() > limits.max_targets as usize
    {
        return Err("query_budget_exceeded");
    }
    if request.relation.is_some() && world.get::<ActorFaction>(scope).is_none() {
        return Err("no_combat_source");
    }
    let identities: BTreeSet<_> = request.identities.iter().map(String::as_str).collect();
    if request.require_all && (identities.is_empty() || identities.len() > request.limit as usize) {
        return Err("invalid_required_targets");
    }
    let session = *world
        .get::<SessionScopedEntity>(scope)
        .ok_or("no_live_scope")?;
    let room = scope_room(world, scope).ok_or("no_live_room")?;
    let mut bodies = world.query::<(
        Entity,
        &SimId,
        &SessionScopedEntity,
        &InRoomInstance,
        &ae::BodyKinematics,
    )>();
    let mut found = BTreeSet::new();
    let mut targets = Vec::new();
    for (entity, id, owner, at, kin) in bodies.iter(world) {
        if owner.0 != session.0 || at.0 != room || !request.include_self && entity == scope {
            continue;
        }
        if !identities.is_empty() && !identities.contains(id.as_str()) {
            continue;
        }
        if crate::util::body_is_untouchable(
            world.get(entity),
            world
                .entity(entity)
                .contains::<crate::death_rules::OutOfPlay>(),
            world.get(entity),
        ) {
            continue;
        }
        let relation = crate::targeting::observed_relation(world, scope, entity);
        if request
            .relation
            .is_some_and(|wanted| Some(wanted) != relation)
        {
            continue;
        }
        let dx = f64::from(kin.pos.x) - f64::from(request.origin[0]);
        let dy = f64::from(kin.pos.y) - f64::from(request.origin[1]);
        let squared = dx * dx + dy * dy;
        if !squared.is_finite() {
            return Err("invalid_body_position");
        }
        if squared > f64::from(request.radius).powi(2) {
            continue;
        }
        if !found.insert(id.as_str()) {
            return Err("ambiguous_body_identity");
        }
        let aabb = kin.aabb();
        targets.push((squared, BodyTarget {
            id: id.as_str().to_owned(),
            position: kin.pos.to_array(),
            size: (aabb.max - aabb.min).to_array(),
            velocity: kin.vel.to_array(),
            driven: world.get::<DrivingParticipant>(entity).is_some(),
            relation,
            distance: squared.sqrt() as f32,
        }));
    }
    if request.require_all && found.len() != identities.len() {
        return Err("target_missing");
    }
    targets.sort_by(|(a, left), (b, right)| a.total_cmp(b).then(left.id.cmp(&right.id)));
    let truncated = targets.len() > request.limit as usize;
    targets.truncate(request.limit as usize);
    Ok((targets.into_iter().map(|(_, target)| target).collect(), truncated))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::control::PlayerSlot;
    use ambition_extension_sdk::action::ReplyTo;
    use ambition_combat_port::targets::TargetRelation;
    use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, SessionScopeId};

    fn body(world: &mut World, name: &str, x: f32, team: &str, room: u32) -> Entity {
        world
            .spawn((
                SimId::placement(name),
                ae::BodyKinematics {
                    pos: ae::Vec2::new(x, 0.0),
                    ..Default::default()
                },
                ActorFaction::Npc,
                crate::targeting::MatchTeam::new(team),
                SessionScopedEntity(SessionScopeId(1)),
                InRoomInstance(LiveRoomInstance::from_ordinal(room)),
            ))
            .id()
    }
    fn request() -> BodyQuery {
        BodyQuery {
            reply: ReplyTo {
                request: 0,
                selector: "test::query/result".into(),
            },
            origin: [0.0, 0.0],
            radius: 100.0,
            limit: 1,
            relation: Some(TargetRelation::Foe),
            include_self: false,
            identities: Vec::new(),
            require_all: false,
        }
    }

    #[test]
    fn bounded_queries_use_shared_relationships_and_identity_order_for_every_controller() {
        for driven in [false, true] {
            let mut world = World::new();
            let scope = body(&mut world, "source", 0.0, "red", 0);
            if driven {
                world
                    .entity_mut(scope)
                    .insert(DrivingParticipant(PlayerSlot(0)));
            }
            body(&mut world, "b", -10.0, "blue", 0);
            let first = body(&mut world, "a", 10.0, "blue", 0);
            world
                .entity_mut(first)
                .insert(DrivingParticipant(PlayerSlot(1)));
            body(&mut world, "ally", 1.0, "red", 0);
            body(&mut world, "other-room", 2.0, "blue", 1);
            let gone = body(&mut world, "out-of-play", 3.0, "blue", 0);
            world.entity_mut(gone).insert(crate::death_rules::OutOfPlay);
            let foreign = body(&mut world, "other-session", 4.0, "blue", 0);
            world
                .entity_mut(foreign)
                .insert(SessionScopedEntity(SessionScopeId(2)));
            let (targets, truncated) =
                select(&mut world, scope, &request(), BodyQueryLimits::default()).unwrap();
            assert!(truncated);
            assert_eq!(targets.len(), 1);
            assert_eq!(targets[0].id, SimId::placement("a").as_str());
            assert!(targets[0].driven);
            assert_eq!(targets[0].relation, Some(TargetRelation::Foe));
        }
    }

    #[test]
    fn an_exact_follow_up_refuses_a_body_that_left_or_changed_relationship() {
        let mut world = World::new();
        let scope = body(&mut world, "source", 0.0, "red", 0);
        let target = body(&mut world, "target", 10.0, "blue", 0);
        let mut ask = request();
        ask.identities = vec![SimId::placement("target").as_str().into()];
        ask.require_all = true;
        assert_eq!(
            select(&mut world, scope, &ask, BodyQueryLimits::default())
                .unwrap()
                .0
                .len(),
            1
        );
        world
            .entity_mut(target)
            .insert(crate::targeting::MatchTeam::new("red"));
        assert_eq!(
            select(&mut world, scope, &ask, BodyQueryLimits::default()),
            Err("target_missing")
        );
        let mut aggression = crate::components::ActorAggression::passive();
        aggression.grudge = Some(crate::components::Grudge::Body(target));
        world.entity_mut(scope).insert(aggression);
        assert!(select(&mut world, scope, &ask, BodyQueryLimits::default()).is_ok());
        world.despawn(target);
        assert_eq!(
            select(&mut world, scope, &ask, BodyQueryLimits::default()),
            Err("target_missing")
        );
    }
}
