//! The fall rescue: a room that catches a fall (`RoomMetadata::fall_rescue`).
//!
//! The Mockingbird's air chase is fought on burning sharks fleeing across an
//! open sky. Jon (2026-10-06): a player who falls off "is caught by a shark
//! and carried back up, so the fight keeps going". A driven body that falls
//! toward the room's bottom gets a FERRY (`MovingPlatformState::ferry`, drawn
//! as the room's carrier sheet) spawned right under its feet: it rises into
//! play, then flies off with the flock and, past the room's left edge, is
//! taken away.
//!
//! Only while the room's fight is on: once every boss in the room is dead, a
//! fall is a fall, and the bottom's exit takes the body to the room below
//! (where the boss's treasure fell).

use ambition_platformer2d_core as ae;
use ambition_platformer2d_world::collision::MovingPlatformSet;
use ambition_platformer2d_world::platforms::MovingPlatformState;
use bevy::prelude::*;

/// A falling body is caught when its feet come within this of the room's
/// bottom: high enough that the bottom's exit (a strip along the edge) is
/// never reached first.
pub const RESCUE_LINE_PX: f32 = 64.0;
/// The ferry's size: a shark's back.
pub const FERRY_SIZE: ae::Vec2 = ae::Vec2::new(120.0, 22.0);
/// How fast it rises, and where to: a little under the room's middle.
pub const FERRY_RISE_SPEED: f32 = 300.0;
pub const FERRY_RISE_TO: f32 = 0.56;
/// Risen, it flies off with the flock (left, toward the boss) at this speed.
pub const FERRY_DRIFT: f32 = -85.0;
/// The id every ferry's starts with.
pub const FERRY_ID_PREFIX: &str = "fall_rescue";

/// Catch every falling driven body in a room that catches falls, and take away
/// the ferries whose rescue is over.
///
/// Before the platforms advance, so a ferry spawned under a body this tick
/// rises under it with the rest of the room's platforms.
#[allow(clippy::type_complexity)]
pub fn rescue_falling_bodies(
    mut rooms: Query<
        (Entity, &mut MovingPlatformSet, &ae::RoomGeometry),
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
    specs: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    mut bodies: Query<
        (
            Entity,
            &mut ae::BodyKinematics,
            &ae::BodyGroundState,
            Option<&mut ae::SweepSample>,
        ),
        With<ambition_characters::control::DrivingParticipant>,
    >,
    bosses: Query<
        (Entity, &ambition_characters::actor::BodyHealth),
        With<ambition_boss_encounter::BossConfig>,
    >,
    tick: Option<Res<ambition_time::SimTick>>,
) {
    let tick = tick.map_or(0, |tick| tick.0);
    for (root, mut platforms, geometry) in &mut rooms {
        if platforms.0.iter().any(|p| p.is_ferry() && p.is_spent()) {
            platforms.0.retain(|p| !(p.is_ferry() && p.is_spent()));
        }
        let Some(carrier) = specs
            .definition_of(root)
            .and_then(|definition| specs.rooms().spec(definition).metadata.fall_rescue.clone())
        else {
            continue;
        };
        let room = specs.live().of(root);
        // The fight is over once every boss in the room is dead.
        let mut in_room = bosses.iter().filter(|(boss, _)| specs.live().of(*boss) == room).peekable();
        if in_room.peek().is_some() && in_room.all(|(_, health)| !health.alive()) {
            continue;
        }
        let size = geometry.0.size;
        for (body, mut kin, ground, sweep) in &mut bodies {
            if specs.live().of(body) != room || ground.on_ground || kin.vel.y <= 0.0 {
                continue;
            }
            let feet = kin.pos.y + kin.size.y * 0.5;
            if feet < size.y - RESCUE_LINE_PX {
                continue;
            }
            let half = FERRY_SIZE * 0.5;
            // One carrier a fall: a body already over a ferry is being caught.
            let caught = platforms.0.iter().any(|p| {
                p.is_ferry()
                    && (p.pos.x - kin.pos.x).abs() < half.x + kin.size.x * 0.5
                    && p.pos.y - half.y >= feet - 8.0
            });
            if caught {
                continue;
            }
            let x = kin.pos.x.clamp(half.x, (size.x - half.x).max(half.x));
            let ferry = MovingPlatformState::ferry(
                format!("{FERRY_ID_PREFIX}:{tick}:{}", platforms.0.len()),
                "Rescue carrier",
                ae::Vec2::new(x, feet + half.y + 1.0),
                FERRY_SIZE,
                size.y * FERRY_RISE_TO,
                FERRY_RISE_SPEED,
                FERRY_DRIFT,
                -FERRY_SIZE.x,
            )
            .with_visual(carrier.clone())
            // Like the flock: a body lands on its back from above.
            .one_way();
            platforms.0.push(ferry);
            // Caught: the fall stops on the carrier's back.
            let (pos, vel) = (kin.pos, ae::Vec2::new(kin.vel.x * 0.5, 0.0));
            ae::movement::constrain_body_pose(&mut kin, sweep.map(|sweep| sweep.into_inner()), pos, vel);
        }
    }
}
