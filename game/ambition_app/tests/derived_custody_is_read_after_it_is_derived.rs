//! `InCustodyOf` is DERIVED rollback state: a load does not restore it, and
//! its two derivers insert it again through `Commands` each tick
//! (`project_custody_onto_residency` for items, `project_body_custody` for
//! riders, limbs and possessed bodies). So it is right only after both derivers
//! and their command flush. A simulation system that reads it earlier in the
//! tick reads the value the latest forward frame left, and a resimulated frame
//! diverges from the first run of it. The save mirror did
//! (SAVE-DIVERGES-AFTER-RELEASE). This asks the shipped schedule the same
//! question about every reader.
//!
//! ⚠ The readers are found by access: a probe system that writes
//! `InCustodyOf` conflicts with every system that reads its value. A query
//! that only filters on it (`Without<InCustodyOf>`, `RoomResident`) has no
//! access to conflict with, so it is not in this population.

#![cfg(feature = "rl_sim")]

use std::collections::{BTreeSet, HashSet, VecDeque};

use ambition_app::rl_sim::{Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::platformer::lifecycle::InCustodyOf;
use bevy::ecs::schedule::graph::Direction;
use bevy::ecs::schedule::{NodeId, ScheduleGraph, Schedules};

/// Writes `InCustodyOf`, unordered, so the schedule names every reader as a
/// conflict with it.
fn probe_writes_custody(_custody: bevy::prelude::Query<&mut InCustodyOf>) {}

const DERIVERS: [&str; 2] = ["project_custody_onto_residency", "project_body_custody"];

/// The nodes ordered after `start`: a dependency edge from a node, or from a
/// set that contains it, puts the target and everything inside the target
/// after it.
fn after(graph: &ScheduleGraph, start: NodeId) -> HashSet<NodeId> {
    let hierarchy = graph.hierarchy().graph();
    let dependency = graph.dependency().graph();
    let containers = |node: NodeId| {
        let mut found = Vec::new();
        let mut queue = vec![node];
        while let Some(next) = queue.pop() {
            for parent in hierarchy.neighbors_directed(next, Direction::Incoming) {
                found.push(parent);
                queue.push(parent);
            }
        }
        found
    };
    let members = |node: NodeId| {
        let mut found = vec![node];
        let mut queue = vec![node];
        while let Some(next) = queue.pop() {
            for child in hierarchy.neighbors_directed(next, Direction::Outgoing) {
                found.push(child);
                queue.push(child);
            }
        }
        found
    };
    let mut after = HashSet::new();
    let mut expanded = HashSet::new();
    let mut queue: VecDeque<NodeId> = std::iter::once(start).chain(containers(start)).collect();
    while let Some(node) = queue.pop_front() {
        if !expanded.insert(node) {
            continue;
        }
        for target in dependency.neighbors_directed(node, Direction::Outgoing) {
            for member in members(target) {
                if after.insert(member) {
                    queue.push_back(member);
                    queue.extend(containers(member));
                }
            }
        }
    }
    after
}

#[test]
fn every_reader_of_in_custody_of_runs_after_both_derivers() {
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            use ambition_platformer2d::sim::SimScheduleExt as _;
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.add_systems(label, probe_writes_custody);
            Ok(())
        },
    )
    .expect("the sync-test harness builds with the probe");
    let world = sim.world_mut();
    let label = world.resource::<ambition_platformer2d::sim::SimSchedule>().label();
    let custody = world
        .components()
        .component_id::<InCustodyOf>()
        .expect("the shipped sim registers InCustodyOf");
    world.resource_scope::<Schedules, _>(|world, mut schedules| {
        let schedule = schedules.get_mut(label).expect("the sim schedule exists");
        schedule.initialize(world).expect("the sim schedule builds");
        // A built schedule keeps its systems in the executable, not the graph.
        let names: std::collections::HashMap<_, String> = schedule
            .systems()
            .expect("the sim schedule is built")
            .map(|(key, system)| (key, system.name().to_string()))
            .collect();
        let graph = schedule.graph();
        let name = |key| names.get(&key).cloned().unwrap_or_default();
        let named = |part: &str| {
            let mut found: Vec<_> = names.iter().filter(|(_, name)| name.contains(part)).map(|(key, _)| *key).collect();
            found.sort();
            found
        };
        let probe = named("probe_writes_custody");
        assert_eq!(probe.len(), 1, "the probe is in the sim schedule once");
        let derivers: Vec<_> = DERIVERS.iter().flat_map(|part| named(part)).collect();
        assert_eq!(derivers.len(), DERIVERS.len(), "each deriver is in the sim schedule once: {DERIVERS:?}");
        let readers: BTreeSet<String> = graph
            .conflicting_systems()
            .iter()
            .filter(|(_, _, ids)| ids.contains(&custody))
            .filter_map(|(a, b, _)| {
                if *a == probe[0] {
                    Some(*b)
                } else if *b == probe[0] {
                    Some(*a)
                } else {
                    None
                }
            })
            .filter(|reader| !derivers.contains(reader))
            .map(name)
            .collect();
        let after_all: Vec<HashSet<NodeId>> = derivers.iter().map(|deriver| after(graph, NodeId::System(*deriver))).collect();
        let early: BTreeSet<&String> = readers
            .iter()
            .filter(|reader| {
                let key = named(reader)[0];
                !after_all.iter().all(|after| after.contains(&NodeId::System(key)))
            })
            .collect();
        assert!(
            readers.iter().any(|reader| reader.contains("persist_occurrence_horizon_to_save")),
            "the census does not find the save mirror, a reader it must find: {readers:?}"
        );
        assert_eq!(
            early,
            BTreeSet::new(),
            "these simulation systems read `InCustodyOf` without running after both of its \
             derivers, so after a rollback load they read the value of the latest forward \
             frame. All readers: {readers:?}"
        );
    });
}
