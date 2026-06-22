//! Ambition app shell (Stage 20 / A3 bisection).
//!
//! The COMPOSITION layer: Bevy app assembly (`app/`), platform/host glue
//! (`host/`), app-level dev overlays (`dev/`), the game-side menu host stack
//! (`menu/`), the headless driver (`headless`), and the RL stepping API
//! (`rl_sim`, feature-gated). Sits on top of the machinery lib
//! (`ambition_gameplay_core`) and the named-content crate (`ambition_content`); this is
//! the only crate allowed to name both.
//!
//! Binaries: `ambition_game_bin`, `headless`, `rl_random_walker`, `rl_smoke`,
//! and `trace_replay` (rl_sim feature for the stepping drivers).

pub mod app;
pub mod dev;
pub mod headless;
pub mod host;
pub mod menu;
#[cfg(feature = "rl_sim")]
pub mod rl_sim;

pub use headless::{run_headless, HeadlessReport};
#[cfg(feature = "rl_sim")]
pub use rl_sim::{
    AgentAction, AgentObservation, Lcg, RandomWalkPolicy, RandomWalkTuning, SandboxSim,
    SandboxSimOptions, TimestepMode,
};

/// Android shared-library entry point.
///
/// Desktop builds enter through `src/bin/ambition_game_bin.rs`, but the Android
/// Gradle project packages this crate as a shared library. GameActivity /
/// android-activity expects the library to export `android_main`; Bevy's
/// `#[bevy_main]` macro generates that boilerplate and registers the Android
/// app handle for `bevy_winit` before calling into our normal visible app
/// builder. NOTE (A3): the .so was previously `libambition_gameplay_core.so` built
/// from the old monolith; the Gradle config must point at `libambition_app.so`
/// after this split.
#[cfg(target_os = "android")]
#[bevy::prelude::bevy_main]
fn main() {
    app::run_visible();
}

/// Browser (`wasm32-unknown-unknown`) entry point. The analog of the Android
/// shim above: the platform supplies the entry-point convention, and we hand
/// off to the browser-flavored Bevy app builder.
#[cfg(all(target_arch = "wasm32", feature = "web_platform"))]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn web_start() {
    // Forward panics to `console.error` instead of the default `abort`
    // so a first-pass crash is debuggable from devtools without a
    // separate wasm symbol pass. Cheap; `set_once` is idempotent.
    console_error_panic_hook::set_once();
    app::run_web();
}
