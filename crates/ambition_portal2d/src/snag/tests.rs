//! A body goes through an opening whole: part of it past an end holds it.

use bevy::prelude::*;

use ambition_platformer2d_core::{self as ae, BodyKinematics, SweepSample};

use super::snag_push;
use crate::color::{PortalChannel, PortalGunColor};
use crate::transit::{FreePortalBody, PortalTransit};
use crate::types::PlacedPortal;

const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);
const ORANGE: PortalChannel = PortalChannel::Gun(PortalGunColor::ORANGE);
/// A floor portal at y = 300 (facing up), 60 wide.
const FLOOR: Vec2 = Vec2::new(0.0, 300.0);
const HALF: f32 = 30.0;
const BODY: Vec2 = Vec2::new(10.0, 20.0);

fn floor() -> PlacedPortal {
    PlacedPortal::fixed(BLUE, FLOOR, Vec2::new(0.0, -1.0), HALF)
}

fn exit() -> PlacedPortal {
    PlacedPortal::fixed(ORANGE, Vec2::new(600.0, 300.0), Vec2::new(0.0, -1.0), HALF)
}

fn at(x: f32, y: f32) -> ae::Aabb {
    ae::Aabb::new(Vec2::new(x, y), BODY)
}

#[test]
fn a_body_over_the_opening_is_not_held() {
    assert_eq!(snag_push(at(0.0, 300.0), Some(at(0.0, 290.0)), &floor().aperture()), None);
}

#[test]
fn a_body_beside_the_opening_is_the_hosts_business() {
    // Its corner dips into the host past the end, and none of it is over the
    // opening: the host holds it, or lets its feet stand in it.
    assert_eq!(snag_push(at(50.0, 305.0), Some(at(50.0, 300.0)), &floor().aperture()), None);
}

/// Half past an end, it came down onto the host there (from just touching
/// it): it goes back out the way it came, along the normal, and stands on
/// the host.
#[test]
fn a_body_half_past_an_end_is_held_on_the_host() {
    let (push, against) = snag_push(at(30.0, 306.0), Some(at(30.0, 280.0)), &floor().aperture()).expect("held");
    assert!((push - Vec2::new(0.0, -26.0)).length() < 1e-3, "{push:?}");
    assert_eq!(against, Vec2::new(0.0, -1.0));
}

/// Straddling the opening and drifting sideways in it, it meets the end of
/// the opening (and its squeeze) and goes back along the face.
#[test]
fn a_body_drifting_past_an_end_inside_the_opening_is_held_at_its_side() {
    let x = 22.0 + super::SQUEEZE;
    let (push, _) = snag_push(at(x, 300.0), Some(at(x - 4.0, 300.0)), &floor().aperture()).expect("held");
    assert!(push.y.abs() < 1e-3 && (push.x + 2.0).abs() < 1e-3, "back toward the middle by 2: {push:?}");
}

/// A free body that falls into a floor portal with part of it past the end
/// is held on the host and does not go through; one over the opening does.
#[derive(Component, Clone, Copy, Debug)]
struct Probe {
    pos: Vec2,
    vel: Vec2,
    step: Option<(Vec2, Vec2)>,
}

#[derive(Component)]
struct Held;

impl FreePortalBody for Probe {
    fn pos(&self) -> Vec2 {
        self.pos
    }
    fn vel(&self) -> Vec2 {
        self.vel
    }
    fn half_extent(&self) -> Vec2 {
        BODY
    }
    fn emerge(&mut self, pos: Vec2, vel: Vec2) {
        self.pos = pos;
        self.vel = vel;
    }
    fn last_step(&self) -> Option<(Vec2, Vec2)> {
        self.step
    }
    type Presence = Held;
    fn present(held: Option<&Held>) -> bool {
        held.is_none()
    }
}

fn fall(mut bodies: Query<&mut Probe>) {
    for mut body in &mut bodies {
        body.vel.y += 1400.0 / 60.0;
        let from = body.pos;
        let vel = body.vel;
        body.pos += vel / 60.0;
        body.step = Some((from, body.pos));
    }
}

/// Where a probe dropped from 20 px over the floor at `x` is after a second,
/// and the furthest right it went (the exit is at x = 600; a floor pair
/// bounces a body between its ends).
fn dropped_at(x: f32) -> (Vec2, f32) {
    let mut app = App::new();
    app.init_resource::<crate::PortalTuning>();
    app.add_message::<crate::PortalBodyEntered>();
    app.add_message::<crate::PortalBodyTransited>();
    app.add_systems(
        Update,
        (fall, crate::snag_free_straddlers::<Probe>, crate::portal_transit_free_bodies::<Probe>).chain(),
    );
    app.world_mut().spawn(floor());
    app.world_mut().spawn(exit());
    let body = app.world_mut().spawn(Probe { pos: Vec2::new(x, 260.0), vel: Vec2::ZERO, step: None }).id();
    let mut furthest = x;
    for _ in 0..60 {
        app.update();
        furthest = furthest.max(app.world().get::<Probe>(body).unwrap().pos.x);
    }
    (app.world().get::<Probe>(body).unwrap().pos, furthest)
}

#[test]
fn a_body_falls_through_an_opening_whole_or_not_at_all() {
    let (_, over) = dropped_at(0.0);
    assert!(over > 500.0, "control: over the opening it goes through: furthest x {over}");
    let (half_off, furthest) = dropped_at(30.0);
    assert!(furthest < 100.0, "half past the end it went through: furthest x {furthest}");
    assert!((half_off.y + BODY.y - 300.0).abs() < 1.0, "it stands on the host at the end: {half_off:?}");
}

/// The kernel body is held the same way, and loses the speed that carried
/// it into the host.
#[test]
fn a_kernel_body_half_past_an_end_is_held() {
    let mut app = App::new();
    app.add_systems(Update, crate::snag_straddlers);
    app.world_mut().spawn(floor());
    app.world_mut().spawn(exit());
    let kin = BodyKinematics { pos: Vec2::new(30.0, 286.0), vel: Vec2::new(0.0, 600.0), size: BODY * 2.0, facing: 1.0 };
    let record = SweepSample { prev: Vec2::new(30.0, 278.0), curr: kin.pos, vel: kin.vel, half: BODY, down: Vec2::new(0.0, 1.0) };
    let body = app.world_mut().spawn((kin, record)).id();
    app.update();
    let kin = app.world().get::<BodyKinematics>(body).unwrap();
    assert!((kin.pos - Vec2::new(30.0, 280.0)).length() < 1e-3, "{:?}", kin.pos);
    assert_eq!(kin.vel, Vec2::ZERO);
}

/// A body sunk in a slope's opening that walks right along it toward the
/// upper end (the robot on pair E, Jon 2026-10-16): where it cuts the slope
/// reaches the end, and it is held there, back along the slope, not lifted
/// out of the opening across the face.
#[test]
fn a_body_walking_past_an_end_in_a_slope_opening_meets_its_side() {
    let normal = Vec2::new(-0.5, -0.866).normalize();
    let ap = PlacedPortal::fixed(BLUE, Vec2::ZERO, normal, 46.0).aperture();
    let up_the_slope = ap.frame.tangent();
    assert!(up_the_slope.x > 0.0, "premise: the tangent runs up the slope to the right");
    // 27.5 along (and the squeeze) and 10 deep: a step of 4.5 px right took
    // where it cuts the slope past the end.
    let end = 46.0 + super::SQUEEZE;
    let centre = up_the_slope * (27.5 + super::SQUEEZE) - normal * 10.0;
    let half = Vec2::new(12.0, 20.0);
    let body = ae::Aabb::new(centre, half);
    let came_from = ae::Aabb::new(centre - Vec2::new(4.5, 0.0), half);
    let (t0, t1) = super::cut(&super::corners(came_from, &ap)).expect("premise: it straddles");
    assert!(t0 > -end && t1 < end, "premise: before the step it cut the plane within the opening: {t0}..{t1}");
    let (_, t1) = super::cut(&super::corners(body, &ap)).expect("it straddles");
    assert!(t1 > end, "premise: after the step it cuts the plane past the end: ..{t1}");
    let (push, against) = snag_push(body, Some(came_from), &ap).expect("the end holds it");
    assert!((push.length() - (t1 - end)).abs() < 1e-3, "back to the end: {push:?}");
    assert!((against + up_the_slope).length() < 1e-3, "held back down the slope: {against:?}");
    assert!(push.dot(normal).abs() < 1e-3, "not lifted across the face: {push:?}");
}

/// Its centre is past the plane, and it still cuts the plane past an end:
/// the pair does not take it until it fits.
#[test]
fn a_body_that_cuts_the_plane_past_an_end_is_not_taken() {
    use crate::placement::{transit_step_with_tuning, TransitStep};
    let portals = [floor(), exit()];
    let straddling = Some(PortalTransit { straddling: BLUE, crossed: false });
    let step = |x: f32| {
        transit_step_with_tuning(
            Vec2::new(x, 302.0),
            BODY * 2.0,
            Vec2::new(0.0, 100.0),
            None,
            straddling,
            None,
            &portals,
            Vec2::new(0.0, 1.0),
            &crate::types::PortalHostDepths::default(),
            &crate::PortalTuning::default(),
        )
    };
    assert!(matches!(step(0.0), TransitStep::Transfer { .. }), "control: within the opening it is taken");
    assert!(matches!(step(28.0), TransitStep::Continue), "past the end it was taken: {:?}", step(28.0));
}

/// A body that comes up through a slope from behind, as through a one-way
/// platform (Jon 2026-10-16), is on the back of the pair: the pair does not
/// begin its crossing, and once it is wholly past the plane it is not.
#[test]
fn a_body_that_comes_at_an_opening_from_behind_is_on_its_back() {
    let mut app = App::new();
    app.init_resource::<crate::PortalTuning>();
    app.add_message::<crate::PortalBodyEntered>();
    app.add_message::<crate::PortalBodyTransited>();
    app.add_systems(Update, crate::portal_transit);
    app.world_mut().spawn(floor());
    app.world_mut().spawn(exit());
    // Rising from under the floor portal (behind it), through it.
    let kin = |y: f32| BodyKinematics { pos: Vec2::new(0.0, y), vel: Vec2::new(0.0, -200.0), size: BODY * 2.0, facing: 1.0 };
    let body = app.world_mut().spawn(kin(312.0)).id();
    app.update();
    assert_eq!(
        app.world().get::<crate::PortalBackside>(body).copied(),
        Some(crate::PortalBackside { channel: BLUE }),
        "its centre behind the plane, cutting it: on the back"
    );
    // Its centre now in front, its feet still behind: no crossing begins.
    *app.world_mut().get_mut::<BodyKinematics>(body).unwrap() = kin(292.0);
    app.update();
    assert!(app.world().get::<PortalTransit>(body).is_none(), "a crossing began from behind");
    assert!(app.world().get::<crate::PortalBackside>(body).is_some(), "still on the back while it cuts the plane");
    // Wholly past the plane: an ordinary body again.
    *app.world_mut().get_mut::<BodyKinematics>(body).unwrap() = kin(270.0);
    app.update();
    assert!(app.world().get::<crate::PortalBackside>(body).is_none(), "past the plane, it is on no back");
}
