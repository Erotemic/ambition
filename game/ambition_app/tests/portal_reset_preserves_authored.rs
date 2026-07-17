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
//! tested in `ambition::portal::lifecycle` and the host bridge, since a gun pair is
//! auto-despawned here when the player holds no portal gun).

use crate::common::{authored_portal_pairs, base, fixed_60hz_room_sim};

use ambition::actors::features::{ResetRoomFeaturesEvent, RoomResetReason};
use ambition::portal::PlacedPortal;
use ambition_app::SandboxSim;

fn authored_count(sim: &mut SandboxSim) -> usize {
    let mut q = sim.world_mut().query::<&PlacedPortal>();
    let world = sim.world();
    q.iter(world).filter(|p| !p.channel.is_gun_pair()).count()
}

#[test]
fn authored_portals_survive_both_death_and_manual_resets() {
    let mut sim = fixed_60hz_room_sim("portal_lab");
    sim.step(base());

    let baseline = authored_count(&mut sim);
    assert!(
        baseline > 0,
        "portal_lab should have authored level portals, got {baseline}"
    );
    assert!(
        !authored_portal_pairs(&mut sim).is_empty(),
        "portal_lab should have at least one linked authored portal pair"
    );

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
