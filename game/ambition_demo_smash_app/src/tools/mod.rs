//! The tools behind `smash_tool`, one module per subcommand.
//!
//! One binary with subcommands, not nine binaries: the separate executables
//! shared almost all their symbols, and each build linked nine copies.
//!
//! Each module owns its argument surface as a `clap::Args` struct beside its
//! `run`. Keep the flag spellings: `docs/` and scripts use them.
//!
//! A subcommand behind a feature stays in `--help` when the feature is off,
//! and exits non-zero naming the feature to rebuild with.

pub mod capture_probe;
pub mod ladder_probe;
pub mod ladder_rig;
pub mod match_diagram;
pub mod match_report;
#[cfg(all(feature = "visible", feature = "capture"))]
pub mod match_shots;
pub mod roll_probe;
pub mod select_walkthrough;
pub mod stage_diagram;
