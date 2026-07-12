//! **The real Mary-O lifecycle, gate-enforced.**
//!
//! The second customer of the architecture Sanic proved: Mary-O is a real
//! composable experience with direct standalone entry, host-relative Quit to
//! Home, complete activation-scoped teardown, and repeatable relaunch — proven by
//! driving the actual `build_demo_app()` host headlessly. That two unrelated
//! demos pass the identical lifecycle proof against the identical session-scope +
//! provider + shell machinery is the campaign's shared-architecture claim.

use bevy::prelude::*;

use ambition::game_shell::{ShellCommand, ShellLauncherCommand, ShellRouter};
use ambition::platformer::lifecycle::{ActiveSessionScope, SessionScopeId, SessionScopedEntity};
use ambition_demo_smb1::Smb1LevelState;
use ambition_demo_smb1_app::{build_demo_app, build_demo_app_with_home};

fn active_route(app: &App) -> Option<String> {
    app.world()
        .resource::<ShellRouter>()
        .active
        .as_ref()
        .map(|active| active.route_id.as_str().to_owned())
}

fn primary_players(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, With<ambition::actors::actor::PrimaryPlayer>>();
    query.iter(app.world()).count()
}

fn level_states(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&Smb1LevelState>();
    query.iter(app.world()).count()
}

fn session_scoped_entities(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&SessionScopedEntity>();
    query.iter(app.world()).count()
}

fn live_session_scope(app: &App) -> Option<SessionScopeId> {
    app.world().resource::<ActiveSessionScope>().current()
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

#[test]
fn mary_o_launch_quit_relaunch_is_leak_free() {
    let mut app = build_demo_app();
    settle(&mut app);

    assert_eq!(active_route(&app), Some("mary_o_gameplay".to_owned()));
    assert_eq!(
        primary_players(&mut app),
        1,
        "exactly one player in gameplay"
    );
    assert_eq!(
        level_states(&mut app),
        1,
        "the level owner exists in-session"
    );
    let first = live_session_scope(&app).expect("a session is live during gameplay");

    // Quit to the host's home (the Mary-O launcher).
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_eq!(active_route(&app), Some("mary_o_launcher".to_owned()));
    assert_eq!(
        primary_players(&mut app),
        0,
        "no player survives Quit to Home"
    );
    assert_eq!(level_states(&mut app), 0, "no stale level state at home");
    assert_eq!(
        session_scoped_entities(&mut app),
        0,
        "no session-scoped entity leaks to the launcher"
    );
    assert_eq!(live_session_scope(&app), None, "no session is live at home");

    // Relaunch through the real launcher path.
    app.world_mut()
        .write_message(ShellLauncherCommand::LaunchSelected);
    settle(&mut app);
    assert_eq!(active_route(&app), Some("mary_o_gameplay".to_owned()));
    assert_eq!(
        primary_players(&mut app),
        1,
        "exactly one player after relaunch — not two, not zero"
    );
    assert_eq!(
        level_states(&mut app),
        1,
        "a fresh level owner after relaunch"
    );
    let second = live_session_scope(&app).expect("a fresh session is live after relaunch");
    assert_ne!(first, second, "relaunch mints a DISTINCT session scope");

    // Second return is equally clean.
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_eq!(active_route(&app), Some("mary_o_launcher".to_owned()));
    assert_eq!(primary_players(&mut app), 0, "second teardown is clean");
    assert_eq!(
        session_scoped_entities(&mut app),
        0,
        "second teardown leaks nothing"
    );
}

/// The SAME `Smb1ExperiencePlugin` under a DIFFERENT host resolves `QuitToHome` to
/// THAT host's declared home route — the provider named neither launcher.
#[test]
fn mary_o_quit_to_home_is_host_relative() {
    for home in ["cabinet_home", "collection_menu"] {
        let mut app = build_demo_app_with_home(home);
        settle(&mut app);
        assert_eq!(active_route(&app), Some("mary_o_gameplay".to_owned()));
        assert_eq!(primary_players(&mut app), 1);

        app.world_mut().write_message(ShellCommand::QuitToHome);
        settle(&mut app);
        assert_eq!(
            active_route(&app),
            Some(home.to_owned()),
            "the identical provider returns to whichever home THIS host declared"
        );
        assert_eq!(primary_players(&mut app), 0, "teardown is host-independent");
    }
}
