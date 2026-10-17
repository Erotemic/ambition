//! A limb its owner poses meets what is there
//! (`docs/planning/engine/portals-a-body-in-two-places.md`, G6).
//!
//! A boss's conductor poses a limb each tick (`PoseOwnedExternally`): the pose
//! is where the limb is asked to go. It goes there from where it was until it
//! meets something solid, on its own side or, when it reaches into a portal
//! pair, at the far end: the world it is swept against is the one it meets
//! (`ComposedRooms::solids_for`), whose glue puts what is in front of the far
//! end behind the near one. It stops there. Where it touches is the owner's to
//! read (`ambition.boss.mount`'s `contact`); the host does not recoil.
//!
//! A limb posed into something it was already in, or moved further in one
//! tick than four times its size (a pose that puts it somewhere new, not one
//! it moves to), is left where it is posed.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;

/// Stop each posed limb where the pose its owner gave it this tick meets
/// something solid.
pub fn stop_posed_limbs_at_what_they_meet(
    collision: ambition_platformer2d_world::collision::CollisionWorld,
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
        let half = kin.size * 0.5;
        if delta.length() <= 1e-4 || delta.length() > 4.0 * kin.size.max_element().max(1.0) {
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
        let mut at = step.curr;
        if let Some(hit) = world.first_body_sweep(from, delta, solid) {
            at = step.prev + delta * hit.time_of_impact;
        }
        // What a pair turns is an outline, not a box.
        if let Some((out, _)) = ae::solid_lines::put_out_of_solid_lines(&world, ae::Aabb::new(at, half)) {
            at += out;
        }
        if at != step.curr {
            kin.pos = at;
            step.curr = at;
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
    use ambition_platformer2d_shared_tangle::feature_overlay::{FeatureEcsWorldOverlay, PortalGlue, PortalGluePass};

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
                ae::BodyKinematics { pos: at, vel: ae::Vec2::ZERO, size: ae::Vec2::splat(16.0), facing: 1.0 },
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
            overlay.portal_glues.push(PortalGluePass {
                glue: PortalGlue {
                    enter: PortalAperture { frame: PortalFrame::fixed(ae::Vec2::new(100.0, 300.0), -ae::Vec2::Y), half_length: 30.0 },
                    exit: PortalAperture { frame: PortalFrame::fixed(ae::Vec2::new(600.0, 100.0), ae::Vec2::Y), half_length: 30.0 },
                    convention: MapConvention::Reflection,
                    depth: 60.0,
                },
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
}
