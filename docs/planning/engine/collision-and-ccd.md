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

There are three distinct questions: what geometry the target publishes, which
contact a moving projectile reaches first, and what the victim does with an
accepted contact. Keep them explicit rather than creating a universal collision
service or feature registry.

The current projectile road can use feature-family boss/breakable predicates
before its world sweep, and boss damage can fall back to geometry that differs
from published authored hurtboxes. See the exact evidence and confidence limits
in F2/F3; these are source findings, not executed gameplay results from this
review.

### Stage 1 - target geometry

Make projectile boss admission and downstream reaction honor the existing
published simulation target geometry. Distinguish absent authored geometry from
present-empty authored geometry. Empty must not accidentally become coarse-body
fallback. Do not use sprite pixels or device quality as mechanical authority.
Keep damageability separate from collision: invulnerable, blocking and deflecting
contacts may have different consumption/reaction policies.

### Stage 2 - travel and obstruction

Capture or consume the actual canonical movement leg(s). A reconstruction from
end position minus a changed velocity is not automatically that leg. Compare
world and target candidates in the same coordinate frame using the relevant
projectile shape and collision policy. A center ray can disagree with a finite
projectile sweep, one-way policy or a portal-transformed segment.

Specify nearest-hit order, equal-time tie policy and stable identity order. A
world blocker before a target must not be bypassed by evaluating a feature family
first. Continuous body-target tests must cover high-speed passage with no endpoint
overlap. Preserve owner/team exclusions, intentional pierce/bounce/deflect rules,
lifetime, target deduplication and authored muzzle placement.

### Stage 3 - accepted contact handoff

Send the selected victim and contact fact through the existing deterministic hit
road; do not ask a later `UnresolvedFeatures` arm to reclassify the target by
family string. The victim owner applies damage/destruction/knockback policy;
rules own score/stocks; presentation consumes a published consequence.

Prefer a small typed handoff or direct dependency with one real producer and
consumer. Do not create a global query bus, executable target registry or
`BreakableLike`/`BossLike` marker vocabulary simply to delete an import.

### Acceptance

Use production-path fixtures for displaced authored boss hurtboxes,
present-empty hurtboxes, absent fallback, target before wall, target behind wall,
finite-shape corner collision, high-speed body crossing, one-way behavior,
equal-time contacts, same-tick multiple projectiles and rollback replay. Include
portal/moving-world cases only through the supported travel contract; explicitly
classify unsupported cases rather than inventing an alternate transform road.

A2's commits first characterize geometry, then repair ordering, then remove the
obsolete dispatch. Each keeps behavior changes separate from code movement.

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
