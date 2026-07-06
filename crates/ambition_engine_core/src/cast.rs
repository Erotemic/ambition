//! `cast` — the swept-primitive library (collision-and-ccd.md §2, CC1).
//!
//! THE SWEEP LAW: anything that changes state as a function of a body's path
//! evaluates against the continuous swept path `pos → pos + vel·dt`, never
//! sampled endpoints. To make that enforceable, the primitive queries every
//! kernel and every trigger reader calls live behind ONE module — this one —
//! so no system rolls its own overlap/step check (the disease behind every
//! historical OOB/tunneling bug).
//!
//! What lives here / is surfaced here:
//! - **Swept AABB vs AABB** — [`AabbExt::sweep_hit`] (Parry-backed), the base
//!   solid-contact primitive both movement kernels share.
//! - **Swept AABB vs the composed world** — [`body_sweep`], the earliest
//!   predicate-filtered block hit for a body moving by `delta`. THE body-vs-world
//!   entry (player movement solids, blink blockers, one-way landing tests, spawn
//!   blockers, enemy collision all ask it their own question via the predicate).
//!
//! - **Segment ray vs the solid world** — [`raycast_solids`] over the narrow
//!   [`SolidWorldQuery`] seam, and the underlying [`ray_aabb`] slab query
//!   (moved down from `ambition_platformer_primitives` per the CC1 ruling —
//!   collision-and-ccd.md §3.4(b)).
//!
//! Deliberately NOT absorbed (ruled, collision-and-ccd.md §3.4(a)): the
//! **swept-circle** primitive (`first_circle_hit`) is load-bearing interior of
//! the momentum kernel (`SurfaceChain` / `resolve_surface` intimacy, on the
//! no-pushout/OOB path) and stays kernel-private in [`crate::surface`]; a
//! public swept-circle query is minted here only when a consumer outside the
//! kernel lands. The **portal-aware** cast rides CC5's aperture vocabulary
//! ([`crate::frame`]).
//!
//! CC2 (the trigger-sweep audit) converts discrete path-dependent readers to
//! call THESE entry points; new unswept readers are then a flagged review
//! pattern.

use crate::frame::{self, MapConvention, PortalAperture};
use crate::geometry::Aabb;
use crate::world::{Block, BlockKind, SweepHit, World};
use crate::Vec2;

// The swept-AABB primitive + its hit record — re-exported so `cast` is the ONE
// name a caller reaches for. `AabbExt::sweep_hit(delta, other)` is the
// Parry-backed base both kernels build on.
pub use crate::geometry::{AabbExt, AabbSweepHit};
// The body-vs-world hit record ([`body_sweep`]'s return).
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
/// `ambition_platformer_primitives::world_query`, CC1 ruling §3.4(b); the
/// canonical `impl` for [`World`] lives right below, beside the type.)
pub trait SolidWorldQuery {
    /// Invoke `visit` once for each solid AABB the ray should test.
    ///
    /// When `include_one_way` is true, one-way platforms are visited too;
    /// otherwise they are skipped (blink/dive/grapple pass through them, while
    /// portal placement adheres to them).
    fn for_each_solid_aabb(&self, include_one_way: bool, visit: &mut dyn FnMut(Aabb));
}

/// The engine_core world's solid-block policy: `Solid` and `BlinkWall` blocks
/// are always hittable; one-way platforms only when `include_one_way` is set
/// (portal placement adheres to them; blink/dive/grapple pass through).
impl SolidWorldQuery for World {
    fn for_each_solid_aabb(&self, include_one_way: bool, visit: &mut dyn FnMut(Aabb)) {
        for block in &self.blocks {
            let hittable = matches!(block.kind, BlockKind::Solid | BlockKind::BlinkWall { .. })
                || (include_one_way && matches!(block.kind, BlockKind::OneWay));
            if hittable {
                visit(block.aabb);
            }
        }
    }
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
    let dir = dir.normalize_or_zero();
    if dir == Vec2::ZERO {
        return None;
    }
    let mut best_t = max_dist;
    let mut best_normal = Vec2::ZERO;
    world.for_each_solid_aabb(include_one_way, &mut |aabb| {
        if let Some((t, n)) = ray_aabb(origin, dir, aabb) {
            if t < best_t {
                best_t = t;
                best_normal = n;
            }
        }
    });
    if best_normal == Vec2::ZERO {
        None
    } else {
        Some((origin + dir * best_t, best_normal))
    }
}

/// Recursive, portal-aware raycast — THE portal-aware cast family entry
/// (collision-and-ccd.md §3.4(c)/§3.5, landed with CC5). Cast from `origin`
/// along `dir`; if the ray crosses an aperture's PLANE within its opening,
/// entering from the front (`dir · normal < 0`), before any solid hit, it
/// re-anchors at the mapped point on the exit and continues along the mapped
/// direction. Bounded by `max_depth` so two facing apertures can't loop
/// forever; ONE `max_dist` budget decrements across hops.
///
/// Pinned semantics (§3.5): the aperture is a SEGMENT on its plane — the
/// crossing point must lie within `half_length` along the tangent; a ray
/// crossing the plane outside the opening ignores the aperture and hits
/// whatever is behind. The returned `(hit, normal)` is in the FINAL chart.
///
/// This is pure aperture GEOMETRY: pairs come from the caller
/// (`ambition_portal` supplies them from its `PlacedPortal`s and keeps the
/// gameplay — channels, tuning, the game-wide convention flag).
pub fn ray_through_apertures<W: SolidWorldQuery + ?Sized>(
    world: &W,
    pairs: &[(PortalAperture, PortalAperture)],
    origin: Vec2,
    dir: Vec2,
    max_dist: f32,
    include_one_way: bool,
    max_depth: u32,
    convention: MapConvention,
) -> Option<(Vec2, Vec2)> {
    let mut origin = origin;
    let mut dir = dir.normalize_or_zero();
    if dir == Vec2::ZERO {
        return None;
    }
    let mut budget = max_dist;
    for _ in 0..=max_depth {
        let solid = raycast_solids(world, origin, dir, budget, include_one_way);
        let solid_t = solid
            .map(|(hit, _)| (hit - origin).length())
            .unwrap_or(f32::INFINITY);
        // Nearest aperture plane the ray ENTERS (front side, within the
        // opening) before that solid.
        let mut nearest: Option<(f32, &PortalAperture, &PortalAperture)> = None;
        for (enter, exit) in pairs {
            let denom = dir.dot(enter.frame.normal);
            // Only enter through the front of the face (moving into it).
            if denom >= 0.0 {
                continue;
            }
            // TIE-BREAK: an aperture flush on a host face crosses at exactly
            // the solid's t — the aperture wins the tie (`>` not `>=`), else
            // every wall-mounted portal would be occluded by its own host.
            let t = (enter.frame.origin - origin).dot(enter.frame.normal) / denom;
            if t < 0.0 || t > budget || t > solid_t {
                continue;
            }
            let at = origin + dir * t;
            if (at - enter.frame.origin).dot(enter.frame.tangent()).abs() > enter.half_length {
                continue;
            }
            if nearest.map_or(true, |(bt, _, _)| t < bt) {
                nearest = Some((t, enter, exit));
            }
        }
        match nearest {
            Some((t, enter, exit)) => {
                let entry = origin + dir * t;
                // Emerge just out of the exit face, redirected through the pair.
                origin = frame::map_point(&enter.frame, &exit.frame, convention, entry)
                    + exit.frame.normal;
                dir =
                    frame::map_vec_between(dir, enter.frame.normal, exit.frame.normal, convention)
                        .normalize_or_zero();
                budget -= t;
                if budget <= 0.0 || dir == Vec2::ZERO {
                    return None;
                }
            }
            None => return solid,
        }
    }
    None
}

/// Ray-vs-AABB slab query. Returns `(t_near, face_normal)` for a forward hit
/// (`t >= 0`).
pub fn ray_aabb(origin: Vec2, dir: Vec2, aabb: Aabb) -> Option<(f32, Vec2)> {
    // 1/0 -> +/-inf is the intended slab-method behavior for axis-parallel rays.
    let inv = Vec2::new(1.0 / dir.x, 1.0 / dir.y);
    let tx1 = (aabb.min.x - origin.x) * inv.x;
    let tx2 = (aabb.max.x - origin.x) * inv.x;
    let ty1 = (aabb.min.y - origin.y) * inv.y;
    let ty2 = (aabb.max.y - origin.y) * inv.y;
    let tminx = tx1.min(tx2);
    let tmaxx = tx1.max(tx2);
    let tminy = ty1.min(ty2);
    let tmaxy = ty1.max(ty2);
    let t_near = tminx.max(tminy);
    let t_far = tmaxx.min(tmaxy);
    if t_near > t_far || t_far < 0.0 {
        return None;
    }
    // The axis that produced t_near is the face we hit; its normal opposes
    // the ray's travel on that axis.
    let normal = if tminx > tminy {
        Vec2::new(-dir.x.signum(), 0.0)
    } else {
        Vec2::new(0.0, -dir.y.signum())
    };
    Some((t_near.max(0.0), normal))
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
    fn ray_through_apertures_continues_through_a_pair() {
        use crate::frame::PortalFrame;
        use crate::world::{Block, BlockKind};
        // A solid wall at x=200; a wall aperture on its face redirects the ray
        // to a floor aperture at (500, 300), so the ray continues DOWNWARD-free
        // space and reports no solid hit within budget.
        let mut world = World::new("test", Vec2::new(2000.0, 2000.0), Vec2::ZERO, vec![]);
        world.blocks.push(Block {
            id: crate::geo_id::GeoId::anon(),
            name: "wall".into(),
            aabb: Aabb::new(Vec2::new(210.0, 0.0), Vec2::new(10.0, 200.0)),
            kind: BlockKind::Solid,
            velocity: Vec2::ZERO,
        });
        let enter = PortalAperture {
            frame: PortalFrame::fixed(Vec2::new(200.0, 0.0), Vec2::new(-1.0, 0.0)),
            half_length: 46.0,
        };
        let exit = PortalAperture {
            frame: PortalFrame::fixed(Vec2::new(500.0, 300.0), Vec2::new(0.0, -1.0)),
            half_length: 46.0,
        };
        let pairs = [(enter, exit)];
        // Straight at the wall through the aperture: without the aperture the
        // ray hits the solid; with it, it emerges from the floor and flies on.
        let without = ray_through_apertures(
            &world,
            &[],
            Vec2::new(0.0, 0.0),
            Vec2::X,
            1000.0,
            false,
            4,
            MapConvention::Reflection,
        );
        assert!(without.is_some(), "the bare ray must hit the wall");
        let with = ray_through_apertures(
            &world,
            &pairs,
            Vec2::new(0.0, 0.0),
            Vec2::X,
            1000.0,
            false,
            4,
            MapConvention::Reflection,
        );
        assert!(
            with.is_none(),
            "through the aperture the ray reaches open space, got {with:?}"
        );
        // Crossing the PLANE outside the opening ignores the aperture: the ray
        // behaves exactly as if no aperture existed (hits the wall).
        let bare = ray_through_apertures(
            &world,
            &[],
            Vec2::new(0.0, 120.0),
            Vec2::X,
            1000.0,
            false,
            4,
            MapConvention::Reflection,
        );
        let outside = ray_through_apertures(
            &world,
            &pairs,
            Vec2::new(0.0, 120.0),
            Vec2::X,
            1000.0,
            false,
            4,
            MapConvention::Reflection,
        );
        assert!(bare.is_some());
        assert_eq!(
            outside, bare,
            "outside the opening the aperture must not divert the ray"
        );
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
