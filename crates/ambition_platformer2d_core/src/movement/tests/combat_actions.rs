//! Dodge roll trigger / cooldown / ability gate, and shield/parry
//! activation, deactivation, dash conflict, parry window reset.

use super::super::*;
use super::{step_scratch, test_world};
use crate::body_clusters::BodyClusterScratch;
use crate::AbilitySet;
use crate::Vec2;

fn scratch_at(spawn: Vec2) -> BodyClusterScratch {
    BodyClusterScratch::new_with_abilities(spawn, AbilitySet::sandbox_all())
}

fn scratch_with(abilities: AbilitySet, spawn: Vec2) -> BodyClusterScratch {
    BodyClusterScratch::new_with_abilities(spawn, abilities)
}

#[test]
fn dodge_roll_triggers_on_ground_with_ability() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.dodge.cooldown = 0.0;
    assert!(
        scratch.abilities.abilities.dodge,
        "sandbox_all enables dodge"
    );
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Dash,
                crate::Edge {
                    pressed: true,
                    held: false,
                    released: false,
                },
            ),
            ..Default::default()
        },
    );
    assert!(
        events.operations.contains(&MovementOp::DodgeRoll),
        "dash on ground with dodge ability should trigger DodgeRoll"
    );
    assert!(
        scratch.axis().dodge_roll_timer > 0.0,
        "dodge_roll_timer should be set"
    );
    assert!(
        scratch.kinematics.vel.x.abs() > 100.0,
        "should have lateral velocity from dodge"
    );
}

#[test]
fn dodge_roll_blocked_by_cooldown() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.dodge.cooldown = 0.3;
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Dash,
                crate::Edge {
                    pressed: true,
                    held: false,
                    released: false,
                },
            ),
            ..Default::default()
        },
    );
    assert!(
        !events.operations.contains(&MovementOp::DodgeRoll),
        "dodge should be blocked when on cooldown"
    );
}

#[test]
fn dodge_roll_disabled_when_ability_off() {
    let world = test_world();
    let mut abilities = AbilitySet::sandbox_all();
    abilities.dodge = false;
    let mut scratch = scratch_with(abilities, world.spawn);
    scratch.ground.on_ground = true;
    scratch.dodge.cooldown = 0.0;
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Dash,
                crate::Edge {
                    pressed: true,
                    held: false,
                    released: false,
                },
            ),
            ..Default::default()
        },
    );
    assert!(
        !events.operations.contains(&MovementOp::DodgeRoll),
        "dodge should not trigger when ability is disabled"
    );
}

#[test]
fn shield_activates_when_held_with_ability() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.abilities.abilities.shield = true;
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(scratch.shield.active, "shield should be active while held");
    assert!(
        scratch.shield.parry_window_timer > 0.0,
        "parry window should start on first activation"
    );
    assert!(
        events.operations.contains(&MovementOp::ShieldUp),
        "ShieldUp op should be recorded"
    );
}

#[test]
fn shield_deactivates_when_released() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.abilities.abilities.shield = true;
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(scratch.shield.active);
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: false,
            ..Default::default()
        },
    );
    assert!(
        !scratch.shield.active,
        "shield should drop when button released"
    );
}

#[test]
fn shield_blocked_during_dash() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.abilities.abilities.shield = true;
    scratch.axis_mut().dash_timer = 0.10; // force active dash
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(
        !scratch.shield.active,
        "shield cannot be raised during a dash"
    );
}

#[test]
fn shield_gives_fresh_parry_on_each_activation() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.abilities.abilities.shield = true;
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(scratch.shield.parry_window_timer > 0.0);
    // Expire the parry window and drop shield.
    scratch.shield.parry_window_timer = 0.0;
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: false,
            ..Default::default()
        },
    );
    // Re-raise: fresh parry window.
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(
        scratch.shield.parry_window_timer > 0.0,
        "raising shield again should reset the parry window"
    );
}

#[test]
fn shield_disabled_when_ability_off() {
    let world = test_world();
    let abilities = AbilitySet::basic(); // basic() has shield: false
    let mut scratch = scratch_with(abilities, world.spawn);
    scratch.ground.on_ground = true;
    let events = step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            ..Default::default()
        },
    );
    assert!(
        !scratch.shield.active,
        "shield should not activate without the ability"
    );
    assert!(
        !events.operations.contains(&MovementOp::ShieldUp),
        "ShieldUp should not fire without the ability"
    );
}

// ── The air dodge ───────────────────────────────────────────────────────────
//
// The aerial evade is a DIFFERENT maneuver from the ground roll, and these
// tests exist to keep it that way: a roll that merely learned to fire in the
// air would pass a "dodge works airborne" assertion while giving the body a
// floor-hugging slide, no once-per-airtime budget, and an animation nobody can
// distinguish. Each test below names the property that separates them.

/// The stick + the dash button in the air.
fn air_dodge_input(x: f32, y: f32) -> InputState {
    InputState {
        movement: crate::ActionEdges::EMPTY.with(
            crate::MovementAction::Dash,
            crate::Edge {
                pressed: true,
                held: false,
                released: false,
            },
        ),
        ..InputState::with_axes(x, y)
    }
}

/// Put the body well clear of the floor so the airborne branch is the one under
/// test rather than a body that is technically off the ground for one tick.
fn airborne_scratch(world: &crate::World) -> BodyClusterScratch {
    let mut scratch = scratch_at(world.spawn);
    scratch.kinematics.pos.y = world.size.y - 400.0;
    scratch.ground.on_ground = false;
    scratch
}

/// **A body that AUTHORS an air dodge.** The default tuning does not have one
/// (an airborne dash press is the air dash for every exploration body), so
/// every fixture below states the window it is testing — which is also the
/// production shape: a fighter authors it, a wanderer does not.
fn air_dodge_tuning() -> crate::test_support::TestTuning {
    let mut tuning = crate::test_support::TEST_TUNING;
    tuning.air_dodge_time = crate::movement::AIR_DODGE_TIME;
    tuning
}

fn step_air_dodger(
    world: &crate::World,
    scratch: &mut BodyClusterScratch,
    input: InputState,
) -> FrameEvents {
    crate::test_support::update_player_with_tuning_scratch(
        world,
        scratch,
        input,
        1.0 / 60.0,
        air_dodge_tuning(),
    )
}

/// **An air dodge is not a roll fired off the ground.** It publishes its own
/// op, sets its own timer, leaves the roll's timer alone, and travels where the
/// STICK points — here down-and-forward, which a roll (side-only, and gated on
/// `on_ground`) cannot produce at all.
#[test]
fn an_air_dodge_travels_along_the_stick_and_is_its_own_maneuver() {
    let world = test_world();
    let mut scratch = airborne_scratch(&world);
    // Local `y` points toward the FEET (the drop-through convention), so this
    // stick is "forward and down" — the recovery-relevant direction.
    let events = step_air_dodger(&world, &mut scratch, air_dodge_input(0.7, 0.7));
    assert!(
        events.operations.contains(&MovementOp::AirDodge),
        "airborne dash with the dodge ability is an air dodge, got {:?}",
        events.operations
    );
    assert!(
        !events.operations.contains(&MovementOp::DodgeRoll),
        "and NOT a ground roll"
    );
    assert!(scratch.axis().air_dodge_timer > 0.0, "its own window opens");
    assert_eq!(
        scratch.axis().dodge_roll_timer,
        0.0,
        "the roll's timer is untouched — the two maneuvers do not share state"
    );
    let vel = scratch.kinematics.vel;
    assert!(
        vel.x > 100.0 && vel.y > 100.0,
        "the evade aims down-forward with the stick (world +y is toward the          feet under this fixture's gravity), got {vel:?}"
    );
}

/// **A neutral stick still dodges** — in place. The invulnerability is the
/// option; refusing the input on a centred stick would make the maneuver
/// unusable for exactly the defensive situation it exists for.
#[test]
fn a_neutral_stick_air_dodges_in_place() {
    let world = test_world();
    let mut scratch = airborne_scratch(&world);
    let events = step_air_dodger(&world, &mut scratch, air_dodge_input(0.0, 0.0));
    assert!(events.operations.contains(&MovementOp::AirDodge));
    assert!(
        scratch.kinematics.vel.x.abs() < 1.0,
        "no travel, got {:?}",
        scratch.kinematics.vel
    );
}

/// **One per trip through the air, and landing is what gives it back.** The
/// budget lives on the dodge cluster, and the refresh rides the same authority
/// that restores air jumps — so this test fails both if the budget is missing
/// and if the refresh was written as a separate call somebody forgot.
#[test]
fn the_air_dodge_is_spent_for_the_airtime_and_returns_on_landing() {
    let world = test_world();
    let mut scratch = airborne_scratch(&world);
    let first = step_air_dodger(&world, &mut scratch, air_dodge_input(1.0, 0.0));
    assert!(first.operations.contains(&MovementOp::AirDodge));
    assert!(scratch.dodge.air_dodge_spent, "the budget is spent");

    // Let the window and its endlag expire — WITHOUT letting the body land,
    // because landing is precisely what refunds the budget this test is about.
    for _ in 0..26 {
        step_air_dodger(&world, &mut scratch, InputState::default());
        assert!(!scratch.ground.on_ground, "the fixture stayed airborne");
    }
    let second = step_air_dodger(&world, &mut scratch, air_dodge_input(-1.0, 0.0));
    assert!(
        !second.operations.contains(&MovementOp::AirDodge),
        "a second air dodge in the SAME airtime is refused"
    );

    // Land. `on_ground` is the environment's fact; the refresh is the kernel's.
    scratch.kinematics.pos = world.spawn;
    scratch.kinematics.vel = Vec2::ZERO;
    for _ in 0..60 {
        step_air_dodger(&world, &mut scratch, InputState::default());
        if scratch.ground.on_ground {
            break;
        }
    }
    assert!(scratch.ground.on_ground, "the fixture actually landed");
    assert!(
        !scratch.dodge.air_dodge_spent,
        "landing restores the air dodge"
    );
}

/// **The i-frames are real, and they END before the maneuver does.** The window
/// grants invulnerability through the ONE `evading()` term the damage rule
/// reads; the endlag that follows is the punish window, and it must NOT be
/// invulnerable or the option has no cost.
#[test]
fn the_air_dodge_window_grants_i_frames_and_its_endlag_does_not() {
    let world = test_world();
    let mut scratch = airborne_scratch(&world);
    step_air_dodger(&world, &mut scratch, air_dodge_input(1.0, 0.0));
    let facts = crate::movement::BodyMotionFacts::from_model(&scratch.model);
    assert!(facts.air_dodging, "the aerial fact is published");
    assert!(!facts.dodge_rolling, "and it is not the roll's fact");
    assert!(facts.evading(), "so the damage rule sees an evade");

    // Step past the window; the endlag opens the tick it closes.
    let mut endlag_seen = false;
    for _ in 0..30 {
        step_air_dodger(&world, &mut scratch, InputState::default());
        scratch.ground.on_ground = false;
        let facts = crate::movement::BodyMotionFacts::from_model(&scratch.model);
        if facts.air_dodge_endlag {
            endlag_seen = true;
            assert!(
                !facts.evading(),
                "endlag is committed but VULNERABLE — that is the cost"
            );
        }
    }
    assert!(endlag_seen, "the endlag state was reached at all");
}

/// **A body that authors NO window does not air dodge** — its airborne dash
/// press stays the air dash it always was. This is the regression that keeps
/// the maneuver from being a silent, game-wide change of what a button means.
#[test]
fn a_body_without_an_authored_window_has_no_air_dodge() {
    let world = test_world();
    let mut scratch = airborne_scratch(&world);
    assert_eq!(
        crate::movement::DEFAULT_TUNING.air_dodge_time,
        0.0,
        "the engine default is OFF; a character opts in"
    );
    let events = step_scratch(&world, &mut scratch, air_dodge_input(1.0, 0.0));
    assert!(
        !events.operations.contains(&MovementOp::AirDodge),
        "no authored window, no air dodge, got {:?}",
        events.operations
    );
    assert!(
        events.operations.contains(&MovementOp::Dash)
            || events.operations.contains(&MovementOp::DoubleDash),
        "and the press still means the air dash it always meant (`DoubleDash` is \
         the airborne charge), got {:?}",
        events.operations
    );
}
