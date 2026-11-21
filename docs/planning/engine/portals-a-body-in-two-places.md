# Portals: one material domain, several spatial images

**State:** implemented material model with open consumer corrections. Reviewed
2026-11-21 at `b91b29d359358ea417353afce4f472dc7fab15ec`.
The [portal delivery plan](portal-parts-similarity-and-chirality.md) selects work.
This page owns the geometric meaning. It replaces the historical G1–G6 case
file; those commits remain in Git history.

## The rule

A portal changes where material is. It does not create another gameplay owner.
A body, a limb or an attack keeps its existing health, custody, controller and
logical identity while its material can occupy several regions.

For one canonical primitive `D`, a spatial image consists of a source subset
`D_i`, a similarity map `T_i`, and its ordered portal itinerary. At one sample:

- The source subsets cover `D`; their interiors do not overlap.
- Each image has one map and the cuts that select its source subset.
- An eligible finite aperture cuts the material again. A supporting infinite
  line outside that aperture does not teleport anything.
- Different images may overlap in world coordinates. That is not duplicate
  canonical material and does not create duplicate damage authority.
- A cyclic itinerary can visit an end again with a different map and subset.
  Endpoint visitation alone cannot terminate the partition or its broad phase.

`frame::thread::VolumePiece` already carries canonical source bounds, `Sim2`
and path data. `MaterialCastCells` extends the representation over a recorded
time interval. Preserve exact circular intersections; a polygon used to draw
a circle does not replace its collision primitive.

## Motion, reach and visibility ask different questions

These consumers share maps, finite apertures and clipping operations. Their
eligibility rules remain explicit:

| Query | Source and selection rule |
| --- | --- |
| Body material | A finite body passes through a finite opening; its exact material and contact law decide whether it fits |
| Owned reach | A strike or extension reaches from its owner through the wedge formed by that point and the aperture ends |
| Ray or finite beam | Remaining reach is measured in the initial chart; each leg spends world distance divided by cumulative scale |
| Visibility | A selected eye/facing governs visibility; a complete presented camera chart governs projection, and the two origins can differ |
| Discrete arrival | A validated new pose has no continuous path through the space between departure and arrival |

`partition` and `partition_reaching` implement the first two material cuts.
A swing's root can be its hand or owner. A visibility eye can follow a body
while the displayed camera eases or clamps elsewhere. A shared
clipping primitive does not make these the same simulation operation.
Do not infer passage permission from visibility or turn every teleport into
a swept path.

The ray trace already preserves this metric and starts each leg on the exact
exit face. Keep bounded termination and deterministic first-hit selection;
an arbitrary world-unit exit advance cannot skip a close target or change
authored reach. This is implemented behavior to retain during query linking.

A body that is too wide cannot pass by dropping its beside pieces. The
current AABB-based snag is not the final fit authority for rotated material.
The delivery plan replaces it with canonical aperture-rim contact in the
accepted motion operation.

## One geometry authority does not mean one shape for all uses

A body collision box, an authored hurt region, a sword strike, a sprite and
a glow can have different source domains. They can share a material chart,
owner and portal relation while retaining their own geometry and extent.

Share the exact source when two readers mean the same primitive. Otherwise
share its map and the applicable cut rules, then project each source domain.
A broad-phase AABB remains a candidate bound. It cannot become the precise
body, attack or image merely because every reader can consume a rectangle.

This distinction is required for long art, offset attachments, asymmetric
poses, independent limbs and particles without physics. A renderer must not
clip a sword to its owner's torso collision box. A larger glow must not make
the actor wider for passage admission.

## Retain chronology as well as occupancy

A partition at one instant answers where the material is now. It does not
answer which path it travelled, when a portal existed, or which motion was
accepted. A completed root rebase must not replace the current tick's path
with a stationary final pose.

The [moving-frame contract](portals-are-moving-frames.md) owns accepted motion
intervals, chart changes and contact kinematics. Queries retain the topology
and pose sample that gave their path meaning. A later topology edit does not
reinterpret a completed motion interval through today's endpoints.

An itinerary's dense link numbers are local to that selected snapshot.
Longer-lived receipts use the [pair/end occurrence](portal-pair-identity-and-aperture.md).
A picture, a mutable endpoint and a path index are three different identities.

## Refusal preserves a valid operation

`PieceBudget` bounds hops, parts and scale. Numerical work also has a finite
limit. A query returns complete admitted geometry or a typed refusal; it
must not emit an uncut whole shape as successful portal geometry.

A refused owned reach may use the existing explicitly clipped near-side
remainder when its owner permits that result. That is a refused continuation,
not permission to hit through the failed path. Body motion instead commits a
valid candidate under its accepted restrictions. A later transit pass cannot
reopen the refused route.

The applicable response belongs to the operation owner. A rendering budget
can reduce detail or use an admitted proxy; it cannot change body collision,
portal activation or authoritative material limits.

## Presentation follows material without creating another body

The general carry road already handles ordinary top-level `Sprite`, `Mesh2d`
and `Text2d` families. Children and `PortalDrawnWith` members stay with their
family. Preserve this structural coverage when reducing capture work.
A new gameplay drawable must not need a new portal-specific opt-in branch.

Independent families keep independent roots, cut domains and depth. A common
capture target does not authorize flattening unrelated bodies into one image.
A limb can carry part of a host picture without making the whole host that
limb's physical body. `PortalHostReach` is a constrained presentation relation.

A ready source image can supply texture coordinates directly to the same
clipped geometry when it already contains the complete intended family.
Other families retain the capture path. The [view and capture plan](portals-one-transit-for-every-mover.md)
owns this specialization and its proof of equivalent image/depth semantics.

An open live window updates every presented frame. Current window tiers bound
their count and retain view pixel density. Carry targets have a separate
resolution and proxy budget. A window must not silently show old geometry
while the current carried body crosses its seam. A picture may be reused only
when the facts it depicts are unchanged. A missing ready picture has an
explicit presentation fallback and changes no simulation state.

## Established work to preserve

- General material partitions, typed refusal, scale and persistent parity are
  implemented. The playable 80/40 pair is recorded at `fddf6084a`.
- Simultaneous and chained limb contact is recorded at `2c4bd3822`, with
  chained damage/replay at `99d51bc9f` and two active fists at `6ae7a653a`.
- Universal drawable carry was introduced in `63797b0ac`; independent
  captures and depth were repaired in `ae8f2b07a`.
- Adaptive carry storage and explicit proxy overflow were repaired in
  `1d551594b`. Current retained pools are not missing infrastructure.

These are bounded historical receipts, not new verification on this review.
Root timing, canonical rim contact, snapshot-bound queries and view identity
remain open under the current delivery plan.

## Source map

- [Canonical partitions](../../../crates/ambition_platformer2d_core/src/frame/thread.rs).
- [Temporal material queries](../../../crates/ambition_platformer2d_core/src/frame/material_cast.rs).
- [Body material and room query adapter](../../../crates/ambition_platformer2d_shared_tangle/src/body/geometry.rs).
- [Drawable carry](../../../crates/ambition_portal2d_presentation/src/carry.rs).
- [Retained clip geometry](../../../crates/ambition_portal2d_presentation/src/clip_material.rs).
