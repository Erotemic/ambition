//! GEOMETRY-OBSERVABILITY GOBS-0: what F1's world-blocks layer draws, against
//! what a body collides with.
//!
//! F1 draws the room's base blocks (`draw_world_blocks` over
//! `RoomGeometry`). A body's sweep reads the composed walls
//! (`CollisionWorld`: the base, the moving platforms, and the feature overlay
//! of gates, breakables and other content solids), and for a given body the
//! gates open for it and the portal holes it opened. The probe prints, for a
//! room with a breakable platform, before and after the platform breaks:
//! - each solid the composed walls hold that the base does not (no outline in
//!   the blocks layer);
//! - each base block the composed walls do not hold (an outline of nothing).
//!
//! The owner is `docs/planning/engine/geometry-observability-and-f1.md`.

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::breakables::BreakableFeature;
use ambition_platformer2d::combat::components::FeatureName;
use ambition_platformer2d::platformer::lifecycle::InRoomInstance;
use ambition_platformer2d::world::collision::CollisionWorld;
use bevy::ecs::system::RunSystemOnce as _;
use bevy::prelude::{Query, With};

use crate::common::fixed_60hz_room_options;

const ROOM: &str = "basement_breakables";
const PLATFORM: &str = "respawning breakable platform";

/// A solid as the probe compares it: name, kind, and box rounded to a pixel.
type Wall = (String, String, [i32; 4]);

fn wall(block: &ambition_platformer2d::engine_core::Block) -> Wall {
    let (min, max) = (block.aabb.min, block.aabb.max);
    (
        block.name.clone(),
        format!("{:?}", block.kind),
        [min.x.round() as i32, min.y.round() as i32, max.x.round() as i32, max.y.round() as i32],
    )
}

/// (what F1's blocks layer draws, what a body collides with) in Player 1's
/// room, each sorted.
fn walls(sim: &mut Platformer2dSimHarness) -> (Vec<Wall>, Vec<Wall>) {
    sim.world_mut()
        .run_system_once(
            |collision: CollisionWorld,
             player: Query<&InRoomInstance, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>| {
                let room = collision.room(player.single().ok()).expect("Player 1's room has collision");
                let mut drawn: Vec<Wall> = room.base().blocks.iter().map(wall).collect();
                let mut met: Vec<Wall> = room.solids().expect("composed walls").blocks.iter().map(wall).collect();
                drawn.sort();
                met.sort();
                (drawn, met)
            },
        )
        .expect("the probe system runs")
}

fn report(label: &str, drawn: &[Wall], met: &[Wall]) -> (usize, usize) {
    let unoutlined: Vec<&Wall> = met.iter().filter(|wall| !drawn.contains(wall)).collect();
    let phantom: Vec<&Wall> = drawn.iter().filter(|wall| !met.contains(wall)).collect();
    println!("[f1] {label}: {} drawn, {} met", drawn.len(), met.len());
    for wall in &unoutlined {
        println!("[f1]   met, not outlined: {wall:?}");
    }
    for wall in &phantom {
        println!("[f1]   outlined, not met: {wall:?}");
    }
    (unoutlined.len(), phantom.len())
}

/// Break the platform as its own break road does (zero health), and let the
/// walls be composed again.
fn break_the_platform(sim: &mut Platformer2dSimHarness) {
    {
        let world = sim.world_mut();
        let mut q = world.query::<(&FeatureName, &mut BreakableFeature)>();
        let (_, mut feature) = q
            .iter_mut(world)
            .find(|(name, _)| name.0.as_str() == PLATFORM)
            .expect("the room authors the breakable platform");
        let mut state = feature.state().clone();
        let max = state.health.max;
        assert!(state.apply_damage(max), "the platform breaks");
        *feature = BreakableFeature::new(state);
    }
    for _ in 0..3 {
        sim.app_mut().update();
    }
}

/// Found by the census below: a platform a body stands on to crumble also
/// lends the world a pogo orb (`PogoTargetContributor`'s fallback). Broken,
/// its solid went and the orb stayed, so a body could pogo off the air where
/// it had been. Broken, nothing stands where it was.
#[test]
fn a_broken_platform_leaves_no_pogo_orb_behind() {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(ROOM)).expect("the room boots");
    for _ in 0..5 {
        sim.app_mut().update();
    }
    let (_, whole) = walls(&mut sim);
    let boxes: Vec<[i32; 4]> = whole.iter().filter(|wall| wall.0.ends_with(PLATFORM)).map(|wall| wall.2).collect();
    assert_eq!(boxes.len(), 1, "premise: the platform is one solid while whole: {whole:?}");
    assert!(
        whole.iter().any(|wall| wall.2 == boxes[0] && wall.1 == "PogoOrb"),
        "premise: the whole platform lends a pogo orb: {whole:?}"
    );
    break_the_platform(&mut sim);
    let (_, broken) = walls(&mut sim);
    let left: Vec<&Wall> = broken.iter().filter(|wall| wall.2 == boxes[0]).collect();
    assert!(left.is_empty(), "the broken platform left walls behind: {left:?}");
}

#[test]
#[ignore = "probe: prints where F1's blocks layer and the collision walls differ; run with --ignored --nocapture"]
fn probe_f1_blocks_against_the_walls_a_body_meets() {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(ROOM)).expect("the room boots");
    for _ in 0..5 {
        sim.app_mut().update();
    }
    let (drawn, met) = walls(&mut sim);
    let whole = report("the platform whole", &drawn, &met);

    break_the_platform(&mut sim);
    let (drawn_after, met_after) = walls(&mut sim);
    let broken = report("the platform broken", &drawn_after, &met_after);
    assert_ne!(met, met_after, "premise: breaking the platform changed the walls a body meets");
    println!("[f1] differences (met not outlined, outlined not met): whole {whole:?}, broken {broken:?}");
}
