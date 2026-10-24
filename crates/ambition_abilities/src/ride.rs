//! The adapters of a summoned ride: `ambition.world.summon_ride`, the facts
//! `ambition.body.ride`, `ambition.body.saddle`, `ambition.body.presses`,
//! `ambition.body.tumbling` and `ambition.body.room`, and the requests
//! `ambition.mount.dismount`, `ambition.body.hold_course` and
//! `ambition.world.dismiss`. See the port cards in
//! `ambition_combat_port::ride`.
//!
//! The mount system keeps riding; this keeps only the course a body holds,
//! so the module that summons a ride needs no component of its own.

use ambition_characters::control::ActorControl;
use ambition_combat_port::{
    BodyPresses, BodyPressesPort, BodyRoom, BodyRoomPort, DismissPort, DismountPort, DismountWhy, HoldCoursePort, Ride,
    RidePort, Saddle, SaddlePort, SummonRidePort, TumblingPort,
};
use ambition_extension_host::{ExtensionAppExt, ExtensionOutbox};
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use ambition_mount::{DismountReason, DismountRequested, MountReservedFor, MountSlot, RidingOn};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::construction::SpawnOrigin;
use ambition_platformer2d_shared_tangle::schedule::{ActorDecisionSet, GameplayGated, PlayerInputSet, SimScheduleExt};
use bevy::prelude::*;

/// Install the summoned ride's ports in `technique_execution`, and the steer
/// of a held course.
pub fn install_for_technique_execution(app: &mut App) {
    app.add_message::<ambition_vfx::EffectRequest>();
    app.add_message::<DismountRequested>();
    app.install_extension_submitted_request::<SummonRidePort, _>(TECHNIQUE_EXECUTION, "ambition_abilities", lower_summon_rides);
    app.install_extension_observation::<RidePort>(TECHNIQUE_EXECUTION, "ambition_abilities", ride_of);
    app.install_extension_observation::<SaddlePort>(TECHNIQUE_EXECUTION, "ambition_abilities", saddle_of);
    app.install_extension_observation::<BodyPressesPort>(TECHNIQUE_EXECUTION, "ambition_abilities", presses_of);
    app.install_extension_observation::<TumblingPort>(TECHNIQUE_EXECUTION, "ambition_abilities", tumbling_of);
    app.install_extension_observation::<BodyRoomPort>(TECHNIQUE_EXECUTION, "ambition_abilities", room_of);
    app.install_extension_submitted_request::<DismountPort, _>(TECHNIQUE_EXECUTION, "ambition_abilities", lower_dismounts);
    app.install_extension_submitted_request::<HoldCoursePort, _>(TECHNIQUE_EXECUTION, "ambition_abilities", lower_held_courses);
    app.install_extension_submitted_request::<DismissPort, _>(TECHNIQUE_EXECUTION, "ambition_abilities", lower_dismissals);
    // After the brains publish, so a brain cannot write over the course
    // before movement, and before the control gate, so every restriction
    // sees it.
    let sim = app.sim_schedule();
    app.add_systems(
        sim,
        steer_held_courses
            .run_if(any_with_component::<HeldCourse>)
            .after(ActorDecisionSet::Publish)
            .before(PlayerInputSet::ControlGate)
            .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::WorldPrep)
            .in_set(GameplayGated),
    );
}

/// The course a body steers in place of its brain
/// (`ambition.body.hold_course`).
///
/// Rollback state: it decides where the body goes each tick.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct HeldCourse(pub Vec2);

/// The rollback value projection: both axes of the velocity.
pub fn held_course_probe(course: &HeldCourse) -> u64 {
    ((course.0.x.to_bits() as u64) << 32) ^ course.0.y.to_bits() as u64
}

/// Summon each ride a module asked for, where its asker is.
fn lower_summon_rides(
    mut outbox: ResMut<ExtensionOutbox>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    bodies: Query<&BodyKinematics>,
) {
    for submitted in outbox.drain::<SummonRidePort>(&TECHNIQUE_EXECUTION) {
        let summon = submitted.value;
        if summon.label.is_empty() {
            warn!("extension entry {} asked for a ride with no label; refused", submitted.entry);
            continue;
        }
        let Ok(kin) = bodies.get(submitted.scope) else {
            continue;
        };
        effects.write(ambition_vfx::EffectRequest {
            owner: submitted.scope,
            effect: ambition_vfx::Effect::Summon(ambition_vfx::SummonSpec {
                // Where the asker is, so boarding is a weld, not a teleport.
                pos: kin.pos,
                half_size: Vec2::from(summon.half_size),
                character_id: summon.character_id,
                // A ride belongs to no boss's encounter.
                encounter_id: summon.label.clone(),
                id: summon.label,
                faction: ambition_vfx::HitSide::Neutral,
                ridden_by_summoner: Some(ambition_vfx::SummonedRide {
                    seconds: summon.seconds,
                    board_within: summon.board_within,
                    board_deadline_s: summon.board_deadline_s,
                }),
                health: summon.health,
                keeps_contact_damage: summon.keeps_contact_damage,
                holds: if summon.holds.is_empty() { Vec::new() } else { vec![summon.holds] },
            }),
        });
    }
}

/// What `scope` rides (`ambition.body.ride`).
fn ride_of(world: &World, scope: Entity) -> Option<Ride> {
    if let Some(riding) = world.get::<RidingOn>(scope) {
        let mount_alive =
            world.get::<ambition_characters::actor::BodyHealth>(riding.mount).is_some_and(|health| health.alive());
        return Some(Ride::Aboard { mount_alive });
    }
    let Some(mut reservations) = world.try_query::<&MountReservedFor>() else {
        return Some(Ride::None);
    };
    let awaiting = reservations.iter(world).any(|reservation| reservation.rider == scope);
    Some(if awaiting { Ride::Awaiting } else { Ride::None })
}

/// Who is in `scope`'s saddle, or coming (`ambition.body.saddle`).
fn saddle_of(world: &World, scope: Entity) -> Option<Saddle> {
    Some(Saddle {
        ridden: world.get::<MountSlot>(scope).is_some_and(|slot| slot.rider.is_some()),
        awaiting: world.get::<MountReservedFor>(scope).is_some(),
    })
}

/// The edges `scope`'s control pressed (`ambition.body.presses`).
fn presses_of(world: &World, scope: Entity) -> Option<BodyPresses> {
    world
        .get::<ActorControl>(scope)
        .map(|control| BodyPresses { jump: control.0.jump_pressed, burst: control.0.burst_pressed })
}

/// Whether `scope` tumbles (`ambition.body.tumbling`).
fn tumbling_of(world: &World, scope: Entity) -> Option<bool> {
    world.get::<ambition_platformer2d_core::BodyMotionFacts>(scope).map(|facts| facts.tumbling)
}

/// The size of `scope`'s live room (`ambition.body.room`).
fn room_of(world: &World, scope: Entity) -> Option<BodyRoom> {
    let room = world.get::<ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>(scope);
    let size = ambition_platformer2d_world::collision::with_room_in_world(world, room, |room| {
        let size = room.base().size;
        [size.x, size.y]
    });
    Some(BodyRoom { size })
}

/// Ask the mount system to take each rider off, once per rider per tick.
fn lower_dismounts(mut outbox: ResMut<ExtensionOutbox>, mut dismounts: MessageWriter<DismountRequested>) {
    // The mount system applies every request it reads, and a second one for
    // a rider whose first is still deferred would announce a second empty
    // saddle.
    let mut asked: Vec<Entity> = Vec::new();
    for submitted in outbox.drain::<DismountPort>(&TECHNIQUE_EXECUTION) {
        if asked.contains(&submitted.scope) {
            continue;
        }
        asked.push(submitted.scope);
        let reason = match submitted.value.why {
            DismountWhy::Bailed => DismountReason::RiderBailed,
            DismountWhy::Launched => DismountReason::RiderLaunched,
            DismountWhy::LeftPlay => DismountReason::RiderLeftPlay,
            DismountWhy::MountLost => DismountReason::MountLost,
        };
        dismounts.write(DismountRequested { rider: submitted.scope, reason });
    }
}

/// Hold or let go of each body's course, in the order asked.
fn lower_held_courses(mut commands: Commands, mut outbox: ResMut<ExtensionOutbox>, bodies: Query<(), With<BodyKinematics>>) {
    for submitted in outbox.drain::<HoldCoursePort>(&TECHNIQUE_EXECUTION) {
        if !bodies.contains(submitted.scope) {
            continue;
        }
        match submitted.value.velocity {
            Some(velocity) if velocity.iter().all(|v| v.is_finite()) => {
                commands.entity(submitted.scope).try_insert(HeldCourse(Vec2::from(velocity)));
            }
            Some(velocity) => {
                warn!("extension entry {} asked for the course {velocity:?}; refused", submitted.entry);
            }
            None => {
                commands.entity(submitted.scope).try_remove::<HeldCourse>();
            }
        }
    }
}

/// Remove each summoned body a module dismissed.
fn lower_dismissals(mut commands: Commands, mut outbox: ResMut<ExtensionOutbox>, origins: Query<&SpawnOrigin>) {
    let mut gone: Vec<Entity> = Vec::new();
    for submitted in outbox.drain::<DismissPort>(&TECHNIQUE_EXECUTION) {
        if !matches!(origins.get(submitted.scope), Ok(SpawnOrigin::Dynamic { .. })) {
            warn!(
                "extension entry {} asked to dismiss {:?}, which the simulation did not make; refused",
                submitted.entry, submitted.scope
            );
            continue;
        }
        gone.push(submitted.scope);
    }
    gone.sort();
    gone.dedup();
    for body in gone {
        commands.entity(body).despawn();
    }
}

/// Steer each body that holds a course by it: an intent, so one integrator
/// owns the position.
pub fn steer_held_courses(mut bodies: Query<(&HeldCourse, &mut ActorControl)>) {
    for (course, mut control) in &mut bodies {
        control.0.velocity_target = ambition_platformer2d_core::WorldVec2(course.0);
        control.0.locomotion = Default::default();
    }
}

#[cfg(test)]
mod tests;
