//! Start admits an unjoined participant and constructs its home body.
//!
//! Membership belongs to the session. Device changes, death and possession
//! do not admit a participant again. The join runs in simulation so rollback
//! restores both membership and body construction on the same frame.

use bevy::prelude::*;

use ambition_characters::control::{PlayerSlot, SlotControls};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_world::collision::CollisionWorld;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, InRoomInstance, LiveRoomOf, SessionCommands, SessionRoot, SessionScopedEntity,
};
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayer;

use ambition_platformer2d_actor_monolith::avatar::{HomeBodyAbilities};
use crate::initial_body::{HomeBodyResources, InitialBodyPolicy, StartingCharacter};
use crate::setup::{spawn_home_body, HomeBody};

/// Build a home body when an unjoined participant presses Start.
///
/// Only a session that builds a home body for its primary seat seats a second
/// one: a match owns its whole cast (`InitialBodyPolicy`). The primary must be
/// in play, because the new body enters its live room.
#[allow(clippy::too_many_arguments)]
pub fn seat_a_joining_participant(sprite_content: Res<ambition_sprite_sheet::SpriteContent>,
    mut commands: SessionCommands,
    controls: Res<SlotControls>,
    active: Option<Res<ActiveSessionScope>>,
    tuning: Res<ae::ActiveMovementTuning>,
    cast: Option<Res<ambition_characters::session_cast::ActiveSessionCast>>,
    roots: Query<(&SessionRoot, &InitialBodyPolicy, &HomeBodyResources, &HomeBodyAbilities, &crate::participants::SessionParticipants)>,
    rooms: LiveRoomOf<ae::RoomGeometry>,
    collision: CollisionWorld,
    primary: Query<
        (Entity, &ae::BodyKinematics, &ambition_characters::actor::WornCharacter, Option<&InRoomInstance>, Option<&ae::SweepSample>),
        (With<PrimaryPlayer>, Without<ambition_combat::death_rules::OutOfPlay>),
    >,
    bodies: Query<(
        &ae::BodyKinematics,
        Option<&ae::SweepSample>,
        Option<&InRoomInstance>,
        Option<&SessionScopedEntity>,
        Option<&ae::DepthPlane>,
    )>,
) {
    if !(1..SlotControls::MAX_SLOTS).any(|index| controls.get(PlayerSlot(index as u8)).start_pressed) {
        return;
    }
    let Some(session) = active.as_deref().and_then(ActiveSessionScope::current) else {
        return;
    };
    let Some((_, policy, resources, abilities, members)) = roots.iter().find(|(root, ..)| root.0 == session) else {
        return;
    };
    let joining: Vec<PlayerSlot> = (1..SlotControls::MAX_SLOTS)
        .map(|index| PlayerSlot(index as u8))
        .filter(|seat| controls.get(*seat).start_pressed && !members.contains(*seat))
        .collect();
    if joining.is_empty() { return; }
    if !matches!(policy, InitialBodyPolicy::SpawnCharacter(_)) {
        return;
    }
    let Ok((primary, kin, worn, room, sample)) = primary.single() else {
        return;
    };
    let Some(world) = rooms.of(primary) else {
        return;
    };
    let Some(scope) = commands.spawn_scope() else {
        return;
    };
    let scope = scope.in_room(room.map(|room| room.0));
    let character = StartingCharacter::new(worn.id());
    let Some(walls) = collision.room(room).and_then(|room| room.solids()) else {
        return;
    };
    let reference = kin.collision_box(sample);
    let frame = ae::AccelerationFrame::new(sample.map_or(ae::DEFAULT_GRAVITY_DIR, |sample| sample.down));
    let mut occupied: Vec<_> = bodies.iter()
        .filter(|(_, _, body_room, owner, depth)| {
            body_room.copied() == room.copied()
                && owner.is_none_or(|owner| owner.0 == session)
                && depth.is_none_or(|depth| depth.is_playable())
        })
        .map(|(kin, sample, ..)| kin.collision_box(sample))
        .collect();
    for seat in joining {
        let mut body = HomeBody {
            seat,
            at: kin.pos,
            world,
            tuning: &tuning,
            character: &character,
            default_character_id: worn.id(),
            prepared_characters: cast.as_deref().and_then(|cast| cast.cast()),
            resources,
            abilities,
        };
        let shape = body.placement_shape(&sprite_content);
        let Some(at) = ambition_platformer2d_world::rooms::spawn_near_body(
            &walls, reference, frame, shape, &occupied,
        ) else {
            info!(target: "ambition_platformer2d::session", "seat {} could not join: no clear position near the primary body", seat.0);
            continue;
        };
        body.at = at;
        let body = spawn_home_body(&sprite_content, &mut commands, scope, body, ());
        crate::participants::admit_participant(&mut commands, scope, seat);
        occupied.push(ae::Aabb::new(at, shape.body_half));
        info!(target: "ambition_platformer2d::session", "seat {} joined as {body:?}", seat.0);
    }
}
