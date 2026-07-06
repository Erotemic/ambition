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
