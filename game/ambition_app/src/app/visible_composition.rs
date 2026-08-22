//! Shared visible Ambition composition used by every platform host.
//!
//! Platform-specific code owns foundation/run-loop details; this module owns the
//! common engine, game, shell, presentation, route, and asset composition.
//! Keeping that middle layer single-sourced prevents desktop, capture, and web
//! hosts from drifting into different game compositions.

use bevy::prelude::*;

use ambition_platformer2d::sprite_sheet::game_assets::GameAssetConfig;

use super::plugins::{
    AmbitionGameLdtkRuntimePlugin, AmbitionGamePresentationPlugin, AmbitionGameSimulationPlugin,
};

/// The few genuine choices a platform host makes about the game it composes.
///
/// every field here is a thing hosts really differ on. Anything a host
/// merely *happens* to do differently belongs in [`compose_ambition_visible_game`]
/// instead — that is the whole point of the split. The test of a candidate field
/// is whether two hosts would answer it differently *for a reason*.
#[derive(Clone, Debug)]
pub struct VisibleGameSpec {
    /// Which route the shell host boots into: `true` → the multi-game launcher,
    /// `false` → straight to gameplay.
    ///
    /// the name is historical and kept on purpose. Since K2b BOTH arms are
    /// shell-hosted; this only chooses the initial route, which is what
    /// `--direct` and every `--start-room` alias mean. Renaming it would touch
    /// 33 call sites to restate a boolean whose two values are unchanged.
    pub shell_hosted: bool,
    /// Whether to install the LDtk runtime — the painted tile spine.
    ///
    /// `false` only for hosts that have no render app to draw it into:
    /// `bevy_ecs_tilemap` (inside `LdtkPlugin`) requires a `RenderApp`, and the
    /// `backends: None` no-window recipe deliberately omits one. Ambition's own
    /// room visuals are ordinary sprites and still draw without it.
    pub tile_spine: bool,
    /// Whether to hold gameplay behind the opaque boot curtain
    /// (`InitialGameplayReadiness::closed()` + the direct startup reveal).
    ///
    /// Direct windowed entry only: it stops the OS compositor from showing stale
    /// desktop pixels or an uninitialized swapchain. A shell-hosted boot has its
    /// own route/load lifecycle, and a windowless harness has nothing to cover.
    pub startup_loading_curtain: bool,
    /// Where this host's assets come from. The native builder reads it from the
    /// CLI; the browser takes the profile its Cargo feature set was built for.
    pub asset_config: GameAssetConfig,
}

impl VisibleGameSpec {
    /// The browser persona, named here rather than spelled inline in
    /// `run_web` so it is a value a test can hold and compare instead of a
    /// passage a reader has to re-derive.
    ///
    /// Compiled on every target on purpose: the answers below are about what a
    /// `<canvas>` host wants, and none of them needs a wasm toolchain to state.
    pub fn browser(asset_config: GameAssetConfig) -> Self {
        Self {
            // The browser boots the multi-game launcher — a player's first
            // screen, the same one a desktop player sees. `--direct` is a
            // development entry and a browser has no command line to pass it on.
            shell_hosted: true,
            // A canvas is a real render surface with a real RenderApp, so the
            // painted LDtk tile spine has somewhere to go.
            tile_spine: true,
            // The boot curtain hides a desktop compositor showing stale pixels
            // behind a window that exists before its first frame. A canvas has
            // no such window, and `web/index.html` owns the page's own loading
            // state.
            startup_loading_curtain: false,
            asset_config,
        }
    }
}

/// Compose the visible Ambition game onto an App whose platform host has
/// already installed Bevy's plugin foundation (`DefaultPlugins`, or a modified
/// group, with its window/render/asset settings already chosen).
///
/// `compose_inputs` is the pre-simulation hook — the one moment a caller can
/// reach: after the App exists, before the simulation plugin builds.
///
/// `StartRoomOverride`, `StartRoomMustResolve`, `StartingCharacterOverride` and
/// `SeatsAMatchInsteadOfAHomeBody` are COMPOSITION INPUTS: `init_sandbox_resources` removes
/// them while the simulation plugin builds, so a caller that wants to set one must write it
/// into a world that already exists and has not yet built that plugin.
///
/// a closure rather than a struct of known inputs. A struct would have to
/// enumerate the composition inputs, and the fifth one added elsewhere would not
/// be reachable here — the same "a caller cannot say this" hole, one release
/// later. The hook says *when*; the resources say *what*.
pub fn compose_ambition_visible_game(
    app: &mut App,
    spec: VisibleGameSpec,
    compose_inputs: impl FnOnce(&mut App),
) {
    // DefaultPlugins installs StatesPlugin, so initialize GameMode after it.
    ambition_platformer2d::runtime::init_engine_states(app);
    // Main-world frame schedules run serially: headless measurement showed
    // gameplay bodies at <2% of CPU vs ~40% executor bookkeeping + thread
    // parking (3.7x wall, 32x fewer context switches — see
    // serialize_frame_schedules). The render sub-app keeps its own parallel
    // schedules; this only serializes main-world dispatch. On wasm the same
    // call is not an optimization but a necessity: one thread, so the
    // multithreaded executor's bookkeeping is pure overhead.
    ambition_platformer2d::runtime::serialize_frame_schedules(app);

    {
        // NOT GATED ON `dev_tools`, AND THE SAME HOST ON EVERY PLATFORM.
        // Bevy seals `SimSchedule` when the first simulation plugin registers,
        // so the host is chosen here for the whole build — which is why this
        // sits above the plugins below rather than beside its documentation.
        //
        // The engine owns the session now (the rollback backend's local-session owner), so every
        // build can have one.
        //
        // Ordinary play runs a zero-distance baseline: GGRS drives the
        // simulation deterministically and rollback stays dormant. F9 raises the
        // check distance for one bounded proof pulse and drops it back.
        use ambition_platformer2d::runtime::SimulationHostAppExt as _;
        app.set_simulation_host(ambition_platformer2d::runtime::SimulationHost::Rollback);
    }

    if spec.startup_loading_curtain {
        app.insert_resource(
            ambition_platformer2d::platformer::lifecycle::InitialGameplayReadiness::closed(),
        );
    }

    let active_profile = spec.asset_config.asset_profile;
    app.insert_resource(spec.asset_config);
    // Launch-time "choose your character": inserted BEFORE the plugins so the
    // sandbox preparation consumes it before publishing session authority.
    insert_starting_character_override(app);
    // this resource must be inserted BEFORE the sim plugins build: it is what
    // `publish_direct_prepared_session_root` checks, and without it the app
    // carries the build-time root AND the activation's, which is two canonical
    // roots and a panic on the first read.
    app.insert_resource(super::shell_host::AmbitionShellHosted);

    // THE PRE-SIMULATION HOOK — the last instruction before the simulation
    // plugin builds, which is the deadline every composition input has.
    compose_inputs(app);

    if spec.tile_spine {
        app.add_plugins((
            AmbitionGameSimulationPlugin,
            AmbitionGameLdtkRuntimePlugin,
            AmbitionGamePresentationPlugin,
        ));
    } else {
        app.add_plugins((AmbitionGameSimulationPlugin, AmbitionGamePresentationPlugin));
    }

    // K2b: the shell host is composed EITHER WAY, and the mode only
    // decides which route it boots into. Direct entry stops being a second way
    // to build a game and becomes what `tracks.md` says it should be — *a shell
    // host whose initial route is the gameplay route*, the recipe
    // `ambition_demo_sanic_app` already proves.
    if spec.shell_hosted {
        super::shell_host::compose_ambition_shell_host(app);
    } else {
        super::shell_host::compose_ambition_shell_host_booting_to(
            app,
            super::shell_host::AMBITION_GAMEPLAY_ROUTE,
        );
    }
    // NO ROOM WITHOUT THIS. Losing this one line is what made the
    // browser show a blank canvas, and what made `capture_scene` photograph an
    // empty world for two days. It is not a decoration pass — it is how an
    // activated session's world becomes visible at all.
    super::shell_host::install_ambition_shell_visuals(app);

    if spec.startup_loading_curtain {
        super::startup_loading::install_direct_startup_loading(app);
    }

    // AssetSource registration runs LAST so EmbeddedAssetRegistry (added by
    // `AssetPlugin` inside `DefaultPlugins`) is already present.
    app.add_plugins(
        ambition_platformer2d::actors::assets::platformer_assets::AmbitionAssetSourcePlugin::for_profile(
            active_profile,
            &ambition_content::worlds::world_manifest(),
        ),
    );
}

/// A no-op in the browser, where the env read simply returns `Err`.
fn insert_starting_character_override(app: &mut App) {
    let Ok(raw) = std::env::var("AMBITION_START_CHARACTER") else {
        return;
    };
    let id = raw.trim();
    if id.is_empty() {
        return;
    }
    bevy::log::info!(
        target: "ambition_app",
        "starting as character '{id}' (AMBITION_START_CHARACTER)"
    );
    app.insert_resource(super::resources::StartingCharacterOverride(
        ambition_platformer2d::actors::avatar::StartingCharacter::new(id),
    ));
}
