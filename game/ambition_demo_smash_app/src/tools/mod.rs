//! The tools behind `smash_tool`, one module per subcommand.
//!
//! ⭐ **NINE BINARIES COLLAPSED INTO ONE (2026-09-03, Jon's ruling).** Measured
//! before the change: the nine `src/bin/*.rs` executables shared **99.88% of
//! their defined symbols** — `ladder_probe` and `roll_probe` had 501,778 of
//! ~502,000 in common — and the union across five of them was 822 symbols
//! larger than the largest single one, 0.16%. Nine copies of ~500 K symbols and
//! ~300 MB of identical DWARF were being written and linked on every build of
//! this crate.
//!
//! ⛔ **EACH MODULE OWNS ITS OWN ARGUMENT SURFACE**, as a `clap::Args` struct
//! beside the `run` it fills in. The old binaries scanned `std::env::args()` by
//! hand — `ladder_rig` did it in eight separate places — which documented
//! nothing, produced no `--help`, and accepted a flag anywhere in the vector
//! including after `--`. The flag SPELLINGS are preserved exactly: they are a
//! public surface named in `docs/` and in scripts.
//!
//! ⚠ **A SUBCOMMAND BEHIND A FEATURE STAYS VISIBLE WHEN THE FEATURE IS OFF.**
//! Cargo used to drop `match_shots` silently when `required-features` were
//! absent, which is indistinguishable from the bin not existing. Here the name
//! is always in `--help` and the disabled arm exits non-zero naming the feature
//! to rebuild with — a missing tool must say why, not read as a typo.

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
