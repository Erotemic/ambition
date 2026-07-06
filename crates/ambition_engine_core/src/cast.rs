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
//! Deliberately NOT yet absorbed (single-owner in their correctly-layered homes;
//! their move into `cast` is tracked, not lost):
//! - the **swept-circle** primitive (`first_circle_hit`) is intimate with the
//!   momentum kernel's surface types (`SurfaceChain`, `resolve_surface`) and
//!   stays in [`crate::surface`]; extracting it without behavior change is its
//!   own slice.
//! - the **segment raycast** tier (`raycast_solids`/`ray_aabb`) lives in
//!   `ambition_platformer_primitives` (it is generic over `SolidWorldQuery`);
//!   the **portal-aware** cast (`raycast_through_portals`) lives in
//!   `ambition_portal` and depends on portal geometry — its absorption is the
//!   `PortalFrame` slice (CC5), which introduces the engine-level aperture type
//!   this module would need to own a portal-aware cast without inverting layers.
//!
//! CC2 (the trigger-sweep audit) converts discrete path-dependent readers to
//! call THESE entry points; new unswept readers are then a flagged review
//! pattern.

use crate::geometry::Aabb;
use crate::world::{Block, SweepHit, World};
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
