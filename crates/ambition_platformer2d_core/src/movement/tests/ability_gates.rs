//! Pure ability-flag sanity: a flag set to `false` must keep the
//! corresponding op out of the FrameEvents / state.

use super::super::*;
use super::{step_scratch, test_world};
use crate::body_clusters::BodyClusterScratch;
use crate::AbilitySet;

fn scratch_with(abilities: AbilitySet, spawn: bevy_math::Vec2) -> BodyClusterScratch {
    BodyClusterScratch::new_with_abilities(spawn, abilities)
}

/// **A BODY THAT CANNOT JUMP DOES NOT JUMP WHEN THE BUTTON IS PRESSED.**
///
/// ⛔⛔ **the base `jump` flag was the one ability gate NOTHING pinned** — the
/// sibling below covers `double_jump`, and `double_dash` and `wall_climb` have
/// theirs, but the plainest capability in the set had no test at all. Its gate
/// is one `&&` in `apply_intent`, and one `&&` is exactly the kind of thing a
/// refactor drops without a compiler error.
///
/// ⭐ **it is Jon's own compositional acceptance criterion, at the engine end.**
/// *"Force a Puppy Slug into Smash … Jump → no jump if its body cannot jump …
/// no generic humanoid jump"* (campaign P3.27). The content half is pinned in
/// `puppy_slug_forced_seat.rs`: the shipped `npc_puppy_slug` authors
/// `move_horizontal` and nothing else, so its seated mask says `jump: false`.
/// That assertion is only worth having if the engine HONOURS the mask, and this
/// is the half that says so. Neither test is the claim on its own.
///
/// ⚠ **both directions, and the grounded control is not decoration**: a gate
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
    // ⚠ the OP is the intent and the VELOCITY is the consequence; asserting only
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

/// **A BODY THAT CANNOT DASH STILL RUNS**, and the dash attack's whole
/// reachability rests on that being two different facts.
///
/// ⛔⛔ **the move selector used to ask `BodyMotionFacts::dashing`** — the
/// TRAVERSAL dash's timer — and `SMASH_FIGHTER_KIT` switches `AbilitySet::dash`
/// off deliberately, so the running attack was unreachable in the only game
/// that authors one. Every test of the selector passed, because each told the
/// selector the body was dashing; none could ask whether a fighter ever is.
///
/// ⭐ **the second assertion is the poison, not decoration.** A `running` that
/// was merely an alias for `dashing` would satisfy "it runs" on a dash-capable
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

    // ⚠ and the gait has to be able to be FALSE, or the assertion above is a
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
