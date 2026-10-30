//! A body with a balance (`crate::Balance`) stands by its centre of gravity:
//! it is pushed off a ledge its centre is past, it teeters within the band,
//! and a body without one still stands on a lip.

use crate::body_clusters::BodyClusterScratch;
use crate::movement::InputState;
use crate::test_support::update_player_with_tuning_scratch;
use crate::world::{Block, World};
use crate::{AbilitySet, Vec2};

/// The giant gnu's collision box.
const GIANT: Vec2 = Vec2::new(630.0, 464.0);
/// The top of every floor in these worlds.
const TOP: f32 = 1000.0;

fn world_of(blocks: Vec<Block>) -> World {
    let mut blocks = blocks;
    // Far below, so a body that goes off the ledge lands somewhere.
    blocks.push(Block::solid("pit floor", Vec2::new(-2000.0, 2900.0), Vec2::new(8000.0, 100.0)));
    World {
        name: "balance".to_string(),
        size: Vec2::new(4000.0, 3000.0),
        spawn: Vec2::new(500.0, 500.0),
        blocks: blocks.into(),
        climbable_regions: Vec::new(),
        chains: (Vec::new()).into(),
        edges: Default::default(),
        water_regions: Vec::new(),
    }
}

/// A ledge from x 0 to 1000, its top at [`TOP`].
fn ledge() -> World {
    world_of(vec![Block::solid("ledge", Vec2::new(0.0, TOP), Vec2::new(1000.0, 200.0))])
}

/// A body of `size` with its centre at `x`, dropped onto the floor at
/// [`TOP`] and stepped `ticks` times with no input. It has no abilities, so
/// it does not catch a ledge it goes off.
fn settled(world: &World, size: Vec2, x: f32, ticks: usize, tuning: crate::test_support::TestTuning) -> BodyClusterScratch {
    let mut scratch = BodyClusterScratch::new_with_abilities(Vec2::new(x, TOP - size.y * 0.5 - 2.0), AbilitySet::NONE);
    scratch.kinematics.size = size;
    scratch.ground.on_ground = false;
    for _ in 0..ticks {
        update_player_with_tuning_scratch(world, &mut scratch, InputState::default(), 1.0 / 60.0, tuning);
    }
    scratch
}

fn balanced() -> crate::test_support::TestTuning {
    let mut tuning = crate::test_support::TEST_TUNING;
    tuning.balance = Some(crate::Balance::LONG_BODY);
    tuning
}

/// The giant rests with 10 px of its 630 px box on the ledge, its centre 305
/// past the edge: the ledge does not hold it. It is pushed off and falls.
#[test]
fn a_giant_on_a_sliver_of_a_ledge_is_pushed_off_and_falls() {
    let world = ledge();
    let x = 1000.0 + GIANT.x * 0.5 - 10.0;
    let giant = settled(&world, GIANT, x, 90, balanced());
    assert!(
        giant.kinematics.pos.y - GIANT.y * 0.5 > TOP + 100.0,
        "the giant stayed up on a 10 px sliver of the ledge: centre {:?}, on_ground {}",
        giant.kinematics.pos,
        giant.ground.on_ground
    );
    assert!(
        giant.kinematics.pos.x - GIANT.x * 0.5 >= 1000.0,
        "it fell through the ledge rather than off it: centre {:?}",
        giant.kinematics.pos
    );
}

/// The giant's centre is 30 px past the edge, inside its teeter band (a
/// sixth of its 315 half-width): it stands where it is, and teeters.
#[test]
fn a_giant_with_its_centre_just_past_the_edge_stands_and_teeters() {
    let world = ledge();
    let x = 1030.0;
    let giant = settled(&world, GIANT, x, 40, balanced());
    assert!(giant.ground.on_ground, "the giant in its teeter band did not stand");
    assert!((giant.kinematics.pos.x - x).abs() < 1e-3, "the giant in its teeter band was moved: {:?}", giant.kinematics.pos);
    assert!(giant.axis().teetering, "the giant with its centre past the edge did not teeter");

    // Control: with its centre over the ledge it stands and does not teeter.
    let inside = settled(&world, GIANT, 900.0, 40, balanced());
    assert!(inside.ground.on_ground && !inside.axis().teetering, "a giant over the ledge teetered or fell");
}

/// The same sliver, with no balance (a player's tuning): the body stands on
/// the lip, as every body without a balance does.
#[test]
fn a_body_without_a_balance_still_stands_on_a_lip() {
    let world = ledge();
    let x = 1000.0 + GIANT.x * 0.5 - 10.0;
    let body = settled(&world, GIANT, x, 60, crate::test_support::TEST_TUNING);
    assert!(body.ground.on_ground, "a body without a balance fell off the lip");
    assert!((body.kinematics.pos.x - x).abs() < 1e-3, "a body without a balance was pushed: {:?}", body.kinematics.pos);
}

/// The support is the stretch from the first surface under the feet to the
/// last: a centre over a seam between two tiles, or over a gap the body
/// bridges with ground on both sides, is over its support.
#[test]
fn a_centre_over_a_seam_or_a_bridged_gap_stands() {
    let seam = world_of(vec![
        Block::solid("tile a", Vec2::new(0.0, TOP), Vec2::new(500.0, 200.0)),
        Block::solid("tile b", Vec2::new(500.0, TOP), Vec2::new(500.0, 200.0)),
    ]);
    let on_seam = settled(&seam, GIANT, 500.0, 60, balanced());
    assert!(on_seam.ground.on_ground && (on_seam.kinematics.pos.x - 500.0).abs() < 1e-3, "a giant over a seam was moved");

    let gap = world_of(vec![
        Block::solid("bank a", Vec2::new(0.0, TOP), Vec2::new(400.0, 200.0)),
        Block::solid("bank b", Vec2::new(600.0, TOP), Vec2::new(400.0, 200.0)),
    ]);
    let bridging = settled(&gap, GIANT, 500.0, 60, balanced());
    assert!(
        bridging.ground.on_ground && (bridging.kinematics.pos.x - 500.0).abs() < 1e-3,
        "a giant bridging a 200 px gap was moved: {:?}",
        bridging.kinematics.pos
    );
}

/// A dog (93 px wide) stands with its centre over a hole in the floor 96 px
/// wide (a portal's opening of 92 with its squeeze), its box partly on the
/// floor past the hole's end. Its centre is not over its support: it is
/// pushed into the hole and falls through it. Without a balance it would
/// stand on the end of the floor.
#[test]
fn a_dog_with_its_centre_over_a_hole_goes_into_it() {
    const DOG: Vec2 = Vec2::new(93.0, 60.0);
    let holed = world_of(vec![
        Block::solid("floor left", Vec2::new(0.0, TOP), Vec2::new(452.0, 200.0)),
        Block::solid("floor right", Vec2::new(548.0, TOP), Vec2::new(452.0, 200.0)),
    ]);
    let dog = settled(&holed, DOG, 530.0, 90, balanced());
    assert!(
        dog.kinematics.pos.y > TOP + 300.0,
        "the dog with its centre over the hole did not fall through it: centre {:?}",
        dog.kinematics.pos
    );
    let lip = settled(&holed, DOG, 530.0, 90, crate::test_support::TEST_TUNING);
    assert!(lip.ground.on_ground, "control: a body without a balance did not stand on the end of the floor");
}
