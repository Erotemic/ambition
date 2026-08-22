//! Gameplay input must not depend on developer instrumentation.
//! A rollback host owns `SlotControlLatches`; frame-stepped hosts do not need the
//! device-to-tick bridge.

#![cfg(feature = "input")]

use bevy::prelude::*;

use ambition_platformer2d_runtime::host_input::SlotControlLatches;
use ambition_platformer2d_runtime::SimulationHost;

/// A host with a device and nothing else: no app crate, no dev tooling, no
/// observatory. `SimulationHost` is inserted rather than set through
/// `set_simulation_host`, because that seals the GGRS schedule and this crate
/// deliberately does not depend on `bevy_ggrs` — the plugin asks the same
/// question the same way (`SimulationHost::is_rollback`).
fn host_with_device(sim_host: SimulationHost) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(sim_host);
    app.add_plugins(ambition_platformer2d_host::HostInputBindingsPlugin);
    app
}

#[test]
fn a_rollback_host_owns_the_primary_device_latch_without_any_developer_tooling() {
    let app = host_with_device(SimulationHost::Rollback);

    // Seat zero is row zero in the shared latch table.
    assert!(
        app.world().contains_resource::<SlotControlLatches>(),
        "a GGRS host composed the device input bridge and installed no \
         `SlotControlLatches`. `capture_latched_local_input` treats the missing \
         latch as 'nobody feeds me' and leaves `PendingLocalInput` at its neutral \
         default, so the session publishes a motionless seat zero every tick — \
         the browser's 'menus respond, character does not move' bug. The device \
         host owns this bridge; a developer instrument may not."
    );
}

/// Frame-stepped hosts do not need a device-to-tick latch.
#[test]
fn a_frame_stepped_host_still_installs_no_latch_because_it_has_nothing_to_bridge() {
    let app = host_with_device(SimulationHost::RenderFrame);

    assert!(
        !app.world().contains_resource::<SlotControlLatches>(),
        "a frame-stepped host installed a frame→tick latch it has no use for; \
         the rollback assertion above is then vacuous"
    );
}

// Splitting the two across the arms is exactly the half-move that caused this.
//
// It does not belong in this file. Stepping the plugin's `Update` needs session
// state the host group does not own (`declare_in_session_input_contexts` wants
// `ActiveCutscene`), and hand-feeding those resources builds a fixture that
// models nothing — the composition it would prove is one nobody ships. The real
// web composition already boots natively in `ambition_app`'s `web_persona_boot`
// example, which asserts the latch has been FED after startup, under the
// browser's Cargo features and with no dev tooling anywhere. That is the same
// claim against the actual thing.
