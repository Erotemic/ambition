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

/// The track the music director currently has on the base channel (empty =
/// silence). This is the REAL playback state the director writes, not merely the
/// selection — `build_visible_app` composes the actual audio director (the ALSA
/// warnings on a device-less CI box are harmless; the state machine still runs).
fn active_music_track(app: &App) -> String {
    app.world()
        .resource::<ambition::audio::library::MusicPlaybackState>()
        .active_track
        .clone()
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
    assert_eq!(
        launcher_ui_roots(app),
        1,
        "{context}: exactly one launcher/frontend UI root owns the title"
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

    // ── Mary-O, through the SAME generic session visuals ───────────────
    app.world_mut()
        .write_message(ShellCommand::GoTo("mary_o_gameplay".into()));
    settle(&mut app);
    assert_eq!(
        active_route(&app),
        Some("mary_o_gameplay".to_owned()),
        "mary-o session active"
    );
    assert!(
        count::<RoomVisual>(&mut app) > 0,
        "mary-o: the 1-1 room draws through the provider-agnostic session visuals"
    );
    assert_eq!(
        main_cameras(&mut app),
        1,
        "mary-o: still exactly one host main camera"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_title_ownership(&mut app, "title after mary-o");

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

/// **Provider-relative music at the PLAYBACK layer** (Issues 1–3).
///
/// Drives the real visible composition — which runs the actual music director,
/// `MusicIntent`, and `MusicPlaybackState` — and asserts what the base channel
/// actually plays at each stop:
///
/// - title plays the host's configured frontend theme (`a_possible_morning`);
/// - Ambition gameplay plays an Ambition-authored gameplay track (not the theme);
/// - Quit to Home restores the frontend theme;
/// - Sanic gameplay plays Sanic's own track — never Ambition's residue;
/// - Mary-O gameplay is DELIBERATELY silent (a music-less provider stops
///   playback rather than retaining the previous track).
#[test]
fn provider_relative_music_drives_the_base_channel() {
    let mut app = ambition_app::app::build_visible_app(VisibleRenderMode::NoWindow, true);
    settle(&mut app);
    assert_eq!(
        active_music_track(&app),
        "a_possible_morning",
        "the title plays the host's configured frontend theme"
    );

    // Ambition: a gameplay track takes over from the title theme.
    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    settle(&mut app);
    let ambition_track = active_music_track(&app);
    assert!(
        !ambition_track.is_empty() && ambition_track != "a_possible_morning",
        "ambition gameplay plays an authored gameplay track, not the title theme \
         (got {ambition_track:?})"
    );

    // Quit to Home restores the frontend theme.
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_eq!(
        active_music_track(&app),
        "a_possible_morning",
        "Quit to Home restores the frontend policy (the title theme resumes)"
    );

    // Sanic plays ITS track — the Ambition track that just played is still
    // resident in the combined library, but this provider does not authorize it.
    app.world_mut()
        .write_message(ShellCommand::GoTo("sanic_gameplay".into()));
    settle(&mut app);
    assert_eq!(
        active_music_track(&app),
        "you_are_too_slow",
        "Sanic plays its own authored track, never Ambition's"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_eq!(active_music_track(&app), "a_possible_morning");

    // Mary-O authored no music: deliberate silence, not the retained theme.
    app.world_mut()
        .write_message(ShellCommand::GoTo("mary_o_gameplay".into()));
    settle(&mut app);
    assert_eq!(
        active_music_track(&app),
        "",
        "Mary-O is deliberately silent — a music-less provider STOPS playback"
    );
}
