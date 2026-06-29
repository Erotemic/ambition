//! Portal lifecycle / persistence policy for placed portals and transit cooldowns.
//!
//! The gravity-zone mechanic (room-reset gravity reset + the ambient
//! gravity-flip switch) moved to `crate::gravity` (Stage 6 follow-up):
//! it is a gravity mechanic, not portal behavior.

use bevy::prelude::*;

use super::messages::ClearPortals;
use super::types::{PlacedPortal, PortalTransitCooldown};

/// Despawn disposable gun-owned portals on a [`ClearPortals`] signal, and clear
/// any body's transit cooldown. Authored portals are level content and are
/// spared — a room reset must not wipe the purple/yellow/etc. portals the level
/// placed.
///
/// FIXME(portal-api): when authored portals become movable, reset should snap
/// them back to their authored spec instead of merely sparing their current
/// entity. The host emits this portal-owned message; core never names the
/// host's reset event.
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
        // Spare authored level portals; only gun-owned ephemeral pairs clear.
        if portal.channel.is_gun_pair() {
            commands.entity(entity).despawn();
        }
    }
    for entity in &cooldowns {
        commands.entity(entity).remove::<PortalTransitCooldown>();
    }
}

// Gun-owned portal cleanup lives in `gun_lifecycle.rs` so the core lifecycle
// module remains about portal state and room-reset policy, not the current
// Ambition gun ownership model.


#[cfg(test)]
mod reset_tests {
    //! `clear_portals_on_reset` clears disposable gun-owned pairs but spares
    //! authored level portals — a reset must never wipe level-placed portals.
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
            .spawn(portal(PortalChannel::Gun(PortalGunColor::BLUE)));
        app.world_mut()
            .spawn(portal(PortalChannel::Gun(PortalGunColor::ORANGE)));
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
