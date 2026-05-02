//! Headless Ambition sandbox driver.
//!
//! Builds the simulation App with no rendering, audio, or windowing plugins
//! and runs `Update` for a fixed number of ticks, then exits. Useful for
//! environments without a display (CI, remote VMs) and as a foundation for
//! future RL drivers that need deterministic stepping. See
//! `crate::headless::run_headless` for what is and is not exercised.
//!
//! Usage:
//!
//! ```bash
//! cargo run -p ambition_sandbox --bin headless           # 120 ticks (default)
//! cargo run -p ambition_sandbox --bin headless -- 600    # 600 ticks
//! ```

fn main() {
    let max_ticks: u32 = std::env::args()
        .nth(1)
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(120);

    match ambition_sandbox::run_headless(max_ticks) {
        Ok(report) => {
            println!("{report}");
        }
        Err(error) => {
            eprintln!("headless run failed: {error}");
            std::process::exit(1);
        }
    }
}
