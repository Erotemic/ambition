//! **The Ambition multi-game shell host** — title screen, provider
//! composition, and the Ambition game as one provider among equals.
//!
//! `compose_ambition_shell_host` turns the visible Ambition app into a
//! shell-routed host: `./run_game.sh` boots into the Ambition launcher
//! (title screen), whose entries derive from registered experience providers
//! (Ambition, Sanic, Mary-O — plus Exit). Selecting an entry activates that
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
    ExperienceRegistration, GameplaySessionAppExt, GameplaySessionEvent, GameplaySessionSet,
    ShellCommand, ShellCompletionPolicy, ShellEvent, ShellHostConfiguration, ShellHostSpec,
    ShellRouteCatalog, ShellRouteSpec,
};
use ambition::platformer::lifecycle::SessionSpawnScope;

use ambition::actors::ldtk_world;
use ambition::actors::rooms;
use ambition::actors::session::setup;
use ambition::engine_core::RoomGeometry;
use ambition::platformer::lifecycle::SessionScopeSet;

/// The Ambition experience/provider identity and routes.
pub const AMBITION_EXPERIENCE: &str = "ambition";
pub const AMBITION_GAMEPLAY_ROUTE: &str = "ambition_gameplay";
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

/// The boot-prepared immutable Ambition world data every activation clones
/// from: the validated LDtk room set, its runtime index, and the starting
/// character resolved at boot (CLI/env overrides included). Captured once by
/// `init_sandbox_resources`; each activation republishes FRESH copies so a
/// relaunch cannot observe a previous session's room state, and a previously
/// activated provider (which republished the shared world-pointer resources
/// for ITS world) cannot leak into an Ambition session.
#[derive(Resource, Clone)]
pub struct AmbitionPreparedWorld {
    pub room_set: rooms::RoomSet,
    pub ldtk_index: ldtk_world::LdtkRuntimeIndex,
    pub starting_character: ambition::actors::avatar::StartingCharacter,
}

/// The Ambition game as a reusable experience provider: registration + the
/// session-construction system. Headless-safe — presentation and LDtk visual
/// roots are wired by the visible plugin stacks, reacting to the same
/// session events.
pub struct AmbitionExperiencePlugin;

impl Plugin for AmbitionExperiencePlugin {
    fn build(&self, app: &mut App) {
        app.register_gameplay_experience(
            ExperienceRegistration::new(AMBITION_EXPERIENCE, "Ambition", AMBITION_GAMEPLAY_ROUTE)
                .with_description("The main Ambition campaign"),
            ShellRouteSpec::new(AMBITION_GAMEPLAY_ROUTE, AMBITION_EXPERIENCE)
                .on_complete(ShellCompletionPolicy::ReturnHome),
        );
        app.add_systems(
            Update,
            ambition_activate_session.in_set(GameplaySessionSet::Providers),
        );
    }
}

/// Construct a fresh Ambition gameplay session for THIS activation: republish
/// fresh world authority from the boot-prepared data, then spawn the
/// simulation world with the activation's captured session scope so the
/// generic scope sweep retires all of it on `QuitToHome`.
#[allow(clippy::too_many_arguments)]
fn ambition_activate_session(
    mut sessions: MessageReader<GameplaySessionEvent>,
    mut commands: Commands,
    prepared: Res<AmbitionPreparedWorld>,
    sandbox_data_asset: Option<Res<ambition::actors::session::data::SandboxDataAsset>>,
    sandbox_asset_collection: Option<
        Res<ambition::actors::assets::loading::SandboxAssetCollection>,
    >,
    asset_server: Res<AssetServer>,
    editable_tuning: Res<ambition::dev_tools::dev_tools::EditableMovementTuning>,
    editable_abilities: Res<ambition::dev_tools::dev_tools::EditableAbilitySet>,
    character_catalog: Res<ambition::characters::actor::character_catalog::CharacterCatalog>,
    character_roster: Res<ambition::actors::features::CharacterRoster>,
    boss_catalog: Res<ambition::actors::boss_encounter::BossCatalog>,
    mut platform_set: ResMut<ambition::world::collision::MovingPlatformSet>,
) {
    for event in sessions.read() {
        let GameplaySessionEvent::Activated { activation, scope } = event else {
            continue;
        };
        if activation.experience_id.as_str() != AMBITION_EXPERIENCE {
            continue;
        }

        // Fresh world authority: clones of the boot-prepared immutable data,
        // never whatever a previous session (of any provider) left resident.
        let room_set = prepared.room_set.clone();
        let world = RoomGeometry(room_set.active_world().clone());
        let starting_character = prepared.starting_character.clone();

        let _player = setup::simulation_world(
            &mut commands,
            SessionSpawnScope::scoped(*scope),
            setup::SimulationSetup {
                world: &world,
                room_set: &room_set,
                ldtk_index: &prepared.ldtk_index,
                editable_abilities: &editable_abilities,
                editable_tuning: &editable_tuning,
                starting_character: &starting_character,
                character_catalog: &character_catalog,
                character_roster: &character_roster,
                boss_catalog: &boss_catalog,
                default_character_id: ambition_content::character_catalog::PLAYABLE_ROSTER[0],
                sandbox_data_asset: sandbox_data_asset.as_deref(),
                sandbox_asset_collection: sandbox_asset_collection.as_deref(),
                asset_server: &asset_server,
            },
        );
        platform_set.0 =
            ambition::actors::world::platforms::moving_platforms_for_room(room_set.active_spec());

        commands.insert_resource(prepared.ldtk_index.clone());
        commands.insert_resource(world);
        commands.insert_resource(rooms::ActiveRoomMetadata::default());
        commands.insert_resource(room_set);
        commands.insert_resource(starting_character);
    }
}

/// Compose the shell-routed multi-game host on top of the already-composed
/// visible Ambition app: shell/load/session plugins, the three linked
/// providers, the launcher-as-home routing, process exit, and the universal
/// in-session Quit to Home binding.
pub fn compose_ambition_shell_host(app: &mut App) {
    app.insert_resource(AmbitionShellHosted);

    app.add_plugins((
        ambition::game_shell::MinimalShellPlugins,
        ambition::load::AmbitionLoadPlugin,
        ambition::load_presentation::MinimalLoadPresentationPlugins,
    ));

    // The linked providers. Each registers its experience, routes, catalog
    // fragments, session construction, and rules; the launcher below derives
    // its entries from these registrations — no per-game match arms.
    app.add_plugins((
        AmbitionExperiencePlugin,
        ambition_demo_sanic::SanicExperiencePlugin,
        ambition_demo_smb1::Smb1ExperiencePlugin,
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

/// Visible-host wiring: per-session presentation (room visuals, parallax,
/// moving platforms, HUD, LDtk visual spine roots) constructed on Ambition
/// activation with the session's captured scope. Registered only by the
/// windowed composition — headless hosts run the same lifecycle without it.
pub fn install_ambition_shell_visuals(app: &mut App) {
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
    prepared: Res<AmbitionPreparedWorld>,
    physics_settings: Res<ambition::actors::world::physics::PhysicsSandboxSettings>,
    game_assets: Option<Res<ambition::sprite_sheet::game_assets::GameAssets>>,
    quality: Option<Res<ambition::render::quality::ResolvedVisualQuality>>,
    ui_fonts: Option<Res<ambition::render::ui_fonts::UiFonts>>,
    asset_server: Res<AssetServer>,
    world_assets: Option<Res<ldtk_world::LdtkWorldAssets>>,
    sandbox_asset_collection: Option<
        Res<ambition::actors::assets::loading::SandboxAssetCollection>,
    >,
    players: Query<Entity, With<ambition::actors::actor::PrimaryPlayer>>,
) {
    for event in sessions.read() {
        let GameplaySessionEvent::Activated { activation, scope } = event else {
            continue;
        };
        if activation.experience_id.as_str() != AMBITION_EXPERIENCE {
            continue;
        }
        let Some(game_assets) = game_assets.as_deref() else {
            // No presentation assets loaded (headless composition) — the
            // session is sim-only by construction.
            continue;
        };
        let scope = ambition::platformer::lifecycle::SessionSpawnScope::scoped(*scope);
        let world = RoomGeometry(prepared.room_set.active_world().clone());
        let Ok(player) = players.single() else {
            continue;
        };
        super::scene_setup::session_presentation(
            &mut commands,
            scope,
            super::scene_setup::SessionPresentationSetup {
                world: &world,
                room_set: &prepared.room_set,
                physics_settings: *physics_settings,
                game_assets,
                quality: quality.as_deref(),
                ui_fonts: ui_fonts.as_deref(),
            },
            player,
        );
        super::plugins::spawn_ldtk_world_roots_scoped(
            &mut commands,
            scope,
            &asset_server,
            &prepared.ldtk_index,
            &prepared.room_set,
            world_assets.as_deref(),
            sandbox_asset_collection.as_deref(),
        );
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
/// retires it and returns to the host's home route. Presentation-level
/// binding → semantic command; no provider names a route. (The in-game pause
/// menu can grow a "Quit to Home" entry on top of the same command.)
fn quit_to_home_on_key(
    // Optional because it models DEVICE presence, not authority: a headless
    // host has no keyboard and simply has no key binding.
    keys: Option<Res<ButtonInput<KeyCode>>>,
    session: Res<ambition::game_shell::ActiveGameplaySession>,
    mut shell: MessageWriter<ShellCommand>,
) {
    let Some(keys) = keys else {
        return;
    };
    if session.0.is_some() && keys.just_pressed(KeyCode::F10) {
        shell.write(ShellCommand::QuitToHome);
    }
}
