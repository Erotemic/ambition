//! DIAGNOSTIC TOOL for the pinned duel replay gap (see
//! `a_staged_restore_rebuilds_the_duel_roster_completely` tooth 3): names the
//! registered entries that diverge at tick 0 of the duel oracle's replay and
//! prints the restore report. `#[ignore]`d — run explicitly while closing the
//! gap, then delete alongside the pin.
#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::TimestepMode;
use ambition_app::AmbitionSim;
use ambition_app::{RandomWalkPolicy, SandboxSim, SandboxSimOptions};

#[test]
#[ignore = "diagnostic for the pinned duel replay gap — run explicitly"]
fn diag_duel_tick0_divergence() {
    use ambition::runtime::snapshot::{restore, take};
    use ambition::world::rooms::{RoomSet, RoomTransitionRequested};

    let opts = SandboxSimOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_start_room("duel_arena");
    let mut s = SandboxSim::new_with_options(opts).unwrap();
    let reg = s
        .world_mut()
        .remove_resource::<ambition::runtime::snapshot::SnapshotRegistry>()
        .unwrap();

    let mut warm = RandomWalkPolicy::traversal_stress(3);
    for _ in 0..60 {
        s.step(warm.act());
    }
    let snap = take(s.world(), &reg);

    let door = {
        let rs = ambition::platformer::lifecycle::session_world_component::<RoomSet>(s.world())
            .expect("a RoomSet");
        let zone = rs
            .active_loading_zones()
            .iter()
            .find(|z| z.id == "duel_arena_entry")
            .unwrap()
            .clone();
        rs.transition_for_player(zone.aabb, ambition::engine_core::Vec2::ZERO, true)
            .unwrap()
    };
    let inputs: Vec<_> = {
        let mut p = RandomWalkPolicy::traversal_stress(99);
        (0..60).map(|_| p.act()).collect()
    };

    // First run: step tick 0 only, capture per-entry hashes.
    s.step(inputs[0].clone());
    let first_entries = reg.hash_by_entry(s.world());
    // Finish the suffix so the world leaves the room, as the real test does.
    for (i, a) in inputs.iter().enumerate().skip(1) {
        if i == 10 {
            s.world_mut()
                .resource_mut::<bevy::ecs::message::Messages<RoomTransitionRequested>>()
                .write(RoomTransitionRequested::new(door.clone(), None));
        }
        s.step(a.clone());
    }

    let report = restore(s.world_mut(), &snap, &reg).unwrap();
    eprintln!(
        "RESTORE REPORT: patched={} rebuilt={} respawned={} despawned={} unapplied_rows={} \
         cursors_unresolved={} stale={:?}",
        report.patched,
        report.rebuilt,
        report.respawned,
        report.despawned,
        report.unapplied_rows,
        report.resource_cursors_unresolved,
        report
            .stale_components
            .iter()
            .map(|c| c.name.clone())
            .collect::<Vec<_>>(),
    );
    s.step(inputs[0].clone());
    let second_entries = reg.hash_by_entry(s.world());

    let culprits: Vec<&str> = first_entries
        .iter()
        .zip(&second_entries)
        .filter(|((_, x), (_, y))| x != y)
        .map(|((name, _), _)| *name)
        .collect();
    panic!("tick-0 divergent entries: {culprits:?}");
}
