//! The Smash demo's CPU instruments, run in the SHIPPED composition.
//!
//! `smash_tool match-report` and `capture-probe` compose the demo shell, whose
//! catalog seats George and the two stand-ins and nobody else. The fighters
//! Jon plays Smash with — the three polygons, the cast — live in
//! `ambition_content`, and only the shipped composition carries them
//! (`app_it`'s road: build the visible app windowless, install a roster, route
//! to the gameplay screen). This binary is that road under the demo's two
//! instruments; the counting is theirs, unchanged.
//!
//! ```bash
//! cargo run -p ambition_app_tools --bin smash_cpu_report -- \
//!     match-report 60 pointed_polygon --opponent pugnacious_polygon
//! cargo run -p ambition_app_tools --bin smash_cpu_report -- \
//!     capture-probe 60 --character pointed_polygon --opponent pugnacious_polygon
//! ```
use ambition_demo_smash_app::tools::{capture_probe, match_report};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "smash_cpu_report", about = "Smash CPU instruments in the shipped composition")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Count what two CPUs do to each other (`smash_tool match-report`).
    MatchReport(match_report::MatchReportArgs),
    /// Probe captures in an unarranged CPU match (`smash_tool capture-probe`).
    CaptureProbe(capture_probe::CaptureProbeArgs),
}

/// The shipped composition, windowless: the full render graph with no wgpu
/// backend, stepped by hand. The same builder `smash_match_profile` and the
/// `app_it` tests use.
fn shipped() -> bevy::prelude::App {
    ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true)
}

fn main() {
    match Cli::parse().command {
        Command::MatchReport(args) => match_report::run_with(args, shipped),
        Command::CaptureProbe(args) => capture_probe::run_with(args, shipped),
    }
}
