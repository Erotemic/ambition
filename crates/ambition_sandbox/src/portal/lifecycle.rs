//! Portal lifecycle / persistence policy: clear portals on room reset and
//! despawn gun-orphaned portals.
//!
//! The gravity-zone mechanic (room-reset gravity reset + the ambient
//! gravity-flip switch) moved to `crate::mechanics::gravity` (Stage 6 follow-up):
//! it is a gravity mechanic, not portal behavior.

use bevy::prelude::*;

use super::gun::PortalGun;
use super::messages::ClearPortals;
use super::pickup::PortalGunPickup;
use super::shot::PortalShot;
use super::types::{PlacedPortal, PortalTransitCooldown};

/// Despawn all portals on a [`ClearPortals`] signal, and clear any body's transit
/// cooldown — portals are per-room, so stale ones from a previous room must not
/// linger and teleport the player unexpectedly. Portal core consumes the
/// portal-owned `ClearPortals` message; the Ambition room-reset adapter
/// (`crate::ambition_content::portal::bridge_room_reset_to_clear_portals`) emits
/// it from `ResetRoomFeaturesEvent`, so core never names the Ambition reset.
pub fn clear_portals_on_reset(
    mut commands: Commands,
    mut resets: MessageReader<ClearPortals>,
    portals: Query<Entity, With<PlacedPortal>>,
    cooldowns: Query<Entity, With<PortalTransitCooldown>>,
) {
    if resets.read().next().is_none() {
        return;
    }
    for entity in &portals {
        commands.entity(entity).despawn();
    }
    for entity in &cooldowns {
        commands.entity(entity).remove::<PortalTransitCooldown>();
    }
}

/// The GUN's portals must not outlive the gun that made them: despawn the
/// gun-pair portals (blue/orange) + in-flight shots when **no** portal gun is
/// present in the room — neither held (`PortalGun`) nor lying as a
/// `PortalGunPickup`. This is the "gun is destroyed" case. Authored pairs (other
/// colors, e.g. a test room's portals) are NOT gun-owned, so they persist even
/// with no gun around. A merely *dropped* gun still exists as a pickup, so its
/// portals persist; leaving the room is handled by [`clear_portals_on_reset`].
pub fn despawn_orphaned_portals(
    mut commands: Commands,
    guns: Query<(), With<PortalGun>>,
    pickups: Query<(), With<PortalGunPickup>>,
    portals: Query<(Entity, &PlacedPortal)>,
    shots: Query<Entity, With<PortalShot>>,
) {
    if !guns.is_empty() || !pickups.is_empty() {
        return;
    }
    for (entity, portal) in &portals {
        if portal.channel.is_gun_pair() {
            commands.entity(entity).despawn();
        }
    }
    for entity in &shots {
        commands.entity(entity).despawn();
    }
}
