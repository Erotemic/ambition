//! App-side simulation-resource residue (E5 step 6 slimmed this file).
//!
//! What remains here is genuinely Ambition-assembly:
//!
//! - App-local Ambition character-fragment registration (CONTENT choice),
//! - the LDtk data-asset Startup chain (`load_data_asset_handle` →
//!   `setup_simulation_system` — the host's world construction),
//! - the startup-profiler phase marks + PostStartup report.
//!
//! [`AmbitionGameSimulationSetupPlugin`] is mounted by
//! [`super::add_simulation_plugins`] before the engine group.

use bevy::prelude::*;

use ambition_platformer2d::actors::session::data;

pub struct AmbitionGameSimulationSetupPlugin;

impl Plugin for AmbitionGameSimulationSetupPlugin {
    fn build(&self, app: &mut App) {
        // Registration is App-local and idempotent, so direct users of this
        // plugin receive the same catalog as the full AmbitionGameSimulationPlugin.
        ambition_content::character_catalog::register(app);
        app.add_systems(
            Startup,
            (
                ambition_platformer2d::dev_tools::profiling::phase_mark("startup_begin"),
                data::load_data_asset_handle,
                ambition_platformer2d::dev_tools::profiling::phase_mark("after_load_data_handle"),
                // `SimulationSetupSet` is the machinery-facing label for
                // this slot: engine/host startup systems that need the sim
                // world set up (e.g. the host's input-component attach)
                // order `.after(the set)` instead of naming this system.
                // Direct entry constructs the simulation world at boot; the
                // shell host constructs a SESSION-scoped world per activation
                // (`shell_host::ambition_activate_session_visuals`).
                ambition_platformer2d::dev_tools::profiling::phase_mark("after_setup_simulation"),
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
                ambition_platformer2d::dev_tools::profiling::phase_mark("post_startup_begin"),
                ambition_platformer2d::dev_tools::profiling::report_startup_phases,
                ambition_platformer2d::dev_tools::profiling::report_schedule_census,
            )
                .chain(),
        )
        // The image census counts decoded textures, so it reads as a
        // presentation concern and its resource is installed with the other
        // presentation resources — but the SYSTEM is registered here, which a
        // headless app also runs. That split panicked every headless test with
        // "Resource does not exist". Initialising it beside its own system is
        // what makes the pairing local; `init_resource` is a no-op when the
        // presentation side has already inserted it.
        .init_resource::<ambition_platformer2d::render::asset_census::ImageCensus>()
        // The profiling-only workload censuses and the clock they share. Sim
        // side, so a headless VM run gets entity/body/schedule/frame rows on
        // the same clock as its perf and Tracy captures; the presentation half
        // (cameras, targets, portals, render passes) is added by
        // `add_presentation_plugins`. Off unless `AMBITION_PROFILE_CENSUS` is
        // set — see `ambition_dev_tools::runtime_census`.
        .add_plugins(ambition_platformer2d::dev_tools::runtime_census::RuntimeCensusPlugin)
        // The steady-state counterparts of the one-shot reports above.
        // `Last` so the frame census measures the whole frame, render work
        // included, rather than the part of it that happens to precede
        // whatever schedule we registered in.
        .add_systems(
            Last,
            (
                ambition_platformer2d::dev_tools::profiling::report_frame_census,
                ambition_platformer2d::render::asset_census::report_image_census,
            ),
        );
    }
}
