//! Reusable momentum-horizontal + phased-gravity jump laws used by Mary-O Classic.

use super::super::*;
use super::test_world;
use crate::body_clusters::BodyClusterScratch;
use crate::test_support::{update_player_with_tuning_scratch, TestTuning};
use crate::{AbilityGrant, AbilitySet, AccelerationFrame, LocalAxes, MotionFrame, Vec2};

fn classic_tuning() -> MovementTuning {
    MovementTuning {
        horizontal_law: AxisHorizontalLaw::Momentum(MomentumHorizontalTuning {
            ground_reverse_accel: 1500.0,
            ground_coast_decel: 1200.0,
            air_reverse_accel: 900.0,
            air_coast_decel: 0.0,
        }),
        jump_law: AxisJumpLaw::PhasedGravity(PhasedGravityJumpTuning {
            speed_thresholds: [120.0, 187.5, 210.0],
            launch_offsets: [-30.0, -15.0, 0.0, 30.0],
            held_rise_gravity_scale: 0.2,
            released_rise_gravity_scale: 1.0,
            fall_gravity_scale: 1.0,
            held_phase_min_upward_speed: 240.0,
        }),
        gravity: 2250.0,
        run_accel: 393.75,
        air_accel: 393.75,
        max_run_speed: 300.0,
        max_fall_speed: 480.0,
        jump_speed: 450.0,
        coyote_time: 0.0,
        jump_buffer: 0.0,
        ..DEFAULT_TUNING
    }
}

fn jump_held_input(x: f32) -> InputState {
    InputState {
        axes: LocalAxes::new(x, 0.0),
        movement: ActionEdges::EMPTY.with(
            MovementAction::Jump,
            Edge {
                pressed: false,
                held: true,
                released: false,
            },
        ),
        ..Default::default()
    }
}

fn jump_edge(pressed: bool, held: bool, released: bool) -> InputState {
    InputState {
        movement: ActionEdges::EMPTY.with(
            MovementAction::Jump,
            Edge {
                pressed,
                held,
                released,
            },
        ),
        ..Default::default()
    }
}

fn step_spine(
    vel: &mut Vec2,
    phase: &mut PhasedJumpState,
    input: InputState,
    frame: MotionFrame,
    on_ground: bool,
) {
    step_spine_with_variable_jump(vel, phase, input, frame, on_ground, true);
}

fn step_spine_with_variable_jump(
    vel: &mut Vec2,
    phase: &mut PhasedJumpState,
    input: InputState,
    frame: MotionFrame,
    on_ground: bool,
    can_variable_jump: bool,
) {
    let mut fast_falling = false;
    let mut gliding = false;
    let mut carried_run = 0.0;
    let mut carried_hold = 0.0;
    integrate_normal_spine(
        vel,
        &mut fast_falling,
        &mut gliding,
        &mut carried_run,
        &mut carried_hold,
        phase,
        NormalSpineCtx {
            can_variable_jump,
            ..NormalSpineCtx::bare(on_ground)
        },
        input,
        1.0 / 60.0,
        frame,
        classic_tuning().axis_swept_params(),
    );
}

/// The phased-gravity law resolves its arc in the INTEGRATOR, not in
/// `apply_jump_release`, so it has to re-apply the same `variable_jump`
/// capability gate the velocity-cut law gets for free. Without this, selecting
/// `PhasedGravity` silently hands out variable jump height to a body whose grant
/// list never granted it — the law would be mis-composable for every character
/// but the one it was written for.
#[test]
fn a_body_without_variable_jump_cannot_shorten_a_phased_arc() {
    let frame = MotionFrame::from_direction(Vec2::Y, 2250.0);
    let released = jump_edge(false, false, true);

    // Same state, same released input; only the capability differs.
    let mut with_capability = Vec2::new(0.0, -420.0);
    let mut phase = PhasedJumpState::default();
    phase.begin(0);
    step_spine_with_variable_jump(
        &mut with_capability,
        &mut phase,
        released,
        frame,
        false,
        true,
    );
    assert!(
        phase.hold_cancelled,
        "a variable-jump body ends its weak-ascent phase on release"
    );
    assert!(
        (with_capability.y - (-382.5)).abs() < 1.0e-4,
        "and takes a full-gravity tick"
    );

    let mut without_capability = Vec2::new(0.0, -420.0);
    let mut phase = PhasedJumpState::default();
    phase.begin(0);
    step_spine_with_variable_jump(
        &mut without_capability,
        &mut phase,
        released,
        frame,
        false,
        false,
    );
    assert!(
        !phase.hold_cancelled,
        "a body denied variable_jump commits to the whole arc; the button cannot end it"
    );
    assert!(
        (without_capability.y - (-412.5)).abs() < 1.0e-4,
        "it keeps the 20% held-ascent gravity until the apex threshold, \
         not the 100% release gravity"
    );
}

/// The apex threshold — never the button — is what ends a committed arc, so a
/// fixed-height jump still lands instead of floating forever.
#[test]
fn a_committed_arc_still_ends_at_the_apex_threshold() {
    let frame = MotionFrame::from_direction(Vec2::Y, 2250.0);
    let mut vel = Vec2::new(0.0, -200.0); // already below held_phase_min_upward_speed
    let mut phase = PhasedJumpState::default();
    phase.begin(0);
    step_spine_with_variable_jump(
        &mut vel,
        &mut phase,
        jump_held_input(0.0),
        frame,
        false,
        false,
    );
    assert!(
        phase.hold_cancelled,
        "below the apex threshold the weak phase ends even while held"
    );
    assert!(
        (vel.y - (-162.5)).abs() < 1.0e-4,
        "and full gravity brings it down"
    );
}

#[test]
fn classic_speed_bands_match_the_converted_reference_table() {
    let AxisJumpLaw::PhasedGravity(params) = classic_tuning().jump_law else {
        panic!("classic profile must use phased gravity");
    };
    let base = classic_tuning().jump_speed;
    assert_eq!(params.band_for_side_speed(0.0), 0);
    assert_eq!(params.band_for_side_speed(119.999), 0);
    assert_eq!(params.band_for_side_speed(120.0), 1);
    assert_eq!(params.band_for_side_speed(187.5), 2);
    assert_eq!(params.band_for_side_speed(210.0), 3);
    assert_eq!(params.launch_speed_for_band(base, 0), 420.0);
    assert_eq!(params.launch_speed_for_band(base, 1), 435.0);
    assert_eq!(params.launch_speed_for_band(base, 2), 450.0);
    assert_eq!(params.launch_speed_for_band(base, 3), 480.0);
    // The top band is reachable by RUNNING, not only by an external fling: a
    // running jump is the highest jump, as in the classic games this converts.
    assert!(params.top_band_speed() < classic_tuning().max_run_speed);
    assert_eq!(
        params.band_for_side_speed(classic_tuning().max_run_speed),
        3
    );
}

#[test]
fn ground_jump_selects_and_latches_the_local_speed_band() {
    let world = test_world();
    let abilities = AbilitySet::compose(&[AbilityGrant::RunJump]);
    let mut scratch = BodyClusterScratch::new_with_abilities(world.spawn, abilities);
    scratch.ground.on_ground = true;
    scratch.kinematics.vel.x = 187.5;
    let tuning = TestTuning::from(classic_tuning());

    update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        jump_edge(true, true, false),
        1.0 / 60.0,
        tuning,
    );

    assert_eq!(scratch.axis().phased_jump.launch_band, 2);
    assert!(scratch.axis().phased_jump.active);
    assert!(
        (scratch.kinematics.vel.y - (-442.5)).abs() < 1.0e-3,
        "450 px/s band launch plus one 7.5 px/s held-gravity tick"
    );
}

#[test]
fn full_kernel_release_changes_gravity_phase_without_an_impulse_cut() {
    let world = test_world();
    let abilities = AbilitySet::compose(&[AbilityGrant::RunJump]);
    let mut scratch = BodyClusterScratch::new_with_abilities(world.spawn, abilities);
    scratch.ground.on_ground = true;
    let tuning = TestTuning::from(classic_tuning());

    update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        jump_edge(true, true, false),
        1.0 / 60.0,
        tuning,
    );
    let before_release = scratch.kinematics.vel.y;
    update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        jump_edge(false, false, true),
        1.0 / 60.0,
        tuning,
    );

    assert!(scratch.axis().phased_jump.hold_cancelled);
    assert!(
        (scratch.kinematics.vel.y - (before_release + 37.5)).abs() < 1.0e-3,
        "release adds one full-gravity tick and does not rewrite velocity"
    );
}

#[test]
fn neutral_air_preserves_classic_horizontal_momentum() {
    let frame = MotionFrame::from_direction(Vec2::Y, 0.0);
    let mut vel = Vec2::new(250.0, -100.0);
    let mut phase = PhasedJumpState::default();
    step_spine(&mut vel, &mut phase, InputState::default(), frame, false);
    assert_eq!(vel.x, 250.0, "neutral air must not apply stop assist");
}

#[test]
fn neutral_ground_coast_stops_cleanly_without_touching_air_momentum() {
    let frame = MotionFrame::from_direction(Vec2::Y, 0.0);
    let mut vel = Vec2::new(300.0, 0.0);
    let mut phase = PhasedJumpState::default();
    step_spine(&mut vel, &mut phase, InputState::default(), frame, true);
    assert!(
        (vel.x - 280.0).abs() < 1.0e-5,
        "1200 px/s² ground coast removes 20 px/s per 60 Hz tick"
    );
}

#[test]
fn ground_reversal_uses_the_stronger_skid_rate() {
    let frame = MotionFrame::from_direction(Vec2::Y, 0.0);
    let mut vel = Vec2::new(300.0, 0.0);
    let mut phase = PhasedJumpState::default();
    step_spine(
        &mut vel,
        &mut phase,
        InputState::with_axes(-1.0, 0.0),
        frame,
        true,
    );
    assert!((vel.x - 275.0).abs() < 1.0e-5, "1500 px/s² skid at 60 Hz");
}

#[test]
fn held_jump_scales_only_gravity_not_external_force() {
    let basis = AccelerationFrame::new(Vec2::Y);
    let frame =
        MotionFrame::with_accelerations(basis, Vec2::new(0.0, 1000.0), Vec2::new(300.0, 0.0));
    let mut vel = Vec2::new(10.0, -400.0);
    let mut phase = PhasedJumpState::default();
    phase.begin(0);
    step_spine(&mut vel, &mut phase, jump_held_input(0.0), frame, false);

    assert!((vel.x - 15.0).abs() < 1.0e-5, "wind remains full strength");
    assert!(
        (vel.y - (-396.666_66)).abs() < 1.0e-3,
        "held ascent receives 20% of gravity"
    );
}

#[test]
fn release_latches_strong_gravity_without_cutting_velocity() {
    let frame = MotionFrame::from_direction(Vec2::Y, 2250.0);
    let mut vel = Vec2::new(0.0, -420.0);
    let mut phase = PhasedJumpState::default();
    phase.begin(0);
    phase.cancel_hold();
    step_spine(&mut vel, &mut phase, InputState::default(), frame, false);

    assert!(phase.hold_cancelled);
    assert!(
        (vel.y - (-382.5)).abs() < 1.0e-5,
        "release applies one frame of full gravity, not a 0.54 velocity cut"
    );
}

#[test]
fn active_jump_reinterprets_motion_in_a_new_gravity_frame() {
    let mut vel = Vec2::new(-450.0, -450.0);
    let mut phase = PhasedJumpState::default();
    phase.begin(3);

    step_spine(
        &mut vel,
        &mut phase,
        jump_held_input(0.0),
        MotionFrame::from_direction(Vec2::Y, 2250.0),
        false,
    );
    assert!((vel.y - (-442.5)).abs() < 1.0e-5);

    // The zone turns local down from +Y to +X. No launch direction is cached:
    // the existing world velocity is projected into the new frame, the band is
    // retained, and weak held-ascent gravity follows the new local down.
    step_spine(
        &mut vel,
        &mut phase,
        jump_held_input(0.0),
        MotionFrame::from_direction(Vec2::X, 2250.0),
        false,
    );

    assert_eq!(phase.launch_band, 3);
    assert!(!phase.hold_cancelled);
    assert!((vel.x - (-442.5)).abs() < 1.0e-5);
    assert!((vel.y - (-442.5)).abs() < 1.0e-5);
}

#[test]
fn classic_laws_are_c4_covariant() {
    let downs = [Vec2::Y, -Vec2::Y, Vec2::X, -Vec2::X];
    let mut reference = None;
    for down in downs {
        let basis = AccelerationFrame::new(down);
        let frame = MotionFrame::with_accelerations(basis, down * 2250.0, basis.side * 120.0);
        let mut vel = basis.to_world(Vec2::new(200.0, -450.0));
        let mut phase = PhasedJumpState::default();
        phase.begin(2);
        step_spine(&mut vel, &mut phase, jump_held_input(1.0), frame, false);
        let local = basis.to_local(vel);
        match reference {
            None => reference = Some(local),
            Some(expected) => {
                assert!((local.x - expected.x).abs() < 1.0e-4);
                assert!((local.y - expected.y).abs() < 1.0e-4);
            }
        }
    }
}

/// ⭐⭐ A TIMED GRAVITY MODIFIER COMPOSES WITH THE KERNEL'S OTHER SCALES AND
/// EXPIRES ON THE MOVEMENT DOMAIN'S OWN CLOCK.
///
/// The Parasol row's shape: a move ASKS for a locomotion regime and never
/// becomes the authority for it. What that buys is here — the caller writes a
/// scale and a duration, and nothing outside this crate ends it.
///
/// ⛔⛔ THE THIRD ASSERTION IS THE ONE THAT MATTERS AND IT IS ABOUT ZERO. The
/// neutral value of a multiplier is `1.0`; the neutral value of an `f32` is
/// `0.0`. A default body, a rollback restore of an untouched frame and a bare
/// enemy all present a zeroed struct, and if the scale were read
/// unconditionally every one of them would fall under NO gravity — a weightless
/// roster, from a field nobody set.
#[test]
fn a_timed_gravity_modifier_scales_the_fall_and_then_gives_it_back() {
    let frame = MotionFrame::from_direction(Vec2::Y, 2250.0);
    let dt = 1.0 / 60.0;

    let fall_after = |modifier: f32| -> f32 {
        let mut vel = Vec2::ZERO;
        let mut phase = PhasedJumpState::default();
        let mut fast_falling = false;
        let mut gliding = false;
        let mut carried_run = 0.0;
        let mut carried_hold = 0.0;
        integrate_normal_spine(
            &mut vel,
            &mut fast_falling,
            &mut gliding,
            &mut carried_run,
            &mut carried_hold,
            &mut phase,
            NormalSpineCtx {
                gravity_modifier: modifier,
                ..NormalSpineCtx::bare(false)
            },
            InputState::default(),
            dt,
            frame,
            classic_tuning().axis_swept_params(),
        );
        vel.y
    };

    let full = fall_after(1.0);
    assert!(
        full > 0.0,
        "poison: an UNMODIFIED body did not accelerate downward at all, so \
         every comparison below is between two zeroes"
    );
    let halved = fall_after(0.5);
    assert!(
        (halved - full * 0.5).abs() < 1e-3,
        "a 0.5 modifier did not halve the fall ({halved} vs {full}) — the \
         factor is not reaching the product"
    );

    // ── the state half: the timer gates the scale, and the movement domain
    // spends it.
    let mut state = crate::movement::AxisManeuverState::default();
    assert_eq!(
        state.gravity_modifier(),
        1.0,
        "a DEFAULT body reported a gravity modifier of {} — a zeroed struct \
         must read as NEUTRAL, or every untouched body in the game is weightless",
        state.gravity_modifier()
    );

    state.set_gravity_modifier(0.35, 0.5);
    assert_eq!(
        state.gravity_modifier(),
        0.35,
        "a move asked for a modifier and the body did not take it"
    );

    // ⛔ A NON-POSITIVE DURATION CLEARS RATHER THAN ARMS. A move that asks for
    // zero seconds of float is asking for none, and the alternative — storing
    // the scale with a dead timer — is the state this design exists to make
    // unrepresentable.
    state.set_gravity_modifier(0.0, 0.0);
    assert_eq!(
        state.gravity_modifier(),
        1.0,
        "a zero-second request armed a ZERO-GRAVITY modifier instead of clearing"
    );
}
