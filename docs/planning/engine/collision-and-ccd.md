# Collision & CCD — sweep everything, kill the OOB class, free the geometry

**Authored by fable, 2026-07-05.** The doctrine that retires out-of-bounds
bugs as a CLASS, makes non-axis-aligned geometry first-class, and takes
portals from static wall decorations to movable physical objects. Companion:
[`spatial-model.md`](spatial-model.md) (authoring/IR),
[`frame-awareness.md`](frame-awareness.md) (the "relative to what?" review
discipline). Binding rules inherited: **no pushout** (M10; sole exception:
portal-close straddle eviction), no speed caps as bug fixes (Jon: "speedy
thing comes out" is sacred), frame-agnosticism (C4 harness for every rule).

---

## 1. The diagnosis

Every OOB/tunneling bug we have ever fixed had the same shape: **some check
sampled positions discretely** while the mover was swept, or vice versa. The
mockingbird clip (pushout-teleport), the portal high-speed embed (§7.6 of the
07-05 plan — discrete transit trigger vs. swept solids), the historical ledge
and wall-cling escapes — all instances of ONE disease: *mixed sampling
disciplines within a single frame of motion.*

The cure is not more guards. It is a single rule:

> **THE SWEEP LAW.** Anything that changes state as a function of a body's
> path — solid contact, trigger volumes, portal transit, hazard touch, blink
> destination validity, one-way admission, ledge grab, water/zone entry —
> evaluates against the CONTINUOUS swept path `pos → pos + vel·dt`, never
> against sampled endpoints. Discrete sampling is permitted only for state
> that is genuinely positional (camera zones, ambient metadata, music
> regions) where a one-frame late/early transition is imperceptible and
> harmless.

Both movement kernels already obey the law for SOLIDS (axis-swept AABB; the
momentum circle's TOI casts). The remaining offenders are the TRIGGER-shaped
readers. The §7.6 fix (swept portal transit, landed `31342e6f`) is the
template: keep the cheap discrete check as the low-speed fast path where it
is provably equivalent, add the segment/shape cast tier above it.

## 2. The unification target: one contact vocabulary, two kernels, shared casts

We deliberately keep **two movement kernels** — the axis-swept AABB kernel
(the protected classic-feel fast path) and the surface-momentum kernel
(chains/blocks, circle proxy). What unifies them is BELOW and BESIDE them:

- **Below — the cast library.** One module owns the primitive queries:
  swept AABB vs. AABB, swept circle vs. segment/AABB (parry-backed), segment
  raycast through the composited world, and **portal-aware casts**
  (`raycast_through_portals` generalized: any cast can opt into continuing
  through apertures). Both kernels and every trigger reader call THESE.
  No system rolls its own overlap/step check. (Today the pieces exist —
  `AabbExt::sweep_hit`, `first_body_sweep`, `first_circle_hit`,
  `raycast_through_portals` — scattered; the slice work is consolidation
  into `ambition_engine_core::cast` with the trigger-tier entry points.)
- **Beside — the `Contact` vocabulary.** Every contact from any kernel or
  cast reports the same `Contact { point, normal, toi, surface_velocity,
  source }` (landed 2026-07-05). Gameplay consumes contacts; it never asks
  "which kernel produced you?".

**Blocks ARE surfaces** (landed with the Sanic fix, `0189338b`): a solid
block's exterior boundary is a closed rectangular `SurfaceChain`
(`Block::boundary_chain`), swept per-face and ridden by the momentum kernel
with the same stick/joint rules as authored chains. This is the seed of the
whole section-3 story: the AABB is now formally a special case of the
polyline surface — cheap because rectangular, not because privileged.

## 3. Non-axis-aligned geometry

The end state: **a room may be built from arbitrary polyline/polygon
geometry**, with axis-aligned tiles as the fast common case.

- **S1. Chains are already the answer for actors on the momentum path.**
  `SurfaceChain` (open/closed, one-sided, validated winding) + the follower
  solver handle slopes, valleys, loops, moving surfaces. Nothing new needed.
- **S2. The AABB kernel gets a bounded slope vocabulary, not a rewrite.**
  Classic bodies (knight-likes) on gentle ramps is the deferred Q15; when it
  opens, the shape is: chains tagged `walkable_by_aabb` project a ground
  height under the body's feet within a slope-angle budget; the axis-swept
  step treats that as a moving floor plane. The AABB kernel NEVER learns
  general polygons — bodies that need loops ride the momentum kernel (that's
  what `MotionModel` per-body policy is FOR).
- **S3. Combat and triggers are already free.** `CombatVolume` supports
  OBB/convex (parry) since the CombatVolume rewire; trigger volumes follow
  the cast library, which is shape-generic. Authoring: LDtk `SurfaceChain`
  entities + generated markers (`SurfaceLoop`) exist; **`SurfaceRamp`**
  (Q27 ruling, 2026-07-06) adds the quarter-circle floor↔wall transition
  as another generated marker (radius, corner orientation, segments) —
  parameterized generator entities are how LDtk stays sufficient without a
  second backend. Add `SurfacePolygon` (closed solid region: boundary
  chain + interior solid flag → the IR emits the chain + a conservative
  AABB hull for broadphase) when a demo needs true rotated solids.
- **S4. Broadphase honesty.** Casting against every segment in a big room is
  the momentum kernel's current behavior; before Sanic-scale zones land, add
  a uniform grid/interval index over `World.blocks + chains` keyed by the
  swept path's AABB. Bounded, mechanical, measured (profile first — the
  current N is small).

## 4. Portals become physical objects

Today: portals are static wall-mounted apertures; transit is swept
(`31342e6f`); the carve (host geometry subtraction) is static per placement;
angled portals are post-1.0. The arc, in order:

- **P1. The `PortalFrame` type.** The long-flagged `FIXME(portal-api)` arc:
  a portal endpoint IS a frame (origin, tangent, normal, and now
  **velocity**). The pair transform (`map_point`, `map_velocity`) becomes a
  pure frame-to-frame map. This is the first consumer allowed to introduce a
  shared frame TYPE (AJ13 ruling honored: built for a real pressure, not
  speculatively). All existing cardinal logic re-expresses as the special
  case `tangent ∈ {±x, ±y}`; byte-parity pinned on the existing suite.
- **P2. Moving portals (translation).** A portal riding a moving platform or
  path: the carve re-cuts as the host face moves (the overlay already
  re-composes per frame; the carve keys off the host block + local offset,
  not absolute position); the swept transit trigger tests the segment
  against the aperture's SWEPT plane (relative sweep: body path minus
  portal path — one subtraction, since both are linear over the frame);
  `map_velocity` adds the frame-velocity delta so exiting a moving portal
  imparts/removes the relative motion (Galilean composition — this is
  frame-awareness made mechanical, and it is the physically correct "speedy
  thing comes out" generalization).
- **P3. Angled portals.** With P1 landed this is authoring + math, not new
  structure: apertures at arbitrary tangents; transit maps the full frame
  (position, velocity, gravity-relative orientation policy). The C4 harness
  extends to arbitrary-angle conjugation tests (transit through a θ-portal ==
  rotate, transit through cardinal, rotate back).
- **P4. Portal-carried bodies & straddle.** The partial-piece machinery
  (`portal_pieces.rs`, the Core invariant) already renders straddling
  bodies; P2 makes the STRADDLE state dynamic (the aperture moves under a
  straddling body). Rule: the piece map is re-evaluated per frame from the
  frame transform; eviction stays the ONLY pushout in the engine and only on
  CLOSE.

Execution grades: P1 [fable — design + parity cut, or opus against this spec
with the parity suite as the gate]; P2 [opus, fable-specced by P1's shapes];
P3/P4 [opus after P2, post-demo].

## 5. The OOB endgame: guarantee, not vigilance

- **The composed-world invariant test**: a headless fuzz rig (per room:
  random spawns, random high-speed impulses incl. through portals, N seconds
  stepped) asserting the standing invariant *no body center ever inside a
  solid; no body outside the world AABB without a hazard/reset event*. Runs
  in CI over every shipped room. The OOB trace tooling (debug_traces on OOB)
  stays as the diagnostic when the rig trips.
- **Delete the guards the law obsoletes.** Fixed "guard windows" sized to
  worst-case per-frame steps (`APPROACH_CARVE_REACH`, `CARVE_DEPTH`
  compensations) shrink to geometric truths once every trigger is swept;
  each deletion cites the sweeping slice that made it safe.
- **`is_contact_range_snap` discipline** (the mockingbird lesson, twice):
  landing snaps only within contact range along the contact normal — audited
  as part of the cast-library consolidation; the cast module owns the ONE
  implementation.

## 6. CC5 design sketch — `PortalFrame` (pre-solved)

Grounding: endpoints ALREADY expose `frame()` and the pair transform
already routes through `pp::map_point(entry, &enter.frame(),
&exit.frame())` + `portal_map_vec` (see `raycast_through_portals` in `ambition_portal`'s `placement.rs`) — the frame
concept half-exists in `ambition_platformer_primitives`. CC5 promotes it:

```rust
/// A portal endpoint IS a frame. World-frame fields (AJ13 naming).
pub struct PortalFrame {
    pub origin: Vec2,
    pub tangent: Vec2,   // unit, along the aperture
    pub normal: Vec2,    // unit, out of the wall face (tangent ⟂ normal)
    pub velocity: Vec2,  // the aperture's own motion, px/s (ZERO today)
}
impl PortalFrame {
    pub fn to_local(&self, p: Vec2) -> Vec2;   // (tangent·(p-origin), normal·(p-origin))
    pub fn from_local(&self, l: Vec2) -> Vec2;
}
/// The pair map: local coords conjugate with a flip through the aperture.
pub fn map_point(a: &PortalFrame, b: &PortalFrame, p: Vec2) -> Vec2;
/// Galilean velocity composition — THE moving-portal rule (P2):
/// v_out = R(v_in − a.velocity) + b.velocity, where R is the pair rotation.
pub fn map_velocity(a: &PortalFrame, b: &PortalFrame, v: Vec2) -> Vec2;
```

Execution shape: introduce the type; re-express the existing cardinal
`map_point`/`portal_map_vec`/orientation policy as `PortalFrame` calls
with `velocity = ZERO` and tangents restricted to cardinals; the ENTIRE
portal suite (46 tests) must pass byte-identically before any new
capability is used — that parity gate is what makes this card
opus-safe. Only THEN does P2 read non-zero `velocity` (relative sweep =
cast the segment `(body_path − portal_path)` against the static
aperture) and P3 relax the cardinal restriction.

## 7. Slices (executor-graded)

| # | Slice | Grade |
|---|---|---|
| CC1 | ⏳ PARTIAL LANDED 2026-07-06 (opus). `ambition_engine_core::cast` minted as THE swept-primitive API surface: re-exports `AabbExt`/`AabbSweepHit` + owns the public `body_sweep()` entry (delegates to `World::first_body_sweep`, which holds privileged block-set access); external caller (platformer `kinematic`) repointed; engine_core 236/236 green (no behavior change). **REMAINING (a genuine hard problem → see fable log below):** `first_circle_hit` is intimate with the momentum kernel's surface types and won't extract without behavior-change risk on the most-guarded code; `raycast_solids`/`ray_aabb` live in `ambition_platformer_primitives` (generic over `SolidWorldQuery`); and `raycast_through_portals` (`ambition_portal`) can't move into engine_core without the engine-level aperture type that **IS CC5's `PortalFrame`** — so the portal-cast absorption is fable's CC5, not opus's CC1. | [opus done to the safe boundary; **fable** owns the kernel-extraction + portal-tier absorption] |
| CC2 | ⏳ FIRST PASS LANDED 2026-07-06 (opus; **fable reviews the classification below**). Added the trigger-tier swept primitive `cast::aabb_path_contacts(center, half, delta, target)` (preserves the discrete standing-in-it case exactly, adds the swept path — parity for slow bodies, catches tunnels; unit-tested). **Converted:** hazard touch (player + actor victims, both now sweep their path; tunneling test proves a fast body across a thin spike takes the hit). **Classification audit:** ⬇︎ hazards→SWEPT (done); blink-destination validity→ALREADY SWEPT (`first_body_sweep`, blink.rs); GroundItem pickup→DISCRETE-OK (button-gated `melee_pressed`, not a path auto-collect — annotated in code); solid movement (both kernels)→ALREADY SWEPT. **REMAINING to classify/convert (fable-reviewed): auto-collect coins/rings** (if touch-to-grab → SWEPT), **mid-room Door loading zones** (SWEPT) vs EdgeExit (DISCRETE-OK, at the boundary), **water/climbable region entry** (likely DISCRETE-OK — large regions, one-frame-late grab harmless — but a thin fast-crossed strip wants swept), **ledge grab** (contact-based; audit). Each remaining site gets its in-code classification note + conversion in the CC2 completion pass. | [opus first pass; fable reviews] |
| CC3 | The composed-world fuzz invariant rig + CI wiring | [opus] |
| CC4 | Broadphase grid for chains+blocks casts (profile first) | [opus] |
| CC5 | `PortalFrame` (P1) + parity pins | [fable / opus-with-parity-gate] |
| CC6 | Moving portals (P2): moving carve + relative swept trigger + velocity composition + C4/portal conjugation tests | [opus, fable-specced after CC5] |
| CC7 | Angled portals (P3) + dynamic straddle (P4) | [opus, post-demo] |
| CC8 | AABB slope vocabulary (S2, Q15) — only when a demo/content demands it | [opus, fable-specced] |

Exit for the doctrine: CC1–CC3 landed and the fuzz rig green over all
shipped rooms; §7.6-style bugs become impossible to write without failing
review (an unswept path-dependent reader is a flagged pattern, same tier as
`AMBITION_REVIEW(spatial)`).
