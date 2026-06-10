//! Room-level verification that `blink_run` actually requires + delivers the Blink
//! ability. Blink is a precise 150px teleport (15+ ... well, 5 unit tests in
//! blink.rs); this proves the LDtk content (GroundItem held_item="blink" + low
//! ceiling + hazard gaps) wires up to it: grab the ability, then blink across two
//! ~120px death gaps in a corridor too low to jump. Driven through the public
//! action/observation API; asserts only on player position.
//!
//! Sequence: walk onto the ability (Attack to grab) → for each gap, walk to its
//! near edge and Attack with aim right (a blink carries the player 150px across).

use ambition_app::rl_sim::TimestepMode;
use ambition_app::{AgentAction, SandboxSim, SandboxSimOptions};

/// A fully-neutral action; build real ones with struct update.
fn base() -> AgentAction {
    AgentAction {
        move_x: 0.0,
        move_y: 0.0,
        up_pressed: false,
        down_pressed: false,
        jump: false,
        jump_held: false,
        jump_released: false,
        dash: false,
        attack: false,
        blink: false,
        blink_held: false,
        blink_released: false,
        pogo: false,
        interact: false,
        projectile: false,
        projectile_held: false,
        projectile_released: false,
        fly_toggle: false,
        reset: false,
        start: false,
        aim_x: 0.0,
        aim_y: 0.0,
    }
}

/// Walk right until at least `edge`, then fire a blink aimed right and let it land
/// (the idle tail also covers the blink cooldown before the next gap).
fn walk_to_then_blink(sim: &mut SandboxSim, edge: f32) {
    for _ in 0..80 {
        if sim
            .step(AgentAction {
                move_x: 1.0,
                ..base()
            })
            .player_pos
            .0
            >= edge
        {
            break;
        }
    }
    sim.step(AgentAction {
        attack: true,
        aim_x: 1.0,
        ..base()
    }); // blink right
    for _ in 0..30 {
        sim.step(base());
    }
}

#[test]
fn blink_run_blinks_across_the_death_gaps() {
    let mut sim = SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("blink_run"),
    )
    .expect("SandboxSim::new");
    assert!(
        sim.observation().player_pos.0 < 110.0,
        "spawns at the start of the corridor, got x={}",
        sim.observation().player_pos.0
    );

    // Grab the Blink ability off the start floor (x[120,160]).
    let mut grabbed = false;
    for _ in 0..60 {
        if sim
            .step(AgentAction {
                move_x: 1.0,
                ..base()
            })
            .player_pos
            .0
            >= 115.0
        {
            grabbed = true;
            break;
        }
    }
    assert!(grabbed, "should reach the blink pickup");
    sim.step(AgentAction {
        attack: true,
        ..base()
    }); // Attack while overlapping = grab
    sim.step(base());

    // Gap 1: near edge ≈ x300; blink should land on the mid floor x[420,620].
    walk_to_then_blink(&mut sim, 275.0);
    let after1 = sim.observation();
    assert!(
        after1.player_pos.0 > 360.0 && after1.resets == 0,
        "should have blinked across gap 1 onto the mid floor (x={}, resets={}); a \
         jump can't clear it under the low ceiling and walking falls onto the hazard",
        after1.player_pos.0,
        after1.resets,
    );

    // Gap 2: near edge ≈ x620; blink should land on the end floor x[740,1008].
    walk_to_then_blink(&mut sim, 595.0);
    let after2 = sim.observation();
    assert!(
        after2.player_pos.0 > 740.0 && after2.resets == 0,
        "should have blinked across gap 2 to the end floor (x={}, resets={})",
        after2.player_pos.0,
        after2.resets,
    );
}
