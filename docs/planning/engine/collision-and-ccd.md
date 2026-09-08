# Collision and continuous contact - remaining work

The implemented movement contract remains in
[movement and collision](../../concepts/movement-collision.md). This page tracks
current contact/geometry work, not the completed CC1-CC8 campaign. The architecture
review adds packet A2 in the [frontier](actor-monolith-work-frontier.md) and
findings F2/F3 in [the source findings](architecture-review-findings.md).

## Established foundation to preserve

`ambition_platformer2d_core::cast::aabb_path_contacts` provides swept trigger
queries. Movement publishes the tick's canonical `SweepSample`; room and loading
zone entry consume the path. Water and climb regions intentionally retain
discrete enter/exit reads with `thin_region_warnings` for tunnelable authoring.
Ledge probing follows resolved wall contact. Manual pickup remains button-gated
overlap; a future automatic collector needs an explicit swept-trigger policy.

`GeoId`/`GeoFaceRef`, portal frame/aperture vocabulary and moving-host mapping are
existing infrastructure. The collision-invariant oracle remains an on-demand
room/seed/tick diagnostic. Do not replace these mechanisms merely to unify names.

Hazard contact already reads the current path, and the wrapper writes its sample
before invoking the gate. `SimPhaseReach::Completed` preserves the original gate
population; sample writing still occurs on zero-dt and early-return paths so a
stale path is not reused. Missing samples retain the existing endpoint-only
compatibility arm. Moving the gate without preserving those populations would
reopen a resolved defect. Source and regression home:
`crates/ambition_platformer2d_core/src/movement/tests/hazard_sweep.rs`.

## A2: one contact decision through selection and reaction

The [projectile contact protocol](projectile-contact-protocol.md) is the detailed
A2 owner. It specifies the source edits, response matrix and acceptance cases;
this page owns reusable geometry primitives and the broader CCD program.

A2a establishes one authored/fallback/empty target geometry. A2b sweeps the actual
finite projectile through its actual travel legs and orders world/target contact
under one collision policy. A solid destructible's own collider and hurt target
can produce one compound contact, so world contributors need stable identity.
A2c preserves immediate interception while delivering later damage to exactly the
selected identity, without repeating broad feature queries.

The first implementation uses targets stationary at the declared simulation
sample. Swept projectile versus sampled target solves endpoint tunneling for that
model; it does not solve target-relative motion, rotation or arbitrary deforming
silhouettes. Those require separate measured/behavioral cases in this program.
Broad-phase bounds cannot replace a supported exact combat shape in narrow phase.

Retain the existing projectile processing order, returning-shot hit memory and
explicit splash multiplicity. Reflection/absorption ends the current step even
before deferred despawn is applied. Keep portal/teleport discontinuities out of
the swept path. See the protocol's fixtures for equal-time ties, one-ways,
compound solids, first-frame geometry, stale recipients and resimulation.

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
