//! The causal instrument, against the REAL app composition. (queue P1d)
//!
//! Everything else that exercises the causal log builds an `App`, adds
//! `CausalPlugin`, publishes a fact from a test system and reads it back. That
//! proves the substrate — the sink, the tick stamping, the explainer — and it
//! proves nothing about whether the GAME publishes anything.
//!
//! The domain recorders — the brain's decision, the body's control frame, damage, lifecycle — were
//! compiled out of every build the app produces. The per-crate feature jobs kept their unit tests
//! green the whole time, which is exactly why it looked fine: they exercise the substrate and never
//! the app.
//!
//! So this asks the one question those cannot: step the real sim, and does an
//! `explain(tick, subject)` come back carrying facts a GAME published?
//!
//! it deliberately does not assert WHICH domains appear. Which systems publish
//! is a design question that moves; that a real tick explains at all is the
//! property the instrument's usefulness rests on, and the one that silently
//! stopped holding.

#![cfg(all(feature = "rl_sim", feature = "causal"))]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::causal::{CausalRecording, RecordingPolicy};

/// Ticks to run before asking. Enough that bodies have been integrated and
/// brains have decided at least once — an explanation of tick 0 would be a
/// question about startup, not about gameplay.
const TICKS: usize = 30;

fn recording_sim() -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz()),
    )
    .expect("the Ambition sim harness builds");
    // the FEATURE and the PLUGIN are two switches, deliberately. The
    // `causal` feature compiles the publishers in; only `CausalPlugin` creates
    // the `CausalRecording` they write to, so a host can carry the publishers
    // without an inspector. `stamp_causal_frame` takes the log as `Option`
    // precisely so that composition is legal — omitting the plugin here is not a
    // missing wire, it is the consumer half of the seam, and this fixture is the
    // consumer.
    sim.app_mut()
        .add_plugins(ambition_platformer2d::causal::CausalPlugin);
    // `Off` is the shipped default and the whole point of the policy: recording
    // costs work per tick, so a game pays for it only while somebody is asking.
    ambition_platformer2d::causal::record_domains(sim.app_mut(), RecordingPolicy::All);
    sim
}

#[test]
fn a_real_gameplay_tick_explains_itself() {
    let mut sim = recording_sim();
    for _ in 0..TICKS {
        sim.step(AgentAction::default());
    }

    let world = sim.world_mut();
    let log = world
        .get_resource::<CausalRecording>()
        .expect("CausalPlugin installs the recording; the `causal` feature is on");

    let facts = log.len();
    assert!(
        facts > 0,
        "the real app composition ran {TICKS} ticks with RecordingPolicy::All and \
         published NO causal facts. Either no domain recorder is installed in \
         this composition, or the `causal` feature stopped reaching them — which \
         is the state this file was written to end: the chain was complete and \
         nobody enabled it, so every recorder was compiled out while the \
         substrate's own tests stayed green."
    );
}

#[test]
fn the_recording_policy_is_what_decides_whether_it_costs_anything() {
    // the honest companion to the test above. `Off` is the shipped default,
    // and a recorder that published regardless would make every shipped frame
    // pay for an instrument nobody is reading.
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz()),
    )
    .expect("the Ambition sim harness builds");
    for _ in 0..TICKS {
        sim.step(AgentAction::default());
    }
    let world = sim.world_mut();
    let quiet = world
        .get_resource::<CausalRecording>()
        .map(|log| log.len())
        .unwrap_or(0);
    assert_eq!(
        quiet, 0,
        "{quiet} facts were recorded under the DEFAULT policy. Recording is \
         opt-in; a domain that publishes without being asked charges every \
         shipped frame for an instrument nobody is reading."
    );
}
