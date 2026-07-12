//! **S0 — the standard host-input path, proven end to end.**
//!
//! Synthetic keyboard input → the normal leafwing action state → the standard
//! device→`ControlFrame` bridge → the fixed-tick latch → `SlotControls` → the
//! canonical player's brain → observable movement. This is the SAME production
//! wiring the full app runs (`PlatformerHostPlugins` under the `input` feature),
//! assembled by the demo's own `build_demo_app`; nothing here inserts a
//! `ControlFrame` directly or adds a Sanic-specific input adapter.
//!
//! The test asserts observable movement (the load-bearing claim) plus the
//! non-neutral state at each intermediate seam, so removing any one edge —
//! the bridge, the latch, the slot population, or the brain — breaks it.
#![cfg(feature = "input")]

use bevy::input::ButtonInput;
use bevy::prelude::*;

use ambition::characters::brain::{PlayerSlot, SlotControls};
use ambition::engine_core::ControlFrame;

fn player_pos_x(app: &mut App) -> f32 {
    let mut q = app.world_mut().query_filtered::<
        &ambition::actors::actor::BodyKinematics,
        With<ambition::actors::actor::PrimaryPlayer>,
    >();
    q.iter(app.world())
        .next()
        .expect("the demo spawned a primary player body")
        .pos
        .x
}

fn press_key(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
}

/// Test-owned keyboard levels applied inside Bevy's canonical input stage.
///
/// Mutating `ButtonInput` immediately before `App::update()` is adequate for a
/// held-level smoke test, but it is not a faithful way to synthesize a release:
/// Bevy's keyboard input system clears frame edges before Leafwing collects the
/// physical inputs. Installing this driver in `InputSystems`, after the ordinary
/// keyboard event consumer, makes press and release transitions visible at the
/// same seam as real window input without bypassing Leafwing, the ControlFrame
/// bridge, the fixed-tick latch, slots, or the player brain.
#[derive(Resource, Default)]
struct SyntheticBallDashKeyboard {
    down: bool,
    rev: bool,
}

fn drive_synthetic_ball_dash_keyboard(
    desired: Res<SyntheticBallDashKeyboard>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
) {
    if desired.down {
        keys.press(KeyCode::ArrowDown);
    } else {
        keys.release(KeyCode::ArrowDown);
    }
    if desired.rev {
        keys.press(KeyCode::KeyX);
    } else {
        keys.release(KeyCode::KeyX);
    }
}

fn install_synthetic_ball_dash_keyboard(app: &mut App) {
    app.init_resource::<SyntheticBallDashKeyboard>();
    app.add_systems(
        PreUpdate,
        drive_synthetic_ball_dash_keyboard
            .in_set(bevy::input::InputSystems)
            .after(bevy::input::keyboard::keyboard_input_system),
    );
}

/// Hold the default preset's "move right" key (ArrowRight) and watch the player
/// body travel right through the whole standard path.
#[test]
fn synthetic_keyboard_moves_the_player_through_the_standard_path() {
    let mut app = ambition_demo_sanic_app::build_demo_app();

    app.update(); // Startup: spawn the body, attach the leafwing ActionState.
                  // Let the body settle onto the floor with NO input so the x baseline is clean.
    for _ in 0..30 {
        app.update();
    }
    let x_before = player_pos_x(&mut app);

    // Synthetic device input: hold ArrowRight (KeyboardPreset index 0 → MoveRight).
    press_key(&mut app, KeyCode::ArrowRight);
    for _ in 0..90 {
        app.update();
    }

    // 2. The standard bridge produced a NON-neutral ControlFrame.
    let frame = *app.world().resource::<ControlFrame>();
    assert!(
        frame.axis_x > 0.0 || frame.right_pressed,
        "the host bridge must emit a rightward ControlFrame from held ArrowRight; got {frame:?}"
    );

    // 3./4. The fixed-tick latch delivered that frame to the player's slot.
    let slot = app
        .world()
        .resource::<SlotControls>()
        .get(PlayerSlot::PRIMARY);
    assert!(
        slot.axis_x > 0.0 || slot.right_pressed,
        "the latched frame must reach SlotControls[PRIMARY]; got {slot:?}"
    );

    // 5. Observable simulation effect: the canonical player moved right.
    let x_after = player_pos_x(&mut app);
    assert!(
        x_after > x_before + 5.0,
        "held ArrowRight must move the player body right: x {x_before:.1} → {x_after:.1}"
    );
}

#[test]
fn d_toggles_sanic_to_super_sanic_and_back_through_the_standard_path() {
    use ambition::characters::actor::WornCharacter;
    use ambition_demo_sanic::{SANIC_CHARACTER_ID, SUPER_SANIC_CHARACTER_ID};

    fn worn_id(app: &mut App) -> String {
        let mut q = app
            .world_mut()
            .query_filtered::<&WornCharacter, With<ambition::actors::actor::PrimaryPlayer>>();
        q.iter(app.world())
            .next()
            .expect("the demo spawned a primary worn character")
            .id()
            .to_string()
    }

    fn pulse_d_until(app: &mut App, target: &str) {
        for frame in 0..120 {
            {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                if frame % 4 == 0 {
                    keys.press(KeyCode::KeyD);
                } else if frame % 4 == 1 {
                    keys.release(KeyCode::KeyD);
                }
            }
            app.update();
            if worn_id(app) == target {
                return;
            }
        }
        panic!("semantic Utility/D never transformed the player to {target}");
    }

    let mut app = ambition_demo_sanic_app::build_demo_app();
    app.update();
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(worn_id(&mut app), SANIC_CHARACTER_ID);

    pulse_d_until(&mut app, SUPER_SANIC_CHARACTER_ID);
    assert_eq!(worn_id(&mut app), SUPER_SANIC_CHARACTER_ID);
    let flight = {
        let mut q = app.world_mut().query_filtered::<
            &ambition::actors::actor::BodyFlightState,
            With<ambition::actors::actor::PrimaryPlayer>,
        >();
        *q.iter(app.world())
            .next()
            .expect("the demo player carries flight state")
    };
    assert!(
        !flight.fly_enabled,
        "D belongs to the Sanic transformation and must not leak into the generic fly toggle"
    );

    pulse_d_until(&mut app, SANIC_CHARACTER_ID);
    assert_eq!(worn_id(&mut app), SANIC_CHARACTER_ID);
}

/// Non-vacuity: with NO synthetic input the body does not drift right. Proves the
/// movement above is driven by the injected input, not by gravity/momentum alone.
#[test]
fn without_input_the_player_does_not_drift_right() {
    let mut app = ambition_demo_sanic_app::build_demo_app();
    app.update();
    for _ in 0..30 {
        app.update();
    }
    let x_before = player_pos_x(&mut app);
    for _ in 0..90 {
        app.update();
    }
    let x_after = player_pos_x(&mut app);
    assert!(
        (x_after - x_before).abs() < 5.0,
        "with no input the body must not travel horizontally: x {x_before:.1} → {x_after:.1}"
    );
    // And the standard bridge holds a neutral frame with no device input.
    let frame = *app.world().resource::<ControlFrame>();
    assert!(
        frame.axis_x == 0.0 && !frame.right_pressed,
        "no device input → neutral ControlFrame; got {frame:?}"
    );
}

/// The assembled Sanic customer receives the raw host inputs, but its authored
/// peaceful persona removes the protagonist-only combat verbs before effects.
/// This is stronger than inspecting ActionSet: it drives the production keyboard
/// bridge and observes both the sanitized ActorControl and inert body state.
///
/// The demo steps a FIXED tick (it does not fire every frame), and `attack` /
/// `projectile` are rising edges (`just_pressed`), so a single read would race
/// the latch. We hold the shield level and re-arm the attack/projectile edges
/// across a window, ACCUMULATING what the slot saw and what the body ever did.
/// That is robust to tick timing and a stronger claim than a one-frame snapshot:
/// the standard path must deliver every host combat input, and the peaceful kit
/// must suppress every one of them on every frame.
#[test]
fn peaceful_sanic_filters_host_combat_inputs_before_effects() {
    use ambition::characters::brain::ActorControl;

    let mut app = ambition_demo_sanic_app::build_demo_app();
    app.update();
    for _ in 0..10 {
        app.update();
    }

    // Default Arrows+Z/X/C preset: X = attack, E = quick-action/shield,
    // V = host chargeable projectile.
    let mut slot_saw_attack = false;
    let mut slot_saw_shield = false;
    let mut slot_saw_projectile = false;
    let mut control_ever_meleed = false;
    let mut control_ever_pogoed = false;
    let mut control_ever_fired = false;
    let mut control_ever_shielded = false;
    let mut control_ever_projectiled = false;
    let mut shield_ever_active = false;
    let mut melee_ever_swinging = false;
    let mut projectiles_ever_live = false;

    for i in 0..120 {
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            // Toggle X/V so `just_pressed` re-arms every other frame — at least one
            // rising edge then coincides with a fixed tick. Hold E as a level.
            if i % 2 == 0 {
                keys.press(KeyCode::KeyX);
                keys.press(KeyCode::KeyV);
            } else {
                keys.release(KeyCode::KeyX);
                keys.release(KeyCode::KeyV);
            }
            keys.press(KeyCode::KeyE);
        }
        app.update();

        let slot = app
            .world()
            .resource::<SlotControls>()
            .get(PlayerSlot::PRIMARY);
        slot_saw_attack |= slot.attack_pressed;
        slot_saw_shield |= slot.shield_held;
        slot_saw_projectile |= slot.projectile_pressed || slot.projectile_held;

        let (control, shield_active, melee_swinging) = {
            let mut q = app.world_mut().query_filtered::<(
                &ActorControl,
                &ambition::actors::actor::BodyShieldState,
                &ambition::actors::actor::BodyMelee,
            ), With<ambition::actors::actor::PrimaryPlayer>>(
            );
            let (control, shield, melee) = q
                .iter(app.world())
                .next()
                .expect("the demo spawned its primary body");
            (control.0, shield.active, melee.is_swinging())
        };
        control_ever_meleed |= control.melee_pressed;
        control_ever_pogoed |= control.pogo_pressed;
        control_ever_fired |= control.fire.is_some();
        control_ever_shielded |= control.shield_held;
        control_ever_projectiled |=
            control.projectile_pressed || control.projectile_held || control.projectile_released;
        shield_ever_active |= shield_active;
        melee_ever_swinging |= melee_swinging;

        let live = {
            let mut q = app
                .world_mut()
                .query::<&ambition::projectiles::LiveProjectile>();
            q.iter(app.world()).count()
        };
        projectiles_ever_live |= live > 0;
    }

    // Non-vacuity: the standard path DID deliver the host combat inputs to the slot.
    assert!(slot_saw_attack, "the standard input path saw X (attack)");
    assert!(
        slot_saw_shield,
        "the standard input path saw held E (quick-action / shield)"
    );
    assert!(
        slot_saw_projectile,
        "the standard input path saw V (projectile)"
    );

    // Suppression: the peaceful authored persona filtered every combat verb before
    // any body/effects system, on every frame of the window.
    assert!(!control_ever_meleed, "peaceful kit filters raw attack");
    assert!(
        !control_ever_pogoed,
        "peaceful kit filters the melee variant"
    );
    assert!(!control_ever_fired, "peaceful kit filters flat ranged fire");
    assert!(
        !control_ever_shielded,
        "peaceful kit filters the body shield"
    );
    assert!(
        !control_ever_projectiled,
        "peaceful kit filters the host charge projectile"
    );
    assert!(!shield_ever_active, "the body shield never activates");
    assert!(
        !melee_ever_swinging,
        "the body never starts a melee lifecycle"
    );
    assert!(
        !projectiles_ever_live,
        "the host projectile input cannot create a projectile for Sanic"
    );
}

/// The visible control contract is real, not just documented: local Down plus
/// the ordinary X/attack edge is captured before the peaceful-kit gate, builds
/// charge, and releasing Down launches the momentum body without reopening
/// generic melee.
#[test]
fn down_plus_x_revs_and_releasing_down_launches_the_ball_dash() {
    use ambition::actors::features::MotionModel;
    use ambition::engine_core::BodyMode;
    use ambition::sprite_sheet::character::CharacterAnim;
    use ambition_demo_sanic::ball_dash::{BallDash, BallDashTuning, Rolling};

    let mut app = ambition_demo_sanic_app::build_demo_app();
    install_synthetic_ball_dash_keyboard(&mut app);
    app.update();
    for _ in 0..30 {
        app.update();
    }

    // Re-arm X until one edge crosses a fixed tick. While Down is held the
    // body must enter its compact mode and the shared animation picker must
    // request DashStartup (or a sheet fallback from that row), rather than
    // freezing on idle. Stop as soon as the state proves a launchable charge;
    // an arbitrary post-charge delay would test charge decay, not release.
    let tuning = BallDashTuning::default();
    let mut max_charge = 0.0_f32;
    let mut slot_saw_attack = false;
    let mut saw_crouching = false;
    let mut saw_rev_pose = false;
    for frame in 0..120 {
        {
            let mut keyboard = app.world_mut().resource_mut::<SyntheticBallDashKeyboard>();
            keyboard.down = true;
            // One frame pressed, three released: each cycle produces exactly
            // one physical X edge inside the same input stage real keys use.
            keyboard.rev = frame % 4 == 0;
        }
        app.update();

        let slot = app
            .world()
            .resource::<SlotControls>()
            .get(PlayerSlot::PRIMARY);
        slot_saw_attack |= slot.attack_pressed;

        let (charge, mode, anim) = {
            let mut q = app.world_mut().query_filtered::<(
                &BallDash,
                &ambition::actors::actor::BodyModeState,
                &ambition::sim_view::BodyPoseView,
            ), With<ambition::actors::actor::PrimaryPlayer>>(
            );
            let (dash, mode, pose) = q
                .iter(app.world())
                .next()
                .expect("the demo spawned a primary player body with dash and pose state");
            (dash.charge, mode.body_mode, pose.anim)
        };
        max_charge = max_charge.max(charge);
        saw_crouching |= mode == BodyMode::Crouching;
        saw_rev_pose |= anim == CharacterAnim::DashStartup;
        if charge >= tuning.min_launch_charge && saw_crouching && saw_rev_pose {
            break;
        }
    }
    assert!(slot_saw_attack, "the standard input path must deliver X");
    assert!(
        max_charge >= tuning.min_launch_charge,
        "Down+X must build enough charge to launch; max charge was {max_charge}"
    );
    assert!(saw_crouching, "held Down must put Sanic into crouch mode");
    assert!(
        saw_rev_pose,
        "revving must request the shared dash-startup animation pose"
    );

    {
        let mut keyboard = app.world_mut().resource_mut::<SyntheticBallDashKeyboard>();
        keyboard.down = false;
        keyboard.rev = false;
    }
    let mut saw_slot_release = false;
    let mut saw_technique_release = false;
    let mut saw_rolling = false;
    let mut saw_charge_spent = false;
    let mut max_launch_speed = 0.0_f32;
    let mut final_dash = BallDash::default();
    for _ in 0..60 {
        app.update();
        let slot = app
            .world()
            .resource::<SlotControls>()
            .get(PlayerSlot::PRIMARY);
        saw_slot_release |= slot.axis_y.abs() < 0.01;

        let (rolling, speed, dash, technique_input) = {
            let mut q = app.world_mut().query_filtered::<(
                &BallDash,
                Option<&Rolling>,
                &MotionModel,
                &ambition::actors::actor::BodyKinematics,
                &ambition_demo_sanic::ball_dash::BallDashInput,
            ), With<ambition::actors::actor::PrimaryPlayer>>(
            );
            let (dash, rolling, motion, kin, technique_input) = q
                .iter(app.world())
                .next()
                .expect("the demo spawned a primary player body");
            let speed = match motion {
                MotionModel::SurfaceMomentum(momentum) => match momentum.state {
                    ambition::engine_core::SurfaceMotion::Riding { v_t, .. } => v_t.abs(),
                    ambition::engine_core::SurfaceMotion::Airborne => kin.vel.length(),
                },
                MotionModel::AxisSwept(_) | MotionModel::AdhesiveCrawler(_) => 0.0,
            };
            (rolling.is_some(), speed, *dash, *technique_input)
        };
        saw_technique_release |= !technique_input.crouch_held;
        saw_rolling |= rolling;
        saw_charge_spent |= dash.charge < tuning.min_launch_charge;
        max_launch_speed = max_launch_speed.max(speed);
        final_dash = dash;
        if saw_slot_release && saw_technique_release && saw_rolling && saw_charge_spent {
            break;
        }
    }
    // The focused `capture_ball_dash_input` unit test directly proves that the
    // PlayerInput seam emits a one-tick `crouch_released` edge. At this full-app
    // boundary that transient may already have been consumed by
    // `tick_ball_dash` before `App::update` returns, so the durable behavioral
    // oracle is that the launch spent the armed charge and entered Rolling.
    assert!(
        saw_slot_release,
        "the real keyboard path must publish a neutral Down level to the fixed-tick player slot"
    );
    assert!(
        saw_technique_release,
        "the Sanic technique seam must observe that Down is no longer held"
    );
    assert!(
        saw_charge_spent,
        "releasing Down must consume the armed spin-dash charge; final dash state was {final_dash:?}"
    );
    assert!(
        saw_rolling,
        "releasing Down after revving must launch and enter the rolling state; final dash state was {final_dash:?}"
    );
    assert!(
        max_launch_speed >= tuning.launch_speed * tuning.min_launch_charge,
        "the launch must produce at least the authored minimum speed; observed {max_launch_speed}"
    );
}
