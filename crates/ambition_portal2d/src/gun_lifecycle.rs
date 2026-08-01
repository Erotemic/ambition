//! Compatibility lifecycle for gun-owned portals.
//!
//! This module keeps Ambition's current "portals opened by a gun disappear when
//! the gun is gone" policy away from the reusable portal topology/transit core.
//! Static authored portals, scripted emitters, and moving portals should not be
//! forced through this ownership model.

use bevy::prelude::*;

use super::gun::PortalGun;
use super::gun_pickup::PortalGunPickup;
use super::gun_projectile::PortalShot;
use super::types::PlacedPortal;

/// The gun's portals must not outlive the gun that made them: despawn the
/// gun-pair portals plus in-flight shots when **no** portal gun is present in
/// the room — neither held ([`PortalGun`]) nor lying as a [`PortalGunPickup`].
///
/// FIXME(portal-api): this should become a host-installed policy plugin or a
/// generic "emitter owns portal set" cleanup rule. It is not part of the pure
/// portal chart-transition model.
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
