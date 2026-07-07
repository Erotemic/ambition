//! Ambition Tangent Space Sandbox visible binary.
//!
//! This is a thin shim around `ambition_app::app::run_visible`. App-builder
//! logic now lives in the `ambition_app` library so the visible binary,
//! browser entry point, and headless drivers share the same composition layer.
//! Gameplay simulation systems live in `ambition_actors`.
//!
//! Web (`wasm32-unknown-unknown`) builds skip this `fn main()` entirely.
//! The browser entry point is the `#[wasm_bindgen(start)]` shim exported
//! from `ambition_app::lib`, which calls `app::run_web` after the wasm module
//! finishes loading. See `docs/recipes/web-build.md` for the bootstrap.

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    ambition_app::app::run_visible();
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // The browser entry point is the `#[wasm_bindgen(start)]` shim in
    // `ambition_app::lib`; this `main` exists only so `cargo build` is happy about
    // the binary target on a wasm32 host.
}
