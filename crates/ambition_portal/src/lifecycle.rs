//! Portal lifecycle / persistence policy: clear portals on room reset and
//! despawn gun-orphaned portals.
//!
//! The gravity-zone mechanic (room-reset gravity reset + the ambient
//! gravity-flip switch) moved to `crate::gravity` (Stage 6 follow-up):
//! it is a gravity mechanic, not portal behavior.

use bevy::prelude::*;

use super::gun::PortalGun;
use super::messages::ClearPortals;
use super::pickup::PortalGunPickup;
use super::shot::PortalShot;
use super::types::{PlacedPortal, PortalTransitCooldown};

/// Despawn the GUN's portals on a [`ClearPortals`] signal, and clear any body's
/// transit cooldown. AUTHORED portals are level content and are spared — a room
/// reset (death or the manual delete-key) must not wipe the purple/yellow/etc.
/// portals the level placed; only the player's gun-spawned Blue/Orange pair is
/// disposable. (Authored portals can't be repositioned yet, so leaving them in
/// place is the same as "reset to their original position"; when they become
/// movable, the manual reset should additionally snap them back to their authored
/// spec — TODO.) Portal core consumes the portal-owned `ClearPortals` message;
/// the Ambition room-reset adapter emits it from `ResetRoomFeaturesEvent`, so core
/// never names the Ambition reset.
pub fn clear_portals_on_reset(
    mut commands: Commands,
    mut resets: MessageReader<ClearPortals>,
    portals: Query<(Entity, &PlacedPortal)>,
    cooldowns: Query<Entity, With<PortalTransitCooldown>>,
) {
    if resets.read().next().is_none() {
        return;
    }
    for (entity, portal) in &portals {
        // Spare authored level portals; only the gun's ephemeral pair is cleared.
        if portal.channel.is_gun_pair() {
            commands.entity(entity).despawn();
        }
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

#[cfg(test)]
mod reset_tests {
    //! `clear_portals_on_reset` clears the disposable GUN pair but SPARES authored
    //! level portals — a room reset must never wipe the level's placed portals.
    use super::*;
    use crate::color::{PortalChannel, PortalChannelColor, PortalGunColor};
    use crate::types::{portal_half_extent, PlacedPortal};
    use bevy::math::Vec2;

    fn portal(channel: PortalChannel) -> PlacedPortal {
        let normal = Vec2::new(1.0, 0.0);
        PlacedPortal {
            channel,
            pos: Vec2::new(0.0, 0.0),
            normal,
            half_extent: portal_half_extent(normal),
        }
    }

    #[test]
    fn clear_on_reset_spares_authored_and_clears_gun() {
        let mut app = App::new();
        app.add_message::<ClearPortals>();
        app.add_systems(Update, clear_portals_on_reset);

        app.world_mut()
            .spawn(portal(PortalChannel::Gun(PortalGunColor::Blue)));
        app.world_mut()
            .spawn(portal(PortalChannel::Gun(PortalGunColor::Orange)));
        let authored = app
            .world_mut()
            .spawn(portal(PortalChannel::Authored(PortalChannelColor::Purple)))
            .id();

        app.world_mut().write_message(ClearPortals);
        app.update();

        let remaining: Vec<PortalChannel> = {
            let mut q = app.world_mut().query::<&PlacedPortal>();
            q.iter(app.world()).map(|p| p.channel).collect()
        };
        assert_eq!(
            remaining,
            vec![PortalChannel::Authored(PortalChannelColor::Purple)],
            "only the authored portal should survive a clear; gun pair cleared"
        );
        assert!(
            app.world().get_entity(authored).is_ok(),
            "the authored portal entity must still exist"
        );
    }
}
