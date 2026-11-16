//! Ambition Tangent Space Sandbox visible binary.
//!
//! This is a thin shim around `ambition_app::app::run_visible`. App-builder
//! logic now lives in the `ambition_app` library so the visible binary,
//! browser entry point, and headless drivers share the same composition layer.
//! Gameplay simulation systems live in `ambition_platformer2d::actors`.
//!
//! Web (`wasm32-unknown-unknown`) builds skip this `fn main()` entirely.
//! The browser entry point is the `#[wasm_bindgen(start)]` shim exported
//! from `ambition_app::lib`, which calls `app::run_web` after the wasm module
//! finishes loading. See `docs/recipes/web-build.md` for the bootstrap.

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    // `content ...` and `--schema-identity` are the binary's own validator
    // (`app::content_check`); anything else starts the game.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = ambition_app::app::content_check::run_if_asked(&args) {
        return code;
    }
    ambition_app::app::run_visible();
    std::process::ExitCode::SUCCESS
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // The browser entry point is the `#[wasm_bindgen(start)]` shim in
    // `ambition_app::lib`; this `main` exists only so `cargo build` is happy about
    // the binary target on a wasm32 host.
}
