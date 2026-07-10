//! The Sanic demo's shell, as a function — so the binary and the exit-3
//! regression test assemble the SAME app.
//!
//! See `main.rs` for the doctrine this file encodes.

use bevy::prelude::*;

use ambition_demo_sanic::{SanicDemoContentPlugin, SanicRulesPlugin};

/// Assemble the demo: foundation + the engine group + the host group + this
/// demo's content and rules. **Zero engine edits, zero `ambition_app`.**
///
/// Headless-foundation here; a windowed shell swaps that one call for
/// `DefaultPlugins` + `ambition::engine::init_engine_states`. Everything below it
/// is identical, which is the claim exit 3 makes.
pub fn build_demo_app() -> App {
    let mut app = App::new();
    ambition::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    app.add_plugins((SanicDemoContentPlugin, SanicRulesPlugin::global()));
    // Pin the frame dt to the tick dt so one `update()` is exactly one sim tick.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}
