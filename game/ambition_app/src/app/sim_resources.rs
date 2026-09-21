//! App-side simulation-resource residue (E5 step 6 slimmed this file).
//!
//! What remains here is genuinely Ambition-assembly:
//!
//! - App-local Ambition character-fragment registration (CONTENT choice),
//! - the LDtk data-asset Startup chain (`load_data_asset_handle`, then the
//!   empty `SimulationSetupSet` slot — see the chain itself; the host system
//!   that used to construct the world there is gone),
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
                // ⛔⛤ **THERE IS NO SYSTEM BETWEEN THESE TWO MARKS, AND THE
                // SECOND ONE NAMED IT UNTIL 2026-09-19.** `d3135def0` deleted
                // `setup_simulation_system` — it had never been registered, so
                // the `.after()` edge that appeared to order this chain against
                // it was a claim rather than a constraint. The mark outlived
                // the system and went on printing `after_setup_simulation`
                // every boot, which `docs/recipes/profiling.md` reproduces as
                // the dominant startup cost.
                //
                // ⚠ IT STILL BRACKETS SOMETHING, which is why it is renamed
                // rather than removed: `SimulationSetupSet` is the
                // machinery-facing label for this slot — engine/host startup
                // systems needing the sim world (e.g. the host's
                // input-component attach) order `.after(the set)` rather than
                // naming a system — and the demo fixtures do fill it. THIS
                // composition puts nothing in it, so what the interval now
                // holds is unordered `Startup` residue, not a stage. A number
                // measured here before the deletion says nothing about what it
                // reports today.
                //
                // The shell host builds a SESSION-scoped world per activation
                // (`shell_host::ambition_activate_session_visuals`) and does
                // not come through here at all.
                ambition_platformer2d::dev_tools::profiling::phase_mark("after_simulation_setup_slot"),
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
        // ⛔ **THE CENSUS CLOCK MOVED INTO `PlatformerEnginePlugins` ON
        // 2026-09-20 AND MUST NOT BE ADDED AGAIN HERE.** It was composed by
        // this host alone, so the demo hosts — which compose the engine group
        // and not this file — got census REPORTERS from the group without the
        // `RuntimeCensus` they take. Adding it a second time is a duplicate
        // plugin, which panics; the group is now the one owner, and the
        // presentation half is still added by `add_presentation_plugins`.
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
