//! **X1 — rendered (no-window) ownership across the host lifecycle.**
//!
//! Drives the REAL visible composition (`build_visible_app` — the exact App
//! the desktop binary runs, minus the window/wgpu backend) through
//! title → Ambition gameplay → title → Sanic gameplay → title, and asserts
//! presentation OWNERSHIP at every stop:
//!
//! - the host cameras exist from boot and survive every transition
//!   (host-owned infrastructure, not gameplay leakage);
//! - the title screen shows the launcher UI and ZERO gameplay presentation
//!   (no room visuals, no HUD text, no LDtk spine roots, no player);
//! - an Ambition session draws its LDtk room + HUD, all session-scoped;
//! - a Sanic session draws through the SAME provider-agnostic
//!   `SessionRoomVisualsPlugin` — no per-game visual wiring in the host;
//! - Quit to Home retires every session-owned visual exactly.

use bevy::prelude::*;

use ambition::game_shell::{ShellCommand, ShellLauncherCommand, ShellRouter};
use ambition::platformer::lifecycle::{RoomVisual, SessionScopedEntity};
use ambition::render::rendering::HudText;
use ambition_app::app::{shell_host, VisibleRenderMode};

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

fn active_route(app: &App) -> Option<String> {
    app.world()
        .resource::<ShellRouter>()
        .active
        .as_ref()
        .map(|active| active.route_id.as_str().to_owned())
}

fn count<C: Component>(app: &mut App) -> usize {
    let mut query = app.world_mut().query_filtered::<Entity, With<C>>();
    query.iter(app.world()).count()
}

fn main_cameras(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, With<ambition::platformer::camera_layers::MainCamera>>();
    query.iter(app.world()).count()
}

fn launcher_ui_roots(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, With<ambition::menu::render::bevy_ui::BevyUiMenuRoot>>();
    query.iter(app.world()).count()
}

fn assert_title_ownership(app: &mut App, context: &str) {
    assert_eq!(
        active_route(app),
        Some(shell_host::AMBITION_LAUNCHER_ROUTE.to_owned()),
        "{context}: launcher route active"
    );
    assert_eq!(
        main_cameras(app),
        1,
        "{context}: exactly one host main camera"
    );
    assert!(
        launcher_ui_roots(app) >= 1,
        "{context}: the title/launcher UI is present"
    );
    assert_eq!(
        count::<RoomVisual>(app),
        0,
        "{context}: zero room visuals under the title"
    );
    assert_eq!(count::<HudText>(app), 0, "{context}: zero gameplay HUD");
    assert_eq!(
        count::<SessionScopedEntity>(app),
        0,
        "{context}: zero session-owned entities at the title"
    );
}

#[test]
fn rendered_ownership_across_the_title_and_two_games() {
    let mut app = ambition_app::app::build_visible_app(VisibleRenderMode::NoWindow, true);
    settle(&mut app);
    assert_title_ownership(&mut app, "boot title");

    // ── Ambition ───────────────────────────────────────────────────────
    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    settle(&mut app);
    assert_eq!(
        active_route(&app),
        Some(shell_host::AMBITION_GAMEPLAY_ROUTE.to_owned()),
        "ambition session active"
    );
    assert!(
        count::<RoomVisual>(&mut app) > 0,
        "ambition: the LDtk room draws"
    );
    assert_eq!(count::<HudText>(&mut app), 1, "ambition: the HUD exists");
    assert_eq!(
        main_cameras(&mut app),
        1,
        "ambition: still exactly one host main camera"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_title_ownership(&mut app, "title after ambition");

    // ── Sanic, through the SAME generic session visuals ────────────────
    app.world_mut()
        .write_message(ShellCommand::GoTo("sanic_gameplay".into()));
    settle(&mut app);
    assert_eq!(
        active_route(&app),
        Some("sanic_gameplay".to_owned()),
        "sanic session active"
    );
    assert!(
        count::<RoomVisual>(&mut app) > 0,
        "sanic: the speedway draws through the provider-agnostic session visuals"
    );
    assert_eq!(
        count::<HudText>(&mut app),
        0,
        "sanic: Ambition's HUD does not leak into another provider's session"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_title_ownership(&mut app, "title after sanic");

    // The launcher still works after the whole cycle: relaunch Ambition
    // through the real launcher command path.
    app.world_mut()
        .write_message(ShellLauncherCommand::LaunchSelected);
    settle(&mut app);
    assert_eq!(
        active_route(&app),
        Some(shell_host::AMBITION_GAMEPLAY_ROUTE.to_owned()),
        "relaunch through the launcher lands in Ambition"
    );
    assert!(count::<RoomVisual>(&mut app) > 0, "relaunch draws again");
}
