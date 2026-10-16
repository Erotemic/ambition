#![cfg(all(feature = "portal", feature = "rl_sim"))]
//! A portal on a moving host is carved where it is on the tick a body
//! crosses it, in the shipped schedule.
//!
//! The carve was cut before the platforms moved, from the pose the portal had
//! the tick before, and the portal took its host's new pose only after the
//! bodies integrated. A body sank into a hole one tick behind the plane it
//! crossed. Now the hosts move, the portals take their pose, the carves are
//! cut, and then the bodies integrate.

use crate::common::{base, fixed_60hz_room_sim};

use ambition_platformer2d::engine_core::Vec2;
use ambition_platformer2d::item::GroundItem;
use ambition_platformer2d::portal::{pieces, PlacedPortal, PortalCarves, PortalHost};
use bevy::prelude::*;

#[test]
fn a_moving_portal_is_carved_where_it_is() {
    let mut sim = fixed_60hz_room_sim("portal_lab");
    for _ in 0..30 {
        sim.step(base());
    }
    // The portal on the moving plank (pair F): hosted on a line that moves.
    let (portal, normal) = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &PlacedPortal)>();
        q.iter(world)
            .find(|(_, p)| matches!(p.host, PortalHost::Line { .. }) && p.vel.length() > 1.0)
            .map(|(entity, p)| (entity, p.normal))
            .expect("premise: portal_lab has a portal on a moving line")
    };
    let mut carved_ticks = 0;
    for tick in 0..20 {
        // An item dropped into it each tick, so the portal is open.
        let at = sim.world().get::<PlacedPortal>(portal).expect("the portal stays").pos;
        let spec = ambition_platformer2d::character::held_item_by_id("gun_sword").expect("gun_sword is registered");
        sim.world_mut().spawn(GroundItem::released(spec, at + normal * 6.0, -normal * 300.0, Vec2::splat(6.0)));
        sim.step(base());

        let p = sim.world().get::<PlacedPortal>(portal).expect("the portal stays").clone();
        let here = pieces::carve_face_with_depth(&p.aperture(), f32::INFINITY);
        let carves = sim.world().resource::<PortalCarves>();
        // Its carve, by where it is: within a few px of the plane's centre.
        let Some((_, face)) = carves.faces.iter().find(|(_, face)| face.center.distance(here.center) < 12.0) else {
            continue;
        };
        carved_ticks += 1;
        assert!(
            face.center.distance(here.center) < 1e-3 && face.normal.distance(here.normal) < 1e-4,
            "tick {tick}: the carve is not where the portal is: carve {face:?}, portal {here:?} (moving {:?})",
            p.vel
        );
    }
    assert!(carved_ticks > 0, "premise: no tick carved the moving portal");
}
