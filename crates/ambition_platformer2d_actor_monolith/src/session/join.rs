//! A second seat joins the session (Q153 default, until the maintainer rules
//! how a second player joins Ambition).
//!
//! A seat with no body that presses Jump gets its home body beside the
//! primary body, in the primary's live room, wearing the primary's character.
//! The join is a simulation fact: the press is in the seat's input for the
//! frame, so a rewind across the join frame builds the same body again, and a
//! remote peer builds it on the same frame.

use bevy::prelude::*;

use ambition_characters::control::{DrivingParticipant, PlayerSlot, SlotControls};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, InRoomInstance, LiveRoomOf, SessionCommands, SessionRoot,
};
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

use crate::avatar::{HomeBodyAbilities, HomeBodyResources, InitialBodyPolicy, StartingCharacter};
use crate::session::setup::{spawn_home_body, HomeBody};

/// Build the home body of each seat that presses Jump and has no body yet.
///
/// Only a session that builds a home body for its primary seat seats a second
/// one: a match owns its whole cast (`InitialBodyPolicy`). The primary must be
/// in play, because the new body stands where the primary stands.
#[allow(clippy::too_many_arguments)]
pub fn seat_a_joining_participant(
    mut commands: SessionCommands,
    controls: Res<SlotControls>,
    active: Option<Res<ActiveSessionScope>>,
    tuning: Res<ae::ActiveMovementTuning>,
    cast: Option<Res<ambition_characters::session_cast::ActiveSessionCast>>,
    roots: Query<(&SessionRoot, &InitialBodyPolicy, &HomeBodyResources, &HomeBodyAbilities)>,
    rooms: LiveRoomOf<ae::RoomGeometry>,
    primary: Query<
        (Entity, &ae::BodyKinematics, &ambition_characters::actor::WornCharacter, Option<&InRoomInstance>),
        (With<PrimaryPlayer>, Without<ambition_combat::death_rules::OutOfPlay>),
    >,
    seated: Query<&DrivingParticipant, With<PlayerEntity>>,
) {
    let joining: Vec<PlayerSlot> = (1..SlotControls::MAX_SLOTS)
        .map(|index| PlayerSlot(index as u8))
        .filter(|seat| controls.get(*seat).jump_pressed)
        .filter(|seat| !seated.iter().any(|driver| driver.0 == *seat))
        .collect();
    if joining.is_empty() {
        return;
    }
    let Some(session) = active.as_deref().and_then(ActiveSessionScope::current) else {
        return;
    };
    let Some((_, policy, resources, abilities)) = roots.iter().find(|(root, ..)| root.0 == session) else {
        return;
    };
    if !matches!(policy, InitialBodyPolicy::SpawnCharacter(_)) {
        return;
    }
    let Ok((primary, kin, worn, room)) = primary.single() else {
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
    for seat in joining {
        let body = spawn_home_body(
            &mut commands,
            scope,
            HomeBody {
                seat,
                at: kin.pos,
                world,
                tuning: &tuning,
                character: &character,
                default_character_id: worn.id(),
                prepared_characters: cast.as_deref().and_then(|cast| cast.cast()),
                resources,
                abilities,
            },
            (),
        );
        info!(target: "ambition_platformer2d::session", "seat {} joined as {body:?}", seat.0);
    }
}
