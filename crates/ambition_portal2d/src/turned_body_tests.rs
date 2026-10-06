//! The portal core is given the box a body has.
//!
//! The transit, the carve and the eviction asked a body's size and built its
//! box from `kin.size`: the level box. In turned gravity a body that is not
//! square lies along its floor, so a body that fits an opening was refused, a
//! body too long for it crossed, a body that touched an opening cut no hole,
//! and a closing portal pushed a body its plane did not touch.
//!
//! The core is a reader of every body and has no frame of one. The record of
//! the last step ([`SweepSample`]) has the DOWN of the body, so each reader
//! asks `BodyKinematics::collision_box(record)`.
//!
//! Each arm gives the body a record with DOWN `+x` and has a control with
//! DOWN `+y`, so that it is the turn that changes the answer and not the
//! record.

use bevy::prelude::*;

use ambition_platformer2d_core::{BodyKinematics, SweepSample};

use crate::color::{PortalChannel, PortalGunColor};
use crate::types::{portal_half_extent, portal_half_extent_with_length, PlacedPortal};

const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);
const ORANGE: PortalChannel = PortalChannel::Gun(PortalGunColor::ORANGE);

const SIDEWAYS: Vec2 = Vec2::new(1.0, 0.0);
const LEVEL: Vec2 = Vec2::new(0.0, 1.0);

/// A body that stands in normal gravity: 24 wide, 40 tall.
const TALL: Vec2 = Vec2::new(24.0, 40.0);
/// A body that is 40 wide and 24 tall in normal gravity.
const WIDE: Vec2 = Vec2::new(40.0, 24.0);

/// The record of a step that ended where the body is, with the DOWN `down`.
fn record(body: BodyKinematics, down: Vec2) -> SweepSample {
    SweepSample { vel: body.vel, ..SweepSample::at_rest(body, down) }
}

/// A wall pair: blue at x = 20 facing +x, orange at x = 380 facing -x, each
/// with an opening of `opening` along y.
fn wall(channel: PortalChannel, opening: f32) -> PlacedPortal {
    let (x, normal) = if channel == BLUE {
        (20.0, Vec2::new(1.0, 0.0))
    } else {
        (380.0, Vec2::new(-1.0, 0.0))
    };
    PlacedPortal::fixed(
        channel,
        Vec2::new(x, 200.0),
        normal,
        portal_half_extent_with_length(normal, opening * 0.5),
    )
}

/// The opening of the standard portal.
const STANDARD: f32 = crate::types::PORTAL_OPENING_HALF * 2.0;
/// An opening that a side of 24 fits and a side of 40 does not.
const NARROW: f32 = 32.0;

/// A body of `size` at the blue portal, moving into it. Answer its x after
/// the two frames a transit takes.
fn x_after_the_blue(size: Vec2, down: Vec2, opening: f32) -> f32 {
    let mut app = App::new();
    app.add_message::<crate::PortalBodyEntered>();
    app.add_message::<crate::PortalBodyTransited>();
    app.init_resource::<crate::PortalTuning>();
    app.add_systems(Update, crate::portal_transit);
    for channel in [BLUE, ORANGE] {
        app.world_mut().spawn(wall(channel, opening));
    }
    let body = BodyKinematics {
        pos: Vec2::new(20.0, 200.0),
        vel: Vec2::new(-100.0, 0.0),
        size,
        facing: -1.0,
    };
    let entity = app.world_mut().spawn((body, record(body, down))).id();
    // Frame 1 begins (the leading edge is in the opening), frame 2 transfers.
    app.update();
    app.update();
    app.world().get::<BodyKinematics>(entity).unwrap().pos.x
}

#[track_caller]
fn crosses(size: Vec2, down: Vec2, opening: f32) {
    let x = x_after_the_blue(size, down, opening);
    assert!(x > 250.0, "the body {size:?} with DOWN {down:?} fits the opening {opening} and must cross: x = {x}");
}

#[track_caller]
fn stays(size: Vec2, down: Vec2, opening: f32) {
    let x = x_after_the_blue(size, down, opening);
    assert!(x < 100.0, "the body {size:?} with DOWN {down:?} does not fit the opening {opening} and must stay: x = {x}");
}

/// The premise: a record does not stop a transit, and the narrow opening
/// refuses a side of 40 and takes a side of 24.
#[test]
fn a_level_body_crosses_the_opening_its_height_fits() {
    crosses(TALL, LEVEL, STANDARD);
    stays(TALL, LEVEL, NARROW);
    crosses(WIDE, LEVEL, NARROW);
}

#[test]
fn a_body_that_lies_along_sideways_gravity_crosses_the_opening_its_own_box_fits() {
    // It lies along its floor: 24 along the opening, not 40.
    crosses(TALL, SIDEWAYS, NARROW);
}

#[test]
fn a_body_that_lies_along_sideways_gravity_does_not_cross_an_opening_its_own_box_does_not_fit() {
    // It lies along its floor: 40 along the opening, not 24.
    stays(WIDE, SIDEWAYS, NARROW);
}

/// How many holes the carve cuts for a body at rest in front of the blue
/// wall, with its centre at x = 50. The capture box of the wall ends at
/// x = 35: 15 from the centre of the body.
fn holes_cut(size: Vec2, down: Vec2) -> usize {
    let mut app = App::new();
    app.init_resource::<crate::PortalCarves>();
    app.add_systems(Update, crate::publish_portal_carves);
    for channel in [BLUE, ORANGE] {
        app.world_mut().spawn(wall(channel, STANDARD));
    }
    let body = BodyKinematics {
        pos: Vec2::new(50.0, 200.0),
        vel: Vec2::ZERO,
        size,
        facing: -1.0,
    };
    app.world_mut().spawn((body, record(body, down)));
    app.update();
    app.world().resource::<crate::PortalCarves>().holes.len()
}

/// The control of the two carve arms, in normal gravity.
#[test]
fn a_level_body_cuts_the_hole_of_the_opening_it_touches() {
    assert_eq!(holes_cut(TALL, LEVEL), 0, "the level body is 12 deep toward the wall, and is 15 from the opening");
    assert_eq!(holes_cut(WIDE, LEVEL), 1, "the level body is 20 deep toward the wall");
}

#[test]
fn a_body_that_lies_toward_the_wall_cuts_the_hole_of_the_opening_its_own_box_touches() {
    assert_eq!(holes_cut(TALL, SIDEWAYS), 1, "the body is 20 deep toward the wall, and touches the opening");
}

#[test]
fn a_body_that_lies_along_the_wall_cuts_no_hole_its_own_box_does_not_touch() {
    assert_eq!(holes_cut(WIDE, SIDEWAYS), 0, "the body is 12 deep toward the wall, and is 15 from the opening");
}

/// A floor portal at y = 300 closes under a body whose centre is 15 over its
/// plane. Answer where the body is after the eviction.
fn after_the_floor_closes(size: Vec2, down: Vec2) -> Vec2 {
    let mut app = App::new();
    app.init_resource::<crate::PortalFrameHistory>();
    app.add_systems(Update, crate::evict_straddlers_on_portal_change);
    let normal = Vec2::new(0.0, -1.0);
    let closing = app
        .world_mut()
        .spawn(PlacedPortal::fixed(BLUE, Vec2::new(500.0, 300.0), normal, portal_half_extent(normal)))
        .id();
    let body = BodyKinematics {
        pos: Vec2::new(500.0, 285.0),
        vel: Vec2::ZERO,
        size,
        facing: 1.0,
    };
    let entity = app.world_mut().spawn((body, record(body, down))).id();
    app.update();
    app.world_mut().entity_mut(closing).despawn();
    app.update();
    app.world().get::<BodyKinematics>(entity).unwrap().pos
}

const OVER_THE_PLANE: Vec2 = Vec2::new(500.0, 285.0);

/// The control of the two eviction arms, in normal gravity: the tall body is
/// 20 deep toward the plane and is pushed clear of it; the wide body is 12
/// deep and is not touched.
#[test]
fn a_closing_portal_pushes_a_level_body_its_plane_cuts() {
    let tall = after_the_floor_closes(TALL, LEVEL);
    assert!(tall.y + 20.0 <= 300.0 + 1e-3 && tall != OVER_THE_PLANE, "{tall:?}");
    assert_eq!(after_the_floor_closes(WIDE, LEVEL), OVER_THE_PLANE);
}

/// In sideways gravity the tall body lies level with the plane: 12 deep.
#[test]
fn a_closing_portal_does_not_push_a_body_whose_own_box_its_plane_does_not_cut() {
    assert_eq!(
        after_the_floor_closes(TALL, SIDEWAYS),
        OVER_THE_PLANE,
        "the plane does not cut the box of the body, and the portal must not push it"
    );
}

/// In sideways gravity the wide body is 20 deep toward the plane.
#[test]
fn a_closing_portal_pushes_clear_a_body_whose_own_box_its_plane_cuts() {
    let wide = after_the_floor_closes(WIDE, SIDEWAYS);
    assert!(
        wide != OVER_THE_PLANE && wide.y + 20.0 <= 300.0 + 1e-3,
        "the plane cuts the box of the body, and the portal must push it clear: {wide:?}"
    );
}
