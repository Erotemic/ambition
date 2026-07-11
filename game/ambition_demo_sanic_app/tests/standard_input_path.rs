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
