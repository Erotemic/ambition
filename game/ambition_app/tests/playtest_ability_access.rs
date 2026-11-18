//! Developer ability access stays active in authored progression rooms.

#![cfg(feature = "rl_sim")]

use ambition_app::{AmbitionSim, AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::dev_tools::dev_tools::EditableAbilitySet;
use ambition_platformer2d::engine_core::BodyAbilities;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;

#[test]
fn hobford_keeps_the_playtest_kit_until_the_developer_turns_it_off() {
    let mut sim = Platformer2dSimHarness::new_with_options(crate::common::fixed_60hz_room_options("intro_hobford")).unwrap();
    sim.step_n(AgentAction::default(), 30);
    let kit = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world.query_filtered::<&BodyAbilities, PrimaryPlayerOnly>().single(world).unwrap().abilities
    };
    assert!(sim.world().resource::<EditableAbilitySet>().unlock_all_abilities);
    let unlocked = kit(&mut sim);
    assert!(unlocked.fly && unlocked.blink && unlocked.double_jump);
    sim.world_mut().resource_mut::<EditableAbilitySet>().unlock_all_abilities = false;
    sim.step_n(AgentAction::default(), 5);
    let limited = kit(&mut sim);
    assert!(!limited.fly && !limited.blink && !limited.double_jump);
    sim.world_mut().resource_mut::<EditableAbilitySet>().unlock_all_abilities = true;
    sim.step_n(AgentAction::default(), 5);
    let restored = kit(&mut sim);
    assert!(restored.fly && restored.blink && restored.double_jump);
}
