//! **The Ambition multi-game shell host** — title screen, provider
//! composition, and the Ambition game as one provider among equals.
//!
//! `compose_ambition_shell_host` turns the visible Ambition app into a
//! shell-routed host: `./run_game.sh` boots into the Ambition launcher
//! (title screen), whose entries derive from registered experience providers
//! (Ambition, Sanic, Mary-O, Pocket — plus Exit). Selecting an entry activates that
//! provider's gameplay session through the shared shell/session/load
//! lifecycle; `QuitToHome` retires the exact session and resumes the
//! launcher; Exit leaves the process.
//!
//! The Ambition GAME lives behind [`AmbitionExperiencePlugin`] — the same
//! provider contract the demos use. Its activation constructs a fresh
//! session-scoped simulation world from the boot-prepared LDtk data
//! ([`AmbitionPreparedWorld`]); teardown is the generic session-scope sweep.
//! The provider names no launcher route: home is host-relative.
//!
//! Direct development entry (`--direct`, or any `--start-room`/mode alias
//! that wants to land in gameplay immediately) keeps the pre-shell path: the
//! world constructs at `Startup` exactly as before and no launcher exists.
//! That choice is host CONFIGURATION, not a second gameplay implementation —
//! both paths run the same construction code, differing only in when it runs
//! and who owns the spawned entities.

use bevy::prelude::*;

use ambition::game_shell::{
    GameplaySessionEvent, ShellCommand, ShellCompletionPolicy, ShellEvent,
    ShellHostConfiguration, ShellHostSpec, ShellRouteCatalog, ShellRouteSpec,
};

use ambition::actors::ldtk_world;
use ambition::platformer::lifecycle::SessionScopeSet;

/// The host's home/title route. Providers never name it — `QuitToHome`
/// resolves here because the HOST declared it, not because any game knows it.
pub const AMBITION_LAUNCHER_ROUTE: &str = "ambition_launcher";

/// Marker: this App is composed as the shell-routed multi-game host. Startup
/// world construction is skipped (sessions construct on activation) and the
/// launcher owns the frontend. Absent in direct-entry and headless harnesses.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct AmbitionShellHosted;

/// Bevy run condition: this App boots straight into gameplay (the pre-shell
/// path). True whenever [`AmbitionShellHosted`] is absent.
pub fn direct_entry(hosted: Option<Res<AmbitionShellHosted>>) -> bool {
    hosted.is_none()
}

/// Ambition's gameplay implementation is a reusable provider crate. The host
/// re-exports its public identities for compatibility while owning only home,
/// startup, platform, and process policy.
pub use ambition_content::provider::{
    AmbitionExperienceConfig, AmbitionExperiencePlugin, AmbitionPreparedWorld,
    AMBITION_EXPERIENCE, AMBITION_GAMEPLAY_ROUTE,
};

/// Compose the shell-routed multi-game host on top of the already-composed
/// visible Ambition app: shell/load/session plugins, the three linked
/// providers, the launcher-as-home routing, process exit, and the universal
/// in-session Quit to Home binding.
pub fn compose_ambition_shell_host(app: &mut App) {
    app.insert_resource(AmbitionShellHosted);

    // The title screen has its own theme. The engine's frontend audio policy
    // loops this track whenever no gameplay session is live (and enforces
    // silence otherwise); the host names the song, the engine owns the seam.
    app.insert_resource(
        ambition::audio::selection::FrontendAudioProfile::new(ambition_content::AMBITION_CONTENT_PROVIDER)
            .with_title_track("a_possible_morning")
            .with_sfx([
                ambition::sfx::ids::UI_MENU_MOVE,
                ambition::sfx::ids::UI_MENU_ACCEPT,
                ambition::sfx::ids::UI_MENU_BACK,
            ]),
    );

    app.add_plugins((
        ambition::game_shell::MinimalShellPlugins,
        ambition::load::AmbitionLoadPlugin,
        ambition::load_presentation::MinimalLoadPresentationPlugins,
    ));
    app.add_plugins(ambition::session_world::PlatformerSessionWorldProjectionPlugin);

    // The linked providers. Each registers its experience, routes, catalog
    // fragments, session construction, and rules; the launcher below derives
    // its entries from these registrations — no per-game match arms.
    app.add_plugins((
        AmbitionExperiencePlugin::new(AmbitionExperienceConfig::default()),
        ambition_demo_sanic::SanicExperiencePlugin,
        ambition_demo_smb1::Smb1ExperiencePlugin,
        ambition_demo_pocket::PocketExperiencePlugin,
    ));

    // Host routing: boot into the launcher; every provider's ReturnHome
    // resolves to the launcher. The home route is a plain shell experience
    // (the basic launcher presentation renders it).
    app.world_mut()
        .resource_mut::<ShellRouteCatalog>()
        .register(ShellRouteSpec::new(
            AMBITION_LAUNCHER_ROUTE,
            ambition::game_shell::ShellLaunchCatalog::basic_experience_id(),
        ));
    app.world_mut()
        .resource_mut::<ShellHostConfiguration>()
        .spec = Some(ShellHostSpec::new(
        AMBITION_LAUNCHER_ROUTE,
        AMBITION_LAUNCHER_ROUTE,
    ));

    app.add_systems(Update, (exit_on_shell_request, quit_to_home_on_key));
}

/// The optional "Powered by Ambition" startup vanity sequence.
pub const AMBITION_STARTUP_EXPERIENCE: &str = "ambition_startup";
pub const AMBITION_STARTUP_ROUTE: &str = "ambition_startup";

/// Compose the optional startup vanity screen in front of the launcher.
///
/// The HOST chooses this frontend presentation policy — `--direct` and the
/// rendered-ownership tests simply don't compose it and boot straight to the
/// launcher. It is one text card, auto-advancing after a couple seconds and
/// skippable with confirm (Enter / South); on completion it routes to the
/// launcher. No gameplay session exists during startup: it is a plain shell
/// experience, not a gameplay route, so the simulation stays asleep and the
/// launcher owns exactly one frontend authority once the card hands off.
///
/// Uses the existing shell SEQUENCE mechanism (no new state machine): a
/// `ShellSequenceCatalog` entry keyed by the startup experience, a route whose
/// `on_complete` is `GoTo(launcher)`, and the startup route as the initial one.
pub fn compose_ambition_startup_sequence(app: &mut App) {
    use ambition::game_shell::{
        ShellExperienceId, ShellSegmentSpec, ShellSequenceCatalog, ShellSequenceSpec,
    };

    app.world_mut()
        .resource_mut::<ShellRouteCatalog>()
        .register(
            ShellRouteSpec::new(AMBITION_STARTUP_ROUTE, AMBITION_STARTUP_EXPERIENCE)
                .on_complete(ShellCompletionPolicy::GoTo(AMBITION_LAUNCHER_ROUTE.into())),
        );
    app.world_mut()
        .resource_mut::<ShellSequenceCatalog>()
        .register(
            ShellExperienceId::new(AMBITION_STARTUP_EXPERIENCE),
            ShellSequenceSpec {
                segments: vec![ShellSegmentSpec::text(
                    "powered_by_ambition",
                    "Powered by Ambition",
                )],
            },
        );
    // Boot into the startup card; home stays the launcher, so the startup's
    // completion AND any later QuitToHome both resolve to the launcher.
    app.world_mut()
        .resource_mut::<ShellHostConfiguration>()
        .spec = Some(ShellHostSpec::new(
        AMBITION_STARTUP_ROUTE,
        AMBITION_LAUNCHER_ROUTE,
    ));
}

/// Visible-host wiring: per-session presentation (room visuals, parallax,
/// moving platforms, HUD, LDtk visual spine roots) constructed on Ambition
/// activation with the session's captured scope. Registered only by the
/// windowed composition — headless hosts run the same lifecycle without it.
pub fn install_ambition_shell_visuals(app: &mut App) {
    // Provider-agnostic per-session room presentation: parallax + static room
    // visuals for WHATEVER RoomSet the activating provider republished —
    // Sanic and Mary-O draw in this host through the same one system.
    app.add_plugins(ambition::render::platformer_presentation::SessionRoomVisualsPlugin);
    app.add_systems(
        Update,
        ambition_activate_session_visuals.in_set(SessionScopeSet::Presentation),
    );
}

/// Spawn the SESSION-owned presentation for a fresh Ambition activation. Runs
/// after the session bridge + providers (command flush between), so the
/// session's player entity already exists.
#[allow(clippy::too_many_arguments)]
fn ambition_activate_session_visuals(
    mut sessions: MessageReader<GameplaySessionEvent>,
    mut commands: Commands,
    active_session: Res<ambition::game_shell::ActiveGameplaySession>,
    session_worlds: Query<&ambition::runtime::PlatformerSessionWorld>,
    game_assets: Option<Res<ambition::sprite_sheet::game_assets::GameAssets>>,
    ui_fonts: Option<Res<ambition::render::ui_fonts::UiFonts>>,
    asset_server: Res<AssetServer>,
    world_assets: Option<Res<ldtk_world::LdtkWorldAssets>>,
    sandbox_asset_collection: Option<
        Res<ambition::actors::assets::loading::SandboxAssetCollection>,
    >,
    // Present iff the LDtk plugin stack is composed (absent in the no-window
    // render recipe, where bevy_ecs_tilemap cannot run without a RenderApp).
    ldtk_projects: Option<Res<Assets<bevy_ecs_ldtk::assets::LdtkProject>>>,
    players: Query<Entity, With<ambition::actors::actor::PrimaryPlayer>>,
) {
    for event in sessions.read() {
        let GameplaySessionEvent::Activated { activation, scope } = event else {
            continue;
        };
        if activation.experience_id.as_str() != AMBITION_EXPERIENCE {
            continue;
        }
        if game_assets.is_none() {
            // No presentation assets loaded (headless composition) — the
            // session is sim-only by construction.
            continue;
        }
        let Some(world_entity) = active_session.active_world_entity() else {
            continue;
        };
        let Ok(session_world) = session_worlds.get(world_entity) else {
            continue;
        };
        let scope = ambition::platformer::lifecycle::SessionSpawnScope::scoped(*scope);
        let Ok(player) = players.single() else {
            continue;
        };
        // Parallax + room visuals are the generic `SessionRoomVisualsPlugin`'s
        // job; this system adds only Ambition's own dressing.
        super::scene_setup::session_gameplay_dressing(
            &mut commands,
            scope,
            super::scene_setup::SessionDressingSetup {
                world: &session_world.geometry,
                room_set: &session_world.room_set,
                ui_fonts: ui_fonts.as_deref(),
            },
            player,
        );
        if ldtk_projects.is_some() {
            super::plugins::spawn_ldtk_world_roots_scoped(
                &mut commands,
                scope,
                &asset_server,
                &session_world.runtime_rooms,
                &session_world.room_set,
                world_assets.as_deref(),
                sandbox_asset_collection.as_deref(),
            );
        }
    }
}

/// The HOST owns process exit: the launcher's Exit entry (and any
/// `ShellCommand::ExitProcess`) raises `ShellEvent::ExitRequested`, which the
/// shell crates deliberately do not act on.
fn exit_on_shell_request(mut events: MessageReader<ShellEvent>, mut exit: MessageWriter<AppExit>) {
    for event in events.read() {
        if matches!(event, ShellEvent::ExitRequested) {
            exit.write(AppExit::Success);
        }
    }
}

/// Universal semantic Quit to Home: while any gameplay session is live, F10
/// (keyboard) or the controller's Start button retires it and returns to the
/// host's home route. Presentation-level binding → semantic command; no
/// provider names a route. (The in-game pause menu can grow a "Quit to Home"
/// entry on top of the same command.)
fn quit_to_home_on_key(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    pads: Query<&bevy::input::gamepad::Gamepad>,
    session: Res<ambition::game_shell::ActiveGameplaySession>,
    mut shell: MessageWriter<ShellCommand>,
    mut analog: Local<ambition::game_shell::ShellAnalogLatch>,
) {
    if session.0.is_none() {
        return;
    }
    let actions = ambition::game_shell::shell_action_edges(keys.as_deref(), &pads, &mut analog);
    if actions.quit_to_home {
        shell.write(ShellCommand::QuitToHome);
    }
}
