//! Publish this tick's pairs and straddlers in core geometry
//! ([`BodyGeometry`]), for a domain that is not the portal's (combat) to
//! tell where a volume is.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use crate::rooms::PortalRooms;
use ambition_platformer2d_shared_tangle::body::geometry::{BodyGeometry, MaterialBudget};

use crate::rooms::PortalsByRoom;
use crate::transit::PortalTransit;
use crate::tuning::PortalTuning;
use crate::types::{find_portal, PlacedPortal};

/// Each linked pair of `portals` (one room's, in their stable order) once,
/// from the end whose channel orders first: the pairs every reader of
/// [`BodyGeometry`] and the drawing of pieces cut by.
pub fn linked_pairs(portals: &[PlacedPortal]) -> Vec<(ae::frame::PortalAperture, ae::frame::PortalAperture)> {
    linked_pairs_iter(portals).map(|(a,b)|(a.aperture(),b.aperture())).collect()
}

fn linked_pairs_iter(portals:&[PlacedPortal]) -> impl Iterator<Item=(&PlacedPortal,PlacedPortal)> {
    portals
        .iter()
        .filter(|p| p.channel <= p.channel.partner())
        .filter_map(move |p| find_portal(portals, p.channel.partner()).map(|q| (p,q)))
}

/// Publish current apertures and occupancy before body integration.
pub fn publish_portal_apertures(
    mut commands: Commands,
    portal_state: Query<Entity, Or<(
        With<PortalTransit>, With<crate::PortalBackside>, With<crate::PortalTransitCooldown>,
        With<crate::PortalEmission>, With<crate::PortalInputWarp>,
    )>>,
    portals: Query<(Entity, &PlacedPortal)>,
    transits: Query<(Entity, &PortalTransit)>,
    own_policies: Query<(Entity, &MaterialBudget)>,
    live: PortalRooms,
    tuning: Option<Res<PortalTuning>>,
    mut passages: ResMut<BodyGeometry>,
    // The rings last said, by room, so each is said once.
    mut rings_said: Local<Vec<(crate::rooms::PortalRoom, Vec<u32>)>>,
) {
    // This publisher runs when the transit set sleeps. A disabled room must
    // release its old portal state before body integration and input guards.
    for body in &portal_state {
        if !live.allows(body) {
            commands.entity(body).remove::<(
                PortalTransit, crate::PortalBackside, crate::PortalTransitCooldown,
                crate::PortalEmission, crate::PortalInputWarp,
            )>();
        }
    }
    let tuning = tuning.as_deref().copied().unwrap_or_default();
    passages.reset_apertures(tuning.convention.map_convention(), tuning.geometry.budget());
    for (body, own) in &own_policies {
        passages.set_body_budget(body, own.0);
    }
    let by_room = PortalsByRoom::collect(&portals, &live);
    let mut rings: Vec<(crate::rooms::PortalRoom, Vec<u32>)> = Vec::new();
    for (room, all) in by_room.rooms() {
        for (a,b) in linked_pairs_iter(all) {
            passages.add_pair_with_steps(room,[(a.aperture(),a.step),(b.aperture(),b.step)]);
        }
        // A ring of pairs (a pair facing itself, two doors in line) cuts a
        // volume in it again and again until the budget stops it: said once,
        // where pieces are composed.
        if tuning.geometry.budget().max_hops > 1 {
            let ring = ae::frame::thread::recursive_links(passages.links_in(room));
            if !ring.is_empty() {
                if !rings_said.iter().any(|(r, said)| *r == room && *said == ring) {
                    bevy::log::info!(
                        target: "ambition_portal::rings",
                        "the portals of room {room:?} make a ring (links {ring:?}): a volume in it is cut until {:?} stops it",
                        tuning.geometry
                    );
                }
                rings.push((room, ring));
            }
        }
    }
    *rings_said = rings;
    write_straddlers(&mut passages, &by_room, transits.iter().map(|(body, transit)| (body, live.of(body), *transit)).collect());
}

/// Refresh body occupancy after transit without changing the admitted apertures.
pub fn publish_portal_passages(
    portals: Query<(Entity, &PlacedPortal)>,
    transits: Query<(Entity, &PortalTransit)>,
    live: PortalRooms,
    mut passages: ResMut<BodyGeometry>,
) {
    let by_room = PortalsByRoom::collect(&portals, &live);
    write_straddlers(&mut passages, &by_room, transits.iter().map(|(body, transit)| (body, live.of(body), *transit)).collect());
}

fn write_straddlers(
    passages: &mut BodyGeometry,
    by_room: &PortalsByRoom,
    mut straddlers: Vec<(Entity, crate::rooms::PortalRoom, PortalTransit)>,
) {
    passages.clear_straddlers();
    straddlers.sort_by_key(|(body, _, _)| *body);
    for (body, room, transit) in straddlers {
        let all = by_room.in_room(room);
        if let (Some(front), Some(back)) = (
            find_portal(all, transit.straddling), find_portal(all, transit.straddling.partner()),
        ) { passages.add_straddler(body, room, front.aperture(), back.aperture()); }
    }
}


#[cfg(test)]
mod publication_measurements_tests;

#[cfg(test)]
mod capability_isolation_tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::body::geometry::{publish_body_geometry_before_motion, publish_body_geometry_after_motion};
    use ambition_platformer2d_shared_tangle::body::pose::BodyPoseAdmission;
    use ambition_platformer2d_shared_tangle::capability::{Capability, UsedCapabilities};
    use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
    use ambition_platformer2d_world::governing_rules::test_fixtures::two_game_session;
    use ambition_platformer2d_world::scoped_rules::{DeclaredRules, RulesScope};

    fn pose(poses: BodyPoseAdmission, mut bodies: Query<(Entity, &mut ae::BodyKinematics, &mut ae::SweepSample)>) {
        for (entity, mut body, mut sweep) in &mut bodies {
            let desired = ae::BodyKinematics { pos: Vec2::new(2.0, 0.0), ..*body };
            let _ = poses.constrain(entity, &mut body, Some(&mut sweep), desired);
        }
    }

    #[test]
    fn publication_scopes_restrictions_and_clears_a_disabled_rooms_links() {
        let mut app = App::new();
        let first = two_game_session(&mut app, true);
        let second = first.next();
        let mut rules = DeclaredRules::<UsedCapabilities>::default();
        rules.declare(RulesScope::UntaggedRooms, UsedCapabilities::ALL);
        rules.declare(RulesScope::Mode("smash"), UsedCapabilities::ALL.without(Capability::Portals));
        app.insert_resource(rules).init_resource::<BodyGeometry>();
        app.add_message::<crate::PortalFireIntent>().add_message::<crate::PortalShotFired>();
        app.add_systems(Update, (
            crate::portal_fire_system, publish_body_geometry_before_motion, publish_portal_apertures, pose,
            publish_body_geometry_after_motion, publish_portal_passages,
        ).chain());
        let mut owners = Vec::new();
        for room in [first, second] {
            app.world_mut().write_message(crate::PortalFireIntent {
                origin: Vec2::ZERO, dir: Vec2::X,
                channel: crate::PortalChannel::Gun(crate::PortalGunColor::BLUE),
                id: None, room: Some(room),
            });
            for (channel, pos, normal) in [
                (crate::PortalChannel::Gun(crate::PortalGunColor::BLUE), Vec2::ZERO, -Vec2::X),
                (crate::PortalChannel::Gun(crate::PortalGunColor::ORANGE), Vec2::new(500.0,0.0), Vec2::X),
            ] {
                app.world_mut().spawn((PlacedPortal::fixed(channel, pos, normal, 80.0).unwrap(), InRoomInstance(room)));
            }
            let body = ae::BodyKinematics { pos: Vec2::new(-30.0,0.0), canonical_size: Vec2::splat(16.0), ..Default::default() };
            owners.push(app.world_mut().spawn((body, ae::SweepSample::at_rest(body, Vec2::Y), InRoomInstance(room),
                MaterialBudget(ae::frame::thread::PieceBudget { max_hops: 0, ..ae::frame::thread::PieceBudget::COMPOSED }))).id());
        }
        app.update();
        let shots: Vec<_> = app.world_mut().query_filtered::<&InRoomInstance, With<crate::PortalShot>>().iter(app.world()).map(|room| room.0).collect();
        assert_eq!(shots, vec![first]);
        assert_eq!(app.world().get::<ae::BodyKinematics>(owners[0]).unwrap().pos, Vec2::new(-30.0,0.0));
        assert_eq!(app.world().get::<ae::BodyKinematics>(owners[1]).unwrap().pos, Vec2::new(2.0,0.0));
        let geometry = app.world().resource::<BodyGeometry>();
        assert_eq!(geometry.links_in(Some(first)).len(), 2);
        assert!(geometry.links_in(Some(second)).is_empty());
        let channel = crate::PortalChannel::Gun(crate::PortalGunColor::BLUE);
        app.world_mut().entity_mut(owners[0]).insert((
            PortalTransit { straddling: channel, crossed: true },
            crate::PortalBackside { channel },
            crate::PortalTransitCooldown { pair: channel, remaining: 1.0 },
            crate::PortalEmission { exit_normal: Vec2::X, timer: 1.0 },
            crate::PortalInputWarp { n_in: Vec2::X, n_out: -Vec2::X, anchor: Vec2::X },
        ));
        let mut rules = DeclaredRules::<UsedCapabilities>::default();
        rules.declare(RulesScope::EveryRoom, UsedCapabilities::ALL.without(Capability::Portals));
        app.insert_resource(rules);
        app.update();
        let geometry = app.world().resource::<BodyGeometry>();
        assert!(geometry.links_in(Some(first)).is_empty());
        assert!(geometry.links_in(Some(second)).is_empty());
        assert_eq!(app.world().get::<ae::BodyKinematics>(owners[0]).unwrap().pos, Vec2::new(2.0,0.0));
        let body = app.world().entity(owners[0]);
        assert!(!body.contains::<PortalTransit>());
        assert!(!body.contains::<crate::PortalBackside>());
        assert!(!body.contains::<crate::PortalTransitCooldown>());
        assert!(!body.contains::<crate::PortalEmission>());
        assert!(!body.contains::<crate::PortalInputWarp>());
    }
}
