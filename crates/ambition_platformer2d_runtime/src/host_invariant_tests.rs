//! Composition invariants owned by the generic simulation host.

use bevy::prelude::App;

use crate::{Platformer2dSimulationFoundationPlugin, SimulationHost};

#[test]
#[should_panic(expected = "SimulationHost::Rollback requires a concrete rollback backend")]
fn semantic_rollback_without_a_backend_is_refused_by_the_engine_foundation() {
    let mut app = App::new();
    app.add_plugins(Platformer2dSimulationFoundationPlugin {
        host: SimulationHost::Rollback,
    });
}
