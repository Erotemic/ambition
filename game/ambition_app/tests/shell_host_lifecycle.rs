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
    shell_host_app_started_in(host, None)
}

/// The same host, with Ambition's start room PINNED.
///
/// ⛔ A ROOM IS A POPULATION, WHICH IS WHY A WITNESS NEEDS THIS. Several
/// rollback registrations only ever have carriers in a room that authors them —
/// `portal.placed` needs a fixed portal, `feature.hazard` a hazard block — so an
/// arm that walks only the authored start room reports agreement about rows that
/// were never present. Measured 2026-09-17: six of S7's twelve sharp rows carried
/// state from the authored start room, and the six silent ones were not evidence
/// of anything.
///
/// ⛔ `StartRoomMustResolve` travels with the override, always. A programmatic
/// override that does not resolve falls back to the authored start room and says
/// so only in a log line, so a renamed room would leave this comparing two hosts
/// somewhere neither of them asked for — which is the failure `capture_scene`
/// already paid for once.
fn shell_host_app_started_in(
    host: ambition_platformer2d::runtime::SimulationHost,
    room: Option<&str>,
) -> App {
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
    // Before `init_sandbox_resources`, which is what consumes both.
    if let Some(room) = room {
        app.insert_resource(ambition_app::app::StartRoomOverride(room.to_string()));
        app.insert_resource(ambition_app::app::StartRoomMustResolve);
    }
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

/// Every canonical `SimId` the live world holds, sorted.
///
/// ⛔⛤ THE CENSUS IS OVER VALUES, NOT TYPES. `id_peer_audit` censuses registered
/// TYPE NAMES, and `SimId` is a type that is SUPPOSED to be canonical — both
/// provenance defects found so far (the match-spawn tick, in a constructor's
/// argument; the session root, in a singleton's key) were invisible to a type
/// census because the TYPE was right and the STRING it held was not. This reads
/// the strings out of a fully built world.
fn canonical_identities(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    let mut query = world.query::<&ambition_platformer2d::observation::SimId>();
    let mut rows: Vec<String> = query.iter(world).map(|id| id.as_str().to_owned()).collect();
    rows.sort();
    rows
}

/// The host's local route history, as the tokens that actually differ between
/// two hosts: the session scope counter and the content epoch.
fn local_lifecycle_tokens(app: &mut App) -> String {
    let scope = format!("{:?}", live_scope(app));
    let world = app.world_mut();
    let mut query = world.query::<&ambition_platformer2d::runtime::PreparedContentIdentity>();
    let epochs: Vec<String> = query
        .iter(world)
        .map(|identity| format!("{:?}", identity.epoch))
        .collect();
    format!("{scope} {epochs:?}")
}

/// The pair every differing-history arm needs: the same shipped route reached by
/// two different local route histories.
///
/// ⭐ ONE ROAD, TWO QUESTIONS. The identity arm asks whether the two worlds NAME
/// their entities the same; the value arm asks whether they COMPUTE the same
/// numbers. Both need the same construction and the same control, and building it
/// twice is how the control drifts out of one of them.
///
/// Returns `(fresh, veteran)`: Ambition launched first, and Ambition launched
/// third after Sanic and Mary-O have each come and gone.
fn two_hosts_with_different_route_histories() -> (App, App) {
    let mut fresh = shell_host_app();
    settle(&mut fresh);
    launch_labeled(&mut fresh, "Ambition");
    settle(&mut fresh);

    let mut veteran = shell_host_app();
    settle(&mut veteran);
    for provider in ["Sanic", "Mary-O"] {
        launch_labeled(&mut veteran, provider);
        settle(&mut veteran);
        veteran.world_mut().write_message(ShellCommand::QuitToHome);
        settle(&mut veteran);
    }
    launch_labeled(&mut veteran, "Ambition");
    settle(&mut veteran);
    (fresh, veteran)
}

/// ⭐⭐ **THE SAME ROUTE, REACHED BY TWO DIFFERENT LOCAL HISTORIES, NAMES EVERY
/// SIMULATED ENTITY IDENTICALLY — measured over the whole world, not one id.**
///
/// ⛔⛤ **AND THE ARM THAT WAS CITED FOR THIS CLASS HELD THE ROAD PRODUCTION DOES
/// NOT TAKE — MEASURED 2026-09-16 BY POISONING EACH MINT SEPARATELY.** The
/// session root had two mints, and the arm that closed this class called the one
/// with NO production caller: A10's candidate road builds its own root and hands
/// it to `adopt_world`, which `PlatformerSessionBuilder::build_candidate` says in
/// its own doc it must. Re-keying the dead primitive's mint on the activation
/// count left this walk's census untouched; re-keying the CANDIDATE mint on the
/// scope counter turns `session:root` into `session:root-0` against
/// `session:root-2` — and the whole pre-existing app suite stayed green at 705
/// passed / 0 failed under exactly that poison. The dead primitive and its arm
/// are both deleted now, so this is the only arm the class has.
///
/// ⇒ This arm holds all 22 identities against the shipped composition: launch
/// Ambition first, and launch Ambition third after two other providers have come
/// and gone. The local tokens genuinely differ — scope `0` vs `2`, epoch `1` vs
/// `3`, asserted below so the comparison is controlled rather than two readings
/// of the same input — and the census does not move.
///
/// ⚠ **AN EQUALITY ASSERTION IS SATISFIED BY EVERY PROJECTION THAT THROWS
/// INFORMATION AWAY, THE CONSTANT INCLUDED**, so the third arm is a
/// DISAGREEMENT: a Sanic session's census is a different 43 rows, sharing only
/// the three identities that are supposed to be shared — the session root, the
/// player slot, and the process-wide encounter authority the content crate
/// installs at App build. A projection that collapsed to a constant would fail
/// there, and a projection that lost the two rows whose provenance defects were
/// actually found would fail the floor.
#[test]
fn two_local_histories_name_every_simulated_entity_identically() {
    let (mut first_launch, mut third_launch) = two_hosts_with_different_route_histories();

    // The comparison is only worth anything if the INPUT moved.
    let fresh_tokens = local_lifecycle_tokens(&mut first_launch);
    let veteran_tokens = local_lifecycle_tokens(&mut third_launch);
    assert_ne!(
        fresh_tokens, veteran_tokens,
        "the two hosts must reach Ambition with different local lifecycle state, \
         or this test compares one input with itself"
    );

    let fresh = canonical_identities(&mut first_launch);
    let veteran = canonical_identities(&mut third_launch);
    assert_eq!(
        fresh, veteran,
        "a canonical identity may not record how many routes the host visited \
         first ({fresh_tokens} vs {veteran_tokens})"
    );

    // The floor: the two rows whose provenance defects were actually found, in a
    // census large enough to be the world rather than a fragment of it.
    assert!(
        fresh.len() >= 20,
        "the canonical census collapsed to {} rows, so it is no longer reading a \
         built world: {fresh:?}",
        fresh.len()
    );
    for required in ["session:root", "slot:0"] {
        assert!(
            fresh.contains(&required.to_owned()),
            "{required} is absent from the census, so this test cannot see the \
             class of defect it exists to catch: {fresh:?}"
        );
    }

    // The disagreement half: a different provider is named differently.
    let mut other_provider = shell_host_app();
    settle(&mut other_provider);
    launch_labeled(&mut other_provider, "Sanic");
    settle(&mut other_provider);
    let sanic = canonical_identities(&mut other_provider);
    assert_ne!(
        fresh, sanic,
        "two different worlds project to the same census, so the projection is \
         throwing away everything this test claims to compare"
    );
    let shared: Vec<&String> = fresh.iter().filter(|id| sanic.contains(id)).collect();
    assert_eq!(
        shared,
        vec!["encounter:symmetry_attunement", "session:root", "slot:0"],
        "exactly the session root, the player slot, and the App-build encounter \
         authority are shared between two providers; anything else shared is one \
         provider's content leaking into another's identity space"
    );
}

type BodyAnimFacts = ambition_platformer2d::characters::actor::BodyAnimFacts;
type GroundItem = ambition_platformer2d::item::GroundItem;

/// Every animation-fact carrier, BITWISE, keyed by canonical identity.
///
/// ⛔ `SimId`-keyed rather than entity-keyed, because two hosts that built the
/// same world in a different order hold different `Entity` values by
/// construction — the identity arm above is what makes this key trustworthy.
/// ⛔ `to_bits`, not `==`: this is a peer-agreement question, and two floats that
/// compare equal under a tolerance are still two different checksums.
fn animation_facts(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    let mut rows: Vec<String> = world
        .query::<(&ambition_platformer2d::observation::SimId, &BodyAnimFacts)>()
        .iter(world)
        .map(|(id, facts)| {
            format!(
                "{}|{:08x}|{:08x}|{}|{:08x}|{}|{:08x}|{}|{:08x}|{:08x}",
                id.as_str(),
                facts.slash_anim_timer.to_bits(),
                facts.land_anim_timer.to_bits(),
                u8::from(facts.land_anim_hard),
                facts.dash_startup_timer.to_bits(),
                u8::from(facts.anim_prev_dashing),
                facts.shoot_anim_timer.to_bits(),
                u8::from(facts.aim_anim_active),
                facts.wall_jump_anim_timer.to_bits(),
                facts.interact_anim_timer.to_bits(),
            )
        })
        .collect();
    rows.sort();
    rows
}

/// Every ground item's `pos`/`vel`/`half_extent`, bitwise, keyed the same way.
fn ground_item_values(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    let mut rows: Vec<String> = world
        .query::<(&ambition_platformer2d::observation::SimId, &GroundItem)>()
        .iter(world)
        .map(|(id, item)| {
            format!(
                "{}|{:08x},{:08x}|{:08x},{:08x}|{:08x},{:08x}",
                id.as_str(),
                item.pos.x.to_bits(),
                item.pos.y.to_bits(),
                item.vel.x.to_bits(),
                item.vel.y.to_bits(),
                item.half_extent.x.to_bits(),
                item.half_extent.y.to_bits(),
            )
        })
        .collect();
    rows.sort();
    rows
}

/// ⭐⭐ **S7's ACCEPTANCE TEST HAS A SHAPE THAT CAN BE ASKED TODAY, AND
/// `simulation-authority-and-determinism.md` SAID IT COULD NOT.**
///
/// That page ranks 25 rows that sit OUTSIDE the session checksum, are read by an
/// unfiltered per-tick query, and carry a float. Two of them were measured clean
/// under local resimulation, and the page then records the limit honestly: a
/// value outside the peer checksum can be perfectly reproducible under one
/// machine rewinding itself and still differ between two peers, *because nothing
/// compares it between peers at all*. Its table answers *"do two peers agree
/// about this value?"* with **"unmeasured, and unmeasurABLE here"**.
///
/// ⇒ The second half of that is too strong, and this arm is the counter-example.
/// A P2P SESSION cannot be built — `SyncTestSession` is the only one this
/// workspace constructs — but **two Apps with different local histories can**, and
/// comparing what they compute is a different KIND of evidence from replaying one
/// App against its own past. It does not replace N2: no transport, no input
/// exchange, no interleaving. It answers the state half.
///
/// ⛔⛤ **AND THE FIRST VERSION OF THIS MEASUREMENT WAS VACUOUS, WHICH IS WHY THE
/// CONTROL IS THE FIRST ASSERTION.** Two `Platformer2dSimHarness` instances built
/// in one process were compared and agreed — and then the tokens were printed:
/// both read `SessionScopeId(0)` at tick 1. A fresh App is a fresh counter, so a
/// bare sim harness cannot CARRY a differing history; that comparison was one
/// input against itself. The differing history has to live inside ONE App that
/// has been somewhere first, which is what the shell host provides.
///
/// ⚠ **AND THE TWO HALVES BELOW ARE DIFFERENT CLAIMS, LABELLED SO NOBODY READS
/// THE WEAKER ONE AS THE STRONGER.** The ground items are AT REST in this route
/// — measured, not assumed: their census does not move across the window — so
/// that half witnesses CONSTRUCTION agreement. The animation facts move, so that
/// half witnesses PER-TICK agreement, and its motion is floored rather than
/// hoped for.
#[test]
fn two_local_histories_compute_the_same_mechanical_values() {
    const STEPS: usize = 120;

    let (mut fresh, mut veteran) = two_hosts_with_different_route_histories();

    // The control, first: without it every assertion below is one reading
    // compared with itself.
    let fresh_tokens = local_lifecycle_tokens(&mut fresh);
    let veteran_tokens = local_lifecycle_tokens(&mut veteran);
    assert_ne!(
        fresh_tokens, veteran_tokens,
        "the two hosts reached Ambition with the same local lifecycle state, so \
         nothing below is about a host's history reaching its mechanics"
    );

    // ── The construction half: values a route builds and does not then move.
    let fresh_items = ground_item_values(&mut fresh);
    let veteran_items = ground_item_values(&mut veteran);
    assert!(
        fresh_items.len() >= 14,
        "the ground-item census collapsed to {} rows, so it is reading a fragment \
         of the world rather than the world",
        fresh_items.len()
    );
    assert_eq!(
        fresh_items, veteran_items,
        "two hosts built the same route's ground items at different positions, \
         sizes or velocities, decided by what each visited first \
         ({fresh_tokens} vs {veteran_tokens})"
    );

    // ── The per-tick half.
    let mut fresh_trace = Vec::with_capacity(STEPS);
    let mut veteran_trace = Vec::with_capacity(STEPS);
    for _ in 0..STEPS {
        fresh.update();
        veteran.update();
        fresh_trace.push(animation_facts(&mut fresh));
        veteran_trace.push(animation_facts(&mut veteran));
    }

    assert!(
        fresh_trace.iter().all(|rows| rows.len() >= 2),
        "the animation-fact census lost a carrier mid-window, so some steps \
         compare fewer bodies than others"
    );

    // ⭐ THE MOTION FLOOR, AND IT IS THE ASSERTION THIS FILE'S SUBJECT MOST NEEDS.
    // `simulation-authority-and-determinism.md` records the same trap one level
    // down: a window chosen from a field name produced 36 clean comparisons of
    // values that never left `0.000`, and a verdict about rest reads exactly like
    // a verdict about motion. A census that takes ONE value across the whole
    // window agrees with itself for free.
    let distinct: std::collections::BTreeSet<&Vec<String>> = fresh_trace.iter().collect();
    assert!(
        distinct.len() > 1,
        "the animation facts took ONE value across all {STEPS} steps, so the \
         agreement below is about a world at rest"
    );

    // ⛔ THE FIRST DIVERGING STEP, NOT THE WHOLE TRACE. A bare `assert_eq!` over
    // 120 steps prints both traces in full — hundreds of lines in which the one
    // row that moved is invisible. A comparison this wide has to report where it
    // broke, or its failure is unreadable and gets re-run rather than read.
    let first_divergence = fresh_trace
        .iter()
        .zip(veteran_trace.iter())
        .enumerate()
        .find(|(_, (fresh_rows, veteran_rows))| fresh_rows != veteran_rows);
    assert!(
        first_divergence.is_none(),
        "two hosts computed different animation facts for the same route from \
         the same starting world, decided by what each visited first \
         ({fresh_tokens} vs {veteran_tokens}). That is a value outside the peer \
         checksum diverging on local history — S7's question answered as a \
         defect.\n  step {}\n  fresh   {:?}\n  veteran {:?}",
        first_divergence.map(|(step, _)| step).unwrap_or_default(),
        first_divergence.map(|(_, (fresh_rows, _))| fresh_rows),
        first_divergence.map(|(_, (_, veteran_rows))| veteran_rows),
    );
}

/// ⭐⭐ **THE WHOLE PEER-VISIBLE SURFACE, ACROSS TWO LOCAL HISTORIES — THE
/// HOSTILE VERSION OF THE ARM ABOVE, AND IT FOUND SOMETHING.**
///
/// The arm above compares two hand-named component types. This one asks the
/// rollback registry which registrations FEED THE PEER CHECKSUM — **146 of the
/// baseline's 491 rows, MEASURED 2026-09-17 at `88274c3ba`** — censuses exactly
/// those on both hosts, and compares. ⚠ Neither figure is asserted here and
/// neither should be read as current: the arm's floor is `keep.len() >= 140`,
/// and the way to re-derive them is to raise that floor until it fails, plus
/// `tail -n +2 game/ambition_app/tests/rollback_schema_baseline.txt | wc -l`.
/// This comment said `145 of 488` for a day after `Q142` added three rows.
/// Requested by the
/// GPT architecture review of 2026-09-16 as the last check before calling
/// ID-PEER complete for its current scope. It was not clean.
///
/// ⛔⛤ **THE FIRST VERSION CENSUSED ALL 364 PROBES AND REPORTED SEVEN DIFFERING
/// ROWS, WHICH IS NOT THE QUESTION.** `probes.rs` says so in its own words —
/// *"what makes an entry dangerous is that it ALSO feeds the peer checksum,
/// which the registry knows and this does not; the JOIN is the finding"* — and
/// records a previous instance of exactly this over-reporting. Seven was also
/// what that earlier broken instrument produced. ⇒ The join against
/// `RollbackEntryKind::feeds_peer_checksum` is what makes the number mean
/// anything.
///
/// ⛔⛤ **AND THE PROBE IS STILL NOT THE PEER PROJECTION FOR THREE REGISTRATION
/// ARMS.** `rollback_component_canonical_checksum`,
/// `rollback_resource_canonical_checksum` and
/// `rollback_resource_optional_canonical_checksum` each hand a
/// `fn(&T) -> u64` to GGRS and then register the probe with `census_state` —
/// the whole canonical state, including the local terms the projection exists to
/// drop. `census_with`'s own doc claims the opposite: *"the registration arms
/// that take `checksum: fn(&T) -> u64` hand the same function to GGRS and to
/// this, so the probe measures byte-for-byte what the session's aggregate
/// measures"*. Measured false for three of the four such arms.
///
/// ⇒ **`TransactionId` is the proof and the warning.** Its census differs
/// between the two hosts; its ACTUAL projection, folded by hand, is
/// `(18, 5177721695145214374)` on both. ID-PEER's closure holds and the probe
/// was over-reporting. So a row in `EXPECTED_TO_DIFFER` below is only a finding
/// once its real projection has been read.
///
/// ⚠ **WHAT THIS IS NOT.** No transport, no input exchange, no interleaving, no
/// timeline rebase. It answers the state half of ID-PEER's acceptance test and
/// does not retire netcode's `N2`.
/// ⭐⭐ **THE OTHER HALF OF THE SURFACE: ROWS NO PEER CHECKSUM COMPARES.**
///
/// `the_peer_visible_surface_does_not_record_which_route_the_host_visited_first`
/// joins the probe census against the registrations that feed the peer checksum
/// (146 of 491 when last measured; the arm's own floor is the live authority). This arm asks the complement question S7 poses in
/// [`simulation-authority-and-determinism.md`]: of the rows OUTSIDE that
/// checksum, twenty-five are read by an unfiltered per-tick query AND carry a
/// float-bearing field, and **twelve of those are mutably borrowed in
/// production**. A value nothing compares is reproducible locally and divergent
/// across peers at the same time, and only the second half is invisible from
/// inside one App.
///
/// ⛔ **THIS IS NOT N2 AND DOES NOT RETIRE IT.** Two Apps with different local
/// histories is not two peers: no transport, no input exchange, no interleaving,
/// no rebase. What it CAN decide is whether a row's value depends on where this
/// host has been — which is the failure mode ID-PEER exists for, and the one a
/// type census cannot see.
///
/// ⚠ **AND A DIFFERENCE HERE IS A FINDING, NOT AUTOMATICALLY A DEFECT.** These
/// rows are outside the checksum for reasons; `EXPECTED_TO_DIFFER` names each
/// one that legitimately moves, so a NEW divergence cannot hide among them.
#[test]
fn two_local_histories_agree_about_the_sharp_unchecksummed_rows() {
    use ambition_platformer2d::rollback::{RollbackChecksumProbes, RollbackRegistry};

    /// The twelve rows S7 ranks sharpest: outside the peer checksum, read by an
    /// unfiltered per-tick query, float-bearing, AND mutably borrowed in
    /// production. Row names, as the registry spells them.
    const SHARP_ROWS: &[&str] = &[
        "item.ground_item",
        "actor.animation_facts",
        "portal.placed",
        "boss.death_animation",
        "actor.render_size",
        "feature.hazard",
        "gravity.flip_switch",
        "player.blink_camera_state",
        "portal.emission",
        "portal.gun_pickup",
        "portal.shot",
        "entity.transform",
    ];

    /// Rows measured to differ, each with the reason. ⛔ A row here is a reading,
    /// not a waiver.
    const EXPECTED_TO_DIFFER: &[(&str, &str)] = &[];

    fn build(veteran: bool, room: Option<&str>) -> App {
        let mut app = shell_host_app_started_in(
            ambition_platformer2d::runtime::SimulationHost::Rollback,
            room,
        );
        settle(&mut app);
        if veteran {
            for provider in ["Sanic", "Mary-O"] {
                launch_labeled(&mut app, provider);
                settle(&mut app);
                app.world_mut().write_message(ShellCommand::QuitToHome);
                settle(&mut app);
            }
        }
        launch_labeled(&mut app, "Ambition");
        settle(&mut app);
        app
    }

    /// `{type name: row name}` for the sharp rows this App actually registers.
    fn sharp_types(app: &App) -> std::collections::BTreeMap<String, String> {
        let registry = app
            .world()
            .get_resource::<RollbackRegistry>()
            .expect("the rollback host installs a registry");
        let mut out = std::collections::BTreeMap::new();
        for descriptor in registry.descriptors() {
            if SHARP_ROWS.contains(&descriptor.name.as_str()) {
                // ⛔ THE PREMISE OF THE JOIN, ASSERTED RATHER THAN ASSUMED. A row
                // that started feeding the peer checksum is covered by the other
                // arm and must leave this one, or both arms drift into reading
                // the same thing while claiming to split the surface.
                assert!(
                    !descriptor.kind.feeds_peer_checksum(),
                    "`{}` feeds the peer checksum now, so it belongs to the \
                     peer-visible arm rather than to this one",
                    descriptor.name
                );
                out.insert(descriptor.type_name.clone(), descriptor.name.clone());
            }
        }
        out
    }

    fn census(
        app: &mut App,
        keep: &std::collections::BTreeMap<String, String>,
    ) -> std::collections::BTreeMap<String, (usize, u64)> {
        let probes = app
            .world()
            .get_resource::<RollbackChecksumProbes>()
            .cloned()
            .expect("the rollback host registers probes");
        probes
            .census_all(app.world_mut())
            .into_iter()
            .filter_map(|(name, reading)| {
                keep.get(name)
                    .map(|row| (row.clone(), (reading.count, reading.xor)))
            })
            .collect()
    }

    /// One walk: where it starts, whether anything is pressed, and what it is
    /// here to carry.
    struct Walk {
        room: Option<&'static str>,
        driven: bool,
        carries: &'static str,
    }

    /// One step of a walk.
    ///
    /// ⛔ **BOTH HOSTS GET THE SAME SCRIPT, WHICH IS WHAT KEEPS A DIVERGENCE A
    /// FINDING.** The script is a pure function of the step index, so the two
    /// hosts differ in exactly one thing — which routes each visited before this
    /// one — and any disagreement below is about that.
    ///
    /// ⚠ `step % 10` is a PRESS AND A RELEASE, not a held button: the pickup and
    /// the fire both read an edge, and a permanently-held attack is one press
    /// followed by nothing.
    fn advance(app: &mut App, step: usize, driven: bool) {
        if driven {
            ambition_platformer2d::sim::drive_control_frame(
                app.world_mut(),
                ambition_platformer2d::engine_core::ControlFrame {
                    axis_x: 1.0,
                    attack_pressed: step % 10 == 0,
                    ..Default::default()
                },
            );
        }
        app.update();
    }

    /// ⛔⛤ **THE ROOMS, BECAUSE A ROOM IS THE POPULATION.** This arm walked only
    /// the authored start room until 2026-09-17, and measured six of the twelve
    /// sharp rows carrying any state at all: the other six agreed the way two
    /// empty sets agree. A row that no room in the walk AUTHORS is not evidence
    /// of a host's history failing to reach it. Each entry names what it is here
    /// to carry, measured over `game/ambition_content/assets/worlds/*.ldtk`.
    const ROOMS: &[Walk] = &[
        Walk {
            room: None,
            driven: false,
            carries: "the authored start room -- what pressing launch reaches",
        },
        // ⛔⛤ **DRIVEN SINCE 2026-09-17, AND THAT IS WHAT CARRIES
        // `portal.emission`.** S7 read this row as wanting an AIMED script — fire
        // at a wall, then walk into the aperture. It wants neither: `portal_lab`
        // AUTHORS the aperture (`a_purple`, a ground-ground pair at x 254..346
        // with normal `up`, 174px to the player's right), so holding right walks
        // the body into it and the transit emits at tick 50.
        Walk {
            room: Some("portal_lab"),
            driven: true,
            carries: "fourteen authored `Portal` placements, and a ground-ground \
                      pair holding right walks into",
        },
        Walk {
            room: Some("basement_hazards"),
            driven: false,
            carries: "three authored `DamageVolume` placements",
        },
        // ⛔⛤ **THE WALK THAT PRESSES AND CARRIES A GUN**, and it is the only way
        // to reach a row a room cannot author AT ALL: a shot exists because
        // somebody fired. Measured 2026-09-17 in this room — the player starts at
        // x=94 and the pickup sits at x=180 with a 20px half-extent, so
        // holding right reaches it, and the gun is in hand on step 20.
        Walk {
            room: Some("portal_bridge"),
            driven: true,
            carries: "an authored `PortalGunSpawn` 86px to the player's right",
        },
        // ⛔⛤ **A BOSS DOES NOT HAVE TO DIE, WHICH IS WHY THIS WALK IS UNDRIVEN
        // AND WHY S7 SAID THE OPPOSITE.** `BossDeathAnimation::default()` is
        // inserted AT SPAWN (`actor_spawn/mod.rs:1141`), so a room that places a
        // `BossSpawn` carries the row. ⚠ **AND THE CARRIER IS CONSTANT** —
        // measured `(1, 0)` at every one of the 121 observation points, driven or
        // not, because `remaining_s` stays `0.0` until something kills the boss
        // and 120 driven steps of holding right do not. ⇒ What this compares is
        // the carrier's PRESENCE and identity across two histories, not a varying
        // float. A real boss death is still the stronger observation and still
        // the expensive fixture; what is wrong is calling the row unreachable.
        Walk {
            room: Some("basement_boss"),
            driven: false,
            carries: "an authored `BossSpawn`, whose spawn inserts the row",
        },
    ];

    let mut compared = std::collections::BTreeSet::new();
    let mut registered_sharp_rows = 0usize;
    for Walk {
        room,
        driven,
        carries,
    } in ROOMS
    {
        let room_label = room.unwrap_or("<authored>");
        eprintln!("[sharp-rows] room {room_label}: {carries}");
        let mut fresh = build(false, *room);
        let mut veteran = build(true, *room);

        // The control, first.
        let fresh_tokens = local_lifecycle_tokens(&mut fresh);
        let veteran_tokens = local_lifecycle_tokens(&mut veteran);
        assert_ne!(
            fresh_tokens, veteran_tokens,
            "the two hosts reached Ambition with the same local lifecycle state, so \
             nothing below is about a host's history reaching its peer state"
        );

        let keep = sharp_types(&fresh);
        registered_sharp_rows = keep.len();
        assert_eq!(
            keep,
            sharp_types(&veteran),
            "the two hosts register different sharp-row sets, so the comparison below \
             is between two different questions"
        );
        // ⛔⛤ **A RATCHET ON THE POPULATION, NOT A FLOOR UNDER IT — NAMED BY THE GPT
        // ARCHITECTURE REVIEW OF 2026-09-16.** The anti-vacuity check below asks
        // only that SOMETHING was compared, so eleven rows leaving the registry
        // while the twelfth agreed would read green: exactly the coverage erosion
        // this family of guards exists to stop. `SHARP_ROWS` is a named target
        // population, so the assertion is SET EQUALITY, which also names a spelling
        // or join mistake instead of quietly shrinking the census.
        // ⛔⛤ **AND THE SET EQUALITY BELOW CANNOT SEE THE LIST SHRINKING, WHICH A
        // POISON SHOWED RATHER THAN AN ARGUMENT.** Deleting `portal.shot` from
        // `SHARP_ROWS` left the arm GREEN: both sides of that comparison are derived
        // from the list, so editing the list moves them together. It catches the
        // registry losing a row and nothing else. ⇒ The population's SIZE is pinned
        // against the number S7 states, which is a constant a reviewer can check
        // without running anything.
        assert_eq!(
            SHARP_ROWS.len(),
            12,
            "S7 ranks TWELVE rows as sharp — outside the peer checksum, read every \
             tick, float-bearing and mutably written in production. This list has \
             {}. If S7's census genuinely moved, re-derive it THERE first and bring \
             the new number here with it; shrinking the list to make this arm green \
             is how a witness quietly stops witnessing",
            SHARP_ROWS.len()
        );
        let registered: std::collections::BTreeSet<&str> =
            keep.values().map(String::as_str).collect();
        let targeted: std::collections::BTreeSet<&str> = SHARP_ROWS.iter().copied().collect();
        assert_eq!(
            registered, targeted,
            "the registry no longer spells exactly S7's twelve sharp rows. A row that \
             LEFT is coverage this arm silently lost; a row that arrived is one S7 \
             has not classified. Re-derive the list against \
             `docs/planning/engine/simulation-authority-and-determinism.md` rather \
             than editing SHARP_ROWS to match the registry"
        );

        let expected: std::collections::BTreeMap<&str, &str> =
            EXPECTED_TO_DIFFER.iter().copied().collect();
        // ⛔⛤ **EVERY TICK, BECAUSE A LADDER CANNOT SEE A SHORT-LIVED ROW — AND
        // THAT, NOT A MISSING ROUTE, IS WHY `portal.emission` READ SILENT UNTIL
        // 2026-09-17.** The four-rung ladder sampled 0/1/30/120. `PortalEmission`
        // lives `PortalTuning::emission_time_s` = 0.18 s, about 11 ticks, and the
        // driven `portal_lab` walk carries it at ticks 50-60, 68-78 and 140-150:
        // every window falls in a gap, and the widest gap was 90 ticks. A sample
        // point chosen to land inside an 11-tick window would be a magic number
        // that any movement-tuning change silently retires, so the arm samples
        // them all. ⇒ The 121 observation points are a SUPERSET of the old four,
        // and the labels-vs-delta error the GPT review of 2026-09-16 named here
        // (advancing by the label each time, so 0/1/30/120 were really 0/1/31/151)
        // is now unspellable: there is one index and it is both.
        // ⛔ PER ROOM, because the union cannot say WHICH walk carries a row —
        // and that attribution is what turns "no route places it" into a road.
        let mut here = std::collections::BTreeSet::new();
        for step in 0..=120usize {
            if step > 0 {
                advance(&mut fresh, step - 1, *driven);
                advance(&mut veteran, step - 1, *driven);
            }
            let ours = census(&mut fresh, &keep);
            let theirs = census(&mut veteran, &keep);
            // ⛔ ONLY ROWS WITH CARRIERS SAY ANYTHING. A row at count 0 in both hosts
            // agrees the way two empty sets agree, so it is not counted as compared.
            here.extend(
                ours.iter()
                    .filter(|(_, (count, _))| *count > 0)
                    .map(|(row, _)| row.clone()),
            );
            let unexpected: Vec<String> = ours
                .iter()
                .filter(|(row, reading)| {
                    theirs.get(*row) != Some(*reading) && !expected.contains_key(row.as_str())
                })
                .map(|(row, reading)| {
                    format!("{row}  fresh={reading:?} veteran={:?}", theirs.get(row))
                })
                .collect();
            assert!(
                unexpected.is_empty(),
                "after {step} steps, a SHARP unchecksummed row differs between \
                 two hosts whose only difference is which routes they visited first \
                 ({fresh_tokens} vs {veteran_tokens}).\n  {}\n\n\
                 ⛔ Nothing two peers compare would notice this: these rows are \
                 outside the peer checksum by construction. See S7 in \
                 `docs/planning/engine/simulation-authority-and-determinism.md`.",
                unexpected.join("\n  ")
            );
        }
        eprintln!("[sharp-rows] room {room_label} carried {here:?}");
        compared.extend(here);
    }

    // ⛔⛤ **THE COVERAGE IS PINNED AS A SET, BECAUSE THE FLOOR BELOW CANNOT SEE
    // IT ERODE.** `!compared.is_empty()` answers only "did this compare
    // anything", so seven rows losing their carriers while the eighth agreed
    // would read green. A row that LEAVES this set is coverage this arm lost in
    // silence — usually a room rename, since the override falls back to the
    // authored start room when it cannot resolve. A row that JOINS it is a room
    // authoring something S7 has not counted. Either way re-derive the split
    // from the walk rather than editing this list to match it.
    //
    // ⇒ **THE ONE THAT IS NOT HERE IS NOT AN OVERSIGHT.**
    // `gravity.flip_switch` was measured 2026-09-17 to be placeable by no route
    // at all (`Q137`): its only mutable writer is registered once in the
    // workspace and that registration is inside a `#[cfg(test)]` module.
    //
    // ⛔⛤ **`boss.death_animation` WAS A SECOND, ON THE SAME MISTAKE AS THE
    // THIRD.** It was read as needing "a boss to die, the most expensive fixture
    // of the set". A boss does not have to die: the component is inserted at
    // SPAWN, so `basement_boss` carries it. Twice in one day a row read as
    // unreachable because the sentence described the EVENT the field is named
    // for rather than the code that inserts it — so read the insert site, not
    // the field name.
    //
    // ⛔⛤ **`portal.emission` WAS A THIRD, AND ITS STATED REASON WAS WRONG IN
    // BOTH HALVES.** It was read as needing an aimed script because the
    // gun-carrying walk never straddled what it shot. It needed no gun: a room
    // already in this list AUTHORS the aperture, and the row was invisible
    // because the observation ladder's narrowest gap was 29 ticks and the
    // component lives 11. A row can read "no route places it" when what is
    // missing is the LOOK, not the road.
    const CARRIED_BY_THE_WALK: &[&str] = &[
        "actor.animation_facts",
        "actor.render_size",
        "boss.death_animation",
        "entity.transform",
        "feature.hazard",
        "item.ground_item",
        "player.blink_camera_state",
        "portal.emission",
        "portal.gun_pickup",
        "portal.placed",
        "portal.shot",
    ];
    let carried: std::collections::BTreeSet<&str> =
        CARRIED_BY_THE_WALK.iter().copied().collect();
    assert_eq!(
        compared.iter().map(String::as_str).collect::<std::collections::BTreeSet<_>>(),
        carried,
        "the rooms in this walk no longer carry exactly the sharp rows they were measured to carry on 2026-09-17. {} of {} sharp rows are registered",
        registered_sharp_rows,
        SHARP_ROWS.len()
    );

    // ⛔ THE ANTI-VACUITY FLOOR, ON THE INTERSECTION RATHER THAN ITS OPERANDS. A
    // join keyed wrongly — row names on one side, type names on the other —
    // returns an empty map, and every assertion above then passes over nothing.
    assert!(
        !compared.is_empty(),
        "no sharp row had a single carrier in either host, so this arm compared \
         nothing: the registry spells {} of the {} sharp rows, and the probe \
         census matched {}",
        registered_sharp_rows,
        SHARP_ROWS.len(),
        compared.len()
    );
    eprintln!(
        "[sharp-rows] {} of {} sharp rows registered, {} carried state and were \
         compared: {:?}",
        registered_sharp_rows,
        SHARP_ROWS.len(),
        compared.len(),
        compared
    );
}

#[test]
fn the_peer_visible_surface_does_not_record_which_route_the_host_visited_first() {
    use ambition_platformer2d::rollback::{RollbackChecksumProbes, RollbackRegistry};

    /// Rows that differ for a reason already owned elsewhere. ⛔ A row here is a
    /// reading, not a waiver: each names the owner that has it, so a NEW
    /// divergence cannot hide among them.
    const EXPECTED_TO_DIFFER: &[(&str, &str)] = &[
        (
            "ambition_time::SimTick",
            "ID-PEER's open `canonical timeline` road — an absolute per-App step \
             count, registered `resource-canonical`. Blocked on Q128, and a \
             projection excluding it would exclude the TIMELINE",
        ),
        (
            "ambition_persistence::save::AmbitionGameSave",
            "Q129 — whether the save file is part of what two peers agree on is \
             a maintainer question, and removing it from the checksum to make \
             this arm green is explicitly the wrong repair",
        ),
        // ⛔⛤ **`TransactionId` WAS HERE AND IS NOT NOW, BECAUSE THE INSTRUMENT
        // WAS THE DEFECT.** The waiver read *"NOT A DIVERGENCE — the probe
        // measures `census_state` while the peer checksum is
        // `peer_stable_checksum`"*, with both hosts' hand-folded projections
        // recorded as `(18, 5177721695145214374)`. Three registration arms hand
        // `projection` to GGRS and recorded the probe with the WHOLE-STATE
        // census, so the census asked the restore question and this arm read the
        // answer as a peer divergence. Each of those arms declares a peer census
        // now (`ChecksumProbe::with_peer`), this arm calls
        // `census_all_as_peers_compare`, and the row needs no waiver.
    ];

    fn build(veteran: bool) -> App {
        let mut app =
            shell_host_app_hosted_by(ambition_platformer2d::runtime::SimulationHost::Rollback);
        settle(&mut app);
        if veteran {
            for provider in ["Sanic", "Mary-O"] {
                launch_labeled(&mut app, provider);
                settle(&mut app);
                app.world_mut().write_message(ShellCommand::QuitToHome);
                settle(&mut app);
            }
        }
        launch_labeled(&mut app, "Ambition");
        settle(&mut app);
        app
    }

    fn peer_types(app: &App) -> std::collections::BTreeSet<String> {
        app.world()
            .get_resource::<RollbackRegistry>()
            .expect("the rollback host installs a registry")
            .descriptors()
            .filter(|descriptor| descriptor.kind.feeds_peer_checksum())
            .map(|descriptor| descriptor.type_name.clone())
            .collect()
    }

    fn census(
        app: &mut App,
        keep: &std::collections::BTreeSet<String>,
    ) -> std::collections::BTreeMap<String, (usize, u64)> {
        let probes = app
            .world()
            .get_resource::<RollbackChecksumProbes>()
            .cloned()
            .expect("the rollback host registers probes");
        // ⛔ THE PEER QUESTION, ASKED THROUGH THE PEER PROJECTION. `census_all`
        // is the RESTORE question — whole state, local terms included, which is
        // what a rewind must put back — and asking it here over-reports for
        // every type whose peer checksum deliberately drops a host-local term.
        probes
            .census_all_as_peers_compare(app.world_mut())
            .into_iter()
            .filter(|(name, _)| keep.contains(*name))
            .map(|(name, reading)| (name.to_owned(), (reading.count, reading.xor)))
            .collect()
    }

    let mut fresh = build(false);
    let mut veteran = build(true);

    // The control, first.
    let fresh_tokens = local_lifecycle_tokens(&mut fresh);
    let veteran_tokens = local_lifecycle_tokens(&mut veteran);
    assert_ne!(
        fresh_tokens, veteran_tokens,
        "the two hosts reached Ambition with the same local lifecycle state, so \
         nothing below is about a host's history reaching its peer state"
    );

    let keep = peer_types(&fresh);
    assert_eq!(
        keep,
        peer_types(&veteran),
        "the two hosts register different peer-visible sets, so the comparison \
         below is between two different questions"
    );
    // ⛔⛤ **THE SPLIT THIS ARM DEPENDS ON, ASSERTED RATHER THAN ASSUMED.**
    // `census_all_as_peers_compare` falls back to whole state for a registration
    // that declares no peer projection, and that fallback is correct — but if the
    // three `*_canonical_checksum` arms stopped declaring one, this arm would
    // quietly return to asking the RESTORE question and `TransactionId` would
    // redden with a message about a divergence that is not one. So the premise is
    // checked here, where losing it is loud.
    let projected = fresh
        .world()
        .get_resource::<RollbackChecksumProbes>()
        .expect("the rollback host registers probes")
        .peer_projected_type_names();
    assert!(
        projected.contains("ambition_platformer2d_shared_tangle::construction::TransactionId"),
        "no registration declares a peer projection for `TransactionId`, so this \
         arm is comparing its WHOLE state — session stamp and app-local content \
         epoch included — and any difference it reports is the instrument. \
         {} type(s) declare one: {projected:?}",
        projected.len()
    );
    assert!(
        keep.len() >= 140,
        "only {} registrations feed the peer checksum, so this arm is reading a \
         fragment of the surface rather than the surface",
        keep.len()
    );

    let expected: std::collections::BTreeMap<&str, &str> =
        EXPECTED_TO_DIFFER.iter().copied().collect();
    let mut moved = 0usize;
    let mut previous = None;
    // ⛔ ABSOLUTE LABELS, DELTA ADVANCE. See the sharp-row arm above: this
    // advanced by the label each time and sampled 0/1/31/151 while reporting
    // 0/1/30/120.
    let mut advanced = 0usize;
    for step in [0usize, 1, 30, 120] {
        for _ in advanced..step {
            fresh.update();
            veteran.update();
        }
        advanced = step;
        let ours = census(&mut fresh, &keep);
        let theirs = census(&mut veteran, &keep);
        if previous.as_ref().is_some_and(|before| before != &ours) {
            moved += 1;
        }
        previous = Some(ours.clone());

        let differing: Vec<&String> = ours
            .iter()
            .filter(|(name, reading)| theirs.get(*name) != Some(*reading))
            .map(|(name, _)| name)
            .collect();
        let unexpected: Vec<String> = differing
            .iter()
            .filter(|name| !expected.contains_key(name.as_str()))
            .map(|name| {
                format!(
                    "{name}  fresh={:?} veteran={:?}",
                    ours.get(*name),
                    theirs.get(*name)
                )
            })
            .collect();
        assert!(
            unexpected.is_empty(),
            "after {step} more steps, peer-visible state differs between two \
             hosts whose only difference is which routes they visited first \
             ({fresh_tokens} vs {veteran_tokens}).\n  {}\n\n\
             Before treating one as a defect, read its registration kind: for the \
             three `*_canonical_checksum` arms the probe censuses `census_state`, \
             not the projection peers compare, and `TransactionId` is the \
             standing example of a row that differs here and agrees there.",
            unexpected.join("\n  ")
        );

        // ⛔ AND THE ROWS WE EXPECT TO DIFFER MUST STILL DIFFER. Each is a live
        // finding or an open question; one going quiet means it was closed and
        // nobody deleted its row, and the next reader would inherit a list that
        // no longer describes anything.
        if step == 0 {
            for (name, why) in EXPECTED_TO_DIFFER {
                assert!(
                    differing.iter().any(|found| found.as_str() == *name),
                    "`{name}` no longer differs between the two hosts. If that is \
                     a repair, delete its row here; if it is an accident, the \
                     reason it was listed is: {why}"
                );
            }
        }
    }

    // ⭐ THE MOTION FLOOR. A census that takes one value across the whole window
    // agrees with itself for free — the trap
    // `simulation-authority-and-determinism.md` records one section below its S7
    // table, where 36 clean comparisons were all of `0.000`.
    assert!(
        moved > 0,
        "the peer-visible census never moved across the window, so the agreement \
         above is about a world at rest"
    );
}

/// ⭐⭐ **THE CHECKSUM GGRS ACTUALLY COMPUTES IS THE SAME ON TWO HOSTS WITH
/// DIFFERENT SHELL HISTORIES — AND IT WAS NOT UNTIL 2026-09-17.**
///
/// `ComponentChecksumPlugin` does not hash a component's value alone:
///
/// ```text
/// for (rollback_id, component) in carriers {
///     hasher = fresh;
///     rollback_ordered.order(rollback_id).hash(&mut hasher);
///     projection(component).hash(&mut hasher);
///     result ^= hasher.finish();
/// }
/// ```
///
/// `RollbackOrdered` assigns each `RollbackId` an insertion-order index the first
/// time `Rollback` is added and keeps every index ever handed out — despawned
/// entities included. Nothing rebased it: `install_rebased_sync_test_session`
/// reset `RollbackFrameCount`, `ConfirmedFrameCount`, the input authority and
/// `Time<GgrsTime>`, and deliberately retained the snapshot infrastructure. So
/// the peer-compared checksum carried this App's whole rollback lineage.
///
/// ⛔⛤ **MEASURED BEFORE THE REPAIR, and the numbers are why this arm exists.**
/// Two hosts reaching the shipped Ambition route by different shell histories
/// held the SAME 22 canonical identities at orders `0..21` against `74..95` — a
/// constant offset of 74, the rollback entities Sanic and Mary-O registered and
/// retired — and **59 of 146 real `ChecksumPart`s disagreed**, `BodyHealth`,
/// `ActorPose`, `Brain` and `WornCharacter` among them. After the rebase: **2**,
/// and both are open roads somebody else owns (`SimTick`/`Q128`,
/// `AmbitionGameSave`/`Q129`), each named below with its reading.
///
/// ⚠ **A VALUE CENSUS CANNOT SEE ANY OF THIS, WHICH IS THE LESSON.**
/// `RollbackChecksumProbes` folds `count` and a wrapping sum of the per-value
/// projection and deliberately ignores which entity carried each value — so the
/// two-host value census agreed throughout. The probe measures GGRS's
/// PROJECTION; only this arm measures GGRS's CHECKSUM. Found by the GPT
/// architecture review of 2026-09-17, one day after the probe was corrected to
/// use the right projection.
///
/// ⇒ The repair is `rebase_rollback_carrier_order`, called where a session
/// declares frame zero. It keys on canonical `SimId` rather than `RollbackId`,
/// because `RollbackId` is the Bevy `Entity` that first received `Rollback` and
/// ordering by it would swap insertion history for allocation order — the same
/// defect one layer down.
///
/// ⚠ **AND THE INSTALLATION REFUSES WHILE A CANDIDATE WORLD IS IN FLIGHT** —
/// the INSTALLATION, not merely the rebase, and the distinction cost a review
/// round. The rebase's enumeration is an ordinary query and `InactiveCandidate`
/// is a disabling component, so a second review pass found the rebase blind to
/// candidates; the repair for THAT left
/// `install_rebased_sync_test_session` resetting the frame counters and
/// installing a session on the old order history anyway, which a third pass
/// named. The precondition is now the first thing the install road does and a
/// refusal mutates nothing. See `install_rebased_sync_test_session` and
/// `a_hidden_candidate_refuses_the_installation_and_mutates_nothing`; the
/// rebase's own doc carries why INCLUDING candidates would be worse than
/// refusing.
#[test]
fn two_local_histories_compute_the_same_ggrs_component_checksums() {
    use ambition_platformer2d::platformer::sim_id::SimId;
    use ambition_platformer2d::rollback::{ChecksumPart, RollbackId, RollbackOrdered};

    fn build(veteran: bool) -> App {
        let mut app =
            shell_host_app_hosted_by(ambition_platformer2d::runtime::SimulationHost::Rollback);
        settle(&mut app);
        if veteran {
            for provider in ["Sanic", "Mary-O"] {
                launch_labeled(&mut app, provider);
                settle(&mut app);
                app.world_mut().write_message(ShellCommand::QuitToHome);
                settle(&mut app);
            }
        }
        launch_labeled(&mut app, "Ambition");
        settle(&mut app);
        // ⛔ THE SHELL FIXTURE COMPOSES FOR ROLLBACK AND DECLARES NO
        // PARTICIPANTS, so `rollback::start` refuses it with
        // `NotComposedForRollback` and NO CHECKSUM IS EVER COMPUTED — measured:
        // zero `ChecksumPart` entities exist until a session runs. A composition
        // built through `PlatformerApp::rollback(n)` inserts this; the shell host
        // is driven by the launcher instead, so the declaration is made here.
        app.insert_resource(ambition_platformer2d::rollback::DeclaredParticipants(1));
        let session = ambition_platformer2d::rollback::start(
            &mut app,
            ambition_platformer2d::rollback::RollbackPlan::new(),
        )
        .expect("the shipped Ambition route must reach a running rollback session");
        assert_eq!(session.participants(), 1);
        for _ in 0..8 {
            app.update();
        }
        app
    }

    /// `{canonical identity: carrier order}` for every live rollback entity the
    /// sim can name, plus how many orders this App has ever handed out.
    fn ordering(app: &mut App) -> (usize, std::collections::BTreeMap<String, u64>) {
        let world = app.world_mut();
        let pairs: Vec<(String, RollbackId)> = world
            .query::<(&SimId, &RollbackId)>()
            .iter(world)
            .map(|(id, rb)| (id.as_str().to_string(), *rb))
            .collect();
        let ordered = world.resource::<RollbackOrdered>().clone();
        let total = ordered.len();
        (
            total,
            pairs
                .into_iter()
                .map(|(sim, rb)| (sim, ordered.order(rb)))
                .collect(),
        )
    }

    /// `{checksummed type: its ChecksumPart}`, labelled by the `ChecksumFlag<T>`
    /// the part entity carries. ⛔ A bare list of 146 numbers cannot say WHICH
    /// type disagreed, and that is the half a repair needs.
    fn parts(app: &mut App) -> std::collections::BTreeMap<String, u128> {
        let world = app.world_mut();
        let raw: Vec<(bevy::prelude::Entity, u128)> = world
            .query::<(bevy::prelude::Entity, &ChecksumPart)>()
            .iter(world)
            .map(|(e, p)| (e, p.0))
            .collect();
        raw.into_iter()
            .map(|(entity, value)| {
                let flag = world
                    .inspect_entity(entity)
                    .expect("a part entity this query just yielded is live")
                    .map(|c| c.name().to_string())
                    .find(|name| name.contains("ChecksumFlag"))
                    .unwrap_or_else(|| format!("<unflagged {entity:?}>"));
                (flag, value)
            })
            .collect()
    }

    let mut fresh = build(false);
    let mut veteran = build(true);

    let (fresh_total, fresh_order) = ordering(&mut fresh);
    let (veteran_total, veteran_order) = ordering(&mut veteran);

    // ⛔ THE PREMISE, ASSERTED FIRST. Without differing histories the comparison
    // below is between two identical Apps and proves nothing.
    //
    // ⛔⛤ **AND THE PREMISE MUST NOT BE THE DEFECT.** It was
    // `fresh_total != veteran_total` — the two Apps having handed out different
    // numbers of rollback orders — which is exactly what a carrier-order rebase
    // makes false. The first run against the repair failed on the PREMISE, not
    // on the subject: an arm whose control dies when the bug is fixed cannot
    // witness the fix. The local lifecycle tokens are the same premise every
    // sibling arm here uses and they survive it.
    let fresh_tokens = local_lifecycle_tokens(&mut fresh);
    let veteran_tokens = local_lifecycle_tokens(&mut veteran);
    assert_ne!(
        fresh_tokens, veteran_tokens,
        "the two hosts reached Ambition with the same local lifecycle state, so \
         nothing below is about a host's history reaching its peer state"
    );
    // ⚠ Reported, not asserted: after a rebase both Apps hand out one order per
    // live carrier, so this pair is EQUAL on a repaired timeline and unequal on a
    // broken one. It is the diagnosis a failure wants, not the control.
    println!(
        "[carrier order] orders handed out: fresh {fresh_total}, veteran {veteran_total}"
    );
    // ⭐ AND THE CONTROL: the canonical layer AGREES. A checksum difference under
    // differing identities would be an ordinary desync, not this finding.
    assert_eq!(
        fresh_order.keys().collect::<Vec<_>>(),
        veteran_order.keys().collect::<Vec<_>>(),
        "the two hosts name different entities, so a checksum difference below \
         would not be about carrier ORDER"
    );

    let fresh_parts = parts(&mut fresh);
    let veteran_parts = parts(&mut veteran);
    /// The two rows that still differ, each with the OPEN question that owns it.
    /// ⛔ A row here is a reading, not a waiver: both are already open roads with
    /// a ruling in front of them, and neither is about carrier order.
    const OWNED_ELSEWHERE: &[(&str, &str)] = &[
        (
            "ambition_time::SimTick",
            "`Q128` — the absolute tick is `resource-canonical`, so two Apps that              have run for different lengths of time disagree from the first              compared frame. A projection excluding it would exclude the TIMELINE",
        ),
        (
            "ambition_persistence::save::AmbitionGameSave",
            "`Q129` — whether a save FILE is part of what two peers agree on.              Thirteen of its nineteen writers are sim systems, so it is              simulation-adjacent in practice whatever it is in principle",
        ),
    ];

    let owned = |name: &str| {
        OWNED_ELSEWHERE
            .iter()
            .any(|(ty, _)| name.contains(ty))
    };
    let differing: Vec<&String> = fresh_parts
        .iter()
        .filter(|(name, value)| veteran_parts.get(*name) != Some(*value))
        .map(|(name, _)| name)
        .filter(|name| !owned(name))
        .collect();
    // ⛔⛤ **A STALE WAIVER IS A HOLE WITH A COMMENT OVER IT.** Each row above is
    // excused because it differs for a reason somebody else owns; the day one of
    // them stops differing, the excuse must go rather than sit here covering
    // whatever moves in next.
    for (ty, why) in OWNED_ELSEWHERE {
        let still_differs = fresh_parts
            .iter()
            .any(|(name, value)| name.contains(ty) && veteran_parts.get(name) != Some(value));
        assert!(
            still_differs,
            "`{ty}` no longer differs between the two hosts, so the reading that \
             excuses it here is spent: {why}. Delete the row rather than leaving \
             it to cover the next type that lands on this surface"
        );
    }
    let agreeing_nonzero = fresh_parts
        .iter()
        .filter(|(name, value)| **value != 0 && veteran_parts.get(*name) == Some(*value))
        .count();
    // ⛔ THE PUREST ROW MUST BE IN THE POPULATION. `EntityChecksumPlugin` folds
    // `hash(active carrier count, RollbackOrdered.len())` — upstream's own words
    // for the second term are *"the quantity of total spawned rollback
    // entities"* — so it is this defect with nothing else mixed in. If it ever
    // leaves the part set, this arm keeps passing over a smaller surface and the
    // clearest evidence goes with it.
    assert!(
        fresh_parts
            .keys()
            .any(|name| name.ends_with("ChecksumFlag<bevy_ecs::entity::Entity>")),
        "no entity checksum part in {} part(s); the population no longer contains \
         the row that hashes `RollbackOrdered.len()` directly",
        fresh_parts.len()
    );
    // ⛔ THE ANTI-VACUITY FLOOR. Two Apps that computed no checksums at all agree
    // on nothing and on everything; a run that reaches here with no non-zero
    // agreeing part measured an empty session, not a clean one.
    assert!(
        agreeing_nonzero > 0,
        "no checksum part is both non-zero and equal, so no comparison happened"
    );
    assert!(
        differing.is_empty(),
        "{} of {} GGRS checksum parts differ between two hosts whose canonical \
         identities and values are identical. The usual cause is the CARRIER \
         ORDER: `ComponentChecksumPlugin` hashes `RollbackOrdered.order(..)` \
         beside each value, and that index is App-lifetime unless a session \
         rebases it — {fresh_total} orders handed out here against \
         {veteran_total} there, which are equal on a rebased timeline. \
         First few: {:?}",
        differing.len(),
        fresh_parts.len(),
        &differing[..differing.len().min(6)]
    );
}

