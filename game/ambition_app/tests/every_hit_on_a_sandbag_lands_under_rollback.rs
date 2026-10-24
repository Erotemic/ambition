//! Every hit on a plain enemy lands under rollback, as it does without it.
//!
//! A GGRS sync test rewinds `check_distance` ticks each tick and plays them
//! again. When the state that gates a hit rewinds with the world, the replay
//! equals the first run: the enemy loses the same hit points at each tick
//! with and without the rewinds. The sandbag of the scroll lab is an enemy
//! that stands to be hit, so it is the plain body of this test. The winded
//! Mockingbird has the same test for a boss
//! (`the_robots_fireball_against_the_mockingbird`).

#![cfg(feature = "rl_sim")]

use crate::common::{base, fixed_60hz_room_options};
use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;

fn sandbag(sim: &mut Platformer2dSimHarness) -> (ae::BodyKinematics, i32) {
    let world = sim.world_mut();
    let mut q = world.query::<(&WornCharacter, &ae::BodyKinematics, &BodyHealth)>();
    q.iter(world)
        .find(|(worn, _, _)| worn.id() == "sandbag")
        .map(|(_, kin, health)| (*kin, health.health.current))
        .expect("the scroll lab has a sandbag")
}

/// The sandbag's hit points, tick by tick, while the player strikes it from
/// `standoff` world units off its left side, under a sync test with
/// `check_distance`.
///
/// ⛔ The player is put beside it through the harness, which makes that
/// world the rollback baseline. In the window only the input moves anything:
/// a world write there is erased by the next rewind.
fn sandbag_hp(check_distance: usize, standoff: f32, input: &[AgentAction]) -> Vec<i32> {
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options("scroll_lab").with_sync_test_rollback_settings(check_distance, 12),
    )
    .expect("the scroll lab builds headlessly");
    for _ in 0..120 {
        sim.step(base());
    }
    let (kin, _) = sandbag(&mut sim);
    sim.teleport_player((kin.pos.x - kin.size.x * 0.5 - standoff, kin.pos.y));
    {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&mut ae::BodyKinematics, PrimaryPlayerOnly>();
        q.single_mut(world).expect("the player").facing = 1.0;
    }
    sim.rebase_rollback_history().expect("the placed player becomes the rollback baseline");
    // Settle on the floor; a body far from every player sleeps, and this one
    // is beside the player now.
    for _ in 0..30 {
        sim.step(base());
    }
    let loads_before = sim.rollback_execution_stats().map_or(0, |stats| stats.lifetime_load_runs);
    let mut hp = vec![sandbag(&mut sim).1];
    for action in input {
        sim.step(*action);
        hp.push(sandbag(&mut sim).1);
    }
    sim.rollback_health().expect("the resimulated ticks agree with the first run");
    let loads = sim.rollback_execution_stats().map_or(0, |stats| stats.lifetime_load_runs) - loads_before;
    if check_distance > 0 {
        assert!(loads > input.len() as u64 / 2, "premise: the window was resimulated ({loads} rewinds)");
    }
    hp
}

fn assert_every_hit_lands_under_rollback(name: &str, standoff: f32, input: Vec<AgentAction>) {
    let unrewound = sandbag_hp(0, standoff, &input);
    let landed = unrewound.windows(2).filter(|pair| pair[1] < pair[0]).count();
    assert!(landed >= 3, "{name}: premise: three or more hits land without rollback; hp {unrewound:?}");
    for distance in [1usize, 4] {
        assert_eq!(
            sandbag_hp(distance, standoff, &input),
            unrewound,
            "{name}: at check distance {distance} the sandbag's hit points differ from the run without rollback"
        );
    }
}

#[test]
fn every_blade_hit_on_a_sandbag_lands_under_rollback() {
    let input = (0..75u32).map(|tick| AgentAction { attack: tick % 15 == 0, ..base() }).collect();
    assert_every_hit_lands_under_rollback("blade, a press each 0.25 s", 10.0, input);
}

#[test]
fn every_fireball_on_a_sandbag_lands_under_rollback() {
    let input = (0..80u32)
        .map(|tick| match tick % 20 {
            0 => AgentAction { projectile: true, projectile_held: true, ..base() },
            1 => AgentAction { projectile_released: true, ..base() },
            _ => base(),
        })
        .collect();
    assert_every_hit_lands_under_rollback("fireball tap each 0.33 s", 60.0, input);
}
