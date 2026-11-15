#![cfg(feature = "rl_sim")]
//! STORY SEQUENCES SS1 + SS2, in level 1's raid: when two raiders have
//! fallen, a cutscene that starts on that condition (`starts_when:
//! actor.fallen_is npc_lab_raider 2`) opens the yard gate, sends the last raider out through
//! the lane edge with the case (`actor.leave_through`), and waits until he is
//! gone (`WaitFor actor.gone`). His going wins the raid. All of it is content
//! (`data/cutscenes/intro.ron`); the player fights with its own inputs.

use ambition_app::rl_sim::AmbitionSim as _;
use bevy::prelude::World;

use crate::common::walk::{through_the_raid, Pad, Press, RaidStart, Walker as _};
use crate::common::{base, fixed_60hz_room_options};

fn raiders(world: &mut World) -> (usize, usize) {
    let mut bodies = world.query::<(
        &ambition_platformer2d::characters::actor::CharacterForm,
        Option<&ambition_platformer2d::characters::actor::BodyHealth>,
    )>();
    let mut live = 0;
    let mut fallen = 0;
    for (form, health) in bodies.iter(world) {
        if form.id() != "npc_lab_raider" {
            continue;
        }
        if health.is_none_or(|health| health.health.current > 0) {
            live += 1;
        } else {
            fallen += 1;
        }
    }
    (live, fallen)
}

fn flag(world: &World, id: &str) -> bool {
    world
        .get_resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .is_some_and(|save| save.data().flag(id))
}

#[test]
fn the_raids_last_raider_runs_out_with_the_case() {
    let mut sim = ambition_app::Platformer2dSimHarness::new_with_options(fixed_60hz_room_options("intro_wake_room"))
        .expect("the workshop boots");
    sim.step_n(base(), 30);
    let refusals = sim.world().resource::<ambition_platformer2d::runtime::cutscene::CutsceneLineRefusals>().0.clone();
    assert!(refusals.is_empty(), "every shipped cutscene line prepares: {refusals:?}");
    let mut pad = Pad::new(&mut sim);
    through_the_raid(&mut pad, RaidStart::Door).unwrap_or_else(|why| panic!("the raid: {why}"));
    // The raid is won the tick he goes; the scene sees him gone on its next
    // tick, and the Creator's last words follow.
    for frame in 0..1200 {
        if flag(pad.world(), "intro_case_run_seen") {
            break;
        }
        pad.step(Press { confirm: frame % 10 == 0, ..Press::default() });
    }
    assert!(flag(sim.world(), "intro_case_run_seen"), "the escape played when two raiders had fallen");
    assert!(flag(sim.world(), "intro_case_taken"), "the yard gate's flag is set");
    let (live, fallen) = raiders(sim.world_mut());
    assert_eq!(live, 0, "the last raider is gone");
    assert_eq!(fallen, 2, "two raiders fell, and the third ran rather than fell");
}
