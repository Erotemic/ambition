//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod surface_ramp_winding_oracle` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

use super::{surface_ramp_points, RampOrientation};
use ambition_engine_core as ae;

const R: f32 = 200.0;
const SEGMENTS: usize = 8;
const CORNER: ae::Vec2 = ae::Vec2::new(600.0, 400.0);
const DT: f32 = 1.0 / 60.0;
/// Fast enough to be momentum, slow enough that a 200px fillet can supply the
/// centripetal demand (`v²·angle/r_body` vs `stick_factor · press`). A body
/// that launches off the ramp is a level-design fact, not a winding bug.
const SPEED: f32 = 300.0;

fn corner_for(o: RampOrientation) -> ae::Vec2 {
    let x = match o {
        RampOrientation::FloorToRightWall | RampOrientation::CeilingToRightWall => CORNER.x,
        _ => CORNER.x - 2.0 * R,
    };
    let y = match o {
        RampOrientation::FloorToRightWall | RampOrientation::FloorToLeftWall => CORNER.y,
        _ => CORNER.y - 2.0 * R,
    };
    ae::Vec2::new(x, y)
}

/// A fillet plus the straight flat that leads into it and the straight wall it
/// leaves along — one chain, in the order `surface_ramp_points` chose.
fn ramp_chain(o: RampOrientation) -> ae::SurfaceChain {
    let corner = corner_for(o);
    let arc = surface_ramp_points(corner, R, o, SEGMENTS);
    let room = o.into_room();

    // Long lead-ins on purpose: the joint at the fillet's mouth then sits at an
    // arc length where a fixed-epsilon joint nudge used to round away (see
    // `ambition_engine_core::movement::surface_momentum::joint_nudge`). The oracle should ride a
    // realistic chain, not a convenient one.
    let flat_far = ae::Vec2::new(corner.x + room.x * 8.0 * R, corner.y);
    let wall_far = ae::Vec2::new(corner.x, corner.y + room.y * 8.0 * R);

    let first_is_flat = (arc[0].y - corner.y).abs() < 1.0;
    let mut points = Vec::with_capacity(arc.len() + 2);
    if first_is_flat {
        points.push(flat_far);
        points.extend(arc.iter().copied());
        points.push(wall_far);
    } else {
        points.push(wall_far);
        points.extend(arc.iter().copied());
        points.push(flat_far);
    }
    ae::SurfaceChain::open(format!("{o:?}"), points)
}

/// A ramp's local "down" is the opposite of the flat surface's outward normal:
/// `+y` under a floor, `−y` above a ceiling. This is the C4 gravity conjugation
/// the doc asks the mirrored cases to be ridden under.
fn gravity_for(o: RampOrientation) -> ae::Vec2 {
    ae::Vec2::new(0.0, -o.into_room().y) * 1450.0
}

/// Ride the chain from the flat end into the corner; report the velocity the
/// moment the body clears the fillet onto the wall.
fn ride_into_the_corner(o: RampOrientation) -> ae::Vec2 {
    let chain = ramp_chain(o);
    let corner = corner_for(o);
    let room = o.into_room();
    let world = ae::World::new(
        "ramp",
        ae::Vec2::new(6000.0, 6000.0),
        ae::Vec2::ZERO,
        Vec::new(),
    )
    .with_chains(vec![chain.clone()]);
    // Frictionless: this test is about winding, not about whether the body has
    // enough speed left after 1600px of floor.
    let params = ae::MomentumParams {
        friction: 0.0,
        ..Default::default()
    };
    let total = chain.total_length();

    // Start near the FLAT end, moving toward the arc. Which end of the chain
    // that is depends on the winding the converter derived, so ask the chain.
    let flat_at_zero = (chain.points[0].y - corner.y).abs() < 1.0;
    let (s, v_t) = if flat_at_zero {
        (30.0, SPEED)
    } else {
        (total - 30.0, -SPEED)
    };

    let f = chain.frame_at(s);
    let radius = 14.0;
    let mut body = ae::movement::surface_momentum::SurfaceBody {
        pos: f.point + f.normal * radius,
        vel: ae::Vec2::ZERO,
        radius,
        depth_lane: 0,
        motion: ae::movement::surface_momentum::SurfaceMotion::Riding {
            on: ae::movement::surface_momentum::SurfaceRef::Chain(0),
            s,
            v_t,
        },
    };
    let gravity = gravity_for(o);
    // `run` is along the CHAIN's tangent (increasing arc length), not along
    // world `+x`. Which world direction that is depends on the winding the
    // converter derived — so hold the stick in the direction we are already
    // travelling, and let the chain say what that means. Passing `-room.x` here
    // braked the ceiling cases into a stop, which is a fact about `run`, not
    // about the ramp.
    let run = v_t.signum();
    // Sample the moment the body clears the fillet onto the wall — NOT seconds
    // later. A body that climbed the wall correctly decelerates under gravity
    // and comes back down, and "it is falling" is not a winding bug.
    for _ in 0..2000 {
        ae::movement::surface_momentum::step_surface_body(
            &mut body,
            &world,
            &params,
            ae::MotionFrame::from_acceleration(gravity).expect("non-zero acceleration"),
            ae::movement::surface_momentum::SurfaceInputs {
                local_axis: ae::Vec2::new(run, 0.0),
                jump_pressed: false,
            },
            DT,
            None,
        );
        // Past the fillet's far tangent point, measured along the wall axis.
        if (body.pos.y - corner.y) * room.y > R * 1.25 {
            break;
        }
    }
    body.vel
}

/// **The oracle.** A body that runs into the fillet at speed leaves along the
/// WALL, in the direction the room opens — up, for a floor; down, for a
/// ceiling. A winding sign error turns this into a launch or a clip, and no
/// amount of reading the arc table would show it.
#[test]
fn a_momentum_body_carries_its_speed_from_the_flat_onto_the_wall() {
    for o in [
        RampOrientation::FloorToRightWall,
        RampOrientation::FloorToLeftWall,
        RampOrientation::CeilingToRightWall,
        RampOrientation::CeilingToLeftWall,
    ] {
        let vel = ride_into_the_corner(o);
        let expected_sign = o.into_room().y; // floors exit up (−y), ceilings down (+y)
        assert!(
            vel.y * expected_sign > 0.0,
            "{o:?}: exited with vel {vel:?}; expected the wall-axis component to \
             run toward {expected_sign:+}"
        );
        assert!(
            vel.y.abs() > vel.x.abs(),
            "{o:?}: exited with vel {vel:?} — that is still along the flat, not up \
             the wall"
        );
    }
}

/// The arc itself: endpoints tangent to the two surfaces, `segments + 1` points,
/// every point on the circle of radius `r` about the fillet's center. The doc's
/// table, checked.
#[test]
fn the_arc_is_the_documented_quarter_circle() {
    for o in [
        RampOrientation::FloorToRightWall,
        RampOrientation::FloorToLeftWall,
        RampOrientation::CeilingToRightWall,
        RampOrientation::CeilingToLeftWall,
    ] {
        let corner = ae::Vec2::new(600.0, 400.0);
        let pts = surface_ramp_points(corner, R, o, SEGMENTS);
        assert_eq!(pts.len(), SEGMENTS + 1, "{o:?}");

        let center = corner + o.into_room() * R;
        for p in &pts {
            assert!(
                ((*p - center).length() - R).abs() < 0.01,
                "{o:?}: {p:?} is not on the fillet's circle"
            );
        }
        // One endpoint is tangent on the flat surface, the other on the wall.
        let on_flat = pts.iter().filter(|p| (p.y - corner.y).abs() < 0.01).count();
        let on_wall = pts.iter().filter(|p| (p.x - corner.x).abs() < 0.01).count();
        assert_eq!((on_flat, on_wall), (1, 1), "{o:?}");
    }
}

/// The winding is DERIVED, not tabulated: every orientation's first segment has
/// its outward normal pointing into the room. That is the property the
/// converter enforces, and the reason there is one code path for four cases.
#[test]
fn every_orientation_winds_so_its_normals_point_into_the_room() {
    for o in [
        RampOrientation::FloorToRightWall,
        RampOrientation::FloorToLeftWall,
        RampOrientation::CeilingToRightWall,
        RampOrientation::CeilingToLeftWall,
    ] {
        let corner = ae::Vec2::new(600.0, 400.0);
        let pts = surface_ramp_points(corner, R, o, SEGMENTS);
        let center = corner + o.into_room() * R;
        for w in pts.windows(2) {
            let t = (w[1] - w[0]).normalize();
            let normal = ae::Vec2::new(t.y, -t.x);
            let midpoint = (w[0] + w[1]) * 0.5;
            assert!(
                normal.dot(center - midpoint) > 0.0,
                "{o:?}: a segment's normal points into the solid"
            );
        }
    }
}
