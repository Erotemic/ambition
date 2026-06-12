// Portal integration test: only built with the portal mechanic + RL stepping
// API. Compiled out (empty test binary) when `portal` is disabled.
#![cfg(all(feature = "portal", feature = "rl_sim"))]
//! Room-level verification that the LDtk-authored `portal_lab` static portals are
//! actually live and usable — i.e. a player who has NOT picked up the portal gun
//! can walk onto an authored portal and be carried through it. (Jon hit "I still
//! couldn't enter them"; the gun-less transit path + this test guard it.)
//!
//! Station A is a purple↔yellow ground↔ground pair at x≈300 / x≈600 on the floor.
//! Walking the player right across the purple portal must produce a TELEPORT —
//! a single-frame position jump far larger than walking speed — which only the
//! portal transfer can do. Driven through the public SandboxSim API, asserting
//! only on observed player position.

mod common;
use common::{base, fixed_60hz_room_sim};

use ambition_app::AgentAction;

#[test]
fn portal_lab_authored_portals_are_enterable_without_the_gun() {
    let mut sim = fixed_60hz_room_sim("portal_lab");

    // Spawns on the left floor (PlayerStart x≈92), left of station A (x≈300).
    let spawn = sim.observation().player_pos;
    assert!(spawn.0 < 200.0, "spawns on the left, got x={}", spawn.0);

    // Walk right across the purple floor portal. Track the biggest single-frame
    // position jump: a normal walk step is only a few px at 60 Hz, so any jump of
    // >150px is a portal transfer (purple x≈300 → yellow x≈600 is ~300px).
    let mut prev = spawn;
    let mut max_jump = 0.0_f32;
    let mut resets = sim.observation().resets;
    for _ in 0..240 {
        let obs = sim.step(AgentAction {
            move_x: 1.0,
            ..base()
        });
        let cur = obs.player_pos;
        let jump = ((cur.0 - prev.0).powi(2) + (cur.1 - prev.1).powi(2)).sqrt();
        max_jump = max_jump.max(jump);
        prev = cur;
        resets = obs.resets;
        if max_jump > 150.0 {
            break;
        }
    }

    assert!(
        max_jump > 150.0,
        "walking onto the authored purple portal should teleport the player \
         (biggest single-frame move was {max_jump:.1}px) — the static portals are \
         not transiting; is the gun-less transit / carve path wired?"
    );
    assert_eq!(
        resets, 0,
        "transited the portal without dying (resets={resets})"
    );
}
