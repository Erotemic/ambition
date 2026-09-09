#![cfg(all(feature = "input", feature = "visible"))]

//! ⛔⛔ **C2's MOVED INSTALLER WAS UNFALSIFIABLE ON THE APPLICATION-TEST ROAD.**
//!
//! `61f297e` moved the registration of `publish_latched_slot_controls` — its sim
//! phase, its fixed-tick condition and its `.before(InputSet::Route)` edge — out
//! of the host and into `install_latched_slot_publication`, beside the system
//! whose facts those are. The ownership decision is right. The COVERAGE was not:
//! the row's own poison recorded that all 602 `app_it` tests survive deleting
//! the installer outright, so nothing on the shipped road asserted that a latched
//! frame ever reaches the simulation at all.
//!
//! It survives because `drive_slot_frame` — which is how those tests introduce
//! input — writes `SlotControls` DIRECTLY when the composition has no latch.
//! That is a legitimate convenience for a fixture with no frame→tick seam, and
//! it means a test using it cannot witness the seam either way.
//!
//! ⭐ SO THIS FILE INTRODUCES INPUT THE WAY A DEVICE DOES: it accumulates into
//! `SlotControlLatches` on the FRAME clock and never calls the driving helper.
//! The only thing that can move that frame into `SlotControls` is the drain
//! running on the fixed tick — `publish_seat_controls_when_nobody_else_does`
//! deliberately stands down while latches exist (`another_authority_publishes`),
//! so with the installer gone nothing publishes and the press is simply lost.
//!
//! ⚠ `Fixed60Hz`, NOT `Rollback`. Both are fixed-tick, but a rollback host
//! drains the same latches at `ReadInputs` through `capture_latched_local_input`,
//! which would publish the frame with this installer deleted and make the test
//! green for the wrong reason. `Fixed60Hz` is the composition where this
//! registration is the only road.
//!
//! ⛔⛔ **AND THE SIMULATION HAS TO BE AUTHORIZED, WHICH IS A THIRD PREMISE.**
//! The drain sits in `Platformer2dSimulationPhaseMonolith::PlayerInput`, under
//! `GameplaySimulationRoot`, which runs only while `simulation_authorized` finds
//! a LIVE SESSION SCOPE. Written first against the shell host, this file failed
//! with the seat still neutral — not because the drain was missing but because
//! the shell boots to a launcher with no session, so the whole phase never ran.
//! A test that cannot tell those two apart would have been recorded as a defect
//! in the very installer it exists to defend. Hence the PROBE below: a system in
//! the same set, asserted to have run, so "the sim did not run" and "the drain is
//! not installed" are separate answers.
//!
//! No `SessionGatedSimulation` here, so `live_scope_of` takes its direct-entry
//! branch and the one spawned `SessionRoot` IS the authority.

use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::transform::TransformPlugin;
use bevy::MinimalPlugins;

use ambition_platformer2d::characters::control::{
    PlayerSlot, SlotControlLatches, SlotControls,
};
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::lifecycle::{SessionRoot, SessionScopeId};
use ambition_platformer2d::sim::Platformer2dSimulationPhaseMonolith;

const SEAT: PlayerSlot = PlayerSlot(0);

/// Counts ticks on which the phase the drain lives in actually ran.
#[derive(Resource, Default)]
struct PhaseRan(u32);

fn note_the_phase_ran(mut ran: ResMut<PhaseRan>) {
    ran.0 += 1;
}

/// The shipped fixed-tick composition, headless.
fn fixed_tick_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<ambition_platformer2d::platformer::schedule::GameMode>();
    use ambition_platformer2d::runtime::SimulationHostAppExt as _;
    app.set_simulation_host(ambition_platformer2d::runtime::SimulationHost::Fixed60Hz);
    ambition_app::app::init_sandbox_resources(&mut app);
    ambition_app::app::add_simulation_plugins(&mut app);
    app.add_plugins(ambition_platformer2d::host::PlatformerHostPlugins);

    // The live session scope the gameplay phases require. Direct-entry: no
    // `SessionGatedSimulation` is composed here, so this single root is the
    // authority `live_scope_of` looks for.
    app.world_mut().spawn(SessionRoot(SessionScopeId(1)));

    // The probe, in the SAME set as the drain, so a dormant simulation is
    // distinguishable from a missing registration.
    use ambition_platformer2d::sim::SimScheduleExt as _;
    let sim = app.sim_schedule();
    app.init_resource::<PhaseRan>();
    app.add_systems(
        sim,
        note_the_phase_ran.in_set(Platformer2dSimulationPhaseMonolith::PlayerInput),
    );
    app
}

fn phase_ran(app: &App) -> u32 {
    app.world().resource::<PhaseRan>().0
}

/// Advance without letting the fixed clock move: FRAME schedules only.
fn frame_only(app: &mut App) {
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ));
    app.update();
}

/// Advance by exactly one fixed timestep, so the tick schedules run.
fn one_tick(app: &mut App) {
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app.update();
}

fn press() -> ControlFrame {
    ControlFrame {
        axis_x: 1.0,
        right_pressed: true,
        ..Default::default()
    }
}

fn published_axis(app: &App) -> f32 {
    app.world().resource::<SlotControls>().get(SEAT).axis_x
}

#[test]
fn a_latched_frame_reaches_the_simulation_only_on_the_fixed_tick() {
    let mut app = fixed_tick_app();
    frame_only(&mut app);

    // ⛔ THE PREMISES, ASSERTED. Each one, if false, makes every assertion below
    // pass for a reason that has nothing to do with the installer.
    use ambition_platformer2d::sim::SimScheduleExt as _;
    assert!(
        app.sim_is_fixed_tick(),
        "this composition is not fixed-tick, so `install_latched_slot_publication` \
         returns early and there is no drain to test"
    );
    assert!(
        app.world().get_resource::<SlotControlLatches>().is_some(),
        "no `SlotControlLatches` in this composition — a device would have \
         nothing to latch into and `drive_slot_frame` would write `SlotControls` \
         directly, which is exactly the bypass this file exists to avoid"
    );
    assert_eq!(
        published_axis(&app),
        0.0,
        "the seat is not neutral at boot, so 'it became pressed' proves nothing"
    );
    assert_eq!(
        phase_ran(&app),
        0,
        "the gameplay phase ran on a FRAME update; this composition is supposed \
         to run it only on the fixed tick"
    );

    // A DEVICE'S DELIVERY: the frame clock, straight into the latch. No
    // `drive_slot_frame`, which is the helper that can bypass the seam.
    app.world_mut()
        .resource_mut::<SlotControlLatches>()
        .accumulate(SEAT, press());

    // ⛔ THE "EXTRA FRAME" VERSION OF HALF ONE IS A CONFOUND, AND MEASURING IT
    // SAID SO. Re-running the frame clock here to show the seat still neutral
    // looks like the natural check and is not one: `populate_seat_control_frames`
    // REBUILDS every seat's latch from that participant's `ActionState` on each
    // frame, so a synthetic accumulation with no device behind it is overwritten
    // before the tick ever sees it. The test then fails with the seat neutral
    // and blames the installer — which is exactly the misreading this file
    // exists to prevent, so it is not written that way.
    //
    // The unconfounded form of the same claim is the probe premise above: the
    // gameplay phase, and therefore the drain, does not run on a frame update at
    // all. `phase_ran == 0` after `frame_only` is that assertion.

    // ⭐ AND THE TICK DRAINS IT. This is the assertion that dies when
    // `install_latched_slot_publication` is deleted.
    one_tick(&mut app);
    // ⛔ THE THIRD PREMISE, and the one that cost this file a rewrite: if the
    // phase never ran, nothing below is a statement about the installer.
    assert!(
        phase_ran(&app) > 0,
        "the simulation phase the drain lives in never ran, so this test cannot \
         speak about `install_latched_slot_publication` at all. `GameplaySimulationRoot` \
         needs `simulation_authorized` to find a live session scope"
    );
    assert_eq!(
        published_axis(&app),
        1.0,
        "the latched frame never reached `SlotControls` after a fixed tick. \
         Nothing else publishes while latches exist — \
         `publish_seat_controls_when_nobody_else_does` stands down via \
         `another_authority_publishes` — so the seat's input is simply lost, \
         which is the state 602 app_it tests could not see"
    );
}

/// ⭐ CONTROL. The drain TAKES the latch, so a press is spent by the tick that
/// published it rather than repeating until something clears it. Without this,
/// a "publisher" that copied a never-emptied latch would satisfy the test above
/// while turning every tap into a hold.
#[test]
fn the_tick_spends_the_latch_it_published() {
    let mut app = fixed_tick_app();
    frame_only(&mut app);
    app.world_mut()
        .resource_mut::<SlotControlLatches>()
        .accumulate(SEAT, press());
    one_tick(&mut app);
    assert!(phase_ran(&app) > 0, "premise: the gameplay phase ran");
    assert_eq!(published_axis(&app), 1.0, "premise: the press was published");

    one_tick(&mut app);
    assert_eq!(
        published_axis(&app),
        0.0,
        "the seat is still holding right on the tick after the press was \
         published: the drain is copying the latch rather than taking it, so a \
         tap becomes a hold"
    );
}
