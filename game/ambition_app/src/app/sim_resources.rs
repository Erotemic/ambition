//! App-side simulation-resource residue (E5 step 6 slimmed this file).
//!
//! The engine-generic sim messages + resource defaults moved to
//! `ambition::runtime::SimCoreResourcesPlugin` (in the engine group), so a
//! demo app gets a bootable sim without this crate. What remains here is
//! genuinely Ambition-assembly:
//!
//! - App-local Ambition character-fragment registration (CONTENT choice),
//! - the LDtk data-asset Startup chain (`load_data_asset_handle` →
//!   `setup_simulation_system` — the host's world construction),
//! - the startup-profiler phase marks + PostStartup report.
//!
//! [`SandboxSimulationResourcesPlugin`] is mounted by
//! [`super::add_simulation_plugins`] before the engine group.

use bevy::prelude::*;

use super::setup_systems::setup_simulation_system;
use ambition::actors::session::data;
use ambition::platformer::schedule::SimulationSetupSet;

pub struct SandboxSimulationResourcesPlugin;

impl Plugin for SandboxSimulationResourcesPlugin {
    fn build(&self, app: &mut App) {
        // Registration is App-local and idempotent, so direct users of this
        // plugin receive the same catalog as the full SandboxSimulationPlugin.
        ambition_content::character_catalog::register(app);
        app.add_systems(
            Startup,
            (
                ambition::dev_tools::profiling::phase_mark("startup_begin"),
                data::load_data_asset_handle,
                ambition::dev_tools::profiling::phase_mark("after_load_data_handle"),
                // `SimulationSetupSet` is the machinery-facing label for
                // this slot: engine/host startup systems that need the sim
                // world set up (e.g. the host's input-component attach)
                // order `.after(the set)` instead of naming this system.
                // Direct entry constructs the simulation world at boot; the
                // shell host constructs a SESSION-scoped world per activation
                // (`shell_host::ambition_activate_session`).
                setup_simulation_system
                    .in_set(SimulationSetupSet)
                    .run_if(super::shell_host::direct_entry),
                ambition::dev_tools::profiling::phase_mark("after_setup_simulation"),
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
                ambition::dev_tools::profiling::phase_mark("post_startup_begin"),
                ambition::dev_tools::profiling::report_startup_phases,
            )
                .chain(),
        );
    }
}
