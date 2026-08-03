//! **The rollback plugin may only be added under a GGRS host.**
//!
//! Both directions, because a guard that only ever sees its passing case is a
//! guard whose failing case has never been observed.

use bevy::prelude::App;

use crate::rollback::AmbitionRollbackPlugin;
use crate::{SimulationHost, SimulationHostAppExt as _};

/// The PROBE: a composition that installs rollback under a fixed-tick host is
/// exactly the silent half-configuration the assert exists to stop, so it must
/// panic rather than build a session over recorded-only registrations.
#[test]
#[should_panic(expected = "simulation host is Ggrs")]
fn installing_rollback_under_a_fixed_host_is_refused() {
    let mut app = App::new();
    app.set_simulation_host(SimulationHost::Fixed60Hz);
    app.add_plugins(AmbitionRollbackPlugin);
}

/// And with no host chosen at all — the case that reads as "nobody decided",
/// which is the one a default would quietly paper over.
#[test]
#[should_panic(expected = "simulation host is Ggrs")]
fn installing_rollback_with_no_host_chosen_is_refused() {
    let mut app = App::new();
    app.add_plugins(AmbitionRollbackPlugin);
}
