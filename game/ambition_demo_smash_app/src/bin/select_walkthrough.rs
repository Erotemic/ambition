//! **Watch the select screen decide.**
//!
//! `cargo run -p ambition_demo_smash_app --bin select_walkthrough`
//!
//! Drives the real screen with real button presses and prints what it says after
//! each one — through `select_ui::panel_text`, the SAME function the UI panels
//! render, so this cannot show a screen the player would not see.
//!
//! It is text rather than pixels on purpose. `stage_diagram` draws geometry
//! because geometry is what that room is; a select screen is WORDS, and a
//! hand-rolled 5x7 font rendering them into a PNG would be a less faithful view
//! of the same strings, not a more faithful one.

use ambition_platformer2d::input::{MenuControlFrame, SeatMenuFrames};
use ambition_demo_smash::select::{MAX_SMASH_SEATS, SmashSelect};
use ambition_demo_smash::select_ui::{panel_text, prompt_text};
use ambition_demo_smash_app::build_demo_app;
use bevy::prelude::*;

fn main() {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let devices: Vec<Entity> = (0..3).map(|_| app.world_mut().spawn(()).id()).collect();
    app.world_mut()
        .insert_resource(ambition_platformer2d::input::LocalDeviceOrder::from_devices(devices));

    show(&mut app, "the screen as three people find it");
    step(
        &mut app,
        0,
        confirm(),
        "P1 presses confirm at an empty seat",
    );
    step(&mut app, 0, right(), "P1 moves the cursor");
    step(&mut app, 0, confirm(), "P1 locks in");
    step(
        &mut app,
        2,
        confirm(),
        "P3 joins while P2 is still deciding",
    );
    step(&mut app, 1, confirm(), "P2 joins");
    step(
        &mut app,
        1,
        confirm(),
        "P2 locks in — but P3 is still browsing",
    );
    step(&mut app, 2, confirm(), "P3 locks in");

    let started = app
        .world()
        .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
        .is_some();
    println!("\nmatch published: {started}");
}

fn confirm() -> MenuControlFrame {
    MenuControlFrame {
        select: true,
        ..Default::default()
    }
}

fn right() -> MenuControlFrame {
    MenuControlFrame {
        right: true,
        ..Default::default()
    }
}

fn step(app: &mut App, seat: u8, frame: MenuControlFrame, what: &str) {
    let mut frames = app.world_mut().resource_mut::<SeatMenuFrames>();
    frames.clear();
    frames.set(seat, frame);
    app.update();
    app.world_mut().resource_mut::<SeatMenuFrames>().clear();
    app.update();
    show(app, what);
}

fn show(app: &mut App, what: &str) {
    let select = *app.world().resource::<SmashSelect>();
    println!("\n── {what} ──");
    println!("   ┌──────────────────────────────────┐");
    // EVERY seat is printed, because every seat is on the real screen: the ones
    // past the pad count cannot be joined, and can still be CPUs. `offered` is
    // what changes the WORDS on a panel, not whether it exists.
    let offered = app
        .world()
        .get_resource::<ambition_platformer2d::input::LocalDeviceOrder>()
        .map(ambition_demo_smash::select::seats_offered)
        .unwrap_or(1);
    for seat in 0..MAX_SMASH_SEATS {
        println!(
            "   │ {:<32} │",
            panel_text(seat, select.seat(seat), seat < offered)
        );
    }
    println!("   ├──────────────────────────────────┤");
    println!("   │ {:<32} │", prompt_text(&select));
    println!("   └──────────────────────────────────┘");
}
