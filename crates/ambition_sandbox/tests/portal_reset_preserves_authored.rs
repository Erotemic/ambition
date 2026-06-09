// Portal integration test: only built with the portal mechanic + RL stepping
// API. Compiled out (empty test binary) when `portal` is disabled.
#![cfg(all(feature = "portal", feature = "rl_sim"))]
//! Room-reset portal policy (Jon, 2026-06-09):
//!
//! - A player DEATH preserves EVERY portal — the player's gun-spawned pair AND
//!   the authored level portals survive, so dying mid-puzzle doesn't wipe your
//!   portal setup.
//! - A MANUAL reset (delete-key / scripted replay) clears the disposable gun pair
//!   but still spares the authored level portals (they are room content; with
//!   movable authored portals a manual reset will additionally snap them back to
//!   their authored positions — TODO).
//!
//! Driven through the real app: `portal_lab`'s authored portals must survive BOTH
//! a player-death reset and a manual reset (the gun-pair clear-on-manual is unit-
//! tested in `ambition_portal::lifecycle` and the host bridge, since a gun pair is
//! auto-despawned here when the player holds no portal gun).

use ambition_sandbox::features::{ResetRoomFeaturesEvent, RoomResetReason};
use ambition_sandbox::portal::{PlacedPortal, PortalChannel, PortalChannelColor};
use ambition_sandbox::rl_sim::TimestepMode;
use ambition_sandbox::{AgentAction, SandboxSim, SandboxSimOptions};

const PURPLE: PortalChannel = PortalChannel::Authored(PortalChannelColor::Purple);

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

fn authored_count(sim: &mut SandboxSim) -> usize {
    let mut q = sim.world_mut().query::<&PlacedPortal>();
    let world = sim.world();
    q.iter(world).filter(|p| !p.channel.is_gun_pair()).count()
}

#[test]
fn authored_portals_survive_both_death_and_manual_resets() {
    let opts = SandboxSimOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_start_room("portal_lab");
    let mut sim = SandboxSim::new_with_options(opts).expect("SandboxSim::new in portal_lab");
    sim.step(base());

    let baseline = authored_count(&mut sim);
    assert!(
        baseline > 0,
        "portal_lab should have authored level portals (Purple/Yellow/…), got {baseline}"
    );
    {
        let mut q = sim.world_mut().query::<&PlacedPortal>();
        let world = sim.world();
        assert!(
            q.iter(world).any(|p| p.channel == PURPLE),
            "portal_lab should have an authored Purple portal"
        );
    }

    // A DEATH reset must not despawn any authored portal.
    sim.world_mut().write_message(ResetRoomFeaturesEvent {
        reason: RoomResetReason::PlayerDeath,
    });
    sim.step(base());
    assert_eq!(
        authored_count(&mut sim),
        baseline,
        "a DEATH reset must preserve every authored level portal"
    );

    // A MANUAL reset also spares authored portals (it only clears the gun pair).
    sim.world_mut().write_message(ResetRoomFeaturesEvent {
        reason: RoomResetReason::Manual,
    });
    sim.step(base());
    assert_eq!(
        authored_count(&mut sim),
        baseline,
        "a MANUAL reset must SPARE authored level portals (it clears only the gun pair)"
    );
}
