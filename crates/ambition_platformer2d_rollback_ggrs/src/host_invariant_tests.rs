//! The backend only runs under the explicit semantic rollback host.

use bevy::prelude::App;
use ambition_platformer2d_runtime::{SimulationHost, SimulationHostAppExt as _};

use crate::AmbitionRollbackPlugin;

#[test]
#[should_panic(expected = "SimulationHost::Rollback")]
fn installing_ggrs_under_a_fixed_host_is_refused() {
    let mut app = App::new();
    app.set_simulation_host(SimulationHost::Fixed60Hz);
    app.add_plugins(AmbitionRollbackPlugin);
}

#[test]
#[should_panic(expected = "SimulationHost::Rollback")]
fn installing_ggrs_with_no_host_chosen_is_refused() {
    let mut app = App::new();
    app.add_plugins(AmbitionRollbackPlugin);
}

#[test]
fn the_backend_turns_semantic_rollback_into_the_ggrs_schedule() {
    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;
    use bevy_ggrs::GgrsSchedule;

    let mut app = App::new();
    app.set_simulation_host(SimulationHost::Rollback);
    app.add_plugins(AmbitionRollbackPlugin);

    assert!(app.sim_is(GgrsSchedule));
    assert!(
        app.world()
            .contains_resource::<ambition_platformer2d_runtime::RollbackHostReady>(),
        "the backend selected GgrsSchedule but did not publish RollbackHostReady"
    );
}

/// ⛔⛔ THE ENGINE GROUP MUST NOT COMPOSE ON A BACKEND THAT DECLARED NOTHING.
///
/// Since the backend split (2026-09-04) `GgrsBackendPlugin` installs GGRS and
/// declares no gameplay state — correct for a capability host that composes no
/// engine domains, and a SILENT DESYNC for one that composes the engine group,
/// because every plugin involved still builds successfully.
///
/// ⭐ `RollbackHostReady` cannot catch it: the bare backend publishes that too,
/// and rightly — it IS a ready host. The second marker exists because "a backend
/// is installed" and "the engine's state is declared" are different facts, and
/// only the second one is what the engine group depends on.
#[test]
#[should_panic(expected = "declared no engine")]
fn the_engine_group_refuses_a_backend_that_declared_no_engine_state() {
    let mut app = App::new();
    app.set_simulation_host(SimulationHost::Rollback);
    app.add_plugins(crate::GgrsBackendPlugin);
    app.add_plugins(
        ambition_platformer2d_runtime::Platformer2dSimulationFoundationPlugin {
            host: SimulationHost::Rollback,
        },
    );
}

/// The same composition with the PAIRED plugin builds — so the guard above
/// cannot pass by refusing everything.
#[test]
fn the_engine_group_composes_on_the_paired_plugin() {
    let mut app = App::new();
    app.set_simulation_host(SimulationHost::Rollback);
    app.add_plugins(AmbitionRollbackPlugin);
    app.add_plugins(
        ambition_platformer2d_runtime::Platformer2dSimulationFoundationPlugin {
            host: SimulationHost::Rollback,
        },
    );
    assert!(app
        .world()
        .contains_resource::<ambition_platformer2d_runtime::EngineRollbackStateDeclared>());
}
