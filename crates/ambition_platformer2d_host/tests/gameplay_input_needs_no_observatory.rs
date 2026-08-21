//! **Removing a developer instrument may not remove gameplay input.**
//!
//! Jon opened the served browser build on 2026-08-15 and found a split that
//! makes no sense until you know where the latch lived: arrow keys navigated the
//! menus, a gamepad navigated the menus, and neither moved the character. Not a
//! keycode problem — the same gamepad failed, and device input plainly reached
//! leafwing, since the menus responded to it.
//!
//! The device→tick latch table, `SlotControlLatches`, was installed by
//! `ambition_app::dev::rollback_observatory`, which is behind `dev_tools`. The
//! web persona does not enable `dev_tools`. So the browser composed a live GGRS
//! session with live device actions and NO latch, and
//! `capture_latched_local_input` takes it as `Option` — absent means "leave
//! `PendingLocalInput` alone", and its default is neutral. Seat zero therefore
//! told the simulation the player was holding nothing, every tick, forever.
//! Menus were unaffected because menu frames never enter the session.
//!
//! ⭐ **this crate is the enforcement.** `ambition_platformer2d_host` cannot
//! depend on `ambition_app`, so it cannot have a `dev_tools` feature and cannot
//! borrow an observatory — a composition assembled here is exactly the shape the
//! browser ships. If the latch ever migrates back to an instrument, this file
//! stops compiling a working host and says so.

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

    // ⚠ **ONE assertion where there were two.** Seat zero had its own
    // `ControlFrameLatch` resource beside this table and both had to be checked;
    // seat zero is row zero now.
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

/// The other half, so the assertion above is a claim and not a tautology: a
/// FRAME-STEPPED host must NOT get a latch. One rendered frame is one tick
/// there, so there is nothing to bridge, and an unconditional install would make
/// the test above pass while saying nothing about the rollback arm at all.
#[test]
fn a_frame_stepped_host_still_installs_no_latch_because_it_has_nothing_to_bridge() {
    let app = host_with_device(SimulationHost::RenderFrame);

    assert!(
        !app.world().contains_resource::<SlotControlLatches>(),
        "a frame-stepped host installed a frame→tick latch it has no use for; \
         the rollback assertion above is then vacuous"
    );
}

// ⚠ **INSTALLED IS NOT WIRED, and the other half of that is not proven here.**
// An untouched latch reports `is_device_authority() == false`, and
// `capture_latched_local_input` deliberately declines to publish one — so a
// resource registered without its accumulator reproduces the original bug while
// satisfying the presence check above. Splitting the two across the arms is
// exactly the half-move that caused this.
//
// It does not belong in this file. Stepping the plugin's `Update` needs session
// state the host group does not own (`declare_in_session_input_contexts` wants
// `ActiveCutscene`), and hand-feeding those resources builds a fixture that
// models nothing — the composition it would prove is one nobody ships. The real
// web composition already boots natively in `ambition_app`'s `web_persona_boot`
// example, which asserts the latch has been FED after startup, under the
// browser's Cargo features and with no dev tooling anywhere. That is the same
// claim against the actual thing.
