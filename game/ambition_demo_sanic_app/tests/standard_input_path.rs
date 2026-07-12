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
