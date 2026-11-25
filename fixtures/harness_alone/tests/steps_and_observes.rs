//! A harness-alone consumer steps a sim and reads it back (CB4).

use ambition_sim_harness::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions};

#[test]
fn a_harness_alone_consumer_steps_and_observes_a_sim() {
    let mut sim = Platformer2dSimHarness::build(Platformer2dSimHarnessOptions::default(), harness_alone::compose)
        .expect("the harness composes this consumer's room");
    assert_eq!(sim.observation().active_room, harness_alone::ROOM);
    let mut action = AgentAction::default();
    action.move_x = 1.0;
    assert_eq!(sim.step(action).tick, 1, "one step advances the tick");
    let obs = sim.step_n(AgentAction::default(), 9);
    assert_eq!((obs.tick, obs.active_room.as_str()), (10, harness_alone::ROOM), "ten steps, the same room");
}
