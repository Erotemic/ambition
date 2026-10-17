//! A body is not inside a solid outline ([`SurfaceKind::Solid`]): what a
//! portal pair carries of a block when it turns it is that block's true shape,
//! turned, and a walker's box meets it as it is
//! (`docs/planning/engine/portals-a-body-in-two-places.md`, exact turned
//! solids). The box is swept against boxes and stands on lines; an outline it
//! ends a step inside puts it out the shortest way (separating axes: the two
//! world axes and each edge's normal) and takes the speed it had into it.
//! Its edges that face up are ground the way a slope's are (`walkable_lines`).

use crate::world::{SurfaceKind, World};
use crate::{Aabb, AabbExt, Vec2};

/// How deep a box must be in an outline to be put out: less is touching.
const SLOP: f32 = 1e-3;

/// How far to move `body` to be out of the outline `points` (wound with
/// outward normals `(t.y, -t.x)`), and the outline's normal there; `None`
/// when it is not in it.
pub fn push_out(body: Aabb, points: &[Vec2]) -> Option<(Vec2, Vec2)> {
    if points.len() < 3 {
        return None;
    }
    let corners = [body.min, Vec2::new(body.max.x, body.min.y), body.max, Vec2::new(body.min.x, body.max.y)];
    let span = |pts: &mut dyn Iterator<Item = f32>| pts.fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)));
    let mut best: Option<(f32, Vec2)> = None;
    let mut axes: Vec<Vec2> = vec![Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y];
    for i in 0..points.len() {
        let t = points[(i + 1) % points.len()] - points[i];
        let n = Vec2::new(t.y, -t.x).normalize_or_zero();
        if n != Vec2::ZERO {
            axes.push(n);
        }
    }
    for n in axes {
        let (body_lo, _) = span(&mut corners.iter().map(|c| c.dot(n)));
        let (_, solid_hi) = span(&mut points.iter().map(|p| p.dot(n)));
        // Moving the box along `n` by this much puts it past the outline's
        // far side on that axis.
        let depth = solid_hi - body_lo;
        if depth <= SLOP {
            return None;
        }
        if best.is_none_or(|(d, _)| depth < d) {
            best = Some((depth, n));
        }
    }
    best.map(|(depth, n)| (n * depth, n))
}

/// Put `body` out of every solid outline of `world` it is in: the total move,
/// and the normal of the last outline it was put out of.
pub fn put_out_of_solid_lines(world: &World, mut body: Aabb) -> Option<(Vec2, Vec2)> {
    let mut moved = Vec2::ZERO;
    let mut normal = None;
    // An outline put out of can leave the box in another; a few rounds settle
    // it (the outlines a pair carries are few and apart).
    for _ in 0..4 {
        let mut pushed = false;
        for chain in world.chains.near(body) {
            if chain.kind != SurfaceKind::Solid || !chain.closed {
                continue;
            }
            if let Some((push, n)) = push_out(body, &chain.points) {
                body = Aabb::new(body.center() + push, body.half_size());
                moved += push;
                normal = Some(n);
                pushed = true;
            }
        }
        if !pushed {
            break;
        }
    }
    normal.map(|n| (moved, n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::SurfaceChain;

    fn diamond(center: Vec2, r: f32) -> SurfaceChain {
        SurfaceChain::solid(
            "diamond",
            vec![center + Vec2::new(0.0, -r), center + Vec2::new(r, 0.0), center + Vec2::new(0.0, r), center + Vec2::new(-r, 0.0)],
        )
    }

    #[test]
    fn a_box_in_a_turned_square_is_put_out_the_shortest_way_and_one_beside_it_is_not() {
        let world = World::new("solid", Vec2::splat(400.0), Vec2::ZERO, Vec::new());
        let mut world = world;
        world.chains.push(diamond(Vec2::new(200.0, 200.0), 40.0));
        // Beside the diamond, in the box around it: not in it.
        let corner = Aabb::new(Vec2::new(232.0, 168.0), Vec2::splat(5.0));
        assert_eq!(put_out_of_solid_lines(&world, corner), None);
        // Sunk 6 into its lower-right face: out along that face's normal.
        let n = Vec2::new(1.0, 1.0).normalize();
        let face = Vec2::new(220.0, 220.0);
        let body = Aabb::new(face + n * (5.0 * 2f32.sqrt() - 6.0), Vec2::splat(5.0));
        let (moved, normal) = put_out_of_solid_lines(&world, body).expect("it is in it");
        assert!((normal - n).length() < 1e-4, "{normal:?}");
        assert!((moved - n * 6.0).length() < 1e-3, "{moved:?}");
        assert_eq!(put_out_of_solid_lines(&world, Aabb::new(body.center() + moved, body.half_size())), None);
    }
}
