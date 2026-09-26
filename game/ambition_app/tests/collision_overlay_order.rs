// Drives the real Ambition app, which needs the RL stepping API.
#![cfg(feature = "rl_sim")]
//! The collision overlay is complete before anything in the sim reads it.
//!
//! `FeatureEcsWorldOverlay` is the dynamic half of the collision world: the
//! rebuild clears it every frame, and lock walls, arena gates, broken bricks
//! and monitors add to it again. Brains, body modes and integration read it.
//!
//! When a reader has no edge to a writer, the executor picks an order, and a
//! change anywhere in the schedule can flip it. It did: after a room commit,
//! a returning enemy integrated against the lock wall of the room the player
//! had just left and stood 10 px from where a fresh entry puts it, but only
//! after an unrelated system was added to `ActorDecisionSet::Observe`.
//!
//! A behavioural test cannot hold this, because an unordered pair still runs
//! in some stable order and passes whether the edge exists or not. The
//! invariant is the edge, so this asks the schedule.

use crate::common::fixed_60hz_room_sim;

use ambition_platformer2d::platformer::schedule::{
    FeatureWorldOverlayContributions, FeatureWorldOverlaySet,
};
use ambition_platformer2d::sim::SimScheduleExt;
use ambition_platformer2d::world::FeatureEcsWorldOverlay;
use bevy::ecs::schedule::{NodeId, ScheduleLabel as _, SystemKey, SystemSet as _};
use bevy::prelude::*;

/// Every system in the sim schedule that reads or writes the overlay without an
/// edge to a contributor. Only contributors may be unordered among themselves:
/// each adds its own geometry, and none reads what another added.
#[test]
fn the_collision_overlay_is_complete_before_anything_reads_it() {
    let mut sim = fixed_60hz_room_sim("combat_calibration_lab");
    let app = sim.app_mut();
    let label = app.sim_schedule();
    app.world_mut()
        .resource_scope(|world, mut schedules: Mut<Schedules>| {
            let overlay = world
                .component_id::<FeatureEcsWorldOverlay>()
                .expect("the shipped sim holds the collision overlay");
            let schedule = schedules.get_mut(label.intern()).expect("the sim schedule exists");
            schedule.initialize(world).expect("the sim schedule builds");
            let graph = schedule.graph();

            let mut writers: std::collections::HashSet<SystemKey> = Default::default();
            for set in [FeatureWorldOverlaySet.intern(), FeatureWorldOverlayContributions.intern()] {
                writers.extend(graph.systems_in_set(set).expect("the overlay sets are registered"));
            }

            // ⛔ THE EXCLUSIVE WRITER IS INVISIBLE TO THE CONFLICT LIST BELOW.
            // `sync_authored_gated_lock_walls` takes `&mut World`, and Bevy records
            // an exclusive system's conflicts with no component ids, so no filter on
            // the overlay's id can see it. It was the writer behind the defect, so
            // its membership is checked by type.
            let lock_walls = System::system_type(&IntoSystem::into_system(
                ambition_platformer2d::actors::world::gated_lock_walls::sync_authored_gated_lock_walls,
            ));
            let member_types: Vec<std::any::TypeId> = schedule
                .systems()
                .expect("initialized")
                .filter(|(key, _)| writers.contains(key))
                .map(|(_, system)| System::system_type(&**system))
                .collect();
            assert!(
                member_types.contains(&lock_walls),
                "the authored lock walls write the overlay but are not in \
                 FeatureWorldOverlayContributions, so nothing orders them before \
                 the bodies that collide with them"
            );
            // Anti-vacuity: the rebuild, both lock-wall writers and the arena gates.
            assert!(
                writers.len() >= 5,
                "only {} overlay writers are in the overlay sets; the guard below \
                 would pass on an empty set",
                writers.len()
            );

            let name = |key: SystemKey| -> String {
                graph
                    .hierarchy()
                    .graph()
                    .neighbors_directed(
                        NodeId::System(key),
                        bevy::ecs::schedule::graph::Direction::Incoming,
                    )
                    .filter_map(|node| match node {
                        NodeId::Set(set) => graph.system_sets.get(set).map(|s| format!("{s:?}")),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("+")
            };
            let unordered: Vec<String> = graph
                .conflicting_systems()
                .iter()
                .filter(|(_, _, ids)| ids.contains(&overlay))
                .filter(|(a, b, _)| !(writers.contains(a) && writers.contains(b)))
                .map(|(a, b, _)| format!("[{}] and [{}]", name(*a), name(*b)))
                .collect();
            assert!(
                unordered.is_empty(),
                "{} system pair(s) read and write the collision overlay with \
                 nothing ordering them. A reader that wins the race collides with \
                 last frame's overlay, which after a room commit is the previous \
                 room's walls. A contributor joins FeatureWorldOverlayContributions; \
                 a reader orders after it:\n  {}",
                unordered.len(),
                unordered.join("\n  ")
            );
        });
}
