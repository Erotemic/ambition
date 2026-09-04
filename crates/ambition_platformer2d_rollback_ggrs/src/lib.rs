//! GGRS rollback host for Ambition's platformer simulation.
//!
//! The generic runtime owns simulation composition and stable rollback-schema
//! metadata. This crate owns the concrete `bevy_ggrs` schedule, snapshot/history
//! machinery, session lifecycle, checksum probes, and post-load repair. Domains
//! continue to declare rollback state through the backend-neutral
//! `RollbackRegistrar`; this host installs those same declarations through
//! [`GgrsRollbackRegistrar`].

use bevy::{
    ecs::schedule::{LogLevel, ScheduleBuildSettings, SingleThreadedExecutor},
    prelude::*,
};
use bevy_ggrs::{GgrsPlugin, RollbackFrameRate};

use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;

pub use ambition_platformer2d_runtime::rollback::{
    RollbackEntryKind, RollbackRegistrationDescriptor, RollbackRegistrationError,
    RollbackRegistrationOutcome, RollbackRegistry, GGRS_ROLLBACK_SCHEMA_VERSION,
};
pub use ambition_platformer2d_runtime::{PreparedContentIdentity, SnapshotSchemaFingerprint};

pub use bevy_ggrs::{
    AdvanceWorld, AdvanceWorldSystems, ConfirmedFrameCount, GgrsSchedule, LoadWorld,
    LoadWorldSystems, Rollback, RollbackFrameCount, RunGgrsSystems, SaveWorld,
};

pub mod codec;
#[cfg(test)]
mod codec_tests;
#[cfg(test)]
mod host_invariant_tests;
pub mod lifecycle_commit;
pub mod local_session;
mod probes;
mod reconcile;
mod registrar;
mod registration;
pub mod session;
#[cfg(test)]
mod session_ownership_tests;

pub use codec::*;
pub use probes::*;
pub use registrar::GgrsRollbackRegistrar;
pub use registration::AmbitionRollbackApp;
pub use session::*;

/// Ambition-owned work that must run after every `bevy_ggrs` entity/data/map restore.
#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum AmbitionLoadWorldSet {
    Reconcile,
}

/// FORK(ggrs-frame-timing): publish the rollback driver's own intra-tick phase
/// for render-clock pose resampling. See the workspace `bevy_ggrs` patch note.
fn sample_ggrs_accumulator_phase(
    timing: Res<bevy_ggrs::GgrsFrameTiming>,
    mut phase: ResMut<ambition_sim_view::PresentationPhase>,
) {
    phase.set(timing.overstep_fraction());
}

/// Install only the concrete GGRS backend into an App whose semantic simulation
/// host has already been selected as rollback. This plugin owns selection of the
/// concrete `GgrsSchedule`.
///
/// ⭐⭐ IT DOES TWO JOBS, AND SINCE 2026-09-04 THEY ARE SEPARABLE. Installing the
/// GGRS backend and declaring the ENGINE'S rollback state are not the same job,
/// and fusing them made the backend uncomposable without twenty domains: a host
/// that adds only this plugin inherits every checksummed resource those domains
/// declare, and `bevy_ggrs`'s `ResourceChecksumPlugin` takes `Res<R>` — so an
/// absent one is a frame-one panic rather than a skipped system.
/// ⇒ [`GgrsBackendPlugin`] is the backend alone. This plugin is that plus the
/// engine's declarations, which is what every engine composition wants and is
/// why its name and behaviour are unchanged.
pub struct AmbitionRollbackPlugin;

impl Plugin for AmbitionRollbackPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GgrsBackendPlugin);
        // Install the exact same domain/runtime declaration set whose metadata is
        // fingerprinted by the generic runtime.
        let mut registrar = GgrsRollbackRegistrar::new(app);
        ambition_platformer2d_runtime::rollback::register_engine_rollback_state(&mut registrar);
    }
}

/// The GGRS backend, and NOTHING about which domains exist.
///
/// ⭐ THE MINIMUM-HOST HALF of [`AmbitionRollbackPlugin`]. It selects the
/// concrete `GgrsSchedule`, installs `GgrsPlugin`, the session bridge and the
/// driver census — everything the backend itself owns — and declares no
/// gameplay state at all. A capability that wants rollback for ITS OWN types
/// composes this and then declares those types through the registrar, which is
/// what `examples/capability_demo` does.
///
/// ⛔ IT IS NOT A REDUCED ENGINE. A host that composes engine domains and this
/// plugin instead of [`AmbitionRollbackPlugin`] gets those domains WITHOUT their
/// rollback state, which is a desync, not a smaller game. The pairing is the
/// contract: engine domains go with the engine's declarations.
pub struct GgrsBackendPlugin;

impl Plugin for GgrsBackendPlugin {
    fn build(&self, app: &mut App) {
        let host = app
            .world()
            .get_resource::<ambition_platformer2d_runtime::SimulationHost>()
            .copied();
        assert_eq!(
            host,
            Some(ambition_platformer2d_runtime::SimulationHost::Rollback),
            "AmbitionRollbackPlugin may only be installed for SimulationHost::Rollback \
             (found {host:?})"
        );
        // The concrete backend owns the concrete schedule.
        app.set_sim_schedule(GgrsSchedule);

        app.add_plugins(GgrsPlugin::<AmbitionGgrsConfig>::default())
            .insert_resource(RollbackFrameRate(
                ambition_platformer2d_runtime::SIM_TICK_HZ as usize,
            ))
            .init_resource::<ambition_sim_view::PresentationPhase>()
            .insert_resource(ambition_platformer2d_runtime::RollbackHostReady);

        // The driver bracket, installed only when the workload census is on.
        // See `session::install_ggrs_driver_census` for why it lives in this
        // crate rather than beside the other censuses.
        crate::session::install_ggrs_driver_census(app);

        app.add_systems(
            Update,
            sample_ggrs_accumulator_phase
                .in_set(ambition_sim_view::presented_pose::PresentedPoseStage::SamplePhase),
        );

        app.edit_schedule(GgrsSchedule, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Ignore,
                ..default()
            });
        });

        app.configure_sets(
            LoadWorld,
            AmbitionLoadWorldSet::Reconcile.after(LoadWorldSystems::Mapping),
        )
        .add_systems(
            LoadWorld,
            reconcile::reconcile_brain_bindings.in_set(AmbitionLoadWorldSet::Reconcile),
        )
        .add_systems(SaveWorld, probes::record_saved_census)
        .add_systems(
            LoadWorld,
            probes::compare_restored_census.after(AmbitionLoadWorldSet::Reconcile),
        );

        session::install_session_bridge(app);
    }
}

/// One-shot composition plugin for the GGRS-backed platformer engine.
///
/// It establishes the semantic rollback host, installs the backend (which owns
/// the concrete `GgrsSchedule`), then assembles the ordinary content-free engine
/// group. The runtime itself never names GGRS.
pub struct RollbackEnginePlugin;

impl Plugin for RollbackEnginePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ambition_platformer2d_runtime::SimulationHost::Rollback);
        app.add_plugins(AmbitionRollbackPlugin);
        app.add_plugins(ambition_platformer2d_runtime::PlatformerEnginePlugins::new(
            ambition_platformer2d_runtime::SimulationHost::Rollback,
        ));
    }
}
