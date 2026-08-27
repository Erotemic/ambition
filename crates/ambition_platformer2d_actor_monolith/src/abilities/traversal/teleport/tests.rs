//! ⛔⛔ THE ARMS STRADDLE THE RADIUS AND THE DIRECTION. A ledge assist tested
//! only against a ledge it should catch agrees with an implementation that snaps
//! to the nearest surface in the world, which would drag a fighter off the stage
//! and onto the wrong platform. Every "it helps" arm below is paired with a
//! "and it refuses" one.

use super::*;

/// A solid named by its centre and half-extents. `Block::solid` takes min +
/// size, so this is the conversion in one place rather than at seven call sites.
fn solid(name: &str, center: ae::Vec2, half: ae::Vec2) -> ae::Block {
    ae::Block::solid(name, center - half, half * 2.0)
}

fn world_with(blocks: Vec<ae::Block>) -> ae::World {
    ae::World::new(
        "teleport_test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::ZERO,
        blocks,
    )
}

/// The fighter's half-extents throughout: a 32x64 body.
const HALF: ae::Vec2 = ae::Vec2::new(16.0, 32.0);

/// A platform whose top face is at y = 0, spanning x in [-100, 100].
fn stage() -> ae::World {
    world_with(vec![solid("stage", ae::Vec2::new(0.0, 50.0), ae::Vec2::new(100.0, 50.0))])
}

#[test]
fn an_arrival_just_under_a_ledge_is_placed_standing_on_it() {
    let world = stage();
    // The arrival's CENTRE 20px below the platform's top face — the miss a few
    // degrees of stick angle makes.
    let assisted = ledge_assisted_arrival(&world, ae::Vec2::new(0.0, 20.0), HALF, 40.0);
    assert!(
        assisted.y < 20.0,
        "the arrival must be lifted onto the ledge, and it stayed at {}",
        assisted.y
    );
    assert!(
        (assisted.y - (0.0 - HALF.y)).abs() < 1e-3,
        "…standing exactly on the top face, and it is at {}",
        assisted.y
    );
}

/// ⛔ THE PAIRED ARM: the same miss, outside the authored radius, is left alone.
#[test]
fn an_arrival_far_below_a_ledge_is_left_where_it_landed() {
    let world = stage();
    let assisted = ledge_assisted_arrival(&world, ae::Vec2::new(0.0, 200.0), HALF, 40.0);
    assert_eq!(
        assisted,
        ae::Vec2::new(0.0, 200.0),
        "a ledge 200px away is not a ledge this teleport was aimed at"
    );
}

/// ⛔ A DISABLED ASSIST IS DISABLED. `0.0` is what a teleport that is not a
/// recovery authors, and it must not quietly acquire the behaviour anyway.
#[test]
fn a_zero_radius_never_moves_an_arrival() {
    let world = stage();
    let at = ae::Vec2::new(0.0, 20.0);
    assert_eq!(ledge_assisted_arrival(&world, at, HALF, 0.0), at);
}

/// ⛔⛔ IT ONLY EVER LIFTS. `+y` is gravity-down, so a ledge BELOW the arrival
/// has a LARGER y and is one the fighter CLEARED; dragging them down onto it
/// would end a recovery that had already worked. A naive "nearest surface" rule
/// does exactly that, and so did the first version of this function.
#[test]
fn a_ledge_the_fighter_already_cleared_never_drags_them_back_down() {
    // Two platforms: the one at y = 0 the fighter cleared, and nothing above.
    let world = stage();
    // Well ABOVE the platform's top face (smaller y) and unsupported — a
    // successful recovery, mid-air, with the surface within the radius.
    let cleared = ae::Vec2::new(0.0, -60.0);
    assert_eq!(
        ledge_assisted_arrival(&world, cleared, HALF, 200.0),
        cleared,
        "a surface the fighter is already above is not a ledge to be pulled onto"
    );
}

/// ⛔ AN ARRIVAL THAT ALREADY HAD SUPPORT IS NOT MOVED, or the assist would
/// choose a different platform than the player did.
#[test]
fn an_arrival_already_standing_on_something_is_left_alone() {
    let world = stage();
    let standing = ae::Vec2::new(0.0, 0.0 - HALF.y);
    assert_eq!(ledge_assisted_arrival(&world, standing, HALF, 80.0), standing);
}

/// ⛔ AND IT NEVER PLACES A BODY INSIDE GEOMETRY. A ledge with a wall sitting on
/// it has no room to stand, and the miss is better than the embed.
#[test]
fn a_ledge_with_no_room_to_stand_is_refused() {
    let world = world_with(vec![
        solid("stage", ae::Vec2::new(0.0, 50.0), ae::Vec2::new(100.0, 50.0)),
        // A wall occupying exactly where a body standing on that ledge would be.
        solid("wall", ae::Vec2::new(0.0, -HALF.y), ae::Vec2::new(20.0, HALF.y)),
    ]);
    let at = ae::Vec2::new(0.0, 20.0);
    assert_eq!(
        ledge_assisted_arrival(&world, at, HALF, 60.0),
        at,
        "a placement that would embed the body is worse than the miss"
    );
}

/// ⛔ THE NEAREST QUALIFYING LEDGE WINS, so a teleport near two platforms lands
/// on the one it was aimed at rather than on whichever the scan reached first.
#[test]
fn the_nearest_qualifying_ledge_wins() {
    let world = world_with(vec![
        // ⛔ `Aabb::top()` IS THE MIN Y, because `+y` is gravity-down. A centre
        // of `c` with half-height `h` has its top face at `c - h`, and the first
        // version of this test placed both platforms 100px lower than it meant
        // to — then asserted the number it had meant, and failed against correct
        // code.
        //
        // Far above: top face at y = -200.
        solid("far", ae::Vec2::new(0.0, -150.0), ae::Vec2::new(100.0, 50.0)),
        // Near above: top face at y = -20.
        solid("near", ae::Vec2::new(0.0, 30.0), ae::Vec2::new(100.0, 50.0)),
    ]);
    let assisted = ledge_assisted_arrival(&world, ae::Vec2::new(0.0, 20.0), HALF, 400.0);
    assert!(
        (assisted.y - (-20.0 - HALF.y)).abs() < 1e-3,
        "the NEAR ledge decides, and the arrival went to {}",
        assisted.y
    );
}
