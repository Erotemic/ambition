//! Ambition's binding of the reusable [`ambition_sim_harness`] to its own content.
//!
//! The programmatic stepping seam — [`Platformer2dSimHarness`], [`AgentAction`],
//! [`AgentObservation`], the example [`reward`] shaping, and the
//! [`random_policy`] fuzz driver — lives in the reusable `ambition_sim_harness`
//! crate (below this product shell). This module re-exports it and supplies the
//! ONE Ambition-specific piece: the composition that installs Ambition's content
//! + `AmbitionGameSimulationPlugin` onto the harness App. External drivers (RL agents,
//! fuzz harnesses, replay tools) call `Platformer2dSimHarness::new()` here; a demo/test with
//! DIFFERENT content calls `ambition_sim_harness::Platformer2dSimHarness::build` with its own
//! composition, never linking this crate.
//!
//! ```no_run
//! use ambition_app::rl_sim::{AgentAction, AmbitionSim, Platformer2dSimHarness};
//!
//! let mut sim = Platformer2dSimHarness::new().expect("sim builds");
//! let mut action = AgentAction::default();
//! action.move_x = 1.0;
//! action.jump = true;
//! let obs = sim.step(action);
//! println!("after one tick: pos {:?}, on_ground {}", obs.player_pos, obs.on_ground);
//! ```

use bevy::prelude::App;

use crate::app::StartRoomOverride;

pub use ambition_sim_harness::{
    AgentAction, AgentObservation, EnemyObs, Lcg, PickupObs, Platformer2dSimHarness,
    Platformer2dSimHarnessOptions, RandomWalkPolicy, RandomWalkTuning, RollbackMode, TimestepMode,
    reward,
};

#[cfg(test)]
mod tests;

/// Compose Ambition's content onto a harness [`App`]: validate the embedded LDtk
/// world (a bad file is a hard `Err`, not a silent default), install the
/// provider world manifest, honor the programmatic `start_room` override, and add
/// the flagship `AmbitionGameSimulationPlugin` (which composes the Ambition content
/// catalogs + the engine simulation group). Runs AFTER the harness has added the
/// engine foundation and chosen the sim schedule.
pub fn ambition_sim_composition(
    app: &mut App,
    options: &Platformer2dSimHarnessOptions,
) -> Result<(), String> {
    use ambition_platformer2d::ldtk_map as ldtk_world;
    // Provider-owned catalogs are composed as App-local resources by the
    // simulation plugin; validation reads the provider's manifest directly.
    let world_manifest = ambition_content::worlds::world_manifest();
    let project = ldtk_world::LdtkProject::load_default_for_dev(&world_manifest)?;
    let report = project.validate(&crate::composed_ldtk_vocabulary());
    if !report.is_ok() {
        report.print_to_stderr();
        return Err(format!(
            "sandbox LDtk validation failed: {} error(s)",
            report.errors.len()
        ));
    }
    if let Err(errors) = project.to_room_set(&world_manifest, &crate::composed_ldtk_vocabulary()) {
        return Err(errors.join("; "));
    }
    // Programmatic start-room override: insert before AmbitionGameSimulationPlugin
    // builds (its `init_sandbox_resources` consumes the override).
    if let Some(room_id) = options.start_room.clone() {
        app.insert_resource(StartRoomOverride(room_id));
        // Tolerance stays the default because it is a promise a test is named after; a caller
        // that means "this room must be there" says so with `with_required_start_room` and gets
        // the loud failure, which lists every valid id.
        if options.start_room_must_resolve {
            app.insert_resource(crate::app::StartRoomMustResolve);
        }
    }
    // Same kind of composition input, and it has to arrive here for the same
    // reason: `init_sandbox_resources` consumes it while building the prepared
    // world, long before any roster is published.
    if options.seats_a_match {
        app.insert_resource(crate::app::SeatsAMatchInsteadOfAHomeBody);
    }
    // the resource goes in BEFORE the plugin builds. It is what stopped
    // `publish_direct_prepared_session_root` publishing a root, and now that the
    // publisher is gone it is what tells the rest of the app which composition
    // this is.
    crate::app::shell_host::compose_ambition_gameplay_host(app);
    // recorded that as an open residue, and it is the same shape as every other "works only
    // when somebody is watching" gap: the durable horizon is SIM state (its own row: *"the
    // on-disk form IS the checkpoint's own description, serialized"*), so a composition that
    // simulates should be able to persist.
    //
    // `isolated()` is not optional here. `PersistenceRoot::default()` is
    // the PLAYER's platform data dir, so installing the writer without a root of
    // its own would point every headless run at the user's real save. The
    // windowless CLI host already redirects the same way, for the same reason
    // audio goes to `AudioOutputMode::Recording` rather than the speakers.
    app.insert_resource(ambition_platformer2d::persistence::PersistenceRoot::isolated());
    app.add_plugins(ambition_platformer2d::persistence::PersistenceSchedulePlugin);
    Ok(())
}

/// Ergonomic Ambition-composed constructors for the reusable [`Platformer2dSimHarness`].
///
/// Bring this trait into scope to build a `Platformer2dSimHarness` wired with Ambition's
/// content (`Platformer2dSimHarness::new()` / `new_with_options` / `new_with_timestep`), the
/// same entry points the RL binaries and behavior/oracle tests use. Under the
/// hood each defers to [`Platformer2dSimHarness::build`] with [`ambition_sim_composition`].
pub trait AmbitionSim: Sized {
    /// Build with the embedded LDtk world and the default wall-clock timestep.
    fn new() -> Result<Self, String>;
    fn new_with_options(options: Platformer2dSimHarnessOptions) -> Result<Self, String>;
    /// Build with the given timestep policy (see [`Platformer2dSimHarness::build`]).
    fn new_with_timestep(timestep: TimestepMode) -> Result<Self, String>;
}

impl AmbitionSim for Platformer2dSimHarness {
    fn new() -> Result<Self, String> {
        Self::new_with_options(Platformer2dSimHarnessOptions::default())
    }

    fn new_with_options(options: Platformer2dSimHarnessOptions) -> Result<Self, String> {
        Platformer2dSimHarness::build(options, ambition_sim_composition)
    }

    fn new_with_timestep(timestep: TimestepMode) -> Result<Self, String> {
        Self::new_with_options(Platformer2dSimHarnessOptions {
            timestep,
            ..Platformer2dSimHarnessOptions::default()
        })
    }
}
