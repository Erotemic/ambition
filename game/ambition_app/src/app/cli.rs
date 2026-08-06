use bevy::prelude::*;
// Fixed-resolution desktop window sizing — only the native `build_visible_app`
// sets it; the web build sizes its canvas from CSS.
#[cfg(not(target_arch = "wasm32"))]
use bevy::window::WindowResolution;

#[cfg(not(target_arch = "wasm32"))]
use ambition_platformer2d::engine_core::config::{WINDOW_H, WINDOW_W};
use ambition_platformer2d::sprite_sheet::game_assets::GameAssetConfig;

use super::plugins::{
    AmbitionGameLdtkRuntimePlugin, AmbitionGamePresentationPlugin, AmbitionGameSimulationPlugin,
};

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
/// The reader that spans two roots used to live here, ~120 lines of it, where
/// nothing outside this shell could reach it — which is recorded SDK leak #3:
/// "consumer-owned art still has no home". It is
/// `ambition_asset_manager::consumer_source` now, so a third party's game can
/// own its art the same way Ambition's content crate owns its worlds, and there
/// is ONE copy of the packaged-build rule about not shadowing the platform
/// reader.
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

/// Fixed timestep [`run_shared_host_headless`] advances per tick.
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
    /// ⚠ `backends: None` OMITS THE RENDER APP. Nothing is ever drawn, so
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
    /// tool could never reach (queue Z1).
    ///
    /// Needs a working wgpu adapter, software or otherwise. A machine without
    /// one should use `NoWindow` and expect no image.
    OffscreenGpu,
}

/// Assemble the visible Ambition app — the ONE composition the desktop binary
/// runs and the rendered ownership tests drive.
///
/// ⚠ **`shell_hosted` no longer means what its name says, and the name is kept
/// on purpose.** Since K2b both arms ARE shell-hosted; the flag only chooses the
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
    let asset_config = GameAssetConfig::from_args();
    let asset_root = desktop_asset_root();
    eprintln!("ambition_app: asset root = {asset_root}");
    let mut app = App::new();
    {
        // ⭐ **NO LONGER GATED ON `dev_tools`.** Bevy seals `SimSchedule` when the
        // first simulation plugin registers, so the host is chosen here for the
        // whole build. It used to be chosen inside `#[cfg(feature = "dev_tools")]`
        // — not a developer convenience but a COUPLING: the only thing that
        // installed a GGRS session was the dev observatory, and a GGRS host with
        // no session composes, boots, renders and never simulates. The engine
        // owns the session now (`runtime::rollback::local_session`), so every
        // build can have one.
        //
        // Ordinary play runs a zero-distance baseline: GGRS drives the simulation
        // deterministically and rollback stays dormant. F9 raises the check
        // distance for one bounded proof pulse and drops it back.
        use ambition_platformer2d::runtime::SimulationHostAppExt as _;
        app.set_simulation_host(ambition_platformer2d::runtime::SimulationHost::Ggrs);
    }
    let direct_windowed = matches!(render, VisibleRenderMode::Windowed) && !shell_hosted;
    if direct_windowed {
        app.insert_resource(
            ambition_platformer2d::platformer::lifecycle::InitialGameplayReadiness::closed(),
        );
    }
    if matches!(
        render,
        VisibleRenderMode::NoWindow | VisibleRenderMode::OffscreenGpu
    ) {
        // Automated no-window hosts exercise the real ownership, resolver, and
        // playback-state path, but the final output side effect is recorded
        // instead of issuing Kira `play` commands to the user's speakers.
        app.insert_resource(ambition_platformer2d::audio::AudioOutputMode::Recording);
        // ...and the SAME rule for the other side effect a non-session App must
        // not have: writing the user's settings and save. A windowless host is a
        // test, a capture, or a headless acceptance run — none of them is a
        // player, and all of them used to read and write
        // `~/.local/share/ambition/` because the path was resolved from the
        // environment rather than owned by the App.
        //
        // ⚠ that directory is per-USER, not per-checkout: every `app_it` test
        // shared three mutable files with every other test, every other
        // worktree, and every concurrent session on the machine. A headless
        // acceptance run could overwrite a real save.
        app.insert_resource(ambition_platformer2d::persistence::PersistenceRoot::isolated());
        // ⛔ **...and the clock, for the THIRD side effect of the same kind.**
        // Bevy's default `TimeUpdateStrategy::Automatic` advances the clock by
        // REAL elapsed time, so `app.update()` is a unit of wall clock rather
        // than of simulation: almost no movement on an idle machine, many fixed
        // steps under load. A windowless host has no display to pace against, so
        // "real time" is not a thing it is synchronising to — it is just
        // whatever the machine was doing.
        //
        // ⚠ **this defect has now landed FOUR times**, and the fourth is why the
        // rule moved here. `shell_host_startup` pins for it; `shell_host_rendered`
        // was fixed for it; `smash_in_the_host` was written without it and failed
        // only under concurrent load — two full `app_it` runs at once fail in
        // BOTH processes, every time, while three sequential runs are green.
        // `dev/journals/code_smells.md` already states the lesson, and stating a
        // lesson is what a rule does instead of enforcing it.
        //
        // ⭐ **the same shape as the two above**: a non-session App must not have
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
    let plugins = DefaultPlugins.set(bevy::asset::AssetPlugin {
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
                        render_creation: RenderCreation::Automatic(WgpuSettings {
                            backends: None,
                            ..default()
                        }),
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
    // DefaultPlugins installs StatesPlugin, so initialize GameMode after it.
    ambition_platformer2d::runtime::init_engine_states(&mut app);
    // Main-world frame schedules run serially: headless measurement showed
    // gameplay bodies at <2% of CPU vs ~40% executor bookkeeping + thread
    // parking (3.7x wall, 32x fewer context switches — see
    // serialize_frame_schedules). The render sub-app keeps its own parallel
    // schedules; this only serializes main-world dispatch.
    ambition_platformer2d::runtime::serialize_frame_schedules(&mut app);
    let active_profile = asset_config.asset_profile;
    app.insert_resource(asset_config);
    // Launch-time "choose your character": inserted BEFORE the plugins so the
    // sandbox preparation consumes it before publishing session authority.
    insert_starting_character_override(&mut app);
    // Host mode: the shell-routed multi-game title screen is the DEFAULT.
    // Direct development entry (straight into gameplay, no launcher) is host
    // configuration: `--direct`, or any explicit start-room request (the
    // run_game.sh mode aliases pass `--start-room`, and their intent is to
    // land in that room immediately).
    // ⭐ **K2b edit 1: the shell host is composed EITHER WAY**, and the mode
    // only decides which route it boots into. Direct entry stops being a second
    // way to build a game and becomes what `tracks.md` says it should be — *a
    // shell host whose initial route is the gameplay route*, the recipe
    // `ambition_demo_sanic_app` already proves.
    //
    // ⚠ this resource must be inserted BEFORE the sim plugins build: it is what
    // `publish_direct_prepared_session_root` checks, and without it the app
    // carries the build-time root AND the activation's, which is two canonical
    // roots and a panic on the first read.
    app.insert_resource(super::shell_host::AmbitionShellHosted);
    match render {
        VisibleRenderMode::Windowed => {
            app.add_plugins((
                AmbitionGameSimulationPlugin,
                AmbitionGameLdtkRuntimePlugin,
                AmbitionGamePresentationPlugin,
            ));
        }
        VisibleRenderMode::NoWindow => {
            // bevy_ecs_tilemap (inside LdtkPlugin) requires a RenderApp, which
            // the no-backend recipe deliberately omits. Ambition's own room
            // visuals are ordinary sprites and still draw; only the painted
            // LDtk tile spine is absent in this mode. The session LDtk roots
            // guard on the asset registry so nothing dangles.
            app.add_plugins((AmbitionGameSimulationPlugin, AmbitionGamePresentationPlugin));
        }
        VisibleRenderMode::OffscreenGpu => {
            // The FULL set, tile spine included — this mode has a render app,
            // which is the whole reason it exists. A capture that quietly
            // dropped the painted tiles would be a photograph of a different
            // game than the one a player runs.
            app.add_plugins((
                AmbitionGameSimulationPlugin,
                AmbitionGameLdtkRuntimePlugin,
                AmbitionGamePresentationPlugin,
            ));
        }
    }
    if shell_hosted {
        super::shell_host::compose_ambition_shell_host(&mut app);
    } else {
        // Straight to gameplay, no launcher and no vanity run-in — which is what
        // `--direct` and every `--start-room` alias mean.
        super::shell_host::compose_ambition_shell_host_booting_to(
            &mut app,
            super::shell_host::AMBITION_GAMEPLAY_ROUTE,
        );
    }
    super::shell_host::install_ambition_shell_visuals(&mut app);
    if direct_windowed {
        super::startup_loading::install_direct_startup_loading(&mut app);
    }
    // AssetSource registration runs LAST so EmbeddedAssetRegistry
    // (added by `AssetPlugin` inside `DefaultPlugins`) is already present.
    app.add_plugins(
        ambition_platformer2d::actors::assets::platformer_assets::AmbitionAssetSourcePlugin::for_profile(
            active_profile,
            &ambition_content::worlds::world_manifest(),
        ),
    );
    app
}

/// True when this process should boot straight into gameplay (the pre-shell
/// behavior): `--direct`, `AMBITION_DIRECT=1`, or an explicit start-room
/// request.
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

/// Read an optional starting-character override from the
/// `AMBITION_START_CHARACTER` env var. When set to a non-empty
/// `character_catalog.ron` id, the local player spawns AS that character —
/// its sprite, combat moveset, and name — instead of the default protagonist.
/// This is the launch-time surface behind Jon's "choose your character" ask
/// (`AMBITION_START_CHARACTER=goblin cargo run -p ambition_app`); an in-game
/// selection menu is the natural follow-up. Unknown ids still spawn a fully
/// controllable player (the sprite falls back to the colored rectangle).
fn insert_starting_character_override(app: &mut App) {
    let Ok(raw) = std::env::var("AMBITION_START_CHARACTER") else {
        return;
    };
    let id = raw.trim();
    if id.is_empty() {
        return;
    }
    eprintln!("ambition_app: starting as character '{id}' (AMBITION_START_CHARACTER)");
    app.insert_resource(super::resources::StartingCharacterOverride(
        ambition_platformer2d::actors::avatar::StartingCharacter::new(id),
    ));
}

/// Build + run the visible Bevy app for a browser (wasm32) target.
///
/// Bypasses every desktop-only branch in [`run_visible`]: no CLI parsing
/// (`std::env::args` is empty in the browser), no `DISPLAY` / Wayland probe,
/// and no headless fallback (the browser has no terminal to print to and
/// `process::exit` traps). The window is attached to the `#bevy` canvas
/// from `web/index.html` and uses the same sandbox plugin trio the desktop
/// build composes.
///
/// First-pass: audio, dev tools, file watcher, mobile touch, and physics
/// debris are intentionally OFF (controlled by the Cargo feature set —
/// build with `--no-default-features --features web`). LDtk loads via the
/// embedded `static_map` fallback because the wasm build has no working
/// synchronous filesystem reader for `sandbox.ldtk` in this pass.
///
/// The `#[wasm_bindgen(start)]` shim that calls this lives in
/// `ambition_app::lib`'s root, behind the same `cfg(target_arch = "wasm32")` +
/// `feature = "web_platform"` gate.
#[cfg(all(target_arch = "wasm32", feature = "web_platform"))]
pub fn run_web() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
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
            // shortcuts; first-pass build wants the user to be able to
            // refresh / open devtools without leaving the page.
            prevent_default_event_handling: false,
            ..default()
        }),
        ..default()
    }));
    // DefaultPlugins installs StatesPlugin, so initialize GameMode after it.
    ambition_platformer2d::runtime::init_engine_states(&mut app);
    // wasm has one thread; the multithreaded executor's bookkeeping is pure
    // overhead there. Same measured rationale as the desktop adoption.
    ambition_platformer2d::runtime::serialize_frame_schedules(&mut app);
    // GameAssetConfig defaults match the no-args desktop path — no
    // `std::env::args` parsing on the web because the browser provides
    // none and the helper hits stdlib paths that don't exist on wasm.
    let asset_config = GameAssetConfig::default();
    let active_profile = asset_config.asset_profile;
    // One-line boot banner so anyone opening browser devtools can see
    // which asset profile + feature bundle this wasm artifact was
    // built with. Particularly useful when diagnosing
    // "why is everything a colored rectangle?" — the answer is almost
    // always "the build does not have `static_core_assets`."
    bevy::log::info!(
        target: "ambition_platformer2d::platformer_assets",
        "web start: AssetProfile = {} | static_map = {} | static_core_assets = {} | static_sfx_bank = {}",
        active_profile.label(),
        cfg!(feature = "static_map"),
        cfg!(feature = "static_core_assets"),
        cfg!(feature = "static_sfx_bank"),
    );
    app.insert_resource(asset_config);
    // Launch-time starting-character override (no-op on wasm: env reads Err).
    insert_starting_character_override(&mut app);
    // The browser has exactly one surface — the `<canvas>` configured on the
    // WindowPlugin above — so install the windowed web composition directly.
    // (repair_wasm.md failure #5: this used to `match render`, a variable copied
    // from the native `build_visible_app` builder that never existed here.)
    // ⭐ **THE SAME SIMULATION HOST AS THE DESKTOP BUILD** (Jon, 2026-08-03: *"the
    // web build is another deployment of the game so likely needs ggrs if
    // multiplayer is ever gonna be a real thing"*).
    //
    // ⛔ this entry used to set NO host, so it fell to the render-frame default
    // and the browser stepped the simulation once per RENDER FRAME with the real
    // frame delta — `refresh_world_time` reads the schedule-local `Res<Time>`.
    // A platformer's feel is a function of its timestep, so the same game had a
    // different jump arc in a browser than on a desktop, and at 144 Hz than at 60.
    //
    // ⚠ this is only safe because the ENGINE now owns the local GGRS session
    // (`runtime::rollback::local_session`). While the only installer was the dev
    // observatory, choosing this host outside `dev_tools` produced a build that
    // composed, booted, rendered and never simulated.
    //
    // Must precede the first simulation plugin: Bevy seals `SimSchedule` on the
    // first read.
    {
        use ambition_platformer2d::runtime::SimulationHostAppExt as _;
        app.set_simulation_host(ambition_platformer2d::runtime::SimulationHost::Ggrs);
    }
    app.add_plugins((
        AmbitionGameSimulationPlugin,
        AmbitionGameLdtkRuntimePlugin,
        AmbitionGamePresentationPlugin,
    ));
    // AssetSource registration runs LAST so EmbeddedAssetRegistry (added
    // by `AssetPlugin` inside `DefaultPlugins`) is already present.
    app.add_plugins(
        ambition_platformer2d::actors::assets::platformer_assets::AmbitionAssetSourcePlugin::for_profile(
            active_profile,
            &ambition_content::worlds::world_manifest(),
        ),
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
