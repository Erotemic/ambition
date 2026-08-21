//! The smash demo's binary.
//!
//! Headless by default (it steps the sim and reports), windowed under
//! `--features visible` — the same split every other demo shell uses, so the
//! regression tests and the playable build are the same app.

/// What a sim-only run steps when nobody says otherwise.
const DEFAULT_TICKS: u32 = 600;

fn main() {
    let mut app = ambition_demo_smash_app::build_demo_app();
    #[cfg(not(feature = "visible"))]
    {
        // Headless: step a bounded number of ticks so a CI run is a run rather
        // than a hang, and say what happened.
        //
        // ⛔ **`--ticks` used to be IGNORED here**, alone among the four demo
        // shells: this one hardcoded 600 while `run_game.sh --help` documents the
        // flag as the demo-shell convention, so `--headless -- --ticks 3` stepped
        // six hundred and reported six hundred. The other three each carried
        // their own byte-identical parser; all four read the launcher's now.
        let ticks = ambition_platformer2d::demo_shell::headless_ticks(DEFAULT_TICKS);
        for _ in 0..ticks {
            app.update();
        }
        println!("[smash_demo] stepped {ticks} ticks headless");
    }
    #[cfg(feature = "visible")]
    app.run();
}
