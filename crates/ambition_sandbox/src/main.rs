//! Ambition Tangent Space Sandbox visible binary.
//!
//! This is a thin shim around `ambition_sandbox::app::run_visible`. All the
//! gameplay systems, helpers, and App-builder logic moved to the library
//! crate (`crates/ambition_sandbox/src/app.rs`) in Slice 5 of ADR 0012's
//! events refactor so the headless binary can drive the same simulation
//! loop. See `docs/archive/historical-roadmaps/events-refactor-plan.md`.
//!
//! Web (`wasm32-unknown-unknown`) builds skip this `fn main()` entirely.
//! The browser entry point is the `#[wasm_bindgen(start)]` shim exported
//! from `ambition_sandbox::lib`, which calls `app::run_web` after the
//! wasm module finishes loading. See `docs/web_build.md` for the bootstrap.

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    ambition_sandbox::app::run_visible();
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // The browser entry point is the `#[wasm_bindgen(start)]` shim in
    // `lib.rs`; this `main` exists only so `cargo build` is happy about
    // the binary target on a wasm32 host.
}
