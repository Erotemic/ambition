//! Pure ability-flag sanity: a flag set to `false` must keep the
//! corresponding op out of the FrameEvents / state.

use super::super::*;
use super::{step_scratch, test_world};
use crate::body_clusters::BodyClusterScratch;
use crate::AbilitySet;

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
    assert!(abilities
        .compatibility_warnings()
        .iter()
        .any(|w| w.contains("wall_climb")));
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

/// THE INITIAL DASH: at speed on frame one, free to reverse, and gone again.
///
/// Five arms. The fourth is the one the mechanic exists for — reversing inside
/// the window is INSTANT, which is what makes a dash-dance a conversation
/// rather than a commitment — and the fifth is what keeps every other world in
/// this repo exactly where it was.
#[test]
fn an_initial_dash_is_at_speed_at_once_and_may_still_reverse() {
    let world = test_world();
    let dashing = || {
        let mut tuning = super::TEST_TUNING;
        tuning.initial_dash_time = 14.0 / 60.0;
        tuning.initial_dash_speed = 0.0; // inherit the run speed
        tuning
    };
    let hold = |x: f32| InputState {
        axes: crate::reference_frame::LocalAxes::new(x, 0.0),
        ..InputState::default()
    };
    let settled = |tuning| {
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
        scratch.ground.on_ground = true;
        // ⛔ LAND FIRST. `on_ground` is re-derived every step, so setting it
        // above holds only until the body's real height is consulted — and an
        // airborne body has no dash phase at all, which reads exactly like a
        // ramp. The neutral input also leaves `prev_steer_dir` a real zero, so
        // the first press below is a genuine CHANGE.
        for _ in 0..40 {
            super::update_player_with_tuning_scratch(
                &world,
                &mut scratch,
                InputState::default(),
                1.0 / 60.0,
                tuning,
            );
        }
        assert!(
            scratch.ground.on_ground,
            "the fixture never reached the floor, so every arm below measures an airborne body"
        );
        scratch
    };
    let top = super::TEST_TUNING.params().locomotion.max_run_speed;

    // ARM 1 — AT SPEED ON FRAME ONE. A ramp would read a fraction of this.
    let mut scratch = settled(dashing());
    super::update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        hold(1.0),
        1.0 / 60.0,
        dashing(),
    );
    assert!(
        (scratch.kinematics.vel.x - top).abs() < 1.0,
        "the initial dash ramped instead of starting at speed: {} vs {top}",
        scratch.kinematics.vel.x
    );
    assert!(
        scratch.axis().initial_dash_timer > 0.0,
        "the phase did not start"
    );

    // ARM 2 — AND IT ENDS. Hold the same direction and the window closes.
    for _ in 0..20 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            hold(1.0),
            1.0 / 60.0,
            dashing(),
        );
    }
    assert!(
        scratch.axis().initial_dash_timer <= 0.0,
        "a held direction never let the dash phase expire — it would never become a run"
    );

    // ARM 3 — A HELD DIRECTION DOES NOT RE-TRIGGER, which is what arm 2 relies
    // on and is worth its own assertion: the entry rule is a CHANGE.
    assert_eq!(
        scratch.axis().initial_dash_dir,
        0.0,
        "the phase re-armed itself under a held direction"
    );

    // ARM 4 — REVERSING IS FREE AND IMMEDIATE. This is the mechanic.
    super::update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        hold(-1.0),
        1.0 / 60.0,
        dashing(),
    );
    assert!(
        (scratch.kinematics.vel.x + top).abs() < 1.0,
        "reversing inside the dash was not instant: {} (wanted {})",
        scratch.kinematics.vel.x,
        -top
    );

    // ARM 5 — A WORLD THAT DECLARES NO PHASE IS UNTOUCHED, which is every
    // Ambition room. Same press, engine tuning: it ACCELERATES.
    let mut plain = settled(super::TEST_TUNING);
    super::update_player_with_tuning_scratch(
        &world,
        &mut plain,
        hold(1.0),
        1.0 / 60.0,
        super::TEST_TUNING,
    );
    assert!(
        plain.kinematics.vel.x < top * 0.5,
        "an undeclared world got the dash phase anyway: {} is already most of {top}",
        plain.kinematics.vel.x
    );
    assert_eq!(
        plain.axis().initial_dash_timer,
        0.0,
        "an undeclared world armed the phase"
    );
}

/// A DASH MAY ONLY SPEED YOU UP — it must never eat a launch.
///
/// ⛔⛔ THIS IS THE BUG THE PHASE SHIPPED WITH, and an emergent match test is
/// what caught it: with the dash overwriting the along-surface velocity, a
/// grounded fighter launched at 1313px/s while holding a direction came out at
/// 270 (holding toward) or 0 (holding away), so nobody was ever knocked off the
/// stage and a one-stock match never ended
/// (`the_stage_kills::a_second_match_on_the_same_stage_counts_in_and_ends`).
///
/// ⭐ SAME CLASS AS THE GROUND ROLL'S SHED: a maneuver reaching into a shared
/// velocity it does not own, and the same asymmetry answers it — a body already
/// travelling faster than the dash is carrying somebody else's speed, so the
/// dash leaves it alone.
#[test]
fn an_initial_dash_never_slows_a_body_that_was_launched() {
    let world = test_world();
    let mut tuning = super::TEST_TUNING;
    tuning.initial_dash_time = 14.0 / 60.0;
    let launched_peak = |hold_dir: f32| -> f32 {
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
        scratch.ground.on_ground = true;
        for _ in 0..40 {
            super::update_player_with_tuning_scratch(
                &world,
                &mut scratch,
                InputState::default(),
                1.0 / 60.0,
                tuning,
            );
        }
        assert!(
            scratch.ground.on_ground,
            "the fixture never landed, so this measures an airborne body"
        );
        scratch.flight.pending_launch = crate::Vec2::new(1400.0, -60.0);
        let hold = InputState {
            axes: crate::reference_frame::LocalAxes::new(hold_dir, 0.0),
            ..InputState::default()
        };
        let mut peak: f32 = 0.0;
        for _ in 0..10 {
            super::update_player_with_tuning_scratch(
                &world,
                &mut scratch,
                hold,
                1.0 / 60.0,
                tuning,
            );
            peak = peak.max(scratch.kinematics.vel.x);
        }
        peak
    };
    let run = tuning.params().locomotion.max_run_speed;
    // Holding INTO the launch and AWAY from it both keep it: the dash is not
    // allowed to decide either way. Away is the arm that actually failed.
    for dir in [1.0f32, -1.0] {
        let peak = launched_peak(dir);
        assert!(
            peak > run * 2.0,
            "a dash ate a launch while the body held {dir}: peaked at {peak:.0}, \
             which is not much more than the {run:.0} run speed the dash wanted"
        );
    }
}

/// FOXTROT AND DASH DANCE, DRIVEN END TO END.
///
/// Both are parity rows in their own right, and both should fall out of the
/// initial dash's entry rule — a direction CHANGE — without a line of code
/// each. This test is what turns "should" into "does": a re-tap through
/// neutral re-arms the phase (foxtrot), and alternating directions re-arms it
/// each time while the body stays put (dash dance).
#[test]
fn the_foxtrot_and_the_dash_dance_fall_out_of_the_same_edge() {
    let world = test_world();
    let mut tuning = super::TEST_TUNING;
    tuning.initial_dash_time = 14.0 / 60.0;
    let hold = |x: f32| InputState {
        axes: crate::reference_frame::LocalAxes::new(x, 0.0),
        ..InputState::default()
    };
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.ground.on_ground = true;
    for _ in 0..40 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(scratch.ground.on_ground, "the fixture never landed");
    let step = |input: InputState, scratch: &mut crate::body_clusters::BodyClusterScratch| {
        super::update_player_with_tuning_scratch(&world, scratch, input, 1.0 / 60.0, tuning);
    };

    // THE FOXTROT — tap, let the phase run out, tap again through neutral.
    step(hold(1.0), &mut scratch);
    assert!(
        scratch.axis().initial_dash_timer > 0.0,
        "the first tap did not dash"
    );
    for _ in 0..20 {
        step(hold(1.0), &mut scratch);
    }
    assert!(
        scratch.axis().initial_dash_timer <= 0.0,
        "the phase never expired under a held direction"
    );
    step(InputState::default(), &mut scratch);
    step(hold(1.0), &mut scratch);
    assert!(
        scratch.axis().initial_dash_timer > 0.0,
        "a RE-TAP of the same direction did not re-arm the phase — no foxtrot"
    );

    // THE DASH DANCE — alternate, and the body should stay roughly where it is
    // rather than committing to either direction.
    let anchor = scratch.kinematics.pos.x;
    let mut rearms = 0;
    for t in 0..24 {
        let was = scratch.axis().initial_dash_timer;
        step(hold(if t % 4 < 2 { 1.0 } else { -1.0 }), &mut scratch);
        if scratch.axis().initial_dash_timer > was {
            rearms += 1;
        }
    }
    assert!(
        rearms >= 4,
        "alternating directions re-armed the phase only {rearms} times in 24 ticks — \
         a dash dance is exactly that re-arm"
    );
    let drift = (scratch.kinematics.pos.x - anchor).abs();
    assert!(
        drift < tuning.params().locomotion.max_run_speed * 0.25,
        "a dash dance travelled {drift:.0}px, which is a run rather than a dance"
    );
}

/// REVERSING OUT OF A RUN COSTS A TURNAROUND; REVERSING INSIDE A DASH DOES NOT.
///
/// The pair is the mechanic. Either half alone is just a speed: a game where
/// every reversal is free has no ground game, and one where every reversal is
/// slow has no dash dance. The third arm is the one that would catch a rule
/// that charged the phase too eagerly.
#[test]
fn a_run_pays_to_turn_around_and_a_dash_does_not() {
    let world = test_world();
    let mut tuning = super::TEST_TUNING;
    tuning.initial_dash_time = 14.0 / 60.0;
    tuning.turnaround_time = 7.0 / 60.0;
    let hold = |x: f32| InputState {
        axes: crate::reference_frame::LocalAxes::new(x, 0.0),
        ..InputState::default()
    };
    let landed = || {
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
        scratch.ground.on_ground = true;
        for _ in 0..40 {
            super::update_player_with_tuning_scratch(
                &world,
                &mut scratch,
                InputState::default(),
                1.0 / 60.0,
                tuning,
            );
        }
        assert!(scratch.ground.on_ground, "the fixture never landed");
        scratch
    };

    // ARM 1 — INSIDE THE DASH, TURNING IS FREE. Press right, reverse on the
    // very next tick: facing flips at once and no turnaround is owed.
    let mut dancing = landed();
    super::update_player_with_tuning_scratch(&world, &mut dancing, hold(1.0), 1.0 / 60.0, tuning);
    super::update_player_with_tuning_scratch(&world, &mut dancing, hold(-1.0), 1.0 / 60.0, tuning);
    assert_eq!(
        dancing.kinematics.facing, -1.0,
        "reversing inside the dash window did not flip facing immediately"
    );
    assert!(
        dancing.axis().turnaround_timer <= 0.0,
        "reversing inside the dash charged a turnaround — that deletes dash-dancing"
    );

    // ARM 2 — OUT OF A COMMITTED RUN, IT COSTS. Run until the gait line is
    // crossed, then reverse: facing holds for the authored window.
    let mut running = landed();
    for _ in 0..60 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut running,
            hold(1.0),
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(
        running.axis().running,
        "the fixture never committed to a run, so there is nothing to turn out of"
    );
    assert_eq!(running.kinematics.facing, 1.0);
    super::update_player_with_tuning_scratch(&world, &mut running, hold(-1.0), 1.0 / 60.0, tuning);
    assert!(
        running.axis().turnaround_timer > 0.0,
        "reversing out of a run owed no turnaround"
    );
    assert_eq!(
        running.kinematics.facing, 1.0,
        "the body faced the other way on the same tick it asked to — the turnaround is free"
    );
    // ...and it does arrive.
    for _ in 0..12 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut running,
            hold(-1.0),
            1.0 / 60.0,
            tuning,
        );
    }
    assert_eq!(
        running.kinematics.facing, -1.0,
        "the turnaround never completed — the body is stuck facing the wrong way"
    );

    // ARM 3 — A WORLD THAT DECLARES NO TURNAROUND FLIPS AT ONCE, which is every
    // Ambition room. Same committed run, engine tuning.
    let mut plain = {
        let mut t = super::TEST_TUNING;
        t.initial_dash_time = 14.0 / 60.0;
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
        scratch.ground.on_ground = true;
        for _ in 0..40 {
            super::update_player_with_tuning_scratch(
                &world,
                &mut scratch,
                InputState::default(),
                1.0 / 60.0,
                t,
            );
        }
        for _ in 0..60 {
            super::update_player_with_tuning_scratch(
                &world,
                &mut scratch,
                hold(1.0),
                1.0 / 60.0,
                t,
            );
        }
        super::update_player_with_tuning_scratch(&world, &mut scratch, hold(-1.0), 1.0 / 60.0, t);
        scratch
    };
    assert_eq!(
        plain.kinematics.facing, -1.0,
        "a world declaring no turnaround was charged one anyway"
    );
    assert!(plain.axis().turnaround_timer <= 0.0);
    let _ = &mut plain;
}

/// THE REVERSE AERIAL RUSH EMERGES — turn, jump, and your back is pointed at
/// where you came from while your momentum still carries you there.
///
/// ⛔ NO RAR STATE, which the inventory row rules out by name. What makes it
/// work is that a turnaround RESOLVES when the body leaves the floor instead of
/// being abandoned: measured before that rule, a fighter who jumped mid-turn
/// stayed facing its old way forever, because an airborne body may not turn at
/// all.
#[test]
fn a_reverse_aerial_rush_falls_out_of_jumping_from_a_turnaround() {
    let world = test_world();
    let mut tuning = super::TEST_TUNING;
    tuning.initial_dash_time = 14.0 / 60.0;
    tuning.turnaround_time = 3.0 / 60.0;
    let hold = |x: f32| InputState {
        axes: crate::reference_frame::LocalAxes::new(x, 0.0),
        ..InputState::default()
    };
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.ground.on_ground = true;
    for _ in 0..40 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    // Commit to a run to the RIGHT.
    for _ in 0..60 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            hold(1.0),
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(
        scratch.axis().running,
        "the fixture never committed to a run"
    );
    assert_eq!(scratch.kinematics.facing, 1.0);

    // Tap the other way — a turnaround, facing still right.
    super::update_player_with_tuning_scratch(&world, &mut scratch, hold(-1.0), 1.0 / 60.0, tuning);
    assert!(
        scratch.axis().turnaround_timer > 0.0,
        "the tap did not start a turnaround, so this measures an ordinary jump"
    );
    assert_eq!(scratch.kinematics.facing, 1.0);

    // Jump out of it, then steer FORWARD again — back the way the run was
    // going. ⭐ THAT IS THE TECHNIQUE: an airborne body may not turn
    // (`can_turn` is grounded-or-flying), so the reversed facing STICKS while
    // the stick carries the fighter onward. ⛔ letting go instead would stop
    // the body dead — this engine's air stop assist is tight, so "momentum
    // carries you" is not a property it has, and the rush here is bought with
    // the stick rather than with drift.
    let mut jump = hold(-1.0);
    jump.movement = crate::ActionEdges::EMPTY.with(
        crate::MovementAction::Jump,
        crate::Edge {
            pressed: true,
            held: true,
            released: false,
        },
    );
    super::update_player_with_tuning_scratch(&world, &mut scratch, jump, 1.0 / 60.0, tuning);
    for _ in 0..6 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            hold(1.0),
            1.0 / 60.0,
            tuning,
        );
    }

    assert!(!scratch.ground.on_ground, "the body never left the ground");
    // THE RUSH: facing LEFT, still travelling RIGHT. A back-air thrown here
    // points at where the fighter came from, which is the whole technique.
    assert_eq!(
        scratch.kinematics.facing, -1.0,
        "the fighter carried its OLD facing into the air — no rush, just a jump"
    );
    assert!(
        scratch.kinematics.vel.x > 40.0,
        "the fighter lost the momentum it turned out of: vel_x {:.0}",
        scratch.kinematics.vel.x
    );
}

/// RAISING A GUARD MID-RUN PLANTS YOU — but a ROLL still travels.
///
/// ⛔⛔ THE BUG THIS PINS: the whole ground-speed block, friction included, sits
/// inside `can_move_horizontal`, which a raised guard turns off. So a fighter
/// who shielded out of a run KEPT the run — measured at 270px/s, still 270
/// sixty ticks later with the guard up the whole time. "May not steer" is not
/// "may not stop".
///
/// ⛔ THE SECOND ARM IS WHY THE FIX IS NOT SIMPLY "BRAKE WHEN SHIELDING": a
/// roll is shield-held too and SETS its own velocity, so braking it would be
/// the movement law reaching into a speed it does not own — the mistake the
/// initial dash made with knockback.
#[test]
fn a_guard_raised_out_of_a_run_plants_the_body_but_a_roll_still_travels() {
    let world = test_world();
    let mut tuning = super::TEST_TUNING;
    tuning.initial_dash_time = 14.0 / 60.0;
    tuning.turnaround_time = 3.0 / 60.0;
    let hold = |x: f32| InputState {
        axes: crate::reference_frame::LocalAxes::new(x, 0.0),
        ..InputState::default()
    };
    let committed = || {
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
        scratch.ground.on_ground = true;
        scratch.abilities.abilities.shield = true;
        for _ in 0..40 {
            super::update_player_with_tuning_scratch(
                &world,
                &mut scratch,
                InputState::default(),
                1.0 / 60.0,
                tuning,
            );
        }
        for _ in 0..60 {
            super::update_player_with_tuning_scratch(
                &world,
                &mut scratch,
                hold(1.0),
                1.0 / 60.0,
                tuning,
            );
        }
        assert!(
            scratch.kinematics.vel.x > 200.0,
            "the fixture never reached a real run, so there is nothing to cancel"
        );
        scratch
    };

    // ARM 1 — THE CANCEL. Guard up, stick neutral: the run is gone in a few
    // frames rather than gliding on.
    let mut planted = committed();
    let mut guard = InputState::default();
    guard.shield_held = true;
    let mut stopped_after = None;
    for t in 0..60 {
        super::update_player_with_tuning_scratch(&world, &mut planted, guard, 1.0 / 60.0, tuning);
        if stopped_after.is_none() && planted.kinematics.vel.x.abs() < 1.0 {
            stopped_after = Some(t + 1);
        }
    }
    assert!(planted.shield.active, "the guard never came up");
    let ticks = stopped_after.expect("a raised guard never stopped the run — the body glided");
    assert!(
        ticks <= 8,
        "a raised guard took {ticks} ticks to plant the body — that is a glide, not a cancel"
    );

    // ARM 2 — AND A ROLL STILL TRAVELS. Same guard button, WITH a direction:
    // that is the evade, it sets its own speed, and the brake must leave it be.
    let mut rolling = committed();
    let mut roll = hold(1.0);
    roll.shield_held = true;
    super::update_player_with_tuning_scratch(&world, &mut rolling, roll, 1.0 / 60.0, tuning);
    assert!(
        rolling.axis().dodge_roll_timer > 0.0,
        "shield + direction did not roll, so arm 2 measures nothing"
    );
    // ⛔ MEASURED OVER THE ROLL, NOT ON ITS FIRST TICK. One tick of friction
    // barely dents 530px/s, so a single sample passes even when the brake is
    // eating the roll — the version that braked regardless of the evade slipped
    // through exactly that way.
    let start = rolling.kinematics.pos.x;
    let mut ticks = 0;
    while rolling.axis().dodge_roll_timer > 0.0 && ticks < 30 {
        super::update_player_with_tuning_scratch(&world, &mut rolling, roll, 1.0 / 60.0, tuning);
        ticks += 1;
    }
    let travelled = rolling.kinematics.pos.x - start;
    assert!(
        travelled > 60.0,
        "the brake ate the roll: it covered {travelled:.0}px over {ticks} ticks"
    );

    // ARM 3 — AND IT MUST NOT EAT A LAUNCH. A body holding its guard on the
    // ground is "planted" by every test this branch applies, INCLUDING one that
    // was just hit — which is how the first version of this brake deleted
    // knockback and reddened
    // `a_ground_roll_ends_stopped_but_never_eats_a_launch`. The bound is
    // ownership: a brake may only take back speed the body could have walked
    // up to.
    let mut launched = committed();
    launched.flight.pending_launch = crate::Vec2::new(1400.0, -60.0);
    let mut guard_only = InputState::default();
    guard_only.shield_held = true;
    // ⛔ THE DISTANCE COVERED, not the peak: the peak is the launch's own first
    // tick and it survives any amount of braking afterwards, so a peak
    // assertion cannot tell the two versions apart — the unbounded brake passed
    // it.
    let from = launched.kinematics.pos.x;
    for _ in 0..10 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut launched,
            guard_only,
            1.0 / 60.0,
            tuning,
        );
    }
    let flew = launched.kinematics.pos.x - from;
    assert!(
        flew > 150.0,
        "planting the body ate a launch it did not create: it covered {flew:.0}px in 10 ticks"
    );
}

/// STANDING ON THE BRINK IS A FACT ABOUT WHERE YOU ARE, AND WHICH WAY YOU FACE.
///
/// Four arms. The third is the one a careless probe would get wrong: the same
/// body on the same spot, facing INWARD, is not teetering — the lean goes the
/// other way and there is floor under it. Without that arm a rule that answered
/// "near an edge" rather than "about to leave one" would pass.
#[test]
fn a_body_teeters_only_on_the_brink_it_is_facing() {
    use crate::world::{Block, World};
    // A single platform spanning x 300..500, its top at y 300.
    let world = World {
        name: "teeter".to_string(),
        size: crate::Vec2::new(800.0, 600.0),
        spawn: crate::Vec2::new(400.0, 100.0),
        blocks: vec![Block::solid(
            "ledge",
            crate::Vec2::new(300.0, 300.0),
            crate::Vec2::new(200.0, 40.0),
        )],
        climbable_regions: Vec::new(),
        chains: Vec::new(),
        edges: Default::default(),
        water_regions: Vec::new(),
    };
    let mut tuning = super::TEST_TUNING;
    tuning.teeter_margin = 0.25;

    let settled = |x: f32, facing: f32, tuning: crate::test_support::TestTuning| {
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), crate::Vec2::new(x, 250.0));
        scratch.ground.on_ground = false;
        for _ in 0..40 {
            super::update_player_with_tuning_scratch(
                &world,
                &mut scratch,
                InputState::default(),
                1.0 / 60.0,
                tuning,
            );
        }
        scratch.kinematics.facing = facing;
        super::update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
        scratch
    };

    // ARM 1 — MID-PLATFORM IS NOT THE BRINK, whichever way you face.
    let middle = settled(400.0, 1.0, tuning);
    assert!(
        middle.ground.on_ground,
        "the fixture never landed on the platform"
    );
    assert!(
        !middle.axis().teetering,
        "a body in the middle of a platform was called a teeter"
    );

    // ARM 2 — THE LIP, FACING OUT. Right at the right-hand edge, facing right.
    // Measured 2026-08-25 with a 15px half-width on a platform ending at 500:
    // the brink begins at x=492, which is where the leading quarter of the
    // footprint clears the edge. 488 is not teetering and 492 is.
    let brink = settled(496.0, 1.0, tuning);
    assert!(brink.ground.on_ground, "the fixture fell off the lip");
    assert!(
        brink.axis().teetering,
        "a body on the lip facing off it was not teetering"
    );

    // ARM 3 — THE SAME SPOT, FACING IN. There is floor the way it leans, so it
    // is not on any brink it can reach. ⛔ this is what separates "about to
    // leave an edge" from "near an edge".
    let inward = settled(496.0, -1.0, tuning);
    assert!(
        !inward.axis().teetering,
        "a body facing INTO the platform was called a teeter"
    );

    // ARM 4 — A WORLD THAT DECLARES NO MARGIN NEVER TEETERS, which is every
    // world in this repo. Same lip, same facing, engine tuning.
    let plain = settled(496.0, 1.0, super::TEST_TUNING);
    assert!(
        !plain.axis().teetering,
        "a world declaring no teeter margin produced one anyway"
    );
}

/// FAST-FALL COMES BACK WHEN THE LAUNCH LETS GO OF YOU, AND NOT BEFORE.
///
/// The parity row asks for exactly this and no new state: "reuse fast-fall once
/// hitstun/control gates permit it". Measured 2026-08-25 — it already holds,
/// because `tick_knockdown` strips control input for the tumble's duration and
/// hands it back whole. This pins BOTH halves, since either alone is a
/// different game: refused forever is a fighter who cannot come down, and
/// permitted always is a launch you can cancel.
#[test]
fn fast_fall_is_refused_inside_a_tumble_and_returns_when_it_ends() {
    let world = test_world();
    let mut tuning = super::TEST_TUNING;
    tuning.tumble_speed = 500.0;
    let mut ff = InputState::default();
    ff.movement = crate::ActionEdges::EMPTY.with(
        crate::MovementAction::FastFall,
        crate::Edge {
            pressed: true,
            held: true,
            released: false,
        },
    );
    let launched = || {
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), crate::Vec2::new(400.0, 150.0));
        scratch.ground.on_ground = false;
        scratch.flight.pending_launch = crate::Vec2::new(120.0, -700.0);
        scratch
    };

    // DURING — still tumbling AND already descending. ⛔ the descending half
    // matters: fast-fall on a RISING body does nothing anyway, so a probe taken
    // at the top of the arc passes whether the tumble suppresses control or
    // not. Launched flat so gravity has it falling well inside the window.
    let mut mid = {
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), crate::Vec2::new(200.0, 150.0));
        scratch.ground.on_ground = false;
        scratch.flight.pending_launch = crate::Vec2::new(700.0, -40.0);
        scratch
    };
    for _ in 0..8 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut mid,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(
        mid.axis().tumble_timer > 0.0 && mid.kinematics.vel.y > 0.0,
        "the fixture is not a tumbling, DESCENDING body: tumble {:.2}, vy {:.0}",
        mid.axis().tumble_timer,
        mid.kinematics.vel.y
    );
    super::update_player_with_tuning_scratch(&world, &mut mid, ff, 1.0 / 60.0, tuning);
    assert!(
        !mid.axis().fast_falling,
        "a tumbling body fast-fell — a launch you can cancel is not a launch"
    );

    // AFTER — the tumble has run out and the body is still in the air.
    let mut free = launched();
    for _ in 0..40 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut free,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(
        free.axis().tumble_timer <= 0.0 && !free.ground.on_ground,
        "the fixture is not a free airborne body: tumble {:.2}, on_ground {}",
        free.axis().tumble_timer,
        free.ground.on_ground
    );
    let before = free.kinematics.vel.y;
    super::update_player_with_tuning_scratch(&world, &mut free, ff, 1.0 / 60.0, tuning);
    assert!(
        free.axis().fast_falling && free.kinematics.vel.y > before,
        "fast-fall did not come back after the tumble: {before:.0} -> {:.0}",
        free.kinematics.vel.y
    );
}

/// ⛔⛔ A GUST DECIDES NO FLOOR GAME — IT NEITHER PINS NOR TUMBLES.
///
/// The one gateway every launch passes through asked `jab_lock` and
/// `launch_into_tumble` from `launch.length()` alone, because
/// `BodyFlightState::pending_launch` was a bare `Vec2` carrying no kind. So a
/// WEAK gust pinned a prone body where it lay — the jab-lock rule, which exists
/// to reward hitting somebody who is down — and a STRONG one sent it tumbling,
/// both against a volume whose authored contract is *"moves you and leaves you
/// in control"*.
///
/// ⭐ THE ARMS STRADDLE THE KIND AND NOTHING ELSE: the same speed, the same
/// state, one `flinchless` flag apart. Without the strike arms this would pass
/// for a kernel that had stopped pinning and tumbling altogether.
#[test]
fn a_flinchless_push_neither_pins_a_prone_body_nor_starts_a_tumble() {
    let world = test_world();
    let mut tuning = super::TEST_TUNING;
    tuning.tumble_speed = 500.0;
    tuning.jab_lock_speed = 320.0;
    tuning.jab_lock_limit = 3;

    // A body already down: the state the jab lock is about.
    let pinned_by = |flinchless: bool| {
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), crate::Vec2::new(400.0, 300.0));
        scratch.axis_mut().knockdown_timer = 0.5;
        let before = scratch.axis().jab_locks;
        scratch
            .flight
            .stage_launch(crate::Vec2::new(120.0, -40.0), flinchless);
        super::update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
        scratch.axis().jab_locks > before
    };
    assert!(
        pinned_by(false),
        "a weak STRIKE did not pin a prone body, so the refusal below is a \
         kernel that stopped jab-locking rather than one reading the kind"
    );
    assert!(
        !pinned_by(true),
        "a GUST pinned a body that was already down — the floor game is being \
         decided from speed alone"
    );

    // …and a hard one launches into tumble, unless it is a push.
    let tumbled_by = |flinchless: bool| {
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), crate::Vec2::new(400.0, 300.0));
        scratch
            .flight
            .stage_launch(crate::Vec2::new(0.0, -900.0), flinchless);
        super::update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
        scratch.axis().tumble_until_landing || scratch.axis().tumble_timer > 0.0
    };
    assert!(
        tumbled_by(false),
        "a hard STRIKE did not tumble the body, so the refusal below proves \
         nothing"
    );
    assert!(
        !tumbled_by(true),
        "a GUST sent the body tumbling — a push that takes control away is the \
         one thing a windbox must not do"
    );
}

/// OWNING A RISE MEANS HAVING BOUGHT IT, NOT BEING SLOW ENOUGH TO HAVE.
///
/// ⛔⛔ THE ARM THAT MATTERS IS THE LAUNCH BELOW `double_jump_speed`. What stood
/// here asserted the opposite: it gave a body a WEAK external launch and called
/// the climb "owned" because the number was small. That is a magnitude standing
/// in for ownership, and it is the bug — `air_jump_rising` also required that an
/// air jump had been spent *at some point*, which stays true for the rest of the
/// airtime, so a fighter who had double-jumped and then been launched upward at
/// any speed under its own jump read as riding its own jump. An aerial DELETED
/// the opponent's launch.
///
/// ⭐ THE FACT IS AN AMOUNT NOW, granted only by the SPEND and only ever
/// shrunk after it. This drives a REAL double jump for the owning arm rather
/// than manufacturing the resource state, because a manufactured spend is
/// exactly what let the old version agree with the bug.
#[test]
fn owned_air_jump_rise_is_bought_by_the_jump_and_never_by_a_launch() {
    let world = test_world();
    let tuning = super::TEST_TUNING;
    let jump_speed = tuning.params().locomotion.double_jump_speed;
    assert!(jump_speed > 0.0, "the fixture has no air jump to own");

    let airborne = || {
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), crate::Vec2::new(400.0, 300.0));
        scratch.ground.on_ground = false;
        scratch.axis_mut().coyote_timer = 0.0;
        scratch
    };
    let jump_press = InputState {
        movement: crate::ActionEdges::EMPTY.with(
            crate::MovementAction::Jump,
            crate::Edge {
                pressed: true,
                held: false,
                released: false,
            },
        ),
        ..Default::default()
    };

    // ⭐ A REAL DOUBLE JUMP. The spend is the only thing that grants.
    let mut jumped = airborne();
    jumped.jump.air_jumps_available = 1;
    let events = step_scratch(&world, &mut jumped, jump_press);
    assert!(
        events
            .operations
            .contains(&crate::movement::MovementOp::DoubleJump),
        "the fixture never air-jumped, so the grant below is about nothing"
    );
    assert!(
        jumped.axis().air_jump_rise_owned > jump_speed * 0.5,
        "an air jump the body just spent granted {} of owned rise against a \
         jump speed of {jump_speed}",
        jumped.axis().air_jump_rise_owned
    );

    // ⛔⛔ THE FAILING SCENARIO. Same body, jump long since eaten by gravity,
    // then an opponent launches it upward SLOWER than its own jump — the exact
    // shape the magnitude test could not tell from a jump.
    let mut launched = jumped;
    for _ in 0..240 {
        step_scratch(&world, &mut launched, InputState::default());
        if launched.axis().air_jump_rise_owned <= 0.0 {
            break;
        }
    }
    assert_eq!(
        launched.axis().air_jump_rise_owned,
        0.0,
        "gravity never finished the jump, so the launch arm below would be \
         measuring leftover jump rather than the launch"
    );
    launched.flight.pending_launch = crate::Vec2::new(0.0, -jump_speed * 0.5);
    step_scratch(&world, &mut launched, InputState::default());
    assert!(
        -launched.kinematics.vel.y > 0.0,
        "the launch did not lift the body, so the refusal below is vacuous"
    );
    assert_eq!(
        launched.axis().air_jump_rise_owned,
        0.0,
        "a launch below the body's own jump speed handed it {} of OWNED rise — \
         an aerial would now delete the opponent's knockback",
        launched.axis().air_jump_rise_owned
    );

    // And a body that never air-jumped owns nothing, however gently it rises.
    let mut never = airborne();
    never.flight.pending_launch = crate::Vec2::new(0.0, -jump_speed * 0.5);
    step_scratch(&world, &mut never, InputState::default());
    assert_eq!(
        never.axis().air_jump_rise_owned,
        0.0,
        "a body that never spent an air jump owned some of its climb"
    );
}

/// ⛔⛔ A ROOTED MOVE HANDED BACK A FREE DASH THE PLAYER NEVER ASKED FOR.
///
/// The initial dash remembers direction by comparing this tick's stick with last
/// tick's. A move with `motion_scale: 0.0` scales the stick to zero, so a player
/// who simply HELD right through an attack was recorded as neutral for its whole
/// duration — and the tick it ended read as "pressed right from nothing", which
/// is exactly the edge that arms a full-speed dash.
///
/// ⭐ THE ARMS SEPARATE THE TWO THINGS THE OLD READING COULD NOT TELL APART: a
/// stick that was DAMPED and a stick that was RELEASED. Letting go and pressing
/// again SHOULD buy a dash — that is the third arm, and without it this test
/// would pass just as well for a kernel that had stopped granting dashes.
#[test]
fn holding_a_direction_through_a_rooted_move_does_not_buy_a_dash() {
    let world = test_world();
    // ⛔ THE EXPLORATION DEFAULT AUTHORS NO INITIAL DASH — the fighter profile
    // does. Stated here rather than assumed: the premise guard below caught this
    // fixture measuring a kernel that could not dash at all.
    let mut tuning = super::TEST_TUNING;
    tuning.initial_dash_time = 0.18;
    tuning.initial_dash_speed = 520.0;
    assert!(
        tuning.params().locomotion.initial_dash_time > 0.0,
        "the fixture has no initial dash to be handed"
    );

    /// Hold right until the first dash has expired, then feed `during` for ten
    /// ticks, then a plain held-right tick. Reports whether that last tick armed
    /// a dash.
    fn dash_after(world: &crate::World, tuning: super::TestTuning, during: InputState) -> bool {
        let held = InputState::with_axes(1.0, 0.0);
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
        for _ in 0..30 {
            super::update_player_with_tuning_scratch(world, &mut scratch, held, 1.0 / 60.0, tuning);
        }
        assert_eq!(
            scratch.axis().initial_dash_timer,
            0.0,
            "the opening dash never expired, so the arm below cannot tell a NEW \
             dash from the first one"
        );
        for _ in 0..10 {
            super::update_player_with_tuning_scratch(
                world,
                &mut scratch,
                during,
                1.0 / 60.0,
                tuning,
            );
        }
        super::update_player_with_tuning_scratch(world, &mut scratch, held, 1.0 / 60.0, tuning);
        scratch.axis().initial_dash_timer > 0.0
    }

    // ⛔ THE DEFECT. The stick is damped to nothing by a rooted move; the player
    // never let go.
    let rooted = InputState {
        axes: crate::LocalAxes::ZERO,
        undamped_axes: Some(crate::LocalAxes::new(1.0, 0.0)),
        ..InputState::default()
    };
    assert!(
        !dash_after(&world, tuning, rooted),
        "a rooted move gave the body a free initial dash the frame it ended — \
         the damped stick was recorded as a RELEASE, so holding right read as \
         pressing it again"
    );

    // ⭐ AND A REAL RELEASE STILL BUYS ONE, or the arm above proves nothing.
    assert!(
        dash_after(&world, tuning, InputState::default()),
        "letting go and pressing again did not buy a dash, so the refusal above \
         is a kernel that stopped dashing rather than one that stopped being \
         fooled"
    );
}

/// AN ACCEPTED ROLL TRAVELS THE SAME DISTANCE WHETHER OR NOT YOU KEEP HOLDING
/// THE BUTTON THAT STARTED IT.
///
/// ⛔⛔ JON'S PLAYTEST, 2026-08-25: "roll distance is input/history-dependent
/// AFTER the roll has already begun". Ordinary ground steering is disabled by
/// `shield_held && on_ground` — NOT by the roll being active — so releasing the
/// guard mid-roll switches the ordinary friction/steer law back on top of a
/// velocity the roll owns. The existing roll arm holds the button down for the
/// whole maneuver, so it cannot see this.
///
/// ⭐ A MANEUVER THE GAME HAS ALREADY ACCEPTED IS NOT STILL TAKING INPUT. Once
/// the roll is committed its travel is authored, and what the hand does next is
/// the NEXT action's business.
#[test]
fn a_roll_travels_the_same_distance_however_the_button_is_released() {
    let world = test_world();
    let tuning = super::TEST_TUNING;

    // The stick, held toward `dir`, which is what turns a guard into a ROLL.
    let hold = |dir: f32| {
        let mut i = InputState::default();
        i.axes = crate::reference_frame::LocalAxes::new(dir, 0.0);
        i
    };

    // Every arm starts the SAME roll, then differs only in what is held after.
    let roll_travel = |after: InputState| {
        let mut body = scratch_with(AbilitySet::sandbox_all(), world.spawn);
        // ⛔ SETTLE ONTO THE FLOOR FIRST. The spawn is above it, and a roll begun
        // mid-fall is an AIR roll for its first eight ticks — which measured the
        // air law and hid the ground one completely.
        for _ in 0..30 {
            super::update_player_with_tuning_scratch(
                &world,
                &mut body,
                InputState::default(),
                1.0 / 60.0,
                tuning,
            );
        }
        assert!(body.ground.on_ground, "the fixture never landed");
        let mut start_roll = hold(1.0);
        start_roll.shield_held = true;
        super::update_player_with_tuning_scratch(&world, &mut body, start_roll, 1.0 / 60.0, tuning);
        assert!(
            body.axis().dodge_roll_timer > 0.0,
            "the fixture never rolled, so this arm measures nothing"
        );
        let start = body.kinematics.pos.x;
        let mut ticks = 0;
        while body.axis().dodge_roll_timer > 0.0 && ticks < 40 {
            super::update_player_with_tuning_scratch(&world, &mut body, after, 1.0 / 60.0, tuning);
            ticks += 1;
        }
        body.kinematics.pos.x - start
    };

    let mut held = hold(1.0);
    held.shield_held = true;
    let held_travel = roll_travel(held);

    let released = InputState::default();
    let released_travel = roll_travel(released);

    let mut against = hold(-1.0);
    against.shield_held = false;
    let against_travel = roll_travel(against);

    assert!(
        held_travel > 60.0,
        "the control arm did not roll ({held_travel:.0}px), so the comparisons below are vacuous"
    );
    let released_gap = (held_travel - released_travel).abs();
    assert!(
        released_gap < 1.0,
        "releasing the guard mid-roll changed the distance by {released_gap:.0}px \
         (held {held_travel:.0} vs released {released_travel:.0}) — the roll does not \
         own its own movement, so ordinary friction is editing a velocity it did not author"
    );
    let against_gap = (held_travel - against_travel).abs();
    // ⚠ 2px, not zero: the loop's LAST tick is the one where the roll expires, and
    // on that tick the stick legitimately steers again. The defect this arm exists
    // for was 71px.
    assert!(
        against_gap < 2.0,
        "steering AGAINST an accepted roll changed the distance by {against_gap:.0}px \
         (held {held_travel:.0} vs opposed {against_travel:.0}) — a committed maneuver \
         is still taking input it should have stopped reading"
    );
}

/// STALING WEARS THE I-FRAMES, NOT THE DISTANCE — the sentence `spend_evade`
/// has always CLAIMED, now asserted.
///
/// ⛔⛔ It was never true. `spend_evade` returns the STALED window and that value
/// lands in `dodge_roll_timer`, which governs travel, endlag AND commitment — so
/// a spammed roll got shorter, not merely less safe. At the Smash floor that is
/// about a third of the authored distance.
#[test]
fn staling_wears_an_evades_i_frames_and_leaves_its_distance_alone() {
    let world = test_world();
    let mut tuning = super::TEST_TUNING;
    tuning.base.dodge_stale_step = 0.25;
    tuning.base.dodge_stale_floor = 0.34;
    tuning.base.dodge_stale_recovery = 0.5;
    assert!(
        tuning.params().abilities.dodge_stale_step > 0.0,
        "the fixture declares no staling, so both arms below are the same body"
    );

    let hold = |dir: f32| {
        let mut i = InputState::default();
        i.axes = crate::reference_frame::LocalAxes::new(dir, 0.0);
        i
    };
    let roll = |evades_recent: u8| {
        let mut body = scratch_with(AbilitySet::sandbox_all(), world.spawn);
        body.ground.on_ground = true;
        body.dodge.evades_recent = evades_recent;
        let mut start_roll = hold(1.0);
        start_roll.shield_held = true;
        super::update_player_with_tuning_scratch(&world, &mut body, start_roll, 1.0 / 60.0, tuning);
        let start = body.kinematics.pos.x;
        let mut ticks = 0;
        let mut intangible_ticks = 0;
        while body.axis().dodge_roll_timer > 0.0 && ticks < 40 {
            if crate::movement::BodyMotionFacts::from_model(&body.model).evading() {
                intangible_ticks += 1;
            }
            super::update_player_with_tuning_scratch(
                &world,
                &mut body,
                start_roll,
                1.0 / 60.0,
                tuning,
            );
            ticks += 1;
        }
        (body.kinematics.pos.x - start, intangible_ticks, ticks)
    };

    let (fresh_travel, fresh_iframes, fresh_ticks) = roll(0);
    let (stale_travel, stale_iframes, stale_ticks) = roll(3);

    assert!(
        fresh_travel > 60.0 && fresh_iframes > 0,
        "the fresh arm neither travelled ({fresh_travel:.0}px) nor went intangible \
         ({fresh_iframes} ticks), so nothing below can be measured"
    );
    let gap = (fresh_travel - stale_travel).abs();
    assert!(
        gap < 1.0,
        "a stale roll travelled {gap:.0}px less than a fresh one \
         (fresh {fresh_travel:.0} over {fresh_ticks} ticks, stale {stale_travel:.0} over \
         {stale_ticks}) — staling is wearing the DISTANCE, which is exactly what \
         spend_evade's own header says it must not do"
    );
    assert!(
        stale_iframes < fresh_iframes,
        "a heavily stale roll was intangible for {stale_iframes} ticks against a fresh \
         roll's {fresh_iframes} — staling is not wearing the i-frames either, so the \
         mechanic does nothing at all"
    );
}

/// A REVERSE TOO SOFT FOR A DASH IS TOO SOFT FOR A TURNAROUND, AND THE FACING
/// STILL ARRIVES.
///
/// ⛔⛔ THE EDGE TEST COMPARED TWO DIFFERENT THRESHOLDS. `prev_steer_dir` is
/// written by the initial dash only past a 0.5 deadzone, and the turnaround
/// tested it against a bare `signum()`: a stick held around -0.2 was NEUTRAL to
/// the writer and A DEFINITE REVERSE to the reader, so that edge was true on
/// every single tick. ⚠ Full-stick and keyboard input clear both thresholds,
/// which is why every existing arm passed.
///
/// ⭐⭐ WHAT THAT DID NOT DO IS RE-ARM FOREVER, and measuring it is what kept a
/// rollback field out of the fix: the facing SNAPS to the stick the moment the
/// timer expires, so `reversing` goes false and the phase cannot retrigger. The
/// review predicted an endless turn; the code cannot produce one. So the defect
/// is the mismatched comparison itself, and the fix is one threshold, not a
/// second memory.
#[test]
fn a_reverse_too_soft_to_dash_does_not_buy_a_turnaround() {
    let world = test_world();
    let mut tuning = super::TEST_TUNING;
    tuning.base.turnaround_time = 0.05;

    let mut body = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    for _ in 0..30 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut body,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    // Commit a run to the RIGHT at full stick, which is what a turnaround needs
    // to be paid out of.
    let mut forward = InputState::default();
    forward.axes = crate::reference_frame::LocalAxes::new(1.0, 0.0);
    for _ in 0..90 {
        super::update_player_with_tuning_scratch(&world, &mut body, forward, 1.0 / 60.0, tuning);
    }
    assert!(
        body.axis().running && body.kinematics.facing > 0.0,
        "the fixture never committed a rightward run, so nothing below is a turnaround"
    );

    // Now hold a MODERATE reverse — past the turnaround's 0.1, under the dash's
    // 0.5 — for far longer than the phase lasts.
    let mut partial = InputState::default();
    partial.axes = crate::reference_frame::LocalAxes::new(-0.2, 0.0);
    let mut armings = 0;
    let mut was_turning = false;
    for _ in 0..60 {
        super::update_player_with_tuning_scratch(&world, &mut body, partial, 1.0 / 60.0, tuning);
        let turning = body.axis().turnaround_timer > 0.0;
        if turning && !was_turning {
            armings += 1;
        }
        was_turning = turning;
    }

    assert_eq!(
        armings, 0,
        "a stick too soft to start a DASH bought {armings} turnaround(s) — the two \
         mechanics are reading one memory at two different thresholds"
    );
    // ⛔ AND THE BODY STILL TURNS. Without this the fix could be "never turn on a
    // soft stick", which would strand an analog player facing the wrong way.
    assert!(
        body.kinematics.facing < 0.0,
        "a soft reverse left the body facing its old direction entirely"
    );
}

/// A GUARD YOU DID NOT DROP COSTS NOTHING.
///
/// ⛔⛔ THE DROP-LAG RULE ASKED THE EFFECT, NOT THE CAUSE. Its own comment says
/// the cost is for "you simply let go", and the condition was `was_up &&
/// !active` — which is EVERY way a guard can end. Under `air_guard: false` a
/// fighter that leaves the ground with Shield STILL HELD has its guard forced
/// down, and this billed the full release penalty; `drop_lag_timer` feeds
/// `hard_lock_timer`, so a platform drop could hard-lock the body for the
/// ordinary cost of letting go of a button the player never let go of.
#[test]
fn a_guard_forced_down_by_leaving_the_ground_owes_no_drop_lag() {
    let world = test_world();
    let mut tuning = super::TEST_TUNING;
    tuning.base.shield.drop_lag = 11.0 / 60.0;
    // The composition the defect needs: a guard that cannot be held in the air.
    tuning.base.shield.air_guard = false;
    assert!(
        tuning.params().abilities.shield.drop_lag > 0.0,
        "the fixture declares no drop lag, so neither arm below measures anything"
    );

    let mut guarding = InputState::default();
    guarding.shield_held = true;

    // ── ARM 1: the guard is taken away by LEAVING THE GROUND, hand still down.
    let mut body = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    for _ in 0..30 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut body,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    super::update_player_with_tuning_scratch(&world, &mut body, guarding, 1.0 / 60.0, tuning);
    assert!(
        body.shield.active,
        "the fixture never raised a guard, so nothing below is a forced drop"
    );
    // Off the floor with the button STILL HELD.
    body.ground.on_ground = false;
    body.kinematics.pos.y -= 64.0;
    super::update_player_with_tuning_scratch(&world, &mut body, guarding, 1.0 / 60.0, tuning);
    assert!(
        !body.shield.active,
        "the guard survived going airborne, so this arm is not a forced drop"
    );
    assert_eq!(
        body.shield.drop_lag_timer, 0.0,
        "a guard taken away by LEAVING THE GROUND charged the release penalty \
         ({:.3}s) — the player never let go, and this timer hard-locks the body",
        body.shield.drop_lag_timer
    );

    // ── ARM 2: AND LETTING GO STILL COSTS. Without this the fix could be
    // "never charge", which deletes the mechanic instead of aiming it.
    let mut dropper = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    for _ in 0..30 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut dropper,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    super::update_player_with_tuning_scratch(&world, &mut dropper, guarding, 1.0 / 60.0, tuning);
    assert!(dropper.shield.active, "arm 2 never raised a guard");
    super::update_player_with_tuning_scratch(
        &world,
        &mut dropper,
        InputState::default(),
        1.0 / 60.0,
        tuning,
    );
    assert!(
        dropper.shield.drop_lag_timer > 0.0,
        "a player who LET GO of the guard on the floor was not charged for it, \
         so the mechanic is gone rather than aimed"
    );
}

/// FORGIVENESS DOES NOT RUN WHILE THE EVADE IS STILL HAPPENING.
///
/// ⛔⛔ `spend_evade`'s contract says the stale count "only starts coming down
/// once the body actually stops", and the decay ticked from the moment the evade
/// was ACCEPTED — so a roll spent part of its own forgiveness delay performing
/// the very maneuver the delay exists to charge for. At Smash's numbers (0.22s
/// roll, 1.2s recovery) that is about 18% of it, every roll.
///
/// ⚠ THE EXISTING DECAY TEST SEEDS STALE STATE ON AN IDLE BODY, so it never runs
/// an accepted evade through its own maneuver — the fixture omits the state the
/// bug lives in.
#[test]
fn stale_forgiveness_starts_only_once_the_evade_is_over() {
    let world = test_world();
    let mut tuning = super::TEST_TUNING;
    tuning.base.dodge_stale_step = 0.25;
    tuning.base.dodge_stale_floor = 0.34;
    tuning.base.dodge_stale_recovery = 1.2;

    let hold = |dir: f32| {
        let mut i = InputState::default();
        i.axes = crate::reference_frame::LocalAxes::new(dir, 0.0);
        i
    };

    let mut body = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    for _ in 0..30 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut body,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(body.ground.on_ground, "the fixture never landed");

    // Accept a real roll.
    let mut start_roll = hold(1.0);
    start_roll.shield_held = true;
    super::update_player_with_tuning_scratch(&world, &mut body, start_roll, 1.0 / 60.0, tuning);
    assert!(
        body.axis().dodge_roll_timer > 0.0 && body.dodge.evades_recent > 0,
        "the fixture never rolled, so there is no forgiveness to measure"
    );
    let armed = body.dodge.stale_decay;
    assert!(
        (armed - 1.2).abs() < 1e-3,
        "the roll did not arm the full forgiveness delay ({armed:.3}s of 1.2)"
    );

    // Run the roll out. Forgiveness must not move while it is happening.
    let mut ticks = 0;
    while body.axis().dodge_roll_timer > 0.0 && ticks < 60 {
        super::update_player_with_tuning_scratch(&world, &mut body, start_roll, 1.0 / 60.0, tuning);
        ticks += 1;
    }
    assert!(
        ticks > 4,
        "the roll ended in {ticks} ticks — too short to measure"
    );
    // ⚠ ONE TICK OF SLACK, AND IT IS THE RIGHT ONE: the roll's own clock is
    // decremented earlier in the same function than this decay, so on the tick
    // the roll ENDS the body is already no longer evading and forgiveness
    // legitimately begins. The defect was thirteen ticks — the whole roll.
    let bled = armed - body.dodge.stale_decay;
    assert!(
        bled <= 1.5 / 60.0,
        "forgiveness bled {bled:.3}s while the fighter was STILL ROLLING — the \
         delay is supposed to start when the body stops, and a roll now pays part \
         of its own penalty away just by happening"
    );

    // ⛔ AND IT STILL FORGIVES ONCE THE BODY IS IDLE. Without this the fix could
    // be "never decay", which deletes the mechanic rather than aiming it.
    let before = body.dodge.evades_recent;
    for _ in 0..90 {
        super::update_player_with_tuning_scratch(
            &world,
            &mut body,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(
        body.dodge.stale_decay < armed,
        "forgiveness never ran on an IDLE body, so staling is now permanent"
    );
    let _ = before;
}
