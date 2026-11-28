# Collision and continuous contact - remaining work

**Scope:** reusable geometry primitives and the remaining continuous-contact
work. The implemented movement contract is in
[movement and collision](../../concepts/movement-collision.md). Projectile
contact is packet A2 in the [packet catalog](actor-monolith-work-frontier.md).

## Established foundation to preserve

`ambition_platformer2d_core::cast::aabb_path_contacts` provides swept trigger
queries. Movement publishes the tick's canonical `SweepSample`; room and loading
zone entry consume the path. Water and climb regions intentionally retain
discrete enter/exit reads with `thin_region_warnings` for tunnelable authoring.
Ledge probing follows resolved wall contact. Manual pickup remains button-gated
overlap; a future automatic collector needs an explicit swept-trigger policy.

[Present material occupancy](world-geometry-and-spatial-semantics.md#present-material-occupancy)
corrects the occupied shape at the existing water/climb read cut. A clipped
root box can produce false presence or miss a real far-side part. This is
separate from tunnelling between samples. Keep discrete timing and strict
interior overlap; a face-contact predicate is not equivalent. Region response
and source-bound climb passability remain with their native owners.

`GeoId`/`GeoFaceRef`, portal frame/aperture vocabulary and moving-host mapping are
existing infrastructure. The collision-invariant oracle remains an on-demand
room/seed/tick diagnostic. Do not replace these mechanisms merely to unify names.

Hazard contact already reads the current path, and the wrapper writes its sample
before invoking the gate. The gate runs on `SimPhaseReach::Completed` and
`LedgeHeld`: a hazard wins over a ledge hang (Q43), and only a hanging body
gives up its ledge intangibility (`BodyMotionFacts::evading_hazards`). Sample writing still occurs on zero-dt and early-return paths so a
stale path is not reused. A missing sample keeps the endpoint-only arm permanently: the fighter
brain's recovery planner runs the kernel over a scratch body that is not an
entity. No ECS reader rebuilds a segment from `vel * dt`; that is a second motion
model. Preserve these populations when you move the gate. Source and regression home:
`crates/ambition_platformer2d_core/src/movement/tests/hazard_sweep.rs`.

**One solidity mechanism (Q102).** `collision_semantics::is_full_collision_surface`
(with `is_support_surface`) names the kinds with full collision: `Solid`,
`BlinkWall` and `Barrier`. A solid breakable publishes `BlockKind::Barrier`,
which no blink upgrade passes
(`the_hard_blink_upgrade_does_not_pass_an_unbroken_solid_breakable`). A moving
platform stays `BlinkWall { Soft }`: the soft upgrade passes it by design, and
no reader treats it differently from a solid
(`the_soft_blink_upgrade_passes_a_moving_platform`). Do not make a solid object
a blink wall to borrow full collision.

## Environmental profile changes (ECP, proposed 2026-11-27)

[ECP](environmental-collision-profiles.md) proposes selectable rectangular,
ECB and paired-support configurations. Keep the native kernel and the
existing contact/overlap and portal contracts. Distinguish directional
contacts, actual occupied material, combat geometry and broad-phase bounds.
A conservative AABB is useful to find candidates, not to prove a new
profile fits or hits. An ECB is not permission to omit relevant corner and
side contact tests.

For shape/orientation changes, the accepted interval must describe previous
and proposed geometry as well as root motion. Stationary roots can change
occupied space and moving contact features can cross a floor. Validate
expansion, rotation, landing eligibility and clearance, including one-way
surfaces and moving material. Preserve the accepted contact chronology for
later readers; do not replace it with endpoint geometry. Reuse existing
motion samples where they express the required fact, and extend the native
representation only when a selected profile demonstrates the missing fact.
Profile-specific queries must be implemented or explicitly unsupported
before a live consumer uses them. Keep A2 and the selected portal work
independent of this candidate track.

## A2: one contact decision through selection and reaction

The [projectile contact protocol](projectile-contact-protocol.md) is the detailed
A2 owner. It specifies the source edits, response matrix and acceptance cases;
this page owns reusable geometry primitives and the broader CCD program.

A2a (landed) establishes one authored/fallback/empty target geometry. A2b sweeps
the actual finite projectile through its actual travel legs and orders
world/target contact under one collision policy; its obstruction half is landed. A solid destructible's own collider and hurt target
can produce one compound contact, so world contributors need stable identity.
A2c preserves immediate interception while delivering later damage to exactly the
selected identity, without repeating broad feature queries.

A2's first implementation used targets stationary at the declared simulation
sample. Native body consumers now have `swept_body_contact` over retained
source material; that is not proof of arbitrary deforming silhouettes or of
history surviving a root crossing. Keep each supported target-motion model
explicit. The portal correction below supplies the lost chronology case.
Broad-phase bounds cannot replace a supported exact combat shape in narrow phase.

Retain the existing projectile processing order, returning-shot hit memory and
explicit splash multiplicity. Reflection/absorption ends the current step even
before deferred despawn is applied. Keep chart transitions as explicit path
boundaries; retain real travel on either side and never cast across the
spatial jump. See the protocol's fixtures for equal-time ties, one-ways,
compound solids, first-frame geometry, stale recipients and resimulation.

## Accepted motion across portal charts

The [portal review](portal-parts-similarity-and-chirality.md) selects P2–P4
against source `b91b29d35935`. `MaterialCast` already represents source and
aperture motion. Native body contact still reconstructs relative motion from
a static mapped displacement, and root transit collapses a target's completed
path before projectile queries read it. These are concrete authority crossings,
not a request for a new collision framework.

Retain accepted continuous legs with their original clock, material and
topology selection. At a chart change, record the event and continue the
remaining interval; never sweep a straight line across the teleport gap.
Next-step initialization must not delete the completed target path. Reuse
`ContactVelocityFrame` for the point/time/path-relative approach and response.
AABB bounds select candidates; canonical material decides aperture-rim fit.

Candidate acceleration must conservatively include curved support and repeated
portal images. A link visited with one region/time/remaining allowance cannot
exclude another state without a subsumption argument. The source-derived
full-turn and repeated-link cases are correctness witnesses even before a new
performance measurement. Other broad spatial optimization remains gated by its
own customer and measured cost.

## Shared bounds-query implementations

The [shared bounds-query kernel](data-structure-kernels.md#k1--snapshot-bounds-queries)
extracts the existing layered-world grid for use by both world queries and
script contact partitions. Keep exact bounds filtering and canonical snapshot
order before a potentially refusing or limited consumer. Unknown material
bounds and deliberate global queries retain conservative fallbacks; a finite-only
tree cannot erase them. The world overlay still owns its shared base,
visibility, current/swept inputs and cache retirement. Alternative backends
follow their own workload measurements, not a broad collision rewrite.

## Contact versus strict overlap is a caller policy

The swept primitive can add touching contacts to a strict-overlap endpoint test.
That is not exact overlap parity: a body sliding along a shared face can touch
without entering the volume. Hazard handling already insets to the interior via
`HAZARD_SURFACE_EPSILON`; its floor-flush regression remains closed.

Room/loading-zone over-trigger behavior is **unmeasured** in this review. Do not
change those callers speculatively. First write their intended touch/entry contract
and a reproducible fixture. Then choose an explicit strict-interior query or
contact query if both are needed. Correct misleading parity documentation in the
same scoped source packet, not through a global collision rewrite.

## Diagnostic and deferred work

Promote a collision-oracle observation to a hard regression only after identifying
a stable illegal behavior and its smallest room/seed/mechanic witness. Legitimate
open room edges remain legal. The whole oracle is not a universal all-rooms gate.

Broadphase changes require a measured collision-cost bottleneck at an intended
content scale and must preserve deterministic tie order and contact semantics.
Slopes, non-axis-aligned geometry and generalized dynamic straddling require a
concrete game customer. No historical campaign checklist independently authorizes
them. Spatial query acceleration and geometry authoring are separate from actor
placement lowering and session lifecycle.

## Exit condition

A2 closes when admitted contacts, obstruction and victim reaction agree in the
supported profiles, the old feature-family reclassification is gone, and the
behavioral matrix passes. Other entries remain only while they have a concrete
failing fixture or an explicit customer/measurement trigger.
