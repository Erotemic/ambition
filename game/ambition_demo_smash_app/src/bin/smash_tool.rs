//! `smash_tool` — every probe, rig, report and diagram for the smash demo,
//! behind one binary.
//!
//! ```text
//! cargo run -p ambition_demo_smash_app --bin smash_tool -- --help
//! cargo run -p ambition_demo_smash_app --bin smash_tool -- ladder-rig --sweep-below --seeds 1
//! ```
//!
//! ⭐ **WHY ONE BINARY.** Measured 2026-09-03, before the collapse: the nine
//! executables this replaces shared **99.88% of their defined symbols**
//! (`ladder_probe` and `roll_probe` had 501,778 of ~502,000 in common), and the
//! union across five was 822 symbols larger than the largest single one — 0.16%.
//! Nine copies of ~500 K symbols and ~300 MB of identical DWARF were written and
//! linked on every build of this crate.
//!
//! ⛔ **A SUBCOMMAND BEHIND A FEATURE IS STILL LISTED WHEN THE FEATURE IS OFF.**
//! Cargo's `required-features` silently omits a binary, which a caller cannot
//! tell from the binary never having existed. Here the name is always in
//! `--help`, and running it without its feature exits non-zero naming the
//! feature to rebuild with.

use clap::{Parser, Subcommand};

use ambition_demo_smash_app::tools;

#[derive(Parser)]
#[command(
    name = "smash_tool",
    about = "Probes, rigs, reports and diagrams for the smash demo.",
    long_about = "One binary for every smash-demo tool.\n\n\
                  These are instruments, not the demo: `smash_demo` is the \
                  playable shell. Each subcommand was its own executable until \
                  2026-09-03; they shared 99.88% of their symbols, so they now \
                  share one binary.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Probe whether the live game can produce a grab hold at all.
    CaptureProbe(tools::capture_probe::CaptureProbeArgs),
    /// Trace the fighter brain's ladder decisions through the causal seam.
    LadderProbe(tools::ladder_probe::LadderProbeArgs),
    /// The ladder rig: sweeps, scenarios and utility-weight overrides.
    LadderRig(tools::ladder_rig::LadderRigArgs),
    /// Draw a match, damaged, so the meter shows something.
    MatchDiagram(tools::match_diagram::MatchDiagramArgs),
    /// Simulate matches and report the tallies.
    MatchReport(tools::match_report::MatchReportArgs),
    /// Photograph a burst of frames from a real match.
    ///
    /// Needs `--features visible,capture`: it captures the presentation it
    /// photographs, and without those features this app has no renderer.
    MatchShots(MatchShotsCommand),
    /// Probe the roll reading, grounded or airborne.
    RollProbe(tools::roll_probe::RollProbeArgs),
    /// Walk the select screen with a real cursor and print what it says.
    SelectWalkthrough,
    /// Draw the stage, including the thing that kills you.
    StageDiagram(tools::stage_diagram::StageDiagramArgs),
}

/// ⚠ `match_shots` needs two features, so its argument struct only exists when
/// they are on. The SUBCOMMAND exists either way — see the header — so this
/// stands in for the arguments when it cannot be built, and accepts whatever
/// the caller typed so the error is about the feature rather than about a flag.
#[cfg(all(feature = "visible", feature = "capture"))]
type MatchShotsCommand = tools::match_shots::MatchShotsArgs;

#[cfg(not(all(feature = "visible", feature = "capture")))]
#[derive(clap::Args, Debug)]
struct MatchShotsCommand {
    /// Accepted and ignored: this build cannot run `match-shots`.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    _ignored: Vec<String>,
}

fn main() {
    match Cli::parse().command {
        Command::CaptureProbe(a) => tools::capture_probe::run(a),
        Command::LadderProbe(a) => tools::ladder_probe::run(a),
        Command::LadderRig(a) => tools::ladder_rig::run(a),
        Command::MatchDiagram(a) => tools::match_diagram::run(a),
        Command::MatchReport(a) => tools::match_report::run(a),
        Command::MatchShots(a) => run_match_shots(a),
        Command::RollProbe(a) => tools::roll_probe::run(a),
        Command::SelectWalkthrough => tools::select_walkthrough::run(),
        Command::StageDiagram(a) => tools::stage_diagram::run(a),
    }
}

#[cfg(all(feature = "visible", feature = "capture"))]
fn run_match_shots(args: MatchShotsCommand) {
    tools::match_shots::run(args);
}

#[cfg(not(all(feature = "visible", feature = "capture")))]
fn run_match_shots(_args: MatchShotsCommand) {
    eprintln!(
        "smash_tool: `match-shots` needs the `visible` and `capture` features, \
         and this binary was built without them.\n\
         \n\
         Rebuild with:\n  \
         cargo run -p ambition_demo_smash_app --features visible,capture \\\n    \
         --bin smash_tool -- match-shots [ARGS]"
    );
    std::process::exit(2);
}
