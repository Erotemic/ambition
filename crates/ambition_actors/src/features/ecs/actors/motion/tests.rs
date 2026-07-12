//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

const DT: f32 = 1.0 / 60.0;
const G: ae::Vec2 = ae::Vec2::new(0.0, 1450.0);

fn chain_world() -> ae::World {
    ae::World::new(
        "momentum-test",
        ae::Vec2::new(4000.0, 2000.0),
        ae::Vec2::ZERO,
        Vec::new(),
    )
    .with_chains(vec![ae::SurfaceChain::open(
        "ramp",
        vec![
            ae::Vec2::new(0.0, 600.0),
            ae::Vec2::new(1200.0, 600.0),
            ae::Vec2::new(1800.0, 300.0),
        ],
    )])
}

fn kin_at(pos: ae::Vec2) -> ae::BodyKinematics {
    ae::BodyKinematics {
        pos,
        vel: ae::Vec2::ZERO,
        size: ae::Vec2::new(28.0, 28.0),
        facing: 1.0,
    }
}

#[test]
fn momentum_body_falls_lands_and_runs_up_the_ramp() {
    let world = chain_world();
    let mut kin = kin_at(ae::Vec2::new(200.0, 400.0));
    let mut m = MomentumMotion::new(ae::surface::MomentumParams::default());
    let mut on_ground = false;
    let mut normal = ae::Vec2::new(0.0, -1.0);
    // Run right until the body is riding partway UP the ramp segment
    // (sample there — kept running, it would launch off the open end,
    // which is the solver's correct behavior, not this test's subject).
    let mut on_ramp = None;
    for _ in 0..600 {
        step_momentum_body(
            &mut kin,
            &mut on_ground,
            &mut normal,
            &mut m,
            &world,
            G,
            1.0,
            ae::Vec2::X,
            false,
            1.0,
            DT,
        );
        if on_ground && kin.pos.x > 1300.0 && kin.pos.x < 1600.0 {
            on_ramp = Some((kin.pos, normal));
            break;
        }
    }
    let (pos, ramp_normal) = on_ramp.expect("landed, ran the flat, and climbed the ramp");
    assert!(pos.y < 590.0, "climbing the ramp: {pos:?}");
    // The body frame follows the slope: on the ramp the normal tilts.
    assert!(
        ramp_normal.x.abs() > 0.1 && ramp_normal.y < -0.5,
        "surface_normal follows the ridden slope: {ramp_normal:?}"
    );
}

#[test]
fn jump_intent_launches_and_facing_writes_back() {
    let world = chain_world();
    let mut kin = kin_at(ae::Vec2::new(200.0, 400.0));
    let mut m = MomentumMotion::new(ae::surface::MomentumParams::default());
    let mut on_ground = false;
    let mut normal = ae::Vec2::new(0.0, -1.0);
    // Settle onto the chain.
    for _ in 0..120 {
        step_momentum_body(
            &mut kin,
            &mut on_ground,
            &mut normal,
            &mut m,
            &world,
            G,
            0.0,
            ae::Vec2::ZERO,
            false,
            0.0,
            DT,
        );
    }
    assert!(on_ground);
    // Jump (any controller pressing jump looks identical here).
    step_momentum_body(
        &mut kin,
        &mut on_ground,
        &mut normal,
        &mut m,
        &world,
        G,
        0.0,
        ae::Vec2::ZERO,
        true,
        -1.0,
        DT,
    );
    assert!(!on_ground, "left the surface");
    assert!(kin.vel.y < -400.0, "launched along +normal: {:?}", kin.vel);
    assert_eq!(kin.facing, -1.0, "explicit facing intent applied");
    assert_eq!(normal, ae::Vec2::new(0.0, -1.0), "airborne frame = gravity");
}

#[test]
fn momentum_wrapper_forwards_two_dimensional_route_intent() {
    let chain = ae::SurfaceChain::open(
        "route-switch",
        vec![
            ae::Vec2::new(-100.0, 50.0),
            ae::Vec2::ZERO,
            ae::Vec2::new(100.0, -50.0),
            ae::Vec2::new(-100.0, -50.0),
            ae::Vec2::ZERO,
            ae::Vec2::new(100.0, 0.0),
        ],
    )
    .with_junctions(vec![ae::SurfaceJunction::new(vec![1, 4])]);
    let entry_s = chain.arc_at_vertex(1);
    let runout_s = chain.arc_at_vertex(4);
    let world = ae::World::new(
        "route-test",
        ae::Vec2::new(1000.0, 1000.0),
        ae::Vec2::ZERO,
        Vec::new(),
    )
    .with_chains(vec![chain]);
    let frame = world.chains[0].frame_at(entry_s - 1.0);
    let mut kin = kin_at(frame.point + frame.normal * 14.0);
    kin.vel = frame.tangent * 240.0;
    let mut motion = MomentumMotion::new(ae::surface::MomentumParams {
        ground_accel: 0.0,
        brake: 0.0,
        friction: 0.0,
        slope_factor: 0.0,
        top_speed: 1000.0,
        air_accel: 0.0,
        stick_factor: 1000.0,
        min_stick_speed: 0.0,
        ..Default::default()
    });
    motion.state = ae::surface::SurfaceMotion::Riding {
        on: ae::surface::SurfaceRef::Chain(0),
        s: entry_s - 1.0,
        v_t: 240.0,
    };
    let mut on_ground = true;
    let mut normal = frame.normal;

    step_momentum_body(
        &mut kin,
        &mut on_ground,
        &mut normal,
        &mut motion,
        &world,
        G,
        1.0,
        ae::Vec2::new(1.0, 1.0),
        false,
        1.0,
        DT,
    );

    let ae::surface::SurfaceMotion::Riding { s, v_t, .. } = motion.state else {
        panic!("an authored route switch should guide rather than launch");
    };
    assert!(
        s > runout_s,
        "the actor wrapper must forward Down+Right so the lower route wins"
    );
    assert!(v_t > 0.0, "route choice preserves forward momentum");
}
