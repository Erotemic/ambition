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
use bevy::ecs::system::RunSystemOnce;
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
use ambition::platformer::lifecycle::{
    session_world_component, session_world_entity, ActiveSessionScope, SessionRoot, SessionScopeId,
    SessionScopedEntity, SessionWorldMut,
};
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

fn session_roots(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&SessionRoot>();
    query.iter(app.world()).count()
}

fn live_room_set(app: &App) -> &RoomSet {
    session_world_component::<RoomSet>(app.world()).expect("one exact live session room set")
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
            .resource::<ambition::game_shell::PreparedSessionRegistry>()
            .is_empty(),
        "{context}: no prepared-session publication remains"
    );
    assert!(
        app.world()
            .resource::<ambition::load::LoadCoordinator>()
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
            Some(ambition::sfx::AudioContextOwner::Frontend(_))
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
            .allows(ambition::sfx::ids::UI_MENU_MOVE),
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
/// Ambition, Sanic, Mary-O, Pocket, Exit) and confirm it.
/// Launcher rows = registered experience entries + built-in host actions (the
/// Exit row, when the host shows it). Derived, never a literal.
fn launcher_row_count(app: &App) -> usize {
    use ambition::game_shell::{ShellLaunchCatalog, ShellLauncherPresentation};
    let experiences = app.world().resource::<ShellLaunchCatalog>().entries.len();
    let exit = app
        .world()
        .resource::<ShellLauncherPresentation>()
        .exit_label
        .is_some() as usize;
    experiences + exit
}

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
        .get::<ambition::runtime::PreparedContent>(world_entity)
        .unwrap_or_else(|| {
            panic!("{context}: the live root owns exact immutable prepared content")
        });
    let prepared_identity = app
        .world()
        .get::<ambition::runtime::PreparedContentIdentity>(world_entity)
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
            .resource::<ambition::runtime::rollback::RollbackRegistry>()
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
        vec!["Ambition", "Sanic", "Mary-O", "Pocket"],
        "launcher entries derive from the four registered providers \
         (Exit is the built-in fifth row)"
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
    let sanic_world_1 = app
        .world()
        .resource::<ActiveGameplaySession>()
        .active_world_entity()
        .expect("sanic #1 owns a canonical world entity");
    let sanic_content_1 = *app
        .world()
        .get::<ambition::runtime::PreparedContentIdentity>(sanic_world_1)
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

    // ── Pocket fourth-provider proof ───────────────────────────────────
    launch_entry(&mut app, 3);
    let scope = assert_in_game(
        &mut app,
        "pocket_gameplay",
        "pocket",
        Some(ambition_demo_pocket::POCKET_CHARACTER_ID),
        "pocket",
        "pocket",
    );
    fresh(scope, "pocket");
    assert_eq!(
        live_room_set(&app).active_spec().id.as_str(),
        "pocket_room",
        "pocket: provider-authored world is active"
    );
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_home(&mut app, "after pocket");

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
        .get::<ambition::runtime::PreparedContentIdentity>(ambition_world_entity)
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
                  mut geometry: SessionWorldMut<ambition::engine_core::RoomGeometry>,
                  mut active_room: SessionWorldMut<ambition::actors::rooms::ActiveRoomMetadata>| {
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
            .get::<ambition::runtime::PreparedContentIdentity>(live_entity)
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
    let sanic_world_2 = app
        .world()
        .resource::<ActiveGameplaySession>()
        .active_world_entity()
        .expect("sanic #2 owns a canonical world entity");
    let sanic_content_2 = *app
        .world()
        .get::<ambition::runtime::PreparedContentIdentity>(sanic_world_2)
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
    launch_entry(&mut app, 4);
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

/// Every live encounter authority, as `(encounter id, owning session scope)`.
fn encounter_authorities(app: &mut App) -> Vec<(String, Option<SessionScopeId>)> {
    let mut query = app.world_mut().query::<(
        &ambition::encounter::Encounter,
        Option<&SessionScopedEntity>,
    )>();
    let mut rows: Vec<_> = query
        .iter(app.world())
        .map(|(enc, owner)| (enc.id.clone(), owner.map(|owner| owner.0)))
        .collect();
    rows.sort();
    rows
}

/// **A GGRS session contract never survives session retirement.**
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
        let mut query = world.query::<&ambition::runtime::PreparedContentIdentity>();
        query
            .single(world)
            .copied()
            .expect("session A exposes prepared identity")
    };

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    let prepared_identity_is_gone = {
        let world = app.world_mut();
        let mut query = world.query::<&ambition::runtime::PreparedContentIdentity>();
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
        let mut query = world.query::<&ambition::runtime::PreparedContentIdentity>();
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

/// **Encounter authorities belong to their session** (GPT-5.6 review,
/// 2026-07-16).
///
/// The wave authorities (`populate_encounter_registry`) and the Noether
/// attunement (content) must be spawned session-scoped, exactly like the boss
/// wraps: `SessionTeardownPlugin` clears `EncounterRegistry` on retirement, so
/// an authority that SURVIVED retirement would be duplicated — same
/// `Encounter` id, same `SimId::encounter` — by the next session's
/// repopulation, and identity uniqueness (the snapshot roster invariant)
/// would be violated. Activate A, prove ownership; retire A, prove nothing
/// remains; activate B, prove exactly one authority per id, all B's.
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
