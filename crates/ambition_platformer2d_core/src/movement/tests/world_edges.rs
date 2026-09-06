//! Blast-zone ownership and exit-cause tests.
//!
//! The stage authors the blast margin, every motion policy uses the same gate,
//! and the gate distinguishes leaving the world from other hazards.

use super::super::*;
use crate::body_clusters::BodyClusterScratch;
#[allow(unused_imports)]
use crate::test_support::*;
use crate::{AbilitySet, Vec2, World};

/// An empty world with no floor, so a body placed past the edge stays there.
/// `fall_out_margin` is the only thing these cases vary.
fn void_world(fall_out_margin: f32) -> World {
    World::new(
        "blast zone rig",
        Vec2::new(1600.0, 900.0),
        Vec2::new(800.0, 450.0),
        Vec::new(),
    )
    .with_fall_out_margin(fall_out_margin)
}

/// One simulation step for a body parked `past` pixels below the world's
/// bottom edge, under `spec`'s motion policy. Returns the step's reset cause.
fn step_below_edge(margin: f32, past: f32, spec: MotionModelSpec) -> Option<ResetCause> {
    let world = void_world(margin);
    let start = Vec2::new(800.0, world.size.y + past);
    let mut scratch = BodyClusterScratch::new_with_abilities(start, AbilitySet::sandbox_all());
    let (model, mut clusters) = scratch.parts();
    switch_motion_model(model, spec);
    update_player_simulation_with_clusters(
        &world,
        model,
        &mut clusters,
        InputState::default(),
        1.0 / 60.0,
        TEST_TUNING,
    )
    .reset
}

/// The three policies a body can move under. A stage's edge is a property of
/// the STAGE; which policy the body happens to be running is none of its
/// business, and the answer must not depend on it.
fn every_policy() -> [(&'static str, MotionModelSpec); 3] {
    [
        (
            "axis-swept",
            MotionModelSpec::AxisSwept(AxisSweptParams::default()),
        ),
        (
            "surface-momentum",
            MotionModelSpec::SurfaceMomentum(MomentumParams::default()),
        ),
        (
            "adhesive-crawler",
            MotionModelSpec::AdhesiveCrawler(CrawlerParams::default()),
        ),
    ]
}

#[test]
fn the_stage_owns_its_own_edge() {
    // 120px past the bottom. Generous default margin: still in play. Tight
    // fighting-stage margin: gone. Same body, same position, same policy —
    // the only thing that decided its fate is the number the stage authored.
    assert_eq!(
        step_below_edge(
            World::DEFAULT_FALL_OUT_MARGIN,
            120.0,
            MotionModelSpec::AxisSwept(AxisSweptParams::default())
        ),
        None,
        "120px past the edge is inside the default 200px margin — still alive"
    );
    assert_eq!(
        step_below_edge(
            64.0,
            120.0,
            MotionModelSpec::AxisSwept(AxisSweptParams::default())
        ),
        Some(ResetCause::LeftTheWorld),
        "a stage that authors a 64px blast zone kills at 120px past the edge"
    );
}

#[test]
fn every_motion_policy_agrees_where_the_world_ends() {
    for (name, spec) in every_policy() {
        assert_eq!(
            step_below_edge(64.0, 120.0, spec),
            Some(ResetCause::LeftTheWorld),
            "{name}: 120px past a 64px blast zone left the world"
        );
        assert_eq!(
            step_below_edge(64.0, 32.0, spec),
            None,
            "{name}: 32px past a 64px blast zone is still inside it"
        );
    }
}

#[test]
fn leaving_the_world_is_not_the_same_event_as_touching_a_hazard() {
    let mut world = void_world(World::DEFAULT_FALL_OUT_MARGIN);
    world.blocks.push(crate::world::Block::hazard(
        "spikes",
        Vec2::new(700.0, 400.0),
        Vec2::new(200.0, 48.0),
    ));

    // Standing in the spikes, well inside the world.
    let mut scratch =
        BodyClusterScratch::new_with_abilities(Vec2::new(800.0, 410.0), AbilitySet::sandbox_all());
    let (model, mut clusters) = scratch.parts();
    let hazard = update_player_simulation_with_clusters(
        &world,
        model,
        &mut clusters,
        InputState::default(),
        1.0 / 60.0,
        TEST_TUNING,
    )
    .reset;
    assert_eq!(
        hazard,
        Some(ResetCause::Hazard),
        "the spikes are a hazard, and the gate must not call them the void"
    );

    // The other half of "not the same event", and the half that makes the
    // test's name true: the void must not report itself as a hazard either.
    // Without this the whole case survives collapsing the two back together.
    let void = step_below_edge(
        64.0,
        120.0,
        MotionModelSpec::AxisSwept(AxisSweptParams::default()),
    );
    assert_eq!(void, Some(ResetCause::LeftTheWorld));
    assert_ne!(
        void, hazard,
        "a pit and a spike are different deaths; a platform fighter scores on \
         one of them and not the other"
    );

    // A body that is BOTH past the blast margin and inside a hazard volume
    // left the world: the void is further out than any authored geometry, so
    // it is the later and larger fact. Pinned because the gate's branch order
    // is the only thing that decides it.
    let mut deep = void_world(64.0);
    deep.blocks.push(crate::world::Block::hazard(
        "spikes past the edge",
        Vec2::new(700.0, deep.size.y + 64.0),
        Vec2::new(200.0, 200.0),
    ));
    let mut scratch = BodyClusterScratch::new_with_abilities(
        Vec2::new(800.0, deep.size.y + 120.0),
        AbilitySet::sandbox_all(),
    );
    let (model, mut clusters) = scratch.parts();
    assert_eq!(
        update_player_simulation_with_clusters(
            &deep,
            model,
            &mut clusters,
            InputState::default(),
            1.0 / 60.0,
            TEST_TUNING,
        )
        .reset,
        Some(ResetCause::LeftTheWorld),
        "past the blast margin AND in spikes: it left the world"
    );

    // Both hazardous — that is what the old single `hazard` bool carried, and
    // the named predicate has to keep carrying it for the hit-flash.
    assert!(ResetCause::Hazard.is_hazardous());
    assert!(ResetCause::LeftTheWorld.is_hazardous());
    assert!(
        !ResetCause::Requested.is_hazardous(),
        "asking to reset is not the world hurting you"
    );
}

/// One simulation step for a body parked `past` pixels beyond the world's
/// RIGHT edge, at mid-height so the fall direction is not also implicated.
fn step_past_side(world: &World, past: f32) -> Option<ResetCause> {
    let start = Vec2::new(world.size.x + past, world.size.y * 0.5);
    let mut scratch = BodyClusterScratch::new_with_abilities(start, AbilitySet::sandbox_all());
    let (model, mut clusters) = scratch.parts();
    update_player_simulation_with_clusters(
        world,
        model,
        &mut clusters,
        InputState::default(),
        1.0 / 60.0,
        TEST_TUNING,
    )
    .reset
}

/// The sides are opt-in, and that is the whole point.
///
/// A platformer walking off the left edge of a room is transitioning to the
/// next room; killing there would break every corridor in the game. A platform
/// fighter thrown off the left edge has lost a stock — and that is where a
/// platform fighter loses MOST of them, so a fall-direction-only blast zone is
/// not really a blast zone at all.
///
/// Both readings have to be available from the same engine, which is why the
/// side margin is an `Option` and the fall margin is not.
#[test]
fn the_sides_kill_only_when_a_stage_says_they_do() {
    let corridor = void_world(World::DEFAULT_FALL_OUT_MARGIN);
    assert_eq!(
        step_past_side(&corridor, 400.0),
        None,
        "a room with no side blast zone lets a body leave sideways — that is a \
         corridor, and the room next door is where it is going"
    );

    let stage = void_world(World::DEFAULT_FALL_OUT_MARGIN).with_side_out_margin(64.0);
    assert_eq!(
        step_past_side(&stage, 120.0),
        Some(ResetCause::LeftTheWorld),
        "a stage that authors a 64px side blast zone loses the fighter at 120px"
    );
    assert_eq!(
        step_past_side(&stage, 32.0),
        None,
        "32px past a 64px side zone is still inside it"
    );
}

/// A body launched straight up leaves through the top under down-gravity and
/// through the BOTTOM of the screen under up-gravity. Both are "the direction
/// you do not fall toward", which is the only frame-agnostic way to say it.
#[test]
fn the_ceiling_kills_only_when_a_stage_says_so_and_it_follows_gravity() {
    let open = void_world(World::DEFAULT_FALL_OUT_MARGIN);
    let ceiling = void_world(World::DEFAULT_FALL_OUT_MARGIN).with_rise_out_margin(64.0);

    // 120px ABOVE the world (negative y is up under default gravity).
    let above = Vec2::new(800.0, -120.0);
    let probe = |world: &World| {
        let mut scratch = BodyClusterScratch::new_with_abilities(above, AbilitySet::sandbox_all());
        let (model, mut clusters) = scratch.parts();
        update_player_simulation_with_clusters(
            world,
            model,
            &mut clusters,
            InputState::default(),
            1.0 / 60.0,
            TEST_TUNING,
        )
        .reset
    };
    assert_eq!(
        probe(&open),
        None,
        "with no ceiling zone a body may rise forever, which is what a \
         platformer with tall rooms needs"
    );
    assert_eq!(
        probe(&ceiling),
        Some(ResetCause::LeftTheWorld),
        "a stage that authors a ceiling zone catches the body that rose past it"
    );

    // C4: flip gravity to point UP, and the ceiling flips with it. The same
    // body, now 120px BELOW the world, is the one that rose too far — because
    // "ceiling" means "the way you do not fall", not "smaller y". Without this
    // arm the test above is satisfied by a hard-coded `-y` and the name lies.
    let mut inverted = TEST_TUNING;
    inverted.gravity_dir = Vec2::new(0.0, -1.0);
    inverted.gravity_sign = -1.0;
    let below = Vec2::new(800.0, ceiling.size.y + 120.0);
    let mut scratch = BodyClusterScratch::new_with_abilities(below, AbilitySet::sandbox_all());
    let (model, mut clusters) = scratch.parts();
    assert_eq!(
        update_player_simulation_with_clusters(
            &ceiling,
            model,
            &mut clusters,
            InputState::default(),
            1.0 / 60.0,
            inverted,
        )
        .reset,
        Some(ResetCause::LeftTheWorld),
        "under up-gravity the ceiling is BELOW the world, and the gate must \
         measure it in the body's frame rather than in screen coordinates"
    );
}
