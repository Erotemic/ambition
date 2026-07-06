//! App-side simulation-resource residue (E5 step 6 slimmed this file).
//!
//! The engine-generic sim messages + resource defaults moved to
//! `ambition_runtime::SimCoreResourcesPlugin` (in the engine group), so a
//! demo app gets a bootable sim without this crate. What remains here is
//! genuinely Ambition-assembly:
//!
//! - the character catalog install + roster plugin (CONTENT choice),
//! - the LDtk data-asset Startup chain (`load_data_asset_handle` →
//!   `setup_simulation_system` — the host's world construction),
//! - the startup-profiler phase marks + PostStartup report.
//!
//! [`SandboxSimulationResourcesPlugin`] is mounted by
//! [`super::add_simulation_plugins`] before the engine group.

use bevy::prelude::*;

use super::setup_systems::setup_simulation_system;
use ambition_gameplay_core::session::data;

pub struct SandboxSimulationResourcesPlugin;

impl Plugin for SandboxSimulationResourcesPlugin {
    fn build(&self, app: &mut App) {
        app
            // CharacterCatalogPlugin installs the parsed character
            // catalog as a Bevy resource and runs a Startup validator
            // that panics on broken references. See
            // `ambition_characters::actor::character_catalog` and ADR 0017
            // (Rust = behavior, RON = content, LDtk = space).
            .add_plugins({
                // The plugin ctor reads the installed catalog RON — install
                // here (idempotent, first-wins) so plugin-mount order can
                // never make the read precede the install.
                ambition_content::character_catalog::install();
                ambition_gameplay_core::character_roster::character_roster_plugin()
            })
            .add_systems(
                Startup,
                (
                    ambition_gameplay_core::dev::profiling::phase_mark("startup_begin"),
                    data::load_data_asset_handle,
                    ambition_gameplay_core::dev::profiling::phase_mark("after_load_data_handle"),
                    // `SimulationSetupSet` is the machinery-facing label for
                    // this slot: engine/host startup systems that need the sim
                    // world set up (e.g. the host's input-component attach)
                    // order `.after(the set)` instead of naming this system.
                    setup_simulation_system
                        .in_set(ambition_gameplay_core::schedule::SimulationSetupSet),
                    ambition_gameplay_core::dev::profiling::phase_mark("after_setup_simulation"),
                )
                    .chain(),
            )
            // Final report. Runs once on the first PostStartup tick. The
            // pre-report mark captures the time between the last Startup
            // mark and PostStartup, so any heavy Startup systems we
            // didn't explicitly mark show up as a delta on the
            // "post_startup_begin" line.
            .add_systems(
                PostStartup,
                (
                    ambition_gameplay_core::dev::profiling::phase_mark("post_startup_begin"),
                    ambition_gameplay_core::dev::profiling::report_startup_phases,
                )
                    .chain(),
            );
    }
}
