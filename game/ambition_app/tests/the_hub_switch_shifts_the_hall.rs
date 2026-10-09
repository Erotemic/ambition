//! The central hub has a switch that changes the state of its look: balanced,
//! corrupt, pure.
//!
//! The state is a fact in the save (`ambition_content::room_look_state`), and
//! the switch changes it with an authored verb, `look.cycle`. The look itself
//! is presentation and is not here; what is here is the road a press takes.

use ambition_content::room_look_state::RoomLookState;
use ambition_platformer2d::persistence::save::AmbitionGameSave;
use ambition_platformer2d::platformer::authored_logic::{
    AuthoredArg, AuthoredAsk, CommandCatalog, CommandId, RunAuthoredCommand,
};

use crate::common::{base, fixed_60hz_room_sim};

const HUB: &str = "central_hub_complex";

fn state(sim: &ambition_app::Platformer2dSimHarness) -> RoomLookState {
    RoomLookState::of(sim.world().resource::<AmbitionGameSave>(), HUB)
}

/// The hub's switch authors `look.cycle`, and the engine it is loaded into
/// knows that verb with the argument the switch gives it. A line the catalog
/// does not know is a switch that does nothing, with no test to notice.
#[test]
fn the_hub_authors_a_state_switch_the_engine_can_run() {
    let mut sim = fixed_60hz_room_sim(HUB);
    sim.step_n(base(), 4);

    let line = {
        let world = sim.world_mut();
        let mut query = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let rooms = query.iter(world).next().expect("the session has a room set");
        let hub = rooms
            .definition_by_id(HUB)
            .map(|definition| rooms.spec(definition))
            .expect("the hub is a room of the shipped world");
        hub.switch_commands
            .iter()
            .find(|command| command.switch_id == "hub_state_switch")
            .map(|command| command.line.clone())
            .expect("the hub authors a switch with id `hub_state_switch` and an `on_activate` line")
    };
    assert_eq!(line, format!("look.cycle {HUB}"));

    let cycle = CommandId::new("look", "cycle");
    assert!(
        sim.world().resource::<CommandCatalog>().describe(&cycle).is_some(),
        "the composed engine does not publish `look.cycle`: `RoomLookStatePlugin` is not installed"
    );
}

/// Three presses go round: a room nobody touched is balanced, then corrupt,
/// then pure, then balanced again. Each press goes through the command
/// dispatcher and the world-fact domain's own request, to the save.
#[test]
fn three_presses_take_the_hub_round_its_three_states() {
    let mut sim = fixed_60hz_room_sim(HUB);
    sim.step_n(base(), 4);
    assert_eq!(state(&sim), RoomLookState::Balanced, "a new save has a balanced hub");

    let mut seen = Vec::new();
    for _ in 0..3 {
        sim.world_mut().write_message(RunAuthoredCommand::new(
            CommandId::new("look", "cycle"),
            vec![AuthoredArg::Name(HUB.to_string())],
            AuthoredAsk::new("probe", "a test"),
        ));
        // The dispatcher runs before `GameplayEffects`, which applies the two
        // flag requests on the same tick. A second tick is margin, not need.
        sim.step_n(base(), 2);
        seen.push(state(&sim));
    }
    assert_eq!(
        seen,
        [RoomLookState::Corrupt, RoomLookState::Pure, RoomLookState::Balanced]
    );
}
