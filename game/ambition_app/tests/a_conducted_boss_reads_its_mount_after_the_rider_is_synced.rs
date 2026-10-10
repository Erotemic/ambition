//! A conducted boss reads its mount after the rider is synced to it.
//!
//! `ambition.boss.mount` gives a conductor the body it rides. GNU-ton's
//! native conductor ran after `RidersSyncedToMounts`, so it read the scholar
//! on the gnu's saddle for this tick. The `boss_conduct` phase and the mount
//! sync are both in `WorldPrepSet::AfterIntegrate`, and no edge was stated
//! between them before 2026-10-10, so the order was the sort's.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_app::AmbitionSim as _;
use bevy::ecs::schedule::{NodeId, Schedules};

use crate::derived_custody_is_read_after_it_is_derived::after;

#[test]
fn the_conduct_collector_runs_after_the_rider_sync() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz()),
    )
    .expect("the sandbox builds");
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
        let the_one = |part: &str| {
            let found: Vec<_> = names.iter().filter(|(_, name)| name.ends_with(part)).map(|(key, _)| *key).collect();
            assert_eq!(found.len(), 1, "`{part}` is in the sim schedule once");
            found[0]
        };
        let later = after(schedule.graph(), NodeId::System(the_one("::sync_riders_to_mounts")));
        assert!(
            later.contains(&NodeId::System(the_one("::queue_boss_conducts"))),
            "the boss_conduct collector must be ordered after the rider sync by a stated edge"
        );
    });
}
