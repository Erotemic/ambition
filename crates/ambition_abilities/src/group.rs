//! The adapters of a coordinated group (`ambition.actor.group_think`): one
//! invocation for each room whose live bodies carry a bound catalog tag,
//! and their steering intent. See the port cards in
//! `ambition_combat_port::group`.
//!
//! The room is the scope, so a rule that pairs two members or reserves an
//! item decides once with every member in view; the members and items are
//! named by their simulation identity, which a rewind keeps.

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_characters::actor::{BodyHealth, CharacterForm};
use ambition_characters::control::{ActorControl, DrivingParticipant};
use ambition_combat::actor_tuning::ActorConfig;
use ambition_combat::components::ActorTarget;
use ambition_combat::held_items::HeldItem;
use ambition_combat::hand::RepertoireQuery;
use ambition_combat_port::{
    GroupItem, GroupMember, GroupThink, GroupThinkPort, ReleaseItemPort, SteerPort, TakeItemPort, TAKE_REACH,
};
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox};
use ambition_extension_sdk::phases::ACTOR_THINK;
use ambition_extension_sdk::Port;
use ambition_held_items::{
    release_carried_item, take_ground_item, GroundItem, HeldMaterialBasis, ItemCustody, ItemReleaseWorld, Release,
    SettledItem, SupportedCarry,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms, RoomInstanceRoot};
use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;
use ambition_platformer2d_shared_tangle::action_results::{begin_action_request, object_action_subject, publish_action_result};
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use ambition_extension_sdk::action::ActionStage;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// Install the group trigger and the steering request in `actor_think`.
pub fn install(app: &mut App) {
    app.install_extension_trigger::<GroupThinkPort, _>(
        ACTOR_THINK,
        "ambition_abilities",
        // A host with no assembled catalog has no tagged body, so no group:
        // the shipped modules bind a group even in a host that omits it.
        queue_group_thinks
            .run_if(ambition_extension_host::trigger_is_bound::<GroupThinkPort>)
            .run_if(resource_exists::<CharacterCatalog>),
    );
    app.install_extension_submitted_request::<SteerPort, _>(ACTOR_THINK, "ambition_abilities", lower_steers);
    for domain in [ambition_combat_port::custody::TAKE_DOMAIN, ambition_combat_port::custody::RELEASE_DOMAIN] {
        ambition_platformer2d_shared_tangle::action_results::register_action_domain(app, domain,
            ambition_platformer2d_shared_tangle::action_results::ActionCancellationPolicy::Unsupported);
    }
    // Releases before takes, as the native rule frees a hand before it fills one.
    app.install_extension_submitted_request::<ReleaseItemPort, _>(ACTOR_THINK, "ambition_abilities", lower_releases);
    app.install_extension_submitted_request::<TakeItemPort, _>(ACTOR_THINK, "ambition_abilities", lower_takes);
}

/// A body that may be a member, as the trigger reads it.
type MemberQuery<'a> = (
    Entity,
    &'a CharacterForm,
    &'a BodyHealth,
    &'a ae::BodyKinematics,
    &'a SimId,
    Option<&'a ActorTarget>,
    Option<&'a HeldItem>,
    Option<&'a SupportedCarry>,
    Has<DrivingParticipant>,
);

/// For each bound tag, one invocation for each room that has a member of it,
/// each tick with a gameplay step: rooms in the order of their instance,
/// members and items in the order of their identity.
#[allow(clippy::too_many_arguments)]
pub fn queue_group_thinks(
    admitted: Res<AdmittedExtensions>,
    time: Res<ambition_time::WorldTime>,
    // Tags are the assembled catalog's; the install runs this only where one exists.
    catalog: Res<CharacterCatalog>,
    mut invocations: ResMut<ExtensionInvocations>,
    bodies: Query<MemberQuery>,
    items: Query<(Entity, &GroundItem, &ae::BodyKinematics, &ItemCustody, &SimId, Has<SettledItem>)>,
    roots: Query<(Entity, &LiveRoomInstance), With<RoomInstanceRoot>>,
    ids: Query<&SimId>,
    tracked: Query<&ae::BodyKinematics>,
    rooms: LiveRooms,
    pace: ambition_platformer2d_shared_tangle::lifecycle::RoomFreeze,
    mut bindings: Local<BTreeMap<String, Vec<String>>>,
) {
    if admitted.is_changed() || catalog.is_changed() {
        let tags: BTreeSet<&str> = admitted.0.entries.iter()
            .filter(|entry| entry.descriptor.trigger.port == GroupThinkPort::KEY)
            .map(|entry| entry.descriptor.trigger.selector.as_ref())
            .collect();
        bindings.clear();
        for (id, entry) in catalog.iter() {
            let matching: BTreeSet<_> = entry.tags.iter().filter(|tag| tags.contains(tag.as_str())).collect();
            if !matching.is_empty() {
                bindings.insert(id.clone(), matching.into_iter().cloned().collect());
            }
        }
    }
    if time.sim_dt() <= 0.0 || bindings.is_empty() {
        return;
    }
    let root_of: BTreeMap<LiveRoomInstance, Entity> = roots.iter().map(|(entity, room)| (*room, entity)).collect();
    let mut groups = BTreeMap::<(&str, LiveRoomInstance), Vec<GroupMember>>::new();
    for (entity, form, health, kin, id, target, held, support, driven) in &bodies {
        let Some((first, rest)) = bindings.get(form.id()).and_then(|tags| tags.split_first()) else { continue; };
        let Some(room) = rooms.of(entity).filter(|room| root_of.contains_key(room)) else { continue; };
        let target = target.and_then(|t| t.entity).and_then(|t| tracked.get(t).ok());
        let member = GroupMember {
            id: id.as_str().to_owned(),
            form: form.id().to_owned(),
            position: kin.pos.to_array(),
            velocity: kin.vel.to_array(),
            size: kin.size().to_array(),
            alive: health.alive(),
            driven,
            held: held.map(|held| held.id().to_owned()),
            supporter: support.and_then(|s| ids.get(s.supporter).ok()).map(|id| id.as_str().to_owned()),
            target: target.map(|t| t.pos.to_array()),
            target_velocity: target.map(|t| t.vel.to_array()),
        };
        for tag in rest {
            groups.entry((tag, room)).or_default().push(member.clone());
        }
        groups.entry((first, room)).or_default().push(member);
    }
    if groups.is_empty() {
        return;
    }
    let observed_rooms: BTreeSet<_> = groups.keys().map(|&(_, room)| room).collect();
    let mut room_items: BTreeMap<LiveRoomInstance, Vec<GroupItem>> = BTreeMap::new();
    for (entity, ground, kin, custody, id, at_rest) in items.iter() {
        let Some(room) = rooms.of(entity).filter(|room| custody.in_world() && observed_rooms.contains(room)) else {
            continue;
        };
        room_items.entry(room).or_default().push(GroupItem {
            id: id.as_str().to_owned(),
            kind: ground.spec.id.clone(),
            position: [kin.pos.x, kin.pos.y],
            at_rest,
        });
    }
    for list in room_items.values_mut() {
        list.sort_by(|a, b| a.id.cmp(&b.id));
    }
    for ((tag, room), mut members) in groups {
        let root = root_of[&room];
        members.sort_by(|a, b| a.id.cmp(&b.id));
        let items = room_items.get(&room).cloned().unwrap_or_default();
        let dt = time.sim_dt() * pace.scale_of(root);
        invocations.trigger::<GroupThinkPort>(&ACTOR_THINK, tag.to_owned(), root, None, false, GroupThink { dt, members, items });
    }
}

/// A member's steering goes into its control frame, before the control gate,
/// as the intent its movement reads: a body that flies (no gravity, or flight
/// enabled) steers `velocity_target`; one on its feet gets a locomotion
/// throttle in its own frame, `velocity / max_run_speed`, as a brain that
/// reasons in speeds writes it. The member must be in the invocation's room
/// and not driven.
#[allow(clippy::type_complexity)]
pub fn lower_steers(
    mut outbox: ResMut<ExtensionOutbox>,
    mut bodies: Query<
        (
            Entity,
            &SimId,
            &mut ActorControl,
            &ActorConfig,
            &ae::ActorSurfaceState,
            Option<&ae::BodyFlightState>,
            Option<&ResolvedMotionFrame>,
        ),
        Without<DrivingParticipant>,
    >,
    rooms: LiveRooms,
) {
    for submitted in outbox.drain::<SteerPort>(&ACTOR_THINK) {
        let room = rooms.of(submitted.scope);
        let Some((_, _, mut control, config, surface, flight, frame)) = bodies
            .iter_mut()
            .find(|(entity, id, ..)| id.as_str() == submitted.value.body && rooms.of(*entity) == room)
        else {
            warn!(
                "extension entry {} steered {:?}, which is not an undriven body of its room; refused",
                submitted.entry, submitted.value.body
            );
            continue;
        };
        let velocity = Vec2::from(submitted.value.velocity);
        let flies = surface.gravity_scale <= 0.001 || flight.is_some_and(|flight| flight.fly_enabled);
        if flies {
            control.0.velocity_target = ae::WorldVec2(velocity);
        } else {
            let basis = frame.map_or_else(|| ResolvedMotionFrame::default().basis(), ResolvedMotionFrame::basis);
            let local = ae::LocalAxes::new(velocity.dot(basis.side), velocity.dot(basis.down));
            let run = config.tuning.max_run_speed;
            control.0.locomotion =
                if run > 1e-3 { (local / run).clamp_length_max(1.0) } else { ae::LocalAxes::ZERO };
        }
        control.0.melee_pressed = false;
        control.0.melee_held = false;
        control.0.fire = None;
    }
}

/// An item lying in the world, as the item owner's take and release read it.
type CarriedItems<'w, 's> =
    Query<'w, 's, (Entity, &'static GroundItem, &'static ae::BodyKinematics, &'static mut ItemCustody, &'static HeldMaterialBasis)>;

/// A body a custody request may name, as the item owner's take and release
/// read it.
type CustodyBodies<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static SimId, &'static ae::BodyKinematics, RepertoireQuery, Has<SupportedCarry>),
    Without<DrivingParticipant>,
>;

/// The undriven body of `room` whose identity is `id`.
fn body_named(bodies: &CustodyBodies, rooms: &LiveRooms, room: Option<LiveRoomInstance>, id: &str) -> Option<Entity> {
    bodies.iter().find(|(entity, sim, ..)| sim.as_str() == id && rooms.of(*entity) == room).map(|(entity, ..)| entity)
}

/// The takes, in the order they were submitted: each is the item owner's own
/// take, for an item lying within reach in the body's room; with a
/// supporter, the sling between them. A take that asks for a reply is
/// answered with what the item owner did; one that does not has its refusal
/// logged.
pub fn lower_takes(world: &mut World) {
    let requests = world.resource_mut::<ExtensionOutbox>().drain::<TakeItemPort>(&ACTOR_THINK);
    for submitted in requests {
        let dispatch = begin_action_request::<TakeItemPort>(world, submitted.state_scope, &submitted.entry, &submitted.value);
        if !dispatch.execute {
            continue;
        }
        // Each take runs alone and its commands apply before the next, so a
        // later take sees the hands and items the earlier ones changed.
        let outcome = world
            .run_system_cached_with(take_one, (submitted.scope, submitted.value.clone()))
            .unwrap_or(Err("item_owner_unavailable"));
        match outcome {
            Ok((body, item)) => {
                let subjects = [body, item].into_iter().filter_map(|entity| object_action_subject(world, entity)).collect();
                publish_action_result(world, dispatch.reply.as_ref(), ActionStage::Applied, true, "item_taken", "", subjects);
            }
            Err(code) => {
                if dispatch.reply.is_none() {
                    warn!("extension entry {} asked {:?} to take {:?}: {code}; refused", submitted.entry, submitted.value.body, submitted.value.item);
                }
                publish_action_result(world, dispatch.reply.as_ref(), ActionStage::Rejected, true, code, "", Vec::new());
            }
        }
    }
}

/// One take, or the code of its refusal.
#[allow(clippy::too_many_arguments)]
fn take_one(
    In((scope, take)): In<(Entity, ambition_combat_port::TakeItem)>,
    mut commands: Commands,
    mut bodies: CustodyBodies,
    mut items: CarriedItems,
    supporting: Query<&SupportedCarry>,
    ids: Query<&SimId>,
    rooms: LiveRooms,
) -> Result<(Entity, Entity), &'static str> {
    let room = rooms.of(scope);
    let body = body_named(&bodies, &rooms, room, &take.body).ok_or("no_such_body")?;
    let supporter = match &take.supporter {
        None => None,
        Some(id) => Some(body_named(&bodies, &rooms, room, id).filter(|s| *s != body).ok_or("no_such_supporter")?),
    };
    let busy: BTreeSet<Entity> = supporting.iter().map(|carry| carry.supporter).collect();
    let empty = |entity: Entity| {
        !busy.contains(&entity) && bodies.get(entity).is_ok_and(|(_, _, _, hand, carrying)| hand.held.is_none() && !carrying)
    };
    if !empty(body) || supporter.is_some_and(|s| !empty(s)) {
        return Err("hand_not_free");
    }
    let item = items
        .iter()
        .find(|(entity, _, _, custody, _)| custody.in_world() && rooms.of(*entity) == room && ids.get(*entity).is_ok_and(|id| id.as_str() == take.item))
        .map(|(entity, ..)| entity)
        .ok_or("no_such_item")?;
    let (_, _, kin, mut hand, _) = bodies.get_mut(body).map_err(|_| "no_such_body")?;
    let (_, ground, item_body, mut custody, _) = items.get_mut(item).map_err(|_| "no_such_item")?;
    let gap = (item_body.pos - kin.pos).abs() - (item_body.size() + kin.size()) * 0.5;
    if gap.max_element() > TAKE_REACH {
        return Err("out_of_reach");
    }
    let kin = *kin;
    take_ground_item(&mut commands, body, &mut hand, item, ground, item_body, &mut custody, &kin);
    if let Some(supporter) = supporter {
        commands.entity(body).insert(SupportedCarry { supporter });
    }
    Ok((body, item))
}

/// The releases, in the order they were submitted: each is the item owner's
/// own release, from under the body or from under the middle of its shared
/// carry, and the end of the sling. Answered as the takes are.
pub fn lower_releases(world: &mut World) {
    let requests = world.resource_mut::<ExtensionOutbox>().drain::<ReleaseItemPort>(&ACTOR_THINK);
    for submitted in requests {
        let dispatch = begin_action_request::<ReleaseItemPort>(world, submitted.state_scope, &submitted.entry, &submitted.value);
        if !dispatch.execute {
            continue;
        }
        let outcome = world
            .run_system_cached_with(release_one, (submitted.scope, submitted.value.clone()))
            .unwrap_or(Err("item_owner_unavailable"));
        match outcome {
            Ok(body) => {
                let subjects = object_action_subject(world, body).into_iter().collect();
                publish_action_result(world, dispatch.reply.as_ref(), ActionStage::Applied, true, "item_released", "", subjects);
            }
            Err(code) => {
                if dispatch.reply.is_none() {
                    warn!("extension entry {} asked {:?} to let go: {code}; refused", submitted.entry, submitted.value.body);
                }
                publish_action_result(world, dispatch.reply.as_ref(), ActionStage::Rejected, true, code, "", Vec::new());
            }
        }
    }
}

/// One release, or the code of its refusal.
#[allow(clippy::too_many_arguments)]
fn release_one(
    In((scope, release)): In<(Entity, ambition_combat_port::ReleaseItem)>,
    mut commands: Commands,
    mut bodies: CustodyBodies,
    mut items: CarriedItems,
    slings: Query<&SupportedCarry>,
    kins: Query<&ae::BodyKinematics>,
    release_world: ItemReleaseWorld,
    rooms: LiveRooms,
) -> Result<Entity, &'static str> {
    let room = rooms.of(scope);
    let body = body_named(&bodies, &rooms, room, &release.body).ok_or("no_such_body")?;
    let (_, _, kin, mut hand, _) = bodies.get_mut(body).map_err(|_| "no_such_body")?;
    let kin = *kin;
    // The carrying geometry: the sling's middle, or under the body.
    let partner = slings
        .get(body)
        .ok()
        .filter(|carry| rooms.of(carry.supporter) == room)
        .and_then(|carry| kins.get(carry.supporter).ok());
    let from = partner.map_or(kin.pos + Vec2::Y * (kin.size().y * 0.5 + 8.0), |other| {
        SupportedCarry::anchor(&kin, other, Vec2::Y)
    });
    let way = if release.thrown { Release::Throw } else { Release::Drop };
    let released = match release_carried_item(&mut commands, body, &mut hand, &mut items, &kin, &release_world, from, Vec2::from(release.velocity), way) {
        Err(refusal) => {
            debug!(?refusal, "the item owner has no room to release {:?}'s item", release.body);
            return Err("no_room_to_release");
        }
        Ok(released) => released,
    };
    if !released {
        ambition_held_items::empty_hand(&mut commands, body, &mut hand);
    }
    commands.entity(body).remove::<SupportedCarry>();
    if released { Ok(body) } else { Err("nothing_held") }
}
