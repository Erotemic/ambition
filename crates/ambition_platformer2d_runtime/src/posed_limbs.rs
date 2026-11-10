//! A posed limb stops at native material contact on either side of a portal.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;

/// Stop each posed limb where the pose its owner gave it this tick meets
/// something solid.
pub fn stop_posed_limbs_at_what_they_meet(
    poses: ambition_platformer2d_shared_tangle::body::pose::BodyPoseAdmission,
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    geometry: Option<Res<ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry>>,
    mut limbs: Query<
        (
            Entity,
            &mut ae::BodyKinematics,
            &mut ae::SweepSample,
            &ae::PoseOwnedExternally,
            Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
            Option<&mut ae::CenteredAabb>,
        ),
        With<ambition_characters::actor::Limb>,
    >,
) {
    let mut composed = ambition_platformer2d_world::collision::ComposedRooms::default();
    for (limb, mut kin, mut step, holds, room, aabb) in &mut limbs {
        if !holds.holds(ae::PoseHold::Conductor) {
            continue;
        }
        let delta = step.curr - step.prev;
        let half = kin.size() * 0.5;
        if delta == Vec2::ZERO {
            continue;
        }
        let Some(world) = composed.solids_for(&collision, room, limb) else {
            continue;
        };
        let from = ae::Aabb::new(step.prev, half);
        let solid = |block: &ae::Block| ae::collision_semantics::is_full_collision_surface(block.kind);
        // Posed in something already: the owner's.
        if world.blocks.near(from).into_iter().any(|block| solid(block) && block.aabb.strict_intersects(from)) {
            continue;
        }
        let Ok(domain) = kin.canonical_box() else { continue; };
        let Ok(cast) = step.material_cast(&domain) else { continue; };
        let admission = geometry.as_deref().and_then(|geometry| geometry.motion_for(limb, room.map(|room| room.0)));
        let contact = world.first_material_sweep(cast, admission, |collider, _, _| {
            Ok(match collider {
                ae::world::MaterialCollider::Block(block) => solid(block),
                ae::world::MaterialCollider::Chain { chain, .. } => chain.kind == ae::world::SurfaceKind::Solid,
            })
        });
        let at = match contact {
            Ok(Some(hit)) => (step.prev.as_dvec2() + delta.as_dvec2() * hit.contact.toi).as_vec2(),
            Ok(None) => step.curr,
            Err(_) => step.prev,
        };
        if at != step.curr {
            // A pin: the limb holds where it stopped, at its own velocity.
            let desired = ae::BodyKinematics { pos: at, ..*kin };
            if poses.constrain(limb, &mut kin, Some(&mut *step), desired).is_err() { continue; }
            if let Some(mut aabb) = aabb {
                aabb.center = at;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::frame::{MapConvention, PortalAperture, PortalFrame};
    use ambition_platformer2d_shared_tangle::feature_overlay::{FeatureEcsWorldOverlay, PortalHolePass};

    /// A floor (its top at y = 300) with a floor portal at x = 100, and a
    /// ceiling over a shelf at x = 600: the ceiling portal at y = 100, the
    /// shelf's top 40 under it.
    fn app(glued: bool) -> (App, Entity) {
        let mut app = App::new();
        let floor = ae::Block::solid("floor", ae::Vec2::new(0.0, 300.0), ae::Vec2::new(400.0, 40.0));
        let shelf = ae::Block::solid("shelf", ae::Vec2::new(560.0, 140.0), ae::Vec2::new(80.0, 10.0));
        let room = ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            ae::RoomGeometry(ae::World::new("limb", ae::Vec2::new(800.0, 600.0), ae::Vec2::ZERO, vec![floor, shelf])),
        );
        app.add_systems(Update, stop_posed_limbs_at_what_they_meet);
        let host = app.world_mut().spawn_empty().id();
        let at = ae::Vec2::new(100.0, 270.0);
        let limb = app
            .world_mut()
            .spawn((
                ae::BodyKinematics { material: Default::default(), pos: at, vel: ae::Vec2::ZERO, canonical_size: ae::Vec2::splat(16.0), facing: 1.0 },
                ae::SweepSample { prev: at, curr: at, ..Default::default() },
                ae::PoseOwnedExternally::only(ae::PoseHold::Conductor),
                ambition_characters::actor::Limb {
                    of: host,
                    slot: ambition_characters::actor::LimbSlot::HAND_RIGHT,
                    home_offset: ae::Vec2::ZERO,
                },
            ))
            .id();
        let mut overlay = FeatureEcsWorldOverlay::default();
        if glued {
            let enter = PortalAperture { frame: PortalFrame::fixed(ae::Vec2::new(100.0, 300.0), -ae::Vec2::Y), half_length: 30.0 };
            let exit = PortalAperture { frame: PortalFrame::fixed(ae::Vec2::new(600.0, 100.0), ae::Vec2::Y), half_length: 30.0 };
            let mut geometry = ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry::new(MapConvention::Reflection);
            geometry.add_pair(None, (enter, exit));
            geometry.add_straddler(limb, enter, exit);
            app.insert_resource(geometry);
            overlay.portal_holes.push(PortalHolePass {
                face: ae::frame::FaceRect { center: ae::Vec2::new(100.0, 330.0), normal: -ae::Vec2::Y, half_along: 30.0, half_out: 30.0 },
                bodies: vec![limb],
            });
        }
        app.world_mut().entity_mut(room).insert(overlay);
        (app, limb)
    }

    /// Pose the limb `step` further down each tick, from where it is.
    fn pose_down(app: &mut App, limb: Entity, step: f32, ticks: usize) -> f32 {
        for _ in 0..ticks {
            let pos = app.world().get::<ae::BodyKinematics>(limb).unwrap().pos;
            let to = pos + ae::Vec2::new(0.0, step);
            app.world_mut().get_mut::<ae::BodyKinematics>(limb).unwrap().pos = to;
            *app.world_mut().get_mut::<ae::SweepSample>(limb).unwrap() = ae::SweepSample { prev: pos, curr: to, ..Default::default() };
            app.update();
        }
        app.world().get::<ae::BodyKinematics>(limb).unwrap().pos.y
    }

    #[test]
    fn a_posed_limb_stops_on_the_floor_it_is_posed_into() {
        let (mut app, limb) = app(false);
        // Its bottom at 278, posed 12 a tick down: it stops on the floor.
        assert_eq!(pose_down(&mut app, limb, 12.0, 6), 292.0);
    }

    #[test]
    fn a_posed_limb_through_a_pair_stops_on_the_shelf_at_the_far_end() {
        let (mut app, limb) = app(true);
        // In the pair it goes into the floor's opening, and stops where its
        // part through the pair meets the shelf: its bottom 40 past the
        // floor's plane, its centre at 332.
        assert_eq!(pose_down(&mut app, limb, 12.0, 10), 332.0);
    }

    fn aperture(x: f32, y: f32, normal: Vec2, half_length: f32) -> PortalAperture {
        PortalAperture { frame: PortalFrame::fixed(Vec2::new(x, y), normal), half_length }
    }

    fn native_scene(pairs: &[(PortalAperture, PortalAperture)], shelves: Vec<ae::Block>, entries: &[usize]) -> (App, Entity, Vec<Entity>) {
        use ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry;
        let mut app = App::new();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(), ae::RoomGeometry(ae::World::new("limb contacts", Vec2::splat(2000.0), Vec2::ZERO, shelves)));
        app.add_systems(Update, stop_posed_limbs_at_what_they_meet);
        let host = app.world_mut().spawn_empty().id();
        let mut geometry = BodyGeometry::new(MapConvention::Reflection)
            .with_default_budget(ae::frame::thread::PieceBudget::COMPOSED);
        for &pair in pairs { geometry.add_pair(None, pair); }
        let mut limbs = Vec::new();
        for (slot, pair) in [ambition_characters::actor::LimbSlot::HAND_LEFT,
            ambition_characters::actor::LimbSlot::HAND_RIGHT].into_iter().zip(entries.iter().map(|&i| &pairs[i])) {
            let at = pair.0.frame.origin - Vec2::Y * 30.0;
            let kin = ae::BodyKinematics { pos: at, canonical_size: Vec2::splat(16.0), ..Default::default() };
            let limb = app.world_mut().spawn((kin, ae::SweepSample::at_rest(kin, Vec2::Y),
                ae::PoseOwnedExternally::only(ae::PoseHold::Conductor),
                ambition_characters::actor::Limb { of: host, slot, home_offset: at })).id();
            geometry.add_straddler(limb, pair.0, pair.1);
            limbs.push(limb);
        }
        app.insert_resource(geometry);
        (app, host, limbs)
    }

    #[test]
    fn two_attached_limbs_meet_independent_scaled_exits_in_the_same_tick() {
        let pairs = [
            (aperture(100.0, 300.0, -Vec2::Y, 40.0), aperture(600.0, 100.0, Vec2::Y, 20.0)),
            (aperture(300.0, 300.0, -Vec2::Y, 20.0), aperture(900.0, 100.0, Vec2::Y, 40.0)),
        ];
        let (mut app, host, limbs) = native_scene(&pairs, vec![
            ae::Block::solid("small hand stop", Vec2::new(560.0, 140.0), Vec2::new(80.0, 10.0)),
            ae::Block::solid("large hand stop", Vec2::new(840.0, 180.0), Vec2::new(120.0, 10.0)),
        ], &[0, 1]);
        for _ in 0..12 {
            for &limb in &limbs {
                let kin = *app.world().get::<ae::BodyKinematics>(limb).unwrap();
                let desired = ae::BodyKinematics { pos: kin.pos + Vec2::Y * 12.0, ..kin };
                app.world_mut().entity_mut(limb).insert((desired,
                    ae::SweepSample { prev: kin.pos, curr: desired.pos, ..ae::SweepSample::at_rest(kin, Vec2::Y) }));
            }
            app.update();
        }
        for (&limb, expected) in limbs.iter().zip([372.0, 332.0]) {
            assert_eq!(app.world().get::<ae::BodyKinematics>(limb).unwrap().pos.y, expected);
            assert_eq!(app.world().get::<ambition_characters::actor::Limb>(limb).unwrap().of, host);
        }
        assert_eq!(app.world_mut().query::<&ambition_characters::actor::Limb>().iter(app.world()).count(), 2);
    }

    #[test]
    fn a_fast_attached_limb_stops_at_the_solid_after_three_portal_pairs() {
        let pairs = [
            (aperture(100.0, 300.0, -Vec2::Y, 40.0), aperture(600.0, 100.0, Vec2::Y, 20.0)),
            (aperture(600.0, 120.0, -Vec2::Y, 20.0), aperture(1000.0, 100.0, Vec2::Y, 20.0)),
            (aperture(1000.0, 120.0, -Vec2::Y, 20.0), aperture(1400.0, 100.0, Vec2::Y, 40.0)),
        ];
        let (mut app, host, limbs) = native_scene(&pairs, vec![
            ae::Block::solid("third exit stop", Vec2::new(1360.0, 140.0), Vec2::new(80.0, 10.0)),
        ], &[0]);
        let limb = limbs[0];
        assert_eq!(pose_down(&mut app, limb, 200.0, 1), 412.0);
        assert_eq!(app.world().get::<ambition_characters::actor::Limb>(limb).unwrap().of, host);
        assert_eq!(app.world().get::<ae::SweepSample>(limb).unwrap().curr.y, 412.0);
    }

}
