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
use ambition_combat_port::{GroupItem, GroupMember, GroupThink, GroupThinkPort, SteerPort};
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox};
use ambition_extension_sdk::phases::ACTOR_THINK;
use ambition_extension_sdk::Port;
use ambition_held_items::{GroundItem, ItemCustody, SettledItem, SupportedCarry};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms, RoomInstanceRoot};
use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;
use std::collections::BTreeMap;

/// Install the group trigger and the steering request in `actor_think`.
pub fn install(app: &mut App) {
    app.install_extension_trigger::<GroupThinkPort, _>(
        ACTOR_THINK,
        "ambition_abilities",
        queue_group_thinks.run_if(ambition_extension_host::trigger_is_bound::<GroupThinkPort>),
    );
    app.install_extension_submitted_request::<SteerPort, _>(ACTOR_THINK, "ambition_abilities", lower_steers);
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
    catalog: Option<Res<CharacterCatalog>>,
    mut invocations: ResMut<ExtensionInvocations>,
    bodies: Query<MemberQuery>,
    items: Query<(Entity, &GroundItem, &ae::BodyKinematics, &ItemCustody, &SimId, Has<SettledItem>)>,
    roots: Query<(Entity, &LiveRoomInstance), With<RoomInstanceRoot>>,
    ids: Query<&SimId>,
    tracked: Query<&ae::BodyKinematics>,
    rooms: LiveRooms,
) {
    if time.sim_dt() <= 0.0 {
        return;
    }
    let Some(catalog) = catalog else {
        return;
    };
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
            let tagged = catalog.get(form.id()).is_some_and(|entry| entry.tags.iter().any(|t| *t == tag));
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
            invocations.trigger::<GroupThinkPort>(&ACTOR_THINK, tag.clone(), root, None, false, GroupThink { members, items });
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
