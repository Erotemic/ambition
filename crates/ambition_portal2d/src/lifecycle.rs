//! Portal lifecycle and persistence policy for placed portals and transit
//! cooldowns.

use bevy::prelude::*;
use ambition_platformer2d_core::replay_messages::ReplayMessageReader;

use super::messages::ClearPortals;
use super::types::{PlacedPortal, PortalTransitCooldown};

/// Despawn disposable gun-owned portals on a [`ClearPortals`] signal, and clear
/// any body's transit cooldown. Authored portals are level content and stay.
/// The host emits [`ClearPortals`] on its room reset.
///
/// FIXME(portal-api): when authored portals become movable, reset should snap
/// them back to their authored spec.
///
/// Only the reset's room clears ([`ClearPortals::room`]): a portal or a body
/// is in the room [`LiveRooms::of`](ambition_platformer2d_shared_tangle::lifecycle::LiveRooms::of)
/// gives it, so with one live room every one is in it.
pub fn clear_portals_on_reset(
    mut commands: Commands,
    mut resets: ReplayMessageReader<ClearPortals>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    portals: Query<(Entity, &PlacedPortal)>,
    cooldowns: Query<Entity, With<PortalTransitCooldown>>,
) {
    let resets: Vec<ClearPortals> = resets.read().copied().collect();
    if resets.is_empty() {
        return;
    }
    let reached = |entity: Entity| {
        let room = live.of(entity);
        resets.iter().any(|reset| reset.reaches(room))
    };
    for (entity, portal) in &portals {
        // Spare authored level portals; only gun-owned ephemeral pairs clear.
        if portal.channel.is_gun_pair() && reached(entity) {
            commands.entity(entity).despawn();
        }
    }
    for entity in &cooldowns {
        if reached(entity) {
            commands.entity(entity).remove::<PortalTransitCooldown>();
        }
    }
}

// Generator-owned portal cleanup is in `generator_lifecycle.rs`.

#[cfg(test)]
mod reset_tests {
    //! `clear_portals_on_reset` clears disposable gun-owned pairs but spares
    //! authored level portals — a reset must never wipe level-placed portals.
    use super::*;
    use crate::color::{PortalChannel, PortalChannelColor, PortalGunColor};
    use crate::types::PlacedPortal;
    use bevy::math::Vec2;

    fn portal(channel: PortalChannel) -> PlacedPortal {
        let normal = Vec2::new(1.0, 0.0);
        PlacedPortal::fixed(
            channel,
            Vec2::new(0.0, 0.0),
            normal,
            crate::PORTAL_OPENING_HALF,
        ).unwrap()
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

        app.world_mut().write_message(ClearPortals::EVERYWHERE);
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

    /// P1: a reset is of its room. Two live rooms each hold a gun pair; a
    /// reset of the first clears its pair and the cooldown of its body, and
    /// the second room's pair and body are as they were.
    #[test]
    fn a_reset_of_one_live_room_spares_the_other_rooms_pair() {
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
        let rooms = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
        let mut app = App::new();
        app.add_message::<ClearPortals>();
        app.add_systems(Update, clear_portals_on_reset);
        for room in rooms {
            app.world_mut().spawn((RoomInstanceRoot, room));
        }
        let mut pairs = Vec::new();
        let mut bodies = Vec::new();
        for room in rooms {
            for color in [PortalGunColor::BLUE, PortalGunColor::ORANGE] {
                pairs.push(app.world_mut().spawn((portal(PortalChannel::Gun(color)), InRoomInstance(room))).id());
            }
            bodies.push(app.world_mut().spawn((
                PortalTransitCooldown { remaining: 1.0, pair: PortalChannel::Gun(PortalGunColor::BLUE) },
                InRoomInstance(room),
            )).id());
        }

        app.world_mut().write_message(ClearPortals::of(rooms[0]));
        app.update();

        let open: Vec<bool> = pairs.iter().map(|e| app.world().get_entity(*e).is_ok()).collect();
        assert_eq!(open, [false, false, true, true], "only the reset room's pair clears");
        let cooling: Vec<bool> = bodies.iter().map(|e| app.world().get::<PortalTransitCooldown>(*e).is_some()).collect();
        assert_eq!(cooling, [false, true], "only the reset room's body loses its cooldown");
    }
}
