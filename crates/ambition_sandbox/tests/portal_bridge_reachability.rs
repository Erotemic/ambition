//! Room-level verification that the `portal_bridge` room actually lets the portal
//! gun flagship cross an otherwise-impassable gap. The portal MECHANIC has 15 unit
//! tests in `portal.rs`; this proves the LDtk content (PortalGunSpawn + walls +
//! death strip) wires up to it: pick up the gun, fire a portal on the far wall and
//! one on the floor, and teleport across the death strip — all driven through the
//! public `SandboxSim` action/observation API, asserting only on player position
//! (so it's robust to internal component layout).
//!
//! Sequence: walk onto the gun (Attack to grab) → step clear of the entry door →
//! fire BLUE aimed right (lands on the right wall over the gap) → Interact to
//! toggle color → fire ORANGE aimed down (a floor portal at the feet) → standing
//! on the floor portal teleports the player to the blue one on the far side.

use ambition_sandbox::rl_sim::TimestepMode;
use ambition_sandbox::{AgentAction, SandboxSim, SandboxSimOptions};

/// A fully-neutral action; build real ones with struct update: `AgentAction {
/// move_x: 1.0, ..base() }`. (AgentAction has no Default, but struct-update works
/// from any base value.)
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

#[test]
fn portal_bridge_lets_the_player_teleport_across_the_death_gap() {
    let opts = SandboxSimOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_start_room("portal_bridge");
    let mut sim = SandboxSim::new_with_options(opts).expect("SandboxSim::new");

    // Spawns on the left floor (x≈94), left of the death strip x[500,780].
    assert!(
        sim.observation().player_pos.0 < 200.0,
        "spawns on the left floor, got x={}",
        sim.observation().player_pos.0
    );

    // Phase 1: walk right onto the gun pickup (x[160,200]) and grab it.
    let mut on_pickup = false;
    for _ in 0..80 {
        let obs = sim.step(AgentAction {
            move_x: 1.0,
            ..base()
        });
        if obs.player_pos.0 >= 165.0 {
            on_pickup = true;
            break;
        }
    }
    assert!(on_pickup, "should have reached the gun pickup");
    sim.step(AgentAction {
        attack: true,
        ..base()
    }); // Attack = grab the gun
    sim.step(base());
    sim.step(base());

    // Phase 2: step clear of the entry door (x[16,64]) so Interact toggles the gun
    // color, not the door — settle near x≈250 (well left of the gap at x=500).
    for _ in 0..80 {
        let obs = sim.step(AgentAction {
            move_x: 1.0,
            ..base()
        });
        if obs.player_pos.0 >= 250.0 {
            break;
        }
    }
    sim.step(base());
    sim.step(base());
    let fire_x = sim.observation().player_pos.0;
    assert!(
        (200.0..500.0).contains(&fire_x),
        "firing position is between the door and the gap, got x={fire_x}",
    );

    // Phase 3: fire BLUE aimed right — it flies over the gap and opens on the right
    // wall. Then wait for the ~1900px/s shot to land (right wall ≈1000px away).
    sim.step(AgentAction {
        attack: true,
        aim_x: 1.0,
        ..base()
    });
    for _ in 0..48 {
        sim.step(base());
    }

    // Phase 4: Interact toggles the next color to ORANGE.
    sim.step(AgentAction {
        interact: true,
        ..base()
    });
    sim.step(base());

    // Phase 5: fire ORANGE straight down — opens a floor portal at the feet.
    sim.step(AgentAction {
        attack: true,
        aim_y: 1.0,
        ..base()
    });

    // Phase 6: standing on the fresh floor portal teleports to the blue one on the
    // far wall. Give it a few ticks to open + carry the player across.
    let mut crossed = false;
    for _ in 0..30 {
        let obs = sim.step(base());
        if obs.player_pos.0 > 820.0 {
            crossed = true;
            break;
        }
    }

    let obs = sim.observation();
    assert!(
        crossed,
        "player should have teleported across the death gap to the right side \
         (final x={}, resets={}); the floor strip x[500,780] is lethal, so x>820 is \
         only reachable through the portals",
        obs.player_pos.0, obs.resets,
    );
    assert_eq!(
        obs.resets, 0,
        "crossed via portals without dying in the death gap (resets={})",
        obs.resets,
    );
}
