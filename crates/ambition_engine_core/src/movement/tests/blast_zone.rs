//! Where the world ENDS, and who gets to say so.
//!
//! Three facts, none of which held before:
//!
//! 1. The edge is the STAGE's. It was a `200.0` literal, so no room could
//!    disagree with it — which made a platform fighter's blast zone, a number
//!    that differs per stage and is the entire loss condition of the genre,
//!    impossible to author.
//! 2. Every motion policy agrees where that edge is. The gate's own doc called
//!    itself "the ONE gate every policy publishes through" while the axis-swept
//!    policy kept a private transcription of it. Two copies of a constant agree
//!    right up until someone edits one.
//! 3. The gate says WHY. Falling out of the world and touching a spike used to
//!    arrive as the same anonymous `hazard` bool, so nothing downstream could
//!    tell a pit from a hazard — and the death publisher said so in a comment
//!    instead of in code.

use super::super::*;
use crate::body_clusters::BodyClusterScratch;
#[allow(unused_imports)]
use crate::test_support::*;
use crate::{AbilitySet, Vec2, World};

/// An empty world with no floor, so a body placed past the edge stays there.
/// `blast_margin` is the only thing these cases vary.
fn void_world(blast_margin: f32) -> World {
    World::new(
        "blast zone rig",
        Vec2::new(1600.0, 900.0),
        Vec2::new(800.0, 450.0),
        Vec::new(),
    )
    .with_blast_margin(blast_margin)
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
            World::DEFAULT_BLAST_MARGIN,
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
    let mut world = void_world(World::DEFAULT_BLAST_MARGIN);
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
