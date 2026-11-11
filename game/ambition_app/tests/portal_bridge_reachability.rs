// Portal integration test: only built with the portal mechanic + RL stepping
// API. Compiled out (empty test binary) when `portal` is disabled.
#![cfg(all(feature = "portal", feature = "rl_sim"))]
//! Room-level verification that the `portal_bridge` room actually lets the portal
//! gun flagship cross an otherwise-impassable gap. The portal MECHANIC has 15 unit
//! tests in `portal.rs`; this proves the LDtk content (PortalGunSpawn + walls +
//! death strip) wires up to it: pick up the gun, fire a portal on the far wall and
//! one on the floor, and teleport across the death strip — all driven through the
//! public `Platformer2dSimHarness` action/observation API, asserting only on player position
//! (so it's robust to internal component layout).
//!
//! Sequence: walk onto the gun (Attack to grab) → step clear of the entry door →
//! fire BLUE aimed right (lands on the right wall over the gap) → Interact to
//! toggle color → fire ORANGE aimed down (a floor portal at the feet) → standing
//! on the floor portal teleports the player to the blue one on the far side.

use crate::common::{base, fixed_60hz_room_sim};

use ambition_app::AgentAction;

#[test]
fn portal_bridge_lets_the_player_teleport_across_the_death_gap() {
    let mut sim = fixed_60hz_room_sim("portal_bridge");

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

/// The portal gun is an ordinary held item: from the press that takes it, the
/// held-item view the renderer draws from has it in the hand (it was drawn
/// only once a portal stood), and the body is a portal generator; and
/// `Shield + Attack` puts it down where it is a generator still.
#[test]
#[ignore = "on demand: run with --ignored when the portal gun's item road changes"]
fn the_portal_gun_is_in_the_hand_from_the_press_that_takes_it() {
    use ambition_platformer2d::portal::{PlacedPortal, PortalGenerator};
    let mut sim = fixed_60hz_room_sim("portal_bridge");
    for _ in 0..80 {
        if sim.step(AgentAction { move_x: 1.0, ..base() }).player_pos.0 >= 165.0 {
            break;
        }
    }
    sim.step(AgentAction { attack: true, ..base() });
    sim.step(base());
    let world = sim.world_mut();
    let portals = world.query::<&PlacedPortal>().iter(world).count();
    assert_eq!(portals, 0, "premise: no portal stands, so nothing woke the portal systems");
    let in_hand: Vec<String> = world
        .resource::<ambition_platformer2d::sim_view::HeldItemView>()
        .0
        .iter()
        .map(|held| held.item_id.clone())
        .collect();
    assert_eq!(in_hand, vec!["portal_gun".to_string()], "the gun is not in the drawn hand");
    // The holder is a generator; so is the gun it carries, which keeps its pair.
    let holders = world
        .query_filtered::<(), (bevy::prelude::With<PortalGenerator>, bevy::prelude::Without<ambition_platformer2d::held_items::GroundItem>)>()
        .iter(world)
        .count();
    assert_eq!(holders, 1, "the holder is not a generator");

    sim.step_frame(ambition_platformer2d::engine_core::ControlFrame {
        attack_pressed: true,
        shield_held: true,
        ..Default::default()
    });
    for _ in 0..30 {
        sim.step(base());
    }
    let world = sim.world_mut();
    assert!(world.resource::<ambition_platformer2d::sim_view::HeldItemView>().0.is_empty(), "the gun is still in the hand");
    let lying = world
        .query::<(&ambition_platformer2d::held_items::GroundItem, Option<&PortalGenerator>)>()
        .iter(world)
        .filter(|(item, _)| item.spec.id == "portal_gun")
        .map(|(_, generator)| generator.is_some())
        .collect::<Vec<_>>();
    assert_eq!(lying, vec![true], "the gun lies in the world as a generator");
}
