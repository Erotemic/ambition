use super::*;
use crate::{AabbExt, Vec2};

/// A walk budget wide enough not to bind in these units tests: what
/// `walk_budget` DOES is the subject of its own test at the bottom, and every
/// other claim here is about the constraint itself.
const WALK: f32 = 1_000.0;

fn body(center_x: f32, center_y: f32) -> Aabb {
    Aabb::new(Vec2::new(center_x, center_y), Vec2::new(10.0, 20.0))
}

/// A blocker that is STANDING THERE. Most of this file is about one mover
/// against bodies that are not going anywhere, and for those the gap is
/// undivided — the arithmetic is the same one this module had before it learned
/// to split a gap between two movers.
fn still(center_x: f32, center_y: f32) -> BodyContactBlocker {
    BodyContactBlocker::new(body(center_x, center_y), Vec2::ZERO)
}

/// A blocker CLOSING on the mover at `speed` along x, positive meaning it
/// travels toward +x.
fn closing(center_x: f32, center_y: f32, speed: f32) -> BodyContactBlocker {
    BodyContactBlocker::new(body(center_x, center_y), Vec2::new(speed, 0.0))
}

/// Build a contact field whose entry velocity matches `delta_along` (`dt == 1`).
/// Production may start, stop, or reverse after the snapshot; this fixture only
/// forbids the inconsistent case of a stationary body proposing nonzero motion.
fn field(
    blockers: &[BodyContactBlocker],
    resistance: f32,
    delta_along: f32,
) -> BodyContactField<'_> {
    BodyContactField::moving(blockers, resistance, Vec2::new(delta_along, 0.0))
}

/// The same, for a mover travelling along the vertical axis.
fn field_y(
    blockers: &[BodyContactBlocker],
    resistance: f32,
    delta_along: f32,
) -> BodyContactField<'_> {
    BodyContactField::moving(blockers, resistance, Vec2::new(0.0, delta_along))
}

/// THE IDENTITY, and it is the one every body in the engine gets.
///
///  body contact is a capability a composition grants.
#[test]
fn a_body_nobody_resists_keeps_every_bit_of_its_motion() {
    let mover = body(0.0, 0.0);
    assert_eq!(
        constrain_motion(mover, 7.5, true, WALK, 1.0, BodyContactField::NONE),
        7.5
    );
    let neighbour = [still(15.0, 0.0)];
    assert_eq!(
        constrain_motion(mover, 7.5, true, WALK, 1.0, field(&neighbour, 0.0, 7.5)),
        7.5,
        "zero resistance is opting OUT, not opting in weakly",
    );
}

/// A SOLID STOPS THE BODY AT CONTACT — and exactly at contact.
///
///  the number is DERIVED from the two boxes, not copied from a run: the mover
/// spans ±10 about x=0 and the blocker ±10 about x=40, so 20 units of free space
/// lie between them. Asking for 30 must yield 20.
#[test]
fn full_resistance_spends_the_gap_and_stops() {
    let blockers = [still(40.0, 0.0)];
    assert_eq!(
        constrain_motion(
            body(0.0, 0.0),
            30.0,
            true,
            WALK,
            1.0,
            field(&blockers, 1.0, 30.0)
        ),
        20.0
    );
    assert_eq!(
        constrain_motion(
            body(0.0, 0.0),
            12.0,
            true,
            WALK,
            1.0,
            field(&blockers, 1.0, 12.0)
        ),
        12.0,
        "a step that fits in the gap is untouched",
    );
}

/// PARTIAL RESISTANCE IS THE SMASH-LIKE FEEL: you push through, slowly.
///
/// Where the genres differ, ship the knob. Ultimate's fighters displace each
/// other rather than colliding; a beat-em-up may want the wall. Neither is more
/// correct, so the capability carries the number instead of choosing.
#[test]
fn partial_resistance_keeps_a_share_of_the_motion_that_goes_deeper() {
    let blockers = [still(40.0, 0.0)];
    // 20 free, 10 deeper, a quarter of which survives.
    assert_eq!(
        constrain_motion(
            body(0.0, 0.0),
            30.0,
            true,
            WALK,
            1.0,
            field(&blockers, 0.75, 30.0)
        ),
        22.5
    );
}

///  A PRE-EXISTING OVERLAP IS NEVER TELEPORTED APART — AND THE WAY OUT IS
/// NEVER RESISTED.
///
/// It can: an infinitesimal step either increases the axis overlap or it does not, and that is
/// arithmetic rather than a guess. With the symmetric version, four fighters spawning on one point
/// could not walk apart — every step out of the pile was cut to a fraction and a free-for-all never
/// resolved.
///
///  declining to resist a body that is leaving is not a pushout, because
/// nothing moves it. It is only this pass keeping its hands off a body that is
/// already resolving the situation itself.
#[test]
fn an_overlap_resists_going_deeper_and_never_resists_coming_out() {
    let blockers = [still(8.0, 0.0)];
    let mover = body(0.0, 0.0);
    assert!(
        mover.strict_intersects(blockers[0].aabb),
        "the fixture must actually start overlapping or this proves nothing",
    );
    assert_eq!(
        constrain_motion(mover, 6.0, true, WALK, 1.0, field(&blockers, 0.5, 6.0)),
        3.0,
        "deeper is halved",
    );
    assert_eq!(
        constrain_motion(mover, -6.0, true, WALK, 1.0, field(&blockers, 0.5, -6.0)),
        -6.0,
        "a body walking OUT of an overlap was slowed down, which is how a pile \
         of bodies becomes a pile that cannot disperse",
    );
}

/// A BODY THAT IS NOT IN THE WAY IS NOT IN THE WAY.
///
///  the falsifier for a pass that tests only the axis it is resolving: two
/// fighters on different platforms are a hundred units apart vertically and
/// perfectly aligned horizontally, and a cross-axis-blind constraint would have
/// them shoving each other through the floor.
#[test]
fn a_body_that_does_not_overlap_across_the_axis_constrains_nothing() {
    let blockers = [still(40.0, 200.0)];
    assert_eq!(
        constrain_motion(
            body(0.0, 0.0),
            30.0,
            true,
            WALK,
            1.0,
            field(&blockers, 1.0, 30.0)
        ),
        30.0
    );
}

/// EDGE TO EDGE IS NOT IN THE WAY, the same strict-overlap rule the world
/// sweep states.
#[test]
fn boxes_touching_exactly_on_the_cross_axis_do_not_block() {
    // Mover spans y ∈ [-20, 20]; blocker spans y ∈ [20, 60].
    let blockers = [BodyContactBlocker::new(
        Aabb::new(Vec2::new(40.0, 40.0), Vec2::new(10.0, 20.0)),
        Vec2::ZERO,
    )];
    assert_eq!(
        constrain_motion(
            body(0.0, 0.0),
            30.0,
            true,
            WALK,
            1.0,
            field(&blockers, 1.0, 30.0)
        ),
        30.0
    );
}

/// THE NEAREST BLOCKER WINS, whatever order the snapshot lists them in.
///
///  order-independence is the whole reason the blockers come from a snapshot;
/// a pass whose answer depended on iteration order would make "who moved first"
/// decide who won, which is a desync under rollback and unfairness on a couch.
#[test]
fn the_answer_does_not_depend_on_the_order_of_the_snapshot() {
    let near = still(40.0, 0.0);
    let far = still(80.0, 0.0);
    let forward = [near, far];
    let backward = [far, near];
    let a = constrain_motion(
        body(0.0, 0.0),
        90.0,
        true,
        WALK,
        1.0,
        field(&forward, 1.0, 90.0),
    );
    let b = constrain_motion(
        body(0.0, 0.0),
        90.0,
        true,
        WALK,
        1.0,
        field(&backward, 1.0, 90.0),
    );
    assert_eq!(a, b);
    assert_eq!(a, 20.0, "the nearest blocker decides");
}

/// THE VERTICAL AXIS IS THE SAME RULE, because gravity is cardinal and may
/// point along either one — the side axis under wall gravity is Y.
#[test]
fn the_constraint_is_axis_symmetric() {
    let blockers = [still(0.0, 80.0)];
    // Mover spans y ∈ [-20, 20], blocker y ∈ [60, 100]: 40 free.
    assert_eq!(
        constrain_motion(
            body(0.0, 0.0),
            60.0,
            false,
            WALK,
            1.0,
            field_y(&blockers, 1.0, 60.0)
        ),
        40.0
    );
}

/// MONOTONE, AND NEVER SIGN-FLIPPING. The property the whole design rests
/// on: shortening a proposed delta can only produce a pose the world sweep after
/// it would already have accepted.
#[test]
fn the_constraint_never_adds_motion_and_never_reverses_it() {
    let blockers = [still(30.0, 0.0), still(-30.0, 5.0), still(4.0, -3.0)];
    for resistance in [0.0, 0.3, 1.0] {
        for asked in [-40.0f32, -7.0, -0.5, 0.5, 7.0, 40.0] {
            let got = constrain_motion(
                body(0.0, 0.0),
                asked,
                true,
                WALK,
                1.0,
                field(&blockers, resistance, asked),
            );
            assert!(
                got.abs() <= asked.abs() + 1.0e-6,
                "asked {asked}, got {got} at resistance {resistance}",
            );
            assert!(
                got == 0.0 || got.signum() == asked.signum(),
                "asked {asked}, got {got} at resistance {resistance}",
            );
        }
    }
}

///  CONTACT IS ABOUT WALKING — a knockback launch ploughs through it.
///
/// A fighter launched sideways at 2400 px/s past a body standing 8px away lost a slice of every
/// tick it stayed in contact; the slices compounded against the controller's own decay, and the
/// body came down about eighty pixels short of the blast margin — close enough to look like it
/// flew, and far enough that the match never ended. The capability had quietly become a way to
/// survive a knockout by standing next to somebody.
///
///  so the rule is a QUESTION, not a budget: is this body walking? Two
/// fighters walking into each other stall where they meet; a launched fighter
/// goes through everybody. One number decides both, and it is the walking body's
/// own top speed.
#[test]
fn contact_only_resists_a_body_that_is_walking() {
    let blockers = [still(15.0, 0.0)];
    let mover = body(0.0, 0.0);
    assert!(
        mover.strict_intersects(blockers[0].aabb),
        "the fixture must start in contact or the budget is never reached",
    );

    // A WALKING step, entirely inside the budget: taken to zero by a wall.
    assert_eq!(
        constrain_motion(mover, 4.5, true, 4.5, 1.0, field(&blockers, 1.0, 4.5)),
        0.0
    );

    // A LAUNCH — nine walks' worth in one step — loses exactly one walk.
    assert_eq!(
        constrain_motion(mover, 40.0, true, 4.5, 1.0, field(&blockers, 1.0, 40.0)),
        40.0
    );
    assert_eq!(
        constrain_motion(mover, -40.0, true, 4.5, 1.0, field(&blockers, 1.0, -40.0)),
        -40.0
    );

    //  and the budget is a FLOOR, never a boost: a step the constraint would
    // not have touched is not lengthened by having a budget.
    let far = [still(400.0, 0.0)];
    assert_eq!(
        constrain_motion(mover, 4.5, true, 1_000.0, 1.0, field(&far, 1.0, 4.5)),
        4.5,
    );
}

/// TWO MOVERS MAY NOT BOTH SPEND ONE GAP.
///
///  resistance did not save it and could not. The free-gap part of a step
/// is granted at full speed by construction, so this happened at `1.0` — the
/// value whose entire promise is that it stops a body exactly at contact.
#[test]
fn two_bodies_closing_on_one_gap_divide_it_instead_of_each_taking_it() {
    // Each spans ±10 about its centre, so centres 25 apart leave a 5 gap.
    let left = body(0.0, 0.0);
    let right = body(25.0, 0.0);
    // Both walking at each other at 4 units per tick, in the same snapshot.
    let speed = 4.0;
    let for_left = [closing(25.0, 0.0, -speed)];
    let for_right = [closing(0.0, 0.0, speed)];

    let moved_left = constrain_motion(
        left,
        speed,
        true,
        WALK,
        1.0,
        BodyContactField::moving(&for_left, 1.0, Vec2::new(speed, 0.0)),
    );
    let moved_right = constrain_motion(
        right,
        -speed,
        true,
        WALK,
        1.0,
        BodyContactField::moving(&for_right, 1.0, Vec2::new(-speed, 0.0)),
    );

    let closed = moved_left - moved_right;
    assert!(
        closed <= 5.0 + 1.0e-4,
        "two solids closed {closed} across a gap of 5 — each was granted the \
         whole of it",
    );
    //  and they must MEET, not stop short: a pass that solved the overlap
    // by refusing both bodies would satisfy the line above and be a worse bug.
    assert!(
        closed >= 5.0 - 1.0e-4,
        "two solids closed only {closed} of a 5 gap — they stopped short of \
         each other",
    );
    // Symmetric bodies at symmetric speeds meet in the middle. That is a
    // consequence of proportion, not a rule of its own.
    assert!(
        (moved_left - 2.5).abs() < 1.0e-4 && (moved_right + 2.5).abs() < 1.0e-4,
        "the split was not symmetric: {moved_left} and {moved_right}",
    );
}

/// A BODY WALKING AT A STATIONARY ONE STILL GETS THE WHOLE GAP.
///
/// Halving passes the pair case and fails this one — a fighter walking at somebody standing
/// still would stop half a gap short of them for no reason a player could see.
#[test]
fn a_lone_mover_is_not_made_to_share_a_gap_with_a_body_that_is_standing_still() {
    let blockers = [still(40.0, 0.0)];
    assert_eq!(
        constrain_motion(
            body(0.0, 0.0),
            30.0,
            true,
            WALK,
            1.0,
            BodyContactField::moving(&blockers, 1.0, Vec2::new(30.0, 0.0)),
        ),
        20.0,
        "a body whose neighbours are not moving must spend the whole 20 of free \
         space, exactly as it did before the gap could be split",
    );
}

/// A BLOCKER LEAVING DOES NOT TAKE A SHARE OF THE GAP.
///
/// The split counts CLOSING speed only. A body walking away is not spending the
/// space between them, and charging it for the space anyway would slow a chase
/// down to nothing.
#[test]
fn a_blocker_travelling_away_is_not_counted_as_spending_the_gap() {
    let fleeing = [closing(40.0, 0.0, 4.0)];
    assert_eq!(
        constrain_motion(
            body(0.0, 0.0),
            30.0,
            true,
            WALK,
            1.0,
            BodyContactField::moving(&fleeing, 1.0, Vec2::new(30.0, 0.0)),
        ),
        20.0,
        "a chaser was charged for space the body ahead of it was vacating",
    );
}

/// FOUR BODIES CLOSING ON ONE POINT STILL DIVIDE WHAT IS THERE.
///
///  the pair case generalises because the share is computed per BLOCKER: each
/// neighbour's own closing speed decides how much of the gap to that neighbour
/// belongs to this body, and the minimum over neighbours is what the body gets.
#[test]
fn the_split_is_per_blocker_so_the_nearest_closing_neighbour_still_binds() {
    // A neighbour 5 away closing at 4, and one 40 away standing still.
    let blockers = [closing(25.0, 0.0, -4.0), still(80.0, 0.0)];
    let moved = constrain_motion(
        body(0.0, 0.0),
        4.0,
        true,
        WALK,
        1.0,
        BodyContactField::moving(&blockers, 1.0, Vec2::new(4.0, 0.0)),
    );
    assert!(
        (moved - 2.5).abs() < 1.0e-4,
        "the near closing neighbour did not bind: moved {moved}",
    );
}
