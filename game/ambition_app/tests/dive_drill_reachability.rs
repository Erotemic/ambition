//! Room-level verification that `dive_drill` lets the Dive ability clear a line of
//! targets. The dive is a clean ~140px position lunge (confirmed: x 404->544
//! instantly), and its damage is the whole dash corridor — so lunging through a
//! row of three targets should clear them. Movement is checked via the public
//! observation; the kills via a world query on the enemies' `BodyHealth`.

use crate::common::{base, fixed_60hz_room_sim};

use ambition_app::{AgentAction, Platformer2dSimHarness};

/// Current HP of each target (enemies carry `BodyHealth`; the player carries
/// player-side health, so this is the target line). Dead-but-not-despawned
/// targets show `current <= 0`, so HP distinguishes "killed" from "survived".
/// ⛔ **this counted the PLAYER too, and that made the test unsatisfiable.**
/// It queried every `BodyHealth` in the world, so its readout was
/// `[player, target]` — and the assertion below demands `after_alive == 0`,
/// i.e. NO living body, while the assertion above it demands `resets == 0`,
/// i.e. the player survived the crossing. Both cannot hold. The name said
/// `enemy_hps` and the query said "everything with health".
fn enemy_hps(sim: &mut Platformer2dSimHarness) -> Vec<i32> {
    let mut q = sim.world_mut().query_filtered::<
        &ambition_platformer2d::characters::actor::BodyHealth,
        bevy::prelude::Without<ambition_platformer2d::platformer::markers::PlayerEntity>,
    >();
    q.iter(sim.world()).map(|h| h.health.current).collect()
}

// ⛔ **UN-IGNORED 2026-08-05, and the reason it carried for 39 days was wrong.**
//
// The ignore said: *"the dive's downward strike now clips a 4-HP target by one
// point instead of killing it … only the exact kill outcome moved."* Measured by
// running it with `--nocapture`, which is all it ever needed: `target HP
// [20, 4] -> [20, 0]`. The 4-HP target **died**. Nothing about the kill outcome
// had moved.
//
// What was actually broken was this file. `enemy_hps` queried every `BodyHealth`
// in the world including the PLAYER's, so `after_alive == 0` demanded that the
// player be dead as well — while the assertion two lines above demands
// `resets == 0`, that the player crossed cleanly. The test could not pass, and
// the diagnosis was written in the same commit as the refactor it blamed
// (`cc8e3c08f1`), which is how a self-contradictory assertion became a
// "re-tune later" for over a month.
//
// ⚠ it was also in NO ledger row, so nothing was tracking it. An `#[ignore]` is
// the one suppression the suite reports as a PASS.
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
