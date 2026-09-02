# Collision & CCD — remaining work

> **State:** residual work only, verified against `cecd01ca` on 2026-08-13.
>
> The original CC1–CC8 design/campaign record is archived at
> [`../../archive/planning-superseded/2026-08-13/engine/collision-and-ccd.md`](../../archive/planning-superseded/2026-08-13/engine/collision-and-ccd.md).
> Settled runtime doctrine lives in
> [`../../concepts/movement-collision.md`](../../concepts/movement-collision.md).

The core collision migration has landed. Do not replay the old campaign. This
page names only collision/CCD work that current source still leaves open or
explicitly trigger-gates for a future capability.

## Verified current foundation

Current source already provides the contracts the old campaign was trying to
establish:

- `ambition_platformer2d_core::cast::aabb_path_contacts` is the shared swept
  trigger primitive;
- the movement kernel publishes the canonical per-tick `SweepSample`;
- room/loading-zone entry consumes the swept path rather than endpoint overlap;
- water and climb regions intentionally remain discrete ENTER/EXIT state reads,
  with `thin_region_warnings` covering tunnelable authoring mistakes;
- ledge probing is downstream of resolved swept wall contact and is deliberately
  discrete;
- manual ground-item pickup is button-gated while overlapping; a future
  auto-collect item must use the swept trigger primitive;
- `GeoId` / `GeoFaceRef` provide durable geometry identity;
- `PortalFrame`, `PortalAperture`, explicit mapping convention, and moving-host
  portal velocity/frame handling are implemented;
- the collision-invariant oracle exists as an on-demand diagnostic with stable
  room/seed/tick evidence.

Those are implementation facts now, not planning tasks.

## 1. Make hazard contact consume the canonical swept path

✔ **CLOSED.** The shared gate reads the tick's `SweepSample` (`prev -> curr`)
through `hazard_contact_on_path`, so a body fast enough to step over a thin
hazard is hit. A body with no sample keeps the endpoint test and nothing else —
the one compatibility arm, chosen so no second motion model reconstructs a
segment from `vel * dt` and disagrees with the kernel; `SweepSample`'s
`TODO(compat-remove)` is the plan to delete it. Teleports needed no exclusion:
the sample spans simulation-phase entry to exit, so a blink or room transfer is
not inside the segment by construction. Guarded by
`movement::tests::hazard_sweep` (5 cases, every motion policy), poison-verified
in both halves.

⛔ **The axis-swept arm ran the gate one tick stale, and that was the harder
half.** `apply_world_hazard_gate` sat at the end of `update_body_simulation_
inner` while that policy writes its `SweepSample` in the wrapper *after* the
inner step returns — so the gate would have read the PREVIOUS tick's segment,
and a zero-length default on the first tick. The other two policies already
wrote their sample immediately before calling the gate. The gate now runs in the
wrapper, after the write, and all three arms share the shape *write sample ->
gate*. A reader that consumes a per-tick record must be ordered against the
writer of that record, not merely placed in the same function.

⛔⛔ **AND MOVING IT CHANGED WHO IT JUDGES, WHICH IS A SECOND DECISION.** The
inner step has three early returns — a `raw_dt <= 0.0` tick, a drowning, and a
frame an active ledge grab consumed — and none reached the gate while it sat in
the tail. Lifting it to the caller silently added all three: a frozen frame would
judge a body nothing had stepped, and a body HANGING on a ledge whose box
overlaps a hazard would start dying, which matters because spikes under a lip is
an authored shape. `SimPhaseReach` now reports whether the phase reached its
tail and the gate runs only on `Completed`, so the population is exactly what it
was. ⚠ The SAMPLE WRITE is deliberately not gated the same way — it must run on
every path, because a zero-dt tick has to record a zero-length segment rather
than keep a stale one. Guarded by `a_hanging_body_is_not_judged_by_the_hazard_
gate`, which proves the same body in the same spikes DOES die when nothing
consumes its frame.

⇒ **Relocating a call relocates its population.** The ordering was the bug; the
population was not, and one edit changed both.

## 2. Turn stable collision-oracle findings into behavioral regressions

The full `collision_invariant_oracle` remains intentionally diagnostic and
ignored because authored open edges and known deferred cases make a single
"all rooms must be clean" assertion too coarse.

When the diagnostic finds a reproducible engine defect:

- capture the smallest room/seed/mechanic reproduction;
- encode the actual illegal behavior as a focused behavioral regression;
- fix the owning collision/movement path;
- leave legitimate authored exits and open gaps legal.

Do **not** make a permanent global hard gate merely to satisfy the old CC3
campaign. Promote individual invariants only when their legal/illegal boundary
is stable enough to express behaviorally without false positives.

## 3. Broadphase only after measurement shows it matters

The old CC4 broadphase idea remains a performance trigger, not active work.
Measure cast/collision cost first. Introduce a spatial index only when a real
profile demonstrates that linear block/chain traversal is material at a target
content scale. Preserve deterministic ordering and exact contact semantics.

## 4. Non-axis-aligned geometry and dynamic straddling are demand-driven

Angled portal/surface geometry, generalized straddle pieces, and dynamic
straddle support remain legitimate future capabilities, but no current customer
requires the full old CC7 program. Start that work only when a concrete room or
game feature needs it, then make the geometry vocabulary reusable rather than
portal-specific.

## 5. Slopes are a customer-triggered capability

AABB slope vocabulary from the old CC8 plan remains deferred until a real game
or demo demands slopes. Do not build it merely to finish the historical
campaign.

## 6. The swept primitive's parity claim is false for touch contacts

**Open, source-confirmed and measured once.** `cast::aabb_path_contacts`
documents itself as *"PARITY by construction: it returns `true` for the
already-overlapping (standing-in-it) case exactly as the old discrete
`strict_intersects` did, then ADDS the swept path on top"*. The first half is
true. The second is not: the discrete test reports boxes that **overlap**, while
`sweep_hit` reports boxes that **contact**, so the sweep adds every TOUCH along
the path — including a body sliding along a face it never enters.
`reject_grazing_contact` does not cover it, because the approach is head-on on
one axis while the other axis merely shares a plane.

⭐ **MEASURED 2026-09-02 in `blink_run`.** A body walking the start floor with
feet at `y = 128.0`, against a hazard whose top face is also `y = 128.0`, was
reported as hitting the hazard the instant its leading edge passed `x = 288.0`
(endpoint AABB `max = (288.87, 128.0)` vs hazard `min = (288.0, 128.0)`). It
walked along a surface and was killed for it. This is not an exotic shape: an
authored death gap ALWAYS shares its top face with the floor it interrupts.

The hazard gate fixed this **on the consumer side**, insetting the hazard to its
interior before the swept query (`movement::collision::HAZARD_SURFACE_EPSILON`),
and deliberately did not change the primitive on the strength of one case.
Guarded by `movement::tests::hazard_sweep::walking_a_floor_flush_with_a_hazards_
top_face_is_not_a_hit`, across every motion policy, poison-verified.

⚠ **THE OTHER CONSUMERS ARE OVER-TRIGGER LATENT, UNMEASURED.** Room and
loading-zone entry both read the same primitive (see "Verified current
foundation"). Neither has been shown to misfire, and neither has been checked —
the reason they may not is that a loading zone is rarely coplanar with a
walkable surface, which is a property of current authoring rather than a rule.
⛔ Do not "fix" them speculatively. The open question is whether the parity claim
should be corrected at the primitive — a strict-overlap variant beside the
contact one — or whether contact is right for triggers and only hazards want
overlap. Answer that with a second measured case, not by reasoning.

## Exit

This residual plan is complete when the source-confirmed swept-hazard gap is
closed and any additional collision work has either acquired a real customer or
is represented as an explicit trigger rather than an evergreen migration task.
