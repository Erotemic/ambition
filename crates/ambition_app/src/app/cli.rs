#[allow(unused_imports)]
use super::dev_runtime::*;
#[allow(unused_imports)]
use super::feedback::*;
#[allow(unused_imports)]
use super::hud::*;
#[allow(unused_imports)]
use super::phases::*;
#[allow(unused_imports)]
use super::player_tick::*;
#[allow(unused_imports)]
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;
#[allow(unused_imports)]
use ambition_gameplay_core::schedule::*;

/// Resolve the on-disk asset root for the desktop app.
///
/// Bevy's `FileAssetReader` anchors relative asset paths at
/// `BEVY_ASSET_ROOT` / the RUNNING binary's `CARGO_MANIFEST_DIR` — which
/// has been `crates/ambition_app/` since the Stage 20 / A3 bisection,
/// while the asset tree stays with the machinery lib at
/// `crates/ambition_gameplay_core/assets` (the lib's `include_str!` paths and
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
    if std::env::var_os("BEVY_ASSET_ROOT").is_some() {
        return "assets".to_string();
    }
    let dev_assets =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ambition_gameplay_core/assets");
    match dev_assets.canonicalize() {
        Ok(path) if path.is_dir() => path.to_string_lossy().into_owned(),
        _ => "assets".to_string(),
    }
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
    use super::parse_headless_ticks;

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
        assert!(root.ends_with("crates/ambition_gameplay_core/assets") || root.ends_with("assets"));
        assert!(
            root.join("ambition/sandbox.ron").exists(),
            "asset root {root:?} must contain ambition/sandbox.ron"
        );
        assert!(
            root.join("dialogue").is_dir(),
            "asset root {root:?} must contain the dialogue/ tree"
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
    if cli_force_headless() || no_display_server_available() {
        let max_ticks = cli_headless_ticks();
        let reason = if cli_force_headless() {
            "--headless flag"
        } else {
            "no DISPLAY / WAYLAND_DISPLAY env var"
        };
        eprintln!(
            "ambition_gameplay_core: running headless ({reason}); use `--bin headless` for the dedicated runner"
        );
        match crate::headless::run_headless(max_ticks) {
            Ok(report) => {
                println!("{report}");
                return;
            }
            Err(error) => {
                eprintln!("headless fallback failed: {error}");
                std::process::exit(1);
            }
        }
    }
    let asset_config = GameAssetConfig::from_args();
    let asset_root = desktop_asset_root();
    eprintln!("ambition_gameplay_core: asset root = {asset_root}");
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(bevy::asset::AssetPlugin {
                // See `desktop_asset_root`: post-bisection the binary's
                // crate has no assets/ tree; the canonical one lives with
                // the machinery lib.
                file_path: asset_root,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Ambition - Tangent Space Sandbox (Bevy)".into(),
                    resolution: WindowResolution::new(WINDOW_W, WINDOW_H),
                    resizable: true,
                    resize_constraints: WindowResizeConstraints {
                        min_width: 640.0,
                        min_height: 360.0,
                        ..default()
                    },
                    ..default()
                }),
                ..default()
            }),
    );
    // DefaultPlugins installs StatesPlugin, so initialize GameMode after it.
    app.init_state::<GameMode>();
    let active_profile = asset_config.asset_profile;
    app.insert_resource(asset_config);
    app.add_plugins((
        SandboxSimulationPlugin,
        SandboxLdtkPlugin,
        SandboxPresentationPlugin,
    ));
    // AssetSource registration runs LAST so EmbeddedAssetRegistry
    // (added by `AssetPlugin` inside `DefaultPlugins`) is already present.
    app.add_plugins(
        ambition_gameplay_core::assets::sandbox_assets::AmbitionAssetSourcePlugin::for_profile(
            active_profile,
        ),
    );
    app.run();
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
/// `ambition_gameplay_core::lib`'s root, behind the same `cfg(target_arch = "wasm32")` +
/// `feature = "web_platform"` gate.
#[cfg(all(target_arch = "wasm32", feature = "web_platform"))]
pub fn run_web() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Ambition - Tangent Space Sandbox (Web)".into(),
            // The canvas selector matches `<canvas id="bevy">` in
            // `crates/ambition_gameplay_core/web/index.html`. Without this Bevy
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
    app.init_state::<GameMode>();
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
        target: "ambition::sandbox_assets",
        "web start: AssetProfile = {} | static_map = {} | static_core_assets = {} | static_sfx_bank = {}",
        active_profile.label(),
        cfg!(feature = "static_map"),
        cfg!(feature = "static_core_assets"),
        cfg!(feature = "static_sfx_bank"),
    );
    app.insert_resource(asset_config);
    app.add_plugins((
        SandboxSimulationPlugin,
        SandboxLdtkPlugin,
        SandboxPresentationPlugin,
    ));
    // AssetSource registration runs LAST so EmbeddedAssetRegistry (added
    // by `AssetPlugin` inside `DefaultPlugins`) is already present.
    app.add_plugins(
        ambition_gameplay_core::assets::sandbox_assets::AmbitionAssetSourcePlugin::for_profile(
            active_profile,
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
