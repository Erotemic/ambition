//! A LINK IS NOT A BOOT.
//!
//! The web build gate compiles and links the release wasm. It passed on an app that loaded the
//! page, initialized wgpu, painted a canvas, and showed nothing at all — no launcher, no room,
//! no error.
//!
//! these are not "is the plugin list still the plugin list" assertions.
//! Each assertion covers composition state whose absence produces a blank app without a
//! diagnostic.

use bevy::prelude::*;

use ambition_app::app::visible_composition::VisibleGameSpec;
use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::game_shell::ShellHostConfiguration;
use ambition_platformer2d::sprite_sheet::game_assets::GameAssetConfig;

struct CompositionContract {
    /// The resource `publish_direct_prepared_session_root` checks. Without it a
    /// host carries the build-time session root AND the activation's, which is
    /// two canonical roots.
    shell_hosted: bool,
    /// The route the host boots into.
    initial_route: Option<String>,
    /// Per-session room presentation. Missing it, the session activates, the
    /// simulation runs, and the world is never made visible.
    room_visuals: bool,
    /// SOMETHING THAT DRAWS A ROUTE. A shell host routes; it does not
    /// paint. `MinimalShellPlugins` adds `BasicShellPresentationPlugin` only
    /// under the `basic_presentation` FEATURE, so a persona whose Cargo features
    /// omit it composes a perfectly correct host that boots to the launcher and
    /// renders nothing at all.
    ///
    /// This is a feature-composition property, so it must be tested under the
    /// persona's actual feature set rather than default desktop features.
    route_presentation: bool,
}

/// Whether anything in this build can draw a shell route.
///
/// Answered by `cfg`, deliberately: without the feature the plugin TYPE does not
/// exist, so "is it installed" is not a question a compiled probe can ask — the
/// honest answer is that nothing draws a route, which is exactly the failure.
fn route_presentation_installed(app: &App) -> bool {
    #[cfg(feature = "basic_shell_presentation")]
    {
        app.is_plugin_added::<ambition_platformer2d::game_shell::BasicShellPresentationPlugin>()
    }
    #[cfg(not(feature = "basic_shell_presentation"))]
    {
        let _ = app;
        false
    }
}

fn measure(app: &App) -> CompositionContract {
    CompositionContract {
        shell_hosted: app
            .world()
            .get_resource::<ambition_app::app::shell_host::AmbitionShellHosted>()
            .is_some(),
        initial_route: app
            .world()
            .get_resource::<ShellHostConfiguration>()
            .and_then(|config| config.spec.as_ref())
            .map(|spec| spec.initial_route.to_string()),
        room_visuals: app.is_plugin_added::<
            ambition_platformer2d::render::platformer_presentation::SessionRoomVisualsPlugin,
        >(),
        route_presentation: route_presentation_installed(app),
    }
}

fn assert_contract_holds(persona: &str, app: &App) {
    let measured = measure(app);
    assert!(
        measured.shell_hosted,
        "{persona}: no `AmbitionShellHosted`, so the session root is claimed twice"
    );
    let route = measured.initial_route.as_deref().unwrap_or("");
    assert!(
        !route.is_empty(),
        "{persona}: the shell host has NO initial route. This is the blank browser \
         canvas: the app runs, the simulation ticks, and the shell never routes \
         anywhere because nothing told it where to start."
    );
    assert!(
        measured.route_presentation,
        "{persona}: nothing in this build draws a shell route. The host composes, \
         routes to '{route}', and paints an empty surface — which is what the \
         browser did after it GAINED a shell host, because `visible_web_base` did \
         not enable `basic_shell_presentation`. A correct composition under a \
         feature set that cannot render it is still a blank screen."
    );
    assert!(
        measured.room_visuals,
        "{persona}: `install_ambition_shell_visuals` did not run, so an activated \
         session has no room presentation. Nothing errors — the world simply never \
         becomes visible, which is how `capture_scene` photographed an empty game \
         for two days."
    );
}

/// this is the BROWSER's game-side composition, run on a native host.
/// `shell_hosted` is read off `VisibleGameSpec::browser` rather than written as
/// `true`, so what gets composed here is what `run_web` composes — the browser
/// and this test cannot drift apart without the shared spec changing under
/// both. Only the render surface differs, and a surface is not a composition.
#[test]
fn the_launcher_persona_composes_a_route_and_room_visuals() {
    let browser = VisibleGameSpec::browser(GameAssetConfig::default());
    let app = build_visible_app(VisibleRenderMode::NoWindow, browser.shell_hosted);
    assert_contract_holds("browser / launcher persona", &app);
}

#[test]
fn the_direct_gameplay_persona_composes_a_route_and_room_visuals() {
    let app = build_visible_app(VisibleRenderMode::NoWindow, false);
    assert_contract_holds("direct gameplay persona", &app);
}

/// THE POISON. Every probe above reports "present" by reading a world;
/// a probe that cannot report "absent" proves nothing. This is the pre-repair
/// browser: Bevy's foundation and no Ambition composition on top of it.
#[test]
fn the_probes_can_see_an_uncomposed_app() {
    let app = App::new();
    let measured = measure(&app);
    assert!(
        !measured.shell_hosted
            && measured.initial_route.is_none()
            && !measured.room_visuals
            && !measured.route_presentation,
        "a bare App reported part of the composition contract as satisfied, so the \
         assertions above cannot fail and pin nothing"
    );
}

/// The browser persona is a VALUE, not a passage inside `run_web` — so the
/// three answers a `<canvas>` host gives can be read here rather than
/// re-derived from a `cfg(wasm32)` function no native test can reach.
///
/// what this defends is the shape of the repair, not the booleans: as long as
/// `run_web` is a platform foundation plus `compose_ambition_visible_game`, the
/// composition above is the composition the browser gets. The day someone adds
/// a fourth game-side step to `run_web` directly, this file is where the reader
/// is told it does not belong there.
#[test]
fn the_browser_persona_boots_the_launcher_with_the_tile_spine_and_no_desktop_curtain() {
    let spec = VisibleGameSpec::browser(GameAssetConfig::default());
    assert!(
        spec.shell_hosted,
        "the browser must boot the launcher; a player's first screen is the same on every platform"
    );
    assert!(
        spec.tile_spine,
        "a canvas has a real RenderApp, so dropping the LDtk tile spine there would \
         paint a different game than the desktop draws"
    );
    assert!(
        !spec.startup_loading_curtain,
        "the boot curtain hides a desktop compositor behind a not-yet-drawn window; \
         a canvas has no such window and `web/index.html` owns the page's loading state"
    );
}
