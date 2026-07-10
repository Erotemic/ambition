//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use ambition_engine_core::Block;

#[test]
fn aabb_bodies_never_collide_with_surface_chains() {
    // The R8.4 protection net, direction two: chains are collision
    // geometry ONLY for surface-momentum bodies. An axis-swept kinematic
    // body falls straight through a chain-only world — the AABB path
    // executes zero chain code by construction.
    let mut world = world_with(Vec::new());
    world.chains.push(ambition_engine_core::SurfaceChain::open(
        "chain floor",
        vec![Vec2::new(0.0, 400.0), Vec2::new(800.0, 400.0)],
    ));
    let mut b = body(Vec2::new(400.0, 300.0));
    for _ in 0..120 {
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
    }
    assert!(!b.on_ground, "no support from a chain");
    assert!(b.pos.y > 500.0, "fell straight through: {:?}", b.pos);
}

#[test]
fn observed_contacts_report_landing_normals_for_all_cardinal_gravities() {
    // C4-style: a body dropped onto a support under EACH cardinal gravity
    // reports a feet contact whose normal is -gravity_dir, with a tangent
    // perpendicular to it. Resolution is unchanged (the plain step is the
    // observed step with a None sink) — this pins the observability.
    for g in [
        Vec2::new(0.0, 1.0),
        Vec2::new(0.0, -1.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(-1.0, 0.0),
    ] {
        let center = Vec2::new(400.0, 300.0);
        let block_center = center + g * 100.0;
        let world = world_with(vec![Block::solid(
            "support",
            block_center - Vec2::splat(60.0),
            Vec2::splat(120.0),
        )]);
        let mut b = body(center);
        let tuning = KinematicTuning {
            gravity: 1450.0,
            max_fall_speed: 760.0,
            gravity_dir: g,
        };
        let mut contacts = Vec::new();
        for _ in 0..240 {
            contacts.clear();
            step_kinematic_observed(
                &mut b,
                &world,
                tuning,
                KinematicInputs::default(),
                1.0 / 60.0,
                Some(&mut contacts),
            );
            if b.on_ground {
                break;
            }
        }
        assert!(b.on_ground, "gravity {g:?}: body landed");
        let feet = contacts
            .iter()
            .find(|c| (c.normal + g).length() < 1e-3)
            .unwrap_or_else(|| {
                panic!("gravity {g:?}: a feet contact with normal == -g, got {contacts:?}")
            });
        assert_eq!(feet.surface_velocity, Vec2::ZERO);
        assert!(feet.tangent().dot(feet.normal).abs() < 1e-6);
    }
}

#[test]
fn observed_rest_contact_carries_moving_platform_velocity() {
    let mut platform = Block::solid("mover", Vec2::new(340.0, 340.0), Vec2::new(120.0, 20.0));
    platform.velocity = Vec2::new(2.0, 0.0);
    let world = world_with(vec![platform]);
    // Feet (pos.y + 23) exactly on the platform top (340).
    let mut b = body(Vec2::new(400.0, 317.0));
    let mut contacts = Vec::new();
    step_kinematic_observed(
        &mut b,
        &world,
        tuning(),
        KinematicInputs::default(),
        1.0 / 60.0,
        Some(&mut contacts),
    );
    assert!(b.on_ground);
    let rest = contacts
        .iter()
        .find(|c| c.surface_velocity == Vec2::new(2.0, 0.0))
        .expect("the rest contact carries the platform's frame velocity");
    assert!((rest.normal - Vec2::new(0.0, -1.0)).length() < 1e-3);
}

fn world_with(blocks: Vec<Block>) -> World {
    World {
        name: "kinematic-test".into(),
        size: Vec2::new(800.0, 600.0),
        spawn: Vec2::new(0.0, 0.0),
        blocks,
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
        chains: Vec::new(),
    }
}

fn body(pos: Vec2) -> KinematicBody {
    KinematicBody::new(pos, Vec2::new(28.0, 46.0))
}

fn tuning() -> KinematicTuning {
    KinematicTuning {
        gravity: 1450.0,
        max_fall_speed: 760.0,
        gravity_dir: Vec2::new(0.0, 1.0),
    }
}

/// Regression for the mockingbird "flies above the arena" OOB
/// (2026-06-21, caught by the actor OOB trace): a gravity-free body
/// deeply overlapping a solid whose nearest in-axis exit face is far
/// away must NOT be pushout-teleported across / out of the world by a
/// single resolution step. Depenetration stays bounded (≤ the body's
/// half-extent); the body's own velocity carries it out at the near
/// face over subsequent frames. The bug snapped the boss's feet to a
/// block face at the world's top edge, flinging it to y = -half in one
/// tick.
#[test]
fn deeply_embedded_body_is_not_pushout_teleported() {
    let world = World {
        name: "embed".into(),
        size: Vec2::new(800.0, 600.0),
        spawn: Vec2::ZERO,
        blocks: vec![Block::solid(
            "slab",
            Vec2::new(200.0, 100.0),
            Vec2::new(400.0, 400.0),
        )],
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
        chains: Vec::new(),
    };
    let start = Vec2::new(400.0, 300.0);
    let mut body = KinematicBody::new(start, Vec2::new(100.0, 100.0));
    let tuning = KinematicTuning {
        gravity: 0.0,
        max_fall_speed: 0.0,
        gravity_dir: Vec2::new(0.0, 1.0),
    };
    step_kinematic(
        &mut body,
        &world,
        tuning,
        KinematicInputs::default(),
        1.0 / 60.0,
    );

    let moved = (body.pos - start).length();
    let cap = body.aabb().half_size().length();
    assert!(
        moved <= cap + 1.0,
        "embedded body was pushout-teleported {moved:.1}px (cap {cap:.1}px); \
         a single resolution step must stay bounded"
    );
    assert!(
        body.pos.y > 0.0 && body.pos.y < world.size.y,
        "body must stay inside the world; got y={}",
        body.pos.y
    );
}

#[derive(Clone, Copy, Debug)]
struct ConformanceArm {
    name: &'static str,
    dir: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct KinematicSample {
    pos: Vec2,
    vel: Vec2,
    on_ground: bool,
}

const CONFORMANCE_CENTER: Vec2 = Vec2::new(400.0, 300.0);
const CONFORMANCE_ARMS: [ConformanceArm; 4] = [
    ConformanceArm {
        name: "down",
        dir: Vec2::new(0.0, 1.0),
    },
    ConformanceArm {
        name: "right",
        dir: Vec2::new(1.0, 0.0),
    },
    ConformanceArm {
        name: "up",
        dir: Vec2::new(0.0, -1.0),
    },
    ConformanceArm {
        name: "left",
        dir: Vec2::new(-1.0, 0.0),
    },
];

fn conf_frame(dir: Vec2) -> ambition_engine_core::AccelerationFrame {
    ambition_engine_core::AccelerationFrame::new(dir)
}

fn conf_world_from_local(dir: Vec2, local: Vec2) -> Vec2 {
    let f = conf_frame(dir);
    CONFORMANCE_CENTER + f.side * local.x + f.down * local.y
}

fn conf_local_vec(dir: Vec2, world: Vec2) -> Vec2 {
    let f = conf_frame(dir);
    Vec2::new(world.dot(f.side), world.dot(f.down))
}

fn conf_local_pos(dir: Vec2, world: Vec2) -> Vec2 {
    conf_local_vec(dir, world - CONFORMANCE_CENTER)
}

fn conf_tuning(dir: Vec2) -> KinematicTuning {
    KinematicTuning {
        gravity: 1450.0,
        max_fall_speed: 760.0,
        gravity_dir: dir,
    }
}

fn conf_block_solid(name: &'static str, dir: Vec2, local_min: Vec2, local_size: Vec2) -> Block {
    let f = conf_frame(dir);
    let world_center = conf_world_from_local(dir, local_min + local_size * 0.5);
    let world_half = f.to_world_half(local_size * 0.5);
    Block::solid(name, world_center - world_half, world_half * 2.0)
}

fn conf_block_one_way(name: &'static str, dir: Vec2, local_min: Vec2, local_size: Vec2) -> Block {
    let f = conf_frame(dir);
    let world_center = conf_world_from_local(dir, local_min + local_size * 0.5);
    let world_half = f.to_world_half(local_size * 0.5);
    Block::one_way(name, world_center - world_half, world_half * 2.0)
}

fn square_body_at_local(dir: Vec2, local_pos: Vec2) -> KinematicBody {
    KinematicBody::new(conf_world_from_local(dir, local_pos), Vec2::splat(32.0))
}

fn conf_sample(dir: Vec2, body: &KinematicBody) -> KinematicSample {
    KinematicSample {
        pos: conf_local_pos(dir, body.pos),
        vel: conf_local_vec(dir, body.vel),
        on_ground: body.on_ground,
    }
}

fn assert_sample_close(label: &str, expected: KinematicSample, actual: KinematicSample) {
    let pos_diff = actual.pos - expected.pos;
    let vel_diff = actual.vel - expected.vel;
    assert!(
        pos_diff.x.abs() <= 2.0 && pos_diff.y.abs() <= 2.0,
        "{label} local pos: got {:?}, expected {:?}, diff {:?}",
        actual.pos,
        expected.pos,
        pos_diff
    );
    assert!(
        vel_diff.x.abs() <= 4.0 && vel_diff.y.abs() <= 4.0,
        "{label} local vel: got {:?}, expected {:?}, diff {:?}",
        actual.vel,
        expected.vel,
        vel_diff
    );
    assert_eq!(
        actual.on_ground, expected.on_ground,
        "{label} on_ground should be gravity-relative"
    );
}

fn assert_trace_c4_symmetric(
    name: &str,
    make_world: impl Fn(Vec2) -> World,
    make_body: impl Fn(Vec2) -> KinematicBody,
    drive: impl Fn(&mut KinematicBody, &World, KinematicTuning, usize),
    ticks: usize,
) {
    let mut reference = Vec::new();
    for (idx, arm) in CONFORMANCE_ARMS.iter().enumerate() {
        let world = make_world(arm.dir);
        let mut body = make_body(arm.dir);
        let tuning = conf_tuning(arm.dir);
        let mut trace = Vec::new();
        for tick in 0..ticks {
            drive(&mut body, &world, tuning, tick);
            trace.push(conf_sample(arm.dir, &body));
        }
        if idx == 0 {
            reference = trace;
        } else {
            for (tick, (expected, actual)) in reference.iter().zip(trace.iter()).enumerate() {
                assert_sample_close(
                    &format!("{name} / {} arm tick {tick}", arm.name),
                    *expected,
                    *actual,
                );
            }
        }
    }
}

#[test]
fn frame_conformance_solid_support_is_c4_symmetric() {
    // A body falling onto a solid support should produce the same local trace
    // no matter which world axis gravity currently occupies.
    assert_trace_c4_symmetric(
        "solid support",
        |dir| {
            world_with(vec![conf_block_solid(
                "support",
                dir,
                Vec2::new(-160.0, 160.0),
                Vec2::new(320.0, 32.0),
            )])
        },
        |dir| square_body_at_local(dir, Vec2::new(0.0, 40.0)),
        |body, world, tuning, _tick| {
            step_kinematic(body, world, tuning, KinematicInputs::default(), 1.0 / 60.0)
        },
        48,
    );
}

#[test]
fn frame_conformance_side_wall_is_not_ground() {
    // Moving into the local side face is a wall in every frame, not support.
    assert_trace_c4_symmetric(
        "side wall",
        |dir| {
            world_with(vec![conf_block_solid(
                "wall",
                dir,
                Vec2::new(120.0, -80.0),
                Vec2::new(32.0, 240.0),
            )])
        },
        |dir| {
            let mut b = square_body_at_local(dir, Vec2::new(0.0, 0.0));
            b.vel = conf_frame(dir).to_world(Vec2::new(260.0, 0.0));
            b
        },
        |body, world, tuning, _tick| {
            step_kinematic(
                body,
                world,
                KinematicTuning {
                    gravity: 0.0,
                    ..tuning
                },
                KinematicInputs::default(),
                1.0 / 60.0,
            )
        },
        32,
    );
}

#[test]
fn frame_conformance_one_way_drop_through_is_c4_symmetric() {
    assert_trace_c4_symmetric(
        "one-way drop-through",
        |dir| {
            world_with(vec![
                conf_block_one_way(
                    "one_way",
                    dir,
                    Vec2::new(-120.0, 120.0),
                    Vec2::new(240.0, 14.0),
                ),
                conf_block_solid(
                    "catcher",
                    dir,
                    Vec2::new(-160.0, 300.0),
                    Vec2::new(320.0, 32.0),
                ),
            ])
        },
        |dir| square_body_at_local(dir, Vec2::new(0.0, 40.0)),
        |body, world, tuning, tick| {
            let inputs = if tick >= 30 {
                KinematicInputs { drop_through: true }
            } else {
                KinematicInputs::default()
            };
            step_kinematic(body, world, tuning, inputs, 1.0 / 60.0)
        },
        80,
    );
}

#[test]
fn frame_conformance_moving_support_carries_along_local_side() {
    assert_trace_c4_symmetric(
        "moving support",
        |dir| {
            let mut support = conf_block_solid(
                "moving_support",
                dir,
                Vec2::new(-160.0, 120.0),
                Vec2::new(320.0, 32.0),
            );
            support.velocity = conf_frame(dir).to_world(Vec2::new(3.0, 0.0));
            world_with(vec![support])
        },
        |dir| square_body_at_local(dir, Vec2::new(0.0, 40.0)),
        |body, world, tuning, _tick| {
            step_kinematic(body, world, tuning, KinematicInputs::default(), 1.0 / 60.0)
        },
        54,
    );
}

#[test]
fn flipped_gravity_makes_a_body_rise_and_land_on_a_ceiling() {
    // A ceiling block above the body; flipped gravity pulls the body UP onto
    // it and it registers as grounded (standing on the ceiling).
    let world = world_with(vec![Block::solid(
        "ceiling",
        Vec2::new(0.0, 0.0),
        Vec2::new(200.0, 32.0),
    )]);
    let mut b = body(Vec2::new(50.0, 300.0));
    let mut tuning = tuning();
    tuning.gravity_dir = Vec2::new(0.0, -1.0); // up
    for _ in 0..120 {
        step_kinematic(
            &mut b,
            &world,
            tuning,
            KinematicInputs::default(),
            1.0 / 60.0,
        );
    }
    assert!(
        b.pos.y < 300.0,
        "flipped gravity should pull the body up, got y={}",
        b.pos.y
    );
    assert!(
        b.on_ground,
        "the body should stand on the ceiling under flipped gravity"
    );
}

#[test]
fn sideways_gravity_makes_a_body_fall_into_and_land_on_a_wall() {
    // A wall on the RIGHT; right-pointing gravity pulls the body into it and
    // it registers as grounded (standing on the wall). This is the enemy/NPC
    // bug — under the old Y-only `gravity_sign` a sideways-gravity body never
    // fell toward the wall at all.
    let world = world_with(vec![Block::solid(
        "right_wall",
        Vec2::new(400.0, -400.0),
        Vec2::new(40.0, 1200.0),
    )]);
    let mut b = body(Vec2::new(100.0, 50.0));
    let mut tuning = tuning();
    tuning.gravity_dir = Vec2::new(1.0, 0.0); // right
    let start_x = b.pos.x;
    for _ in 0..180 {
        step_kinematic(
            &mut b,
            &world,
            tuning,
            KinematicInputs::default(),
            1.0 / 60.0,
        );
    }
    assert!(
        b.pos.x > start_x + 100.0,
        "right gravity should pull the body toward the wall, got x={} (start {start_x})",
        b.pos.x
    );
    assert!(
        b.on_ground,
        "the body should land on (be grounded against) the wall it fell into",
    );
    assert!(
        b.pos.x <= 400.0,
        "the body should stop at the wall's left face, got x={}",
        b.pos.x
    );
}

#[test]
fn gravity_caps_a_normal_fall_at_terminal_velocity() {
    // No floor: a body falling under gravity should accelerate UP TO the
    // terminal velocity and sit there (the equilibrium), never exceeding it.
    let world = world_with(vec![]);
    let mut b = body(Vec2::new(50.0, 0.0));
    for _ in 0..600 {
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
    }
    assert!(
        (b.vel.y - tuning().max_fall_speed).abs() < 1.0,
        "a normal fall should settle at terminal velocity {}, got {}",
        tuning().max_fall_speed,
        b.vel.y
    );
}

#[test]
fn a_fling_above_terminal_is_preserved_not_braked() {
    // A body already moving faster than terminal (a portal fling) must NOT be
    // decelerated by the fall cap — gravity is an equilibrium it accelerates
    // toward, not a brake. The over-cap speed persists (no air drag on the
    // fall axis), so momentum carries through.
    let world = world_with(vec![]);
    let mut b = body(Vec2::new(50.0, 0.0));
    let fling = tuning().max_fall_speed * 2.0;
    b.vel.y = fling;
    step_kinematic(
        &mut b,
        &world,
        tuning(),
        KinematicInputs::default(),
        1.0 / 60.0,
    );
    assert!(
        b.vel.y >= fling,
        "an over-terminal fling ({fling}) should be preserved, got {}",
        b.vel.y
    );
}

#[test]
fn lands_on_solid() {
    // Body falls and stops on a Solid floor.
    let world = world_with(vec![Block::solid(
        "floor",
        Vec2::new(0.0, 100.0),
        Vec2::new(200.0, 32.0),
    )]);
    let mut b = body(Vec2::new(50.0, 0.0));
    for _ in 0..30 {
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
    }
    assert!(b.on_ground, "expected to land on solid floor");
    assert!(b.vel.y.abs() < 0.01, "vel.y reset on landing");
}

#[test]
fn lands_on_one_way_from_above() {
    // OneWay platform behaves like a floor when the body is
    // descending from above.
    let world = world_with(vec![Block::one_way(
        "platform",
        Vec2::new(0.0, 100.0),
        Vec2::new(200.0, 16.0),
    )]);
    let mut b = body(Vec2::new(50.0, 0.0));
    for _ in 0..30 {
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
    }
    assert!(
        b.on_ground,
        "expected to land on one-way platform from above"
    );
}

#[test]
fn drop_through_passes_one_way() {
    // Same scene, but drop_through=true → no landing.
    let world = world_with(vec![Block::one_way(
        "platform",
        Vec2::new(0.0, 100.0),
        Vec2::new(200.0, 16.0),
    )]);
    let mut b = body(Vec2::new(50.0, 50.0));
    // First, settle on the platform.
    for _ in 0..20 {
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
    }
    assert!(b.on_ground, "precondition: must be on the platform");
    // Now drop through. Past the platform's bottom (y=116 in
    // top-left coords) is the success condition; y=160ish after
    // 20 frames of free-fall is well clear.
    let drop = KinematicInputs { drop_through: true };
    for _ in 0..20 {
        step_kinematic(&mut b, &world, tuning(), drop, 1.0 / 60.0);
    }
    assert!(
        b.pos.y - b.size.y * 0.5 > 116.0,
        "drop_through should clear the platform's bottom edge; body top y={}",
        b.pos.y - b.size.y * 0.5
    );
    assert!(!b.on_ground, "should not be grounded mid-fall");
}

#[test]
fn drop_through_does_not_pass_solid() {
    // Drop-through is a OneWay-only affordance — Solid still blocks.
    let world = world_with(vec![Block::solid(
        "floor",
        Vec2::new(0.0, 100.0),
        Vec2::new(200.0, 32.0),
    )]);
    let mut b = body(Vec2::new(50.0, 50.0));
    let drop = KinematicInputs { drop_through: true };
    for _ in 0..40 {
        step_kinematic(&mut b, &world, tuning(), drop, 1.0 / 60.0);
    }
    assert!(b.on_ground, "Solid must still catch the body");
}

#[test]
fn walks_off_ledge_falls() {
    // Solid ledge that ends at x=100. Body starts on the ledge,
    // walks right past the edge — should fall once it's no longer
    // overlapping the ledge horizontally.
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(0.0, 100.0),
        Vec2::new(100.0, 32.0),
    )]);
    let mut b = body(Vec2::new(60.0, 50.0));
    // Settle on the ledge.
    for _ in 0..20 {
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
    }
    assert!(b.on_ground, "precondition: on ledge");
    // Walk right past the edge.
    b.vel.x = 200.0;
    for _ in 0..30 {
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
    }
    assert!(
        b.pos.x > 110.0,
        "must clear the ledge horizontally; x={}",
        b.pos.x
    );
    assert!(!b.on_ground, "should be airborne after clearing the edge");
    assert!(b.vel.y > 0.0, "should be falling");
}

#[test]
fn a_body_rides_a_horizontally_moving_platform() {
    // A solid floor carrying a rightward per-frame velocity. ANY body resting on
    // it is carried right by that velocity — emergent riding, no per-actor flag.
    let mut platform = Block::solid("platform", Vec2::new(0.0, 100.0), Vec2::new(400.0, 32.0));
    platform.velocity = Vec2::new(3.0, 0.0); // 3 px/frame to the right
    let world = world_with(vec![platform]);
    let mut b = body(Vec2::new(50.0, 50.0));
    for _ in 0..30 {
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
    }
    assert!(b.on_ground, "precondition: resting on the platform");
    let x_before = b.pos.x;
    step_kinematic(
        &mut b,
        &world,
        tuning(),
        KinematicInputs::default(),
        1.0 / 60.0,
    );
    assert!(
        (b.pos.x - (x_before + 3.0)).abs() < 1e-3,
        "body should ride +3px right with the platform, got dx={}",
        b.pos.x - x_before
    );
}

#[test]
fn a_body_does_not_ride_static_geometry() {
    // velocity ZERO (the static default) → no carry. Standing on normal ground is
    // byte-identical to before riding existed.
    let world = world_with(vec![Block::solid(
        "floor",
        Vec2::new(0.0, 100.0),
        Vec2::new(200.0, 32.0),
    )]);
    let mut b = body(Vec2::new(50.0, 50.0));
    for _ in 0..30 {
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
    }
    let x_before = b.pos.x;
    step_kinematic(
        &mut b,
        &world,
        tuning(),
        KinematicInputs::default(),
        1.0 / 60.0,
    );
    assert!(
        (b.pos.x - x_before).abs() < 1e-3,
        "a body must NOT drift on static ground, got dx={}",
        b.pos.x - x_before
    );
}

#[test]
fn rising_through_one_way_does_not_get_stuck() {
    // OneWay should never block upward motion. Body starts below
    // the platform with negative vel.y (jumping up).
    let world = world_with(vec![Block::one_way(
        "platform",
        Vec2::new(0.0, 50.0),
        Vec2::new(200.0, 16.0),
    )]);
    let mut b = body(Vec2::new(50.0, 200.0));
    b.vel.y = -800.0;
    // Step a few frames; gravity will reduce vel.y but the body
    // should not be pinned by the one-way platform on the way up.
    let mut min_y = b.pos.y;
    for _ in 0..15 {
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
        if b.pos.y < min_y {
            min_y = b.pos.y;
        }
    }
    assert!(
        min_y < 60.0,
        "rising body should pass through OneWay; min_y={}",
        min_y
    );
}
