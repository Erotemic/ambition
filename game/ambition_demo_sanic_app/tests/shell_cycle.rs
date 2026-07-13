//! **The real Sanic lifecycle, gate-enforced.**
//!
//! The first required campaign milestone: Sanic is a real composable experience
//! with direct standalone entry, host-relative Quit to Home, complete
//! activation-scoped teardown, and repeatable relaunch — proven by driving the
//! actual `build_demo_app()` host (foundation + engine + host + shell + the Sanic
//! provider) headlessly and asserting on the real simulation.
//!
//! This is not a shell-only mock: the player body is `simulation_world`'s real
//! output, the act state is the mode/session-scoped rules owner, and teardown is
//! the generic `SessionScopeRetired` sweep. A leak or a duplicate would show up as
//! a surviving player, a stale act state, or an orphaned session-scoped entity.

use bevy::prelude::*;

use ambition::game_shell::{ShellCommand, ShellLauncherCommand, ShellRouter};
use ambition::platformer::lifecycle::{ActiveSessionScope, SessionScopeId, SessionScopedEntity};
use ambition_demo_sanic::SanicActState;
use ambition_demo_sanic_app::{build_demo_app, build_demo_app_with_home};

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

fn act_states(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&SanicActState>();
    query.iter(app.world()).count()
}

fn session_scoped_entities(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&SessionScopedEntity>();
    query.iter(app.world()).count()
}

fn live_session_scope(app: &App) -> Option<SessionScopeId> {
    app.world().resource::<ActiveSessionScope>().current()
}

/// Pump enough frames that a shell command's route change, the follow-on session
/// (de)activation, and its deferred spawns/despawns have all landed (the launcher
/// has a known one-frame command latency; the retire sweep is deferred too).
fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

#[test]
fn sanic_launch_quit_relaunch_is_leak_free() {
    let mut app = build_demo_app();
    settle(&mut app);

    // Direct standalone entry: the host's initial route is Sanic gameplay, and a
    // real session is live.
    assert_eq!(active_route(&app), Some("sanic_gameplay".to_owned()));
    assert_eq!(
        primary_players(&mut app),
        1,
        "exactly one player in gameplay"
    );
    assert_eq!(
        act_states(&mut app),
        1,
        "the sanic act owner exists in-session"
    );
    let first = live_session_scope(&app).expect("a session is live during gameplay");

    // Quit to the host's home (the Sanic launcher). Host-relative: the provider
    // never named this route.
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_eq!(active_route(&app), Some("sanic_launcher".to_owned()));
    assert_eq!(
        primary_players(&mut app),
        0,
        "no player survives Quit to Home"
    );
    assert_eq!(act_states(&mut app), 0, "no stale sanic act state at home");
    assert_eq!(
        session_scoped_entities(&mut app),
        0,
        "no session-scoped entity leaks to the launcher"
    );
    assert_eq!(
        live_session_scope(&app),
        None,
        "no session is live at the launcher"
    );

    // Relaunch through the REAL launcher path: select the sole registered entry.
    app.world_mut()
        .write_message(ShellLauncherCommand::LaunchSelected);
    settle(&mut app);
    assert_eq!(active_route(&app), Some("sanic_gameplay".to_owned()));
    assert_eq!(
        primary_players(&mut app),
        1,
        "exactly one player after relaunch — not two, not zero"
    );
    assert_eq!(act_states(&mut app), 1, "a fresh act owner after relaunch");
    let second = live_session_scope(&app).expect("a fresh session is live after relaunch");
    assert_ne!(
        first, second,
        "relaunch mints a DISTINCT session scope, not a reuse of the first"
    );

    // A second return is equally clean.
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_eq!(active_route(&app), Some("sanic_launcher".to_owned()));
    assert_eq!(primary_players(&mut app), 0, "second teardown is clean");
    assert_eq!(
        session_scoped_entities(&mut app),
        0,
        "second teardown leaks nothing"
    );
}

/// W1: at the launcher there is no gameplay session, and the SIMULATION —
/// its tick timeline included — sleeps. Not a shell-only claim: this drives
/// the real fixed-tick host and reads the real `SimTick`.
///
/// The poison this guards: an ungated sim at the frontend keeps ticking a
/// stale or placeholder world under the menu (burning time, mutating state,
/// and re-arming the exact class of stale-authority bugs the session model
/// exists to kill).
#[test]
fn simulation_sleeps_at_the_launcher_and_wakes_per_session() {
    let mut app = build_demo_app();
    settle(&mut app);
    assert_eq!(active_route(&app), Some("sanic_gameplay".to_owned()));

    // In-session the timeline advances: one update == one fixed tick.
    let in_session = app.world().resource::<ambition::runtime::SimTick>().0;
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<ambition::runtime::SimTick>().0,
        in_session + 2,
        "fixed-tick simulation advances while a session is live"
    );

    // At the launcher the timeline is FROZEN.
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_eq!(active_route(&app), Some("sanic_launcher".to_owned()));
    let at_home = app.world().resource::<ambition::runtime::SimTick>().0;
    for _ in 0..8 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<ambition::runtime::SimTick>().0,
        at_home,
        "no fixed-update gameplay simulation runs at the launcher"
    );

    // Relaunch wakes it again.
    app.world_mut()
        .write_message(ShellLauncherCommand::LaunchSelected);
    settle(&mut app);
    let relaunched = app.world().resource::<ambition::runtime::SimTick>().0;
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<ambition::runtime::SimTick>().0,
        relaunched + 2,
        "a fresh session resumes the simulation"
    );
}

/// The SAME `SanicExperiencePlugin` under a DIFFERENT host resolves `QuitToHome`
/// to THAT host's declared home route. The provider named neither launcher — the
/// return is semantic and host-relative — so one provider serves every host.
#[test]
fn sanic_quit_to_home_is_host_relative() {
    for home in ["studio_kiosk_home", "arcade_frontend"] {
        let mut app = build_demo_app_with_home(home);
        settle(&mut app);
        assert_eq!(active_route(&app), Some("sanic_gameplay".to_owned()));
        assert_eq!(primary_players(&mut app), 1);

        app.world_mut().write_message(ShellCommand::QuitToHome);
        settle(&mut app);
        assert_eq!(
            active_route(&app),
            Some(home.to_owned()),
            "the identical provider returns to whichever home THIS host declared"
        );
        assert_eq!(
            primary_players(&mut app),
            0,
            "teardown is host-independent — the session dies regardless of home"
        );
    }
}
