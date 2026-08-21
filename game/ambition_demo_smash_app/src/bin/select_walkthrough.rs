//! **Watch the select screen decide.**
//!
//! `cargo run -p ambition_demo_smash_app --bin select_walkthrough`
//!
//! Drives the real screen with a real cursor and real button presses, and
//! prints what it says after each one — through the SAME functions the cards
//! render (`role_button_text`, `card_name_text`, `SmashSelect::blocker`), so
//! this cannot show a screen the player would not see.
//!
//! ⭐ **the geometry is real too.** `select_screen::layout` is a pure function
//! of the viewport, so a headless app lays the screen out against
//! `HEADLESS_VIEWPORT` and the cursor lands on the same rectangles a windowed
//! build would draw. That is what lets a text walkthrough click a BUTTON rather
//! than reach into the decision and set the answer — reaching into the answer is
//! how this screen once came to be fully unit-tested and completely inert.
//!
//! It is text rather than pixels on purpose: `capture_scene --route smash_select`
//! photographs the real thing, and this prints what the screen BELIEVES, which a
//! photograph cannot.

use ambition_demo_smash::select::{SmashRoster, SmashSelect, MAX_SMASH_SEATS};
use ambition_demo_smash::select_screen::cursor::SelectCursors;
use ambition_demo_smash::select_screen::layout::SelectLayout;
use ambition_demo_smash::select_screen::{card_name_text, role_button_text, StartRequested};
use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::input::{MenuControlFrame, SeatMenuFrames};
use bevy::prelude::*;

fn main() {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let devices: Vec<Entity> = (0..3).map(|_| app.world_mut().spawn(()).id()).collect();
    app.world_mut()
        .insert_resource(ambition_platformer2d::input::LocalDeviceOrder::from_devices(devices));

    let fighters = app.world().resource::<SmashRoster>().clone();
    let layout = SelectLayout::for_viewport(None, fighters.cell_count());
    show(&mut app, "the screen as three people find it");

    click(&mut app, layout.role_button(0), "P1 takes a controller");
    click(&mut app, layout.role_button(1), "P2 takes a controller");
    click(&mut app, layout.role_button(2), "P3's card is toggled…");
    click(&mut app, layout.role_button(2), "…and again, to a CPU");

    // Each participating card's token starts in the pool; pick it up and drop
    // it on a portrait. Two clicks, exactly as a pad does it.
    for (slot, character) in [(0usize, 4usize), (1, 0), (2, 6)] {
        click(
            &mut app,
            layout.token_home(slot),
            &format!("P{} picks their token up", slot + 1),
        );
        click(
            &mut app,
            layout.portrait(character).expect("an authored portrait"),
            &format!(
                "P{} drops it on {}",
                slot + 1,
                fighters.get(character).unwrap_or("?")
            ),
        );
    }

    click(&mut app, layout.start_button(), "somebody clicks START");

    let started = app
        .world()
        .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
        .is_some();
    println!("\nmatch published: {started}");
}

/// Put the cursor on a rectangle and press confirm, from seat 0.
fn click(app: &mut App, rect: ambition_demo_smash::select_screen::cursor::HitRect, what: &str) {
    app.world_mut()
        .resource_mut::<SelectCursors>()
        .seat_mut(0)
        .move_to(rect.center());
    let mut frames = app.world_mut().resource_mut::<SeatMenuFrames>();
    frames.clear();
    frames.set(
        0,
        MenuControlFrame {
            select: true,
            ..Default::default()
        },
    );
    app.update();
    // The edge has to fall, or the next frame reads the same press again.
    app.world_mut().resource_mut::<SeatMenuFrames>().clear();
    app.update();
    show(app, what);
}

fn show(app: &mut App, what: &str) {
    let select = *app.world().resource::<SmashSelect>();
    let fighters = app.world().resource::<SmashRoster>().clone();
    // The REAL catalog, so the cards print display names rather than ids —
    // which is also the only check here that the portrait/name lookup resolves.
    let catalog = app
        .world()
        .get_resource::<ambition_platformer2d::character::CharacterCatalog>()
        .cloned();
    let carrying = app.world().resource::<SelectCursors>().seat(0).carrying;
    let asked = app.world().resource::<StartRequested>().0;
    println!("\n── {what} ──");
    println!("   ┌────────────────────────────────────────────────┐");
    for slot in 0..MAX_SMASH_SEATS {
        let card = select.slot(slot);
        let held = if carrying == Some(slot) { " ✋" } else { "" };
        println!(
            "   │ P{}  {:<14} {:<20}{:<3}│",
            slot + 1,
            // The walkthrough renders the screen's own text without a live input
            // world, so the button falls back to its index rather than naming a
            // device it cannot see.
            role_button_text(card.occupant, None),
            card_name_text(
                catalog.as_ref(),
                &fighters,
                card.occupant.participates().then_some(card.pick).flatten()
            ),
            held
        );
    }
    println!("   ├────────────────────────────────────────────────┤");
    println!(
        "   │ {:<46} │",
        select.blocker().unwrap_or(if asked {
            "Starting…"
        } else {
            "Ready — click START"
        })
    );
    println!("   └────────────────────────────────────────────────┘");
}
