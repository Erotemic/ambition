//! Dodge roll trigger / cooldown / ability gate, and shield/parry
//! activation, deactivation, dash conflict, parry window reset.

use super::super::*;
use super::{step_scratch, test_world};
use crate::body_clusters::BodyClusterScratch;
use crate::test_support::{update_player_with_tuning_scratch, TEST_TUNING};
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
                crate::MovementAction::Burst,
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
                crate::MovementAction::Burst,
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
                crate::MovementAction::Burst,
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

/// THE STICK REACHES THE GUARD — the wiring, not the rule.
///
/// [`ambition_platformer2d::combat::util::guard_covers_hit`] is unit-tested against a tilt
/// handed to it directly, which proves the coverage MATHS and nothing about
/// whether a held stick ever produces one. This is the other half: the kernel
/// resolves the lean, so the value the hit test reads is a real input.
///
/// A GENTLE stick, deliberately: past `SPOT_DODGE_STICK` the evade takes the
/// input first, so the band a tilting player actually lives in is this one.
///
/// ⛔ `LocalAxes` is `+y TOWARD-FEET`, so the DOWN stick here is POSITIVE. An
/// earlier draft of this test read it the other way, fed `-0.3` for "down", and
/// passed — against a `tilt_bias` that was inverted to match. The direction is
/// asserted against the named axis, not against what the stick felt like.
#[test]
fn a_gentle_stick_leans_the_raised_guard_and_letting_go_recentres_it() {
    let world = test_world();
    let mut tuning = TEST_TUNING;
    tuning.base.shield.tilt_range = 0.34;
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.abilities.abilities.shield = true;

    let mut step = |input: InputState, scratch: &mut BodyClusterScratch| {
        update_player_with_tuning_scratch(&world, scratch, input, 1.0 / 60.0, tuning);
    };

    // LAND FIRST. `on_ground` is re-derived every step, so setting it above
    // only holds until the body's real height is consulted — and a guard
    // resolved in mid-air is not the case this measures.
    for _ in 0..40 {
        step(InputState::default(), &mut scratch);
    }
    assert!(
        scratch.ground.on_ground,
        "the fixture never reached the floor, so nothing below measures a GROUNDED guard"
    );

    // BASELINE: a raised guard with a centred stick leans nowhere, so a
    // rule that simply wrote a constant could not pass the arms below.
    step(
        InputState {
            shield_held: true,
            ..Default::default()
        },
        &mut scratch,
    );
    assert!(scratch.shield.active);
    assert_eq!(
        scratch.shield.shield_tilt, 0.0,
        "a centred stick leaned the guard"
    );

    let mut gentle_down = InputState::with_axes(0.0, 0.3);
    gentle_down.shield_held = true;
    step(gentle_down, &mut scratch);
    assert!(
        scratch.shield.shield_tilt > 0.0,
        "a gentle DOWN stick did not lean the guard toward the feet: {}",
        scratch.shield.shield_tilt
    );
    assert!(
        scratch.ground.on_ground && scratch.shield.active,
        "the fixture stopped guarding on the ground, so the lean above measured nothing"
    );

    let mut gentle_up = InputState::with_axes(0.0, -0.3);
    gentle_up.shield_held = true;
    step(gentle_up, &mut scratch);
    assert!(
        scratch.shield.shield_tilt < 0.0,
        "a gentle UP stick did not lean the guard toward the head"
    );

    // AND A LOWERED GUARD KEEPS NO LEAN — otherwise the next raise starts
    // already leaning at whatever the stick happened to be doing.
    step(InputState::with_axes(0.0, 0.3), &mut scratch);
    assert!(!scratch.shield.active);
    assert_eq!(
        scratch.shield.shield_tilt, 0.0,
        "a guard that is DOWN kept a stale lean"
    );
}

/// GUARD + DOWN IS THE SPOT DODGE ON SOLID GROUND AND THE PLATFORM DROP ON A
/// SOFT ONE — the terrain arbitrates, and nothing else about the press changes.
///
/// Three arms, because two of them alone would not pin the mechanic: a rule
/// that always dropped would pass the drop arm, and a rule that never did
/// would pass the other two. The declaration arm is what keeps every
/// exploration world — none of which authors `platform_drop` — exactly where it
/// was.
#[test]
fn guard_and_down_drops_through_a_soft_platform_but_spot_dodges_on_solid_ground() {
    use crate::world::{Block, World};
    // ⛔ `DEFAULT_GRAVITY_DIR` is `+y`, so DOWN is INCREASING y here: the body
    // starts high (small y), the platform is below it at 300, and the floor is
    // below that at 500. Feet are the `+y` edge.
    let world = World {
        name: "platform drop".to_string(),
        size: Vec2::new(800.0, 600.0),
        spawn: Vec2::new(400.0, 100.0),
        blocks: vec![
            Block::solid("floor", Vec2::new(0.0, 500.0), Vec2::new(800.0, 40.0)),
            Block::one_way("platform", Vec2::new(300.0, 300.0), Vec2::new(200.0, 12.0)),
        ],
        climbable_regions: Vec::new(),
        chains: Vec::new(),
        edges: Default::default(),
        water_regions: Vec::new(),
    };
    // +y in LOCAL axes is TOWARD-FEET, so this is DOWN — and it is past
    // `SPOT_DODGE_STICK`, so both roads genuinely want this press.
    let mut guard_and_down = InputState::with_axes(0.0, 0.8);
    guard_and_down.shield_held = true;

    let feet =
        |scratch: &BodyClusterScratch| scratch.kinematics.pos.y + scratch.kinematics.size.y * 0.5;

    let land_on_the_platform = |tuning: crate::test_support::TestTuning| {
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), Vec2::new(400.0, 100.0));
        scratch.ground.on_ground = false;
        for _ in 0..120 {
            update_player_with_tuning_scratch(
                &world,
                &mut scratch,
                InputState::default(),
                1.0 / 60.0,
                tuning,
            );
        }
        // EVERY arm checks its own premise. Asserting this for one of them and
        // trusting the rest is how a fixture that never reached the platform
        // gets read as a mechanic that misfired.
        assert!(
            scratch.ground.on_ground
                && (scratch.kinematics.pos.y + scratch.kinematics.size.y * 0.5 - 300.0).abs() < 6.0,
            "the fixture never landed on the one-way platform: on_ground={} feet={}",
            scratch.ground.on_ground,
            scratch.kinematics.pos.y + scratch.kinematics.size.y * 0.5
        );
        scratch
    };

    // ⛔ THE SPOT-DODGE WINDOW COMES WITH THE SHIELD, and leaving it at the
    // engine default silently changes what guard + down MEANS: `apply_dodge`
    // falls through to the ROLL when `spot_dodge_time` is zero, so arm 3 would
    // have been measuring a body rolling off the stage rather than dodging in
    // place. A fixture that declares half a ruleset is not that ruleset.
    let mut smash = TEST_TUNING;
    smash.base.shield = crate::ShieldTuning::PLATFORM_FIGHTER;
    smash.base.spot_dodge_time = 0.16;
    let mut no_declaration = smash;
    no_declaration.base.shield.platform_drop = false;

    // The fixture has to actually be ON the platform, or every arm below is
    // measuring a body in mid-air.
    let mut dropping = land_on_the_platform(smash);
    assert!(
        dropping.ground.on_ground && (feet(&dropping) - 300.0).abs() < 6.0,
        "the fixture never landed on the one-way platform: on_ground={} feet={}",
        dropping.ground.on_ground,
        feet(&dropping)
    );

    // ARM 1 — THE DROP. Guard + down leaves the soft platform behind.
    for _ in 0..90 {
        update_player_with_tuning_scratch(&world, &mut dropping, guard_and_down, 1.0 / 60.0, smash);
    }
    assert!(
        feet(&dropping) > 400.0,
        "guard + down did not drop through the platform: feet={}",
        feet(&dropping)
    );

    // ARM 2 — THE DECLARATION. The same press on the same platform, for a body
    // whose ruleset does not author the drop, keeps it standing.
    let mut staying = land_on_the_platform(no_declaration);
    for _ in 0..90 {
        update_player_with_tuning_scratch(
            &world,
            &mut staying,
            guard_and_down,
            1.0 / 60.0,
            no_declaration,
        );
    }
    assert!(
        (feet(&staying) - 300.0).abs() < 6.0,
        "a body that declares no platform drop fell through anyway: feet={}",
        feet(&staying)
    );

    // ARM 3 — SOLID GROUND STILL SPOT-DODGES. The press is not consumed by the
    // drop road on a floor that cannot be left downward.
    let mut grounded = scratch_with(AbilitySet::sandbox_all(), Vec2::new(100.0, 100.0));
    grounded.ground.on_ground = false;
    for _ in 0..120 {
        update_player_with_tuning_scratch(
            &world,
            &mut grounded,
            InputState::default(),
            1.0 / 60.0,
            smash,
        );
    }
    assert!(grounded.ground.on_ground, "arm 3 never reached the floor");
    let floor_feet = feet(&grounded);
    update_player_with_tuning_scratch(&world, &mut grounded, guard_and_down, 1.0 / 60.0, smash);
    assert!(
        grounded.axis().dodge_roll_timer > 0.0 || grounded.axis().buffer_burst > 0.0,
        "guard + down on solid ground did not start an evade"
    );
    assert!(
        (feet(&grounded) - floor_feet).abs() < 6.0,
        "guard + down on solid ground moved the body off the floor"
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
            crate::MovementAction::Burst,
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

/// A body that AUTHORS an air dodge. The default tuning does not have one
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

/// ⭐⭐ SHIELD IN THE AIR IS THE AIR DODGE — the genre's rule, and the last line
/// of the "dodges come off the shield button" table.
///
/// The grounded evade needs a DIRECTION because a guard is the other thing the
/// press could have meant. Airborne there is nothing else, so a bare press is
/// the whole gesture.
///
/// ⛔ THE OTHER HALF IS ASSERTED HERE TOO, which is why this is one test: a
/// press that both air-dodged and raised a guard would be neither mechanic.
#[test]
fn a_shield_press_in_the_air_is_the_air_dodge_and_raises_no_guard() {
    let world = test_world();
    let mut scratch = airborne_scratch(&world);
    let mut tuning = air_dodge_tuning();
    tuning.base.shield = crate::ShieldTuning::PLATFORM_FIGHTER;
    let events = crate::test_support::update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            ..Default::default()
        },
        1.0 / 60.0,
        tuning,
    );
    assert!(
        events.operations.contains(&MovementOp::AirDodge),
        "a shield press in the air did nothing: {:?}",
        events.operations
    );
    assert!(
        !scratch.shield.active,
        "the same press also put a guard up, in a genre that has none in the air"
    );
    assert!(
        scratch.dodge.air_dodge_spent,
        "the dodge was not spent, so a held button would buy another every tick"
    );
}

/// A ruleset that ALLOWS an airborne guard is untouched: the press raises the
/// shield and buys no dodge.
///
/// ⛔ this is Ambition, not a hypothetical. `sustain_bubble_shield` forces
/// `shield_held` for the whole `bubble_shield` special and that special is not
/// grounded-gated, so a blanket genre law would have taken the protagonist's
/// signature defensive move away mid-jump.
#[test]
fn a_deployable_bubble_still_guards_in_the_air() {
    let world = test_world();
    let mut scratch = airborne_scratch(&world);
    let mut tuning = air_dodge_tuning();
    tuning.base.shield = crate::ShieldTuning::PLATFORM_FIGHTER;
    tuning.base.shield.air_guard = true;
    let events = crate::test_support::update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            ..Default::default()
        },
        1.0 / 60.0,
        tuning,
    );
    assert!(
        scratch.shield.active,
        "a body whose ruleset allows an airborne guard did not get one"
    );
    assert!(
        !events.operations.contains(&MovementOp::AirDodge),
        "the guard ALSO air-dodged, which is the double meaning this rule exists \
         to prevent: {:?}",
        events.operations
    );
}

/// An air dodge is not a roll fired off the ground. It publishes its own
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

/// A neutral stick still dodges — in place. The invulnerability is the
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

/// One per trip through the air, and landing is what gives it back. The
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

/// The i-frames are real, and they END before the maneuver does. The window
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

/// A body that authors NO window does not air dodge — its airborne dash press stays the air
/// dash it always was.
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

// ── The floor game: tumble / knockdown / tech / getup ───────────────────────
//
// These pin the four states and the two ways out of the last one.

use crate::movement::knockdown;

fn floor_game_tuning() -> crate::test_support::TestTuning {
    let mut tuning = crate::test_support::TEST_TUNING;
    tuning.tumble_speed = 420.0;
    // a fighter's floor: the engine default is `0.0`, which means the grounded
    // evade is always the roll. A fixture that left it there would measure the
    // absence rather than the option.
    tuning.spot_dodge_time = crate::movement::tuning::SPOT_DODGE_TIME;
    tuning
}

fn step_fighter(
    world: &crate::World,
    scratch: &mut BodyClusterScratch,
    input: InputState,
) -> FrameEvents {
    crate::test_support::update_player_with_tuning_scratch(
        world,
        scratch,
        input,
        1.0 / 60.0,
        floor_game_tuning(),
    )
}

/// Launch the body the way a real knockback does — through the ONE channel the
/// kernel drains, never by writing the timer, because "was that big enough to
/// tumble" is the kernel's question to answer.
fn launch(world: &crate::World, scratch: &mut BodyClusterScratch, speed: f32) -> FrameEvents {
    scratch.kinematics.pos.y = world.size.y - 400.0;
    scratch.ground.on_ground = false;
    scratch.flight.pending_launch = Vec2::new(speed * 0.6, -speed * 0.8);
    step_fighter(world, scratch, InputState::default())
}

/// A small hit is not a tumble. The threshold is the whole reason a heavy
/// hit reads as heavy, so a launch under it must leave the body in ordinary
/// control — and a body that authors no threshold never tumbles at all.
#[test]
fn only_a_launch_over_the_authored_threshold_tumbles() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    let events = launch(&world, &mut scratch, 200.0);
    assert!(!events.operations.contains(&MovementOp::Tumble));
    assert_eq!(scratch.axis().tumble_timer, 0.0, "under the threshold");

    let mut scratch = scratch_at(world.spawn);
    let events = launch(&world, &mut scratch, 900.0);
    assert!(
        events.operations.contains(&MovementOp::Tumble),
        "over it, got {:?}",
        events.operations
    );
    assert!(scratch.axis().tumble_timer > 0.0);

    // The same launch on a body whose tuning says nothing: no floor game.
    let mut scratch = scratch_at(world.spawn);
    scratch.kinematics.pos.y = world.size.y - 400.0;
    scratch.ground.on_ground = false;
    scratch.flight.pending_launch = Vec2::new(540.0, -720.0);
    let events = step_scratch(&world, &mut scratch, InputState::default());
    assert!(
        !events.operations.contains(&MovementOp::Tumble),
        "an unauthored body keeps the movement it had"
    );
}

/// Every other case above launches a body that is ALREADY AIRBORNE — they each set `on_ground =
/// false` first — so the one situation a platform fighter is actually in when it gets hit, standing
/// on the stage, was never stepped. A launched body carried its stale resting contact into the same
/// step's `tick_knockdown`, which read `on_ground == true`, called that *touched down while still
/// tumbling*, and resolved it to a KNOCKDOWN: `vel = ZERO` on the tick the launch was applied.
///
/// the reason it hid for so long is the threshold. A hit UNDER `tumble_speed` never armed the
/// tumble, so it launched correctly — which is every hit in Ambition and every weak hit in
/// smash.
#[test]
fn a_launch_that_tumbles_a_standing_body_throws_it_instead_of_knocking_it_down() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    // RESTING, not merely near the floor: loop on the property.
    let mut standing = false;
    for _ in 0..600 {
        step_fighter(&world, &mut scratch, InputState::default());
        if scratch.ground.on_ground {
            standing = true;
            break;
        }
    }
    assert!(
        standing,
        "the body never came to rest, so nothing below is about a STANDING body"
    );

    scratch.flight.pending_launch = Vec2::new(540.0, -720.0);
    let events = step_fighter(&world, &mut scratch, InputState::default());

    assert!(
        events.operations.contains(&MovementOp::Tumble),
        "this launch has to clear the tumble threshold or the case is not the          one that broke: {:?}",
        events.operations
    );
    assert!(
        !events.operations.contains(&MovementOp::Knockdown),
        "the tick a body is LAUNCHED is not the tick it lands: {:?}",
        events.operations
    );
    assert_eq!(
        scratch.axis().knockdown_timer,
        0.0,
        "a body thrown off the floor is not prone"
    );
    assert!(
        !scratch.ground.on_ground,
        "a thrown body is not resting on anything"
    );
    assert!(
        scratch.kinematics.vel.y < -600.0,
        "and the launch itself has to survive the step that applied it: {:?}",
        scratch.kinematics.vel
    );
}

/// Landing while tumbling is a knockdown, and the prone body has no control.
/// Without this a launch is just a shove: nothing to punish, nothing to escape.
#[test]
fn a_tumbling_body_that_lands_is_knocked_down_and_stands_up_on_its_own() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    launch(&world, &mut scratch, 900.0);
    // Fall to the floor.
    let mut knocked = false;
    for _ in 0..200 {
        let events = step_fighter(&world, &mut scratch, InputState::default());
        if events.operations.contains(&MovementOp::Knockdown) {
            knocked = true;
            break;
        }
    }
    assert!(knocked, "the landing resolved into a knockdown");
    assert!(scratch.axis().knockdown_timer > 0.0);

    // Prone bodies do not run. A LEAN (under the getup-roll threshold) is the
    // honest probe: a full stick is a getup roll, which is a choice, not control.
    for _ in 0..6 {
        step_fighter(&world, &mut scratch, InputState::with_axes(0.4, 0.0));
    }
    assert!(
        scratch.axis().knockdown_timer > 0.0,
        "still prone — a lean under the roll threshold is not a getup"
    );
    assert!(
        scratch.kinematics.vel.x.abs() < 5.0,
        "a knocked-down body does not run, got {:?} after six frames of stick",
        scratch.kinematics.vel
    );
    let mut stood = false;
    for _ in 0..120 {
        let events = step_fighter(&world, &mut scratch, InputState::default());
        if events.operations.contains(&MovementOp::Getup) {
            stood = true;
            break;
        }
    }
    assert!(stood, "the knockdown ends on its own");
    assert!(
        scratch.axis().getup_invuln_timer > 0.0,
        "standing up is invulnerable, or getting up is a free hit for the winner"
    );
}

/// A tech refuses the knockdown, and the i-frames it grants are the SAME
/// `evading()` term every other evade uses.
#[test]
fn a_tech_on_the_landing_skips_the_knockdown_entirely() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    launch(&world, &mut scratch, 900.0);
    let dash = InputState {
        movement: crate::ActionEdges::EMPTY.with(
            crate::MovementAction::Burst,
            crate::Edge {
                pressed: true,
                held: false,
                released: false,
            },
        ),
        ..Default::default()
    };
    let mut teched = false;
    for _ in 0..200 {
        // Press only once the body is genuinely about to touch down: falling,
        // and within a third of the tech window of the floor at this speed.
        let rest_y = world.size.y - 48.0 - scratch.kinematics.size.y * 0.5;
        let close = scratch.kinematics.vel.y > 1.0
            && scratch.kinematics.pos.y + scratch.kinematics.vel.y * (knockdown::TECH_WINDOW / 3.0)
                >= rest_y;
        let events = step_fighter(
            &world,
            &mut scratch,
            if close { dash } else { InputState::default() },
        );
        if events.operations.contains(&MovementOp::Tech) {
            teched = true;
            break;
        }
        assert!(
            !events.operations.contains(&MovementOp::Knockdown),
            "a press inside the window must not land as a knockdown"
        );
    }
    assert!(teched, "the timed press teched the landing");
    assert_eq!(scratch.axis().knockdown_timer, 0.0, "no prone state at all");
    let facts = crate::movement::BodyMotionFacts::from_model(&scratch.model);
    assert!(
        facts.evading(),
        "a tech is invulnerable through the one term the damage rule reads"
    );
}

/// DOWN ON THE STICK EVADES IN PLACE.
///
/// the pair is the assertion: the same press with a SIDEWAYS stick must still
/// roll, or this is not a second option — it is the first one renamed. And the
/// spot dodge must not travel, which is the whole reason a cornered fighter
/// wants it.
#[test]
fn down_on_the_stick_spot_dodges_instead_of_rolling() {
    let evade = |stick: crate::LocalAxes| {
        let world = test_world();
        let mut scratch = scratch_at(world.spawn);
        scratch.ground.on_ground = true;
        scratch.dodge.cooldown = 0.0;
        let events = step_fighter(
            &world,
            &mut scratch,
            InputState {
                axes: stick,
                movement: crate::ActionEdges::EMPTY.with(
                    crate::MovementAction::Burst,
                    crate::Edge {
                        pressed: true,
                        held: false,
                        released: false,
                    },
                ),
                ..Default::default()
            },
        );
        (events.operations.clone(), scratch.kinematics.vel.x)
    };

    let (ops, travel) = evade(crate::LocalAxes::new(0.0, 1.0));
    assert!(
        ops.contains(&MovementOp::SpotDodge),
        "a held-down evade rolled: {ops:?}"
    );
    assert_eq!(travel, 0.0, "a spot dodge travelled {travel}px sideways");

    let (ops, travel) = evade(crate::LocalAxes::new(1.0, 0.0));
    assert!(
        ops.contains(&MovementOp::DodgeRoll),
        "a sideways evade spot-dodged, so the roll is gone: {ops:?}"
    );
    assert!(
        travel > 0.0,
        "the roll went nowhere, so this compared nothing"
    );
}

/// ⭐ DOWN ON A RAISED GUARD SPOT-DODGES WITH NO SECOND BUTTON. Jon,
/// 2026-08-23: *"Shielding and pressing down should trigger a dodge. But also
/// note that right now dodge isn't really a dodge, it is more like a dash. It
/// actually moves the player."*
///
/// Both halves are one gap, and the test above already shows why: the spot
/// dodge exists and does NOT travel. What did not exist is a way to ask for it
/// from behind a guard — the evade was reachable only through the burst button,
/// and a burst press with no direction is the ROLL, which travels. So the only
/// dodge a shielding player could reach was the one that moves them.
///
/// ⛔ THE PAIR IS THE ASSERTION. The same stick with the guard DOWN must do
/// nothing, or this is not an out-of-shield option — it is "holding down
/// dodges", which would fire while walking down a slope.
#[test]
fn down_on_a_raised_guard_spot_dodges_without_a_burst_press() {
    let evade = |guard_up: bool| {
        let world = test_world();
        let mut scratch = scratch_at(world.spawn);
        scratch.ground.on_ground = true;
        scratch.dodge.cooldown = 0.0;
        scratch.shield.active = guard_up;
        let events = step_fighter(
            &world,
            &mut scratch,
            InputState {
                axes: crate::LocalAxes::new(0.0, 1.0),
                // ⛔ THE BUTTON, not the body's guard state — the evade reads
                // `shield_held`, because a guard OUTLIVES the press that raised
                // it and an evade off a shield you already let go of is the
                // wrong answer. A fixture that set only `shield.active` would be
                // testing the version that was wrong.
                shield_held: guard_up,
                // NO burst edge at all: the stick and the button are the whole
                // input.
                ..Default::default()
            },
        );
        (events.operations.clone(), scratch.kinematics.vel.x)
    };

    let (ops, travel) = evade(true);
    assert!(
        ops.contains(&MovementOp::SpotDodge),
        "down on a raised guard did not spot dodge: {ops:?}"
    );
    assert_eq!(travel, 0.0, "the out-of-shield dodge travelled {travel}px");

    let (ops, _) = evade(false);
    assert!(
        !ops.contains(&MovementOp::SpotDodge),
        "holding down with NO guard dodged, so this fires while walking downhill: {ops:?}"
    );
}

/// ⛔ HOLDING SHIELD ROOTS A GROUNDED BODY. Jon, 2026-08-23: *"If the player is
/// holding shield... they should not be let the control move them left or
/// right."*
///
/// It is what makes shield+direction mean ROLL rather than "shuffle sideways
/// with the guard up". The pair is the assertion: the same stick with the button
/// UP must still walk, or this rooted the body for a reason other than the
/// guard.
#[test]
fn holding_shield_stops_the_stick_from_walking_you() {
    let walk = |shield_held: bool| {
        let world = test_world();
        let mut scratch = scratch_at(world.spawn);
        for _ in 0..8 {
            // RE-ASSERTED EVERY STEP: the rule is about a body STANDING on a
            // floor, and the step re-samples ground contact. Setting it once
            // before the loop measured an airborne body, which is not what this
            // is about — air control is deliberately outside the rule.
            scratch.ground.on_ground = true;
            // ⛔ THE DODGE IS HELD ON COOLDOWN so this measures WALKING and not
            // the roll. Shield+direction rolling is the rule directly above, and
            // a roll SETS velocity — a test that let one fire would be reading
            // `dodge_roll_speed` and calling it walking. What is being asked
            // here is the other half: with no evade available, does the stick
            // still steer a guarded body? It must not.
            scratch.dodge.cooldown = 1.0;
            step_fighter(
                &world,
                &mut scratch,
                InputState {
                    axes: crate::LocalAxes::new(1.0, 0.0),
                    shield_held,
                    ..Default::default()
                },
            );
        }
        scratch.kinematics.vel.x
    };

    assert_eq!(
        walk(true),
        0.0,
        "a grounded body walked while holding shield"
    );
    assert!(
        walk(false) > 0.0,
        "the same stick with no guard did not walk either, so the guard is not what stopped it"
    );
}

/// A WALL IS SOMETHING YOU CAN CATCH YOURSELF ON TOO.
///
/// both halves — the tech FIRES while the body is still airborne, and the velocity it leaves
/// with points AWAY from the wall. A version that only cleared the tumble would leave the body
/// pinned to the surface it just teched off.
#[test]
fn a_tumbling_body_can_tech_off_a_wall() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    // Beside the right wall, high enough that the floor is not the first thing
    // it reaches — the point is that a WALL answered the press.
    scratch.kinematics.pos = Vec2::new(world.size.x - 120.0, world.size.y - 500.0);
    scratch.ground.on_ground = false;
    scratch.flight.pending_launch = Vec2::new(1400.0, -260.0);
    step_fighter(&world, &mut scratch, InputState::default());
    assert!(
        scratch.axis().tumble_timer > 0.0,
        "the launch did not tumble the body, so this measures nothing"
    );

    let dash = InputState {
        movement: crate::ActionEdges::EMPTY.with(
            crate::MovementAction::Burst,
            crate::Edge {
                pressed: true,
                held: false,
                released: false,
            },
        ),
        ..Default::default()
    };
    let mut teched = false;
    for _ in 0..90 {
        let on_wall = scratch.wall.on_wall;
        let events = step_fighter(
            &world,
            &mut scratch,
            if on_wall { dash } else { InputState::default() },
        );
        assert!(
            !events.operations.contains(&MovementOp::Knockdown),
            "it reached the floor before it ever touched the wall"
        );
        if events.operations.contains(&MovementOp::Tech) {
            teched = true;
            break;
        }
    }
    assert!(teched, "a timed press against a wall did not tech");
    assert!(
        !scratch.ground.on_ground,
        "this teched off the FLOOR, which the test above already covers"
    );
    assert_eq!(
        scratch.axis().tumble_timer,
        0.0,
        "the tumble survived the tech"
    );
    assert!(
        scratch.kinematics.vel.x < 0.0,
        "the tech left the body pinned against the wall it came off ({:?})",
        scratch.kinematics.vel
    );
    let facts = crate::movement::BodyMotionFacts::from_model(&scratch.model);
    assert!(
        facts.evading(),
        "a wall tech is invulnerable through the one term the damage rule reads"
    );
}

/// A tech guessed too early costs the option. Mashing has to be worse than
/// reading, or the knockdown is decorative.
#[test]
fn a_mistimed_tech_locks_the_option_out() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    launch(&world, &mut scratch, 900.0);
    let dash = InputState {
        movement: crate::ActionEdges::EMPTY.with(
            crate::MovementAction::Burst,
            crate::Edge {
                pressed: true,
                held: false,
                released: false,
            },
        ),
        ..Default::default()
    };
    step_fighter(&world, &mut scratch, dash);
    assert!(scratch.axis().tech_press_timer > 0.0, "the press is live");
    // Ride the window out in the air.
    for _ in 0..((knockdown::TECH_WINDOW * 60.0) as i32 + 2) {
        step_fighter(&world, &mut scratch, InputState::default());
    }
    assert!(
        scratch.axis().tech_lockout_timer > 0.0,
        "the guess expired into a lockout"
    );
    // A second press inside the lockout buys nothing.
    step_fighter(&world, &mut scratch, dash);
    assert_eq!(
        scratch.axis().tech_press_timer,
        0.0,
        "no tech while locked out"
    );
}

/// A getup is a CHOICE: hold a direction and you roll out of the knockdown
/// instead of standing in place, with the same invulnerability.
#[test]
fn a_held_direction_rolls_out_of_the_knockdown() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    launch(&world, &mut scratch, 900.0);
    for _ in 0..200 {
        let events = step_fighter(&world, &mut scratch, InputState::default());
        if events.operations.contains(&MovementOp::Knockdown) {
            break;
        }
    }
    assert!(scratch.axis().knockdown_timer > 0.0, "prone first");
    let events = step_fighter(&world, &mut scratch, InputState::with_axes(-1.0, 0.0));
    assert!(
        events.operations.contains(&MovementOp::GetupRoll),
        "got {:?}",
        events.operations
    );
    assert_eq!(scratch.axis().knockdown_timer, 0.0, "the roll ends it");
    assert!(scratch.axis().getup_invuln_timer > 0.0);
}

/// A caught parry is a BEAT, not a latch: it decays on the body's own clock
/// like every other timer beside it.
///
/// Without this the flag armed at the strike seam would stay true for the rest
/// of the match and the cue would fire once and never stop.
#[test]
fn a_caught_parry_decays_like_every_other_timer() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.shield.catch_parry();
    assert!(scratch.shield.parry_caught());

    let armed = scratch.shield.parry_caught_timer;
    step_scratch(&world, &mut scratch, InputState::default());
    assert!(
        scratch.shield.parry_caught_timer < armed,
        "the caught-parry beat did not decay, so it is a latch that never clears"
    );
    // Long enough that any plausible window is over.
    for _ in 0..120 {
        step_scratch(&world, &mut scratch, InputState::default());
    }
    assert!(!scratch.shield.parry_caught());
}

/// A body playing by the platform-fighter shield rule.
fn guarded_scratch(world: &crate::World) -> BodyClusterScratch {
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.abilities.abilities.shield = true;
    scratch
}

fn shield_step(world: &crate::World, scratch: &mut BodyClusterScratch, held: bool) -> FrameEvents {
    let mut tuning = TEST_TUNING;
    tuning.base.shield = crate::ShieldTuning::PLATFORM_FIGHTER;
    update_player_with_tuning_scratch(
        world,
        scratch,
        InputState {
            shield_held: held,
            ..Default::default()
        },
        1.0 / 60.0,
        tuning,
    )
}

/// ⭐ A GUARD IS A LAUNCHING PLATFORM, and leaving it SPENDS it.
///
/// Both halves, because either alone is the bug: a jump that the guard refuses
/// is not the genre, and a jump that leaves the guard standing is a body
/// attacking from behind a shield it keeps — which is why nobody would ever
/// lower one.
#[test]
fn a_jump_out_of_shield_is_allowed_and_takes_the_guard_with_it() {
    let world = test_world();
    let mut scratch = guarded_scratch(&world);
    for _ in 0..10 {
        shield_step(&world, &mut scratch, true);
    }
    assert!(scratch.shield.active, "the fixture never raised a guard");

    let mut tuning = TEST_TUNING;
    tuning.base.shield = crate::ShieldTuning::PLATFORM_FIGHTER;
    let events = update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Jump,
                crate::Edge {
                    pressed: true,
                    held: true,
                    released: false,
                },
            ),
            ..Default::default()
        },
        1.0 / 60.0,
        tuning,
    );
    assert!(
        events.operations.contains(&MovementOp::Jump) || scratch.axis().jump_squat_timer > 0.0,
        "the guard refused a jump out of shield, which is the one option every \
         other one in this genre is measured against"
    );
    assert!(
        !scratch.shield.active,
        "the body jumped and kept its guard up"
    );

    // ... and it stays down while the button is still held: a spent guard that
    // re-raised itself would hand back a fresh parry window every time.
    shield_step(&world, &mut scratch, true);
    assert!(
        !scratch.shield.active && scratch.shield.parry_window_timer <= 0.0,
        "the spent guard came straight back up under the same press"
    );
    // ⛔ AND NOT IN THE AIR, which is where the jump just put it.
    // `ShieldTuning::PLATFORM_FIGHTER` sets `air_guard: false`: no title in this
    // genre has an airborne shield, and the same press up there is the air
    // dodge. It also keeps the jump-out-of-shield SPEND honest — a body that
    // could re-guard mid-rise never really paid it.
    shield_step(&world, &mut scratch, false);
    for _ in 0..4 {
        shield_step(&world, &mut scratch, true);
    }
    assert!(
        !scratch.ground.on_ground,
        "the fixture landed before it could measure the airborne refusal"
    );
    assert!(
        !scratch.shield.active,
        "a guard went up in mid-air, where this genre has none"
    );

    // Back on the ground the same release-and-press IS a new guard. The rule was
    // never about altitude — it is that a SPENT guard stays down under the press
    // that spent it — so this is the half that still has to hold.
    //
    // ⛔ AND IT LANDS FOR REAL rather than having `on_ground` written true. It
    // used to, and that made this stanza pass for the wrong reason: the forced
    // flag bought one grounded RAISE, and the guard then survived every airborne
    // step after it because the sustain was not gated. The falsifier and the bug
    // agreed.
    for _ in 0..240 {
        shield_step(&world, &mut scratch, false);
        if scratch.ground.on_ground {
            break;
        }
    }
    assert!(
        scratch.ground.on_ground,
        "the fixture never came back down, so the grounded half is unmeasured"
    );
    for _ in 0..4 {
        shield_step(&world, &mut scratch, true);
    }
    assert!(
        scratch.shield.active,
        "a released and re-pressed button did not buy a new guard on the ground"
    );
}

/// A game that declares no out-of-shield rule is untouched by all of it: the
/// guard forbids nothing and acting does not spend it, which is what every body
/// in Ambition did before the policy existed.
#[test]
fn a_game_with_no_out_of_shield_rule_is_unchanged() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.abilities.abilities.shield = true;
    // `ShieldTuning::OFF` — the engine baseline — declares no policy.
    for _ in 0..10 {
        step_scratch(
            &world,
            &mut scratch,
            InputState {
                shield_held: true,
                ..Default::default()
            },
        );
    }
    assert!(scratch.shield.active);
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            shield_held: true,
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Jump,
                crate::Edge {
                    pressed: true,
                    held: true,
                    released: false,
                },
            ),
            ..Default::default()
        },
    );
    assert!(
        scratch.shield.active,
        "a game with no out-of-shield rule spent a guard on an action anyway"
    );
    assert_eq!(
        scratch.shield.drop_lag_timer, 0.0,
        "a game with no drop rule was charged for one"
    );
}

/// Letting a guard down by itself costs the commitment the policy authors —
/// the other half of what makes holding one a decision.
#[test]
fn dropping_a_guard_costs_the_authored_lag() {
    let world = test_world();
    let mut scratch = guarded_scratch(&world);
    for _ in 0..10 {
        shield_step(&world, &mut scratch, true);
    }
    assert!(scratch.shield.active);
    shield_step(&world, &mut scratch, false);
    assert!(
        scratch.shield.drop_lag_timer > 0.0,
        "letting the guard go cost nothing, so holding one commits to nothing"
    );
}

/// THE WALL TECH JUMP already works, and nothing said so.
///
/// ⚠ WHAT THIS TEST IS FOR. I set out to ADD the wall tech jump and wrote the
/// impulse for it; measured against the same tech without the press, the rise
/// was `-554.7` either way — byte-identical with my code and with it deleted.
/// The existing wall-jump path already answers a jump press against a wall, so
/// the line was dead the moment it was written. It is gone, and this is what
/// stayed: the behaviour was real and undefended, which is the more useful half.
///
/// A wall tech alone leaves the body travelling sideways at whatever height the
/// wall caught it; asking for a jump in the same beat leaves it going UP as
/// well, and it costs no air jump — the fixture spends them all first — which
/// is the difference between surviving the wall and getting something back
/// from it.
///
/// Measured as a DIFFERENCE against the same tech without the press, because
/// the absolute sign proves nothing: a body teching a wall mid-rise is already
/// going up, and an assertion on `vel.y < 0` passes with the rise deleted. That
/// is the version of this test I wrote first, and it is how the dead line got
/// as far as it did.
#[test]
fn a_wall_tech_asked_to_jump_leaves_the_wall_higher_than_one_that_is_not() {
    /// Tech off the right wall, optionally asking for a jump, and report the
    /// vertical velocity the tech left behind.
    fn wall_tech_rise(world: &crate::World, also_jump: bool) -> f32 {
        let mut scratch = scratch_at(world.spawn);
        scratch.kinematics.pos = Vec2::new(world.size.x - 120.0, world.size.y - 500.0);
        scratch.ground.on_ground = false;
        scratch.flight.pending_launch = Vec2::new(1400.0, -260.0);
        step_fighter(world, &mut scratch, InputState::default());
        assert!(
            scratch.axis().tumble_timer > 0.0,
            "the launch did not tumble the body, so this measures nothing"
        );
        // Spend every air jump: the wall tech's rise is bought by the press
        // against the surface, not from what a launch left.
        scratch.jump.air_jumps_available = 0;

        let mut press = InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Burst,
                crate::Edge {
                    pressed: true,
                    held: false,
                    released: false,
                },
            ),
            ..Default::default()
        };
        if also_jump {
            press.movement.set(
                crate::MovementAction::Jump,
                crate::Edge {
                    pressed: true,
                    held: true,
                    released: false,
                },
            );
        }
        for _ in 0..90 {
            let on_wall = scratch.wall.on_wall;
            let events = step_fighter(
                world,
                &mut scratch,
                if on_wall {
                    press
                } else {
                    InputState::default()
                },
            );
            if events.operations.contains(&MovementOp::Tech) {
                if also_jump {
                    assert!(
                        events.operations.contains(&MovementOp::WallJump),
                        "the tech took the jump press and reported no wall jump: {:?}",
                        events.operations
                    );
                }
                return scratch.kinematics.vel.y;
            }
        }
        panic!("a timed press against a wall did not tech");
    }

    let world = test_world();
    let plain = wall_tech_rise(&world, false);
    let jumped = wall_tech_rise(&world, true);
    assert!(
        jumped < plain,
        "asking for a jump out of a wall tech bought no height \
         (plain {plain}, jumped {jumped}), so a wall is still a place launches end"
    );
}

/// ⭐ THE CEILING TECH. A body thrown into a ceiling kept its tumble, fell the
/// whole way back down helpless and arrived as a knockdown it had no say in —
/// one hit bought the attacker the ceiling AND the landing.
#[test]
fn a_tumbling_body_can_tech_off_a_ceiling() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    // Under the ceiling, thrown straight up at it.
    scratch.kinematics.pos = Vec2::new(world.size.x * 0.5, 200.0);
    scratch.ground.on_ground = false;
    scratch.flight.pending_launch = Vec2::new(0.0, -1500.0);
    step_fighter(&world, &mut scratch, InputState::default());
    assert!(
        scratch.axis().tumble_timer > 0.0,
        "the launch did not tumble the body, so this measures nothing"
    );

    let evade = InputState {
        movement: crate::ActionEdges::EMPTY.with(
            crate::MovementAction::Burst,
            crate::Edge {
                pressed: true,
                held: false,
                released: false,
            },
        ),
        ..Default::default()
    };
    let mut teched = false;
    for _ in 0..90 {
        let on_ceiling = scratch.ground.head_contact;
        let events = step_fighter(
            &world,
            &mut scratch,
            if on_ceiling {
                evade
            } else {
                InputState::default()
            },
        );
        assert!(
            !events.operations.contains(&MovementOp::Knockdown),
            "it fell all the way back to the floor before ever reaching the \
             ceiling — this fixture measured nothing"
        );
        if events.operations.contains(&MovementOp::Tech) {
            teched = true;
            break;
        }
    }
    assert!(teched, "a timed press against a ceiling did not tech");
    assert!(
        !scratch.ground.on_ground,
        "this teched off the FLOOR, which another test already covers"
    );
    assert_eq!(
        scratch.axis().tumble_timer,
        0.0,
        "the tumble survived the ceiling tech, so the fall is still helpless"
    );
    let facts = crate::movement::BodyMotionFacts::from_model(&scratch.model);
    assert!(
        facts.evading(),
        "a ceiling tech is invulnerable through the one term the damage rule reads"
    );
}

/// ⛔⛔ A GROUND GUARD DOES NOT RIDE INTO THE AIR — and under `air_guard: false`
/// it MUST NOT, because the same held button is the air dodge up there.
///
/// The sustain used to be ungated (`may_guard_here || active`), on the argument
/// that a body which left the ground guarding had not made a new decision. But
/// leaving the ground with the button down is exactly how a body arrived at
/// `AirDodge` and `shield.active` in the same tick — the one state this policy
/// exists to forbid. The existing airborne test could not see it: it began
/// airborne with the guard already down.
///
/// ⭐ THE POISON IS THE OTHER HALF. `air_guard: true` is Ambition's own
/// deployable bubble, and it keeps the airborne guard — so a fix that simply
/// dropped every shield on takeoff fails the second assertion.
#[test]
fn a_ground_guard_does_not_survive_leaving_the_ground() {
    let world = test_world();
    let airborne_with_guard_held = |air_guard: bool| {
        let mut scratch = guarded_scratch(&world);
        let mut tuning = TEST_TUNING;
        tuning.base.shield = crate::ShieldTuning::PLATFORM_FIGHTER;
        tuning.base.shield.air_guard = air_guard;
        let mut step = |scratch: &mut BodyClusterScratch| {
            update_player_with_tuning_scratch(
                &world,
                scratch,
                InputState {
                    shield_held: true,
                    ..Default::default()
                },
                1.0 / 60.0,
                tuning,
            )
        };
        for _ in 0..10 {
            step(&mut scratch);
        }
        assert!(
            scratch.shield.active,
            "the fixture never raised a guard on the ground"
        );
        // OFF THE GROUND WITH THE BUTTON STILL DOWN — the transition itself, and
        // not a jump: a jump out of shield spends the guard through its own
        // road, which would measure something else entirely.
        scratch.kinematics.pos -= crate::Vec2::new(0.0, 240.0);
        scratch.ground.on_ground = false;
        step(&mut scratch);
        assert!(
            !scratch.ground.on_ground,
            "the fixture never actually left the ground"
        );
        scratch
    };

    let platform_fighter = airborne_with_guard_held(false);
    assert!(
        !platform_fighter.shield.active,
        "a ground guard rode into the air, where the same press is the air dodge"
    );

    let bubble = airborne_with_guard_held(true);
    assert!(
        bubble.shield.active,
        "poison: a deployable bubble must KEEP its airborne guard, or this test \
         passes for a fix that simply drops every shield on takeoff"
    );
}

/// ⭐⭐ A ROLL COMES TO REST, AND OWES A BEAT ON THE FAR SIDE.
///
/// ⛔⛔ THE DEFECT THIS PINS, Jon 2026-08-24: *"shield rolls have too much
/// motion to them. They send the character flying across the stage. They should
/// not be giving that much velocity, and they probably should stop at the end of
/// the roll and leave the character punishable for a frame or two."*
///
/// ⭐ AND THE DISTANCE WAS NEVER THE DEFECT. 530px/s across a 0.22s window is
/// ~117px — a step and a half. What made a roll read as crossing the stage is
/// that NOTHING took the velocity back when the window closed: the i-frames
/// lapsed and the body kept travelling at roll speed until friction or a wall
/// caught it. So this asserts the STOP and the endlag, and deliberately asserts
/// nothing about `dodge_roll_speed` — two nerfs for one defect is how a mechanic
/// gets tuned into uselessness.
#[test]
fn a_roll_stops_when_its_window_closes_and_owes_recovery() {
    let world = test_world();
    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.dodge.cooldown = 0.0;

    let mut tuning = TEST_TUNING;
    tuning.dodge_roll_endlag = 0.08;

    let burst = InputState {
        movement: crate::ActionEdges::EMPTY.with(
            crate::MovementAction::Burst,
            crate::Edge {
                pressed: true,
                held: false,
                released: false,
            },
        ),
        ..Default::default()
    };
    update_player_with_tuning_scratch(&world, &mut scratch, burst, 1.0 / 60.0, tuning);
    assert!(
        scratch.kinematics.vel.x.abs() > 100.0,
        "the roll never launched, so there is no stop to observe"
    );

    // Step the window out. The roll's own timer decides when, so this reads it
    // rather than counting frames against the constant.
    let mut steps = 0;
    while scratch.axis().dodge_roll_timer > 0.0 && steps < 120 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
        steps += 1;
    }
    assert!(steps < 120, "the roll window never closed");

    // ⭐⭐ AN ORDINARY ROLL IS ALREADY AT REST WHEN ITS WINDOW CLOSES, and
    // FRICTION is what does it — not the maneuver. Measured: ground friction
    // takes the 530 px/s roll to zero in about four frames, a fifth of the
    // 0.22s window, over roughly 14px of travel.
    //
    // ⛔ THAT IS WHY THERE IS NO VELOCITY CANCEL HERE. One was written, and it
    // reached into the body's whole `vel` — so a fighter struck mid-roll had its
    // knockback erased when the window closed. This assertion is the honest
    // remainder: the roll ends stationary, by the physics that were always going
    // to stop it.
    assert_eq!(
        scratch.kinematics.vel.x, 0.0,
        "a roll that has run its window is still travelling"
    );

    // ⭐ AND THE PUNISH WINDOW IS OPEN: committed, and no longer evading.
    assert!(
        scratch.axis().dodge_roll_endlag_timer > 0.0,
        "the roll handed off to nothing, so it ends instantly actionable"
    );
    let facts = crate::movement::BodyMotionFacts::from_model(&scratch.model);
    assert!(
        facts.dodge_roll_endlag,
        "the endlag is not published, so no defender or animation can read it"
    );
    assert!(
        !facts.evading(),
        "the recovery still counts as evading, so the punish window is \
         invulnerable and punishes nobody"
    );
}

/// ⭐⭐ A SPAMMED ROLL GETS WORSE, AND A FIGHTER WHO STOPS GETS IT BACK.
///
/// The parity inventory's *"Dodge staling"* row: a small per-body evade history
/// shortens the invulnerable window. It is the genre's answer to rolling being
/// the answer to everything — and it needs no authored content at all, because
/// every fighter already rolls.
///
/// ⛔ FOUR CLAUSES, and three of them are the ways this goes quietly wrong: a
/// game that never asked for staling getting it anyway, a stale evade decaying
/// to nothing (unreadable), the count never coming down (a trap rather than a
/// mechanic), and the count wrapping so that spamming RESETS it.
#[test]
fn a_repeated_evade_is_worn_down_and_recovers_when_it_stops() {
    let world = test_world();
    let mut tuning = TEST_TUNING;
    tuning.base.dodge_stale_step = 0.25;
    tuning.base.dodge_stale_floor = 0.4;
    tuning.base.dodge_stale_recovery = 0.5;

    let burst = InputState {
        movement: crate::ActionEdges::EMPTY.with(
            crate::MovementAction::Burst,
            crate::Edge {
                pressed: true,
                held: false,
                released: false,
            },
        ),
        ..Default::default()
    };
    // One evade, from a body that has not been evading: the authored window.
    let roll_window = |tuning: crate::test_support::TestTuning, evades_recent: u8| -> f32 {
        let mut scratch = scratch_at(world.spawn);
        scratch.ground.on_ground = true;
        scratch.dodge.cooldown = 0.0;
        scratch.dodge.evades_recent = evades_recent;
        update_player_with_tuning_scratch(&world, &mut scratch, burst, 1.0 / 60.0, tuning);
        // ⭐ THE INVULNERABLE WINDOW, which is what this test has always SAID it
        // measures. It read `dodge_roll_timer` until 2026-08-25, when that field
        // became the authored MANEUVER clock — travel, endlag and commitment —
        // and staling stopped touching it. Reading the maneuver clock here is
        // what let staling silently shorten the roll itself.
        scratch.axis().evade_invuln_timer
    };

    let fresh = roll_window(tuning, 0);
    assert!(fresh > 0.0, "the fixture never rolled");

    // ⭐ EACH RECENT EVADE COSTS SOME OF THE WINDOW.
    let once = roll_window(tuning, 1);
    let twice = roll_window(tuning, 2);
    assert!(
        once < fresh && twice < once,
        "a repeated roll kept its whole invulnerable window: {fresh} -> {once} -> {twice}"
    );

    // ⛔ AND IT FLOORS. An evade with no i-frames is not a worse option, it is a
    // broken one, and a player cannot read the difference.
    //
    // ⚠ THE `dt` IS ADDED BACK BEFORE COMPARING, and leaving it out is what made
    // this assertion fail against correct code: the window is read one tick
    // AFTER it is set, so every reading here has already lost a frame, and a
    // ratio taken between two such readings is not the ratio between the two
    // windows. 0.4 x (0.2033 + dt) - dt is the floor; 0.4 x 0.2033 is not.
    const DT: f32 = 1.0 / 60.0;
    let worn_out = roll_window(tuning, 40);
    let floor = (fresh + DT) * 0.4 - DT;
    assert!(
        worn_out >= floor - 0.0001,
        "staling took the window below its floor: {worn_out} against a floor of \
         {floor}"
    );

    // ⛔ A GAME THAT DECLARED NO STALING IS UNTOUCHED, however much it evades.
    assert_eq!(
        roll_window(TEST_TUNING, 40),
        roll_window(TEST_TUNING, 0),
        "a body in a game with no dodge staling was staled anyway"
    );
}

/// ⛔⛔ THE COUNT COMES DOWN ONE AT A TIME, and it must come down at all.
///
/// A step with no recovery is a trap rather than a mechanic: a fighter who
/// rolled four times early would carry it for the rest of the stock. And
/// forgiving the WHOLE count at once would make recovering cost the same
/// whether the option had been abused once or ten times.
#[test]
fn evade_staling_bleeds_off_one_evade_at_a_time() {
    let world = test_world();
    let mut tuning = TEST_TUNING;
    tuning.base.dodge_stale_step = 0.25;
    tuning.base.dodge_stale_recovery = 0.5;

    let mut scratch = scratch_at(world.spawn);
    scratch.ground.on_ground = true;
    scratch.dodge.evades_recent = 3;
    scratch.dodge.stale_decay = tuning.base.dodge_stale_recovery;

    // Half a second of standing still forgives exactly one.
    for _ in 0..30 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    assert_eq!(
        scratch.dodge.evades_recent, 2,
        "half a second forgave the wrong number of evades"
    );

    // …and the rest come off in their own time, down to fresh and no further.
    for _ in 0..200 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    assert_eq!(
        scratch.dodge.evades_recent, 0,
        "the count never reached fresh, so a fighter who stopped rolling never \
         gets the option back"
    );
}

/// ⭐⭐ A LAUNCH HARD ENOUGH TO KILL CANNOT BE TECHED OUT OF.
///
/// The parity inventory's *"Untechable high-launch threshold"*: tech eligibility
/// depends on the resolved launch through one rules knob. The genre's reason is
/// that a hit which should end a stock must not be survivable by a well-timed
/// press against the wall behind you.
///
/// ⛔ BOTH ARMS, because a threshold that refused EVERY tech would satisfy the
/// headline and delete the mechanic: the same fixture, under the same launch,
/// techs when the game declares no threshold.
#[test]
fn a_launch_above_the_untechable_threshold_refuses_the_wall_tech() {
    let world = test_world();

    let teched_under = |untechable_launch_speed: f32| -> bool {
        let mut tuning = floor_game_tuning();
        tuning.untechable_launch_speed = untechable_launch_speed;
        let mut scratch = scratch_at(world.spawn);
        scratch.kinematics.pos = Vec2::new(world.size.x - 120.0, world.size.y - 500.0);
        scratch.ground.on_ground = false;
        // Well over the 420 tumble threshold, so this is a real launch either way.
        scratch.flight.pending_launch = Vec2::new(1400.0, -260.0);
        crate::test_support::update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
        assert!(
            scratch.axis().tumble_timer > 0.0,
            "the launch did not tumble the body, so this measures nothing"
        );

        let dash = InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Burst,
                crate::Edge {
                    pressed: true,
                    held: false,
                    released: false,
                },
            ),
            ..Default::default()
        };
        for _ in 0..90 {
            let on_wall = scratch.wall.on_wall;
            let events = crate::test_support::update_player_with_tuning_scratch(
                &world,
                &mut scratch,
                if on_wall { dash } else { InputState::default() },
                1.0 / 60.0,
                tuning,
            );
            if events.operations.contains(&MovementOp::Tech) {
                return true;
            }
        }
        false
    };

    // ⛔ THE CONTROL FIRST: without a threshold this exact launch DOES tech, so
    // the refusal below is the rule and not the fixture failing to reach a wall.
    assert!(
        teched_under(0.0),
        "the fixture never teched even with no threshold declared, so it cannot \
         show one refusing"
    );
    // The launch is ~1424px/s; a threshold under it makes this hit untechable.
    assert!(
        !teched_under(900.0),
        "a launch above the declared untechable speed was teched anyway"
    );
    // …and a threshold ABOVE the launch leaves it alone.
    assert!(
        teched_under(4000.0),
        "a launch well under the threshold was refused a tech it had earned"
    );
}

/// ⭐⭐ AN EVADE IS A COMMITMENT UNTIL ITS TAIL.
///
/// The parity inventory's *"Spot-dodge attack cancel near tail"*. Today an
/// attack cancels a spot dodge on frame one, which makes the dodge strictly
/// better than the genre's: invulnerable AND instantly actionable. The last N
/// seconds stay cancellable and the rest is committed.
///
/// ⛔ THE DEFAULT DISABLES THE RULE. `0.0` is not "nothing is cancellable" — it
/// is "the rule is off", so a game that declares no tail refuses nothing it
/// previously allowed. Both are asserted, because a version that committed
/// every body would satisfy the headline and silently re-tune every other game
/// in the repo.
#[test]
fn an_evade_is_committed_until_its_tail_and_only_when_declared() {
    let world = test_world();

    let committed_during = |tail: f32| -> Vec<bool> {
        let mut tuning = floor_game_tuning();
        tuning.evade_cancel_tail = tail;
        let mut scratch = scratch_at(world.spawn);
        scratch.ground.on_ground = true;
        scratch.dodge.cooldown = 0.0;
        let burst = InputState {
            movement: crate::ActionEdges::EMPTY.with(
                crate::MovementAction::Burst,
                crate::Edge {
                    pressed: true,
                    held: false,
                    released: false,
                },
            ),
            ..Default::default()
        };
        crate::test_support::update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            burst,
            1.0 / 60.0,
            tuning,
        );
        // Sample the published fact across the whole evade.
        let mut seen = Vec::new();
        for _ in 0..20 {
            let facts = crate::movement::BodyMotionFacts::from_model(&scratch.model);
            if facts.dodge_rolling {
                seen.push(facts.evade_committed);
            }
            crate::test_support::update_player_with_tuning_scratch(
                &world,
                &mut scratch,
                InputState::default(),
                1.0 / 60.0,
                tuning,
            );
        }
        seen
    };

    // ⛔ THE BASELINE FIRST: with the rule off, nothing is ever committed.
    let undeclared = committed_during(0.0);
    assert!(
        !undeclared.is_empty(),
        "the fixture never rolled, so neither reading means anything"
    );
    assert!(
        undeclared.iter().all(|c| !c),
        "a game that declared no cancel tail committed its bodies anyway, which \
         refuses attacks every other game in this repo currently allows"
    );

    // ⭐ AND WITH A TAIL: committed early, actionable at the end.
    let declared = committed_during(0.06);
    assert!(
        declared.first() == Some(&true),
        "the evade was actionable on its first frame, so it is not a commitment"
    );
    assert!(
        declared.last() == Some(&false),
        "the evade never became actionable, so the tail is not a cancel window \
         — it is a longer lockout"
    );
}

/// A ROLL ENDS BY STOPPING, AND A LAUNCHED BODY KEEPS ITS LAUNCH.
///
/// Jon, 2026-08-25: *"shield rolls have too much motion to them. They send the
/// character flying across the stage... they probably should stop at the end of
/// the roll."* The roll set a velocity that nothing ever took back, so the body
/// kept travelling at the full roll speed through the whole cooldown and then
/// rolled again — a continuous slide, not a series of rolls.
///
/// ⛔⛔ THE THIRD ARM IS THE POINT. Ending a roll by zeroing `vel` was tried
/// before and erased the knockback of a body struck mid-roll. The shed is
/// allowed only while the body is still doing no more than the roll itself did.
#[test]
fn a_ground_roll_ends_stopped_but_never_eats_a_launch() {
    let world = test_world();
    let mut tuning = TEST_TUNING;
    tuning.base.shield = crate::ShieldTuning::PLATFORM_FIGHTER;
    tuning.base.spot_dodge_time = 0.16;

    let settle = || {
        let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
        scratch.ground.on_ground = false;
        for _ in 0..60 {
            update_player_with_tuning_scratch(
                &world,
                &mut scratch,
                InputState::default(),
                1.0 / 60.0,
                tuning,
            );
        }
        assert!(
            scratch.ground.on_ground,
            "the fixture never reached the floor"
        );
        scratch
    };
    let mut guard_right = InputState::with_axes(1.0, 0.0);
    guard_right.shield_held = true;
    let side = tuning.frame().side();

    // ARM 1 — BASELINE: the roll really is driving the body, so arm 2 is
    // measuring a stop and not a roll that never started.
    let mut rolling = settle();
    update_player_with_tuning_scratch(&world, &mut rolling, guard_right, 1.0 / 60.0, tuning);
    assert!(
        rolling.axis().dodge_roll_timer > 0.0,
        "guard + direction did not start a roll"
    );
    assert!(
        rolling.kinematics.vel.dot(side).abs() > 100.0,
        "the roll is not moving the body at all: {}",
        rolling.kinematics.vel.dot(side)
    );

    // ARM 2 — THE STOP. Run the roll out and the body is left standing.
    while rolling.axis().dodge_roll_timer > 0.0 {
        update_player_with_tuning_scratch(&world, &mut rolling, guard_right, 1.0 / 60.0, tuning);
    }
    assert!(
        rolling.kinematics.vel.dot(side).abs() < 1.0,
        "the roll ended still carrying its own push: {}",
        rolling.kinematics.vel.dot(side)
    );

    // ARM 3 — THE LAUNCH SURVIVES. A body struck mid-roll is moving faster than
    // its roll ever pushed it, and the end of the roll must not touch that.
    let mut launched = settle();
    update_player_with_tuning_scratch(&world, &mut launched, guard_right, 1.0 / 60.0, tuning);
    assert!(launched.axis().dodge_roll_timer > 0.0, "arm 3 never rolled");
    let launch = side * 3000.0;
    launched.kinematics.vel = launch;
    while launched.axis().dodge_roll_timer > 0.0 {
        update_player_with_tuning_scratch(&world, &mut launched, guard_right, 1.0 / 60.0, tuning);
    }
    assert!(
        launched.kinematics.vel.dot(side) > 2000.0,
        "the end of the roll ate a launch it did not create: {}",
        launched.kinematics.vel.dot(side)
    );

    // ARM 4 — A SMALL LAUNCH THE ROLL'S OWN WAY, and this is the arm that
    // matters. A first version of the shed allowed anything "same direction and
    // no faster than the roll", which this passes — so a body nudged 300px/s
    // along its own roll had that nudge deleted. Arm 3 could not see it,
    // because a 3000px/s launch is faster than the roll and was refused for the
    // wrong reason. `the_stage_kills` caught it: with launches being eaten,
    // NOBODY in a whole match was ever knocked out of the room.
    let mut nudged = settle();
    update_player_with_tuning_scratch(&world, &mut nudged, guard_right, 1.0 / 60.0, tuning);
    assert!(nudged.axis().dodge_roll_timer > 0.0, "arm 4 never rolled");
    let gentle = 300.0;
    assert!(
        gentle < tuning.params().abilities.dodge_roll_speed,
        "arm 4 must use a launch SLOWER than the roll or it is a copy of arm 3"
    );
    nudged.kinematics.vel = side * gentle;
    while nudged.axis().dodge_roll_timer > 0.0 {
        update_player_with_tuning_scratch(&world, &mut nudged, guard_right, 1.0 / 60.0, tuning);
    }
    assert!(
        nudged.kinematics.vel.dot(side) > 200.0,
        "the end of the roll ate a SLOW launch pointing its own way: {}",
        nudged.kinematics.vel.dot(side)
    );
}

/// A WEAK HIT PINS A DOWNED BODY; A STRONG ONE STILL LAUNCHES IT; AND THE PIN
/// RUNS OUT.
///
/// Five arms, because a jab lock is defined by its three refusals as much as by
/// what it does. The dangerous wrong version is not "it never fires" — it is
/// "it always fires", which is an infinite that no test asserting only the
/// happy path would notice.
#[test]
fn a_weak_hit_pins_a_downed_body_but_the_lock_is_bounded() {
    use crate::movement::knockdown::jab_lock;

    const WEAK: f32 = 200.0;
    const STRONG: f32 = 900.0;

    let armed = || {
        let mut tuning = TEST_TUNING;
        tuning.base.jab_lock_speed = 320.0;
        tuning.base.jab_lock_limit = 3;
        tuning
    };
    // A body already on the floor.
    let prone = || {
        let mut state = crate::movement::AxisManeuverState::default();
        state.knockdown_timer = crate::movement::knockdown::KNOCKDOWN_TIME;
        state
    };

    // ARM 1 — THE PIN. A weak hit on a downed body does not launch it.
    let mut state = prone();
    assert!(
        jab_lock(&mut state, armed().params(), WEAK),
        "a weak hit on a downed body did not pin it"
    );
    assert!(
        state.knockdown_timer > 0.0,
        "the pin left the body no longer prone, so it pinned nothing"
    );

    // ARM 2 — A STRONG HIT STILL LAUNCHES. You cannot pin somebody with a
    // smash; without this the rule would swallow every follow-up.
    let mut state = prone();
    assert!(
        !jab_lock(&mut state, armed().params(), STRONG),
        "a hit far above the declared speed was allowed to pin"
    );

    // ARM 3 — A BODY THAT IS NOT DOWN IS NOT PINNABLE. There is nothing to lock
    // about a body standing up.
    let mut standing = crate::movement::AxisManeuverState::default();
    assert!(
        !jab_lock(&mut standing, armed().params(), WEAK),
        "a body that was not prone at all was pinned"
    );

    // ARM 4 — THE BOUND, and this is the arm that matters. Three pins, then the
    // fourth weak hit launches whatever it is worth.
    let mut state = prone();
    for i in 0..3 {
        assert!(
            jab_lock(&mut state, armed().params(), WEAK),
            "pin {i} was refused before the declared limit of 3"
        );
    }
    assert!(
        !jab_lock(&mut state, armed().params(), WEAK),
        "the jab lock had no bound — a fourth pin means an infinite"
    );

    // ARM 5 — A BODY THAT DECLARES NOTHING IS UNTOUCHED, which is every body
    // outside a match. Asserted against a body that IS prone and a hit that IS
    // weak, so it cannot pass for want of the situation.
    let mut state = prone();
    assert!(
        !jab_lock(&mut state, TEST_TUNING.params(), WEAK),
        "a body declaring no jab lock pinned anyway"
    );

    // ARM 6 — THE WIRING, not the rule. Everything above proves the FUNCTION
    // answers correctly and nothing about whether the launch gateway asks it.
    // Drive a real weak launch at a real prone body through the kernel and the
    // body must not move.
    let world = test_world();
    let mut tuning = floor_game_tuning();
    tuning.jab_lock_speed = 320.0;
    tuning.jab_lock_limit = 3;
    let mut scratch = scratch_at(world.spawn);
    for _ in 0..40 {
        crate::test_support::update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    scratch.axis_mut().knockdown_timer = crate::movement::knockdown::KNOCKDOWN_TIME;
    let resting = scratch.kinematics.pos;
    scratch.flight.pending_launch = Vec2::new(WEAK, 0.0);
    crate::test_support::update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState::default(),
        1.0 / 60.0,
        tuning,
    );
    // ⛔ NOT MEASURED BY POSITION. A prone body's velocity is zeroed by the
    // knockdown itself, so it stays put whether or not the pin fired —
    // removing the gateway call left a position assertion perfectly green.
    // `jab_locks` is written by ONE function and reachable only through the
    // gateway, so it answers the wiring question and nothing else.
    let _ = resting;
    assert_eq!(
        scratch.axis().jab_locks,
        1,
        "the launch gateway never asked about the pin"
    );
    assert!(
        scratch.axis().tumble_timer <= 0.0,
        "the pinned body was tumbled as well as pinned"
    );

    // ARM 7 — AND THE GATEWAY DOES NOT SWALLOW EVERYTHING. The same body, a
    // launch far above the threshold, still leaves the floor.
    let mut launched = scratch_at(world.spawn);
    for _ in 0..40 {
        crate::test_support::update_player_with_tuning_scratch(
            &world,
            &mut launched,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    launched.axis_mut().knockdown_timer = crate::movement::knockdown::KNOCKDOWN_TIME;
    launched.flight.pending_launch = Vec2::new(1400.0, -260.0);
    crate::test_support::update_player_with_tuning_scratch(
        &world,
        &mut launched,
        InputState::default(),
        1.0 / 60.0,
        tuning,
    );
    assert_eq!(
        launched.axis().jab_locks,
        0,
        "a launch far above the threshold was pinned instead of thrown"
    );
}

/// A TECH PRESS INTO AN UNTECHABLE LAUNCH SPENDS THE LOCKOUT.
///
/// ⛔⛔ THE COMMENT SAID IT DID AND THE CODE DID NOT. The refusal gate reads
/// *"AND IT STILL SPENDS THE LOCKOUT BELOW: a player who mashes into an
/// untechable launch should not be free to keep mashing"* — but the lockout is
/// only charged where `tech_press_timer` EXPIRES, and an untechable press never
/// armed that timer. So mashing into a launch too hard to tech cost nothing at
/// all, and every press stayed live.
///
/// ⭐ THE ARM THAT MATTERS IS THE SECOND PRESS: charging the lockout is only
/// meaningful if it then refuses something.
#[test]
fn a_tech_press_into_an_untechable_launch_still_spends_the_lockout() {
    let world = test_world();
    let mut tuning = floor_game_tuning();
    // A threshold this launch clears, so the tumble is genuinely untechable.
    tuning.untechable_launch_speed = 500.0;

    let mut scratch = scratch_at(world.spawn);
    scratch.kinematics.pos = Vec2::new(world.size.x - 120.0, world.size.y - 500.0);
    scratch.ground.on_ground = false;
    scratch.flight.pending_launch = Vec2::new(1400.0, -260.0);
    crate::test_support::update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState::default(),
        1.0 / 60.0,
        tuning,
    );
    assert!(
        scratch.axis().tumble_untechable,
        "the fixture's launch is techable, so nothing below is about a refusal"
    );

    let burst = InputState {
        movement: crate::ActionEdges::EMPTY.with(
            crate::MovementAction::Burst,
            crate::Edge {
                pressed: true,
                held: false,
                released: false,
            },
        ),
        ..Default::default()
    };
    crate::test_support::update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        burst,
        1.0 / 60.0,
        tuning,
    );

    assert_eq!(
        scratch.axis().tech_press_timer,
        0.0,
        "an untechable launch armed a tech window, so the refusal is not refusing"
    );
    assert!(
        scratch.axis().tech_lockout_timer > 0.0,
        "mashing into an untechable launch cost NOTHING — the press was refused \
         for free, so a player can keep pressing every frame"
    );
}
