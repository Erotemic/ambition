// Falling-sand room regression: needs the sand slice (`falling_sand`) and the
// RL stepping API. Compiled out (empty module) when either is disabled.
#![cfg(all(feature = "falling_sand", feature = "rl_sim"))]
//! Authored-room regression for the FS2/FS3 sand slice: the REAL
//! `falling_sand_room`, entered by semantic room id, with the REAL authored
//! sand switch activated semantically (by its authored id — no navigation
//! coordinates, no simulated walking to a lever).
//!
//! What it pins, in order down the pipeline:
//!
//! 1. opening the spout EMITS matter into the sand grid,
//! 2. the conservation law (`loose + settled == emitted`) holds in the real
//!    room, not just the unit-test box,
//! 3. finite settling: within a bounded tick budget, grains TRANSFER into the
//!    settled ledger (FS3's atomic ownership move),
//! 4. the settled ledger contributes real collision blocks to the world
//!    overlay, and
//! 5. that contribution PERSISTS across further ticks — the overlay is rebuilt
//!    every frame, and the ledger must survive every rebuild (the transient
//!    flicker defect this slice exists to kill).
//!
//! This is the initial regression oracle for the room, not the FS2 fixed-point
//! proof — that lives as a property test on the grid itself in
//! `ambition_content::falling_sand_sim::sand_grid`.

use ambition_content::falling_sand_sim::{FallingSandWorld, ROOM_ID, SAND_SWITCH};

use crate::common::{base, fixed_60hz_room_sim};

/// One authored-switch activation, delivered the way the interaction system
/// delivers it: the switch's own parsed `SwitchActivation`, cloned off the
/// authored entity, written as a `SwitchActivated` message.
fn activate_authored_switch(sim: &mut ambition_app::SandboxSim, switch_id: &str) {
    let world = sim.world_mut();
    let mut switches = world.query::<&ambition::actors::features::SwitchFeature>();
    let activation = switches
        .iter(world)
        .map(|feature| feature.activation.clone())
        .find(|activation| activation.id == switch_id)
        .unwrap_or_else(|| panic!("authored switch `{switch_id}` exists in {ROOM_ID}"));
    world.write_message(ambition::actors::features::SwitchActivated {
        activation,
        pos: ambition::engine_core::Vec2::ZERO,
    });
}

#[test]
fn the_sand_switch_pours_settles_and_becomes_persistent_ground() {
    let mut sim = fixed_60hz_room_sim(ROOM_ID);

    // Let the room finish loading (LDtk spawn, feature entities) before
    // looking for the authored switch.
    for _ in 0..10 {
        sim.step(base());
    }

    activate_authored_switch(&mut sim, SAND_SWITCH);

    // A few ticks for the activation to reach the spout state and the first
    // grains to enter the grid.
    for _ in 0..5 {
        sim.step(base());
    }
    {
        let sand = sim.world_mut().resource::<FallingSandWorld>();
        let grid = sand.grid.as_ref().expect("the room built its sand grid");
        assert!(
            grid.emitted() > 0,
            "the authored switch opened the spout: something poured"
        );
        assert!(
            grid.conserved_with(&sand.ledger),
            "conservation in the real room: loose={} settled={} emitted={}",
            grid.loose(),
            sand.ledger.total(),
            grid.emitted()
        );
    }

    // Bounded settling budget: the mouth sits at y≈90 and the room is 640
    // world-pixels tall, so at 3 cells/tick a grain reaches ANY floor in
    // under 200 ticks; the transfer into the ledger is same-tick once a pile
    // rests. 400 is deliberate slack, not tuning.
    let mut settled_at = None;
    for tick in 0..400 {
        sim.step(base());
        let sand = sim.world_mut().resource::<FallingSandWorld>();
        let grid = sand.grid.as_ref().expect("grid persists while in the room");
        assert!(
            grid.conserved_with(&sand.ledger),
            "conservation held every observed tick (tick {tick})"
        );
        if !sand.ledger.is_empty() {
            settled_at = Some(tick);
            break;
        }
    }
    assert!(
        settled_at.is_some(),
        "within the budget, poured sand SETTLED: grains transferred into the ledger"
    );

    // Let a pile accumulate past the per-tile block threshold, then demand
    // the persistent collision contribution.
    let mut ground_seen_at = None;
    for tick in 0..400 {
        sim.step(base());
        let overlay = sim
            .world_mut()
            .resource::<ambition::platformer::feature_overlay::FeatureEcsWorldOverlay>();
        if overlay
            .gate_solids
            .iter()
            .any(|block| block.name.starts_with("falling_sand:settled:"))
        {
            ground_seen_at = Some(tick);
            break;
        }
    }
    assert!(
        ground_seen_at.is_some(),
        "the settled ledger became standable ground in the collision overlay"
    );

    // Persistence: the overlay is cleared and rebuilt EVERY tick; the ground
    // must be re-contributed from the ledger on each of the next ticks, and
    // the settled mass must never shrink (nothing drains in this slice).
    let settled_before = sim
        .world_mut()
        .resource::<FallingSandWorld>()
        .ledger
        .total();
    for tick in 0..30 {
        sim.step(base());
        let overlay = sim
            .world_mut()
            .resource::<ambition::platformer::feature_overlay::FeatureEcsWorldOverlay>();
        assert!(
            overlay
                .gate_solids
                .iter()
                .any(|block| block.name.starts_with("falling_sand:settled:")),
            "settled ground survives the per-tick overlay rebuild (tick {tick})"
        );
    }
    let sand = sim.world_mut().resource::<FallingSandWorld>();
    assert!(
        sand.ledger.total() >= settled_before,
        "settled matter never shrinks: {} -> {}",
        settled_before,
        sand.ledger.total()
    );
    let grid = sand.grid.as_ref().expect("still in the room");
    assert!(grid.conserved_with(&sand.ledger));
}
