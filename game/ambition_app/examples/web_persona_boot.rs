//! **Boot the WEB persona's composition on a native host.**
//!
//! ⛔⛔ **THE BROWSER IS A DIFFERENT BUILD, AND THAT IS WHAT KEPT HIDING THE
//! BUG.** Every test in `app_it` runs under the default desktop features, so it
//! answers "does the composition work" for a feature set the browser does not
//! have. Twice in one day that difference was the whole defect: first the web
//! persona had no shell host at all, then it had one and no
//! `basic_shell_presentation` to draw the route it booted into. Neither was
//! visible to a green desktop suite.
//!
//! This is the cheapest thing that is not a browser: compile `ambition_app` with
//! the web persona's Cargo features, compose the same
//! `compose_ambition_visible_game` the browser composes, and step it. A panic in
//! a plugin that only the web build installs surfaces HERE, in a terminal, with
//! a backtrace — instead of as a blank canvas and a console the maintainer has
//! to read for me.
//!
//! It cannot prove pixels: the surface is `NoWindow`, so there is no wgpu
//! adapter and nothing is drawn. What it proves is that the composition BUILDS
//! and RUNS under the browser's feature set, which is the half that kept being
//! wrong.
//!
//! ```sh
//! cargo run -p ambition_app --no-default-features \
//!     --features visible_web_base --example web_persona_boot
//! ```
//!
//! The aggregate `app_it` binary cannot do this job: its other modules need
//! `audio` and `rl_sim`, so it does not compile under the web feature set at
//! all. An example has exactly the crate's features and nothing else.

use ambition_app::app::{build_visible_app, VisibleRenderMode};

fn main() {
    println!("web-persona-boot: composing with the browser's Cargo features");
    println!(
        "  basic_shell_presentation = {}",
        cfg!(feature = "basic_shell_presentation")
    );
    println!(
        "  kaleidoscope_menu        = {}",
        cfg!(feature = "kaleidoscope_menu")
    );
    println!(
        "  bevy_ui_menu             = {}",
        cfg!(feature = "bevy_ui_menu")
    );
    println!(
        "  static_map               = {}",
        cfg!(feature = "static_map")
    );
    println!("  audio                    = {}", cfg!(feature = "audio"));

    // `shell_hosted: true` is what `VisibleGameSpec::browser` chooses.
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    println!("web-persona-boot: composed; stepping");

    for frame in 0..ambition_app::app::shared_host_startup_ticks() {
        app.update();
        if frame % 20 == 0 {
            println!("  frame {frame}");
        }
    }

    let route = app
        .world()
        .get_resource::<ambition_platformer2d::game_shell::ShellHostConfiguration>()
        .and_then(|config| config.spec.as_ref())
        .map(|spec| spec.initial_route.to_string());
    println!("web-persona-boot: SURVIVED startup. initial route = {route:?}");

    // ⛔ **SURVIVING IS NOT SHOWING.** The panic this example was written to catch
    // killed the app on its first menu tick; fixing that proves the app RUNS, and
    // proves nothing about whether anything reached the screen. A `NoWindow` host
    // draws no pixels by construction — but the entities a route builds are
    // ordinary world state, and their ABSENCE is exactly what a blank canvas is.
    let world = app.world_mut();
    let ui_nodes = world.query::<&bevy::ui::Node>().iter(world).count();
    let sprites = world.query::<&bevy::sprite::Sprite>().iter(world).count();
    let texts = world.query::<&bevy::ui::widget::Text>().iter(world).count();
    let cameras = world.query::<&bevy::prelude::Camera>().iter(world).count();
    println!(
        "web-persona-boot: presentation population — {ui_nodes} UI nodes, {texts} UI texts, \
         {sprites} sprites, {cameras} cameras"
    );

    let active = app
        .world()
        .get_resource::<ambition_platformer2d::game_shell::ShellRouter>()
        .and_then(|router| router.active.as_ref())
        .map(|active| active.route_id.to_string());
    println!("web-persona-boot: active route = {active:?}");

    // A route that resolved and built NOTHING is the blank screen, reported here
    // rather than left for a browser to demonstrate.
    if ui_nodes == 0 && sprites == 0 {
        eprintln!(
            "web-persona-boot: ⛔ the web persona composed, routed, and produced NO \
             drawable entities at all — no UI node and no sprite. Whatever a browser \
             would show, it is not this game."
        );
        std::process::exit(1);
    }
}
