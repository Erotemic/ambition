//! Boot the WEB persona's composition on a native host.
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

    // A `NoWindow` host draws no pixels by construction — but the entities a route builds are
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

    // AND A COMPOSITION THAT DRAWS IS NOT A COMPOSITION THAT PLAYS.
    //
    // The browser reached this point — it booted, routed, and painted a menu the
    // arrow keys navigated — and the controlled body still never moved, because
    // the primary device→tick latch was installed by the DEV OBSERVATORY and the
    // web persona has no `dev_tools`. Under GGRS `capture_latched_local_input`
    // takes the latch as `Option`, so its absence is silent: seat zero publishes
    // a neutral frame forever and the simulation is genuinely being told the
    // player is holding nothing.
    //
    // Ownership, not presence, is the invariant: a device-driven host owns the
    // frame→tick bridge, and removing an instrument may never remove input.
    let host = app
        .world()
        .get_resource::<ambition_platformer2d::runtime::SimulationHost>()
        .copied();
    // seat zero is ROW ZERO of the one latch table now; it used to be its own
    // `ControlFrameLatch` resource beside it.
    let latches = app
        .world()
        .get_resource::<ambition_platformer2d::characters::control::SlotControlLatches>()
        .copied();
    println!("web-persona-boot: simulation host = {host:?}, device latches = {latches:?}");
    let Some(latches) = latches else {
        eprintln!(
            "web-persona-boot: ⛔ the web persona composed a {host:?} host with NO \
             `SlotControlLatches` — nothing bridges this frame's device sample to the \
             next tick, so seat zero's input is neutral every tick and the controlled \
             body cannot move. Menus still work; they never go through the session."
        );
        std::process::exit(1);
    };
    // AND INSTALLED IS NOT WIRED. `capture_latched_local_input` publishes the latch only
    // while `is_device_authority()` — an untouched latch means "nothing feeds me", not "the
    // device said nothing", and it declines. Startup ran hundreds of frames above; if
    // `accumulate_control_frame_latch` were scheduled, it has run.
    if !latches.is_device_authority(ambition_platformer2d::characters::control::PlayerSlot::PRIMARY) {
        eprintln!(
            "web-persona-boot: ⛔ the web persona has a `SlotControlLatches` whose seat \
             zero NOTHING \
             HAS FED after {} frames. `capture_latched_local_input` refuses to publish an \
             unfed latch, so seat zero is still neutral every tick — the same dead \
             gameplay input, with the resource present. The accumulator was left behind \
             when the latch moved.",
            ambition_app::app::shared_host_startup_ticks()
        );
        std::process::exit(1);
    }

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
