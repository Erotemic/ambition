//! A participant leaves the session: membership ends and its body retires.
//!
//! The lifetime policy: the participant's player bodies are despawned, and
//! every object one of them holds is put down where that body stood. The bag
//! (`OwnedItems`) belongs to the session, so nothing in it moves. No room is
//! reset. The leave runs in simulation from a stamped host intent, so a rewind
//! across it restores the membership and the body, and replays the leave on
//! the same tick.
//!
//! The primary participant does not leave this way: it owns the session's
//! one-per-session facts (the checkpoint subject, the primary live room).
//! Quitting to the title is its leave.

use bevy::prelude::*;

use ambition_characters::control::{DrivingParticipant, PlayerSlot};
use ambition_held_items::{GroundItem, ItemCustody, SupportedCarry};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::lifecycle::{ActiveSessionScope, SessionRoot, SessionScopedEntity};
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;

use crate::participants::SessionParticipants;

/// Ask the simulation to remove a participant from the session (host intent).
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParticipantLeaveRequested {
    pub participant: PlayerSlot,
}

/// Retire each participant whose leave was released this tick.
///
/// Requests for the primary seat, or for a seat that is not a member, change
/// nothing.
#[allow(clippy::type_complexity)]
pub fn retire_leaving_participants(
    mut requests: MessageReader<ParticipantLeaveRequested>,
    mut commands: Commands,
    active: Option<Res<ActiveSessionScope>>,
    mut roots: Query<(&SessionRoot, &mut SessionParticipants)>,
    bodies: Query<
        (Entity, &DrivingParticipant, &BodyKinematics, Option<&SessionScopedEntity>),
        With<PlayerEntity>,
    >,
    mut items: Query<(Entity, &BodyKinematics, &mut ItemCustody, &ambition_held_items::HeldMaterialBasis), (With<GroundItem>, Without<PlayerEntity>)>,
    supported: Query<(Entity, &SupportedCarry)>,
    release_world: ambition_held_items::ItemReleaseWorld,
) {
    let leaving: Vec<PlayerSlot> = requests.read().map(|request| request.participant).collect();
    if leaving.is_empty() {
        return;
    }
    let Some(session) = active.as_deref().and_then(ActiveSessionScope::current) else {
        return;
    };
    let Some((_, mut members)) = roots.iter_mut().find(|(root, _)| root.0 == session) else {
        return;
    };
    for seat in leaving {
        if seat == PlayerSlot(0) || !members.contains(seat) {
            info!(target: "ambition_platformer2d::session", "seat {} cannot leave: it is the primary or not a member", seat.0);
            continue;
        }
        let retiring: Vec<(Entity, BodyKinematics)> = bodies
            .iter()
            .filter(|(_, driver, _, scope)| driver.0 == seat && scope.is_some_and(|scope| scope.0 == session))
            .map(|(body, _, kin, _)| (body, *kin))
            .collect();
        let prepared = (|| {
            let mut releases = Vec::new();
            for &(body, holder) in &retiring {
                for (item, kin, custody, basis) in &items {
                    if custody.held_by(body) {
                        releases.push((item, release_world.prepare_release(body, Some(item), &holder, kin, *basis, holder.pos,
                            ambition_platformer2d_core::Vec2::ZERO)?));
                    }
                }
            }
            Ok::<_, ambition_platformer2d_core::frame::thread::Refusal>(releases)
        })();
        let releases = match prepared {
            Ok(releases) => releases,
            Err(refusal) => { warn!(?refusal, "participant leave refused"); continue; }
        };
        for (item, prepared) in releases {
            if let Ok((_, _, mut custody, _)) = items.get_mut(item) {
                prepared.put_down(&mut commands, item, &mut custody);
            }
        }
        members.remove(seat);
        for &(body, _) in &retiring {
            // A body that helped carry another's load stops helping; the
            // holder keeps the load.
            for (holder, carry) in &supported {
                if carry.supporter == body {
                    commands.entity(holder).remove::<SupportedCarry>();
                }
            }
            commands.entity(body).despawn();
        }
        info!(target: "ambition_platformer2d::session", "seat {} left; retired {} bodies", seat.0, retiring.len());
    }
}
