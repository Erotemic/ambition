//! X0 — the full multi-game host lifecycle, headless.
//!
//! Drives the REAL Ambition shell-host composition (the same
//! `compose_ambition_shell_host` the visible binary uses) through the whole
//! required acceptance sequence:
//!
//! ```text
//! launcher → Sanic → launcher → Mary-O → launcher
//!          → Ambition → launcher → Sanic (fresh) → launcher → Exit
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
use bevy::ecs::system::RunSystemOnce;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::transform::TransformPlugin;
use bevy::MinimalPlugins;

use ambition_app::app::shell_host;
use ambition_platformer2d::audio::selection::ActiveAudioSelection;
use ambition_platformer2d::game_shell::{
    ActiveGameplaySession, ShellCommand, ShellLauncherCommand, ShellRouter,
};
use ambition_platformer2d::platformer::lifecycle::{
    session_world_component, session_world_entity, ActiveSessionScope, SessionRoot, SessionScopeId,
    SessionScopedEntity, SessionWorldMut,
};
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::world::rooms::RoomSet;

/// The shipped shell host, headless, under an EXPLICIT simulation host.
///
/// ⛔⛔ THE HOST IS NOT A DETAIL OF THE FIXTURE. This walk ran only under
/// [`SimulationHost::RenderFrame`] for its whole life, which means GGRS was
/// never installed and the entire class of cross-session rollback contamination
/// was structurally invisible to the one test that walks every game in
/// sequence. `SimulationHost::Rollback` is what the visible binary actually
/// composes (`visible_composition.rs`), so the render-frame arm is the
/// approximation, not the other way round.
fn shell_host_app_hosted_by(host: ambition_platformer2d::runtime::SimulationHost) -> App {
    use ambition_platformer2d::runtime::SimulationHostAppExt as _;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // PINNED, because `app.update()` is otherwise a unit of WALL CLOCK — and a
    // rollback host derives its tick count from elapsed time, so an unpinned
    // clock advances the GGRS timeline zero frames in a headless walk.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<ambition_platformer2d::platformer::schedule::GameMode>();
    // Host configuration FIRST: the startup constructors consult it.
    app.insert_resource(shell_host::AmbitionShellHosted);
    ambition_app::app::init_sandbox_resources(&mut app);
    // Bevy seals the simulation schedule when the first simulation plugin
    // registers, so the host is chosen before `add_simulation_plugins` — the
    // same deadline `visible_composition.rs` documents.
    app.set_simulation_host(host);
    ambition_app::app::add_simulation_plugins(&mut app);
    shell_host::compose_ambition_shell_host(&mut app);
    app
}

fn shell_host_app() -> App {
    shell_host_app_hosted_by(ambition_platformer2d::runtime::SimulationHost::RenderFrame)
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

fn session_roots(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&SessionRoot>();
    query.iter(app.world()).count()
}

fn live_room_set(app: &App) -> &RoomSet {
    session_world_component::<RoomSet>(app.world()).expect("one exact live session room set")
}

fn sim_tick(app: &App) -> u64 {
    app.world()
        .resource::<ambition_platformer2d::runtime::SimTick>()
        .0
}

fn worn_character(app: &mut App) -> Option<String> {
    let mut query = app
        .world_mut()
        .query_filtered::<&ambition_platformer2d::characters::actor::WornCharacter, With<PrimaryPlayer>>();
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
    // Structural (not merely gated) absence of world authority: the session is
    // the canonical world reference-holder, and there is no session at home.
    assert!(
        app.world()
            .resource::<ActiveGameplaySession>()
            .active_world_entity()
            .is_none(),
        "{context}: no active gameplay-world authority at home (session owns the world ref)"
    );
    assert!(
        session_world_entity(app.world()).is_none(),
        "{context}: no canonical session-world root exists at home"
    );
    assert_eq!(
        session_roots(app),
        0,
        "{context}: title structurally exposes no gameplay-world authority"
    );
    assert!(
        app.world()
            .resource::<ambition_platformer2d::game_shell::PreparedSessionRegistry>()
            .is_empty(),
        "{context}: no prepared-session publication remains"
    );
    assert!(
        app.world()
            .resource::<ambition_platformer2d::load::LoadCoordinator>()
            .is_empty(),
        "{context}: no provider load transaction remains"
    );
    assert_eq!(live_scope(app), None, "{context}: no live session scope");
    assert_eq!(
        session_entities(app),
        0,
        "{context}: zero session-scoped entities at home"
    );
    assert_eq!(primary_players(app), 0, "{context}: zero players at home");
    let selection = app.world().resource::<ActiveAudioSelection>();
    assert!(
        matches!(
            selection.owner(),
            Some(ambition_platformer2d::sfx::AudioContextOwner::Frontend(_))
        ),
        "{context}: the exact launcher activation owns frontend audio"
    );
    // No title-track assertion here on purpose. `preferred_track()` and the
    // music authority are both BUILT from `FrontendAudioProfile::title_track`,
    // so comparing them to it only proves one field can be read through two
    // accessors. What this test is actually about is ownership — asserted above
    // and below: the exact launcher activation owns frontend audio, and menu
    // SFX are authorized without granting gameplay SFX. Whether the configured
    // theme reaches the speakers is proven end-to-end in
    // `shell_host_rendered::provider_relative_music_drives_the_base_channel`,
    // which drives the real director and reads the base channel.
    assert!(
        selection
            .sfx_authority()
            .allows(ambition_platformer2d::sfx::ids::UI_MENU_MOVE),
        "{context}: frontend menu SFX are authorized without granting gameplay SFX"
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
/// Ambition, Sanic, Mary-O, Smash, Versus, Exit) and confirm it.
/// Launcher rows = registered experience entries + built-in host actions (the
/// Exit row, when the host shows it). Derived, never a literal.
fn launcher_row_count(app: &App) -> usize {
    use ambition_platformer2d::game_shell::{ShellLaunchCatalog, ShellLauncherPresentation};
    let experiences = app.world().resource::<ShellLaunchCatalog>().entries.len();
    let exit = app
        .world()
        .resource::<ShellLauncherPresentation>()
        .exit_label
        .is_some() as usize;
    experiences + exit
}

fn launch_entry(app: &mut App, index: usize) {
    select_entry(app, index);
    app.world_mut()
        .write_message(ShellLauncherCommand::LaunchSelected);
    settle(app);
}

/// Launch the row with this LABEL.
///
/// A label is what the walk actually means, it survives reordering, and it makes
/// the panic name the game somebody was looking for.
fn launch_labeled(app: &mut App, label: &str) {
    let index = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellLaunchCatalog>()
        .entries
        .iter()
        .position(|entry| entry.label == label)
        .unwrap_or_else(|| {
            let offered: Vec<&str> = app
                .world()
                .resource::<ambition_platformer2d::game_shell::ShellLaunchCatalog>()
                .entries
                .iter()
                .map(|entry| entry.label.as_str())
                .collect();
            panic!("the launcher offers no `{label}` row; it offers {offered:?}")
        });
    launch_entry(app, index);
}

/// Move the launcher cursor to `index` and prove it arrived, without launching.
///
/// See the exit block in the lifecycle walk.
fn select_entry(app: &mut App, index: usize) {
    // Reset the cursor to the top deterministically, then walk down.
    for _ in 0..8 {
        app.world_mut().write_message(ShellLauncherCommand::Next);
        app.update();
    }
    // Simpler and exact: read-modify via commands only — set with Previous presses to index 0
    // (wrapping), so compute walk from current selection.
    let current = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellLauncherState>()
        .selected;
    // Derive the row count from the registered entries plus the built-in host
    // actions (the Exit row), never a hard-coded literal — adding a provider or
    // toggling Exit must not silently desync this walk.
    let total = launcher_row_count(app);
    let steps = (index + total - current % total) % total;
    for _ in 0..steps {
        app.world_mut().write_message(ShellLauncherCommand::Next);
        app.update();
    }
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d::game_shell::ShellLauncherState>()
            .selected,
        index,
        "launcher cursor reached entry {index}"
    );
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
    // The exact session root is the sole live world authority. Its RoomSet
    // component names THIS activation's active room; no resident projection
    // exists to retain stale state across providers.
    let world_entity = session
        .active_world_entity()
        .unwrap_or_else(|| panic!("{context}: the session owns a live world entity"));
    assert_eq!(
        session_world_entity(app.world()),
        Some(world_entity),
        "{context}: the active session owns the unique canonical world root"
    );
    let session_room = app
        .world()
        .get::<RoomSet>(world_entity)
        .unwrap_or_else(|| panic!("{context}: the live root carries RoomSet authority"))
        .active_spec()
        .id
        .clone();
    let prepared = app
        .world()
        .get::<ambition_platformer2d::runtime::PreparedContent>(world_entity)
        .unwrap_or_else(|| {
            panic!("{context}: the live root owns exact immutable prepared content")
        });
    let prepared_identity = app
        .world()
        .get::<ambition_platformer2d::runtime::PreparedContentIdentity>(world_entity)
        .copied()
        .unwrap_or_else(|| panic!("{context}: the live root exposes exact content identity"));
    assert_eq!(
        prepared.identity(),
        prepared_identity,
        "{context}: inspectable identity describes the exact prepared object",
    );
    assert_eq!(
        prepared.source().catalogs().world_provider.as_str(),
        experience,
        "{context}: prepared world ownership matches the activated provider",
    );
    assert_eq!(
        prepared.snapshot_schema(),
        app.world()
            .resource::<ambition_platformer2d::rollback::RollbackRegistry>()
            .schema_fingerprint(),
        "{context}: prepared content is bound to the active GGRS rollback schema",
    );
    assert_eq!(
        session_room,
        live_room_set(app).active_spec().id,
        "{context}: every reader observes the same root component"
    );
    assert_eq!(
        session_roots(app),
        1,
        "{context}: exactly one canonical session-world root exists"
    );
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
    let selection = app.world().resource::<ActiveAudioSelection>();
    assert_eq!(
        selection.provider_id(),
        Some(audio_provider),
        "{context}: the provider owns audio playback"
    );
    // Authority is the PERMISSION the music director enforces, not merely a
    // selection label. A session that authored music governs exactly its own
    // tracks; a music-less provider is deliberate silence (never "retain the
    // previous provider's track").
    let authority = selection.music_authority();
    assert!(
        authority.is_governed(),
        "{context}: an active session governs music authority"
    );
    assert!(
        selection.sfx_authority().is_governed(),
        "{context}: an active session governs SFX authority (never ungoverned in gameplay)"
    );
    match selection.music() {
        Some(music) => {
            assert!(
                !authority.is_deliberate_silence(),
                "{context}: a provider with music is not silence"
            );
            assert!(
                authority.allows(&music.default_track),
                "{context}: the provider's own default track is authorized"
            );
        }
        None => assert!(
            authority.is_deliberate_silence(),
            "{context}: a music-less provider is deliberate silence, not retain"
        ),
    }
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

/// The whole walk, under BOTH shipped simulation hosts.
///
/// ⭐ Parameterized rather than duplicated: every leak this walk looks for is a
/// property of session lifetime, and session lifetime is exactly what changes
/// when a rollback timeline is installed underneath it.
#[test]
fn the_full_multi_game_lifecycle_is_leak_free() {
    the_full_multi_game_lifecycle(ambition_platformer2d::runtime::SimulationHost::RenderFrame);
}

#[test]
fn the_full_multi_game_lifecycle_is_leak_free_under_rollback() {
    the_full_multi_game_lifecycle(ambition_platformer2d::runtime::SimulationHost::Rollback);
}

fn the_full_multi_game_lifecycle(host: ambition_platformer2d::runtime::SimulationHost) {
    let mut app = shell_host_app_hosted_by(host);
    settle(&mut app);

    // Boot lands on the title screen: no gameplay was constructed at startup.
    assert_home(&mut app, "boot");

    // The launcher derives its entries from provider registrations.
    let entries: Vec<String> = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellLaunchCatalog>()
        .entries
        .iter()
        .map(|entry| entry.label.clone())
        .collect();
    assert_eq!(
        entries,
        vec!["Ambition", "Sanic", "Mary-O", "Smash"],
        "launcher entries derive from the registered experiences, MINUS the \
         unlisted ones. An exact list on purpose — a launcher that silently gains \
         or loses a row is the first thing a player sees."
    );

    // Dropping their composition would have been the tempting reading and the wrong one: `Versus`
    // CANNOT be a standalone binary, because its fighters come from two different provider plugins
    // and this host is the only place both casts exist. So the row goes and the stage stays.
    let registered: Vec<String> = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellExperienceRegistry>()
        .iter()
        .map(|registration| registration.display_name.clone())
        .collect();
    // ⚠ POCKET LEFT THE SHIPPED COMPOSITION 2026-09-01 (it was already unlisted;
    // now it is not registered either, and builds only for tests). `Versus` still
    // carries this property — an experience may be REGISTERED and not offered.
    for unlisted in ["Versus"] {
        assert!(
            registered.iter().any(|name| name == unlisted),
            "`{unlisted}` must stay REGISTERED while being unlisted; found {registered:?}"
        );
        assert!(
            !entries.iter().any(|name| name == unlisted),
            "`{unlisted}` must not be offered in the launcher"
        );
    }

    // A ROW MAY LEAD TO A QUESTION RATHER THAN TO A GAME. Smash is the first
    // one that does: its entry opens character select, and the stage route it
    // reaches afterwards is a different route entirely. A launcher that could
    // only address gameplay routes would have had to drop a lone duelist onto the
    // platform with nobody to fight.
    let smash_row = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellLaunchCatalog>()
        .entries
        .iter()
        .find(|entry| entry.label == "Smash")
        .expect("the smash row exists")
        .clone();
    assert_eq!(
        smash_row.route_id,
        ambition_platformer2d::game_shell::ShellRouteId::new(
            ambition_demo_smash::SMASH_SELECT_ROUTE
        ),
        "the Smash row must open the select screen, not the stage"
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
    launch_labeled(&mut app, "Sanic");
    let scope = assert_in_game(
        &mut app,
        "sanic_gameplay",
        "sanic",
        Some("sanic"),
        "sanic",
        "sanic #1",
    );
    fresh(scope, "sanic #1");
    let sanic_world_1 = app
        .world()
        .resource::<ActiveGameplaySession>()
        .active_world_entity()
        .expect("sanic #1 owns a canonical world entity");
    let sanic_content_1 = *app
        .world()
        .get::<ambition_platformer2d::runtime::PreparedContentIdentity>(sanic_world_1)
        .expect("sanic #1 owns exact content identity");
    assert_eq!(
        live_room_set(&app).active_spec().metadata.mode.as_deref(),
        Some("sanic"),
        "sanic #1: Sanic's world authority is active"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_home(&mut app, "after sanic");

    // ── Mary-O ─────────────────────────────────────────────────────────
    launch_labeled(&mut app, "Mary-O");
    let scope = assert_in_game(
        &mut app,
        "mary_o_gameplay",
        "mary_o",
        Some("mary_o"),
        "mary_o",
        "mary-o",
    );
    fresh(scope, "mary-o");
    // Mary-O authors its own "Support Theme": provider-relative audio selects
    // Mary-O's own track, never inherited residue from Sanic or Ambition.
    assert_eq!(
        app.world()
            .resource::<ActiveAudioSelection>()
            .music()
            .map(|registry| registry.default_track.as_str()),
        Some("support_theme"),
        "mary-o: plays its own authored theme, not a previous provider's music"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_home(&mut app, "after mary-o");

    // ── Ambition ───────────────────────────────────────────────────────
    launch_labeled(&mut app, "Ambition");
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
        live_room_set(&app).active_spec().id.as_str(),
        "central_hub_complex",
        "ambition: the real LDtk entry room is the active world authority"
    );
    let ambition_world_entity = app
        .world()
        .resource::<ActiveGameplaySession>()
        .active_world_entity()
        .expect("Ambition owns a canonical world entity");
    let ambition_identity_before_room_change = *app
        .world()
        .get::<ambition_platformer2d::runtime::PreparedContentIdentity>(ambition_world_entity)
        .expect("Ambition root owns exact prepared identity");
    let alternate_room = live_room_set(&app)
        .rooms
        .iter()
        .find(|room| room.id != "central_hub_complex")
        .map(|room| room.id.clone())
        .expect("Ambition publishes more than one room");
    let alternate_room_for_edit = alternate_room.clone();
    app.world_mut()
        .run_system_once(
            move |mut room_set: SessionWorldMut<RoomSet>,
                  mut geometry: SessionWorldMut<
                ambition_platformer2d::engine_core::RoomGeometry,
            >,
                  mut active_room: SessionWorldMut<
                ambition_platformer2d::world::rooms::ActiveRoomMetadata,
            >| {
                let index = room_set
                    .room_index_by_id(&alternate_room_for_edit)
                    .expect("alternate authored room exists");
                room_set.set_active(index);
                let spec = room_set.active_spec().clone();
                geometry.0 = spec.world.clone();
                active_room.0 = spec.metadata.clone();
            },
        )
        .expect("session-world mutation system runs");
    app.update();
    let live_entity = app
        .world()
        .resource::<ActiveGameplaySession>()
        .active_world_entity()
        .expect("Ambition world remains active");
    assert_eq!(
        app.world()
            .get::<RoomSet>(live_entity)
            .expect("canonical live RoomSet")
            .active_spec()
            .id,
        alternate_room,
        "a room change is recorded directly in the canonical mutable session world",
    );
    assert_eq!(
        live_room_set(&app).active_spec().id.as_str(),
        alternate_room.as_str(),
        "all world readers observe the same exact root component",
    );
    assert_eq!(
        app.world()
            .get::<ambition_platformer2d::runtime::PreparedContentIdentity>(live_entity)
            .copied(),
        Some(ambition_identity_before_room_change),
        "ordinary room movement must retain the exact prepared fingerprint and epoch",
    );

    let ambition_default_track = app
        .world()
        .resource::<ActiveAudioSelection>()
        .music()
        .expect("ambition: Ambition's authored music is selected")
        .default_track
        .clone();

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_home(&mut app, "after ambition");

    // ── Sanic again: a FRESH session, not a resurrection ───────────────
    launch_labeled(&mut app, "Sanic");
    let scope = assert_in_game(
        &mut app,
        "sanic_gameplay",
        "sanic",
        Some("sanic"),
        "sanic",
        "sanic #2",
    );
    fresh(scope, "sanic #2");
    let sanic_world_2 = app
        .world()
        .resource::<ActiveGameplaySession>()
        .active_world_entity()
        .expect("sanic #2 owns a canonical world entity");
    let sanic_content_2 = *app
        .world()
        .get::<ambition_platformer2d::runtime::PreparedContentIdentity>(sanic_world_2)
        .expect("sanic #2 owns exact content identity");
    assert_eq!(
        sanic_content_1.fingerprint, sanic_content_2.fingerprint,
        "same authored definitions have the same content fingerprint",
    );
    assert_ne!(
        sanic_content_1.epoch, sanic_content_2.epoch,
        "a sequential activation receives a fresh App-local content epoch",
    );
    assert_ne!(
        sanic_world_1, sanic_world_2,
        "same-provider relaunch constructs a fresh mutable world entity",
    );
    assert_eq!(
        live_room_set(&app).active_spec().id.as_str(),
        ambition_demo_sanic::SPEEDWAY_ROOM_ID,
        "same-provider relaunch starts from newly authored world state",
    );
    // Provider-relative-authority poison (Issue 1): Ambition ran a moment ago and
    // its default track is still resident in the process-wide combined library.
    // A Sanic session must NOT be authorized to play it — the library is storage,
    // the provider is permission.
    let sanic_authority = app
        .world()
        .resource::<ActiveAudioSelection>()
        .music_authority();
    assert!(
        !sanic_authority.allows(&ambition_default_track),
        "sanic #2: an Ambition track present in the combined library is NOT \
         authorized for a Sanic session"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_home(&mut app, "after sanic #2");

    // ── Exit ───────────────────────────────────────────────────────────
    let exit_index = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellLaunchCatalog>()
        .entries
        .len();
    select_entry(&mut app, exit_index);
    app.world_mut()
        .write_message(ShellLauncherCommand::LaunchSelected);

    // A flaky standing guard is worse than no guard: it teaches the reader to re-run rather than to
    // look.
    let mut saw_app_exit = false;
    for _ in 0..8 {
        app.update();
        if !app.world().resource::<Messages<AppExit>>().is_empty() {
            saw_app_exit = true;
            break;
        }
    }
    assert!(
        app.world().resource::<ShellRouter>().exit_requested,
        "selecting Exit raises the shell exit request"
    );
    assert!(
        saw_app_exit,
        "the HOST maps the shell exit request to Bevy AppExit"
    );
}

/// Every live encounter authority, as `(encounter id, owning session scope)`.
fn encounter_authorities(app: &mut App) -> Vec<(String, Option<SessionScopeId>)> {
    let mut query = app.world_mut().query::<(
        &ambition_platformer2d::encounter::Encounter,
        Option<&SessionScopedEntity>,
    )>();
    let mut rows: Vec<_> = query
        .iter(app.world())
        .map(|(enc, owner)| (enc.id.clone(), owner.map(|owner| owner.0)))
        .collect();
    rows.sort();
    rows
}

/// A GGRS session contract never survives session retirement.
///
/// The shell does not start networking by default, but the exact content/schema
/// contract is session-scoped. Retiring the canonical root removes the only
/// prepared identity a future GGRS session may bind to; successor activation
/// receives a fresh session scope and prepared epoch.
#[test]
fn rollback_contract_inputs_never_leak_across_sessions() {
    let mut app = shell_host_app();
    settle(&mut app);

    launch_entry(&mut app, 0);
    settle(&mut app);
    let scope_a = live_scope(&app).expect("Ambition session A is live");
    let identity_a = {
        let world = app.world_mut();
        let mut query = world.query::<&ambition_platformer2d::runtime::PreparedContentIdentity>();
        query
            .single(world)
            .copied()
            .expect("session A exposes prepared identity")
    };

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    let prepared_identity_is_gone = {
        let world = app.world_mut();
        let mut query = world.query::<&ambition_platformer2d::runtime::PreparedContentIdentity>();
        query.iter(world).next().is_none()
    };
    assert!(
        prepared_identity_is_gone,
        "retirement removes the prepared identity a rollback session would bind to"
    );

    launch_entry(&mut app, 0);
    settle(&mut app);
    let scope_b = live_scope(&app).expect("Ambition session B is live");
    let identity_b = {
        let world = app.world_mut();
        let mut query = world.query::<&ambition_platformer2d::runtime::PreparedContentIdentity>();
        query
            .single(world)
            .copied()
            .expect("session B exposes prepared identity")
    };

    assert_ne!(scope_a, scope_b, "session scopes are never reused");
    assert_ne!(
        identity_a.epoch, identity_b.epoch,
        "successor activation gets a fresh content epoch"
    );
    assert_eq!(
        identity_a.fingerprint, identity_b.fingerprint,
        "equivalent authored content keeps its fingerprint"
    );
}

/// Activate A, prove ownership; retire A, prove nothing remains; activate B, prove exactly one
/// authority per id, all B's.
#[test]
fn the_encounter_authorities_belong_to_their_session() {
    let mut app = shell_host_app();
    settle(&mut app);
    assert_home(&mut app, "boot");

    // ── Session A: Ambition ────────────────────────────────────────────
    launch_entry(&mut app, 0);
    settle(&mut app);
    let scope_a = live_scope(&app).expect("Ambition session A is live");
    let authorities_a = encounter_authorities(&mut app);
    assert!(
        !authorities_a.is_empty(),
        "Ambition's activation populates encounter authorities"
    );
    assert!(
        authorities_a
            .iter()
            .any(|(id, _)| id == "symmetry_attunement"),
        "the Noether attunement authority is among them: {authorities_a:?}"
    );
    for (id, owner) in &authorities_a {
        assert_eq!(
            *owner,
            Some(scope_a),
            "authority `{id}` is owned by session A"
        );
    }
    let ids_a: Vec<&String> = authorities_a.iter().map(|(id, _)| id).collect();
    let mut unique_a = ids_a.clone();
    unique_a.dedup();
    assert_eq!(ids_a, unique_a, "exactly one authority per encounter id");

    // ── Retire A ───────────────────────────────────────────────────────
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_home(&mut app, "after Ambition session A");
    assert_eq!(
        encounter_authorities(&mut app),
        vec![],
        "no encounter authority survives its session's retirement"
    );

    // ── Session B: Ambition again ──────────────────────────────────────
    launch_entry(&mut app, 0);
    settle(&mut app);
    let scope_b = live_scope(&app).expect("Ambition session B is live");
    assert_ne!(scope_a, scope_b, "session scopes are never reused");
    let authorities_b = encounter_authorities(&mut app);
    assert_eq!(
        authorities_b.iter().map(|(id, _)| id).collect::<Vec<_>>(),
        ids_a,
        "session B repopulates the same authority roster, one per id"
    );
    for (id, owner) in &authorities_b {
        assert_eq!(
            *owner,
            Some(scope_b),
            "authority `{id}` is owned by session B, not a survivor of A"
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Cross-game rollback contamination
// ──────────────────────────────────────────────────────────────────────────

/// The confirmation authority the ACTIVE gameplay session may read.
///
/// ⛔ ASKED THE WAY A GAMEPLAY SYSTEM ASKS IT: for the LIVE scope. A test that
/// read the authority's own status would pass while every consumer refused,
/// because the whole defect is that the value belonged to somebody else.
fn confirmation(app: &App) -> ambition_platformer2d::runtime::RollbackConfirmationState {
    use ambition_platformer2d::platformer::lifecycle::live_session_scope;

    app.world()
        .get_resource::<ambition_platformer2d::rollback::ActiveRollbackAuthority>()
        .map(|authority| authority.confirmation_for(live_session_scope(app.world())))
        .unwrap_or(ambition_platformer2d::runtime::RollbackConfirmationState::Unavailable)
}

fn ggrs_session_is_live(app: &App) -> bool {
    ambition_platformer2d::rollback::session_is_active(app.world())
}

/// The active room id of the one exact live session world.
fn active_room(app: &App) -> Option<String> {
    session_world_component::<RoomSet>(app.world())
        .map(|rooms| rooms.active_spec().id.as_str().to_owned())
}

/// Stand the controlled body inside an overlap-fire loading zone of the live
/// room and report the room it should leave.
///
/// ⭐ AN OVERLAP-FIRE ZONE, not a door: `EdgeExit` and `Walk` fire on overlap,
/// so this exercises the transition authority without also requiring the host
/// input stack. `door_entry` and `door_with_the_touch_overlay` own the press.
fn stand_in_an_overlap_transition(app: &mut App) -> String {
    use ambition_platformer2d::world::rooms::LoadingZoneActivation;

    let before = active_room(app).expect("a live session room");
    let zone = {
        let rooms = session_world_component::<RoomSet>(app.world()).expect("a live session room");
        rooms
            .active_loading_zones()
            .iter()
            .find(|zone| {
                matches!(
                    zone.activation,
                    LoadingZoneActivation::EdgeExit | LoadingZoneActivation::Walk
                )
            })
            .cloned()
            .unwrap_or_else(|| {
                panic!("room `{before}` authors no overlap-fire loading zone to walk into")
            })
    };
    let world = app.world_mut();
    let mut bodies = world.query_filtered::<&mut ambition_platformer2d::platformer::body::BodyKinematics, With<PrimaryPlayer>>();
    let mut kin = bodies
        .single_mut(world)
        .expect("the live session seats exactly one primary player");
    kin.pos = ambition_platformer2d::engine_core::AabbExt::center(zone.aabb);
    kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
    before
}

/// Step until the active room changes, or give up after `frames`.
fn settle_until_the_room_changes(app: &mut App, before: &str, frames: u32) -> Option<String> {
    for _ in 0..frames {
        app.update();
        match active_room(app) {
            Some(room) if room != before => return Some(room),
            _ => {}
        }
    }
    None
}

/// Which admissible order the local-session owner and the shell's session
/// bridge run in.
///
/// ⛔⛔ NOTHING IN THE SHIPPED SCHEDULE ORDERS THEM. `LocalSessionSet::Maintain`
/// is constrained only against `InputSet::Collect`; `GameplaySessionSet::Bridge`
/// only against `AmbitionGameShellSet::Pending`. Both live in `Update`, so both
/// orders below are things this app may legitimately do — and they are not
/// equally survivable: with the owner running FIRST, retirement removes the
/// canonical root while the GGRS session is still installed, and the contract
/// check on the next `PreUpdate` reads deliberate teardown as corruption.
///
/// ⭐ The fix orders them (`SessionScopeSet::RetireAuthority`), but ordering is
/// hygiene. These arms exist to prove the OWNERSHIP holds when the ordering does
/// not, which is the only version of the guarantee worth having.
#[derive(Clone, Copy, Debug)]
enum RetirementOrder {
    /// What the shipped schedule happens to produce today.
    AsScheduled,
    /// The other admissible order — a scheduling regression, simulated.
    LocalSessionOwnerFirst,
}

/// ⭐⭐ THE ACCEPTANCE WALK: Smash → title → Ambition, and the doors still work.
///
/// The user-visible symptom was not a status enum. Ambition's player could move
/// and could not change rooms, because room-transition commit refuses every
/// transition while confirmation authority is unhealthy — and the unhealthy
/// value belonged to the SMASH session that had already ended.
///
/// ⛔ SO THE ASSERTION IS A ROOM CHANGE, not `Healthy`. A health flag can be
/// cleared by any number of wrong fixes; the transition is the authority that
/// actually failed.
#[test]
fn a_smash_session_does_not_take_ambitions_doors_with_it() {
    smash_then_ambition(RetirementOrder::AsScheduled);
}

/// The same walk with the scheduling regressed back to the order that broke it.
#[test]
fn a_smash_session_does_not_take_ambitions_doors_even_when_retirement_is_misordered() {
    smash_then_ambition(RetirementOrder::LocalSessionOwnerFirst);
}

fn smash_then_ambition(order: RetirementOrder) {
    let mut app =
        shell_host_app_hosted_by(ambition_platformer2d::runtime::SimulationHost::Rollback);
    if let RetirementOrder::LocalSessionOwnerFirst = order {
        app.configure_sets(
            Update,
            ambition_platformer2d::rollback::local_session::LocalSessionSet::Maintain
                .before(ambition_platformer2d::game_shell::GameplaySessionSet::Bridge),
        );
    }
    settle(&mut app);
    assert_home(&mut app, "boot");

    // ── Session A: Smash ───────────────────────────────────────────────
    // The launcher's Smash row opens character select (a question, not a game),
    // so the stage route is addressed directly. What matters here is only that
    // a DIFFERENT gameplay session ran first and installed a rollback timeline.
    app.world_mut().write_message(ShellCommand::GoTo(
        ambition_platformer2d::game_shell::ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        ),
    ));
    settle(&mut app);
    let scope_a = live_scope(&app).expect("the Smash session is live");
    assert!(
        ggrs_session_is_live(&app),
        "the rollback host installs a GGRS session for the Smash match, or this \
         walk is measuring the render-frame host again ({order:?})"
    );

    // ── Back to the title ──────────────────────────────────────────────
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_home(&mut app, "after Smash");

    // ── Session B: Ambition ────────────────────────────────────────────
    launch_labeled(&mut app, "Ambition");
    settle(&mut app);
    let scope_b = live_scope(&app).expect("the Ambition session is live");
    assert_ne!(scope_a, scope_b, "session scopes are never reused");

    assert_eq!(
        confirmation(&app),
        ambition_platformer2d::runtime::RollbackConfirmationState::Healthy,
        "{order:?}: session B's confirmation authority is its own, and it is \
         healthy — a value inherited from the retired Smash scope is not B's to read"
    );

    // ── The authority that actually failed ─────────────────────────────
    let before = stand_in_an_overlap_transition(&mut app);
    let after = settle_until_the_room_changes(&mut app, &before, 240).unwrap_or_else(|| {
        panic!(
            "{order:?}: the Ambition body stood in an overlap-fire loading zone of \
             `{before}` for 240 frames and the room never changed. Confirmation \
             authority is {:?}: a transition cannot commit while it is unhealthy, \
             and an unhealthy value that outlived session {scope_a:?} is not \
             session {scope_b:?}'s to inherit",
            confirmation(&app)
        )
    });
    assert_ne!(
        before, after,
        "the transition committed and the room changed"
    );
}
