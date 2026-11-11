//! Portals a generator opened do not outlive it.
//!
//! Keeps the "a generator's portals close when it is gone" policy out of the
//! portal core. Authored, scripted, and moving portals do not use it.

use bevy::prelude::*;

use super::generator::PortalGenerator;
use super::gun_projectile::PortalShot;
use super::types::PlacedPortal;

/// Run condition of [`despawn_orphaned_portals`]: there is a portal or a
/// shot that a generator's leaving could orphan.
pub fn a_portal_or_shot_exists(
    portals: Query<(), With<PlacedPortal>>,
    shots: Query<(), With<PortalShot>>,
) -> bool {
    !portals.is_empty() || !shots.is_empty()
}

/// A generator's portals must not outlive it: despawn the generator-pair
/// portals plus in-flight shots whose PAIR no longer has a
/// [`PortalGenerator`]. What a generator is (a gun in a hand, a gun lying on
/// the floor, a turret) is the host's; any one of them keeps its pair open.
///
/// Checked per pair: a generator of a different pair does not keep these
/// portals open.
pub fn despawn_orphaned_portals(
    mut commands: Commands,
    generators: Query<&PortalGenerator>,
    portals: Query<(Entity, &PlacedPortal)>,
    shots: Query<(Entity, &PortalShot)>,
) {
    // Every pair with a generator still in the world.
    let live: std::collections::HashSet<u8> = generators.iter().map(|generator| generator.pair()).collect();
    let orphaned = |channel: &crate::PortalChannel| match channel {
        crate::PortalChannel::Gun(color) => !live.contains(&color.pair()),
        // Authored pairs are not generator-owned and never orphaned.
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

    fn floor_portal(channel: PortalChannel, x: f32) -> PlacedPortal {
        PlacedPortal::fixed(
            channel,
            Vec2::new(x, 300.0),
            Vec2::new(0.0, -1.0),
            crate::PORTAL_OPENING_HALF,
        ).unwrap()
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

    /// Removing the only generator of one pair clears that pair's portals, even
    /// if a generator of another pair remains.
    #[test]
    fn losing_one_generators_pair_does_not_strand_its_portals_or_touch_anothers() {
        let mut app = app();
        let mine = PortalChannel::Gun(PortalGunColor::for_pair(3));
        let theirs = PortalChannel::Gun(PortalGunColor::for_pair(0));
        app.world_mut().spawn(floor_portal(mine, 100.0));
        app.world_mut().spawn(floor_portal(theirs, 200.0));
        // Only the pair-0 generator is still in the world.
        app.world_mut().spawn(PortalGenerator::for_pair(0));

        app.update();

        assert_eq!(
            live_channels(&mut app),
            vec![theirs],
            "pair 3's portals outlived the only generator that could have made them, \
             or pair 0's were taken with them"
        );
    }

    /// Authored pairs are not generator-owned, and a world with no generator
    /// at all keeps them.
    #[test]
    fn an_authored_pair_is_never_orphaned_by_a_missing_generator() {
        let mut app = app();
        let authored = PortalChannel::Authored(PortalChannelColor::Purple);
        let gun = PortalChannel::Gun(PortalGunColor::BLUE);
        app.world_mut().spawn(floor_portal(authored, 100.0));
        app.world_mut().spawn(floor_portal(gun, 200.0));
        // No generator anywhere.

        app.update();

        assert_eq!(
            live_channels(&mut app),
            vec![authored],
            "an authored portal was despawned with the generator pairs, or a \
             generator pair survived with no generator"
        );
    }
}
