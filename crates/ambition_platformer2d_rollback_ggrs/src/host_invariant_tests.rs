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
