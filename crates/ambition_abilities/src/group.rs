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
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// Install the group trigger and the steering request in `actor_think`.
pub fn install(app: &mut App) {
    app.install_extension_trigger::<GroupThinkPort, _>(
        ACTOR_THINK,
        "ambition_abilities",
        queue_group_thinks.run_if(ambition_extension_host::trigger_is_bound::<GroupThinkPort>),
    );
    app.install_extension_submitted_request::<SteerPort, _>(ACTOR_THINK, "ambition_abilities", lower_steers);
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
    // A host with no assembled catalog has no tagged bodies, so no group.
    catalog: Option<Res<CharacterCatalog>>,
    mut invocations: ResMut<ExtensionInvocations>,
    bodies: Query<MemberQuery>,
    items: Query<(Entity, &GroundItem, &ae::BodyKinematics, &ItemCustody, &SimId, Has<SettledItem>)>,
    roots: Query<(Entity, &LiveRoomInstance), With<RoomInstanceRoot>>,
    ids: Query<&SimId>,
    tracked: Query<&ae::BodyKinematics>,
    rooms: LiveRooms,
    pace: ambition_platformer2d_shared_tangle::lifecycle::RoomFreeze,
) {
    if time.sim_dt() <= 0.0 {
        return;
    }
    let mut tags: Vec<String> = admitted
        .0
        .entries
        .iter()
        .filter(|e| e.descriptor.trigger.port == GroupThinkPort::KEY)
        .map(|e| e.descriptor.trigger.selector.to_string())
        .collect();
    tags.sort_unstable();
    tags.dedup();
    let root_of: BTreeMap<LiveRoomInstance, Entity> = roots.iter().map(|(entity, room)| (*room, entity)).collect();
    let mut room_items: BTreeMap<LiveRoomInstance, Vec<GroupItem>> = BTreeMap::new();
    for (entity, ground, kin, custody, id, at_rest) in items.iter() {
        let Some(room) = rooms.of(entity).filter(|_| custody.in_world()) else {
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
    for tag in tags {
        let mut groups: BTreeMap<LiveRoomInstance, Vec<GroupMember>> = BTreeMap::new();
        for (entity, form, health, kin, id, target, held, support, driven) in bodies.iter() {
            let tagged = catalog.as_ref().and_then(|catalog| catalog.get(form.id())).is_some_and(|entry| entry.tags.iter().any(|t| *t == tag));
            let Some(room) = rooms.of(entity).filter(|_| tagged) else {
                continue;
            };
            let target = target.and_then(|t| t.entity).and_then(|t| tracked.get(t).ok());
            groups.entry(room).or_default().push(GroupMember {
                id: id.as_str().to_owned(),
                form: form.id().to_owned(),
                position: [kin.pos.x, kin.pos.y],
                velocity: [kin.vel.x, kin.vel.y],
                size: [kin.size().x, kin.size().y],
                alive: health.alive(),
                driven,
                held: held.map(|held| held.id().to_owned()),
                supporter: support.and_then(|s| ids.get(s.supporter).ok()).map(|id| id.as_str().to_owned()),
                target: target.map(|t| [t.pos.x, t.pos.y]),
                target_velocity: target.map(|t| [t.vel.x, t.vel.y]),
            });
        }
        for (room, mut members) in groups {
            let Some(&root) = root_of.get(&room) else {
                continue;
            };
            members.sort_by(|a, b| a.id.cmp(&b.id));
            let items = room_items.get(&room).cloned().unwrap_or_default();
            let dt = time.sim_dt() * pace.scale_of(root);
            invocations.trigger::<GroupThinkPort>(&ACTOR_THINK, tag.clone(), root, None, false, GroupThink { dt, members, items });
        }
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

/// A take: the item owner's own take, for an item lying within reach in the
/// body's room; with a supporter, the sling between them. A body that holds
/// or supports anything, or an item taken earlier in the tick, is refused.
#[allow(clippy::too_many_arguments)]
pub fn lower_takes(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    mut bodies: CustodyBodies,
    mut items: CarriedItems,
    supporting: Query<&SupportedCarry>,
    ids: Query<&SimId>,
    rooms: LiveRooms,
) {
    let mut busy: BTreeSet<Entity> = supporting.iter().map(|carry| carry.supporter).collect();
    for submitted in outbox.drain::<TakeItemPort>(&ACTOR_THINK) {
        let take = submitted.value;
        let room = rooms.of(submitted.scope);
        let refuse = |why: &str| warn!("extension entry {} asked {:?} to take {:?}: {why}; refused", submitted.entry, take.body, take.item);
        let Some(body) = body_named(&bodies, &rooms, room, &take.body) else {
            refuse("no undriven body of its room has that identity");
            continue;
        };
        let supporter = match &take.supporter {
            None => None,
            Some(id) => match body_named(&bodies, &rooms, room, id).filter(|s| *s != body) {
                Some(supporter) => Some(supporter),
                None => {
                    refuse("its supporter is not another undriven body of the room");
                    continue;
                }
            },
        };
        let empty = |entity: Entity, bodies: &CustodyBodies| {
            !busy.contains(&entity) && bodies.get(entity).is_ok_and(|(_, _, _, hand, carrying)| hand.held.is_none() && !carrying)
        };
        if !empty(body, &bodies) || supporter.is_some_and(|s| !empty(s, &bodies)) {
            refuse("a hand is not free");
            continue;
        }
        let Some(item) = items
            .iter()
            .find(|(entity, _, _, custody, _)| custody.in_world() && rooms.of(*entity) == room && ids.get(*entity).is_ok_and(|id| id.as_str() == take.item))
            .map(|(entity, ..)| entity)
        else {
            refuse("no item of that identity lies in its room");
            continue;
        };
        let Ok((_, _, kin, mut hand, _)) = bodies.get_mut(body) else {
            continue;
        };
        let Ok((_, ground, item_body, mut custody, _)) = items.get_mut(item) else {
            continue;
        };
        let gap = (item_body.pos - kin.pos).abs() - (item_body.size() + kin.size()) * 0.5;
        if gap.max_element() > TAKE_REACH {
            refuse("the item is out of reach");
            continue;
        }
        let kin = *kin;
        take_ground_item(&mut commands, body, &mut hand, item, ground, item_body, &mut custody, &kin);
        busy.insert(body);
        if let Some(supporter) = supporter {
            commands.entity(body).insert(SupportedCarry { supporter });
            busy.insert(supporter);
        }
    }
}

/// A release: the item owner's own release, from under the body or from
/// under the middle of its shared carry, and the end of the sling.
pub fn lower_releases(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    mut bodies: CustodyBodies,
    mut items: CarriedItems,
    slings: Query<&SupportedCarry>,
    kins: Query<&ae::BodyKinematics>,
    release_world: ItemReleaseWorld,
    rooms: LiveRooms,
) {
    for submitted in outbox.drain::<ReleaseItemPort>(&ACTOR_THINK) {
        let release = submitted.value;
        let room = rooms.of(submitted.scope);
        let Some(body) = body_named(&bodies, &rooms, room, &release.body) else {
            warn!("extension entry {} asked {:?} to let go: no undriven body of its room has that identity; refused", submitted.entry, release.body);
            continue;
        };
        let Ok((_, _, kin, mut hand, _)) = bodies.get_mut(body) else {
            continue;
        };
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
        match release_carried_item(&mut commands, body, &mut hand, &mut items, &kin, &release_world, from, Vec2::from(release.velocity), way) {
            Err(refusal) => {
                warn!(?refusal, "extension entry {} asked {:?} to let go; refused", submitted.entry, release.body);
                continue;
            }
            Ok(false) => ambition_held_items::empty_hand(&mut commands, body, &mut hand),
            Ok(true) => {}
        }
        commands.entity(body).remove::<SupportedCarry>();
    }
}
