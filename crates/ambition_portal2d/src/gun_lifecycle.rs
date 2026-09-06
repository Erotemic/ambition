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
/// gun-pair portals plus in-flight shots whose PAIR no longer has a gun in the
/// room — neither held ([`PortalGun`]) nor lying as a [`PortalGunPickup`].
///
/// ⛔ **THIS IS PER-PAIR, AND IT USED TO BE ALL-OR-NOTHING.** The old rule was
/// *"if no gun of any kind exists, clear every gun portal"*, which was exactly
/// right while there was one gun and silently wrong once a gun owns its own
/// pair: taking the only red/yellow gun out of the room left its portals open
/// forever, because a blue/orange gun elsewhere still answered "a gun exists".
/// Portals outliving the gun that made them is the one thing this system is
/// for, so it has to ask about the gun that made THESE.
///
/// FIXME(portal-api): this should become a host-installed policy plugin or a
/// generic "emitter owns portal set" cleanup rule. It is not part of the pure
/// portal chart-transition model.
pub fn despawn_orphaned_portals(
    mut commands: Commands,
    guns: Query<&PortalGun>,
    pickups: Query<&PortalGunPickup>,
    portals: Query<(Entity, &PlacedPortal)>,
    shots: Query<(Entity, &PortalShot)>,
) {
    // Every pair with a gun still in the room, in hand or on the floor.
    let live: std::collections::HashSet<u8> = guns
        .iter()
        .map(|gun| gun.pair())
        .chain(pickups.iter().map(|pickup| pickup.pair))
        .collect();
    let orphaned = |channel: &crate::PortalChannel| match channel {
        crate::PortalChannel::Gun(color) => !live.contains(&color.pair()),
        // Authored pairs are not gun-owned and never orphaned by a gun leaving.
        crate::PortalChannel::Authored(_) => false,
    };
    for (entity, portal) in &portals {
        if orphaned(&portal.channel) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, shot) in &shots {
        if orphaned(&shot.channel) {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{PortalChannel, PortalChannelColor, PortalGunColor};
    use crate::types::portal_half_extent;

    fn floor_portal(channel: PortalChannel, x: f32) -> PlacedPortal {
        PlacedPortal::fixed(
            channel,
            Vec2::new(x, 300.0),
            Vec2::new(0.0, -1.0),
            portal_half_extent(Vec2::new(0.0, -1.0)),
        )
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_systems(Update, despawn_orphaned_portals);
        app
    }

    fn live_channels(app: &mut App) -> Vec<PortalChannel> {
        let mut q = app.world_mut().query::<&PlacedPortal>();
        let mut channels: Vec<PortalChannel> = q.iter(app.world()).map(|p| p.channel).collect();
        channels.sort_by_key(|c| format!("{c:?}"));
        channels
    }

    /// ⛔ THE BUG THE OLD ALL-OR-NOTHING RULE WOULD HAVE SHIPPED once a gun owns
    /// one pair: removing the ONLY red/yellow gun left its portals open forever,
    /// because a blue/orange gun elsewhere still answered "a gun exists".
    #[test]
    fn losing_one_guns_pair_does_not_strand_its_portals_or_touch_another_guns() {
        let mut app = app();
        let mine = PortalChannel::Gun(PortalGunColor::for_pair(3));
        let theirs = PortalChannel::Gun(PortalGunColor::for_pair(0));
        app.world_mut().spawn(floor_portal(mine, 100.0));
        app.world_mut().spawn(floor_portal(theirs, 200.0));
        // Only the pair-0 gun is still in the room.
        app.world_mut().spawn(PortalGun::for_pair(0));

        app.update();

        assert_eq!(
            live_channels(&mut app),
            vec![theirs],
            "pair 3's portals outlived the only gun that could have made them, \
             or pair 0's were taken with them"
        );
    }

    /// A gun lying on the FLOOR still owns its pair — a dropped gun is not a
    /// gone gun, and its portals must survive the trip.
    #[test]
    fn a_pickup_on_the_floor_keeps_its_pairs_portals_alive() {
        let mut app = app();
        let channel = PortalChannel::Gun(PortalGunColor::for_pair(5));
        app.world_mut().spawn(floor_portal(channel, 100.0));
        app.world_mut().spawn(PortalGunPickup {
            pos: Vec2::ZERO,
            half_extent: Vec2::splat(20.0),
            arm_timer: 0.0,
            pair: 5,
        });

        app.update();

        assert_eq!(live_channels(&mut app), vec![channel]);
    }

    /// Authored pairs are not gun-owned and a room with no gun at all keeps them.
    #[test]
    fn an_authored_pair_is_never_orphaned_by_a_missing_gun() {
        let mut app = app();
        let authored = PortalChannel::Authored(PortalChannelColor::Purple);
        let gun = PortalChannel::Gun(PortalGunColor::BLUE);
        app.world_mut().spawn(floor_portal(authored, 100.0));
        app.world_mut().spawn(floor_portal(gun, 200.0));
        // No gun and no pickup anywhere.

        app.update();

        assert_eq!(
            live_channels(&mut app),
            vec![authored],
            "an authored portal was despawned with the gun pairs, or a gun pair \
             survived with no gun in the room"
        );
    }
}
