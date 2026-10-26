#![cfg(feature = "rl_sim")]
//! Force-region lifetime and values across GGRS save, load and replay.

use ambition_app::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::engine_core::{Aabb, Vec2};
use ambition_platformer2d::platformer::{
    frame_env::{collect_force_zones, ForceZone},
    lifecycle::{InRoomInstance, RoomScopedEntity},
    markers::PrimaryPlayerOnly,
    sim_id::SimId,
};
use ambition_platformer2d::runtime::{gravity::GravitySet, SimTick};
use ambition_platformer2d::sim::SimScheduleExt as _;
use bevy::prelude::*;

const WIND_ID: &str = "test.force_zone";

#[test]
fn a_force_zone_replays_its_birth_motion_and_retirement() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("the sync-test host builds");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    sim.rebase_rollback_history()
        .expect("the initial world becomes the baseline");
    let start = sim.world().resource::<SimTick>().get();
    let loads_before = sim.rollback_execution_stats().unwrap().lifetime_load_runs;
    let room = {
        let world = sim.world_mut();
        world
            .query_filtered::<&InRoomInstance, PrimaryPlayerOnly>()
            .single(world)
            .expect("the subject has a live room")
            .0
    };
    let region = Aabb::new(Vec2::splat(-10_000.0), Vec2::splat(20_000.0));
    let initial_accel = Vec2::new(240.0, 0.0);
    let schedule = sim.app_mut().sim_schedule();
    sim.app_mut().add_systems(
        schedule,
        (move |mut commands: Commands,
               tick: Res<SimTick>,
               mut zones: Query<(Entity, &SimId, &mut ForceZone)>| {
            let Some(elapsed) = tick.get().checked_sub(start) else {
                return;
            };
            if elapsed == 4 || elapsed == 24 {
                commands.spawn((
                    ForceZone {
                        aabb: region,
                        accel: initial_accel,
                    },
                    InRoomInstance(room),
                    RoomScopedEntity,
                    SimId::from_snapshot(WIND_ID.to_owned()),
                ));
            }
            for (entity, id, mut zone) in &mut zones {
                if id.as_str() != WIND_ID {
                    continue;
                }
                if elapsed == 16 {
                    commands.entity(entity).despawn();
                } else {
                    zone.accel += Vec2::new(2.0, -1.0);
                    zone.aabb.min += Vec2::ONE;
                    zone.aabb.max += Vec2::ONE;
                }
            }
        })
        .in_set(GravitySet::ZoneSnapshot)
        .before(collect_force_zones),
    );

    for _ in 0..64 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .expect("the force-region replay stays consistent");
    }
    assert!(sim.rollback_execution_stats().unwrap().lifetime_load_runs > loads_before);
    assert!(sim.world().resource::<SimTick>().get() >= start + 32);
    let world = sim.world_mut();
    let rows: Vec<_> = world
        .query::<(&SimId, &ForceZone, &InRoomInstance)>()
        .iter(world)
        .filter(|(id, _, _)| id.as_str() == WIND_ID)
        .map(|(_, zone, room)| (*zone, room.0))
        .collect();
    assert_eq!(rows.len(), 1, "the recreated force region has one lifetime");
    let (zone, actual_room) = rows[0];
    assert_eq!(actual_room, room);
    assert_ne!(zone.aabb.min, region.min);
    assert_ne!(zone.accel, initial_accel);
}
