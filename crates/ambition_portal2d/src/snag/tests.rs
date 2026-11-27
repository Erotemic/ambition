//! A body goes through an opening whole: the transit takes only a body that
//! fits, and a body behind an opening is on its back. (What holds a body past
//! an end is its rim contact, `material_cast/rim.rs` in the core.)

use bevy::prelude::*;

use ambition_platformer2d_core::{self as ae, BodyKinematics};

use crate::color::{PortalChannel, PortalGunColor};
use crate::transit::{PortalTransit};
use crate::types::PlacedPortal;

const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);
const ORANGE: PortalChannel = PortalChannel::Gun(PortalGunColor::ORANGE);
/// A floor portal at y = 300 (facing up), 60 wide.
const FLOOR: Vec2 = Vec2::new(0.0, 300.0);
const HALF: f32 = 30.0;
const BODY: Vec2 = Vec2::new(10.0, 20.0);

fn floor() -> PlacedPortal {
    PlacedPortal::fixed(BLUE, FLOOR, Vec2::new(0.0, -1.0), HALF).unwrap()
}

fn exit() -> PlacedPortal {
    PlacedPortal::fixed(ORANGE, Vec2::new(600.0, 300.0), Vec2::new(0.0, -1.0), HALF).unwrap()
}

/// Its centre is past the plane, and it still cuts the plane past an end:
/// the pair does not take it until it fits.
#[test]
fn a_body_that_cuts_the_plane_past_an_end_is_not_taken() {
    use crate::placement::{transit_step_with_record, TransitStep};
    let portals = [floor(), exit()];
    let straddling = Some(PortalTransit { straddling: BLUE, crossed: false });
    let crosses = |x: f32| crate::first_portal_crossing(Vec2::new(x, 298.0), Vec2::new(x, 302.0), BODY * 2.0, &portals, None)
        .unwrap().is_some();
    assert!(crosses(0.0), "control: within the opening it is taken");
    assert!(!crosses(28.0), "past the end it was taken");
    let body = ambition_platformer2d_core::BodyKinematics { pos: Vec2::new(28.0, 302.0), canonical_size: BODY * 2.0,
        vel: Vec2::new(0.0, 100.0), ..Default::default() };
    let step = transit_step_with_record(body, None, straddling, None, &portals);
    assert!(matches!(step, TransitStep::Continue), "it stays in the pair it cuts: {step:?}");
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
    app.init_resource::<ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry>();
    // Transit crosses the pairs the step admitted (M1b): publish them first.
    app.add_systems(Update, (crate::publish_portal_apertures, crate::portal_transit).chain());
    app.world_mut().spawn(floor());
    app.world_mut().spawn(exit());
    // Rising from under the floor portal (behind it), through it.
    let kin = |y: f32| BodyKinematics { material: Default::default(), pos: Vec2::new(0.0, y), vel: Vec2::new(0.0, -200.0), canonical_size: BODY * 2.0, facing: 1.0 };
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

/// P3: a turned body is tested by its material box, not the world box
/// around it. A 20 by 2 body turned 45 degrees, its long axis along the
/// mouth's normal and 4 px along the face (material spans 3..5 of a ±12
/// opening), is in line with the opening 1 px in front of it. Its world box
/// (half-span 11 in both mouth axes) says it is beside the opening.
/// An oversized body, 30 long along the face, still does not fit.
#[test]
fn a_turned_body_that_fits_the_opening_passes_and_an_oversized_one_does_not() {
    use super::{fits_through, in_line_with, Outline};
    let turn = std::f64::consts::FRAC_PI_4;
    let material = ae::frame::BodyMaterial::IDENTITY.turned(turn).unwrap();
    let long = Vec2::new(-(turn.sin() as f32), turn.cos() as f32);
    let ap = PlacedPortal::fixed(BLUE, Vec2::ZERO, long, 12.0).unwrap().aperture();
    let tangent = ap.frame.tangent();
    let body = |size: Vec2, depth: f32| BodyKinematics { material, pos: tangent * 4.0 + ap.frame.normal * depth,
        canonical_size: size, vel: Vec2::ZERO, facing: 1.0 };
    let thin = Vec2::new(2.0, 20.0);
    let near = body(thin, 1.0);
    assert!(in_line_with(Outline::of(near), &ap), "its material is in line: {:?}", Outline::of(near));
    assert!(!in_line_with(near.aabb(), &ap), "premise: its world box is not");
    let wide = body(Vec2::new(30.0, 2.0), 0.0);
    assert!(!fits_through(Outline::of(wide), &ap), "30 along a 24 opening does not fit");
    assert!(!in_line_with(Outline::of(wide), &ap));
}
