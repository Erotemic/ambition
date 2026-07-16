//! **Sequential-session isolation gate (session-root exclusivity).**
//!
//! The architecture half of the N3.2 session campaign: activate one session,
//! populate it, tear it down through the supported lifecycle, activate a fresh
//! one, and prove that *nothing* — no entity, relationship, resource handle,
//! cache, or published read model — refers to the retired scope.
//!
//! This drives the REAL Sanic host (`build_demo_app`: foundation + engine + host
//! + shell + the Sanic provider) headlessly. The player body is
//! `simulation_world`'s real output; teardown is the shell's real
//! `SessionScopeRetired` sweep plus the provider-installed
//! `SessionTeardownPlugin` that resets the session-scoped resource mirrors.
//!
//! The existing `shell_cycle.rs` proves *entity* isolation. This test adds the
//! dimension that campaign targets: the process-global resource mirrors that the
//! entity sweep does not touch (`MovingPlatformSet`, `PossessionState`,
//! `ControlledSubject`, `EncounterRegistry`, `SandboxSimState`) — the ones that
//! used to retain dangling `Entity` handles across a teardown.

use bevy::prelude::*;

use ambition::actors::abilities::traversal::possession::PossessionState;
use ambition::actors::actor::PrimaryPlayer;
use ambition::actors::encounter::EncounterRegistry;
use ambition::game_shell::{ShellCommand, ShellLauncherCommand, ShellRouter};
use ambition::platformer::lifecycle::{ActiveSessionScope, SessionScopeId, SessionScopedEntity};
use ambition::platformer::markers::ControlledSubject;
use ambition::world::collision::MovingPlatformSet;
use ambition::world::platforms::MovingPlatformState;
use ambition_demo_sanic_app::build_demo_app;

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

fn session_scoped_entities(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&SessionScopedEntity>();
    q.iter(app.world()).count()
}

fn primary_player(app: &mut App) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryPlayer>>();
    let mut it = q.iter(app.world());
    let first = it.next();
    // Exactly one, or none.
    assert!(it.next().is_none(), "more than one primary player is live");
    first
}

const PROBE_ENCOUNTER: &str = "session_isolation_probe";

#[test]
fn a_second_session_shares_no_entity_handle_cache_or_view_with_the_first() {
    let mut app = build_demo_app();
    settle(&mut app);

    // ── Session A is live ──────────────────────────────────────────────────
    assert_eq!(active_route(&app), Some("sanic_gameplay".to_owned()));
    let scope_a = live_scope(&app).expect("a session is live during gameplay");
    let player_a = primary_player(&mut app).expect("session A has a home avatar");

    // Populate the session-scoped resource MIRRORS with distinctive session-A
    // live state. These are exactly the process-global handles the entity sweep
    // does NOT touch, so seeding them proves teardown — not the sweep — clears
    // them. Using the real player entity makes each a genuine dangling handle
    // the instant the sweep despawns it.
    app.world_mut().resource_mut::<PossessionState>().possessed = Some(player_a);
    app.world_mut()
        .resource_mut::<EncounterRegistry>()
        .ids
        .insert(PROBE_ENCOUNTER.to_owned(), player_a);
    app.world_mut().resource_mut::<ControlledSubject>().0 = Some(player_a);
    app.world_mut()
        .resource_mut::<MovingPlatformSet>()
        .0
        .push(MovingPlatformState::from_authored(
            ambition::engine_core::Vec2::new(1.0, 2.0),
            ambition::engine_core::Vec2::new(16.0, 4.0),
            32.0,
            20.0,
        ));

    // ── Tear session A down through the supported lifecycle ────────────────
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_eq!(active_route(&app), Some("sanic_launcher".to_owned()));

    // At the launcher, with A retired and B not yet activated, NOTHING may
    // refer to the retired scope — not entities, and not the resource mirrors.
    assert_eq!(
        session_scoped_entities(&mut app),
        0,
        "a session-scoped entity survived teardown"
    );
    assert_eq!(
        primary_player(&mut app),
        None,
        "the home avatar survived teardown"
    );
    assert_eq!(
        live_scope(&app),
        None,
        "a scope is still live at the launcher"
    );

    assert!(
        app.world().resource::<MovingPlatformSet>().0.is_empty(),
        "MovingPlatformSet still holds session-A platform state at the launcher"
    );
    assert_eq!(
        app.world().resource::<PossessionState>().possessed,
        None,
        "PossessionState still points at the despawned session-A body"
    );
    assert_eq!(
        app.world().resource::<ControlledSubject>().0,
        None,
        "ControlledSubject still names the despawned session-A body \
         (the sim sleeps at the launcher, so only teardown can clear it)"
    );
    assert!(
        !app.world()
            .resource::<EncounterRegistry>()
            .ids
            .contains_key(PROBE_ENCOUNTER),
        "EncounterRegistry still maps an id to the dead session-A entity"
    );

    // ── Activate session B (a fresh scope for the same provider) ───────────
    app.world_mut()
        .write_message(ShellLauncherCommand::LaunchSelected);
    settle(&mut app);
    assert_eq!(active_route(&app), Some("sanic_gameplay".to_owned()));

    let scope_b = live_scope(&app).expect("a fresh session is live after relaunch");
    assert_ne!(
        scope_a, scope_b,
        "relaunch reused the retired session scope"
    );

    let player_b = primary_player(&mut app).expect("session B has a home avatar");
    assert_ne!(
        player_a, player_b,
        "session B reused session A's home-avatar entity"
    );

    // The controlled subject belongs to the NEW session, rediscovered from B's
    // player brain — not the stale A handle.
    assert_eq!(
        app.world().resource::<ControlledSubject>().0,
        Some(player_b),
        "the controlled subject does not name session B's home avatar"
    );
    // No mirror carries session-A state into B.
    assert_eq!(
        app.world().resource::<PossessionState>().possessed,
        None,
        "session B inherited session A's possession handle"
    );
    assert!(
        !app.world()
            .resource::<EncounterRegistry>()
            .ids
            .contains_key(PROBE_ENCOUNTER),
        "session B inherited session A's encounter index probe"
    );
    // MovingPlatformSet was rebuilt from B's room (no authored platforms in the
    // Sanic demo), so the session-A probe platform is gone.
    assert!(
        app.world().resource::<MovingPlatformSet>().0.is_empty(),
        "session B inherited session A's moving-platform state"
    );
}
