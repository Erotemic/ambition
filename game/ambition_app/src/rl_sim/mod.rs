//! Ambition's binding of the reusable [`ambition_sim_harness`] to its own content.
//!
//! The programmatic stepping seam — [`SandboxSim`], [`AgentAction`],
//! [`AgentObservation`], the example [`reward`] shaping, and the
//! [`random_policy`] fuzz driver — lives in the reusable `ambition_sim_harness`
//! crate (below this product shell). This module re-exports it and supplies the
//! ONE Ambition-specific piece: the composition that installs Ambition's content
//! + `SandboxSimulationPlugin` onto the harness App. External drivers (RL agents,
//! fuzz harnesses, replay tools) call `SandboxSim::new()` here; a demo/test with
//! DIFFERENT content calls `ambition_sim_harness::SandboxSim::build` with its own
//! composition, never linking this crate.
//!
//! ```no_run
//! use ambition_app::rl_sim::{AgentAction, AmbitionSim, SandboxSim};
//!
//! let mut sim = SandboxSim::new().expect("sim builds");
//! let mut action = AgentAction::default();
//! action.move_x = 1.0;
//! action.jump = true;
//! let obs = sim.step(action);
//! println!("after one tick: pos {:?}, on_ground {}", obs.player_pos, obs.on_ground);
//! ```

use bevy::prelude::App;

use crate::app::{SandboxSimulationPlugin, StartRoomOverride};

pub use ambition_sim_harness::{
    reward, AgentAction, AgentObservation, EnemyObs, Lcg, PickupObs, RandomWalkPolicy,
    RandomWalkTuning, SandboxSim, SandboxSimOptions, TimestepMode,
};

#[cfg(test)]
mod tests;

/// Compose Ambition's content onto a harness [`App`]: validate the embedded LDtk
/// world (a bad file is a hard `Err`, not a silent default), install the
/// provider world manifest, honor the programmatic `start_room` override, and add
/// the flagship `SandboxSimulationPlugin` (which composes the Ambition content
/// catalogs + the engine simulation group). Runs AFTER the harness has added the
/// engine foundation and chosen the sim schedule.
pub fn ambition_sim_composition(app: &mut App, options: &SandboxSimOptions) -> Result<(), String> {
    use ambition::actors::ldtk_world;
    // Provider-owned catalogs are composed as App-local resources by the
    // simulation plugin; the world manifest must be installed before validation.
    ambition_content::worlds::install();
    let project = ldtk_world::LdtkProject::load_default_for_dev()?;
    let report = project.validate();
    if !report.is_ok() {
        report.print_to_stderr();
        return Err(format!(
            "sandbox LDtk validation failed: {} error(s)",
            report.errors.len()
        ));
    }
    if let Err(errors) = project.to_room_set() {
        return Err(errors.join("; "));
    }
    // Programmatic start-room override: insert before SandboxSimulationPlugin
    // builds (its `init_sandbox_resources` consumes the override).
    if let Some(room_id) = options.start_room.clone() {
        app.insert_resource(StartRoomOverride(room_id));
    }
    app.add_plugins(SandboxSimulationPlugin);
    Ok(())
}

/// Ergonomic Ambition-composed constructors for the reusable [`SandboxSim`].
///
/// Bring this trait into scope to build a `SandboxSim` wired with Ambition's
/// content (`SandboxSim::new()` / `new_with_options` / `new_with_timestep`), the
/// same entry points the RL binaries and behavior/oracle tests use. Under the
/// hood each defers to [`SandboxSim::build`] with [`ambition_sim_composition`].
pub trait AmbitionSim: Sized {
    /// Build with the embedded LDtk world and the default wall-clock timestep.
    fn new() -> Result<Self, String>;
    /// Build with full options control (fixed timestep, start-room, …).
    fn new_with_options(options: SandboxSimOptions) -> Result<Self, String>;
    /// Build with the given timestep policy (see [`SandboxSim::build`]).
    fn new_with_timestep(timestep: TimestepMode) -> Result<Self, String>;
}

impl AmbitionSim for SandboxSim {
    fn new() -> Result<Self, String> {
        Self::new_with_options(SandboxSimOptions::default())
    }

    fn new_with_options(options: SandboxSimOptions) -> Result<Self, String> {
        SandboxSim::build(options, ambition_sim_composition)
    }

    fn new_with_timestep(timestep: TimestepMode) -> Result<Self, String> {
        Self::new_with_options(SandboxSimOptions {
            timestep,
            ..SandboxSimOptions::default()
        })
    }
}

/// **A DELIBERATELY-unregistered mutable sim resource, for the coverage-sensitivity
/// poison test only** (`desync_canary::the_coverage_ledger_reacts_to_a_new_unregistered_resource`).
///
/// The snapshot coverage ledger pins the *number* of unregistered `ambition_`
/// resources. A count-only pin is only trustworthy if the count actually moves when
/// real debt is added (audit M10). This type exists so the poison test can add exactly
/// that debt: it lives in an `ambition_`-named crate so its type name contains
/// `ambition_` — the exact `SnapshotRegistry::unclaimed_resources` filter — because
/// that is the shape of a real sim resource shipped without a codec. It is never
/// inserted by any system and only reachable under the `rl_sim` feature that the sim
/// harness itself requires.
#[doc(hidden)]
#[derive(bevy::prelude::Resource, Default)]
pub struct CoveragePoisonResource;
