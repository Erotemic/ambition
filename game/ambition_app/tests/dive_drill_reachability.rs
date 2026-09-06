//! The dive is a gap-closer that HURTS what it passes through.
//!
//! Two claims, and they need different things. That the dive carries a body
//! across a hazard gap is about the ROOM — `dive_drill` has the pickup, the gap
//! and the far ledge, and that is what the room is for. That the dive damages a
//! body it lunges through is about the ENGINE, and it needs a body, not a
//! particular authored one.
//!
//!  more appropriate is right here. A test that spawns the body it measures
//! states its own preconditions; the old one asserted an engine property and
//! depended on a room's furniture to hold it up. `spawn_enemy_character_at`
//! names a real character, so the target is a body the game can actually build
//! rather than a display name resolving art by string.

use crate::common::{base, fixed_60hz_room_sim};

use ambition_app::{AgentAction, Platformer2dSimHarness};

/// Current HP of each target (enemies carry `BodyHealth`; the player carries player-side
/// health, so this is the target line). Dead-but-not-despawned targets show `current <= 0`, so
/// HP distinguishes "killed" from "survived". this counted the PLAYER too, and that made the
/// test unsatisfiable. It queried every `BodyHealth` in the world, so its readout was
/// `[player, target]` — and the assertion below demands `after_alive == 0`, i.e. NO living
/// body, while the assertion above it demands `resets == 0`, i.e. Both cannot hold. The name
/// said `enemy_hps` and the query said "everything with health".
fn enemy_hps(sim: &mut Platformer2dSimHarness) -> Vec<i32> {
    let mut q = sim.world_mut().query_filtered::<
        &ambition_platformer2d::characters::actor::BodyHealth,
        bevy::prelude::Without<ambition_platformer2d::platformer::markers::PlayerEntity>,
    >();
    q.iter(sim.world()).map(|h| h.health.current).collect()
}

// The 4-HP target died. Nothing about the kill outcome had moved.
//
// What was actually broken was this file. `enemy_hps` queried every `BodyHealth` in the world
// including the PLAYER's, so `after_alive == 0` demanded that the player be dead as well —
// while the assertion two lines above demands `resets == 0`, that the player crossed cleanly.
//
// it was also in NO ledger row, so nothing was tracking it. An `#[ignore]` is
// the one suppression the suite reports as a PASS.
#[test]
fn dive_drill_lunges_through_the_targets() {
    let mut sim = fixed_60hz_room_sim("dive_drill");

    sim.spawn_enemy_character_at(
        "dive_drill_target",
        "Dive Target",
        (540.0, 210.0),
        (12.0, 16.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive,
        "npc_puppy_slug",
    );
    for _ in 0..2 {
        sim.step(base());
    }

    // Grab the Dive ability off the floor (pickup x[110,150]).
    //
    // ⛔⛔ A BOUNDED WALK THAT EXHAUSTS LOOKS EXACTLY LIKE ONE THAT ARRIVED, and
    // that is what made this test's failure unreadable. If sixty frames do not
    // carry the player to the pickup, the loop simply ENDS, the `attack` below
    // presses on empty floor, no Dive is acquired, and the failure surfaces forty
    // lines later as "the dive should carry the player across the hazard gap" —
    // an assertion about the dive, in a run where the ability was never held.
    // ⇒ Each walk now states where it got to. See the KNOWN FLAKE row in
    // `engine/performance-and-iteration.md`, which asks what in this test depends
    // on something other than ticks; a bounded loop's EXHAUSTION is such a thing,
    // because the number of frames a walk needs is not fixed by the assertion.
    let mut reached = 0.0_f32;
    for _ in 0..60 {
        reached = sim
            .step(AgentAction {
                move_x: 1.0,
                ..base()
            })
            .player_pos
            .0;
        if reached >= 120.0 {
            break;
        }
    }
    assert!(
        reached >= 120.0,
        "sixty frames did not carry the player to the Dive pickup (x={reached:.0}, \
         wanted >=120). The grab below would press on empty floor and every later \
         assertion would be about a dive the player never had."
    );
    sim.step(AgentAction {
        attack: true,
        ..base()
    }); // grab
    sim.step(base());

    // Walk to the firing spot ~x400 (left of the target line at x440..540).
    let mut reached = 0.0_f32;
    for _ in 0..80 {
        reached = sim
            .step(AgentAction {
                move_x: 1.0,
                ..base()
            })
            .player_pos
            .0;
        if reached >= 400.0 {
            break;
        }
    }
    assert!(
        reached >= 400.0,
        "eighty frames did not carry the player to the firing spot (x={reached:.0}, \
         wanted >=400), so the dive below starts from the wrong place."
    );
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
