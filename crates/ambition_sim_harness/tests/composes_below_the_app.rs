//! Track 4 exit gate: a test drives a real sim through the harness while
//! linking only the reusable engine surface (the `ambition_platformer2d` facade) — never
//! `ambition_app`, the product shell. If this compiles and runs, the harness is
//! genuinely below the app: any demo/provider can compose its own sim into it.
//!
//! The composition here is deliberately minimal and content-free — a single
//! empty room seeded as the session world — so the proof is about the *seam*, not
//! about any particular game's content. Ambition's own 30+ behavior/oracle suites
//! exercise the same `Platformer2dSimHarness` with the full Ambition composition (via
//! `ambition_app::rl_sim::AmbitionSim`).

use ambition_platformer2d::session::insert_session_world_component;
use ambition_platformer2d::sim::ControlFrame;
use ambition_platformer2d::world::{
    prelude::{AuthoredWorld, Vec2},
    rooms::{RoomSet, RoomSpec},
};

use ambition_sim_harness::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions};

/// Compose a bare one-room session onto the harness App: seed the session
/// `RoomSet` the observation reads, and register the `ControlFrame` the stepper
/// writes. No content crate, no product shell — only reusable engine APIs.
fn compose_minimal_room(
    app: &mut bevy::prelude::App,
    _options: &Platformer2dSimHarnessOptions,
) -> Result<(), String> {
    let world = AuthoredWorld::new(
        "harness_room",
        Vec2::new(800.0, 600.0),
        Vec2::new(100.0, 100.0),
        Vec::new(),
    );
    let set = RoomSet::from_parts(
        "harness_room",
        vec![RoomSpec::new("harness_room", world)],
        Vec::new(),
    );
    insert_session_world_component(app.world_mut(), set);
    app.insert_resource(ControlFrame::default());
    Ok(())
}

#[test]
fn a_minimal_sim_runs_through_the_harness_without_the_product_shell() {
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default(),
        compose_minimal_room,
    )
    .expect("the harness composes a minimal sim");

    // The seeded session is observable through the harness read-model.
    let obs = sim.observation();
    assert_eq!(obs.active_room, "harness_room");
    assert_eq!(obs.tick, 0, "a fresh sim is at tick 0");

    // Stepping advances the tick and re-reads the observation — the reset/step
    // loop drives the composed App end to end.
    let mut action = AgentAction::default();
    action.move_x = 1.0;
    let obs = sim.step(action);
    assert_eq!(obs.tick, 1, "one step advances the harness tick");
    assert_eq!(obs.active_room, "harness_room");

    let obs = sim.step_n(AgentAction::default(), 5);
    assert_eq!(obs.tick, 6, "step_n advances the tick by n");
    assert_eq!(sim.tick_count(), 6);
}

#[test]
fn reset_episode_drives_the_harness_reset_edge() {
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default(),
        compose_minimal_room,
    )
    .expect("the harness composes a minimal sim");
    sim.step(AgentAction::default());
    // reset_episode presses the in-sim reset edge and drains it; it returns a
    // fresh observation without rebuilding the App.
    let obs = sim.reset_episode();
    assert_eq!(obs.active_room, "harness_room");
}
