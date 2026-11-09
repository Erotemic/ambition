//! Shared swept collision primitives. Path-dependent gameplay evaluates the
//! continuous body path rather than sampled endpoints.
//!
//! This module exposes swept AABB/AABB, body/world, and solid-world ray casts.
//! The momentum kernel's swept-circle primitive remains private because it
//! depends on `SurfaceChain` resolution internals; portal-aware casting uses the
//! shared aperture vocabulary in [`crate::frame`].

use crate::geometry::Aabb;
use crate::world::{Block, BlockKind, SweepHit, World};
use crate::Vec2;

// Public swept-AABB primitive and hit record.
pub use crate::geometry::{AabbExt, AabbSweepHit};
// Body-vs-world sweep hit record.
pub use crate::world::SweepHit as WorldSweepHit;

/// The earliest Parry-backed swept-AABB hit for `body` moving by `delta` against
/// the world's solid blocks, keeping only blocks the `predicate` accepts. THE
/// body-vs-world sweep entry point (CC1): one call surface so every gameplay
/// question — movement solids, blink blockers, one-way landing, spawn blockers,
/// enemy collision — asks the SAME swept routine instead of re-deriving it.
///
/// Delegates to [`World::first_body_sweep`], which holds the privileged access
/// to the world's block set; this is the public, discoverable name.
pub fn body_sweep<F>(world: &World, body: Aabb, delta: Vec2, predicate: F) -> Option<SweepHit<'_>>
where
    F: FnMut(&Block) -> bool,
{
    world.first_body_sweep(body, delta, predicate)
}

/// Did a body's AABB, centered at `center` with `half` extents and moving by
/// `delta` this frame, CONTACT the static `target` AABB at any point along its
/// swept path? THE trigger-tier swept primitive (CC2): a path-dependent reader
/// (hazard touch, pickup, thin trigger volume) calls this instead of a discrete
/// endpoint overlap, so a fast body — a blink, a dash, a Sanic run — cannot
/// tunnel THROUGH the volume between frames.
///
/// PARITY by construction: it returns `true` for the already-overlapping
/// (standing-in-it) case exactly as the old discrete `strict_intersects` did,
/// then ADDS the swept path on top — so a body that was already detected is
/// unchanged, and the only new detections are genuine tunnels. `delta == ZERO`
/// (a stationary reader) collapses to the pure discrete check.
///
/// This is the body-vs-STATIC-target form (the target's own motion is ignored).
/// A fast MOVING target wants the relative sweep (`delta_body − delta_target`);
/// that generalization rides the moving-portal/CC6 work — a static or slow
/// target is exact here, which covers the tunneling class this converts.
pub fn aabb_path_contacts(center: Vec2, half: Vec2, delta: Vec2, target: Aabb) -> bool {
    let here = Aabb::new(center, half);
    if here.strict_intersects(target) {
        return true;
    }
    if delta == Vec2::ZERO {
        return false;
    }
    Aabb::new(center - delta, half)
        .sweep_hit(delta, target)
        .is_some()
}

/// The minimal world access [`raycast_solids`] needs: a way to visit every solid
/// AABB the ray could hit.
///
/// `raycast_solids` only ever reads, per block, whether the block is hittable
/// (given the `include_one_way` policy) and its [`Aabb`]. This trait captures
/// exactly that — the world decides which blocks count as solid; the raycast
/// just consumes their AABBs. (Moved down from
/// `ambition_platformer2d::platformer::world_query`, CC1 ruling §3.4(b); the
/// canonical `impl` for [`World`] lives right below, beside the type.)
pub trait SolidWorldQuery {
    /// Invoke `visit` once for each solid AABB the ray should test.
    ///
    /// When `include_one_way` is true, one-way platforms are visited too;
    /// otherwise they are skipped (blink/dive/grapple pass through them, while
    /// portal placement adheres to them).
    fn for_each_solid_aabb(&self, include_one_way: bool, visit: &mut dyn FnMut(Aabb));

    /// Invoke `visit` once for each segment of the world's lines (its
    /// surface chains), as `(a, b)` with the line's outward normal
    /// `(t.y, -t.x)` for `t` from `a` to `b`. None by default.
    fn for_each_line(&self, _visit: &mut dyn FnMut(Vec2, Vec2)) {}
}

/// The engine_core world's solid-block policy: `Solid` and `BlinkWall` blocks
/// are always hittable; one-way platforms only when `include_one_way` is set
/// (portal placement adheres to them; blink/dive/grapple pass through).
impl SolidWorldQuery for World {
    fn for_each_solid_aabb(&self, include_one_way: bool, visit: &mut dyn FnMut(Aabb)) {
        for block in &self.blocks {
            let hittable = crate::collision_semantics::is_full_collision_surface(block.kind)
                || (include_one_way && matches!(block.kind, BlockKind::OneWay));
            if hittable {
                visit(block.aabb);
            }
        }
    }

    fn for_each_line(&self, visit: &mut dyn FnMut(Vec2, Vec2)) {
        for chain in &self.chains {
            for i in 0..chain.segment_count() {
                let (a, b) = chain.segment(i);
                visit(a, b);
            }
        }
    }
}

/// What a cast hit: where, the surface's outward normal there, and, for a
/// line, the segment it hit (`(a, b)`), so a caller can ask how much of the
/// line is around the hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceHit {
    pub distance: f32,
    pub point: Vec2,
    pub normal: Vec2,
    pub segment: Option<(Vec2, Vec2)>,
}

/// The nearest surface a ray from `origin` along `dir` hits within
/// `max_dist`: a solid box (as [`raycast_solids_far`]) or a line, which is
/// one-sided: hit only from the side its normal faces (the side a body
/// stands on), passed from behind.
pub fn raycast_surfaces<W: SolidWorldQuery + ?Sized>(
    world: &W,
    origin: Vec2,
    dir: Vec2,
    max_dist: f32,
    include_one_way: bool,
) -> Option<SurfaceHit> {
    let hit = raycast_surfaces_wide(world, origin.as_dvec2(), dir.as_dvec2().normalize_or_zero(), f64::from(max_dist), include_one_way, true)?;
    Some(SurfaceHit {
        distance: hit.distance as f32,
        point: (origin.as_dvec2()+dir.as_dvec2().normalize_or_zero()*hit.distance).as_vec2(),
        normal: hit.normal, segment: hit.segment,
    })
}

/// Nearest solid surface hit by a ray from `origin` along `dir`.
///
/// Returns the hit point and the outward face normal (pointing back toward the
/// ray). `include_one_way` is opt-in because blink/dive/grapple pathing can pass
/// through one-way platforms, while portal placement should adhere to them.
pub fn raycast_solids<W: SolidWorldQuery + ?Sized>(
    world: &W,
    origin: Vec2,
    dir: Vec2,
    max_dist: f32,
    include_one_way: bool,
) -> Option<(Vec2, Vec2)> {
    raycast_solids_far(world, origin, dir, max_dist, include_one_way).map(|(_, hit, normal)| (hit, normal))
}

/// [`raycast_solids`], with how far along the ray the hit is: the distance the
/// cast compared, so a caller that compares it with a shorter reach gets the
/// answer a cast of that reach gives.
pub fn raycast_solids_far<W: SolidWorldQuery + ?Sized>(
    world: &W,
    origin: Vec2,
    dir: Vec2,
    max_dist: f32,
    include_one_way: bool,
) -> Option<(f32, Vec2, Vec2)> {
    let dir = dir.as_dvec2().normalize_or_zero();
    let hit = raycast_surfaces_wide(world, origin.as_dvec2(), dir, f64::from(max_dist), include_one_way, false)?;
    Some((hit.distance as f32, (origin.as_dvec2()+dir*hit.distance).as_vec2(), hit.normal))
}

mod aperture;
pub use aperture::{trace_through_apertures, ApertureTrace, PortalRay, RayBudget};

/// Ray-vs-AABB slab query. Returns `(t_near, face_normal)` for a forward hit
/// (`t >= 0`).
pub fn ray_aabb(origin: Vec2, dir: Vec2, aabb: Aabb) -> Option<(f32, Vec2)> {
    ray_aabb_wide(origin.as_dvec2(), dir.as_dvec2(), aabb).map(|(t, n)| (t as f32, n))
}

fn ray_aabb_wide(origin: bevy_math::DVec2, dir: bevy_math::DVec2, aabb: Aabb) -> Option<(f64, Vec2)> {
    if !origin.is_finite() || !dir.is_finite() || dir == bevy_math::DVec2::ZERO
        || !aabb.min.is_finite() || !aabb.max.is_finite() { return None; }
    let mut near = f64::NEG_INFINITY;
    let mut far = f64::INFINITY;
    let mut normal = Vec2::ZERO;
    for axis in 0..2 {
        let (lo, hi) = (f64::from(aabb.min[axis]), f64::from(aabb.max[axis]));
        if dir[axis] == 0.0 {
            if origin[axis] <= lo || origin[axis] >= hi { return None; }
            continue;
        }
        let (a, b) = ((lo-origin[axis])/dir[axis], (hi-origin[axis])/dir[axis]);
        if a.min(b) > near {
            near = a.min(b);
            normal = Vec2::ZERO;
            normal[axis] = -dir[axis].signum() as f32;
        }
        far = far.min(a.max(b));
        if near > far { return None; }
    }
    // A ray on a face can leave or move along that face without a new impact.
    if far <= 0.0 || far <= near.max(0.0) { return None; }
    Some((near.max(0.0), normal))
}

struct WideSurfaceHit {
    distance: f64,
    normal: Vec2,
    segment: Option<(Vec2, Vec2)>,
}

fn raycast_surfaces_wide<W: SolidWorldQuery + ?Sized>(
    world: &W, origin: bevy_math::DVec2, dir: bevy_math::DVec2, reach: f64,
    include_one_way: bool, include_lines: bool,
) -> Option<WideSurfaceHit> {
    if !origin.is_finite() || !dir.is_finite() || dir == bevy_math::DVec2::ZERO || !reach.is_finite() || reach < 0.0 {
        return None;
    }
    let mut best: Option<WideSurfaceHit> = None;
    let mut take = |hit: WideSurfaceHit| {
        if hit.distance < 0.0 || hit.distance > reach { return; }
        if best.as_ref().is_none_or(|other| hit.distance.total_cmp(&other.distance)
            .then(hit.normal.x.total_cmp(&other.normal.x)).then(hit.normal.y.total_cmp(&other.normal.y)).is_lt()) {
            best = Some(hit);
        }
    };
    world.for_each_solid_aabb(include_one_way, &mut |bounds| {
        if let Some((distance, normal)) = ray_aabb_wide(origin, dir, bounds) {
            take(WideSurfaceHit { distance, normal, segment: None });
        }
    });
    if include_lines {
        world.for_each_line(&mut |a, b| {
            let edge = b.as_dvec2()-a.as_dvec2();
            let normal = bevy_math::DVec2::new(edge.y, -edge.x).normalize_or_zero();
            let facing = dir.dot(normal);
            if facing >= 0.0 || normal == bevy_math::DVec2::ZERO { return; }
            let distance = (a.as_dvec2()-origin).dot(normal)/facing;
            let along = (origin+dir*distance-a.as_dvec2()).dot(edge)/edge.length_squared();
            if (0.0..=1.0).contains(&along) {
                take(WideSurfaceHit { distance, normal: normal.as_vec2(), segment: Some((a,b)) });
            }
        });
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_contacts_preserves_the_discrete_overlap_case() {
        // Standing IN the target (no motion) is detected exactly as the old
        // discrete check — parity.
        let target = Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        assert!(aabb_path_contacts(
            Vec2::new(5.0, 0.0),
            Vec2::new(4.0, 4.0),
            Vec2::ZERO,
            target
        ));
        // Clear of it, not moving -> no contact.
        assert!(!aabb_path_contacts(
            Vec2::new(100.0, 0.0),
            Vec2::new(4.0, 4.0),
            Vec2::ZERO,
            target
        ));
    }

    #[test]
    fn path_contacts_catches_a_tunnel_the_discrete_check_would_miss() {
        // A thin target; a fast body leaps from one side to the far side in one
        // frame, ending CLEAR of it. Discrete endpoint overlap = miss; the swept
        // path crosses it = hit (the sweep law, CC2).
        let spike = Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(2.0, 20.0));
        let half = Vec2::new(6.0, 6.0);
        let end = Vec2::new(60.0, 0.0);
        let delta = Vec2::new(120.0, 0.0); // started at x = -60, ended at x = 60
                                           // endpoint is clear of the spike...
        assert!(!Aabb::new(end, half).strict_intersects(spike));
        // ...but the path tunneled through it.
        assert!(aabb_path_contacts(end, half, delta, spike));
    }

    #[test]
    fn path_contacts_ignores_a_near_miss() {
        // A fast body passing ABOVE the target (never crossing it) is not a hit.
        let target = Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(2.0, 4.0));
        let half = Vec2::new(6.0, 6.0);
        let end = Vec2::new(60.0, 100.0);
        let delta = Vec2::new(120.0, 0.0); // horizontal run, far above the target
        assert!(!aabb_path_contacts(end, half, delta, target));
    }
}
