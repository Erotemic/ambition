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

/// Declare `ladder` for the demo's Smash rooms: a tool's `--ladder FILE` arm.
/// The demo declares none of its own, so its fighters otherwise fight on the
/// engine floor.
pub(crate) fn declare_smash_ladder(app: &mut bevy::prelude::App, ladder: ambition_platformer2d::characters::brain::fighter::FighterBrainLadder) {
    use ambition_platformer2d::combat::scoped_rules::{DeclareRulesExt as _, RulesScope};
    app.declare_rules(RulesScope::Mode(ambition_demo_smash::SMASH_MODE), ladder);
}

/// Whether a fighter ladder governs the demo's Smash rooms.
pub(crate) fn smash_ladder_is_declared(app: &bevy::prelude::App) -> bool {
    use ambition_platformer2d::combat::scoped_rules::{ActiveRoom, DeclaredRules};
    app.world()
        .get_resource::<DeclaredRules<ambition_platformer2d::characters::brain::fighter::FighterBrainLadder>>()
        .is_some_and(|rules| rules.governing(ActiveRoom::Mode(ambition_demo_smash::SMASH_MODE)).is_some())
}
