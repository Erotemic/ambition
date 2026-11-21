//! Ledge grab latch (blink-wall surface, one-way surface) and the
//! "climb finishes inside one simulation tick" invariant.

use super::super::*;
use super::test_world;
use crate::body_clusters::BodyClusterScratch;
#[allow(unused_imports)]
use crate::test_support::*;
use crate::world::{BlinkWallTier, Block};
use crate::{AbilitySet, Vec2};

fn scratch_with(abilities: AbilitySet, spawn: Vec2) -> BodyClusterScratch {
    BodyClusterScratch::new_with_abilities(spawn, abilities)
}

#[test]
fn simulation_latches_ledge_grab_on_blink_wall_surface() {
    let mut world = test_world();
    world.blocks.push(Block::blink_wall(
        "soft blink ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
        BlinkWallTier::Soft,
    ));
    let mut abilities = AbilitySet::sandbox_all();
    abilities.ledge_grab = true;
    let mut scratch = scratch_with(abilities, Vec2::new(86.0, 110.0));
    scratch.kinematics.vel = Vec2::new(120.0, 20.0);
    scratch.axis_mut().wall_clinging = true;
    scratch.wall.on_wall = true;
    scratch.wall.wall_normal_x = -1.0;

    let events = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            axes: crate::LocalAxes::new(1.0, 0.0),
            control_dt: 1.0 / 60.0,
            ..InputState::default()
        },
        1.0 / 60.0,
        TEST_TUNING,
    );
    assert!(
        scratch.axis().ledge_grab.is_some(),
        "blink wall ledge should latch"
    );
    assert!(events.operations.contains(&MovementOp::LedgeGrab));
}

#[test]
fn simulation_latches_ledge_grab_on_one_way_surface_without_wall_collision() {
    let mut world = test_world();
    world.blocks.push(Block::one_way(
        "one-way ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 16.0),
    ));
    let mut abilities = AbilitySet::sandbox_all();
    abilities.ledge_grab = true;
    let mut scratch = scratch_with(abilities, Vec2::new(86.0, 110.0));
    scratch.ground.on_ground = false;
    scratch.kinematics.vel = Vec2::new(20.0, 40.0);

    let events = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            axes: crate::LocalAxes::new(1.0, 0.0),
            control_dt: 1.0 / 60.0,
            ..InputState::default()
        },
        1.0 / 60.0,
        TEST_TUNING,
    );
    assert!(
        scratch.axis().ledge_grab.is_some(),
        "pressing toward a one-way edge should allow a pull-up even though one-way platforms do not collide on X"
    );
    assert!(events.operations.contains(&MovementOp::LedgeGrab));
}

#[test]
fn attack_press_from_hang_starts_getup_attack_and_fires_slash() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.ledge_grab = true;
    let mut scratch = scratch_with(abilities, Vec2::new(87.0, 119.0));
    let contact = crate::LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(87.0, 119.0),
        climb_target: Vec2::new(118.0, 76.0),
    };
    let mut state = crate::LedgeGrabState::hanging(contact);
    // Skip past the hang debounce so the input is accepted this tick.
    state.elapsed = crate::LEDGE_MIN_CLIMB_DELAY;
    scratch.axis_mut().ledge_grab = Some(state);

    let events = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            attack_pressed: true,
            control_dt: 1.0 / 60.0,
            ..InputState::default()
        },
        1.0 / 60.0,
        TEST_TUNING,
    );

    assert!(
        events.operations.contains(&MovementOp::LedgeGetupAttack),
        "attack from hang should emit LedgeGetupAttack"
    );
    assert!(
        events.operations.contains(&MovementOp::Slash),
        "getup attack should also emit a Slash op so the hitbox fires"
    );
    let new_state = scratch
        .axis()
        .ledge_grab
        .expect("getup-attack keeps ledge state");
    assert!(new_state.climbing, "state should be in getup transition");
    assert_eq!(new_state.getup_kind, crate::LedgeGetupKind::Attack);
    assert!(
        scratch.axis().ledge_invuln_timer > 0.0,
        "getup attack grants intangibility via the LEDGE's own timer — it borrowed \
         the dodge roll's until the two were split"
    );
}

#[test]
fn active_ledge_grab_climb_finishes_inside_simulation_tick() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.ledge_grab = true;
    let mut scratch = scratch_with(abilities, Vec2::new(87.0, 119.0));
    let contact = crate::LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(87.0, 119.0),
        climb_target: Vec2::new(118.0, 76.0),
    };
    let mut state = crate::LedgeGrabState::hanging(contact);
    state.elapsed = crate::LEDGE_MIN_CLIMB_DELAY;
    state.climbing = true;
    state.climb_elapsed = crate::LEDGE_CLIMB_TIME;
    scratch.axis_mut().ledge_grab = Some(state);

    let events = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState::default(),
        1.0 / 60.0,
        TEST_TUNING,
    );
    assert!(
        scratch.axis().ledge_grab.is_none(),
        "completed climb clears ledge state"
    );
    assert_eq!(scratch.kinematics.pos, contact.climb_target);
    assert!(scratch.ground.on_ground);
    assert!(events.operations.contains(&MovementOp::LedgeClimbFinish));
}

/// The engine owns the hang and the climb: one step latches the grab, holding
/// up starts the climb after the hang delay, and it finishes inside the ticks.
mod hang_then_climb {
    use crate as ae;

    fn step_axis(
        world: &ae::World,
        scratch: &mut ae::BodyClusterScratch,
        model: &mut ae::MotionModel,
        input: ae::InputState,
        dt: f32,
    ) -> ae::FrameEvents {
        let frame = ae::MotionFrame::from_direction(ae::DEFAULT_GRAVITY_DIR, ae::GRAVITY);
        let mut clusters = scratch.as_mut();
        ae::step_motion(
            model,
            &mut clusters,
            ae::MotionStepContext {
                admission: None,
                world,
                input,
                frame,
                facing_intent: input.axes.x,
                dt,
                // A hypothetical probe step, not this body's real motion: nothing here
                // may be resisted by a body standing somewhere else.
                contact: ae::BodyContactField::NONE,
                pose_owned_externally: false,
                recovery_commitment_outstanding: false,
            },
        )
        .events
    }

    #[test]
    fn engine_owned_ledge_state_hangs_then_climbs() {
        let world = ae::World::new(
            "ledge_test",
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::new(80.0, 160.0),
            vec![ae::Block::solid(
                "ledge",
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(200.0, 200.0),
            )],
        );
        let mut abilities = ae::AbilitySet::sandbox_all();
        abilities.ledge_grab = true;
        let mut scratch = ae::BodyClusterScratch::new_with_abilities(world.spawn, abilities);
        scratch.kinematics.pos = ae::Vec2::new(86.0, 110.0);
        scratch.kinematics.vel = ae::Vec2::new(30.0, 20.0);
        scratch.wall.on_wall = true;
        scratch.wall.wall_normal_x = -1.0;

        let mut input = ae::InputState {
            axes: ae::LocalAxes::new(1.0, 0.0),
            control_dt: 0.016,
            ..Default::default()
        };
        let mut model = ae::MotionModel::axis_swept(ae::DEFAULT_AXIS_SWEPT_PARAMS);
        // Maneuver state (cling, ledge grab) is policy-private (ADR 0024):
        // arrange and assert through the axis variant.
        let axis_state = |model: &ae::MotionModel| match model {
            ae::MotionModel::AxisSwept(axis) => axis.state,
            other => panic!("test body is not axis-swept: {other:?}"),
        };
        if let ae::MotionModel::AxisSwept(axis) = &mut model {
            axis.state.wall_clinging = true;
        }
        let events = step_axis(&world, &mut scratch, &mut model, input, 0.016);
        assert!(
            axis_state(&model).ledge_grab.is_some(),
            "engine tick should latch ledge state"
        );
        assert!(events.operations.contains(&ae::MovementOp::LedgeGrab));

        input.axes.y = -1.0;
        let mut saw_start = false;
        for _ in 0..16 {
            let events = step_axis(&world, &mut scratch, &mut model, input, 0.016);
            if events.operations.contains(&ae::MovementOp::LedgeClimbStart) {
                saw_start = true;
                break;
            }
        }
        assert!(
            saw_start,
            "engine should start the climb after the hang delay"
        );
        assert!(axis_state(&model)
            .ledge_grab
            .map(|s| s.climbing)
            .unwrap_or(false));

        let mut saw_finish = false;
        for _ in 0..32 {
            let events = step_axis(&world, &mut scratch, &mut model, input, 0.016);
            if events
                .operations
                .contains(&ae::MovementOp::LedgeClimbFinish)
            {
                saw_finish = true;
                break;
            }
        }
        assert!(
            saw_finish,
            "engine should finish the climb inside simulation ticks"
        );
        assert!(axis_state(&model).ledge_grab.is_none());
        assert!(scratch.ground.on_ground);
    }
}
