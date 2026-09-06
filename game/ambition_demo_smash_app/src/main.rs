//! The smash demo's binary.

/// What a sim-only run steps when nobody says otherwise.
#[cfg(not(feature = "visible"))]
const DEFAULT_TICKS: u32 = 600;

#[cfg(not(feature = "visible"))]
fn main() {
    // Headless: step a bounded number of ticks so a CI run is a run rather
    // than a hang, and say what happened.
    //
    // The other three each carried their own byte-identical parser; all four read the
    // launcher's now.
    let mut app = ambition_demo_smash_app::build_demo_app();
    let ticks = ambition_platformer2d::demo_shell::headless_ticks(DEFAULT_TICKS);
    for _ in 0..ticks {
        app.update();
    }
    println!("[smash_demo] stepped {ticks} ticks headless");
}

/// Drawn, through the windowed builder.
///
/// NOT `build_demo_app`: its foundation is `MinimalPlugins`, so it has no
/// renderer and no window whatever features are on, and `run()` on it spun the
/// schedules against no display and drew nothing. That is why this shell had
/// never been seen.
#[cfg(feature = "visible")]
fn main() {
    ambition_demo_smash_app::build_windowed_demo_app(ambition_platformer2d::app::Display::Window)
        .run();
}
