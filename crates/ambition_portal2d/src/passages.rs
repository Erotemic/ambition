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

/// The placed portals of each live room as body integration admitted them
/// this tick: the snapshot `BodyGeometry`'s links were built from. Transit
/// crosses through this snapshot, not through the live portals, so a pair
/// that moves, opens or closes after the bodies stepped cannot give a
/// crossing a pose the step was not admitted against (portal M1b). Derived:
/// the publisher writes it each tick before the bodies step, from the
/// placed portals a rewind restores.
#[derive(Resource, Debug, Default, Clone)]
pub struct AdmittedPortals(pub PortalsByRoom);

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
    backsides: Query<(Entity, &crate::PortalBackside)>,
    live: PortalRooms,
    tuning: Option<Res<PortalTuning>>,
    mut passages: ResMut<BodyGeometry>,
    // The rings last said, by room, so each is said once.
    mut rings_said: Local<Vec<(crate::rooms::PortalRoom, Vec<u32>)>>,
    publication: Publication,
    admitted: Option<Res<AdmittedPortals>>,
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
    passages.reset_apertures_except(
        tuning.convention.map_convention(), tuning.geometry.budget(),
        |room| publication.keeps_room(room), |body| publication.keeps_body(body),
    );
    // The pairs are filed by the room `live.of` gives each portal, which is
    // the sole live room for an unstamped one; a body read with no room is
    // placed by the same rule. A replay sees one room of several: the sole
    // room stays what the live rooms make it.
    if publication.replayed.is_none() {
        passages.set_sole_room(live.sole());
    }
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
    // The kernel moves a body's root through the pair it crosses and goes on
    // in the exit's chart (portal M2); transit publishes what it did. A body
    // on the back of a pair does not cross it.
    passages.set_kernel_reroot(Some(tuning.min_exit_speed));
    for (body, back) in &backsides {
        let room = live.of(body);
        let all = by_room.in_room(room);
        let ends: Vec<_> = [back.channel, back.channel.partner()].into_iter()
            .filter_map(|channel| find_portal(all, channel)).map(|portal| portal.aperture()).collect();
        let keys: Vec<u32> = passages.links_in(room).iter()
            .filter(|link| ends.contains(&link.enter)).map(|link| link.key).collect();
        if !keys.is_empty() { passages.close_for(body, keys); }
    }
    write_straddlers(&mut passages, &by_room, transits.iter().map(|(body, transit)| (body, live.of(body), *transit)).collect(), &publication);
    let mut by_room = by_room;
    if let Some(previous) = admitted.as_deref() {
        by_room.keep_from(&previous.0, |room| publication.keeps_room(room));
    }
    commands.insert_resource(AdmittedPortals(by_room));
}

/// What one run of the publishers covers. A full run covers every live room.
/// A room's replay hides the other rooms while it runs (portal T3): it covers
/// only the replayed room, and keeps the publication of each hidden room and
/// body as the live run left it. A room or body that is gone (retired,
/// despawned) is not hidden, so its entries go.
#[derive(bevy::ecs::system::SystemParam)]
pub struct Publication<'w, 's> {
    replayed: Option<Res<'w, ambition_platformer2d_shared_tangle::lifecycle::RoomReplay>>,
    hidden: Query<'w, 's, (), With<bevy::ecs::entity_disabling::Disabled>>,
}

impl Publication<'_, '_> {
    fn keeps_room(&self, room: crate::rooms::PortalRoom) -> bool {
        self.replayed.as_deref().is_some_and(|replay| room.is_some() && room != Some(replay.room))
    }

    fn keeps_body(&self, body: Entity) -> bool {
        self.replayed.is_some() && self.hidden.contains(body)
    }
}

/// Refresh body occupancy after transit without changing the admitted apertures.
pub fn publish_portal_passages(
    portals: Query<(Entity, &PlacedPortal)>,
    transits: Query<(Entity, &PortalTransit)>,
    live: PortalRooms,
    mut passages: ResMut<BodyGeometry>,
    publication: Publication,
) {
    let by_room = PortalsByRoom::collect(&portals, &live);
    write_straddlers(&mut passages, &by_room, transits.iter().map(|(body, transit)| (body, live.of(body), *transit)).collect(), &publication);
}

fn write_straddlers(
    passages: &mut BodyGeometry,
    by_room: &PortalsByRoom,
    mut straddlers: Vec<(Entity, crate::rooms::PortalRoom, PortalTransit)>,
    publication: &Publication,
) {
    passages.retain_straddlers(|body| publication.keeps_body(body));
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
            crate::portal_fire_system, publish_portal_apertures, pose,
            publish_portal_passages,
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
            crate::PortalInputWarp { map: bevy::math::Mat2::IDENTITY, anchor: Vec2::X },
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
