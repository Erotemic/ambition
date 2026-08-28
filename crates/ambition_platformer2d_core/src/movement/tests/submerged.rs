//! `BodyMode::Submerged`: travel under the stage, and the surface it is under.

use super::super::*;
use super::step_scratch;
use crate::body_clusters::BodyClusterScratch;
#[allow(unused_imports)]
use crate::test_support::*;
use crate::{AbilitySet, LocalAxes, Vec2, World};

/// One platform with a RIGHT LEDGE at x = 600 and nothing past it.
fn platform_with_a_right_ledge() -> World {
    let h = 900.0;
    World {
        name: "submerged test world".to_string(),
        size: Vec2::new(1600.0, h),
        spawn: Vec2::new(560.0, h - 48.0 - 24.0),
        blocks: vec![crate::world::Block::solid(
            "platform",
            Vec2::new(0.0, h - 48.0),
            Vec2::new(600.0, 48.0),
        )],
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
        chains: Vec::new(),
        edges: Default::default(),
    }
}

fn submerged_at(spawn: Vec2) -> BodyClusterScratch {
    let mut scratch = BodyClusterScratch::new_with_abilities(spawn, AbilitySet::sandbox_all());
    scratch.body_mode.body_mode = crate::player_state::BodyMode::Submerged;
    scratch
}

fn stick(x: f32) -> InputState {
    InputState {
        axes: LocalAxes::new(x, 0.0),
        ..Default::default()
    }
}

/// ⛔⛔ THE TRAPDOOR STAYS ON THE BOARDS IT OPENED. Jon, 2026-08-28: the door
/// *"can only move along a ground surface (i.e. it can't go over a ledge)."*
///
/// A submerged body is passable against every block in the world, so nothing
/// else in this pipeline can stop it: before this rule she travelled under open
/// air, off the end of the stage, and surfaced into nothing.
///
/// ⛔ THE ARMS STRADDLE THE LEDGE, and the second one is what makes the first
/// mean something. A rule that refused every submerged step would pass an
/// "off the edge" arm and delete the move; travelling INWARD along the same
/// platform must still work.
#[test]
fn a_submerged_body_travels_along_its_surface_and_stops_at_the_ledge() {
    let world = platform_with_a_right_ledge();
    let feet_on_the_platform = world.spawn;

    // Toward the ledge: she reaches it and stops short of leaving it.
    let mut outward = submerged_at(Vec2::new(560.0, feet_on_the_platform.y));
    for _ in 0..240 {
        let _ = step_scratch(&world, &mut outward, stick(1.0));
    }
    let half_width = outward.kinematics.size.x * 0.5;
    assert!(
        outward.kinematics.pos.x + half_width <= 600.0 + 4.0,
        "she travelled to x = {}, which is past the platform's right edge at 600 — \
         a body under the stage went out over open air",
        outward.kinematics.pos.x,
    );
    assert!(
        outward.kinematics.pos.x > 560.0,
        "she never moved at all (x = {}); the rule refused travel rather than \
         refusing to LEAVE, which deletes the move",
        outward.kinematics.pos.x,
    );

    // Back along the same platform: unobstructed.
    let mut inward = submerged_at(Vec2::new(560.0, feet_on_the_platform.y));
    for _ in 0..60 {
        let _ = step_scratch(&world, &mut inward, stick(-1.0));
    }
    assert!(
        inward.kinematics.pos.x < 500.0,
        "travelling INWARD along the platform stopped at x = {}, and there is \
         nothing there to stop it",
        inward.kinematics.pos.x,
    );
}
