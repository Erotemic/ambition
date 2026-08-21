use super::*;
use crate::{AabbExt, Vec2};

/// A walk budget wide enough not to bind in these units tests: what
/// `walk_budget` DOES is the subject of its own test at the bottom, and every
/// other claim here is about the constraint itself.
const WALK: f32 = 1_000.0;

fn body(center_x: f32, center_y: f32) -> Aabb {
    Aabb::new(Vec2::new(center_x, center_y), Vec2::new(10.0, 20.0))
}

/// **THE IDENTITY, and it is the one every body in the engine gets.**
///
/// ⛔ body contact is a capability a composition grants. A body that never opted
/// in must be resolved byte-for-byte as it was before this module existed, and
/// the two ways that can fail — no blockers, or blockers with no resistance —
/// are both here.
#[test]
fn a_body_nobody_resists_keeps_every_bit_of_its_motion() {
    let mover = body(0.0, 0.0);
    assert_eq!(
        constrain_motion(mover, 7.5, true, WALK, BodyContactField::NONE),
        7.5
    );
    let neighbour = [body(15.0, 0.0)];
    assert_eq!(
        constrain_motion(
            mover,
            7.5,
            true,
            WALK,
            BodyContactField::new(&neighbour, 0.0)
        ),
        7.5,
        "zero resistance is opting OUT, not opting in weakly",
    );
}

/// **A SOLID STOPS THE BODY AT CONTACT — and exactly at contact.**
///
/// ⚠ the number is DERIVED from the two boxes, not copied from a run: the mover
/// spans ±10 about x=0 and the blocker ±10 about x=40, so 20 units of free space
/// lie between them. Asking for 30 must yield 20.
#[test]
fn full_resistance_spends_the_gap_and_stops() {
    let blockers = [body(40.0, 0.0)];
    let field = BodyContactField::new(&blockers, 1.0);
    assert_eq!(
        constrain_motion(body(0.0, 0.0), 30.0, true, WALK, field),
        20.0
    );
    assert_eq!(
        constrain_motion(body(0.0, 0.0), 12.0, true, WALK, field),
        12.0,
        "a step that fits in the gap is untouched",
    );
}

/// **PARTIAL RESISTANCE IS THE SMASH-LIKE FEEL: you push through, slowly.**
///
/// Where the genres differ, ship the knob. Ultimate's fighters displace each
/// other rather than colliding; a beat-em-up may want the wall. Neither is more
/// correct, so the capability carries the number instead of choosing.
#[test]
fn partial_resistance_keeps_a_share_of_the_motion_that_goes_deeper() {
    let blockers = [body(40.0, 0.0)];
    let field = BodyContactField::new(&blockers, 0.75);
    // 20 free, 10 deeper, a quarter of which survives.
    assert_eq!(
        constrain_motion(body(0.0, 0.0), 30.0, true, WALK, field),
        22.5
    );
}

/// **⛔⛔ A PRE-EXISTING OVERLAP IS NEVER TELEPORTED APART — AND THE WAY OUT IS
/// NEVER RESISTED.**
///
/// The first half is Jon's ruling and is not negotiable: AVOID PUSHOUT is about
/// geometry REPAIR, and a constraint that separated overlapping bodies would be
/// exactly the thing the rule forbids wearing a new name. Nothing in this module
/// ever writes a position.
///
/// ⛔⛔ **the second half was WRONG for a day, and it was measured.** This test
/// used to assert that motion is slowed in BOTH directions while overlapping, on
/// the argument that the pass cannot know which way "out" is. It can: an
/// infinitesimal step either increases the axis overlap or it does not, and that
/// is arithmetic rather than a guess. With the symmetric version, four fighters
/// spawning on one point could not walk apart — every step out of the pile was
/// cut to a fraction and a free-for-all never resolved.
///
/// ⚠ **declining to resist a body that is leaving is not a pushout**, because
/// nothing moves it. It is only this pass keeping its hands off a body that is
/// already resolving the situation itself.
#[test]
fn an_overlap_resists_going_deeper_and_never_resists_coming_out() {
    let blockers = [body(8.0, 0.0)];
    let field = BodyContactField::new(&blockers, 0.5);
    let mover = body(0.0, 0.0);
    assert!(
        mover.strict_intersects(blockers[0]),
        "the fixture must actually start overlapping or this proves nothing",
    );
    assert_eq!(
        constrain_motion(mover, 6.0, true, WALK, field),
        3.0,
        "deeper is halved",
    );
    assert_eq!(
        constrain_motion(mover, -6.0, true, WALK, field),
        -6.0,
        "a body walking OUT of an overlap was slowed down, which is how a pile \
         of bodies becomes a pile that cannot disperse",
    );
}

/// **A BODY THAT IS NOT IN THE WAY IS NOT IN THE WAY.**
///
/// ⛔ the falsifier for a pass that tests only the axis it is resolving: two
/// fighters on different platforms are a hundred units apart vertically and
/// perfectly aligned horizontally, and a cross-axis-blind constraint would have
/// them shoving each other through the floor.
#[test]
fn a_body_that_does_not_overlap_across_the_axis_constrains_nothing() {
    let blockers = [body(40.0, 200.0)];
    let field = BodyContactField::new(&blockers, 1.0);
    assert_eq!(
        constrain_motion(body(0.0, 0.0), 30.0, true, WALK, field),
        30.0
    );
}

/// **EDGE TO EDGE IS NOT IN THE WAY**, the same strict-overlap rule the world
/// sweep states.
#[test]
fn boxes_touching_exactly_on_the_cross_axis_do_not_block() {
    // Mover spans y ∈ [-20, 20]; blocker spans y ∈ [20, 60].
    let blockers = [Aabb::new(Vec2::new(40.0, 40.0), Vec2::new(10.0, 20.0))];
    let field = BodyContactField::new(&blockers, 1.0);
    assert_eq!(
        constrain_motion(body(0.0, 0.0), 30.0, true, WALK, field),
        30.0
    );
}

/// **THE NEAREST BLOCKER WINS, whatever order the snapshot lists them in.**
///
/// ⛔ order-independence is the whole reason the blockers come from a snapshot;
/// a pass whose answer depended on iteration order would make "who moved first"
/// decide who won, which is a desync under rollback and unfairness on a couch.
#[test]
fn the_answer_does_not_depend_on_the_order_of_the_snapshot() {
    let near = body(40.0, 0.0);
    let far = body(80.0, 0.0);
    let forward = [near, far];
    let backward = [far, near];
    let a = constrain_motion(
        body(0.0, 0.0),
        90.0,
        true,
        WALK,
        BodyContactField::new(&forward, 1.0),
    );
    let b = constrain_motion(
        body(0.0, 0.0),
        90.0,
        true,
        WALK,
        BodyContactField::new(&backward, 1.0),
    );
    assert_eq!(a, b);
    assert_eq!(a, 20.0, "the nearest blocker decides");
}

/// **THE VERTICAL AXIS IS THE SAME RULE**, because gravity is cardinal and may
/// point along either one — the side axis under wall gravity is Y.
#[test]
fn the_constraint_is_axis_symmetric() {
    let blockers = [body(0.0, 80.0)];
    let field = BodyContactField::new(&blockers, 1.0);
    // Mover spans y ∈ [-20, 20], blocker y ∈ [60, 100]: 40 free.
    assert_eq!(
        constrain_motion(body(0.0, 0.0), 60.0, false, WALK, field),
        40.0
    );
}

/// **MONOTONE, AND NEVER SIGN-FLIPPING.** The property the whole design rests
/// on: shortening a proposed delta can only produce a pose the world sweep after
/// it would already have accepted.
#[test]
fn the_constraint_never_adds_motion_and_never_reverses_it() {
    let blockers = [body(30.0, 0.0), body(-30.0, 5.0), body(4.0, -3.0)];
    for resistance in [0.0, 0.3, 1.0] {
        let field = BodyContactField::new(&blockers, resistance);
        for asked in [-40.0f32, -7.0, -0.5, 0.5, 7.0, 40.0] {
            let got = constrain_motion(body(0.0, 0.0), asked, true, WALK, field);
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

/// **⛔⛔ CONTACT IS ABOUT WALKING — a knockback launch ploughs through it.**
///
/// Measured 2026-08-20, and it took two attempts to get right. A fighter
/// launched sideways at 2400 px/s past a body standing 8px away lost a slice of
/// every tick it stayed in contact; the slices compounded against the
/// controller's own decay, and the body came down about eighty pixels short of
/// the blast margin — close enough to look like it flew, and far enough that the
/// match never ended. THREE `smash_it` guards about matches ENDING went red, and
/// a "take at most a walk's worth per tick" version fixed only two of them. The
/// capability had quietly become a way to survive a knockout by standing next to
/// somebody.
///
/// ⭐ **so the rule is a QUESTION, not a budget: is this body walking?** Two
/// fighters walking into each other stall where they meet; a launched fighter
/// goes through everybody. One number decides both, and it is the walking body's
/// own top speed.
#[test]
fn contact_only_resists_a_body_that_is_walking() {
    let blockers = [body(15.0, 0.0)];
    let solid = BodyContactField::new(&blockers, 1.0);
    let mover = body(0.0, 0.0);
    assert!(
        mover.strict_intersects(blockers[0]),
        "the fixture must start in contact or the budget is never reached",
    );

    // A WALKING step, entirely inside the budget: taken to zero by a wall.
    assert_eq!(constrain_motion(mover, 4.5, true, 4.5, solid), 0.0);

    // A LAUNCH — nine walks' worth in one step — loses exactly one walk.
    assert_eq!(constrain_motion(mover, 40.0, true, 4.5, solid), 40.0);
    assert_eq!(constrain_motion(mover, -40.0, true, 4.5, solid), -40.0);

    // ⚠ and the budget is a FLOOR, never a boost: a step the constraint would
    // not have touched is not lengthened by having a budget.
    let far = [body(400.0, 0.0)];
    assert_eq!(
        constrain_motion(mover, 4.5, true, 1_000.0, BodyContactField::new(&far, 1.0)),
        4.5,
    );
}
