use bevy::prelude::*;
// Fixed-resolution desktop window sizing — only the native `build_visible_app`
// sets it; the web build sizes its canvas from CSS.
#[cfg(not(target_arch = "wasm32"))]
use bevy::window::WindowResolution;

#[cfg(not(target_arch = "wasm32"))]
use ambition_platformer2d::engine_core::config::{WINDOW_H, WINDOW_W};
use ambition_platformer2d::sprite_sheet::game_assets::GameAssetConfig;

/// Resolve the on-disk asset root for the desktop app.
///
/// Bevy's `FileAssetReader` anchors relative asset paths at
/// `BEVY_ASSET_ROOT` / the RUNNING binary's `CARGO_MANIFEST_DIR` — which
/// has been `game/ambition_app/` since the Stage 20 / A3 bisection,
/// while the asset tree stays with the machinery lib at
/// `crates/ambition_platformer2d_actor_monolith/assets` (the lib's `include_str!` paths and
/// the regen scripts anchor there). Under `cargo run` that default broke
/// every AssetServer load (sprites, music OGGs, `.yarn` dialogue, menu
/// icons) while direct-filesystem readers (SFX bank, LDtk) kept working.
///
/// Resolution order:
/// 1. `BEVY_ASSET_ROOT` set → return Bevy's default relative `"assets"`
///    so the explicit override keeps full control.
/// 2. The dev-checkout sandbox tree exists (cargo runs) → its absolute
///    path (an absolute `file_path` replaces Bevy's base when joined).
/// 3. Otherwise (shipped builds) → Bevy's default exe-relative `"assets"`.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn desktop_asset_root() -> String {
    // The single source of truth for the actors-assets file root now lives in
    // `ambition_asset_manager` so the hosted app and every standalone demo app
    // resolve it identically (a demo that rendered nothing standalone was exactly
    // that divergence). It anchors on the machinery lib's own `CARGO_MANIFEST_DIR`,
    // so it no longer matters which `game/` crate the running binary lives in.
    ambition_platformer2d::asset_manager::actors_desktop_asset_root()
}

/// The `game://` asset source root: the content crate's `assets/` tree in
/// a dev checkout (worlds live there post-R3.2), the exe-relative
/// `assets` dir in shipped builds.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn game_asset_root() -> String {
    if std::env::var_os("BEVY_ASSET_ROOT").is_some() {
        return "assets".to_string();
    }
    let dev_assets =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ambition_content/assets");
    match dev_assets.canonicalize() {
        Ok(path) if path.is_dir() => path.to_string_lossy().into_owned(),
        _ => "assets".to_string(),
    }
}

/// A provider-owned `game://` source with a read-only fallback to the shared

/// The game's OWN asset source (`game://`): the content crate's assets dir in a
/// dev checkout, layered over the shared engine tree.
///
/// It is `ambition_asset_manager::consumer_source` now, so a third party's game can own its art the
/// same way Ambition's content crate owns its worlds, and there is ONE copy of the packaged-build
/// rule about not shadowing the platform reader.
#[cfg(not(target_arch = "wasm32"))]
fn game_asset_source_builder() -> bevy::asset::io::AssetSourceBuilder {
    ambition_platformer2d::asset_manager::consumer_source::layered_asset_source(
        game_asset_root(),
        desktop_asset_root(),
    )
}

/// True when no display server is reachable for `bevy_winit` to attach to.
/// Linux only — other platforms always return `false` and rely on Bevy's
/// own diagnostics. The check is conservative: any of `DISPLAY`,
/// `WAYLAND_DISPLAY`, or `WAYLAND_SOCKET` being set means we attempt the
/// visible path. If `--headless` was passed on the CLI, the caller has
/// already chosen the headless path and this check doesn't run.
///
/// The check intentionally skips wasm32 — the browser build has no env
/// vars to consult and would always trip the headless fallback otherwise.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn no_display_server_available() -> bool {
    if cfg!(not(target_os = "linux")) {
        return false;
    }
    std::env::var_os("DISPLAY").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_none()
        && std::env::var_os("WAYLAND_SOCKET").is_none()
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn cli_force_headless() -> bool {
    std::env::args().any(|arg| arg == "--headless")
}

#[cfg(not(target_arch = "wasm32"))]
fn cli_headless_acceptance_cycle() -> bool {
    std::env::args().any(|arg| arg == "--headless-acceptance-cycle")
}

#[cfg(not(target_arch = "wasm32"))]
fn args_request_headless_ticks(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--headless-ticks" || arg.starts_with("--headless-ticks="))
}

#[cfg(not(target_arch = "wasm32"))]
fn cli_headless_ticks_requested() -> bool {
    args_request_headless_ticks(&std::env::args().collect::<Vec<_>>())
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn cli_headless_ticks() -> u32 {
    let args: Vec<String> = std::env::args().collect();
    parse_headless_ticks(&args).unwrap_or(120)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn parse_headless_ticks(args: &[String]) -> Option<u32> {
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--headless-ticks" => return args.get(i + 1).and_then(|raw| raw.parse().ok()),
            arg if arg.starts_with("--headless-ticks=") => {
                return arg.trim_start_matches("--headless-ticks=").parse().ok();
            }
            _ => {}
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod headless_arg_tests {
    use super::{args_request_headless_ticks, parse_headless_ticks};

    fn args(slice: &[&str]) -> Vec<String> {
        slice.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_flag_returns_none() {
        assert_eq!(parse_headless_ticks(&args(&[])), None);
        assert_eq!(parse_headless_ticks(&args(&["--headless"])), None);
    }

    #[test]
    fn space_form() {
        assert_eq!(
            parse_headless_ticks(&args(&["--headless-ticks", "300"])),
            Some(300)
        );
    }

    #[test]
    fn requesting_a_tick_budget_implies_headless_mode() {
        assert!(args_request_headless_ticks(&args(&[
            "--headless-ticks",
            "120"
        ])));
        assert!(args_request_headless_ticks(&args(&[
            "--headless-ticks=120"
        ])));
        assert!(!args_request_headless_ticks(&args(&["--headless"])));
    }

    #[test]
    fn equals_form() {
        assert_eq!(
            parse_headless_ticks(&args(&["--headless-ticks=42"])),
            Some(42)
        );
    }

    /// In a dev checkout the desktop asset root must resolve to the
    /// machinery lib's asset tree (the bisection moved the binary's
    /// crate away from it) and actually contain the sandbox data —
    /// a wrong root reproduces the "game runs but nothing renders /
    /// no music / no dialogue" failure.
    #[test]
    fn desktop_asset_root_resolves_to_the_sandbox_tree_in_dev() {
        let root = std::path::PathBuf::from(super::desktop_asset_root());
        assert!(
            root.is_absolute(),
            "dev checkout should resolve an absolute sandbox assets path, got {root:?}"
        );
        assert!(
            root.ends_with("crates/ambition_platformer2d_actor_monolith/assets")
                || root.ends_with("assets")
        );
        assert!(
            root.join("ambition/platformer_defaults.ron").exists(),
            "asset root {root:?} must contain ambition/platformer_defaults.ron"
        );
        // (Dialogue no longer lives under the asset root — the yarn set is
        // CONTENT, embedded in-memory by ambition_content::dialogue::yarn.)
        assert!(
            root.join("sprites").is_dir(),
            "asset root {root:?} must contain the sprites/ tree"
        );
    }

    #[test]
    fn invalid_value_returns_none() {
        assert_eq!(
            parse_headless_ticks(&args(&["--headless-ticks", "abc"])),
            None
        );
    }
}

/// Build + run the visible Bevy app. The thin `fn main()` shim in
/// `src/main.rs` calls this.
///
/// Falls back to the headless simulation runner when no display server is
/// reachable (no `DISPLAY` / `WAYLAND_DISPLAY` on Linux), or when the
/// caller passes `--headless` on the CLI. The fallback path prints a
/// short diagnostic so users on a headless VM get a working
/// `cargo run` instead of a `bevy_winit` event-loop panic. Override the
/// number of ticks with `--headless-ticks N` (default 120).
#[cfg(not(target_arch = "wasm32"))]
pub fn run_visible() {
    // Wall-clock zero for `[startup]`, taken before any Bevy work. Anchoring it
    // later -- as the profiler resource's own creation used to -- hides plugin
    // construction, which is the larger half of startup.
    ambition_platformer2d::dev_tools::profiling::note_process_start();
    if cli_force_headless()
        || cli_headless_acceptance_cycle()
        || cli_headless_ticks_requested()
        || no_display_server_available()
    {
        let max_ticks = cli_headless_ticks();
        let reason = if cli_headless_acceptance_cycle() {
            "--headless-acceptance-cycle flag"
        } else if cli_force_headless() {
            "--headless flag"
        } else if cli_headless_ticks_requested() {
            "--headless-ticks flag"
        } else {
            "no DISPLAY / WAYLAND_DISPLAY env var"
        };
        if cli_direct_entry() {
            eprintln!("ambition_app: running the explicit direct sandbox headlessly ({reason})");
            match crate::headless::run_headless(max_ticks) {
                Ok(report) => {
                    println!("{report}");
                    return;
                }
                Err(error) => {
                    eprintln!("direct headless run failed: {error}");
                    std::process::exit(1);
                }
            }
        }
        eprintln!("ambition_app: running the production shared host headlessly ({reason})");
        if cli_headless_acceptance_cycle() {
            let report = run_shared_host_acceptance_cycle();
            println!("{report}");
            if !report.completed {
                std::process::exit(1);
            }
        } else {
            let report = run_shared_host_headless(max_ticks);
            println!("{report}");
        }
        return;
    }
    let shell_hosted = !cli_direct_entry();
    let mut app = build_visible_app(VisibleRenderMode::Windowed, shell_hosted);
    // The production windowed host opens on the "Powered by Ambition" card, then
    // hands off to the launcher. Direct entry and the headless ownership tests
    // (which call `build_visible_app` directly) deliberately skip it.
    if shell_hosted {
        super::shell_host::compose_ambition_startup_sequence(&mut app);
    }
    app.run();
}

/// Observable result of stepping the exact production shared-host composition
/// without a window or GPU backend.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SharedHostHeadlessReport {
    pub ticks_run: u32,
    pub active_route: Option<String>,
    pub launcher_active: bool,
    pub gameplay_session_active: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl std::fmt::Display for SharedHostHeadlessReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "shared host: {} tick(s), route={}, launcher={}, gameplay_session={}",
            self.ticks_run,
            self.active_route.as_deref().unwrap_or("<none>"),
            self.launcher_active,
            self.gameplay_session_active,
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub const SHARED_HOST_HEADLESS_TICK_HZ: f64 = 60.0;

/// Ticks needed for [`run_shared_host_headless`] to play the WHOLE startup
/// run-in and reach the launcher, plus a one-second margin.
///
/// Derived from the composed sequence rather than restated as a constant: the
/// run-in has already grown from one card to two, and a hardcoded budget silently
/// becomes "asserts the host is still on the first card" the moment it does.
#[cfg(not(target_arch = "wasm32"))]
pub fn shared_host_startup_ticks() -> u32 {
    let seconds = super::shell_host::ambition_startup_duration().as_secs_f64();
    (seconds * SHARED_HOST_HEADLESS_TICK_HZ).ceil() as u32 + SHARED_HOST_HEADLESS_TICK_HZ as u32
}

/// How many room preparations the neighbour prefetch has performed.
///
/// an instrument, exposed because the cost of FILLING the cache is the one thing its own
/// counters never described — `hits`/`misses`/`stale_misses` all answer "what did a transition
/// get out of it".
pub fn prefetch_preparations(world: &bevy::prelude::World) -> u64 {
    world
        .get_resource::<super::world_flow::room_transition_assets::RoomPreparationPrefetchState>()
        .map(|cache| cache.preparations)
        .unwrap_or_default()
}

/// Step the same multi-game host that the windowed binary ships, using Bevy's
/// no-window/no-backend presentation mode and deterministic frame time.
///
/// This is intentionally distinct from [`crate::headless::run_headless`], the
/// explicit direct-sandbox runner. Startup, launcher, providers, session bridge,
/// frontend audio context, and host-relative routing are all composed here.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_shared_host_headless(max_ticks: u32) -> SharedHostHeadlessReport {
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    // `build_visible_app` drops `LogPlugin` from `NoWindow` because a TEST
    // process builds several Apps and the tracing subscriber is process-global.
    // This is not that process: it is an executable host with exactly one App.
    // Tracy's recorder is a LAYER ON THAT SUBSCRIBER, so without it a
    // `--features profile` headless capture records zero zones — and per-system
    // timing is the measurement a machine with no GPU still has. Gated on the
    // profiling feature, so the no-window tests keep their silent composition.
    #[cfg(feature = "profile")]
    app.add_plugins(ambition_log_plugin());
    super::shell_host::compose_ambition_startup_sequence(&mut app);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / SHARED_HOST_HEADLESS_TICK_HZ,
    )));
    for _ in 0..max_ticks {
        app.update();
    }

    let world = app.world();
    let active_route = world
        .get_resource::<ambition_platformer2d::game_shell::ShellRouter>()
        .and_then(|router| router.active.as_ref())
        .map(|active| active.route_id.as_str().to_owned());
    let launcher_active = world
        .get_resource::<ambition_platformer2d::game_shell::ShellLauncherState>()
        .is_some_and(|launcher| launcher.active);
    let gameplay_session_active = world
        .get_resource::<ambition_platformer2d::game_shell::ActiveGameplaySession>()
        .is_some_and(|session| session.0.is_some());

    SharedHostHeadlessReport {
        ticks_run: max_ticks,
        active_route,
        launcher_active,
        gameplay_session_active,
    }
}

/// Result of the executable multi-provider shipping-host acceptance cycle.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SharedHostAcceptanceReport {
    pub completed: bool,
    pub route_stops: Vec<String>,
    pub title_zero_state_stops: u32,
    pub exit_requested: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl std::fmt::Display for SharedHostAcceptanceReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "shared-host acceptance: completed={}, zero-state-stops={}, exit={}, routes={}",
            self.completed,
            self.title_zero_state_stops,
            self.exit_requested,
            self.route_stops.join(" -> "),
        )
    }
}

/// Execute startup -> launcher -> Ambition -> launcher -> Sanic -> launcher ->
/// Mary-O -> launcher -> Sanic -> launcher -> Exit through the exact shipping
/// composition. This is exposed to `run_game.sh -- --headless-acceptance-cycle`.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_shared_host_acceptance_cycle() -> SharedHostAcceptanceReport {
    use ambition_platformer2d::game_shell::{
        ShellCommand, ShellLaunchCatalog, ShellLauncherCommand, ShellLauncherState, ShellRouter,
    };
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    fn active_route(app: &App) -> Option<&str> {
        app.world()
            .resource::<ShellRouter>()
            .active
            .as_ref()
            .map(|active| active.route_id.as_str())
    }

    fn step_until(app: &mut App, route: &str, budget: usize) -> bool {
        for _ in 0..budget {
            app.update();
            if active_route(app) == Some(route) {
                return true;
            }
        }
        false
    }

    fn title_is_zero_state(app: &App) -> bool {
        let world = app.world();
        world
            .resource::<ambition_platformer2d::game_shell::ActiveGameplaySession>()
            .0
            .is_none()
            && world
                .resource::<ambition_platformer2d::platformer::lifecycle::ActiveSessionScope>()
                .current()
                .is_none()
            && world
                .resource::<ambition_platformer2d::load::LoadCoordinator>()
                .is_empty()
            && world
                .resource::<ambition_platformer2d::game_shell::PreparedSessionRegistry>()
                .is_empty()
            && ambition_platformer2d::platformer::lifecycle::session_world_entity(world).is_none()
    }

    fn step_until_title_zero_state(app: &mut App, route: &str, budget: usize) -> bool {
        for _ in 0..budget {
            app.update();
            if active_route(app) == Some(route) && title_is_zero_state(app) {
                return true;
            }
        }
        false
    }

    fn select_launcher_route(app: &mut App, route: &str) -> bool {
        let target = app
            .world()
            .resource::<ShellLaunchCatalog>()
            .entries
            .iter()
            .filter(|entry| entry.available)
            .position(|entry| entry.route_id.as_str() == route);
        let Some(target) = target else {
            return false;
        };
        let selectable = app
            .world()
            .resource::<ShellLaunchCatalog>()
            .entries
            .iter()
            .filter(|entry| entry.available)
            .count()
            + usize::from(
                app.world()
                    .resource::<ambition_platformer2d::game_shell::ShellLauncherPresentation>()
                    .exit_label
                    .is_some(),
            );
        if selectable == 0 {
            return false;
        }
        for _ in 0..selectable {
            if app.world().resource::<ShellLauncherState>().selected == target {
                return true;
            }
            app.world_mut().write_message(ShellLauncherCommand::Next);
            app.update();
        }
        app.world().resource::<ShellLauncherState>().selected == target
    }

    fn select_launcher_exit(app: &mut App) -> bool {
        let target = app
            .world()
            .resource::<ShellLaunchCatalog>()
            .entries
            .iter()
            .filter(|entry| entry.available)
            .count();
        let has_exit = app
            .world()
            .resource::<ambition_platformer2d::game_shell::ShellLauncherPresentation>()
            .exit_label
            .is_some();
        if !has_exit {
            return false;
        }
        let selectable = target + 1;
        for _ in 0..selectable {
            if app.world().resource::<ShellLauncherState>().selected == target {
                return true;
            }
            app.world_mut().write_message(ShellLauncherCommand::Next);
            app.update();
        }
        app.world().resource::<ShellLauncherState>().selected == target
    }

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    super::shell_host::compose_ambition_startup_sequence(&mut app);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));

    let mut routes = Vec::new();
    let mut title_zero_state_stops = 0_u32;
    let launcher = super::shell_host::AMBITION_LAUNCHER_ROUTE;
    // The first hop plays the whole startup run-in before the launcher exists,
    // so budget it from the composed sequence rather than a constant that goes
    // stale the next time a card is added or retimed.
    let mut completed =
        step_until_title_zero_state(&mut app, launcher, shared_host_startup_ticks() as usize);
    if completed {
        routes.push(launcher.to_owned());
        title_zero_state_stops += 1;
    }

    for route in [
        super::shell_host::AMBITION_GAMEPLAY_ROUTE,
        "sanic_gameplay",
        "mary_o_gameplay",
        "sanic_gameplay",
    ] {
        if !completed {
            break;
        }
        completed = select_launcher_route(&mut app, route);
        if !completed {
            break;
        }
        app.world_mut()
            .write_message(ShellLauncherCommand::LaunchSelected);
        completed = step_until(&mut app, route, 90);
        if !completed {
            break;
        }
        routes.push(route.to_owned());
        app.world_mut().write_message(ShellCommand::QuitToHome);
        completed = step_until_title_zero_state(&mut app, launcher, 90);
        if completed {
            routes.push(launcher.to_owned());
            title_zero_state_stops += 1;
        }
    }

    if completed {
        completed = select_launcher_exit(&mut app);
        if completed {
            app.world_mut()
                .write_message(ShellLauncherCommand::LaunchSelected);
            for _ in 0..8 {
                app.update();
                if app.world().resource::<ShellRouter>().exit_requested {
                    break;
                }
            }
        }
    }
    let exit_requested = app.world().resource::<ShellRouter>().exit_requested;
    completed &= exit_requested && title_zero_state_stops == 5;

    SharedHostAcceptanceReport {
        completed,
        route_stops: routes,
        title_zero_state_stops,
        exit_requested,
    }
}

/// How [`build_visible_app`] creates its render surface.
///
/// Desktop-only: the browser build has exactly one surface (its `<canvas>`), so
/// `run_web` composes plugins directly rather than selecting a mode.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VisibleRenderMode {
    /// A real desktop window.
    Windowed,
    /// The full render graph with NO window and NO wgpu backend — the
    /// standard Bevy recipe for exercising the real presentation composition
    /// in tests/CI without a GPU or display server.
    ///
    /// `backends: None` OMITS THE RENDER APP. Nothing is ever drawn, so
    /// anything that needs pixels to exist — a screenshot, a readback, the
    /// tilemap spine — cannot work here and cannot be made to. That is not a
    /// limitation to route around; it is what this mode is. Use
    /// [`Self::OffscreenGpu`] when the output is an IMAGE.
    NoWindow,
    /// No window, but a REAL wgpu backend — so there IS a render app, the
    /// render graph runs, and a readback can complete.
    ///
    /// The difference from [`Self::NoWindow`] is one field (`backends`) and it
    /// is the whole difference between "composes the presentation" and
    /// "produces pixels". `capture_scene` has always built this shape by hand
    /// for ROOMS; naming it here is what lets a shell ROUTE — the launcher, the
    /// title cards, the versus stage and its HUD — be photographed through the
    /// same composition a player runs, which is the thing the room-only capture
    /// tool could never reach.
    ///
    /// Needs a working wgpu adapter, software or otherwise. A machine without
    /// one should use `NoWindow` and expect no image.
    OffscreenGpu,
}

/// Assemble the visible Ambition app — the ONE composition the desktop binary
/// runs and the rendered ownership tests drive.
///
/// `shell_hosted` no longer means what its name says, and the name is kept
/// on purpose. Since K2b both arms ARE shell-hosted; the flag only chooses the
/// INITIAL ROUTE — `true` boots the multi-game launcher, `false` boots straight
/// to gameplay, which is what `--direct` and every `--start-room` alias mean.
/// Renaming it would touch 33 call sites to restate a boolean whose two values
/// are unchanged, which is churn rather than clarity; this sentence is the fix.
///
/// Desktop-only: it reaches the filesystem asset root and the `game://` source
/// builder (both `not(wasm32)`) and disables the terminal Ctrl-C handler. Every
/// caller (`run_visible`, the shared-host headless + acceptance runners) is
/// already `not(wasm32)`; the browser entry is `run_web`.
#[cfg(not(target_arch = "wasm32"))]
pub fn build_visible_app(render: VisibleRenderMode, shell_hosted: bool) -> App {
    build_visible_app_with(render, shell_hosted, |_| {})
}

/// [`build_visible_app`], plus the ONE moment a caller can reach: after the App
/// exists, before the simulation plugin builds.
///
/// this exists because the alternative was a second app builder, and that fork cost five
/// bugs. `StartRoomOverride`, `StartRoomMustResolve`, `StartingCharacterOverride` and
/// `SeatsAMatchInsteadOfAHomeBody` are COMPOSITION INPUTS: `init_sandbox_resources` removes
/// them while the simulation plugin builds, so a caller who wants to set one has to write it
/// into a world that already exists and has not yet built that plugin. There was no such
/// moment.
///
/// a closure rather than a struct of known inputs. A struct would have to
/// enumerate the composition inputs, and the fifth one added elsewhere would not
/// be reachable here — which is the same "a caller cannot say this" hole, one
/// release later. The hook says *when*, and the resources say *what*.
///
/// It runs AFTER [`insert_starting_character_override`] reads
/// `AMBITION_START_CHARACTER`, so an explicit caller wins over the environment,
/// and after `AmbitionShellHosted`, so nothing here can be undone by the builder.
#[cfg(not(target_arch = "wasm32"))]
/// The log filter this game runs with, and where a person changes it.
///
/// ⭐⭐ THE DEFAULT NAMES THE GAME'S OWN CHANNELS. Bevy's default filter is about
/// Bevy — it quiets `wgpu` and `naga` and says nothing about us — so the
/// engine's own diagnostic targets were on only for whoever knew to export
/// `RUST_LOG`. Jon, after a bug that took a play session to pin down: *"Can we
/// also make those mount and moves info logs enabled by default? … or maybe a
/// config file somewhere where it is very easy to set what you want the defaults
/// of the logging to be."* A log nobody can see by default is not a diagnostic.
///
/// ⭐ THREE PLACES, MOST SPECIFIC WINS:
///
/// 1. `RUST_LOG` — the one-off, unchanged from what everybody already expects.
/// 2. `log_filter.txt` at the repo/working root — the durable per-machine
///    answer, one line, no format to learn.
/// 3. [`DEFAULT_LOG_FILTER`] — what ships.
///
/// ⛔ A MISSING FILE IS NOT AN ERROR, and neither is an unreadable one: logging
/// configuration must never be a reason the game does not start.
const DEFAULT_LOG_FILTER: &str = concat!(
    "info,wgpu=error,naga=warn,",
    // The game's own channels, on by default because they exist to be read.
    "ambition::mount=info,ambition::moves=info",
);

/// Where a person writes their own filter. One line, `EnvFilter` syntax.
const LOG_FILTER_FILE: &str = "log_filter.txt";

fn resolved_log_filter() -> String {
    if let Ok(from_env) = std::env::var("RUST_LOG") {
        if !from_env.trim().is_empty() {
            return from_env;
        }
    }
    if let Ok(text) = std::fs::read_to_string(LOG_FILTER_FILE) {
        // Comments and blank lines, so the file can explain itself.
        let filter: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();
        if !filter.is_empty() {
            return filter.join(",");
        }
    }
    DEFAULT_LOG_FILTER.to_string()
}

/// The log plugin this game's compositions install. See [`DEFAULT_LOG_FILTER`].
pub(crate) fn ambition_log_plugin() -> bevy::log::LogPlugin {
    bevy::log::LogPlugin {
        filter: resolved_log_filter(),
        ..bevy::log::LogPlugin::default()
    }
}

pub fn build_visible_app_with(
    render: VisibleRenderMode,
    shell_hosted: bool,
    compose_inputs: impl FnOnce(&mut App),
) -> App {
    let asset_config = GameAssetConfig::from_args();
    let asset_root = desktop_asset_root();
    eprintln!("ambition_app: asset root = {asset_root}");
    let mut app = App::new();
    // The simulation host, the boot curtain, and every other game-side choice
    // are made once for all platforms by `compose_ambition_visible_game` at the
    // end of this function. What is decided HERE is what a desktop host uniquely
    // answers for: which surface exists, and which side effects a non-session
    // process must not have.
    let direct_windowed = matches!(render, VisibleRenderMode::Windowed) && !shell_hosted;
    if matches!(
        render,
        VisibleRenderMode::NoWindow | VisibleRenderMode::OffscreenGpu
    ) {
        // Automated no-window hosts exercise the real ownership, resolver, and
        // playback-state path, but the final output side effect is recorded
        // instead of issuing Kira `play` commands to the user's speakers.
        app.insert_resource(ambition_platformer2d::audio::AudioOutputMode::Recording);
        // ...and the SAME rule for the other side effect a non-session App must not have: writing
        // the user's settings and save.
        //
        // that directory is per-USER, not per-checkout: every `app_it` test
        // shared three mutable files with every other test, every other
        // worktree, and every concurrent session on the machine. A headless
        // acceptance run could overwrite a real save.
        app.insert_resource(ambition_platformer2d::persistence::PersistenceRoot::isolated());
        // A windowless host has no display to pace against, so "real time" is not a thing it is
        // synchronising to — it is just whatever the machine was doing.
        //
        // `dev/journals/code_smells.md` already states the lesson, and stating a lesson is what
        // a rule does instead of enforcing it.
        //
        // the same shape as the two above: a non-session App must not have
        // the side effect, so the HOST removes it once rather than 42 call sites
        // remembering to. A test that wants a different dt still inserts its own
        // — this is a default, not a lock.
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ));
    }
    // The game's OWN asset source (`game://`): the content crate's assets
    // dir in a dev checkout, the shipped `assets/` dir otherwise. The
    // WorldManifest rows address their .ldtk files through it, so the
    // tile-render spine loads content-owned files without the engine's
    // asset root ever containing a world. Must register before
    // DefaultPlugins builds AssetPlugin.
    app.register_asset_source("game", game_asset_source_builder());
    let plugins = DefaultPlugins
        .set(ambition_log_plugin())
        .set(bevy::asset::AssetPlugin {
            // See `desktop_asset_root`: post-bisection the binary's
            // crate has no assets/ tree; the canonical one lives with
            // the machinery lib.
            file_path: asset_root,
            ..default()
        });
    match render {
        VisibleRenderMode::Windowed => {
            app.add_plugins(plugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Ambition - Tangent Space Sandbox (Bevy)".into(),
                    resolution: WindowResolution::new(WINDOW_W, WINDOW_H),
                    // Direct entry exposes the window only after Startup has
                    // built an opaque loading surface. This prevents the OS
                    // compositor from briefly showing stale desktop pixels or
                    // an uninitialized swapchain. Shell-hosted startup keeps
                    // its existing visible route presentation.
                    visible: !direct_windowed,
                    resizable: true,
                    resize_constraints: WindowResizeConstraints {
                        min_width: 640.0,
                        min_height: 360.0,
                        ..default()
                    },
                    ..default()
                }),
                ..default()
            }));
        }
        VisibleRenderMode::NoWindow => {
            use bevy::render::settings::{RenderCreation, WgpuSettings};
            use bevy::render::RenderPlugin;
            use bevy::window::ExitCondition;
            app.add_plugins(
                plugins
                    // Tests build several Apps per process; the tracing
                    // subscriber is process-global.
                    .disable::<bevy::log::LogPlugin>()
                    // A test process builds several Apps. Ctrl+C ownership is
                    // process-global and belongs to executable hosts, not these
                    // manually stepped no-window Apps.
                    .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
                    // `backends: None` omits the RenderApp; disable plugins whose
                    // only job is to register extraction/render work there.
                    .disable::<bevy::core_pipeline::CorePipelinePlugin>()
                    .disable::<bevy::gizmos_render::GizmoRenderPlugin>()
                    .set(RenderPlugin {
                        render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                            backends: None,
                            ..default()
                        })),
                        ..default()
                    })
                    .set(WindowPlugin {
                        primary_window: None,
                        exit_condition: ExitCondition::DontExit,
                        close_when_requested: false,
                        ..default()
                    })
                    // No window means no event loop; tests run off the main
                    // thread where winit refuses to initialize one.
                    .disable::<bevy::winit::WinitPlugin>(),
            );
        }
        VisibleRenderMode::OffscreenGpu => {
            use bevy::app::ScheduleRunnerPlugin;
            use bevy::window::ExitCondition;
            // The SAME recipe as NoWindow minus the one field that matters:
            // no `backends: None`, so wgpu picks a real adapter, the render app
            // exists, and a readback can complete. `CorePipelinePlugin` and
            // `GizmoRenderPlugin` stay ENABLED for the same reason — their work
            // has somewhere to go.
            app.add_plugins(
                plugins
                    .disable::<bevy::log::LogPlugin>()
                    .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
                    .set(WindowPlugin {
                        primary_window: None,
                        exit_condition: ExitCondition::DontExit,
                        close_when_requested: false,
                        ..default()
                    })
                    .disable::<bevy::winit::WinitPlugin>(),
            );
            // Winit is the loop runner for a windowed app; without it `run()`
            // executes ONE update and exits, having drawn nothing. That is the
            // first thing the Z1 attempt tripped over, and it failed silently.
            app.add_plugins(ScheduleRunnerPlugin::run_loop(
                std::time::Duration::from_millis(0),
            ));
        }
    }
    // AND NOW THE PART THAT IS NOT DESKTOP BUSINESS. Everything above
    // chose a render surface; everything below is the game, and it is the SAME
    // game the browser runs. See `visible_composition` for why that is one
    // function and not a passage repeated per host.
    //
    // Host mode: the shell-routed multi-game title screen is the DEFAULT. Direct
    // development entry (straight into gameplay, no launcher) is host
    // configuration: `--direct`, or any explicit start-room request (the
    // run_game.sh mode aliases pass `--start-room`, and their intent is to land
    // in that room immediately).
    super::visible_composition::compose_ambition_visible_game(
        &mut app,
        super::visible_composition::VisibleGameSpec {
            shell_hosted,
            // bevy_ecs_tilemap (inside LdtkPlugin) requires a RenderApp, which
            // the `backends: None` no-window recipe deliberately omits. Ambition's
            // own room visuals are ordinary sprites and still draw; only the
            // painted LDtk tile spine is absent in that mode, and the session
            // LDtk roots guard on the asset registry so nothing dangles.
            tile_spine: !matches!(render, VisibleRenderMode::NoWindow),
            startup_loading_curtain: direct_windowed,
            asset_config,
        },
        compose_inputs,
    );
    app
}

/// True when this process should boot straight into gameplay (the pre-shell
/// behavior): `--direct`, `AMBITION_DIRECT=1`, or an explicit start-room
/// request.
///
/// Desktop-only, like every other reader of the command line: a browser has no
/// argv to carry a development entry flag, so `run_web` never asks.
#[cfg(not(target_arch = "wasm32"))]
fn cli_direct_entry() -> bool {
    if std::env::var("AMBITION_DIRECT").is_ok_and(|v| v == "1") {
        return true;
    }
    std::env::args().any(|arg| {
        arg == "--direct"
            || arg == "--start-room"
            || arg.starts_with("--start-room=")
            || arg == "--room"
            || arg.starts_with("--room=")
    })
}

/// Build + run the visible Bevy app for a browser (wasm32) target.
///
/// This function is a PLATFORM FOUNDATION and nothing else. It answers the
/// three questions a browser host uniquely owns — which surface (the page's
/// `<canvas>`), which asset profile (the one its Cargo feature set was built
/// for), and how the app runs — and then hands off to
/// [`super::visible_composition::compose_ambition_visible_game`], the same
/// function the desktop builder calls.
///
/// A build gate proves *links*; nothing there proved *composes*.
///
/// It still bypasses every desktop-only branch in [`run_visible`]: no CLI
/// parsing (`std::env::args` is empty in the browser), no `DISPLAY` / Wayland
/// probe, and no headless fallback (the browser has no terminal to print to and
/// `process::exit` traps).
///
/// Audio, dev tools, the file watcher, mobile touch, and physics debris are
/// controlled by the Cargo feature set — build with
/// `--no-default-features --features web` (embedded core assets) or
/// `--features web_served_assets` (the full game over HTTP).
///
/// The `#[wasm_bindgen(start)]` shim that calls this lives in
/// `ambition_app::lib`'s root, behind the same `cfg(target_arch = "wasm32")` +
/// `feature = "web_platform"` gate.
#[cfg(all(target_arch = "wasm32", feature = "web_platform"))]
pub fn run_web() {
    let mut app = App::new();
    // THE `game://` SOURCE WAS NEVER REGISTERED HERE. The world manifest
    // addresses every `.ldtk` file as `game://worlds/<file>` (and the vanity card
    // its own art the same way), so on the browser those loads resolved through
    // a source that did not exist. `static_map` hid it for the worlds — the
    // embedded fallback answered instead — and nothing hid it for anything else.
    //
    // the two roots are ONE root here, and this is the platform default the
    // engine's layering rule already reduces to. `layered_asset_source`
    // documents that equal roots return `AssetSourceBuilder::platform_default`
    // UNCHANGED, and that the equality is load-bearing rather than an
    // optimisation: a packaged build — an APK, a Steam Deck install, a served
    // web tree — has had its roots MERGED BY THE PACKAGER already (here, by
    // `package_asset_guard.py compose`, which publishes `web/assets/`), so
    // there is nothing left to fall back TO and the platform reader is the
    // correct one. On wasm that reader is Bevy's HTTP reader, fetching
    // `/assets/<path>` from the page origin for both sources.
    //
    // spelled as the platform default rather than as `layered_asset_source`
    // because that function is `not(target_arch = "wasm32")` — it is built on
    // `FileAssetReader`, which a browser does not have. The rule is the same;
    // only the half of it that needs a filesystem is absent here.
    //
    // Must register before DefaultPlugins builds AssetPlugin.
    app.register_asset_source(
        "game",
        bevy::asset::io::AssetSourceBuilder::platform_default("assets", None),
    );
    app.add_plugins(
        DefaultPlugins
            .set(ambition_log_plugin())
            .set(bevy::asset::AssetPlugin {
                // NEVER PROBE FOR `.meta`, AND THIS IS NOT LOG HYGIENE.
                //
                // Bevy's default `AssetMetaCheck::Always` asks for `<path>.meta`
                // before every asset. This repo contains ZERO `.meta` files under
                // either asset root and generates none, so every one of those probes
                // is a request that cannot succeed — on the desktop a cheap failed
                // stat, in a browser a full HTTP round trip that 404s.
                //
                // the day this repo starts SHIPPING processed assets with meta
                // sidecars, this is the line that has to change — and the absence of
                // any `.meta` file is what makes it safe today, not a preference.
                meta_check: bevy::asset::AssetMetaCheck::Never,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Ambition - Tangent Space Sandbox (Web)".into(),
                    // The canvas selector matches `<canvas id="bevy">` in
                    // `game/ambition_app/web/index.html`. Without this Bevy
                    // would mint its own canvas and append it to <body>; pinning
                    // the selector lets the page own layout / sizing / focus.
                    canvas: Some("#bevy".to_string()),
                    // Resize the canvas to follow its CSS-styled parent. The
                    // template wraps the canvas in a full-viewport flexbox parent
                    // so this fills the page without needing a JS resize observer.
                    fit_canvas_to_parent: true,
                    // Don't let the canvas swallow the browser's own keyboard
                    // shortcuts; the page wants the user to be able to refresh /
                    // open devtools without leaving it.
                    prevent_default_event_handling: false,
                    ..default()
                }),
                ..default()
            }),
    );
    // GameAssetConfig defaults match the no-args desktop path — no
    // `std::env::args` parsing on the web because the browser provides
    // none and the helper hits stdlib paths that don't exist on wasm.
    let asset_config = GameAssetConfig::default();
    // One-line boot banner so anyone opening browser devtools can see
    // which asset profile + feature bundle this wasm artifact was
    // built with. Particularly useful when diagnosing
    // "why is everything a colored rectangle?" — the answer is almost
    // always "the build does not have `static_core_assets`."
    bevy::log::info!(
        target: "ambition_platformer2d::platformer_assets",
        "web start: AssetProfile = {} | static_map = {} | static_core_assets = {} | static_sfx_bank = {}",
        asset_config.asset_profile.label(),
        cfg!(feature = "static_map"),
        cfg!(feature = "static_core_assets"),
        cfg!(feature = "static_sfx_bank"),
    );
    super::visible_composition::compose_ambition_visible_game(
        &mut app,
        super::visible_composition::VisibleGameSpec::browser(asset_config),
        |_| {},
    );
    app.run();
}

/// Parse + validate the embedded LDtk world, build the `RoomSet`, and insert
/// the sim-required resources both visible and headless binaries need.
///
/// Both binaries call this after registering Bevy's plugin foundation
/// (DefaultPlugins or MinimalPlugins + AssetPlugin + StatesPlugin +
/// `init_state::<GameMode>`) and before the App-builder helpers.
///
/// Exits with status 2 on LDtk validation errors — invalid sandbox content
/// is a hard error per the LDtk authoring rules (see ADR 0009 + LDtk
/// authoring memory).
pub(super) fn cli_start_room_arg() -> Option<String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    parse_start_room_arg(&args)
}

pub(super) fn parse_start_room_arg(args: &[String]) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--start-room" | "--room" => {
                return args.get(i + 1).cloned();
            }
            arg if arg.starts_with("--start-room=") => {
                return Some(arg.trim_start_matches("--start-room=").to_string());
            }
            arg if arg.starts_with("--room=") => {
                return Some(arg.trim_start_matches("--room=").to_string());
            }
            _ => {}
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod cli_arg_tests {
    use super::parse_start_room_arg;

    fn args(slice: &[&str]) -> Vec<String> {
        slice.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_start_room_flag_returns_none() {
        assert_eq!(parse_start_room_arg(&args(&[])), None);
        assert_eq!(parse_start_room_arg(&args(&["--no-assets"])), None);
    }

    #[test]
    fn start_room_space_form() {
        assert_eq!(
            parse_start_room_arg(&args(&["--start-room", "goblin_encounter"])),
            Some("goblin_encounter".to_string())
        );
        assert_eq!(
            parse_start_room_arg(&args(&["--room", "central_hub_main"])),
            Some("central_hub_main".to_string())
        );
    }

    #[test]
    fn start_room_equals_form() {
        assert_eq!(
            parse_start_room_arg(&args(&["--start-room=water_world"])),
            Some("water_world".to_string())
        );
        assert_eq!(
            parse_start_room_arg(&args(&["--room=basement_boss"])),
            Some("basement_boss".to_string())
        );
    }

    #[test]
    fn start_room_first_match_wins() {
        // If both --start-room and --room are provided, the first one
        // in arg order wins. Bevy's own arg parsing leaves both alone.
        assert_eq!(
            parse_start_room_arg(&args(&["--room", "a", "--start-room", "b"])),
            Some("a".to_string())
        );
    }

    #[test]
    fn start_room_without_value_returns_none() {
        // Trailing flag with no value: don't crash, just return None.
        assert_eq!(parse_start_room_arg(&args(&["--start-room"])), None);
    }
}
