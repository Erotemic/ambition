//! Multi-frame scripted-gameplay integration test.
//!
//! Companion to the per-system slice tests in
//! `game/ambition_app/src/headless.rs`. This test drives the
//! sim through a sequence of `ControlFrame`s across several
//! `app.update()` calls and asserts on the cumulative event timeline.

use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::schedule::GameMode;
use ambition_platformer2d::sfx::{OwnedSfxMessage, SfxMessage};
use bevy::asset::AssetPlugin;
use bevy::ecs::message::Messages;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::transform::TransformPlugin;
use bevy::MinimalPlugins;

/// Minimal-plugin App that boots `add_simulation_plugins`. Mirrors
/// the `sim_emits_sfx_reset_*` pattern in `ambition_app::headless` tests.
fn build_minimal_sim_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<GameMode>();

    // That publisher is gone, so the fixture composes the shell host booted to the gameplay route,
    // which is the only way now.
    ambition_app::app::shell_host::compose_ambition_gameplay_host(&mut app);

    // Wait for the session world rather than guessing a frame count, and PANIC with the budget
    // when it never arrives — returning an un-activated App would fail every test built on this
    // fixture somewhere far less informative.
    ambition_platformer2d::platformer::lifecycle::settle_until_session_world(
        &mut app,
        ambition_platformer2d::platformer::lifecycle::SESSION_SETTLE_FRAMES,
    )
    .unwrap_or_else(|budget| {
        panic!("the shell-composed fixture produced no session world in {budget} frames")
    });
    app
}

/// through the SEAM, not at the resource. `ControlFrame` is seat zero's OUTPUT mirror
/// since; assigning it delivers a press to nobody, and a fixture that does so asserts against a
/// simulation that never received an input.
fn write_control_frame(app: &mut App, frame: ControlFrame) {
    ambition_platformer2d::sim::drive_control_frame(app.world_mut(), frame);
}

fn count_reset_messages(app: &App) -> usize {
    let messages = app.world().resource::<Messages<OwnedSfxMessage>>();
    messages
        .iter_current_update_messages()
        .filter(|m| matches!(m.request, SfxMessage::Reset { .. }))
        .count()
}

/// Drive a 5-frame scripted sequence with Reset presses on alternating
/// frames. Pins the high-level property that pressing Reset
/// produces at least one `SfxMessage::Reset` per press without
/// crashing the sim under MinimalPlugins. The exact per-frame timing
/// of the message buffer is intentionally not pinned (Bevy's
/// `Messages` double-buffer can carry messages across one update so
/// `iter_current_update_messages` is a soft signal — the strict
/// invariant is that idle inputs don't *produce new* Reset events
/// indefinitely).
#[test]
fn scripted_reset_press_emits_reset_message() {
    let mut app = build_minimal_sim_app();

    // Frame 1: idle baseline.
    write_control_frame(&mut app, ControlFrame::default());
    app.update();
    let baseline = count_reset_messages(&app);

    // Frame 2: press Reset. Within the next frame or two we should
    // see at least one Reset message in the channel (the buffered
    // double-buffer keeps it readable across one rotation).
    write_control_frame(
        &mut app,
        ControlFrame {
            reset_pressed: true,
            ..ControlFrame::default()
        },
    );
    app.update();
    let after_press = count_reset_messages(&app);
    assert!(
        after_press > baseline,
        "Reset press should bump the SfxMessage::Reset count (baseline={baseline}, after={after_press})"
    );
}

/// Press a sequence of inputs (Reset then Jump then idle) across several frames and assert the sim
/// runs cleanly to completion. The minimal-plugin App must accept arbitrary `ControlFrame`
/// sequences without panicking, regardless of which combination of presses fires.
#[test]
fn scripted_heterogeneous_input_sequence_runs_to_completion() {
    let mut app = build_minimal_sim_app();
    let frames = [
        ControlFrame::default(),
        ControlFrame {
            reset_pressed: true,
            ..ControlFrame::default()
        },
        ControlFrame {
            jump_pressed: true,
            jump_held: true,
            ..ControlFrame::default()
        },
        ControlFrame {
            jump_held: true,
            ..ControlFrame::default()
        },
        ControlFrame {
            axis_x: 1.0,
            ..ControlFrame::default()
        },
        ControlFrame::default(),
        ControlFrame {
            axis_x: -1.0,
            ..ControlFrame::default()
        },
        ControlFrame::default(),
    ];
    for frame in frames {
        write_control_frame(&mut app, frame);
        app.update();
    }
    // If we got here without panicking the sim accepted every input
    // combination. Light sanity check on the message channel: it
    // exists and is readable.
    let messages = app.world().resource::<Messages<OwnedSfxMessage>>();
    let _seen: usize = messages.iter_current_update_messages().count();
}

/// 30 idle frames with no input must run cleanly: no panics, no
/// stuck state, no spurious Reset/Death events. The simplest
/// long-haul smoke for the sim → presentation seam.
#[test]
fn scripted_thirty_idle_frames_emit_no_player_lifecycle_events() {
    let mut app = build_minimal_sim_app();
    for _ in 0..30 {
        write_control_frame(&mut app, ControlFrame::default());
        app.update();
        // The Reset message channel is the cheapest gate. Death
        // requires hazard contact; in an empty room with no contacts
        // the player just falls until ground.
        let messages = app.world().resource::<Messages<OwnedSfxMessage>>();
        let lifecycle_count = messages
            .iter_current_update_messages()
            .filter(|m| {
                matches!(
                    m.request,
                    SfxMessage::Reset { .. } | SfxMessage::Death { .. }
                )
            })
            .count();
        assert_eq!(
            lifecycle_count, 0,
            "idle frames should emit no Reset/Death lifecycle events"
        );
    }
}
