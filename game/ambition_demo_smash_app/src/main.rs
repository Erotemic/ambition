//! The smash demo's binary.

/// What a sim-only run steps when nobody says otherwise.
#[cfg(not(feature = "visible"))]
const DEFAULT_TICKS: u32 = 600;

#[cfg(not(feature = "visible"))]
fn main() {
    // Headless: step a bounded number of ticks, so a CI run ends, and report
    // what happened. The tick count comes from the shared launcher parser.
    let mut app = ambition_demo_smash_app::build_demo_app();
    let ticks = ambition_platformer2d::demo_shell::headless_ticks(DEFAULT_TICKS);
    for _ in 0..ticks {
        app.update();
    }
    println!("[smash_demo] stepped {ticks} ticks headless");
}

/// Drawn, through the windowed builder.
///
/// Not `build_demo_app`: its foundation is `MinimalPlugins`, so it has no
/// renderer or window whatever features are on.
#[cfg(feature = "visible")]
fn main() {
    ambition_demo_smash_app::build_windowed_demo_app(ambition_platformer2d::app::Display::Window)
        .run();
}
