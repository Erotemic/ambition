//! Seat a real smash match, press up-B, and let the ENGINE'S OWN LOG say what
//! happened.
//!
//! ⛔⛔ THE INSTRUMENT THAT DID NOT EXIST, and its absence cost a day. The
//! integration suite can seat a match and drive a press, but `build_visible_app`
//! drops `LogPlugin` from every windowless mode — *"tests build several Apps per
//! process; the tracing subscriber is process-global"* — so every diagnostic the
//! engine emits is invisible there. Meanwhile `capture_scene`, which DOES keep
//! the log, cannot seat anybody: its own notes record `--route smash_gameplay`
//! photographing an empty stage.
//!
//! So a real bug lived in the gap: the suite could reach the behaviour and not
//! see the log; the capture could see the log and not reach the behaviour. Four
//! wrong hypotheses were argued across that gap before a player's log settled it.
//!
//! ⭐ THIS BINARY IS THE OVERLAP. One App, so the process-global subscriber is
//! safe — the same reasoning `capture_scene` already writes down — plus the
//! demo's own roster builder and a driven control frame.

use bevy::prelude::*;

fn main() {
    let mut app = ambition_app::app::build_visible_app(
        ambition_app::app::VisibleRenderMode::NoWindow,
        true,
    );
    // ⭐ ONE APP, ONE PROCESS: the exact condition that makes a global tracing
    // subscriber safe here and unsafe in the test binary.
    app.add_plugins(bevy::log::LogPlugin::default());

    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            "npc_pirate_admiral",
            "npc_pirate_admiral",
        ]));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    for _ in 0..240 {
        app.update();
    }

    // ⛔ ONE press FRAME, then HELD. `special_pressed` is a rising EDGE: holding
    // it true would be a press every tick.
    let up_special = ambition_platformer2d::engine_core::ControlFrame {
        axis_y: -1.0,
        special_pressed: true,
        special_held: true,
        ..Default::default()
    };
    ambition_platformer2d::sim::drive_control_frame(app.world_mut(), up_special);
    app.update();
    for _ in 0..9 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame {
                special_pressed: false,
                ..up_special
            },
        );
        app.update();
    }
    // Long enough for the whole ride: board, five seconds of lease, departure.
    for _ in 0..600 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame::default(),
        );
        app.update();
    }
    eprintln!("shark_ride_probe: done");
}
