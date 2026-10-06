//! Each world read of a step asks the box the step moved.
//!
//! The hazard gate has this rule ([`super::hazard_footprint`]). Four more
//! readers asked the world with the level box (`BodyKinematics::aabb`): the
//! water and the ladder the axis arm caches, the knock-off test of a ledge
//! carry, and the rebound pad of the momentum arm. In sideways gravity a body
//! that is not square lies along its support, so the level box found water
//! beside the body and missed water under its own end.
//!
//! Each case uses the 48x22 body and one thin region. Region `UNDER_END` is
//! under the end of the body when it lies along sideways gravity, and region
//! `BESIDE` is beside it. The two boxes give opposite answers for each region.

use super::super::*;
use super::hazard_footprint::{hazard_at, normal_gravity, room, sideways_gravity, Body, BODY, STEP_HALF};
use crate::world::{Block, ClimbableRegion, WaterKind, WaterRegion};
use crate::{AbilitySet, Aabb, Vec2, World};

/// A thin region as (min, max) offsets from the centre of the body.
type Offsets = (Vec2, Vec2);
/// The box of the step touches this region in sideways gravity; the level box
/// does not.
const UNDER_END: Offsets = (Vec2::new(-4.0, 16.0), Vec2::new(4.0, 20.0));
/// The level box touches this region; the box of the step in sideways gravity
/// does not.
const BESIDE: Offsets = (Vec2::new(15.0, -3.0), Vec2::new(20.0, 3.0));

/// Where the body is at the start of its step.
const AT: Vec2 = Vec2::new(1000.0, 1200.0);
const PAD_IMPULSE: Vec2 = Vec2::new(0.0, -500.0);

fn region(offsets: Offsets) -> Aabb {
    hazard_at(AT, offsets.0, offsets.1)
}

fn overlaps(a: Aabb, b: Aabb) -> bool {
    a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
}

/// The premise of every case: the two boxes disagree about the region.
#[track_caller]
fn the_boxes_disagree(offsets: Offsets, step_box_touches: bool) {
    let region = region(offsets);
    assert_eq!(overlaps(Aabb::new(AT, STEP_HALF), region), step_box_touches, "premise: the box of the step");
    assert_eq!(overlaps(Aabb::new(AT, BODY * 0.5), region), !step_box_touches, "premise: the level box");
}

fn axis_body() -> Body {
    Body::in_sideways_gravity(MotionModelSpec::AxisSwept(AxisSweptParams::default()), Vec2::ZERO)
}

fn with_water(offsets: Offsets) -> World {
    let mut world = room(None);
    world
        .water_regions
        .push(WaterRegion::new(region(offsets), WaterKind::Clear, Default::default()));
    world
}

fn with_ladder(offsets: Offsets) -> World {
    let mut world = room(None);
    world.climbable_regions.push(ClimbableRegion::ladder(region(offsets)));
    world
}

fn with_pad(offsets: Offsets) -> World {
    let mut world = room(None);
    let pad = region(offsets);
    world.blocks.push(Block::rebound("thin pad", pad.min, pad.max - pad.min, PAD_IMPULSE));
    world
}

/// A body with no swim ability drowns in the water it is in.
#[test]
fn a_body_in_sideways_gravity_drowns_in_water_its_own_box_touches() {
    the_boxes_disagree(UNDER_END, true);
    assert!(!axis_body().abilities.swim, "premise: the body cannot swim");
    let stepped = axis_body().step(&with_water(UNDER_END), true);
    assert!(stepped.in_water, "the water under the end of the body is the water it is in");
    assert_eq!(stepped.reset, Some(ResetCause::Drowned));
}

#[test]
fn a_body_in_sideways_gravity_does_not_drown_in_water_beside_it() {
    the_boxes_disagree(BESIDE, false);
    let stepped = axis_body().step(&with_water(BESIDE), true);
    assert!(!stepped.in_water, "the water beside the body does not touch its box");
    assert_eq!(stepped.reset, None);
}

#[test]
fn a_body_in_sideways_gravity_finds_a_ladder_its_own_box_touches() {
    the_boxes_disagree(UNDER_END, true);
    let stepped = axis_body().step(&with_ladder(UNDER_END), true);
    assert!(stepped.on_climbable, "the ladder under the end of the body touches its box");
}

#[test]
fn a_body_in_sideways_gravity_finds_no_ladder_beside_it() {
    the_boxes_disagree(BESIDE, false);
    let stepped = axis_body().step(&with_ladder(BESIDE), true);
    assert!(!stepped.on_climbable, "the ladder beside the body does not touch its box");
}

/// The two policies that read a rebound pad. The axis arm already asked the
/// box of its step, so it is the control of the momentum arm.
fn policies_that_take_a_pad() -> [(&'static str, MotionModelSpec); 2] {
    [
        ("axis-swept", MotionModelSpec::AxisSwept(AxisSweptParams::default())),
        ("surface momentum", MotionModelSpec::SurfaceMomentum(Default::default())),
    ]
}

fn pad_taker(spec: MotionModelSpec) -> Body {
    let mut body = Body::in_sideways_gravity(spec, Vec2::ZERO);
    body.abilities = AbilitySet {
        rebound: true,
        ..AbilitySet::default()
    };
    body
}

/// The pad is read at the end of the step. The premise is from a room with no
/// pad: the body at rest moves less than one unit, and gets no velocity on y.
#[track_caller]
fn judge_pad(name: &str, body: Body, offsets: Offsets, takes_it: bool) {
    let clear = body.step(&room(None), true);
    assert!((clear.pos - AT).length() < 1.0, "{name}: premise, the body moves less than one unit: {:?}", clear.pos);
    assert!(clear.vel.y.abs() < 1.0, "{name}: premise, the fall gives no velocity on y: {:?}", clear.vel);
    let stepped = body.step(&with_pad(offsets), true);
    if takes_it {
        assert!(
            (stepped.vel.y - PAD_IMPULSE.y).abs() < 1.0,
            "{name}: the pad touches the box of the step, and the body must take its impulse: {:?}",
            stepped.vel
        );
    } else {
        assert!(
            stepped.vel.y.abs() < 1.0,
            "{name}: the pad does not touch the box of the step, and the body must not take its impulse: {:?}",
            stepped.vel
        );
    }
}

#[test]
fn a_body_in_sideways_gravity_takes_a_pad_its_own_box_touches() {
    the_boxes_disagree(UNDER_END, true);
    for (name, spec) in policies_that_take_a_pad() {
        judge_pad(name, pad_taker(spec), UNDER_END, true);
    }
}

#[test]
fn a_body_in_sideways_gravity_takes_no_pad_beside_it() {
    the_boxes_disagree(BESIDE, false);
    for (name, spec) in policies_that_take_a_pad() {
        judge_pad(name, pad_taker(spec), BESIDE, false);
    }
}

/// The travel of the solid that carries the hang, in one tick.
const CARRY: Vec2 = Vec2::new(0.0, -0.5);

/// A moving solid, and the contact of a body that hangs on its lip at [`AT`]
/// in sideways gravity. The contact is the inverse the carry rule matches
/// (`ledge_contact_is_on_block`): down is +x and the side axis is -y.
fn carrier_and_hang() -> (Block, crate::LedgeContact) {
    let half = BODY * 0.5;
    // The lip is the face of the solid that faces against gravity (its left
    // face). The body is beside the face of the solid at the smaller y.
    let lip_x = AT.x - (half.y - 4.0);
    let wall_y = AT.y + (half.x - 1.0);
    let mut carrier = Block::solid("carrier", Vec2::new(lip_x, wall_y), Vec2::new(80.0, 40.0));
    carrier.velocity = CARRY;
    let contact = crate::LedgeContact {
        wall_normal_x: 1.0,
        anchor: AT,
        climb_target: Vec2::new(lip_x - half.y - 1.0, wall_y + half.x + 4.0),
    };
    (carrier, contact)
}

fn hanging_body() -> Body {
    let mut body = axis_body();
    body.abilities = AbilitySet {
        ledge_grab: true,
        ..AbilitySet::default()
    };
    body.hang = Some(carrier_and_hang().1);
    body
}

fn with_carrier(solid: Option<Offsets>) -> World {
    let mut world = room(None);
    world.blocks.push(carrier_and_hang().0);
    if let Some(offsets) = solid {
        let solid = region(offsets);
        world.blocks.push(Block::solid("thin solid", solid.min, solid.max - solid.min));
    }
    world
}

/// The premise of the two ledge cases: with no other solid, the solid carries
/// the hang by its travel.
#[track_caller]
fn the_carrier_carries_the_hang() {
    let carried = hanging_body().step(&with_carrier(None), true);
    assert!(carried.hanging, "premise: the hang holds on the moving solid");
    assert!(
        (carried.pos - (AT + CARRY)).length() < 1.0e-3,
        "premise: the solid carries the hang by its travel: {:?}",
        carried.pos
    );
}

#[test]
fn a_hang_on_a_moving_solid_is_not_broken_by_a_solid_beside_the_body() {
    the_boxes_disagree(BESIDE, false);
    the_carrier_carries_the_hang();
    let stepped = hanging_body().step(&with_carrier(Some(BESIDE)), true);
    assert!(stepped.hanging, "the carry does not move the box of the body into the solid beside it");
    assert!((stepped.pos - (AT + CARRY)).length() < 1.0e-3, "the solid carries the hang: {:?}", stepped.pos);
}

#[test]
fn a_hang_on_a_moving_solid_is_broken_by_a_solid_the_carry_moves_its_box_into() {
    the_boxes_disagree(UNDER_END, true);
    the_carrier_carries_the_hang();
    let stepped = hanging_body().step(&with_carrier(Some(UNDER_END)), true);
    assert!(!stepped.hanging, "the carry moves the box of the body into a solid, and the hang must break");
}

/// The rule is the frame's turn, not a swap of the two sides: in normal
/// gravity the box of the step is the level box, and each reader answers as
/// the level box does.
#[test]
fn in_normal_gravity_each_world_read_asks_the_level_box() {
    let level = |spec: MotionModelSpec| {
        let mut body = pad_taker(spec);
        body.frame = normal_gravity();
        body
    };
    let axis = level(MotionModelSpec::AxisSwept(AxisSweptParams::default()));
    assert_eq!(axis.frame.down(), Vec2::new(0.0, 1.0), "premise: normal gravity");
    assert_ne!(sideways_gravity().down(), axis.frame.down());

    let dry = axis.step(&with_water(UNDER_END), true);
    assert!(!dry.in_water && dry.reset.is_none(), "the water is under the level body, and does not touch it");
    let wet = axis.step(&with_water(BESIDE), true);
    assert!(wet.in_water, "the water touches the side of the level body");
    assert_eq!(wet.reset, Some(ResetCause::Drowned));

    assert!(!axis.step(&with_ladder(UNDER_END), true).on_climbable);
    assert!(axis.step(&with_ladder(BESIDE), true).on_climbable);

    for (name, spec) in policies_that_take_a_pad() {
        let body = level(spec);
        let missed = body.step(&with_pad(UNDER_END), true);
        assert!(missed.vel.y > -1.0, "{name}: the pad is under the level body: {:?}", missed.vel);
        let taken = body.step(&with_pad(BESIDE), true);
        assert!((taken.vel.y - PAD_IMPULSE.y).abs() < 1.0, "{name}: the pad touches the level body: {:?}", taken.vel);
    }
}
