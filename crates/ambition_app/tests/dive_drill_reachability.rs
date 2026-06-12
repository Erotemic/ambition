//! Room-level verification that `dive_drill` lets the Dive ability clear a line of
//! targets. The dive is a clean ~140px position lunge (confirmed: x 404->544
//! instantly), and its damage is the whole dash corridor — so lunging through a
//! row of three targets should clear them. Movement is checked via the public
//! observation; the kills via a world query on the enemies' `ActorHealth`.

mod common;
use common::{base, fixed_60hz_room_sim};

use ambition_app::{AgentAction, SandboxSim};

/// Current HP of each target (enemies carry `ActorHealth`; the player carries
/// player-side health, so this is the target line). Dead-but-not-despawned
/// targets show `current <= 0`, so HP distinguishes "killed" from "survived".
fn enemy_hps(sim: &mut SandboxSim) -> Vec<i32> {
    let mut q = sim
        .world_mut()
        .query::<&ambition_sandbox::features::components::ActorHealth>();
    q.iter(sim.world()).map(|h| h.health.current).collect()
}

#[test]
fn dive_drill_lunges_through_the_targets() {
    let mut sim = fixed_60hz_room_sim("dive_drill");

    // Grab the Dive ability off the floor (pickup x[110,150]).
    for _ in 0..60 {
        if sim
            .step(AgentAction {
                move_x: 1.0,
                ..base()
            })
            .player_pos
            .0
            >= 120.0
        {
            break;
        }
    }
    sim.step(AgentAction {
        attack: true,
        ..base()
    }); // grab
    sim.step(base());

    // Walk to the firing spot ~x400 (left of the target line at x440..540).
    for _ in 0..80 {
        if sim
            .step(AgentAction {
                move_x: 1.0,
                ..base()
            })
            .player_pos
            .0
            >= 400.0
        {
            break;
        }
    }
    sim.step(base());

    let before_x = sim.observation().player_pos.0;
    let before_hps = enemy_hps(&mut sim);
    let before_alive = before_hps.iter().filter(|&&hp| hp > 0).count();
    assert!(
        before_alive >= 1,
        "the target should be alive across the gap before the dive (HP {before_hps:?})"
    );

    // Dive right: the lunge crosses the hazard gap and strikes the target at the
    // landing (the dive is an offensive gap-closer).
    sim.step(AgentAction {
        attack: true,
        aim_x: 1.0,
        ..base()
    });
    for _ in 0..20 {
        sim.step(base());
    }

    let obs = sim.observation();
    let after_x = obs.player_pos.0;
    let after_hps = enemy_hps(&mut sim);
    let after_alive = after_hps.iter().filter(|&&hp| hp > 0).count();
    eprintln!(
        "dive: x {before_x:.0}->{after_x:.0} ({:+.0}px), target HP {before_hps:?} -> {after_hps:?}, resets={}",
        after_x - before_x, obs.resets
    );
    assert!(
        after_x > 525.0,
        "the dive should carry the player across the hazard gap onto the far ledge (x={after_x:.0})"
    );
    assert_eq!(
        obs.resets, 0,
        "the dive crosses the hazard cleanly without dying (resets={})",
        obs.resets
    );
    assert_eq!(
        after_alive, 0,
        "the dive should strike down the target at the landing (HP {before_hps:?} -> {after_hps:?})"
    );
}
