//! The smash demo's binary.
//!
//! Headless by default (it steps the sim and reports), windowed under
//! `--features visible` — the same split every other demo shell uses, so the
//! regression tests and the playable build are the same app.

fn main() {
    let mut app = ambition_demo_smash_app::build_demo_app();
    #[cfg(not(feature = "visible"))]
    {
        // Headless: step a fixed number of ticks so a CI run is a run rather
        // than a hang, and say what happened.
        for _ in 0..600 {
            app.update();
        }
        println!("[smash_demo] stepped 600 ticks headless");
    }
    #[cfg(feature = "visible")]
    app.run();
}
