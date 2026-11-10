//! Publication cost at the two body geometry epochs.

use super::*;
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
use bevy::ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Apertures;
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Occupancy;

#[test]
#[ignore = "publication cost measurement"]
fn measure_body_geometry_publication() {
    for count in [0, 100, 1000] {
        for pairs in [0, 8] {
            let mut app = App::new();
            app.init_resource::<BodyGeometry>();
            app.init_schedule(Apertures).init_schedule(Occupancy);
            app.add_systems(Apertures, (
                ambition_platformer2d_shared_tangle::body::geometry::publish_body_geometry_before_motion,
                publish_portal_apertures,
            ).chain());
            app.add_systems(Occupancy, (
                ambition_platformer2d_shared_tangle::body::geometry::publish_body_geometry_after_motion,
                publish_portal_passages,
            ).chain());
            let first = ambition_platformer2d_world::governing_rules::test_fixtures::two_game_session(&mut app, true);
            assert_eq!(first, LiveRoomInstance::ACTIVATION);
            use ambition_platformer2d_world::scoped_rules::{DeclaredRules, RulesScope};
            use ambition_platformer2d_shared_tangle::capability::{UsedCapabilities, Capability};
            let mut rules = DeclaredRules::default();
            rules.declare(RulesScope::UntaggedRooms, UsedCapabilities::ALL);
            rules.declare(RulesScope::Mode("smash"), UsedCapabilities::ALL.without(Capability::Portals));
            app.insert_resource(rules);
            for index in 0..count {
                let body = ae::BodyKinematics { pos: Vec2::new(index as f32 * 100.0, -100.0),
                    canonical_size: Vec2::splat(16.0), ..Default::default() };
                app.world_mut().spawn((body, InRoomInstance(LiveRoomInstance::from_ordinal(index % 2))));
            }
            for pair in 0..pairs {
                for end in 0..2 {
                    let portal = PlacedPortal::fixed(
                        crate::PortalChannel::Authored(crate::PortalChannelColor::Indexed((pair * 2 + end) as u8)),
                        Vec2::new(pair as f32 * 1000.0 + end as f32 * 100.0, 0.0),
                        if end == 0 { -Vec2::X } else { Vec2::X }, 40.0,
                    ).unwrap();
                    app.world_mut().spawn((portal, InRoomInstance(LiveRoomInstance::from_ordinal(0))));
                }
            }
            for _ in 0..5 {
                app.world_mut().run_schedule(Apertures);
                app.world_mut().run_schedule(Occupancy);
            }
            let start = std::time::Instant::now();
            for _ in 0..100 { app.world_mut().run_schedule(Apertures); }
            let aperture_us = start.elapsed().as_secs_f64() * 10_000.0;
            let start = std::time::Instant::now();
            for _ in 0..100 { app.world_mut().run_schedule(Occupancy); }
            let occupancy_us = start.elapsed().as_secs_f64() * 10_000.0;
            let mut query = app.world_mut().query::<(Entity, &ae::BodyKinematics, &InRoomInstance)>();
            let bodies: Vec<_> = query.iter(app.world()).map(|(entity, body, room)| (entity, *body, room.0)).collect();
            let geometry = app.world().resource::<BodyGeometry>();
            assert!(geometry.links_in(Some(LiveRoomInstance::from_ordinal(1))).is_empty());
            let start = std::time::Instant::now();
            for _ in 0..10 {
                for (entity, body, room) in &bodies {
                    let shape = ae::CombatVolume::aabb(ae::Aabb::new(body.pos, body.size() * 0.5));
                    std::hint::black_box(geometry.pieces(&shape,
                        ambition_platformer2d_shared_tangle::body::geometry::PartSource::Body(*entity), body.pos, Some(*room)).unwrap());
                }
            }
            let consumer_us = start.elapsed().as_secs_f64() * 100_000.0;
            let material_payload_bytes = count as usize * std::mem::size_of::<(Entity, Result<ae::frame::Sim2, ae::frame::NumericFault>)>();
            eprintln!("PUBLICATION_COST bodies={count} pairs={pairs} rooms=2 aperture_us={aperture_us:.3} occupancy_us={occupancy_us:.3} consumer_batch_us={consumer_us:.3} material_payload_bytes={material_payload_bytes}");
        }
    }
}
