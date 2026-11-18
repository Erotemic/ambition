//! A technique's transit (`ambition.motion.transit` in `technique_execution`).
//!
//! A transit is travel, and the readers of a body's travelled path
//! (constraints, contacts, portal and room crossings) run in the player phase,
//! before `technique_execution` (in the combat phase). A transit lowered there
//! would arrive after them, so a body could blink through a portal no
//! crossing saw. A technique's transit is therefore held on the body
//! ([`PendingTransit`]) and moved in the next tick's carry
//! (`BodyPathSet::Carry`), the place every other transit moves: one tick
//! later, and every reader of the path sees it.

use ambition_combat_port::{Destination, Transit, TransitPort};
use ambition_extension_host::{ExtensionAppExt, ExtensionOutbox};
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
use ambition_platformer2d_shared_tangle::schedule::{BodyPathSet, GameplayGated, SimScheduleExt};
use bevy::prelude::*;

/// Install the technique transit's lowering, and its move in the carry.
pub fn install_for_technique_execution(app: &mut App) {
    app.install_extension_submitted_request::<TransitPort, _>(TECHNIQUE_EXECUTION, "ambition_abilities", hold_transits);
    let sim = app.sim_schedule();
    app.add_systems(
        sim,
        move_pending_transits
            .run_if(any_with_component::<PendingTransit>)
            .in_set(BodyPathSet::Carry)
            .in_set(GameplayGated),
    );
}

/// The transits a technique asked for a body, in the order asked, moved in
/// the next tick's carry.
///
/// Rollback state: it decides where the body is next tick.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PendingTransit {
    pub transits: Vec<Transit>,
    /// The live room the body was in when they were asked: a body that has
    /// left it since (a room edge, a death) does not take them.
    pub room: Option<InRoomInstance>,
}

/// The rollback value projection: each destination, its facing and the room.
pub fn pending_transit_probe(pending: &PendingTransit) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut fold = |bits: u32| hash = (hash ^ bits as u64).wrapping_mul(0x0100_0000_01b3);
    for transit in &pending.transits {
        match transit.to {
            Destination::Along { direction, distance } => {
                fold(0);
                fold(direction[0].to_bits());
                fold(direction[1].to_bits());
                fold(distance.to_bits());
            }
            Destination::To(at) => {
                fold(1);
                fold(at[0].to_bits());
                fold(at[1].to_bits());
            }
        }
        fold(transit.facing.map_or(u32::MAX, f32::to_bits));
    }
    let room = pending.room.map_or(u64::MAX, |stamp| {
        let mut hasher = std::hash::DefaultHasher::new();
        std::hash::Hash::hash(&stamp, &mut hasher);
        std::hash::Hasher::finish(&hasher)
    });
    hash ^ room.rotate_left(17)
}

/// Hold each technique transit on its body.
fn hold_transits(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    mut bodies: Query<(Option<&mut PendingTransit>, Option<&InRoomInstance>), With<ambition_platformer2d_core::BodyKinematics>>,
) {
    let mut held: Vec<(Entity, PendingTransit)> = Vec::new();
    for submitted in outbox.drain::<TransitPort>(&TECHNIQUE_EXECUTION) {
        let Ok((pending, room)) = bodies.get_mut(submitted.scope) else {
            continue;
        };
        if !finite(&submitted.value) {
            warn!("extension entry {} asked for the transit {:?}; refused", submitted.entry, submitted.value);
            continue;
        }
        match pending {
            Some(mut pending) => pending.transits.push(submitted.value),
            None => match held.iter_mut().find(|(body, _)| *body == submitted.scope) {
                Some((_, pending)) => pending.transits.push(submitted.value),
                None => held.push((submitted.scope, PendingTransit { transits: vec![submitted.value], room: room.copied() })),
            },
        }
    }
    for (body, pending) in held {
        commands.entity(body).try_insert(pending);
    }
}

fn finite(transit: &Transit) -> bool {
    let at = match transit.to {
        Destination::Along { direction, distance } => [direction[0], direction[1], distance],
        Destination::To(at) => [at[0], at[1], 0.0],
    };
    at.iter().all(|v| v.is_finite()) && transit.facing.is_none_or(f32::is_finite)
}

/// Move each body by the transits it holds, and let them go.
pub fn move_pending_transits(
    mut commands: Commands,
    pending: Query<(Entity, &PendingTransit)>,
    world: ambition_platformer2d_world::collision::CollisionWorld,
    mut bodies: crate::extension::TransitBodies,
    mut class_b: Option<ResMut<ambition_platformer2d_shared_tangle::class_b::ClassBRemapLog>>,
) {
    let mut held: Vec<(Entity, &PendingTransit)> = pending.iter().collect();
    // In entity order, so two bodies move in the same order on every peer.
    held.sort_by_key(|(body, _)| *body);
    for (body, pending) in held {
        commands.entity(body).remove::<PendingTransit>();
        let here = bodies.get(body).ok().and_then(|(_, _, room)| room.copied());
        if here != pending.room {
            continue;
        }
        for transit in &pending.transits {
            crate::extension::transit(&world, &mut bodies, class_b.as_deref_mut(), body, transit);
        }
    }
}

