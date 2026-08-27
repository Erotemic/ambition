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

fn floor_world() -> World {
    World::new(
        "ground_baseline",
        Vec2::new(1000.0, 600.0),
        Vec2::new(200.0, 100.0),
        vec![Block::solid(
            "floor",
            Vec2::new(0.0, 400.0),
            Vec2::new(1000.0, 40.0),
        )],
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
            contact: crate::movement::body_contact::BodyContactField::NONE,
            pose_owned_externally: false,
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

#[test]
fn grounded_spawn_initializes_without_a_landing_edge() {
    let world = floor_world();
    let frame = MotionFrame::from_direction(Vec2::new(0.0, 1.0), 900.0);
    let mut model = MotionModel::axis_swept(AxisSweptParams::default());
    let mut scratch = BodyClusterScratch::new_with_abilities(Vec2::ZERO, AbilitySet::default());
    scratch.kinematics.pos = Vec2::new(100.0, 400.0 - scratch.kinematics.size.y * 0.5);
    scratch.ground = crate::BodyGroundState::uninitialized();

    let result = step(
        &mut model,
        &world,
        &mut scratch,
        frame,
        InputState::default(),
    );

    assert!(scratch.ground.on_ground);
    assert!(scratch.ground.contact_initialized);
    assert_eq!(
        result.events.ground_contact,
        GroundContactTransition::InitializedGrounded
    );
}

#[test]
fn airborne_spawn_can_land_during_its_first_tick() {
    let world = floor_world();
    let frame = MotionFrame::from_direction(Vec2::new(0.0, 1.0), 900.0);
    let mut model = MotionModel::axis_swept(AxisSweptParams::default());
    let mut scratch = BodyClusterScratch::new_with_abilities(Vec2::ZERO, AbilitySet::default());
    // Start beyond the resting-contact slop but close enough that a fast
    // downward spawn crosses the floor during this first 1/60 s step.
    scratch.kinematics.pos = Vec2::new(100.0, 394.0 - scratch.kinematics.size.y * 0.5);
    scratch.kinematics.vel = Vec2::new(0.0, 600.0);
    scratch.ground = crate::BodyGroundState::uninitialized();

    let result = step(
        &mut model,
        &world,
        &mut scratch,
        frame,
        InputState::default(),
    );

    assert!(scratch.ground.on_ground);
    assert_eq!(
        result.events.ground_contact,
        GroundContactTransition::Landed {
            impact_speed: 600.0,
            // A body that jumped and came down chose to be here.
            involuntary: false,
        }
    );
}

#[test]
fn airborne_spawn_initializes_without_fabricating_a_transition() {
    let world = floor_world();
    let frame = MotionFrame::from_direction(Vec2::new(0.0, 1.0), 900.0);
    let mut model = MotionModel::axis_swept(AxisSweptParams::default());
    let mut scratch =
        BodyClusterScratch::new_with_abilities(Vec2::new(100.0, 100.0), AbilitySet::default());
    scratch.ground = crate::BodyGroundState::uninitialized();

    let result = step(
        &mut model,
        &world,
        &mut scratch,
        frame,
        InputState::default(),
    );

    assert!(!scratch.ground.on_ground);
    assert_eq!(
        result.events.ground_contact,
        GroundContactTransition::InitializedAirborne
    );
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

/// The adhesive crawler now SAYS when it attaches and detaches.
///
/// the edge is derived in `step_adhesive_crawler` from the attachment either
/// side of the step, NOT pushed at the eight sites inside `step_crawler` that
/// detach or re-attach. `step_crawler` has several early returns, so an
/// emit-at-the-end rule inside it would silently skip the paths that exit early
/// — and a missing edge in a causal log reads as "it did not happen".
#[test]
fn the_crawler_announces_its_attach_and_detach_edges() {
    let floor = Block::solid("floor", Vec2::new(400.0, 600.0), Vec2::new(400.0, 40.0));
    let world = World::new(
        "crawler_edges",
        Vec2::splat(10_000.0),
        Vec2::splat(500.0),
        vec![floor],
    );
    let mut scratch =
        BodyClusterScratch::new_with_abilities(Vec2::new(500.0, 500.0), AbilitySet::default());
    scratch.kinematics.size = Vec2::new(24.0, 16.0);
    let mut model = MotionModel::adhesive_crawler(CrawlerParams::default());
    let down = MotionFrame::from_direction(Vec2::new(0.0, 1.0), 900.0);

    // Fall until it seats. Exactly ONE tick may claim the attach.
    let mut attach_ticks = Vec::new();
    for tick in 0..240 {
        let result = step(
            &mut model,
            &world,
            &mut scratch,
            down,
            InputState::default(),
        );
        if result
            .events
            .operations
            .contains(&crate::movement::MovementOp::CrawlAttach)
        {
            attach_ticks.push(tick);
        }
        assert!(
            !result
                .events
                .operations
                .contains(&crate::movement::MovementOp::CrawlDetach),
            "tick {tick}: nothing detached — the body only ever fell and landed"
        );
    }
    assert_eq!(
        attach_ticks.len(),
        1,
        "the attach is an EDGE: exactly one tick may claim it, not every tick the \
         crawler is attached. Ticks that claimed it: {attach_ticks:?}"
    );
    let MotionModel::AdhesiveCrawler(crawler) = &model else {
        unreachable!();
    };
    assert!(crawler.state.is_attached(), "and it is attached afterwards");

    // Now take the surface away. The detach must announce itself on the tick it
    // happens, from the same derivation.
    let empty = World::new(
        "crawler_edges_empty",
        Vec2::splat(10_000.0),
        Vec2::splat(500.0),
        Vec::new(),
    );
    let result = step(
        &mut model,
        &empty,
        &mut scratch,
        down,
        InputState::default(),
    );
    assert!(
        result
            .events
            .operations
            .contains(&crate::movement::MovementOp::CrawlDetach),
        "the surface vanished, so the crawler detached — and a detach nobody \
         publishes is the state this test exists to end. ops={:?}",
        result.events.operations
    );
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
        if let Some(normal) = crawler.state.attached_normal(&world) {
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

/// O6 evidence: attached crawling is one surface-basis algorithm — the crawler
/// circumnavigates a solid island, wrapping all four convex corners, staying
/// seated one half-thickness off WHICHEVER face it clings to. Every attachment
/// (floor-top, both walls, the underside) exercises the same crawl/corner/seat
/// math; nothing branches on world axes.
#[test]
fn the_crawler_circumnavigates_an_island_gluing_to_all_four_faces() {
    let block = Block::solid("island", Vec2::new(400.0, 600.0), Vec2::new(200.0, 200.0));
    let world = World::new(
        "crawler_lap",
        Vec2::splat(10_000.0),
        Vec2::splat(500.0),
        vec![block],
    );
    let mut scratch =
        BodyClusterScratch::new_with_abilities(Vec2::new(500.0, 560.0), AbilitySet::default());
    scratch.kinematics.size = Vec2::new(24.0, 16.0);
    scratch.kinematics.facing = 1.0;
    let mut model = MotionModel::adhesive_crawler(CrawlerParams {
        crawl_speed: 240.0,
        ..CrawlerParams::default()
    });
    let frame = MotionFrame::from_direction(Vec2::new(0.0, 1.0), 900.0);
    let half = scratch.kinematics.size * 0.5;

    let mut seen: std::collections::BTreeSet<(i32, i32)> = std::collections::BTreeSet::new();
    for _ in 0..2000 {
        step(
            &mut model,
            &world,
            &mut scratch,
            frame,
            InputState::default(),
        );
        let MotionModel::AdhesiveCrawler(crawler) = &model else {
            unreachable!();
        };
        let Some(normal) = crawler.state.attached_normal(&world) else {
            continue;
        };
        seen.insert((normal.x.round() as i32, normal.y.round() as i32));
        // Seated: the body sits half its OWN extent along that face's normal off
        // the face — the seat rule is basis-relative, not a floor special case.
        let pos = scratch.kinematics.pos;
        let (face_coord, want) = match (normal.x.round() as i32, normal.y.round() as i32) {
            (0, -1) => (600.0 - pos.y, half.y), // top face: distance above y=600
            (1, 0) => (pos.x - 600.0, half.x),  // right face at x=600
            (0, 1) => (pos.y - 800.0, half.y),  // underside at y=800
            (-1, 0) => (400.0 - pos.x, half.x), // left face at x=400
            other => panic!("unexpected attachment normal {other:?}"),
        };
        assert!(
            (face_coord - want).abs() <= 2.0,
            "seated {face_coord:.2}px off the {normal:?} face (want ~{want}); \
             a body seated closer than its own extent is INSIDE the geometry"
        );
        if seen.len() == 4 {
            break;
        }
    }
    assert_eq!(
        seen.len(),
        4,
        "the crawler must glue to all four faces in one lap; saw {seen:?}"
    );
}

/// O6 evidence: a crawler falling under an OBLIQUE frame lands on cardinal
/// world geometry and attaches to the SURFACE's true normal (the semantic
/// Support contact), not the frame's anti-down — adhesion is about the
/// surface, and the detached leg is fully frame-covariant.
#[test]
fn an_oblique_frame_crawler_attaches_to_the_landed_surfaces_true_normal() {
    let world = World::new(
        "crawler_oblique_landing",
        Vec2::splat(4_000.0),
        Vec2::splat(500.0),
        vec![Block::solid(
            "floor",
            Vec2::new(0.0, 1000.0),
            Vec2::new(4000.0, 60.0),
        )],
    );
    // Down tilted ~22° off vertical: the fall drifts sideways while dropping.
    let oblique = rotate(Vec2::new(0.0, 1.0), 0.4);
    let frame = MotionFrame::from_direction(oblique, 900.0);
    let mut model = MotionModel::adhesive_crawler(CrawlerParams::default());
    let mut scratch =
        BodyClusterScratch::new_with_abilities(Vec2::new(1200.0, 700.0), AbilitySet::default());
    scratch.kinematics.size = Vec2::new(24.0, 16.0);

    let mut attached = None;
    for _ in 0..600 {
        step(
            &mut model,
            &world,
            &mut scratch,
            frame,
            InputState::default(),
        );
        let MotionModel::AdhesiveCrawler(crawler) = &model else {
            unreachable!();
        };
        if let Some(normal) = crawler.state.attached_normal(&world) {
            attached = Some(normal);
            break;
        }
    }
    assert_eq!(
        attached,
        Some(Vec2::new(0.0, -1.0)),
        "the attachment is the FLOOR's outward normal, not -frame.down ({:?})",
        -frame.down()
    );
}

/// O6 headline evidence: TRUE arbitrary-angle attached crawling. A closed
/// surface chain — a square island rotated by an arbitrary 0.731 rad — captures
/// a falling crawler mid-air, and the crawler circumnavigates it: seated one
/// half-thickness off the OBLIQUE surface at every step, transiting all four
/// oblique corners (the polyline walk IS the corner transit), publishing the
/// oblique attachment normal as the support fact. No world-axis case anywhere.
#[test]
fn the_crawler_circumnavigates_an_arbitrarily_rotated_chain_island() {
    let center = Vec2::new(1000.0, 1000.0);
    let square = [
        Vec2::new(-150.0, -150.0),
        Vec2::new(150.0, -150.0),
        Vec2::new(150.0, 150.0),
        Vec2::new(-150.0, 150.0),
    ];
    let points: Vec<Vec2> = square.iter().map(|p| center + rotate(*p, 0.731)).collect();
    let chain = SurfaceChain::closed_loop("oblique_island", points);
    let world = World::new(
        "crawler_oblique_lap",
        Vec2::splat(4_000.0),
        Vec2::splat(500.0),
        vec![],
    )
    .with_chains(vec![chain]);
    let surface = &world.chains[0];

    // Drop the crawler above the island; a slow terminal speed keeps the fall
    // inside the adhesion capture window (adhesion is a touch, not a sweep).
    let mut model = MotionModel::adhesive_crawler(CrawlerParams {
        crawl_speed: 260.0,
        max_fall_speed: 120.0,
    });
    let mut scratch = BodyClusterScratch::new_with_abilities(
        center + Vec2::new(0.0, -320.0),
        AbilitySet::default(),
    );
    scratch.kinematics.size = Vec2::new(24.0, 16.0);
    scratch.kinematics.facing = 1.0;
    let body_thick = scratch.kinematics.size.y * 0.5;
    let frame = MotionFrame::from_direction(Vec2::new(0.0, 1.0), 300.0);

    let mut segments_seen = std::collections::BTreeSet::new();
    let mut attached_ticks = 0u32;
    for _ in 0..2000 {
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
        let Some(crate::movement::CrawlAttachment::Chain { s, .. }) = crawler.state.attachment()
        else {
            continue;
        };
        attached_ticks += 1;
        // Seated: one half-thickness off the OBLIQUE surface, every tick.
        let (_, signed) = surface.project(scratch.kinematics.pos);
        assert!(
            (signed - body_thick).abs() <= 1.0,
            "seated {signed:.2}px off the oblique surface (want ~{body_thick})"
        );
        // The published support fact is the ATTACHED oblique normal.
        let normal = surface.frame_at(s).normal;
        assert_eq!(result.surface_normal, normal);
        assert!(
            normal.x.abs() > 0.05 && normal.y.abs() > 0.05,
            "the attachment normal is genuinely oblique: {normal:?}"
        );
        segments_seen.insert(surface.frame_at(s).segment);
        if segments_seen.len() == 4 && attached_ticks > 60 {
            break;
        }
    }
    assert_eq!(
        segments_seen.len(),
        4,
        "one lap crosses all four oblique faces; saw {segments_seen:?}"
    );
}

// ── The launch channel, through the real gateway ──────────────────────

/// A riding surface-momentum body with a floor under it, running along it.
fn rider_on_a_long_floor() -> (World, BodyClusterScratch, MotionModel) {
    let chain = SurfaceChain::open(
        "long_floor",
        vec![Vec2::new(0.0, 600.0), Vec2::new(4_000.0, 600.0)],
    );
    let world = World::new(
        "launch_channel",
        Vec2::splat(10_000.0),
        Vec2::splat(500.0),
        Vec::new(),
    )
    .with_chains(vec![chain]);
    let scratch =
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
    (world, scratch, model)
}

/// The reader's half of the knockback seam, entered where production enters
/// it.
///
/// `BodyFlightState::pending_launch` is written by a damage reaction that holds
/// no world and no `MotionModel`; `step_motion` is the one place that can hand it
/// over. This asserts the handover happens and that the model acts on it — a
/// running body, hit upward and back, ends the tick AIRBORNE rather than still
/// riding at its old speed.
#[test]
fn a_pending_launch_takes_a_rider_off_the_floor_through_step_motion() {
    let (world, mut scratch, mut model) = rider_on_a_long_floor();
    let frame = MotionFrame::from_acceleration(Vec2::new(0.0, 900.0)).unwrap();

    {
        let clusters = scratch.as_mut();
        clusters.flight.pending_launch = Vec2::new(-360.0, -260.0);
    }
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
    assert!(
        matches!(motion.state, SurfaceMotion::Airborne),
        "a launch with an off-surface component must END the ride: {:?}",
        motion.state
    );
    let clusters = scratch.as_mut();
    assert!(
        clusters.kinematics.vel.y < 0.0,
        "and the body must actually be moving away from the floor: {:?}",
        clusters.kinematics.vel
    );
}

/// The channel is DRAINED, not merely read. A launch that stayed set would
/// re-fire every tick and a body would never come down — the failure mode of a
/// one-shot written into persistent state.
#[test]
fn the_launch_channel_is_emptied_by_the_step_that_consumes_it() {
    let (world, mut scratch, mut model) = rider_on_a_long_floor();
    let frame = MotionFrame::from_acceleration(Vec2::new(0.0, 900.0)).unwrap();

    {
        let clusters = scratch.as_mut();
        clusters.flight.pending_launch = Vec2::new(-360.0, -260.0);
    }
    step(
        &mut model,
        &world,
        &mut scratch,
        frame,
        InputState::default(),
    );
    {
        let clusters = scratch.as_mut();
        assert_eq!(
            clusters.flight.pending_launch,
            Vec2::ZERO,
            "the step that consumed the launch must clear it"
        );
    }

    // And the tick after it, gravity is the only thing acting: the body is
    // slowing its climb rather than being re-launched.
    let climbing = scratch.as_mut().kinematics.vel.y;
    step(
        &mut model,
        &world,
        &mut scratch,
        frame,
        InputState::default(),
    );
    let after = scratch.as_mut().kinematics.vel.y;
    assert!(
        after > climbing,
        "a drained launch does not re-fire; gravity should be pulling the upward \
         velocity back toward zero ({climbing} -> {after})"
    );
}

/// THE CAPABILITY REACHES THE REAL SWEEP — through `step_motion`, with the
/// real controller running.
///
/// So this enters through `step_motion`, holds RIGHT for a full second of ticks
/// against the controller that would overwrite a force, and measures the body.
///
/// the A/B is the assertion. The same run happens twice — once with the
/// body's contact field populated and once with `NONE`, the documented identity
/// — and the free body must travel measurably further. No distance in this test
/// is chosen; both are measured, and if the constraint were inert they would be
/// equal.
#[test]
fn a_grounded_body_walking_into_another_one_is_stopped_by_the_real_sweep() {
    fn walk_right(blockers: &[crate::movement::BodyContactBlocker]) -> f32 {
        let world = floor_world();
        let start = Vec2::new(200.0, 380.0);
        let mut scratch = BodyClusterScratch::new_with_abilities(start, AbilitySet::default());
        let mut model = MotionModel::axis_swept(AxisSweptParams::default());
        let frame = MotionFrame::from_direction(Vec2::new(0.0, 1.0), 900.0);
        let mut input = InputState::default();
        input.axes = LocalAxes::new(1.0, 0.0);
        for _ in 0..60 {
            // The mover's own velocity as the snapshot would have recorded it,
            // read before the step that is about to change it.
            let own_velocity = scratch.kinematics.vel;
            let mut clusters = scratch.as_mut();
            step_motion(
                &mut model,
                &mut clusters,
                MotionStepContext {
                    world: &world,
                    input,
                    frame,
                    facing_intent: 1.0,
                    dt: DT,
                    contact: crate::movement::body_contact::BodyContactField::moving(
                        blockers,
                        1.0,
                        own_velocity,
                    ),
                    pose_owned_externally: false,
                },
            );
        }
        scratch.as_mut().kinematics.pos.x - start.x
    }

    let free = walk_right(&[]);
    assert!(
        free > 100.0,
        "the fixture never walked anywhere ({free:.1}px), so nothing below \
         measures a constraint",
    );

    // A wall of another body 60px to the right of the start pose.
    let blocker = [crate::movement::BodyContactBlocker::new(
        crate::Aabb::new(Vec2::new(300.0, 380.0), Vec2::new(20.0, 24.0)),
        // Standing there. A stationary blocker leaves the whole gap to the
        // mover, which is what makes this an A/B on the constraint rather than
        // on the gap split.
        Vec2::ZERO,
    )];
    let blocked = walk_right(&blocker);
    assert!(
        blocked < free - 20.0,
        "a body walking into another one travelled {blocked:.1}px against \
         {free:.1}px unobstructed: the constraint is not reaching the sweep, \
         which is exactly how an acceleration term passes its own unit tests \
         and moves nothing in a game",
    );
    assert!(
        blocked > 0.0,
        "the constrained body went backwards ({blocked:.1}px) — this pass may \
         only ever REDUCE motion, never separate bodies",
    );
}

/// TWO BODIES, STEPPED THE WAY THE SCHEDULE STEPS THEM. One snapshot of both
/// poses and both velocities taken before either resolves its controller — what
/// `snapshot_body_contact` does immediately before the integration phase — and
/// then each body driven through the real kernel against it.
///
/// the fixture may not manufacture the velocities, and that is the whole point of it. Every
/// number the constraint divides by here is one the controller produced from input this tick.
///
/// it lands the pair before it drives them. Body contact is a grounded
/// capability by construction, so a fixture that starts walking on tick zero
/// measures two bodies drifting through each other in mid-air and blames the
/// constraint for it.
///
/// Returns `(worst overlap, final overlap, final gap)` in pixels.
fn walk_a_pair(
    gap: f32,
    ticks: usize,
    swap_order: bool,
    input_x: impl Fn(usize) -> (f32, f32),
) -> (f32, f32, f32) {
    const SETTLE: usize = 30;
    let world = floor_world();
    let down = Vec2::new(0.0, 1.0);
    let frame = MotionFrame::from_direction(down, 900.0);
    let size = crate::movement::default_player_body_size();
    let left_x = 400.0;
    let mut bodies = [
        BodyClusterScratch::new_with_abilities(Vec2::new(left_x, 380.0), AbilitySet::default()),
        BodyClusterScratch::new_with_abilities(
            Vec2::new(left_x + size.x + gap, 380.0),
            AbilitySet::default(),
        ),
    ];
    let mut models = [
        MotionModel::axis_swept(AxisSweptParams::default()),
        MotionModel::axis_swept(AxisSweptParams::default()),
    ];
    let mut worst: f32 = 0.0;
    for tick in 0..SETTLE + ticks {
        let axes: [f32; 2] = if tick < SETTLE {
            [0.0, 0.0]
        } else {
            let (a, b) = input_x(tick - SETTLE);
            [a, b]
        };
        // THE SNAPSHOT — one sample, before anybody moves.
        let snapshot: Vec<crate::movement::BodyContactBlocker> = bodies
            .iter()
            .map(|body| {
                crate::movement::BodyContactBlocker::new(
                    body.kinematics.aabb_oriented(down),
                    body.kinematics.vel,
                )
            })
            .collect();
        // the order these two resolve in must not change the answer, so the
        // caller can reverse it.
        let order: [usize; 2] = if swap_order { [1, 0] } else { [0, 1] };
        for which in order {
            let other = 1 - which;
            let mut input = InputState::default();
            input.axes = LocalAxes::new(axes[which], 0.0);
            let blockers = [snapshot[other]];
            let mut clusters = bodies[which].as_mut();
            step_motion(
                &mut models[which],
                &mut clusters,
                MotionStepContext {
                    world: &world,
                    input,
                    frame,
                    facing_intent: axes[which],
                    dt: DT,
                    // `resistance == 1.0` is the value that promises a SOLID,
                    // so it is the value that has to hold exactly.
                    contact: crate::movement::BodyContactField::moving(
                        &blockers,
                        1.0,
                        snapshot[which].entry_velocity,
                    ),
                    pose_owned_externally: false,
                },
            );
        }
        if tick >= SETTLE {
            worst = worst.max(overlap_of(&bodies, down));
        }
    }
    let settled = overlap_of(&bodies, down);
    let gap_left = (-separation_of(&bodies, down)).max(0.0);
    (worst, settled, gap_left)
}

fn separation_of(bodies: &[BodyClusterScratch; 2], down: Vec2) -> f32 {
    bodies[0].kinematics.aabb_oriented(down).max.x - bodies[1].kinematics.aabb_oriented(down).min.x
}

fn overlap_of(bodies: &[BodyClusterScratch; 2], down: Vec2) -> f32 {
    separation_of(bodies, down).max(0.0)
}

/// TWO BODIES STARTING FROM REST MAY NOT BOTH SPEND ONE GAP.
///
/// The proportional split divides by snapshot velocities, and a snapshot taken before either
/// controller has run reads zero for both bodies, so each was told the gap was entirely its own.
/// `resistance == 1.0` promises a solid; a solid pair sitting a pixel inside each other permanently
/// is that promise broken.
#[test]
fn two_bodies_that_begin_walking_at_each_other_on_one_tick_never_overlap() {
    let both_start = |_tick: usize| -> (f32, f32) { (1.0, -1.0) };
    for gap in [0.25_f32, 0.5, 1.0, 2.0, 4.0, 8.0] {
        for swap in [false, true] {
            let (worst, settled, gap_left) = walk_a_pair(gap, 200, swap, both_start);
            assert_eq!(
                worst, 0.0,
                "two bodies at rest {gap}px apart both spent the gap and \
                 overlapped by {worst:.3}px (settling at {settled:.3}px) — at \
                 resistance 1.0 they are solids and may not be inside each \
                 other at all",
            );
            // and they must actually MEET. Halving the gap unconditionally
            // would also score zero overlap while quietly stopping the pair
            // short of each other, which is the failure this catches.
            assert!(
                gap_left < 0.5,
                "the pair stopped {gap_left:.3}px apart from a {gap}px start: \
                 the shares no longer sum to the gap, so contact now happens \
                 somewhere short of contact",
            );
        }
    }
}

/// AND THE SNAPSHOT IS WRONG ABOUT THIS TICK EVERY TIME EITHER BODY CHANGES
/// ITS MIND, not only when both start from rest — so one body is held against
/// the other while the second starts, stops and reverses at every phase relative
/// to the moment they touch.
///
/// a swept pattern rather than three hand-timed scenarios. Timing a stop
/// to land exactly on the tick of contact by hand is a fixture that passes
/// because the collision missed it; varying the period walks the change across
/// every offset, contact included.
#[test]
fn a_pair_whose_motion_changes_this_tick_still_never_overlaps() {
    for period in 2..12usize {
        // The second body alternates between coming and standing still.
        let stutter = move |tick: usize| -> (f32, f32) {
            let coming = (tick / period) % 2 == 0;
            (1.0, if coming { -1.0 } else { 0.0 })
        };
        // The second body alternates between coming and fleeing — a reversal
        // every `period` ticks, so its entry velocity points the wrong way for
        // the step it is about to take.
        let reversing = move |tick: usize| -> (f32, f32) {
            let coming = (tick / period) % 2 == 0;
            (1.0, if coming { -1.0 } else { 1.0 })
        };
        for (label, drive) in [
            (
                "stops and restarts",
                &stutter as &dyn Fn(usize) -> (f32, f32),
            ),
            ("reverses", &reversing),
        ] {
            for swap in [false, true] {
                let (worst, settled, _) = walk_a_pair(6.0, 240, swap, drive);
                assert_eq!(
                    worst,
                    0.0,
                    "the second body {label} every {period} ticks (resolved {}): \
                     the pair overlapped by {worst:.3}px and settled at \
                     {settled:.3}px",
                    if swap { "second first" } else { "first first" },
                );
            }
        }
    }
}

/// AND A BODY WALKING AT SOMEBODY MERELY STANDING THERE STILL GETS THE WHOLE
/// GAP. The falsifier for the no-evidence share: dividing the gap evenly
/// whenever the snapshot is silent would score zero overlap everywhere and still
/// be wrong, because a lone mover would lose half of every approach to a
/// neighbour that is not contesting it. It costs exactly one tick — the one on
/// which the mover is itself still at rest — and nothing after that.
#[test]
fn a_lone_mover_is_not_charged_for_a_neighbour_that_never_moves() {
    let one_walks = |_tick: usize| -> (f32, f32) { (1.0, 0.0) };
    let (worst, _, gap_left) = walk_a_pair(60.0, 200, false, one_walks);
    assert_eq!(worst, 0.0, "the mover walked into the stationary body");
    assert!(
        gap_left < 0.05,
        "a body walking at a neighbour that never moved stopped {gap_left:.3}px \
         short of it: the no-evidence share is being charged to a mover whose \
         neighbour is not contesting the gap at all",
    );
}

/// A BODY SET DOWN EXACTLY ON THE GROUND CAN STILL WALK.
///
/// A respawn, a room placement or a scripted warp arrives at rest
/// ([`TransitVelocity::Zero`]), and a caller that reuses the body's own resting
/// height moves it sideways onto flat ground. If contact resolves from that pose
/// in a way the controller then refuses, the body is inert for good: grounded, at
/// rest, and deaf to input.
#[test]
fn a_body_transited_flush_with_the_ground_can_still_walk() {
    let world = floor_world();
    let down = Vec2::new(0.0, 1.0);
    let frame = MotionFrame::from_direction(down, 900.0);
    let mut scratch =
        BodyClusterScratch::new_with_abilities(Vec2::new(200.0, 380.0), AbilitySet::default());
    let mut model = MotionModel::axis_swept(AxisSweptParams::default());

    // Let her land and come to rest, so the next pose is her OWN resting height.
    for _ in 0..90 {
        let mut clusters = scratch.as_mut();
        step_motion(
            &mut model,
            &mut clusters,
            MotionStepContext {
                world: &world,
                input: InputState::default(),
                frame,
                facing_intent: 0.0,
                dt: DT,
                contact: crate::movement::BodyContactField::NONE,
                pose_owned_externally: false,
            },
        );
    }
    let resting = scratch.kinematics.pos;

    // Move her sideways to the same height, exactly as a placement helper does.
    {
        let mut clusters = scratch.as_mut();
        crate::movement::transit_body(
            &mut model,
            &mut clusters,
            Vec2::new(resting.x + 100.0, resting.y),
            crate::movement::TransitVelocity::Zero,
        );
    }
    for _ in 0..30 {
        let mut clusters = scratch.as_mut();
        step_motion(
            &mut model,
            &mut clusters,
            MotionStepContext {
                world: &world,
                input: InputState::default(),
                frame,
                facing_intent: 0.0,
                dt: DT,
                contact: crate::movement::BodyContactField::NONE,
                pose_owned_externally: false,
            },
        );
    }
    let settled = scratch.kinematics.pos;

    let mut input = InputState::default();
    input.axes = LocalAxes::new(1.0, 0.0);
    for _ in 0..60 {
        let mut clusters = scratch.as_mut();
        step_motion(
            &mut model,
            &mut clusters,
            MotionStepContext {
                world: &world,
                input,
                frame,
                facing_intent: 1.0,
                dt: DT,
                contact: crate::movement::BodyContactField::NONE,
                pose_owned_externally: false,
            },
        );
    }

    let walked = scratch.kinematics.pos.x - settled.x;
    assert!(
        walked > 20.0,
        "a body set down flush with the ground walked {walked:.2}px in a second \
         of held input — a transit that arrives at rest on flat ground must not \
         leave the body inert",
    );
}

/// ⭐⭐ A HELD BODY DOES NOT SPEND ITS LAUNCH, and gets it in full once released.
///
/// ⛔⛔ THE DEFECT THIS PINS, measured on the pirate's shark 2026-08-27: the
/// kernel drained the pending launch on every step, including steps where a
/// saddle owned the rider's pose and overwrote `vel` with zero straight after.
/// So a hit strong enough to end the ride was CONSUMED — the rider tumbled, the
/// dismount fired a stage later, and the body dropped out of the saddle at zero
/// velocity. The knockback had already been spent into a value somebody else
/// was about to erase.
///
/// ⭐ THE ARMS STRADDLE THE OWNERSHIP FLAG with everything else held still: same
/// staged launch, same body, same tick budget. Held, the launch survives and the
/// body has not moved; released, the same staged launch is spent and the body
/// carries it.
#[test]
fn a_pose_owned_body_keeps_its_pending_launch_until_it_is_released() {
    let world = empty_world();
    let launch = Vec2::new(400.0, -300.0);

    let mut held_scratch =
        BodyClusterScratch::new_with_abilities(Vec2::splat(500.0), AbilitySet::default());
    let mut held_model = MotionModel::default();
    {
        let mut clusters = held_scratch.as_mut();
        clusters.flight.stage_launch(launch, false);
        step_motion(
            &mut held_model,
            &mut clusters,
            MotionStepContext {
                world: &world,
                input: InputState::default(),
                frame: MotionFrame::from_direction(Vec2::new(0.0, 1.0), 900.0),
                facing_intent: 0.0,
                dt: DT,
                contact: crate::movement::body_contact::BodyContactField::NONE,
                pose_owned_externally: true,
            },
        );
    }
    assert_eq!(
        held_scratch.as_mut().flight.pending_launch,
        launch,
        "a body whose pose another authority owns spent its launch anyway — the \
         knockback goes into a velocity the constraint overwrites this same tick, \
         so the hit lands and the body never moves"
    );

    // ...AND THE SAME STAGED LAUNCH IS SPENT THE MOMENT NOBODY OWNS THE POSE.
    // Without this arm the assertion above is satisfied by a kernel that never
    // accepts a launch at all.
    {
        let mut clusters = held_scratch.as_mut();
        step_motion(
            &mut held_model,
            &mut clusters,
            MotionStepContext {
                world: &world,
                input: InputState::default(),
                frame: MotionFrame::from_direction(Vec2::new(0.0, 1.0), 900.0),
                facing_intent: 0.0,
                dt: DT,
                contact: crate::movement::body_contact::BodyContactField::NONE,
                pose_owned_externally: false,
            },
        );
    }
    let mut released = held_scratch.as_mut();
    assert_eq!(
        released.flight.pending_launch,
        Vec2::ZERO,
        "the launch was still staged after a free tick, so it is not being spent \
         at all rather than being deferred"
    );
    assert!(
        released.kinematics.vel.x > 1.0,
        "the released body carries no lateral knockback ({:?}), so the launch was \
         dropped rather than deferred",
        released.kinematics.vel
    );
}
