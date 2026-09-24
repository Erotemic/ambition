//! Watch the select screen decide.
//!
//! `cargo run -p ambition_demo_smash_app --bin smash_tool -- select-walkthrough`
//!
//! Drives the real screen with a real cursor and real button presses, and
//! prints what it says after each one, through the same functions the cards
//! render (`role_button_text`, `card_name_text`, `SmashSelect::blocker`).
//!
//! `select_screen::layout` is a pure function of the viewport, so a headless
//! app lays the screen out against `HEADLESS_VIEWPORT` and the cursor lands
//! on the rectangles a windowed build draws. So the walkthrough clicks
//! buttons instead of setting the decision directly.
//!
//! Text, not pixels: `capture_scene --route smash_select` photographs the
//! screen; this prints what the screen believes.

use ambition_demo_smash::select::{SmashRoster, SmashSelect, MAX_SMASH_SEATS};
use ambition_demo_smash::select_screen::cursor::SelectCursors;
use ambition_demo_smash::select_screen::layout::SelectLayout;
use ambition_demo_smash::select_screen::{card_name_text, role_button_text, StartRequested};
use crate::build_demo_app;
use ambition_platformer2d::input::{MenuControlFrame, SeatMenuFrames};
use bevy::prelude::*;

pub fn run() {
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

    click(&mut app, 0, layout.role_button(0), "P1 takes a controller");
    click(&mut app, 1, layout.role_button(1), "P2 takes a controller");
    click(&mut app, 2, layout.role_button(2), "P3 takes a controller");
    click(
        &mut app,
        2,
        layout.role_button(2),
        "…and hands that card to a CPU",
    );

    // Human hands can select portraits directly. The CPU has no hand, so P1
    // borrows its token to choose that card's fighter.
    //
    // Derive the cards from the roster; do not spell indices. The catalog's
    // fighter count is content and changes.
    let cards = fighters.len();
    assert!(
        cards >= 2,
        "the composition seats {cards} fighter(s), so there is no choosing to walk through — \
         this is a finding about the catalog, not about the screen"
    );
    let picks = [(0usize, 0u8, cards - 1), (1, 1, 0), (2, 0, cards / 2)];
    for (owner_slot, driving_seat, character) in picks {
        if matches!(
            app.world()
                .resource::<SmashSelect>()
                .slot(owner_slot)
                .occupant,
            ambition_demo_smash::select::SlotOccupant::Cpu
        ) {
            let token = placed_token(&app, &layout, owner_slot);
            click(
                &mut app,
                driving_seat,
                token,
                &format!("P{} picks the CPU token up", driving_seat + 1),
            );
        }
        click(
            &mut app,
            driving_seat,
            layout.portrait(character).expect("an authored portrait"),
            &format!(
                "slot {} chooses {}",
                owner_slot + 1,
                fighters.get(character).unwrap_or("?")
            ),
        );
    }

    click(&mut app, 0, layout.start_button(), "somebody clicks START");

    let started = app
        .world()
        .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
        .is_some();
    println!("\nmatch published: {started}");
}

fn placed_token(
    app: &App,
    layout: &SelectLayout,
    slot: usize,
) -> ambition_demo_smash::select_screen::cursor::HitRect {
    ambition_demo_smash::select_screen::token_rect(
        layout,
        app.world().resource::<SmashSelect>(),
        app.world().resource::<SmashRoster>(),
        slot,
    )
    .unwrap_or_else(|| panic!("slot {slot} has no placed token on this page"))
}

/// Put one participant's cursor on a rectangle and press confirm.
fn click(
    app: &mut App,
    seat: u8,
    rect: ambition_demo_smash::select_screen::cursor::HitRect,
    what: &str,
) {
    app.world_mut()
        .resource_mut::<SelectCursors>()
        .seat_mut(seat as usize)
        .expect("seat is bounded by the caller's seat count")
        .move_to(rect.center());
    let mut frames = app.world_mut().resource_mut::<SeatMenuFrames>();
    frames.clear();
    frames.set(
        seat,
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
    let carrying = app
        .world()
        .resource::<SelectCursors>()
        .seat(0)
        .expect("seat 0")
        .carrying;
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
