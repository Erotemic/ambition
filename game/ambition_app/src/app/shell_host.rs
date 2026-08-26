//! Multi-game shell host for Ambition and registered experience providers.
//!
//! The launcher derives rows from provider registrations; selecting a row enters
//! that provider's declared entry route and session lifecycle, while `QuitToHome`
//! retires the active session and returns to the host-owned launcher. Ambition
//! itself implements the same provider contract as demos. Direct development
//! entry bypasses the launcher but uses the same gameplay construction path.

use bevy::prelude::*;

use ambition_platformer2d::game_shell::{
    GameplaySessionEvent, ShellCompletionPolicy, ShellEvent, ShellHostConfiguration, ShellHostSpec,
    ShellRouteCatalog, ShellRouteSpec,
};

use ambition_platformer2d::ldtk_map as ldtk_world;
use ambition_platformer2d::world::world_manifest;
use ambition_platformer2d::platformer::lifecycle::SessionScopeSet;

/// The host's home/title route. Providers never name it — `QuitToHome`
/// resolves here because the HOST declared it, not because any game knows it.
pub const AMBITION_LAUNCHER_ROUTE: &str = "ambition_launcher";

/// Marker: this App is composed as the shell-routed multi-game host. Startup
/// world construction is skipped (sessions construct on activation) and the
/// launcher owns the frontend. Absent in direct-entry and headless harnesses.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct AmbitionShellHosted;

/// Ambition gameplay provider identities temporarily re-exported by the host.
/// TODO(compat-remove): migrate callers to `ambition_content::provider`, then
/// remove this host-level re-export.
pub use ambition_content::provider::{
    AmbitionExperienceConfig, AmbitionExperiencePlugin, AmbitionPreparedWorld, AMBITION_EXPERIENCE,
    AMBITION_GAMEPLAY_ROUTE,
};

/// Compose a headless Ambition gameplay host in dependency order: mark the app
/// shell-hosted, install simulation, then install the shell booted to gameplay.
/// The caller supplies the Bevy foundation. Activation remains asynchronous;
/// callers that require a live session world settle it explicitly.
pub fn compose_ambition_gameplay_host(app: &mut App) {
    app.insert_resource(AmbitionShellHosted);
    app.add_plugins(crate::app::AmbitionGameSimulationPlugin);
    compose_ambition_shell_host_booting_to(app, AMBITION_GAMEPLAY_ROUTE);
}

pub fn compose_ambition_shell_host_booting_to(app: &mut App, initial_route: &str) {
    compose_ambition_shell_host_inner(app, initial_route);
}

pub fn compose_ambition_shell_host(app: &mut App) {
    compose_ambition_shell_host_inner(app, AMBITION_LAUNCHER_ROUTE);
}

fn compose_ambition_shell_host_inner(app: &mut App, initial_route: &str) {
    app.insert_resource(AmbitionShellHosted);

    // The title screen has its own theme. The engine's frontend audio policy
    // loops this track whenever no gameplay session is live (and enforces
    // silence otherwise); the host names the song, the engine owns the seam.
    //
    // A provider whose own screen has its own score declares that beside its content and it travels
    // here; smash's character select is the first.
    {
        use ambition_platformer2d::audio::selection::FrontendAudioAppExt;
        app.set_host_frontend_audio(
            ambition_platformer2d::audio::selection::FrontendAudioProfile::new(
                ambition_content::AMBITION_CONTENT_PROVIDER,
            )
            .with_title_track("something_worth_building")
            .with_sfx([
                ambition_platformer2d::sfx::ids::UI_MENU_MOVE,
                ambition_platformer2d::sfx::ids::UI_MENU_ACCEPT,
                ambition_platformer2d::sfx::ids::UI_MENU_BACK,
            ]),
        );
    }

    app.add_plugins(ambition_platformer2d::game_shell::MinimalShellPlugins);
    // The normal visible-app composition already installed contributor-neutral
    // load presentation for direct and room-transition use. Keep this host
    // composer valid in isolation as well, then add only the shell adapter.
    if !app.is_plugin_added::<ambition_platformer2d::load_presentation::AmbitionLoadPresentationPlugin>() {
        app.add_plugins(ambition_platformer2d::load_presentation::MinimalLoadPresentationPlugins);
    }
    if !app.is_plugin_added::<ambition_platformer2d::load_presentation::AmbitionLoadShellPresentationPlugin>() {
        app.add_plugins(ambition_platformer2d::load_presentation::AmbitionLoadShellPresentationPlugin);
    }

    // The linked providers. Each registers its experience, routes, catalog
    // fragments, session construction, and rules; the launcher below derives
    // its entries from these registrations — no per-game match arms.
    app.add_plugins((
        AmbitionExperiencePlugin::new(AmbitionExperienceConfig::default()),
        ambition_demo_sanic::SanicExperiencePlugin,
        ambition_demo_mary_o::MaryOExperiencePlugin,
        ambition_demo_pocket::PocketExperiencePlugin,
        ambition_demo_twintrack::TwinTrackExperiencePlugin,
        // The stocks demo. It is the first provider whose launcher row does NOT
        // open its gameplay route: "Smash" opens CHARACTER SELECT, which the
        // demo registers as a frontend route of its own and which then asks the
        // shell for the stage once every seat has locked in. Nothing here knows
        // that — the row is derived from the registration like every other.
        ambition_demo_smash::SmashExperiencePlugin,
    ));

    // The versus stage. Registered AFTER the providers because its fighters are
    // theirs: `mary_o` and `sanic` are registered by two different provider
    // plugins, and this composition is the only one where both casts exist.
    crate::app::versus::compose_versus_experience(app);

    // Host routing: boot into the launcher; every provider's ReturnHome
    // resolves to the launcher. The home route is a plain shell experience
    // (the basic launcher presentation renders it).
    app.world_mut()
        .resource_mut::<ShellRouteCatalog>()
        .register(ShellRouteSpec::new(
            AMBITION_LAUNCHER_ROUTE,
            ambition_platformer2d::game_shell::ShellLaunchCatalog::basic_experience_id(),
        ));
    app.world_mut()
        .resource_mut::<ShellHostConfiguration>()
        .spec = Some(ShellHostSpec::new(initial_route, AMBITION_LAUNCHER_ROUTE));

    // ORDERED, not ambiguous.
    app.add_systems(
        Update,
        exit_on_shell_request
            .after(ambition_platformer2d::game_shell::AmbitionGameShellSet::Commands),
    );
    // The pause-suppression bridge only exists when the universal pause menu it
    // yields to does — that menu (and its `ShellPauseMenuSuppressed`) ships with
    // `basic_presentation`. The minimal web build omits both, exactly the
    // "standalone demo app" case this bridge documents as absent.
    #[cfg(feature = "basic_shell_presentation")]
    app.add_systems(Update, sync_shell_pause_suppression);
}

/// Ambition's own gameplay has the richer kaleidoscope pause menu, so the
/// universal shell pause menu (which the hosted demos rely on) must yield while
/// an Ambition room is live. `in_base_mode` is true iff the active session is
/// Ambition's own (no demo mode tag) — the exact complement of the kaleidoscope's
/// gate — so the two menus partition every live session with no overlap. In a
/// standalone demo app this bridge is absent and the flag stays `false`.
#[cfg(feature = "basic_shell_presentation")]
fn sync_shell_pause_suppression(
    active: Option<
        ambition_platformer2d::platformer::lifecycle::SessionWorldRef<
            ambition_platformer2d::world::rooms::ActiveRoomMetadata,
        >,
    >,
    mut suppressed: ResMut<ambition_platformer2d::game_shell::ShellPauseMenuSuppressed>,
) {
    suppressed.0 = ambition_platformer2d::runtime::in_base_mode(active);
}

/// The optional startup vanity sequence (engine card, then authorship card).
pub const AMBITION_STARTUP_EXPERIENCE: &str = "ambition_startup";
pub const AMBITION_STARTUP_ROUTE: &str = "ambition_startup";

/// The startup run-in's cards, in the conventional order: what the game was
/// built WITH, then who built it. Each is a separate segment, so each fades
/// in/out on its own and confirm skips ONE card rather than the whole run-in.
///
/// Adding another card is one more entry here — no new state, and every
/// consumer that cares how long the run-in lasts derives it from
/// [`ambition_startup_duration`] rather than restating a number.
fn ambition_startup_segments() -> Vec<ambition_platformer2d::game_shell::ShellSegmentSpec> {
    use ambition_platformer2d::game_shell::{
        ShellSegmentPolicy, ShellSegmentRole, ShellSegmentSpec,
    };

    vec![
        // The ENGINE card. Held longer than the 2s default so its ease-in /
        // hold / ease-out has room to breathe.
        ShellSegmentSpec::text("powered_by_ambition", "Powered by Ambition").with_policy(
            ShellSegmentPolicy {
                auto_advance_after: Some(std::time::Duration::from_millis(3600)),
                ..Default::default()
            },
        ),
        // The AUTHORSHIP card — the authored comic beat, DRAWN rather than played back. Its length
        // is still DERIVED — `made_this_meme_card_duration()` is `frame_ms × frames.len()` off
        // `vanity_card_made_this_meme.ron` — so re-exporting the animation retimes this segment and
        // there is nothing to keep in sync by hand.
        //
        // that RON is the BAKED RIG, not `vanity_card.ron`. The nine-frame
        // manifest and its rendered payload are reference art with no reader in
        // the workspace; naming "the same manifest" here sent
        // readers to the dead one.
        //
        // The id is the punchline because the studio is unnamed. When there IS a
        // studio name, rename this segment to it.
        ShellSegmentSpec::registered(
            "i_made_this",
            ShellSegmentRole::Vanity,
            ambition_content::presentation::vanity_card_made_this_meme::MADE_THIS_MEME_CARD_SEGMENT_KIND,
        )
        .with_policy(ShellSegmentPolicy {
            auto_advance_after: Some(
                ambition_content::presentation::vanity_card_made_this_meme::made_this_meme_card_duration(),
            ),
            ..Default::default()
        }),
    ]
}

/// How long the composed startup run-in plays if nobody presses confirm.
///
/// Derived from the same segment list the host actually composes, so a retimed card, an added
/// card, or a re-exported vanity animation cannot leave a caller waiting on a stale constant.
pub fn ambition_startup_duration() -> std::time::Duration {
    ambition_startup_segments()
        .iter()
        .map(|segment| segment.policy.auto_advance_after.unwrap_or_default())
        .sum()
}

/// Compose the optional startup vanity screens in front of the launcher.
///
/// The HOST chooses this frontend presentation policy — `--direct` and the
/// rendered-ownership tests simply don't compose it and boot straight to the
/// launcher. It is a list of cards, each auto-advancing on its own timing and
/// each skippable with confirm (Enter / South); on completion it routes to the
/// launcher. No gameplay session exists during startup: it is a plain shell
/// experience, not a gameplay route, so the simulation stays asleep and the
/// launcher owns exactly one frontend authority once the last card hands off.
///
/// Adding another card is one more entry in `segments` — no new state.
///
/// Uses the existing shell SEQUENCE mechanism (no new state machine): a
/// `ShellSequenceCatalog` entry keyed by the startup experience, a route whose
/// `on_complete` is `GoTo(launcher)`, and the startup route as the initial one.
pub fn compose_ambition_startup_sequence(app: &mut App) {
    use ambition_platformer2d::game_shell::{
        ShellExperienceId, ShellSequenceCatalog, ShellSequenceSpec,
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
                segments: ambition_startup_segments(),
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
    // visuals for WHATEVER RoomSet the activating provider owns —
    // Sanic and Mary-O draw in this host through the same one system.
    app.add_plugins(
        ambition_platformer2d::render::platformer_presentation::SessionRoomVisualsPlugin,
    );
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
    active_session: Res<ambition_platformer2d::game_shell::ActiveGameplaySession>,
    session_worlds: Query<(
        &ambition_platformer2d::engine_core::RoomGeometry,
        &ambition_platformer2d::world::rooms::RoomSet,
        &ambition_platformer2d::ldtk_map::LdtkRuntimeIndex,
    )>,
    game_assets: Option<Res<ambition_platformer2d::sprite_sheet::game_assets::GameAssets>>,
    ui_fonts: Option<Res<ambition_platformer2d::render::ui_fonts::UiFonts>>,
    asset_server: Res<AssetServer>,
    world_assets: Option<Res<ldtk_world::LdtkWorldAssets>>,
    sandbox_asset_collection: Option<
        Res<ambition_platformer2d::actors::assets::loading::Platformer2dStartupAssets>,
    >,
    // Present iff the LDtk plugin stack is composed (absent in the no-window
    // render recipe, where bevy_ecs_tilemap cannot run without a RenderApp).
    ldtk_projects: Option<Res<Assets<bevy_ecs_ldtk::assets::LdtkProject>>>,
    world_manifest: Res<world_manifest::WorldManifest>,
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
        // the room GEOMETRY is no longer read here: the dressing wanted it only
        // to place moving-platform sprites, and those are a render family's now.
        let Ok((_geometry, room_set, runtime_rooms)) = session_worlds.get(world_entity) else {
            continue;
        };
        let scope = ambition_platformer2d::platformer::lifecycle::SessionSpawnScope::scoped(*scope);
        ambition_platformer2d::menu::map::spawn_map_menu_with_scope(&mut commands, scope);
        // Parallax + room visuals are the generic `SessionRoomVisualsPlugin`'s
        // job; this system adds only Ambition's own dressing.
        super::scene_setup::session_gameplay_dressing(
            &mut commands,
            scope,
            super::scene_setup::SessionDressingSetup {
                ui_fonts: ui_fonts.as_deref(),
            },
        );
        if ldtk_projects.is_some() {
            super::plugins::spawn_ldtk_world_roots_scoped(
                &mut commands,
                scope,
                &asset_server,
                runtime_rooms,
                room_set,
                world_assets.as_deref(),
                sandbox_asset_collection.as_deref(),
                &world_manifest,
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
