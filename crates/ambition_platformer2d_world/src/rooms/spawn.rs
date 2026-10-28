//! Body placement in room coordinates.
//!
//! A requested destination can be repaired. A body can also be placed near
//! another body. Both policies use the same terrain clearance test.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;

use super::{LoadingZone, LoadingZoneActivation};

pub(super) fn block_kind_label(kind: ae::BlockKind) -> &'static str {
    match kind {
        ae::BlockKind::Solid => "solid",
        ae::BlockKind::BlinkWall { .. } => "blink wall",
        ae::BlockKind::Barrier => "barrier",
        ae::BlockKind::OneWay => "one-way platform",
        ae::BlockKind::BonkOnly => "bonk-only block",
        ae::BlockKind::Hazard => "hazard",
        ae::BlockKind::PogoOrb => "pogo orb",
        ae::BlockKind::Rebound { .. } => "rebound pad",
    }
}

pub(super) const WALL: f32 = 36.0;
pub(super) const EDGE_ARRIVAL_INSET: f32 = 92.0;
pub(super) const PLAYER_HALF_W: f32 = 14.0;
pub(super) const PLAYER_HALF_H: f32 = 23.0;
pub(super) const SPAWN_MARGIN: f32 = 3.0;

pub(super) fn arrival_from_target_zone(world: &ae::World, zone: &LoadingZone) -> ae::Vec2 {
    match zone.activation {
        // `Walk` zones can be anywhere in the room, so they use the `Door`
        // rule: the target zone defines the landing point.
        LoadingZoneActivation::Door | LoadingZoneActivation::Walk => door_arrival(zone.aabb),
        LoadingZoneActivation::EdgeExit => edge_arrival(world, zone.aabb),
    }
}

fn edge_arrival(world: &ae::World, zone: ae::Aabb) -> ae::Vec2 {
    // Classify by shape: a tall narrow zone is a side seam (left/right edge);
    // a wide short zone is a top/bottom seam. The author does not need to
    // declare the edge.
    let zone_w = zone.right() - zone.left();
    let zone_h = zone.bottom() - zone.top();
    if zone_w >= zone_h {
        // Top/bottom edge: inset Y, center X over the zone.
        let y = if zone.top() <= WALL + 1.0 {
            EDGE_ARRIVAL_INSET
        } else if zone.bottom() >= world.size.y - WALL - 1.0 {
            world.size.y - EDGE_ARRIVAL_INSET
        } else {
            zone.center().y
        };
        ae::Vec2::new(zone.center().x, y)
    } else {
        // Left/right edge: inset X, center Y over the zone.
        let x = if zone.left() <= WALL + 1.0 {
            EDGE_ARRIVAL_INSET
        } else if zone.right() >= world.size.x - WALL - 1.0 {
            world.size.x - EDGE_ARRIVAL_INSET
        } else {
            zone.center().x
        };
        ae::Vec2::new(x, zone.center().y)
    }
}

fn door_arrival(zone: ae::Aabb) -> ae::Vec2 {
    ae::Vec2::new(
        zone.center().x,
        zone.bottom() - PLAYER_HALF_H - SPAWN_MARGIN,
    )
}

/// Find clear space near a body in its reference frame.
/// The collision world and occupied boxes must describe the same room.
/// No position is returned when the adjacent positions are blocked.
pub fn spawn_near_body(
    world: &ae::World,
    reference: ae::Aabb,
    frame: ae::AccelerationFrame,
    shape: ae::BodySpawnShape,
    occupied: &[ae::Aabb],
) -> Option<ae::Vec2> {
    let project = |half: ae::Vec2| ae::Vec2::new(
        frame.side.abs().dot(half),
        frame.down.abs().dot(half),
    );
    let reference_half = project(reference.half_size());
    let body_half = project(shape.body_half);
    let apart = reference_half + body_half + ae::Vec2::splat(SPAWN_MARGIN);
    let align = reference_half - body_half;
    let candidates = [
        ae::Vec2::new(apart.x, align.y),
        ae::Vec2::new(-apart.x, align.y),
        ae::Vec2::new(apart.x, 0.0),
        ae::Vec2::new(-apart.x, 0.0),
        ae::Vec2::new(apart.x, -align.y),
        ae::Vec2::new(-apart.x, -align.y),
        ae::Vec2::new(align.x, -apart.y),
        ae::Vec2::new(-align.x, -apart.y),
        ae::Vec2::new(0.0, -apart.y),
        ae::Vec2::new(align.x, apart.y),
        ae::Vec2::new(-align.x, apart.y),
        ae::Vec2::new(0.0, apart.y),
        ae::Vec2::new(apart.x, -apart.y),
        ae::Vec2::new(-apart.x, -apart.y),
        ae::Vec2::new(apart.x, apart.y),
        ae::Vec2::new(-apart.x, apart.y),
    ];
    candidates.into_iter()
        .map(|offset| reference.center() + frame.to_world(offset))
        .find(|at| {
            let body = ae::Aabb::new(*at, shape.body_half);
            !reference.strict_intersects(body)
                && !occupied.iter().any(|other| other.strict_intersects(body))
                && body_clear(world, *at, shape.terrain_half, shape.line_depth)
        })
}

/// Clamp and repair a proposed player spawn so transitions never place the
/// player outside the room or embedded in solids.
pub fn validated_spawn(world: &ae::World, desired: ae::Vec2, player_size: ae::Vec2) -> ae::Vec2 {
    let half = player_size * 0.5;
    let base = clamp_spawn_to_room(world, desired, half);
    if body_clear(world, base, half, None) {
        return base;
    }

    const STEP: f32 = 8.0;
    for y_step in 0..=96 {
        let dy = -(y_step as f32) * STEP;
        for x_step in 0..=96 {
            if x_step == 0 {
                let candidate =
                    clamp_spawn_to_room(world, ae::Vec2::new(base.x, base.y + dy), half);
                if body_clear(world, candidate, half, None) {
                    return candidate;
                }
            } else {
                for sign in [-1.0_f32, 1.0] {
                    let dx = sign * x_step as f32 * STEP;
                    let candidate =
                        clamp_spawn_to_room(world, ae::Vec2::new(base.x + dx, base.y + dy), half);
                    if body_clear(world, candidate, half, None) {
                        return candidate;
                    }
                }
            }
        }
    }

    base
}

fn clamp_spawn_to_room(world: &ae::World, pos: ae::Vec2, half: ae::Vec2) -> ae::Vec2 {
    ae::Vec2::new(
        pos.x
            .clamp(half.x + SPAWN_MARGIN, world.size.x - half.x - SPAWN_MARGIN),
        pos.y
            .clamp(half.y + SPAWN_MARGIN, world.size.y - half.y - SPAWN_MARGIN),
    )
}

fn body_clear(world: &ae::World, center: ae::Vec2, half: ae::Vec2, line_depth: Option<i8>) -> bool {
    let body = ae::Aabb::new(center, half);
    if !ae::classify_safety_from_kinematics(
        center, ae::Vec2::ZERO, body, world, -SPAWN_MARGIN,
        |block| ae::collision_semantics::is_support_surface(block.kind)
            || matches!(block.kind, ae::BlockKind::Hazard | ae::BlockKind::Rebound { .. }),
    ).is_safe() || ae::solid_lines::put_out_of_solid_lines(world, body).is_some() {
        return false;
    }
    !world.chains.near(body).into_iter().any(|chain| {
        chain.kind != ae::world::SurfaceKind::Solid && (0..chain.segment_count()).any(|index| {
            if line_depth.is_some_and(|depth| chain.segment_depth(index) != depth) {
                return false;
            }
            let (a, b) = chain.segment(index);
            if a == b {
                return false;
            }
            ae::cast::ray_aabb(a, b - a, body)
                .zip(ae::cast::ray_aabb(b, a - b, body))
                .is_some_and(|((enter, _), (back, _))| enter + back < 1.0)
        })
    })
}

#[cfg(test)]
mod spawn_tests {
    //! Spawn placement safety: clamping into the room and the spiral search that nudges a
    //! player out of a solid.
    use super::*;

    fn world_with(blocks: Vec<ae::Block>) -> ae::World {
        ae::World::new(
            "test",
            ae::Vec2::new(400.0, 400.0),
            ae::Vec2::new(50.0, 50.0),
            blocks,
        )
    }

    fn solid_filter(b: &ae::Block) -> bool {
        matches!(b.kind, ae::BlockKind::Solid)
    }

    #[test]
    fn door_arrival_sits_just_below_the_zone() {
        let zone = ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(20.0, 40.0));
        let pos = door_arrival(zone);
        assert_eq!(pos.x, zone.center().x);
        assert_eq!(pos.y, zone.bottom() - PLAYER_HALF_H - SPAWN_MARGIN);
        assert!(pos.y < zone.bottom(), "arrival is above the doorway sill");
    }

    #[test]
    fn clamp_keeps_spawn_inside_the_room_margins() {
        let world = world_with(Vec::new());
        let half = ae::Vec2::new(14.0, 23.0);
        let clamped = clamp_spawn_to_room(&world, ae::Vec2::new(-100.0, 1000.0), half);
        assert!(clamped.x >= half.x + SPAWN_MARGIN);
        assert!(clamped.y >= half.y + SPAWN_MARGIN);
        assert!(clamped.x <= world.size.x - half.x - SPAWN_MARGIN);
        assert!(clamped.y <= world.size.y - half.y - SPAWN_MARGIN);
    }

    #[test]
    fn validated_spawn_in_open_room_keeps_a_valid_point() {
        let world = world_with(Vec::new());
        let desired = ae::Vec2::new(200.0, 200.0);
        let got = validated_spawn(&world, desired, ae::Vec2::new(28.0, 46.0));
        assert_eq!(
            got, desired,
            "an already-clear in-bounds spawn is unchanged"
        );
    }

    #[test]
    fn validated_spawn_pushes_the_player_out_of_a_solid() {
        // Solid covering (100,100)..(200,200); desired is dead center.
        let block = ae::Block::solid(
            "wall",
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(100.0, 100.0),
        );
        let world = world_with(vec![block]);
        let player_size = ae::Vec2::new(28.0, 46.0);
        let half = player_size * 0.5;
        let got = validated_spawn(&world, ae::Vec2::new(150.0, 150.0), player_size);
        let body = ae::Aabb::new(got, half);
        assert!(
            !world.body_overlaps_any(body, solid_filter),
            "validated spawn must not embed the player in a solid (got {got:?})",
        );
    }

    fn shape(size: ae::Vec2) -> ae::BodySpawnShape {
        ae::MotionModelSpec::AxisSwept(Default::default())
            .spawn_shape(size, ae::Vec2::Y)
    }

    #[test]
    fn a_nearby_body_avoids_existing_bodies_independent_of_their_order() {
        let world = world_with(Vec::new());
        let reference = ae::Aabb::new(ae::Vec2::splat(200.0), ae::Vec2::new(14.0, 23.0));
        let shape = shape(ae::Vec2::new(28.0, 46.0));
        let frame = ae::AccelerationFrame::new(ae::Vec2::Y);
        let first = spawn_near_body(&world, reference, frame, shape, &[]).unwrap();
        let occupied = ae::Aabb::new(first, shape.body_half);
        let far = ae::Aabb::new(ae::Vec2::splat(50.0), shape.body_half);
        let second = spawn_near_body(&world, reference, frame, shape, &[occupied, far]).unwrap();
        assert_ne!(first, second);
        let body = ae::Aabb::new(second, shape.body_half);
        assert!(!reference.strict_intersects(body));
        assert!(!occupied.strict_intersects(body));
        assert_eq!(Some(second), spawn_near_body(&world, reference, frame, shape, &[far, occupied]));
    }

    #[test]
    fn a_nearby_body_fits_beside_a_wall_and_avoids_hazards() {
        for block in [
            ae::Block::solid("wall", ae::Vec2::new(164.0, 0.0), ae::Vec2::new(100.0, 400.0)),
            ae::Block::hazard("hazard", ae::Vec2::new(164.0, 0.0), ae::Vec2::new(100.0, 400.0)),
        ] {
            let obstacle = block.aabb;
            let world = world_with(vec![block]);
            let reference = ae::Aabb::new(ae::Vec2::new(150.0, 200.0), ae::Vec2::new(14.0, 23.0));
            let shape = shape(ae::Vec2::new(28.0, 46.0));
            let at = spawn_near_body(&world, reference, ae::AccelerationFrame::new(ae::Vec2::Y), shape, &[]).unwrap();
            assert!(at.x < reference.center().x);
            assert!(!ae::Aabb::new(at, shape.terrain_half).strict_intersects(obstacle));
        }
    }

    #[test]
    fn a_blocked_or_undersized_room_refuses_a_nearby_spawn() {
        let reference = ae::Aabb::new(ae::Vec2::splat(200.0), ae::Vec2::new(14.0, 23.0));
        let shape = shape(ae::Vec2::new(28.0, 46.0));
        let frame = ae::AccelerationFrame::new(ae::Vec2::Y);
        assert!(spawn_near_body(&world_with(Vec::new()), reference, frame, shape, &[]).is_some());
        let full = world_with(vec![ae::Block::solid("full", ae::Vec2::ZERO, ae::Vec2::splat(400.0))]);
        assert!(spawn_near_body(&full, reference, frame, shape, &[]).is_none());
        let tiny = ae::World::new("tiny", ae::Vec2::splat(20.0), ae::Vec2::splat(10.0), Vec::new());
        assert!(spawn_near_body(&tiny, reference, frame, shape, &[]).is_none());
    }

    #[test]
    fn nearby_spawns_clear_lines_and_turned_solids() {
        let reference = ae::Aabb::new(ae::Vec2::splat(200.0), ae::Vec2::new(14.0, 23.0));
        let shape = shape(ae::Vec2::new(28.0, 46.0));
        let frame = ae::AccelerationFrame::new(ae::Vec2::Y);
        let first = spawn_near_body(&world_with(Vec::new()), reference, frame, shape, &[]).unwrap();
        let line = ae::SurfaceChain::open("line", vec![first - ae::Vec2::Y * 100.0, first + ae::Vec2::Y * 100.0]);
        let solid = ae::SurfaceChain::solid("diamond", vec![
            first - ae::Vec2::Y * 30.0,
            first + ae::Vec2::X * 30.0,
            first + ae::Vec2::Y * 30.0,
            first - ae::Vec2::X * 30.0,
        ]);
        for chain in [line, solid] {
            let world = world_with(Vec::new()).with_chains(vec![chain]);
            let at = spawn_near_body(&world, reference, frame, shape, &[]).unwrap();
            assert_ne!(at, first);
            let body = ae::Aabb::new(at, shape.terrain_half);
            assert!(body.max.x <= first.x || body.min.x >= first.x);
            assert!(ae::solid_lines::put_out_of_solid_lines(&world, body).is_none());
        }
    }

    #[test]
    fn a_surface_riders_contact_circle_also_needs_clearance() {
        let world = world_with(vec![
            ae::Block::solid("left", ae::Vec2::ZERO, ae::Vec2::new(180.0, 400.0)),
            ae::Block::solid("right", ae::Vec2::new(220.0, 0.0), ae::Vec2::new(180.0, 400.0)),
        ]);
        let reference = ae::Aabb::new(ae::Vec2::splat(200.0), ae::Vec2::new(10.0, 30.0));
        let size = ae::Vec2::new(20.0, 60.0);
        let frame = ae::AccelerationFrame::new(ae::Vec2::Y);
        assert!(spawn_near_body(&world, reference, frame, shape(size), &[]).is_some());
        let rider = ae::MotionModelSpec::SurfaceMomentum(Default::default()).spawn_shape(size, ae::Vec2::Y);
        assert!(spawn_near_body(&world, reference, frame, rider, &[]).is_none());
    }

    #[test]
    fn nearby_placement_follows_the_reference_frame() {
        let world = world_with(Vec::new());
        let center = ae::Vec2::splat(200.0);
        let rotate = |p: ae::Vec2| center + ae::Vec2::new(-(p - center).y, (p - center).x);
        let reference = ae::Aabb::new(ae::Vec2::new(180.0, 230.0), ae::Vec2::new(12.0, 30.0));
        let original = shape(ae::Vec2::new(28.0, 46.0));
        let first = spawn_near_body(&world, reference, ae::AccelerationFrame::new(ae::Vec2::Y), original, &[]).unwrap();
        let rotated = ae::BodySpawnShape {
            body_half: ae::Vec2::new(original.body_half.y, original.body_half.x),
            terrain_half: ae::Vec2::new(original.terrain_half.y, original.terrain_half.x),
            line_depth: original.line_depth,
        };
        let reference = ae::Aabb::new(rotate(reference.center()), ae::Vec2::new(30.0, 12.0));
        let at = spawn_near_body(&world, reference, ae::AccelerationFrame::new(-ae::Vec2::X), rotated, &[]).unwrap();
        assert_eq!(at, rotate(first));
        let at = spawn_near_body(&world, reference, ae::AccelerationFrame::new(ae::Vec2::ONE), rotated, &[]).unwrap();
        assert!(!reference.strict_intersects(ae::Aabb::new(at, rotated.body_half)));
    }

    #[test]
    fn block_kind_label_is_nonempty() {
        assert!(!block_kind_label(ae::BlockKind::Solid).is_empty());
    }
}
