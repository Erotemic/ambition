//! **X0 — the full multi-game host lifecycle, headless.**
//!
//! Drives the REAL Ambition shell-host composition (the same
//! `compose_ambition_shell_host` the visible binary uses) through the whole
//! required acceptance sequence:
//!
//! ```text
//! launcher → Sanic → launcher → Mary-O → launcher → Ambition → launcher
//!          → Sanic (fresh) → launcher → Exit
//! ```
//!
//! At every home visit it asserts the zero-state contract (no session, no
//! session entities, no player, no audio authority, frozen sim timeline) and
//! at every activation the identity contract (correct provider, exactly one
//! player wearing the provider's character, the provider's room/world
//! authority, the provider's audio selection, a NEVER-reused session scope).
//!
//! This is not a shell-only mock: Ambition's activation lowers the real LDtk
//! `central_hub_complex` into a session-scoped simulation world, and the two
//! demo providers run their real generated worlds — all in ONE App.

use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::transform::TransformPlugin;
use bevy::MinimalPlugins;

use ambition::actors::actor::PrimaryPlayer;
use ambition::actors::rooms::RoomSet;
use ambition::audio::selection::ActiveAudioSelection;
use ambition::game_shell::{
    ActiveGameplaySession, ShellCommand, ShellLauncherCommand, ShellRouter,
};
use ambition::platformer::lifecycle::{ActiveSessionScope, SessionScopeId, SessionScopedEntity};
use ambition_app::app::shell_host;

fn shell_host_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<ambition::platformer::schedule::GameMode>();
    // Host configuration FIRST: the startup constructors consult it.
    app.insert_resource(shell_host::AmbitionShellHosted);
    ambition_app::app::init_sandbox_resources(&mut app);
    ambition_app::app::add_simulation_plugins(&mut app);
    shell_host::compose_ambition_shell_host(&mut app);
    app
}

fn settle(app: &mut App) {
    for _ in 0..4 {
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

fn live_scope(app: &App) -> Option<SessionScopeId> {
    app.world().resource::<ActiveSessionScope>().current()
}

fn primary_players(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryPlayer>>();
    query.iter(app.world()).count()
}

fn session_entities(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&SessionScopedEntity>();
    query.iter(app.world()).count()
}

fn sim_tick(app: &App) -> u64 {
    app.world().resource::<ambition::runtime::SimTick>().0
}

fn worn_character(app: &mut App) -> Option<String> {
    let mut query = app
        .world_mut()
        .query_filtered::<&ambition::characters::actor::WornCharacter, With<PrimaryPlayer>>();
    query
        .iter(app.world())
        .next()
        .map(|worn| worn.id().to_owned())
}

/// The home/title zero-state contract.
fn assert_home(app: &mut App, context: &str) {
    assert_eq!(
        active_route(app),
        Some(shell_host::AMBITION_LAUNCHER_ROUTE.to_owned()),
        "{context}: the launcher is the active route"
    );
    assert!(
        app.world().resource::<ActiveGameplaySession>().0.is_none(),
        "{context}: no active gameplay session at home"
    );
    assert_eq!(live_scope(app), None, "{context}: no live session scope");
    assert_eq!(
        session_entities(app),
        0,
        "{context}: zero session-scoped entities at home"
    );
    assert_eq!(primary_players(app), 0, "{context}: zero players at home");
    assert!(
        app.world()
            .resource::<ActiveAudioSelection>()
            .current()
            .is_none(),
        "{context}: no provider owns audio playback at home"
    );
    // The simulation — its tick timeline included — sleeps at the title.
    let frozen = sim_tick(app);
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        sim_tick(app),
        frozen,
        "{context}: the sim timeline is frozen at home"
    );
}

/// Select the launcher entry at `index` (registration order:
/// Ambition, Sanic, Mary-O, Exit) and confirm it.
fn launch_entry(app: &mut App, index: usize) {
    // Reset the cursor to the top deterministically, then walk down.
    for _ in 0..8 {
        app.world_mut().write_message(ShellLauncherCommand::Next);
        app.update();
    }
    // 8 Next presses over a 4-row list land back where it started; walk to a
    // known top by pressing Next until selection wraps to 0 is fragile —
    // instead drive Previous enough times to clamp at a full wrap, then Next.
    // Simpler and exact: read-modify via commands only — set with Previous
    // presses to index 0 (wrapping), so compute walk from current selection.
    let current = app
        .world()
        .resource::<ambition::game_shell::ShellLauncherState>()
        .selected;
    let total = 4usize; // Ambition, Sanic, Mary-O, Exit
    let steps = (index + total - current % total) % total;
    for _ in 0..steps {
        app.world_mut().write_message(ShellLauncherCommand::Next);
        app.update();
    }
    assert_eq!(
        app.world()
            .resource::<ambition::game_shell::ShellLauncherState>()
            .selected,
        index,
        "launcher cursor reached entry {index}"
    );
    app.world_mut()
        .write_message(ShellLauncherCommand::LaunchSelected);
    settle(app);
}

/// The in-session identity contract. Returns the session's scope for
/// freshness comparisons.
fn assert_in_game(
    app: &mut App,
    route: &str,
    experience: &str,
    worn: Option<&str>,
    audio_provider: &str,
    context: &str,
) -> SessionScopeId {
    assert_eq!(
        active_route(app),
        Some(route.to_owned()),
        "{context}: gameplay route active"
    );
    let session = app.world().resource::<ActiveGameplaySession>();
    let instance = session.0.as_ref().unwrap_or_else(|| {
        panic!("{context}: a gameplay session is active");
    });
    assert_eq!(
        instance.activation.experience_id.as_str(),
        experience,
        "{context}: session belongs to the selected provider"
    );
    let scope = instance.scope;
    assert_eq!(
        live_scope(app),
        Some(scope),
        "{context}: the live spawn scope is the session's"
    );
    assert_eq!(
        primary_players(app),
        1,
        "{context}: exactly one player in gameplay"
    );
    if let Some(expected_worn) = worn {
        assert_eq!(
            worn_character(app).as_deref(),
            Some(expected_worn),
            "{context}: the player wears the provider's character"
        );
    }
    assert_eq!(
        app.world().resource::<ActiveAudioSelection>().provider_id(),
        Some(audio_provider),
        "{context}: the provider owns audio playback"
    );
    // The simulation runs while a session is live.
    let before = sim_tick(app);
    app.update();
    app.update();
    assert!(
        sim_tick(app) > before,
        "{context}: the sim timeline advances in-session"
    );
    scope
}

#[test]
fn the_full_multi_game_lifecycle_is_leak_free() {
    let mut app = shell_host_app();
    settle(&mut app);

    // Boot lands on the title screen: no gameplay was constructed at startup.
    assert_home(&mut app, "boot");

    // The launcher derives its entries from provider registrations.
    let entries: Vec<String> = app
        .world()
        .resource::<ambition::game_shell::ShellLaunchCatalog>()
        .entries
        .iter()
        .map(|entry| entry.label.clone())
        .collect();
    assert_eq!(
        entries,
        vec!["Ambition", "Sanic", "Mary-O"],
        "launcher entries derive from the three registered providers \
         (Exit is the built-in fourth row)"
    );

    let mut seen_scopes: Vec<SessionScopeId> = Vec::new();
    let mut fresh = |scope: SessionScopeId, context: &str| {
        assert!(
            !seen_scopes.contains(&scope),
            "{context}: session scope must never be reused"
        );
        seen_scopes.push(scope);
    };

    // ── Sanic ──────────────────────────────────────────────────────────
    launch_entry(&mut app, 1);
    let scope = assert_in_game(
        &mut app,
        "sanic_gameplay",
        "sanic",
        Some("sanic"),
        "sanic",
        "sanic #1",
    );
    fresh(scope, "sanic #1");
    assert_eq!(
        app.world()
            .resource::<RoomSet>()
            .active_spec()
            .metadata
            .mode
            .as_deref(),
        Some("sanic"),
        "sanic #1: Sanic's world authority is active"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_home(&mut app, "after sanic");

    // ── Mary-O ─────────────────────────────────────────────────────────
    launch_entry(&mut app, 2);
    let scope = assert_in_game(
        &mut app,
        "mary_o_gameplay",
        "mary_o",
        Some("mary_o"),
        "mary_o",
        "mary-o",
    );
    fresh(scope, "mary-o");
    // Mary-O registered no audio fragments: a DELIBERATE empty set, never
    // Sanic's or Ambition's music.
    assert!(
        app.world()
            .resource::<ActiveAudioSelection>()
            .music()
            .is_none(),
        "mary-o: no inherited music from a previous provider"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_home(&mut app, "after mary-o");

    // ── Ambition ───────────────────────────────────────────────────────
    launch_entry(&mut app, 0);
    let scope = assert_in_game(
        &mut app,
        shell_host::AMBITION_GAMEPLAY_ROUTE,
        shell_host::AMBITION_EXPERIENCE,
        None,
        "ambition",
        "ambition",
    );
    fresh(scope, "ambition");
    assert_eq!(
        app.world().resource::<RoomSet>().active_spec().id.as_str(),
        "central_hub_complex",
        "ambition: the real LDtk entry room is the active world authority"
    );
    assert!(
        app.world()
            .resource::<ActiveAudioSelection>()
            .music()
            .is_some(),
        "ambition: Ambition's authored music is selected"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_home(&mut app, "after ambition");

    // ── Sanic again: a FRESH session, not a resurrection ───────────────
    launch_entry(&mut app, 1);
    let scope = assert_in_game(
        &mut app,
        "sanic_gameplay",
        "sanic",
        Some("sanic"),
        "sanic",
        "sanic #2",
    );
    fresh(scope, "sanic #2");

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_home(&mut app, "after sanic #2");

    // ── Exit ───────────────────────────────────────────────────────────
    launch_entry(&mut app, 3);
    app.update();
    assert!(
        app.world().resource::<ShellRouter>().exit_requested,
        "selecting Exit raises the shell exit request"
    );
    let exit_events = app.world().resource::<Messages<AppExit>>();
    assert!(
        !exit_events.is_empty(),
        "the HOST maps the shell exit request to Bevy AppExit"
    );
}
