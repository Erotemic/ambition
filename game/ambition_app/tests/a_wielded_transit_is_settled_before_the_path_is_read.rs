//! A wielded transit is done before the readers of the travelled path run.
//!
//! A blink, a dive and a recall are transits: each collapses the body's
//! record (`SweepSample`) to a zero-length sample at the arrival. They run in
//! `ItemPickupSet::WieldedAbilities`, in the same phase as the travelled-path
//! contract (`BodyPathSet`), and no edge was stated between the two. So
//! whether the ECS hazards (`BodyPathSet::Contacts`) read the path the kernel
//! wrote or the collapsed record was the result of the schedule's sort.
//!
//! Measured in the shipped schedule on 2026-10-06, before the edge: recall,
//! blink and dive were systems 266, 268 and 272 of 662, and
//! `apply_hazard_contacts` was 294. The edge states that order: the hazards
//! read the record after a wielded transit, so a body that blinks into a
//! hazard is hit on the tick it arrives. The other direction is the open
//! question of LEVEL-BOX-READERS in the planning queue.
//!
//! The loading zones read the record a phase later, after every transit of
//! the tick. That order was already stated, and this holds it too.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use bevy::ecs::schedule::{NodeId, Schedules};

use crate::derived_custody_is_read_after_it_is_derived::after;

/// The wielded abilities that transit a body: the native recall, and the
/// adapter of `ambition.motion.transit` (the blink and dive modules ask it).
const TRANSITS: [&str; 2] = ["::lower_transits", "::mark_recall_system"];
/// The readers of the record: the ECS hazards and the loading zones.
const READERS: [&str; 2] = ["::apply_hazard_contacts", "::detect_room_transition_system"];

#[test]
fn every_reader_of_the_record_runs_after_each_wielded_transit() {
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
        |app, options| ambition_app::rl_sim::ambition_sim_composition(app, options),
    )
    .expect("the sync-test harness builds");
    let world = sim.world_mut();
    let label = world.resource::<ambition_platformer2d::sim::SimSchedule>().label();
    world.resource_scope::<Schedules, _>(|world, mut schedules| {
        let schedule = schedules.get_mut(label).expect("the sim schedule exists");
        schedule.initialize(world).expect("the sim schedule builds");
        let names: Vec<_> = schedule
            .systems()
            .expect("the sim schedule is built")
            .map(|(key, system)| (key, system.name().to_string()))
            .collect();
        let graph = schedule.graph();
        let the_one = |part: &str| {
            let found: Vec<_> = names.iter().filter(|(_, name)| name.ends_with(part)).map(|(key, _)| *key).collect();
            assert_eq!(found.len(), 1, "`{part}` is in the sim schedule once");
            found[0]
        };
        let mut unordered = Vec::new();
        for transit in TRANSITS {
            let later = after(graph, NodeId::System(the_one(transit)));
            for reader in READERS {
                if !later.contains(&NodeId::System(the_one(reader))) {
                    unordered.push(format!("{reader} is not ordered after {transit}"));
                }
            }
        }
        assert!(
            unordered.is_empty(),
            "a reader of the record must be ordered after each wielded transit by a stated edge, \
             not by the sort of the schedule:\n  {}",
            unordered.join("\n  ")
        );
    });
}
