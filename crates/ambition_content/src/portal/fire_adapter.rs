//! Ambition fire-intent resolver: gesture → generic portal fire intent.
//!
//! The input adapter recognizes the *gesture* and emits a [`FirePortalGun`]
//! (implying "the primary player, holding the gun, aiming this way"). Portal core
//! no longer understands that — it consumes a generic
//! [`PortalFireIntent`] `{ origin, dir, channel }`. This resolver bridges the
//! two: it reads `FirePortalGun`, resolves the origin (the primary player's body
//! position), the direction (the gesture's aim), and the channel (the held gun's
//! current color), and emits the generic intent — behavior identical to the old
//! in-core `portal_fire_system`, but now anything (a replay, an AI) can place a
//! portal by emitting `PortalFireIntent` directly.

use bevy::prelude::*;

use ambition_sandbox::player::{BodyKinematics, PlayerEntity, PrimaryPlayer};
use ambition_sandbox::portal::{FirePortalGun, PortalFireIntent, PortalGun};

/// Resolve a [`FirePortalGun`] gesture into a generic [`PortalFireIntent`] for
/// the primary player: origin = the player's body position, dir = the gesture's
/// resolved aim, channel = the held gun's `next_color`. Gun-active gating lives
/// here (it was the old `if !gun.active { return; }` in the core fire system), so
/// the generic intent is only emitted for a genuine, armed fire. A zero aim is
/// dropped by the core fire system.
pub fn resolve_portal_fire_intent(
    mut fires: MessageReader<FirePortalGun>,
    players: Query<(&BodyKinematics, &PortalGun), (With<PlayerEntity>, With<PrimaryPlayer>)>,
    mut intents: MessageWriter<PortalFireIntent>,
) {
    let Some(fire) = fires.read().last().copied() else {
        return;
    };
    let Ok((kin, gun)) = players.single() else {
        return;
    };
    if !gun.active {
        return;
    }
    intents.write(PortalFireIntent {
        origin: kin.pos,
        dir: fire.aim,
        channel: gun.next_color.channel(),
    });
}
