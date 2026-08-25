//! Pure ability-flag sanity: a flag set to `false` must keep the
//! corresponding op out of the FrameEvents / state.

use super::super::*;
use super::{step_scratch, test_world};
use crate::AbilitySet;
use crate::body_clusters::BodyClusterScratch;

fn scratch_with(abilities: AbilitySet, spawn: bevy_math::Vec2) -> BodyClusterScratch {
    BodyClusterScratch::new_with_abilities(spawn, abilities)
}

/// A BODY THAT CANNOT JUMP DOES NOT JUMP WHEN THE BUTTON IS PRESSED.
///
/// Its gate is one `&&` in `apply_intent`, and one `&&` is exactly the kind of thing a refactor
/// drops without a compiler error.
///
/// The content half is pinned in `puppy_slug_forced_seat.rs`: the shipped `npc_puppy_slug` authors
/// `move_horizontal` and nothing else, so its seated mask says `jump: false`. That assertion is
/// only worth having if the engine HONOURS the mask, and this is the half that says so. Neither
/// test is the claim on its own.
///
/// both directions, and the grounded control is not decoration: a gate
/// that refused every jump would satisfy the first half perfectly, and a body
/// that cannot jump is indistinguishable from a body nobody asked to jump
/// unless the same fixture jumps when it is allowed to.
#[test]
fn jump_ability_controls_the_ground_jump() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.jump = false;

    let mut scratch = scratch_with(abilities, world.spawn);
    scratch.ground.on_ground = true;
    let events = step_scratch(&world, &mut scratch, jump_press());
    assert!(
        !events.operations.contains(&MovementOp::Jump),
        "a body whose ability set denies `jump` jumped anyway: {:?}",
        events.operations
    );
    // the OP is the intent and the VELOCITY is the consequence; asserting only
    // the op would pass on an engine that emitted nothing and launched the body
    // anyway. Authored geometry is y-down, so rising is negative y.
    assert!(
        scratch.kinematics.vel.y >= 0.0,
        "no jump op fired and the body rose anyway ({} px/s), so something          other than the gate is launching it",
        scratch.kinematics.vel.y
    );

    abilities.jump = true;
    let mut scratch = scratch_with(abilities, world.spawn);
    scratch.ground.on_ground = true;
    let events = step_scratch(&world, &mut scratch, jump_press());
    assert!(
        events.operations.contains(&MovementOp::Jump),
        "the same fixture with `jump: true` did not jump either, so the half          above is measuring a broken fixture rather than the gate: {:?}",
        events.operations
    );
    assert!(
        scratch.kinematics.vel.y < 0.0,
        "the jump op fired and the body did not rise ({} px/s)",
        scratch.kinematics.vel.y
    );
}

/// One frame of the jump button going down — the edge every gate above reads.
fn jump_press() -> InputState {
    InputState {
        movement: crate::ActionEdges::EMPTY.with(
            crate::MovementAction::Jump,
            crate::Edge {
                pressed: true,
                held: true,
                released: false,
            },
        ),
        ..Default::default()
    }
}

#[test]
fn double_jump_ability_controls_air_jump() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.double_jump = false;
    let mut scratch = scratch_with(abilities, world.spawn);
    scratch.ground.on_ground = false;
    scratch.axis_mut().coyote_timer = 0.0;
    scratch.jump.air_jumps_available = 0;
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Jump,
                crate::Edge {
                    pressed: true,
                    held: false,
                    released: false,
                },
            ),
            ..Default::default()
        },
    );
    assert!(!events.operations.contains(&MovementOp::DoubleJump));

    abilities.double_jump = true;
    let mut scratch = scratch_with(abilities, world.spawn);
    scratch.ground.on_ground = false;
    scratch.axis_mut().coyote_timer = 0.0;
    scratch.jump.air_jumps_available = 1;
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Jump,
                crate::Edge {
                    pressed: true,
                    held: false,
                    released: false,
                },
            ),
            ..Default::default()
        },
    );
    assert!(events.operations.contains(&MovementOp::DoubleJump));
}

#[test]
fn double_dash_ability_controls_dash_charges() {
    let world = test_world();
    let mut single_dash = AbilitySet::sandbox_all();
    single_dash.double_dash = false;
    let scratch = scratch_with(single_dash, world.spawn);
    assert_eq!(scratch.dash.charges_available, 1);

    let scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    assert_eq!(scratch.dash.charges_available, 2);
}

#[test]
fn wall_climb_requires_wall_cling() {
    let mut abilities = AbilitySet::sandbox_all();
    abilities.wall_cling = false;
    assert!(
        abilities
            .compatibility_warnings()
            .iter()
            .any(|w| w.contains("wall_climb"))
    );
}

/// A BODY THAT CANNOT DASH STILL RUNS, and the dash attack's whole
/// reachability rests on that being two different facts.
///
/// Every test of the selector passed, because each told the selector the body was dashing; none
/// could ask whether a fighter ever is.
///
/// the second assertion is the poison, not decoration. A `running` that
/// was merely an alias for `dashing` would satisfy "it runs" on a dash-capable
/// ⭐⭐ A CROUCH COSTS YOU YOUR MOBILITY — and by default it costs nothing.
///
/// Measured 2026-08-24 and it was the second half that was wrong: the movement
/// kernel read `BodyMode` only for `Climbing`, so a crouching body ran at full
/// speed while KEEPING the smaller hurtbox and the shortened launch. A free
/// defensive win is the inverse of the genre's trade.
///
/// ⛔⛔ AND THE DEFAULT MUST STAY FREE, which is the other half of this test.
/// The field arrived on every `MovementTuning` in the engine; a default that
/// slowed anybody would change every Ambition room to buy a Smash rule.
#[test]
fn a_crouch_costs_speed_only_where_a_ruleset_asks_for_it() {
    let world = test_world();
    let settle = |crouch_frac: f32| {
        let mut tuning = super::TEST_TUNING;
        tuning.crouch_speed_frac = crouch_frac;
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
        scratch.ground.on_ground = true;
        scratch.body_mode.body_mode = crate::player_state::BodyMode::Crouching;
        let hold = InputState {
            axes: crate::reference_frame::LocalAxes::new(1.0, 0.0),
            ..InputState::default()
        };
        for _ in 0..80 {
            super::update_player_with_tuning_scratch(
                &world,
                &mut scratch,
                hold,
                1.0 / 60.0,
                tuning,
            );
        }
        scratch.kinematics.vel.x
    };

    let free = settle(1.0);
    assert!(
        free > 100.0,
        "the ENGINE default slowed a crouching body ({free:.1} px/s) — every \
         Ambition room just changed to buy a platform fighter's rule"
    );
    let planted = settle(0.0);
    assert!(
        planted.abs() < 1.0,
        "a ruleset that plants a crouching fighter still let it travel at \
         {planted:.1} px/s, so crouch keeps its smaller hurtbox and its \
         shortened launch for free"
    );
}

/// ⭐⭐ A LIGHT TILT WALKS: it settles at a LOWER top speed and is not a run.
///
/// The genre's neutral is built on the difference, and both halves of it are
/// here already — the stick magnitude scales the TARGET speed (not just the
/// acceleration toward one shared cap), and `run_commit_frac` cuts the result
/// into a walk and a run that the move selector reads.
///
/// ⛔⛔ WRITTEN BECAUSE THE PARITY INVENTORY SAID THIS WAS ABSENT — *"treating
/// all grounded locomotion as one continuum"* — and measuring found otherwise.
/// It IS a continuum, and the gait line already cuts it. What is genuinely
/// missing is narrower: a DIGITAL input can only ever say 1.0, so a keyboard
/// fighter cannot walk. That is an input-mapping gap, not a locomotion one, and
/// this test is what keeps the locomotion half from being rebuilt for it.
///
/// ⭐ THE ORDERING IS THE ASSERTION, not the numbers: a test on `135.0` would
/// pin this body's tuning, and every fighter authors its own.
#[test]
fn a_light_tilt_walks_and_a_full_one_runs() {
    let world = test_world();
    let settle = |tilt: f32| {
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
        scratch.ground.on_ground = true;
        let hold = InputState {
            axes: crate::reference_frame::LocalAxes::new(tilt, 0.0),
            ..InputState::default()
        };
        // Long enough to reach the tilt's cap rather than measuring accel.
        for _ in 0..80 {
            step_scratch(&world, &mut scratch, hold);
        }
        (
            scratch.kinematics.vel.x,
            crate::movement::BodyMotionFacts::from_model(&scratch.model).running,
        )
    };

    let (walk_speed, walking_is_a_run) = settle(0.5);
    let (run_speed, running_is_a_run) = settle(1.0);

    assert!(
        walk_speed > 0.0,
        "a half tilt moved the body nowhere, so what follows compares a walk to \
         a standstill"
    );
    assert!(
        run_speed > walk_speed * 1.2,
        "a half tilt settled at {walk_speed:.1} px/s and a full one at \
         {run_speed:.1} — the stick is scaling ACCELERATION toward one shared \
         cap rather than the cap itself, which is a single gait wearing two names"
    );
    assert!(
        !walking_is_a_run,
        "a half tilt reported as RUNNING at {walk_speed:.1} px/s, so the move \
         selector answers a walking Attack press with the dash attack"
    );
    assert!(
        running_is_a_run,
        "a full tilt never reached the run gait at {run_speed:.1} px/s, so this \
         body has no run at all and the contrast above is vacuous"
    );
}

/// body and fail here; a `running` wired to any ability bit would fail here too.
/// The body below has NO dash ability and reaches the gait anyway.
#[test]
fn the_run_gait_does_not_depend_on_the_dash_ability() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.dash = false;

    let mut scratch = scratch_with(abilities, world.spawn);
    scratch.ground.on_ground = true;
    let hold_right = InputState {
        axes: crate::reference_frame::LocalAxes::new(1.0, 0.0),
        ..InputState::default()
    };
    // Long enough for `run_accel` to carry an ordinary body past the gait line.
    for _ in 0..40 {
        step_scratch(&world, &mut scratch, hold_right);
    }
    let facts = crate::movement::BodyMotionFacts::from_model(&scratch.model);
    assert!(
        facts.running,
        "a body holding a direction on the floor for 40 ticks never reached the \
         run gait (travelling {} px/s)",
        scratch.kinematics.vel.x,
    );
    assert!(
        !facts.dashing,
        "the body has no dash ability, so a `dashing` fact here means `running` \
         is reading the traversal dash after all",
    );

    // and the gait has to be able to be FALSE, or the assertion above is a
    // fact that is simply always on. A body released from the stick brakes back
    // under the line.
    for _ in 0..40 {
        step_scratch(&world, &mut scratch, InputState::default());
    }
    assert!(
        !crate::movement::BodyMotionFacts::from_model(&scratch.model).running,
        "a body that let go of the stick is still reported as running \
         (travelling {} px/s)",
        scratch.kinematics.vel.x,
    );
}

/// A RUN CANCELS INTO A CROUCH IN A HANDFUL OF FRAMES.
///
/// The neighbouring `a_crouch_costs_speed_only_where_a_ruleset_asks_for_it`
/// starts a body ALREADY crouching, so it pins the steady state and says
/// nothing about the TRANSITION — and the transition is the mechanic here:
/// crouching is how a platform fighter kills run momentum on purpose.
///
/// Measured 2026-08-25 at 270px/s top speed: 183 → 97 → 10 → 0, stopped on the
/// fourth tick. ⛔ THIS IS WHAT THE CAP BUYS. A version that scaled the
/// ACCELERATION instead would leave the body coasting at run speed for a long
/// time and still pass the steady-state test, because it eventually arrives.
#[test]
fn crouching_out_of_a_run_kills_the_momentum_within_a_few_frames() {
    let world = test_world();
    let mut tuning = super::TEST_TUNING;
    tuning.crouch_speed_frac = 0.0;
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.ground.on_ground = true;
    let hold = InputState {
        axes: crate::reference_frame::LocalAxes::new(1.0, 0.0),
        ..InputState::default()
    };
    for _ in 0..80 {
        super::update_player_with_tuning_scratch(&world, &mut scratch, hold, 1.0 / 60.0, tuning);
    }
    let top = scratch.kinematics.vel.x;
    assert!(
        top > 200.0,
        "the fixture never reached a real run ({top:.0} px/s), so the cancel below cancels nothing"
    );

    // Crouch, WITHOUT letting go of the direction — a cancel you have to hold
    // through, not a body that simply stopped being asked to move.
    scratch.body_mode.body_mode = crate::player_state::BodyMode::Crouching;
    let mut stopped_after = None;
    for t in 0..60 {
        super::update_player_with_tuning_scratch(&world, &mut scratch, hold, 1.0 / 60.0, tuning);
        if stopped_after.is_none() && scratch.kinematics.vel.x.abs() < 1.0 {
            stopped_after = Some(t + 1);
        }
    }
    let ticks = stopped_after.expect("a crouch that never stops a run is not a cancel");
    assert!(
        ticks <= 8,
        "a crouch took {ticks} ticks to kill a run — that is a slow-down, not a cancel"
    );
}
