//! The pause menu opened over a conversation owns the screen and the input.
//!
//! Regression (Jon, 2026-10-23): Escape during a conversation opened the cube
//! pause menu, but the dialogue box stayed on screen in front of it. Every
//! bevy_ui node draws on the order-9 front camera, and the cube is the order-8
//! `Camera3d`, so the box covered the menu.
//!
//! The scenario: Interact with Alice, open the pause menu while her first line
//! is still revealing, drive the menu, close it. While the menu is open no
//! dialogue box is drawn, the line does not reveal or advance, and the menu
//! takes Confirm. After the menu closes, the box is back on the same line at the
//! same reveal point.

use std::time::Duration;

use ambition_app::menu::model::{MenuPage, MenuPageAction};
use ambition_platformer2d::menu::ActiveMenuPages;
use ambition_platformer2d::characters::control::PlayerSlot;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::platformer::schedule::GameMode;
use ambition_platformer2d::render::dialog_ui::DialogOverlayRoot;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use crate::neighbor_prefetch_prepares_rooms::{alice as primary_body, cross};
use crate::the_note_travels_from_alice_to_bob::{
    the_shipped_session, dialog_active, npc, put_body_at, tap,
};

fn body_text(app: &App) -> String {
    app.world()
        .resource::<ambition_platformer2d::dialog::DialogState>()
        .body()
}

fn mode(app: &App) -> GameMode {
    *app.world().resource::<State<GameMode>>().get()
}

fn menu_open(app: &App) -> bool {
    app.world()
        .resource::<ambition_platformer2d::inventory_ui::InventoryUiState>()
        .visible
}

fn menu_page(app: &App) -> Option<MenuPage> {
    app.world()
        .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active
}

/// Dialogue boxes that would draw: a root that is not hidden by visibility.
fn shown_dialog_boxes(app: &mut App) -> usize {
    let world = app.world_mut();
    world
        .query_filtered::<Option<&InheritedVisibility>, With<DialogOverlayRoot>>()
        .iter(world)
        .filter(|visibility| visibility.is_none_or(|visibility| visibility.get()))
        .count()
}

#[test]
fn a_conversation_waits_under_the_pause_menu() {
    let mut app = the_shipped_session();
    // A pinned frame so the typewriter moves a known amount per update.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)));
    let body = primary_body(&mut app);
    cross(&mut app, body, PlayerSlot(0), "alice_relay");
    let alice = npc(&mut app, "npc_alice");
    let at = app.world().get::<BodyKinematics>(alice).expect("a live NPC").pos;
    put_body_at(&mut app, body, at);
    for _ in 0..10 {
        app.update();
    }

    tap(&mut app, KeyCode::KeyF);
    let mut revealing = false;
    for _ in 0..60 {
        app.update();
        if dialog_active(&app) && !body_text(&app).is_empty() {
            revealing = true;
            break;
        }
    }
    assert!(revealing, "Interact beside Alice did not start her first line");
    assert_eq!(shown_dialog_boxes(&mut app), 1, "premise: the conversation draws its box");

    tap(&mut app, KeyCode::Escape);
    for _ in 0..4 {
        app.update();
    }
    assert!(menu_open(&app), "Escape during the conversation opens the pause menu");
    assert_eq!(mode(&app), GameMode::Paused);
    assert!(dialog_active(&app), "the pause menu does not end the conversation");
    assert_eq!(
        shown_dialog_boxes(&mut app),
        0,
        "the dialogue box is drawn over the open pause menu"
    );

    // The line holds: no reveal, and no press reaches it.
    let held = body_text(&app);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(body_text(&app), held, "the line revealed under the pause menu");

    // The menu takes navigation and Confirm: Left reaches the face edge, and
    // Confirm on it turns the cube. The same Confirm on the conversation would
    // advance the line.
    let page = menu_page(&app);
    tap(&mut app, KeyCode::ArrowLeft);
    tap(&mut app, KeyCode::Enter);
    for _ in 0..4 {
        app.update();
    }
    assert!(menu_open(&app), "the menu is still open");
    assert_ne!(menu_page(&app), page, "Left then Confirm did not turn the pause menu");
    assert_eq!(body_text(&app), held, "a press for the pause menu reached the conversation");
    assert_eq!(shown_dialog_boxes(&mut app), 0, "the box came back while the menu is open");

    // Close: the same line comes back from the same reveal point.
    tap(&mut app, KeyCode::Escape);
    assert!(!menu_open(&app), "Escape closes the pause menu");
    for _ in 0..2 {
        app.update();
    }
    assert_eq!(mode(&app), GameMode::Dialogue, "closing returns to the conversation");
    assert!(dialog_active(&app), "the conversation is still open");
    assert_eq!(shown_dialog_boxes(&mut app), 1, "the box did not come back");
    let mut resumed = body_text(&app);
    for _ in 0..120 {
        if resumed.len() > held.len() {
            break;
        }
        app.update();
        resumed = body_text(&app);
    }
    assert!(
        resumed.starts_with(&held) && resumed.len() > held.len(),
        "the line did not resume from where it was held: held {held:?}, now {resumed:?}"
    );
}
