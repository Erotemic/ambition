//! Frame-law and policy-transition evidence for the unified movement kernel.
//!
//! These tests enter ONLY through [`step_motion`] / [`switch_motion_model`] —
//! the same trusted boundary production uses. They pin the ADR 0024
//! invariants: arbitrary-angle covariance, zero-force orientation, lateral
//! acceleration never rotating the basis, frame changes never resetting
//! policy-private state, and cross-policy swaps initializing only
//! destination-private state.

use super::*;
use crate::movement::adhesive_crawler::CrawlerState;
use crate::movement::surface_momentum::{SurfaceMotion, SurfaceRef};
use crate::movement::{switch_motion_model, MotionModelKind};
use crate::reference_frame::LocalAxes;
use crate::{
    AbilitySet, AccelerationFrame, AxisSweptParams, Block, BodyClusterScratch, CrawlerParams,
    MomentumParams, MotionModelSpec, SurfaceChain,
};

const DT: f32 = 1.0 / 60.0;

fn empty_world() -> World {
    World::new(
        "frame_covariance",
        Vec2::splat(10_000.0),
        Vec2::splat(500.0),
        Vec::new(),
    )
}

fn step(
    model: &mut MotionModel,
    world: &World,
    scratch: &mut BodyClusterScratch,
    frame: MotionFrame,
    input: InputState,
) -> MotionStepResult {
    let mut clusters = scratch.as_mut();
    step_motion(
        model,
        &mut clusters,
        MotionStepContext {
            world,
            input,
            frame,
            facing_intent: 0.0,
            dt: DT,
        },
    )
}

fn one_free_tick(model: &mut MotionModel, frame: MotionFrame, input: InputState) -> (Vec2, Vec2) {
    let world = empty_world();
    let start = Vec2::splat(500.0);
    let mut scratch = BodyClusterScratch::new_with_abilities(start, AbilitySet::default());
    step(model, &world, &mut scratch, frame, input);
    let clusters = scratch.as_mut();
    (clusters.kinematics.pos - start, clusters.kinematics.vel)
}

fn rotate(v: Vec2, radians: f32) -> Vec2 {
    let (sin, cos) = radians.sin_cos();
    Vec2::new(cos * v.x - sin * v.y, sin * v.x + cos * v.y)
}

#[test]
fn all_three_policies_are_covariant_under_an_arbitrary_frame_rotation() {
    let angle = 0.731_f32;
    let acceleration = Vec2::new(130.0, 900.0);
    let base = MotionFrame::from_acceleration(acceleration).expect("non-zero acceleration");
    let rotated =
        MotionFrame::from_acceleration(rotate(acceleration, angle)).expect("non-zero acceleration");
    let input = InputState {
        axes: LocalAxes::new(0.6, -0.2),
        ..InputState::default()
    };

    for mut model in [
        MotionModel::axis_swept(AxisSweptParams::default()),
        MotionModel::surface_momentum(MomentumParams::default()),
        MotionModel::adhesive_crawler(CrawlerParams::default()),
    ] {
        let mut rotated_model = model.clone();
        let (base_delta, base_vel) = one_free_tick(&mut model, base, input);
        let (rotated_delta, rotated_vel) = one_free_tick(&mut rotated_model, rotated, input);

        assert!(
            (rotate(base_delta, angle) - rotated_delta).length() < 1e-3,
            "{:?}: rotated displacement {rotated_delta:?} != rotation of {base_delta:?}",
            model.kind()
        );
        assert!(
            (rotate(base_vel, angle) - rotated_vel).length() < 1e-3,
            "{:?}: rotated velocity {rotated_vel:?} != rotation of {base_vel:?}",
            model.kind()
        );
    }
}

#[test]
fn zero_acceleration_retains_the_explicitly_supplied_orientation() {
    // The environment defines "down" toward +X while applying NO force. The
    // body must not fall, and local input must still be interpreted in the
    // supplied basis — zero force never means "return to normal gravity".
    let basis = AccelerationFrame::new(Vec2::new(1.0, 0.0));
    let frame = MotionFrame::new(basis, Vec2::ZERO);
    assert_eq!(frame.down(), Vec2::new(1.0, 0.0));
    assert_eq!(frame.acceleration(), Vec2::ZERO);

    let run_input = InputState {
        axes: LocalAxes::new(1.0, 0.0),
        ..InputState::default()
    };
    let mut model = MotionModel::axis_swept(AxisSweptParams::default());
    let (_, vel) = one_free_tick(&mut model, frame, run_input);
    // No component along the supplied down axis (no gravity), all motion on
    // the supplied side axis (which is world -Y for down = +X).
    assert!(
        vel.dot(frame.down()).abs() < 1e-4,
        "zero acceleration must not accelerate the body along down: {vel:?}"
    );
    assert!(
        vel.dot(frame.side()) > 1.0,
        "local +x input must run along the SUPPLIED side axis: {vel:?}"
    );
}

#[test]
fn lateral_acceleration_does_not_rotate_the_supplied_basis() {
    // Ordinary down basis plus a lateral inertial component: the body feels
    // the full world acceleration vector, but its side/down axes — and the
    // interpretation of controller intent — do not tilt toward the net force.
    let basis = AccelerationFrame::new(Vec2::new(0.0, 1.0));
    let lateral = MotionFrame::new(basis, Vec2::new(300.0, 900.0));
    assert_eq!(lateral.down(), Vec2::new(0.0, 1.0));
    assert_eq!(lateral.side(), Vec2::new(1.0, 0.0));

    let mut model = MotionModel::axis_swept(AxisSweptParams::default());
    let (_, vel) = one_free_tick(&mut model, lateral, InputState::default());
    // Descent accrues from the acceleration's projection on the SUPPLIED down
    // axis (900·dt), not from the magnitude of the tilted net vector.
    assert!(
        (vel.dot(lateral.down()) - 900.0 * DT).abs() < 1.0,
        "descent must follow the supplied down axis: {vel:?}"
    );
}

#[test]
fn a_frame_change_is_not_a_model_change_and_preserves_private_state() {
    // AXIS: an in-flight coyote window — model-private maneuver state inside
    // the AxisSwept variant — survives a frame rotation.
    let world = empty_world();
    let mut scratch =
        BodyClusterScratch::new_with_abilities(Vec2::splat(500.0), AbilitySet::default());
    let mut model = MotionModel::axis_swept(AxisSweptParams::default());
    let MotionModel::AxisSwept(axis) = &mut model else {
        unreachable!();
    };
    axis.state.coyote_timer = 0.08;
    let rotated = MotionFrame::from_acceleration(rotate(Vec2::new(0.0, 900.0), 0.4)).unwrap();
    step(
        &mut model,
        &world,
        &mut scratch,
        rotated,
        InputState::default(),
    );
    let MotionModel::AxisSwept(axis) = &model else {
        unreachable!();
    };
    assert!(
        (axis.state.coyote_timer - (0.08 - DT)).abs() < 1e-4,
        "a frame rotation must decay, not reset, the coyote window: {}",
        axis.state.coyote_timer
    );

    // SURFACE MOMENTUM: riding state (surface identity, arc position, speed)
    // survives a slow per-tick rotation of the frame.
    let chain = SurfaceChain::open(
        "long_floor",
        vec![Vec2::new(0.0, 600.0), Vec2::new(4_000.0, 600.0)],
    );
    let world = World::new(
        "rotating_frame_ride",
        Vec2::splat(10_000.0),
        Vec2::splat(500.0),
        Vec::new(),
    )
    .with_chains(vec![chain]);
    let mut scratch =
        BodyClusterScratch::new_with_abilities(Vec2::new(600.0, 590.0), AbilitySet::default());
    let mut model = MotionModel::surface_momentum(MomentumParams::default());
    let MotionModel::SurfaceMomentum(motion) = &mut model else {
        unreachable!();
    };
    motion.state = SurfaceMotion::Riding {
        on: SurfaceRef::Chain(0),
        s: 600.0,
        v_t: 400.0,
    };
    let mut last_s = 600.0;
    for tick in 0..20 {
        let angle = tick as f32 * 0.01;
        let frame = MotionFrame::from_acceleration(rotate(Vec2::new(0.0, 900.0), angle)).unwrap();
        step(
            &mut model,
            &world,
            &mut scratch,
            frame,
            InputState::default(),
        );
        let MotionModel::SurfaceMomentum(motion) = &model else {
            unreachable!();
        };
        let SurfaceMotion::Riding { on, s, .. } = motion.state else {
            panic!("tick {tick}: frame rotation shed the rider (state reset)");
        };
        assert_eq!(on, SurfaceRef::Chain(0));
        assert!(s > last_s, "tick {tick}: ride must keep advancing");
        last_s = s;
    }

    // ADHESIVE CRAWLER: an attachment survives the frame flipping upside
    // down — adhesion is policy-private state, not a gravity fact.
    let floor = Block::solid("floor", Vec2::new(400.0, 600.0), Vec2::new(400.0, 40.0));
    let world = World::new(
        "crawler_frame_flip",
        Vec2::splat(10_000.0),
        Vec2::splat(500.0),
        vec![floor],
    );
    let mut scratch =
        BodyClusterScratch::new_with_abilities(Vec2::new(500.0, 500.0), AbilitySet::default());
    scratch.kinematics.size = Vec2::new(24.0, 16.0);
    let mut model = MotionModel::adhesive_crawler(CrawlerParams::default());
    let down = MotionFrame::from_direction(Vec2::new(0.0, 1.0), 900.0);
    for _ in 0..240 {
        step(
            &mut model,
            &world,
            &mut scratch,
            down,
            InputState::default(),
        );
    }
    let MotionModel::AdhesiveCrawler(crawler) = &model else {
        unreachable!();
    };
    assert!(
        crawler.state.is_attached(),
        "crawler must land and attach under ordinary gravity"
    );
    let attachment = crawler.state.attachment();
    let up = MotionFrame::from_direction(Vec2::new(0.0, -1.0), 900.0);
    step(&mut model, &world, &mut scratch, up, InputState::default());
    let MotionModel::AdhesiveCrawler(crawler) = &model else {
        unreachable!();
    };
    assert_eq!(
        crawler.state.attachment(),
        attachment,
        "flipping the frame must not shed or reorient the clung surface"
    );
}

#[test]
fn cross_policy_switches_preserve_shared_state_and_initialize_only_destination_state() {
    let mut scratch = BodyClusterScratch::new_with_abilities(Vec2::ZERO, AbilitySet::default());
    scratch.kinematics.pos = Vec2::new(12.0, 34.0);
    scratch.kinematics.vel = Vec2::new(56.0, -78.0);
    scratch.kinematics.facing = -1.0;
    scratch.dash.charges_available = 2;
    scratch.jump.air_jumps_available = 1;
    let before = scratch.kinematics;

    // Accumulated axis-private maneuver state, inside the variant.
    let mut model = MotionModel::axis_swept(AxisSweptParams::default());
    let MotionModel::AxisSwept(axis) = &mut model else {
        unreachable!();
    };
    axis.state.coyote_timer = 0.1;
    axis.state.wall_clinging = true;
    axis.state.dash_timer = 0.05;

    // Same-variant refresh first: parameters change, maneuver state survives
    // by construction.
    let mut refreshed = AxisSweptParams::default();
    refreshed.locomotion.max_run_speed += 50.0;
    switch_motion_model(&mut model, MotionModelSpec::AxisSwept(refreshed));
    let MotionModel::AxisSwept(axis) = &model else {
        panic!("same-variant refresh changed movement policy");
    };
    assert_eq!(axis.state.coyote_timer, 0.1, "refresh keeps coyote grace");
    assert!(axis.state.wall_clinging, "refresh keeps wall engagement");
    assert_eq!(axis.state.dash_timer, 0.05, "refresh keeps dash maneuver");

    // Axis → surface momentum: shared world state untouched, destination
    // begins Airborne on lane 0 (no route search, no teleport).
    switch_motion_model(
        &mut model,
        MotionModelSpec::SurfaceMomentum(MomentumParams::default()),
    );
    assert_eq!(scratch.kinematics, before);
    let MotionModel::SurfaceMomentum(motion) = &model else {
        panic!("surface destination was not installed");
    };
    assert_eq!(motion.state, SurfaceMotion::Airborne);
    assert_eq!(motion.depth_lane, 0);

    // Simulate accumulated surface-private state, then switch back to axis:
    // shared state still untouched; the axis policy's maneuver state is the
    // fresh default (the old variant value is gone WITH its private state)
    // while body RESOURCES (dash charges, air jumps) survive on the clusters.
    let MotionModel::SurfaceMomentum(motion) = &mut model else {
        unreachable!();
    };
    motion.state = SurfaceMotion::Riding {
        on: SurfaceRef::Chain(1),
        s: 250.0,
        v_t: 800.0,
    };
    motion.depth_lane = -1;
    switch_motion_model(
        &mut model,
        MotionModelSpec::AxisSwept(AxisSweptParams::default()),
    );
    assert_eq!(
        scratch.kinematics, before,
        "shared world state must survive"
    );
    assert_eq!(model.kind(), MotionModelKind::AxisSwept);
    let MotionModel::AxisSwept(axis) = &model else {
        unreachable!();
    };
    assert_eq!(
        axis.state,
        crate::movement::AxisManeuverState::default(),
        "destination maneuver state is initialized, never imported"
    );
    assert_eq!(axis.state.coyote_timer, 0.0, "no imported coyote grace");
    assert!(!axis.state.wall_clinging, "no imported wall engagement");
    assert_eq!(axis.state.dash_timer, 0.0, "no imported dash maneuver");
    assert_eq!(scratch.dash.charges_available, 2, "resources preserved");
    assert_eq!(scratch.jump.air_jumps_available, 1, "resources preserved");

    // Surface → axis → surface round trip initialized only destination-private
    // state: the re-entered surface policy is Airborne again (its old ride was
    // its own private state, legitimately gone), still on the unchanged pose.
    switch_motion_model(
        &mut model,
        MotionModelSpec::SurfaceMomentum(MomentumParams::default()),
    );
    assert_eq!(scratch.kinematics, before);
    let MotionModel::SurfaceMomentum(motion) = &model else {
        panic!("surface destination was not installed");
    };
    assert_eq!(motion.state, SurfaceMotion::Airborne);

    // → crawler: begins detached; acquires support only via its own contact
    // rule on a later tick.
    switch_motion_model(
        &mut model,
        MotionModelSpec::AdhesiveCrawler(CrawlerParams::default()),
    );
    assert_eq!(scratch.kinematics, before);
    let MotionModel::AdhesiveCrawler(crawler) = &model else {
        panic!("crawler destination was not installed");
    };
    assert_eq!(crawler.state, CrawlerState::DETACHED);
}

#[test]
fn the_crawler_crawls_wraps_a_convex_corner_and_keeps_gluing() {
    // A lone solid block: the crawler lands on top, crawls right, wraps the
    // convex corner onto the right face (outward normal +X), and keeps
    // crawling down that face — under unchanged downward gravity.
    let block = Block::solid("island", Vec2::new(400.0, 600.0), Vec2::new(200.0, 200.0));
    let world = World::new(
        "crawler_corner",
        Vec2::splat(10_000.0),
        Vec2::splat(500.0),
        vec![block],
    );
    let mut scratch =
        BodyClusterScratch::new_with_abilities(Vec2::new(500.0, 560.0), AbilitySet::default());
    scratch.kinematics.size = Vec2::new(24.0, 16.0);
    scratch.kinematics.facing = 1.0;
    let mut model = MotionModel::adhesive_crawler(CrawlerParams {
        crawl_speed: 120.0,
        ..CrawlerParams::default()
    });
    let frame = MotionFrame::from_direction(Vec2::new(0.0, 1.0), 900.0);

    let mut seen_wall_cling = false;
    for _ in 0..600 {
        let result = step(
            &mut model,
            &world,
            &mut scratch,
            frame,
            InputState::default(),
        );
        let MotionModel::AdhesiveCrawler(crawler) = &model else {
            unreachable!();
        };
        if let Some(normal) = crawler.state.attachment() {
            assert_eq!(
                result.surface_normal, normal,
                "published support fact must be the clung normal"
            );
            if normal.x > 0.5 {
                seen_wall_cling = true;
                assert!(
                    scratch.kinematics.pos.x > 600.0,
                    "clinging to the right face means standing beside it: {:?}",
                    scratch.kinematics.pos
                );
                break;
            }
        }
    }
    assert!(
        seen_wall_cling,
        "the crawler never wrapped the convex corner onto the wall face"
    );
}

/// O5 evidence: the published support is a SEMANTIC fact selected by contact
/// kind. A grounded body shoved against a wall keeps its FLOOR support normal;
/// the lateral contact can never masquerade as support (the old last-nonzero-
/// contact rule published the wall normal here).
#[test]
fn a_wall_graze_never_masquerades_as_support() {
    use crate::collision_semantics::ContactKind;
    let world = World::new(
        "support_facts",
        Vec2::new(1000.0, 600.0),
        Vec2::new(200.0, 100.0),
        vec![
            Block::solid("floor", Vec2::new(0.0, 400.0), Vec2::new(1000.0, 40.0)),
            Block::solid("wall", Vec2::new(300.0, 0.0), Vec2::new(40.0, 400.0)),
        ],
    );
    let frame = MotionFrame::from_direction(Vec2::new(0.0, 1.0), 900.0);
    let mut model = MotionModel::axis_swept(AxisSweptParams::default());
    // Standing on the floor, hard against the wall's left face.
    let mut scratch =
        BodyClusterScratch::new_with_abilities(Vec2::new(285.0, 377.0), AbilitySet::default());
    scratch.ground.on_ground = true;
    let mut result = MotionStepResult::from_events(crate::movement::FrameEvents::default(), frame);
    for _ in 0..3 {
        let input = InputState {
            axes: LocalAxes::new(1.0, 0.0), // run INTO the wall
            ..InputState::default()
        };
        result = step(&mut model, &world, &mut scratch, frame, input);
    }
    assert!(
        result
            .events
            .contacts
            .iter()
            .any(|c| c.kind == ContactKind::Side),
        "the wall contact is present, classified as Side: {:?}",
        result.events.contacts
    );
    assert_eq!(
        result.surface_normal,
        Vec2::new(0.0, -1.0),
        "support is the FLOOR, not the last lateral contact"
    );
    match result.support {
        SupportFact::Supported(contact) => {
            assert_eq!(
                contact.kind,
                crate::collision_semantics::ContactKind::Support
            );
            assert_eq!(contact.normal, Vec2::new(0.0, -1.0));
        }
        other => panic!("grounded body must be Supported, got {other:?}"),
    }
}

/// O5 evidence: an attached crawler publishes an ATTACHMENT support fact whose
/// normal is the clung surface (independent of the frame), and an airborne body
/// publishes Airborne with the frame-up fallback normal.
#[test]
fn attachment_and_airborne_support_facts_are_semantic() {
    let world = World::new(
        "support_facts_crawler",
        Vec2::new(1000.0, 600.0),
        Vec2::new(200.0, 100.0),
        vec![Block::solid(
            "wall",
            Vec2::new(300.0, 0.0),
            Vec2::new(40.0, 600.0),
        )],
    );
    let frame = MotionFrame::from_direction(Vec2::new(0.0, 1.0), 900.0);

    // A crawler clung to the wall's LEFT face (normal (-1,0)) under ordinary
    // down gravity: support is the attachment, not the gravity floor.
    let mut model = MotionModel::AdhesiveCrawler(crate::movement::AdhesiveCrawlerMotion {
        params: CrawlerParams::default(),
        state: CrawlerState::attached(Vec2::new(-1.0, 0.0)),
    });
    let mut scratch =
        BodyClusterScratch::new_with_abilities(Vec2::new(276.0, 300.0), AbilitySet::default());
    let result = step(
        &mut model,
        &world,
        &mut scratch,
        frame,
        InputState::default(),
    );
    match result.support {
        SupportFact::Attached(contact) => {
            assert_eq!(contact.normal, Vec2::new(-1.0, 0.0));
        }
        other => panic!("attached crawler must publish Attached, got {other:?}"),
    }
    assert_eq!(result.surface_normal, Vec2::new(-1.0, 0.0));

    // A free-falling axis body far from any surface: Airborne + frame-up.
    let mut model = MotionModel::axis_swept(AxisSweptParams::default());
    let mut scratch =
        BodyClusterScratch::new_with_abilities(Vec2::new(700.0, 100.0), AbilitySet::default());
    let result = step(
        &mut model,
        &world,
        &mut scratch,
        frame,
        InputState::default(),
    );
    assert_eq!(result.support, SupportFact::Airborne);
    assert_eq!(result.surface_normal, Vec2::new(0.0, -1.0));
}
