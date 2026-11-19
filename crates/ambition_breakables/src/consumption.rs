//! Atomic consumption and inventory exchange from accepted movement contact.

use super::{BreakableFeature, emit_breakable_destroyed, simulation};
use ambition_characters::{actor::BodyHealth, control::DrivingParticipant};
use ambition_combat::{components::{CenteredAabb, FeatureId}, extension::contact::ContactFactsLimits};
use ambition_platformer2d_core::{self as ae, collision_semantics::ContactSource};
use ambition_platformer2d_items::{inventory::{self, ExchangeApplied, InventoryExchange},
    pickups::{ConsumedSinceCheckpoint, ConsumedWrite}};
use ambition_platformer2d_shared_tangle::{construction::SpawnOrigin, lifecycle::*, sim_id::SimId};
use ambition_platformer2d_world::governing_rules::RulesOf;
use bevy::{ecs::system::SystemState, prelude::*};

pub struct ConsumptionReceipt {
    pub object: SimId,
    pub exchange: ExchangeApplied,
}

type Context = SystemState<(RoomFreeze<'static, 'static>,
    RulesOf<'static, 'static, ContactFactsLimits>)>;
type Effects = SystemState<(Commands<'static, 'static>,
    Query<'static, 'static, &'static mut BreakableFeature>,
    ambition_sfx::SfxWriter<'static>, ambition_vfx::vfx::VfxWriter<'static>,
    MessageWriter<'static, ambition_vfx::vfx::DebrisBurstMessage>)>;

/// Consume the contacted destructible and apply all exchange terms, or neither.
/// The caller must have current movement output in its live room and session.
pub fn consume_contact(world: &mut World, caller: Entity, contact: u32,
    exchange: &InventoryExchange) -> Result<ConsumptionReceipt, &'static str> {
    world.flush();
    let tick = world.get_resource::<ambition_time::SimTick>().copied().ok_or("clock_unavailable")?;
    if world.get_resource::<ambition_time::WorldTime>().is_none_or(|time| time.sim_dt() <= 0.0) {
        return Err("simulation_held");
    }
    let session = world.get::<SessionScopedEntity>(caller).map(|at| at.0).ok_or("invalid_scope")?;
    if world.get_resource::<ActiveSessionScope>().is_some_and(|active| active.current() != Some(session)) {
        return Err("inactive_scope");
    }
    let room = world.get::<InRoomInstance>(caller).map(|at| at.0).ok_or("unscoped_caller")?;
    let mut roots = world.query_filtered::<(&LiveRoomInstance, &SessionScopedEntity), With<RoomInstanceRoot>>();
    if roots.iter(world).filter(|(at, owner)| **at == room && owner.0 == session).count() != 1 {
        return Err("inactive_room");
    }
    if ambition_combat::util::body_is_untouchable(world.get::<BodyHealth>(caller),
        world.get::<ambition_combat::death_rules::OutOfPlay>(caller).is_some(), world.get::<ae::DepthPlane>(caller)) {
        return Err("inactive_body");
    }
    let mut context = Context::new(world);
    let limit = {
        let (pace, rules) = context.get(world).map_err(|_| "contact_rules_unavailable")?;
        if pace.scale_of(caller) <= 0.0 { return Err("room_held"); }
        rules.of(caller).copied().unwrap_or_default().max_world_contacts
    };
    if contact >= limit { return Err("contact_outside_budget"); }
    let step = world.get::<ae::BodyFrameOutput>(caller).ok_or("contact_unavailable")?;
    if step.stamp != Some(ae::BodyStepStamp { tick: tick.0, room: Some(room.ordinal()), session: Some(session.0) }) {
        return Err("stale_contact");
    }
    if step.events.motion_refusal.is_some() { return Err("motion_refused"); }
    let contact = step.events.contacts.get(contact as usize).ok_or("contact_unavailable")?.clone();
    let ContactSource::Block { id, kind } = contact.source else { return Err("unsupported_contact_object"); };
    let ae::geo_id::GeoSource::Placement(placement) = &id.source else { return Err("unsupported_contact_object"); };
    if id.index != 0 { return Err("unsupported_contact_object"); }
    let mut candidates = world.query_filtered::<(Entity, &FeatureId, &CenteredAabb, &BreakableFeature,
        &InRoomInstance, &SessionScopedEntity, &SimId, Option<&SpawnOrigin>), With<FeatureSimEntity>>();
    let candidates: Vec<_> = candidates.iter(world)
        .filter(|(_, feature, _, _, at, owner, _, _)| feature.as_str() == placement.as_str()
            && at.0 == room && owner.0 == session)
        .map(|(entity, _, aabb, feature, _, _, sim_id, origin)|
            (entity, aabb.center, feature.clone(), sim_id.clone(), origin.cloned())).collect();
    if candidates.len() != 1 { return Err("unsupported_contact_object"); }
    let (target, position, feature, object, origin) = candidates.into_iter().next().unwrap();
    let expected = match feature.state().collision {
        ambition_interaction::BreakableCollision::None => return Err("unowned_contact_geometry"),
        ambition_interaction::BreakableCollision::Solid => ae::BlockKind::Barrier,
        ambition_interaction::BreakableCollision::OneWayUp => ae::BlockKind::OneWay,
    };
    if kind != expected || feature.state().pogo_refresh { return Err("unowned_contact_geometry"); }
    commit(world, caller, ConsumptionTarget {entity: target, position, feature, object, origin, room, session, tick: tick.0}, exchange)
}

/// A behavior can spend its own destructible and exchange with one local body.
/// Contact admission and behavior admission use the same native transaction.
pub fn consume_self(world: &mut World, target: Entity, caller: Entity,
    exchange: &InventoryExchange) -> Result<ConsumptionReceipt, &'static str> {
    let session = world.get::<SessionScopedEntity>(target).ok_or("invalid_scope")?.0;
    if world.get::<SessionScopedEntity>(caller).map(|owner| owner.0) != Some(session) { return Err("invalid_recipient"); }
    if world.get_resource::<ActiveSessionScope>().is_some_and(|active| active.current() != Some(session)) { return Err("inactive_scope"); }
    let room = world.get::<InRoomInstance>(target).ok_or("unscoped_object")?.0;
    if world.get::<InRoomInstance>(caller).map(|at| at.0) != Some(room)
        || !shares_simulation_island(world, target, caller) { return Err("recipient_not_local"); }
    if ambition_combat::util::body_is_untouchable(world.get::<BodyHealth>(caller),
        world.get::<ambition_combat::death_rules::OutOfPlay>(caller).is_some(), world.get::<ae::DepthPlane>(caller)) {
        return Err("inactive_body");
    }
    if world.get::<ae::BodyKinematics>(caller).is_none() { return Err("recipient_not_a_body"); }
    if world.get_resource::<ambition_time::WorldTime>().is_none_or(|time| time.sim_dt() <= 0.0) { return Err("simulation_held"); }
    if live_room_root_for(world, session, room).is_none() { return Err("inactive_room"); }
    let mut context = SystemState::<RoomFreeze>::new(world);
    let pace = context.get(world).map_err(|_| "room_pace_unavailable")?;
    if pace.scale_of(target) <= 0.0 || pace.scale_of(caller) <= 0.0 { return Err("room_held"); }
    let position = world.get::<CenteredAabb>(target).ok_or("object_without_geometry")?.center;
    let feature = world.get::<BreakableFeature>(target).ok_or("object_not_destructible")?.clone();
    let object = world.get::<SimId>(target).ok_or("object_without_identity")?.clone();
    let origin = world.get::<SpawnOrigin>(target).cloned();
    let tick = world.resource::<ambition_time::SimTick>().0;
    commit(world, caller, ConsumptionTarget {entity: target, position, feature, object, origin, room, session, tick}, exchange)
}

struct ConsumptionTarget {
    entity: Entity, position: ae::Vec2, feature: BreakableFeature, object: SimId,
    origin: Option<SpawnOrigin>, room: LiveRoomInstance, session: SessionScopeId, tick: u64,
}
fn commit(world: &mut World, caller: Entity, target: ConsumptionTarget,
    exchange: &InventoryExchange) -> Result<ConsumptionReceipt, &'static str> {
    let ConsumptionTarget {entity: target, position, feature, object, origin, room, session, tick} = target;
    let prepared = simulation::prepare_consumption(&feature)?;
    let persistent = if feature.state().respawn == ambition_entity_catalog::placements::HazardRespawn::Persistent {
        let Some(SpawnOrigin::Authored { source, .. }) = origin else {
            return Err("persistent_dynamic_object_unavailable");
        };
        let ledger = world.get_resource::<AuthoredOccurrences>().ok_or("occurrence_history_unavailable")?;
        if ledger.whereabouts(&object).is_some() { return Err("object_already_recorded"); }
        if !world.contains_resource::<ConsumedSinceCheckpoint>() { return Err("occurrence_history_unavailable"); }
        let mut participants = world.query::<(&DrivingParticipant, &InRoomInstance, &SessionScopedEntity)>();
        let mut owners: Vec<_> = participants.iter(world).filter(|(_, at, owner)| at.0 == room && owner.0 == session)
            .map(|(driver, _, _)| driver.0).collect();
        owners.sort_by_key(|owner| owner.0); owners.dedup();
        Some((source, owners))
    } else { None };
    let mut effects = Effects::new(world);
    drop(effects.get_mut(world).map_err(|_| "breakable_effects_unavailable")?);

    let applied = inventory::apply(world, caller, exchange)?;
    // Inventory changes cannot remove this feature or change its prepared state.
    {
        let (mut commands, mut features, mut sfx, mut vfx, mut debris) = effects.get_mut(world)
            .expect("the checked domain services remain installed");
        let mut current = features.get_mut(target).expect("inventory preserves the checked feature");
        simulation::commit_consumption(&mut commands, target, &mut current, prepared);
        emit_breakable_destroyed(position, &mut sfx, &mut vfx.for_room(Some(room)), &mut debris);
    }
    effects.apply(world);
    if let Some((source, owners)) = persistent {
        let written = world.resource_mut::<AuthoredOccurrences>().spend([object.clone()]);
        assert_eq!(written, 1, "the inventory step preserves the checked occurrence");
        world.resource_mut::<ConsumedSinceCheckpoint>().record_spent(object.clone(), source, owners,
            Some(ConsumedWrite { at: RoomWrite { room, tick }, replaced: None }));
    }
    Ok(ConsumptionReceipt { object, exchange: applied })
}
