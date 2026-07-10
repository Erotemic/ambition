# Collision & CCD — sweep everything, kill the OOB class, free the geometry

**Authored by fable, 2026-07-05; runtime contracts pinned 2026-07-06 (fable),
answering the GPT-5.5 contract review.** The doctrine that retires
out-of-bounds bugs as a CLASS, makes non-axis-aligned geometry first-class,
and takes portals from static wall decorations to movable physical objects.
Companion: [`spatial-model.md`](spatial-model.md) (authoring/IR),
[`frame-awareness.md`](frame-awareness.md) (the "relative to what?" review
discipline). Binding rules inherited: **no pushout** (M10; sole exception:
portal-close straddle eviction), no speed caps as bug fixes (Jon: "speedy
thing comes out" is sacred), frame-agnosticism (C4 harness for every rule).

---

## 1. The diagnosis

Every OOB/tunneling bug we have ever fixed had the same shape: **some check
sampled positions discretely** while the mover was swept, or vice versa. The
mockingbird clip (pushout-teleport), the portal high-speed embed (discrete
transit trigger vs. swept solids), the historical ledge and wall-cling
escapes — all instances of ONE disease: *mixed sampling disciplines within a
single frame of motion.*

The cure is not more guards. It is a single rule:

> **THE SWEEP LAW.** Anything that changes state as a function of a body's
> path — solid contact, trigger volumes, portal transit, hazard touch, blink
> destination validity, one-way admission, ledge grab, water/zone entry —
> evaluates against the CONTINUOUS swept path `prev → curr`, never against
> sampled endpoints. Discrete sampling is permitted only where §3.3 grants it
> explicitly, with the grep-able annotation.

Both movement kernels already obey the law for SOLIDS (axis-swept AABB; the
momentum circle's TOI casts). The remaining offenders are the TRIGGER-shaped
readers; CC2 is converting them (hazards done).

## 2. The unification target: one contact vocabulary, two kernels, shared casts

We deliberately keep **two movement kernels** — the axis-swept AABB kernel
(the protected classic-feel fast path) and the surface-momentum kernel
(chains/blocks, circle proxy). What unifies them is BELOW and BESIDE them:

- **Below — the cast library** (`ambition_engine_core::cast`, minted CC1):
  the primitive queries both kernels and every trigger reader call. No system
  rolls its own overlap/step check. The full family registry and ownership
  rulings: §3.4.
- **Beside — the `Contact` vocabulary.** Every contact from any kernel or
  cast reports the same `Contact { point, normal, toi, surface_velocity,
  source }` (landed 2026-07-05). Gameplay consumes contacts; it never asks
  "which kernel produced you?".

**Blocks ARE surfaces** (landed `0189338b`): a solid block's exterior
boundary is a closed rectangular `SurfaceChain`, swept per-face and ridden by
the momentum kernel with the same stick/joint rules as authored chains. The
AABB is formally a special case of the polyline surface — cheap because
rectangular, not because privileged.

## 3. THE RUNTIME CONTRACTS (pinned 2026-07-06 — executors implement these, not their own readings)

### 3.1 The canonical sweep sample — ✅ LANDED (fable, 2026-07-07; the ECS seam RULED and executed)

Every swept reader consumes ONE authoritative motion record per body per
tick. **The seam question opus parked (the decision brief) is RULED and
the ruling is IN CODE:** the sample is **the simulation phase's own
integration segment, with BOTH endpoints captured INSIDE the kernel** —
`prev` at sim-phase entry, `curr` at sim-phase exit
(`update_body_simulation_with_clusters` wraps the inner step and writes
it; `engine_core::SweepSample`, an `Option<&mut>` member of
`BodyClustersMut`/`ActorMut` so adoption is incremental with zero
scratch/test churn).

```rust
/// engine_core::body_clusters — Component on every spawned body
/// (AncillaryMovementBundle carries it for players AND actors).
pub struct SweepSample {
    pub prev: Vec2,     // position at simulation-phase ENTRY
    pub curr: Vec2,     // position at simulation-phase EXIT (may differ from
                        // the body's CURRENT pos on teleport frames — the
                        // sample is the traveled path, not the endpoint)
    pub vel: Vec2,      // velocity at prev (the motion that produced the path)
    pub half: Vec2,     // the body proxy: AABB half-extents
}
```

**Why this shape kills the reset-protocol problem entirely:** a position
change OUTSIDE the sim-phase window can never become path — blink
(control phase), the player respawn wrapper (after sim returns), portal
transfer / room transition / mark-recall / mount positioning (other
systems) are all excluded BY CONSTRUCTION. The brief's "~20-site reset
surface across 3 crates" does not exist under these semantics; there is
no protocol for external writers to violate. The blink classification is
thereby also ruled: **blink is a teleport, never path** — the body does
not traverse the gap (that is blink's design identity: crossing
BlinkWalls and pits), and the phase split enforces it for free.

Contract rules (each is a review-flag when violated):

1. **`prev` is recorded inside the kernel, never reconstructed.**
   `pos − vel·dt` is FORBIDDEN as a path source for readers that have a
   sample. Readers keep a `vel·dt` FALLBACK only for bodies without the
   component (bare test hurtboxes, movers not yet writing one — bosses'
   `integrate_boss_bodies` is the known remaining mover); delete each
   fallback when its mover writes samples.
2. **Every mover writes its own segment.** The shared pipeline writes it
   in the kernel; non-pipeline movers (the surface-walker branch, the
   home momentum path) write theirs around their own step — same capture
   rule, both endpoints inside the mover. `reset_body_clusters` leaves a
   zero-length record at spawn (a respawn is a teleport, never path). A
   zero-dt tick records a zero-length segment, never a stale one.
3. **One frame, one chart.** The segment lives entirely in one coordinate
   chart automatically: a transfer happens outside the sim window, so the
   next segment starts in the new chart. *Documented v1 bound:* the
   sub-frame emergence distance on a transfer frame's exit side is
   unswept for that one frame; the future extension (if fuzzing trips it)
   is a two-segment polyline — do not build it speculatively.
4. **Reference frame:** samples are world-frame. A reader comparing
   against a MOVING target subtracts the target's own frame motion
   (relative sweep, `delta_body − delta_target`) — the CC6 moving-portal
   rule.

Tests pinning the contract (engine lib, `movement/tests/sweep_sample.rs`):
segment recorded; zero-dt = zero-length; a control-phase blink is never
path; the respawn wrapper leaves a zero-length record at spawn. The
hazard reader (both victim arms) consumes `sample.delta()` with the
rule-1 fallback.

**Status: IN CODE (fable, 2026-07-07).** Adopter audit (opus, 2026-07-06):
- **Bosses are NOT a remaining adopter** — the "`integrate_boss_bodies` hits
  the fallback" note was conservative. Bosses spawn AxisSwept
  (`motion_model = None`), so `integrate_boss_bodies` → `integrate_actor_body`
  → `ActorMut::update`'s else-branch → `integrate_body` →
  `update_body_with_tuning_clusters` → `update_body_simulation_with_clusters`,
  which writes the sample; the boss carries the component
  (`AncillaryMovementBundle`) and the actor query plumbs it, so the hazard
  reader already consumes the boss's real segment.
- **The actual gap was the SurfaceMomentum ACTOR mover** — fixed (opus
  2026-07-06). `integrate_actor_body`'s momentum dispatch calls
  `step_momentum_body` and returns early, bypassing the kernel's sample write,
  so a worn-momentum actor kept a stale zero-length sample → the hazard reader
  saw no path → a fast momentum body tunneled spikes. It now writes its own
  segment around the step (rule 2, mirroring the surface-walker branch + the
  home momentum path).
- **Portal transit trigger adopted the sample** (Codex 2026-07-09). The
  portal-local `PortalSweepAnchor` is gone; `portal_transit` feeds its swept CCD
  tier from the canonical `SweepSample`, guarded by a live-endpoint check so a
  post-sim teleport cannot become fake aperture travel. Future movers still
  follow rule 2: write your own segment around any non-kernel motion.

**What the sample deliberately does NOT carry (asked and answered):**
- *No kernel tag / body proxy beyond `half`.* Readers are kernel-agnostic
  by design — that is the Contact-vocabulary rule (§2); a reader that
  wants to know "which kernel" is mis-designed.
- *No frame/chart context.* Samples are world-frame, one chart per frame
  (rules 3–4); chart provenance on transfer frames lives in the transit
  machine, and the reset protocol means readers never need it.
- *No portal context.* A transfer RESETS the sample (rule 2); which
  aperture was crossed is the transit machine's record (and the CC3
  trace's event channel), not motion-record state.
- *No geometry/contact context.* Contacts are the separate `Contact`
  vocabulary (§2); the sample is the QUESTION (the path), contacts are
  ANSWERS.
Growth rule: a field joins `SweepSample` only when TWO independent swept
readers need it (the same two-consumer discipline as every seam).

Implementation home: the sample is engine_core vocabulary; kernels write it
(the AABB kernel at step end; the momentum kernel after `resolve_surface`),
the reset rule binds every teleporting system. CC2-completion carries the
mint (it is the first multi-reader consumer).

### 3.2 Frame event ordering — the three authority classes

What happens when one frame's path crosses several things (portal + hazard +
pickup + door)? Every path consumer is one of three classes, and the classes
have a pinned order:

- **Class A — motion authority** (mutates pos/vel by physics): the two
  movement kernels. Exactly ONE kernel owns a body (`MotionModel`); it
  resolves solid contacts internally at TOI. The post-A position IS the
  frame's sample.
- **Class B — transit authority** (remaps pos/vel/chart): portal transit,
  loading-zone room transitions, death/respawn, scripted teleports. Contract:
  **at most one Class-B action applies per body per frame — the earliest by
  TOI along the sample; ties break by fixed priority `death/reset >
  room-transition > portal-transit`** (death ends the frame's story; a body
  cannot die AND door-warp). Every Class-B application resets the sample
  (§3.1 rule 2), which is what makes a second Class-B reader a no-op the
  same frame — the protocol enforces the "one action" rule structurally,
  not by inter-system negotiation. Schedule order today approximates this
  (transit runs before zone checks); the CC3 fuzz rig asserts the invariant
  ("no two Class-B remaps in one frame"), and violations are re-ordering
  bugs, not tolerated races.
- **Class C — observers** (read the path, never move the body): hazard
  touch, pickups, water/climb entry, ledge, blink validity, camera/music
  zones. Class-C readers are commutative — no ordering among them is
  promised or needed. Ideal rule: a Class-C reader consumes the path only up
  to the earliest Class-B TOI (you don't take spike damage from path you
  never traversed because a door warped you first). *v1 tolerance:* readers
  evaluate the whole segment; the over-read is bounded by one frame and only
  on transit frames. The fuzz oracle does not police it in v1.

### 3.3 Per-trigger semantics — every reader declares its verb

"Make triggers swept" is not a spec. Each path-dependent reader declares ONE
of these semantics (this table is the CC2-completion checklist; new readers
add a row in the same commit):

| Reader | Semantics | Status / ruling |
|---|---|---|
| Solid contact (both kernels) | FIRST-TOI resolve (Class A) | swept since forever |
| Portal transit | FIRST-TOI ENTER, direction-gated (front→behind through the aperture), full state machine (`transit_step`) | swept (`31342e6f`) |
| Hazard touch | ANY-HIT along path → damage event; repeat-contact suppressed by existing i-frames (STAY handled by re-arming, not by the sweep) | swept 2026-07-06 (CC2 first pass) |
| Blink destination validity | full-path body sweep (`cast::body_sweep`) | already swept |
| GroundItem pickup | DISCRETE-OK: button-gated (`melee_pressed`), a deliberate act at rest, not a path auto-collect | annotated in code |
| Auto-collect tokens (coins/rings-analog) | ANY-HIT along path, each volume independent (order along the path irrelevant) | N/A — no auto-collect pickup exists yet (GroundItem is the only pickup, button-gated); the pattern (route through `cast::aabb_path_contacts`) is documented in `items/pickup/mod.rs` for whoever adds one (opus 2026-07-06) |
| Mid-room Door loading zones | FIRST-TOI ENTER (Class B) | swept 2026-07-06 (CC2-completion): `transition_for_player` sweeps the body's `delta` via `cast::aabb_path_contacts`; Door stays interact-gated so the sweep only helps it |
| EdgeExit / Walk loading zones | overlap-fire; `Walk` (mid-room, not edge-backed) IS the real tunnel case; EdgeExit is edge-backed (a tunnel past it is an OOB CC3 catches) | swept 2026-07-06 (CC2-completion): the ONE `transition_for_player` sweep subsumes both; tunnel test pins a fast body crossing a `Walk` band |
| Water / climbable region entry | ENTER/EXIT state edges, DISCRETE-OK for thick regions; the authoring validator flags any region thinner than `max_expected_speed · dt` (thin strips must thicken or the reader converts) — sweep the AUTHORING, not every frame | done 2026-07-06 (CC2-completion): readers annotated `discrete_ok`; `World::thin_region_warnings` (floor = `MAX_EXPECTED_BODY_SPEED/60 = 26px`) wired into room `layout_warnings` |
| Ledge grab | derives from kernel contacts (Class A output) — swept by construction; audit confirms no residual endpoint check | audited 2026-07-06 (CC2-completion): the probe fires off a resolved wall contact, not a trigger overlap — swept by construction, annotated `discrete_ok` at the probe site |
| Camera / music / ambient zones | DISCRETE-OK: genuinely positional, one-frame slop imperceptible | blanket grant |

**The discrete-OK convention (grep-able, mandatory):** any reader granted
discrete sampling carries at its check site:

```rust
// AMBITION_REVIEW(discrete_ok): <one-line reason — why a one-frame miss is
// harmless or impossible here>
```

Same review tier as `AMBITION_REVIEW(spatial)`. An unswept path-dependent
reader WITHOUT the annotation is a flagged pattern in review; CI may grep
for known trigger idioms later, but the convention is the contract now.
(The existing GroundItem note gets the exact marker in the CC2-completion
pass.)

### 3.4 What `engine_core::cast` IS — identity + the family registry

**Ruling: `cast` is the permanent naming and discovery surface for every
path/overlap query in the engine — not a temporary facade.** Implementations
live where their data intimacy demands; `cast` is where callers LOOK, and
the only module allowed to re-export query entry points. Concretely, the
three CC1 rulings (asked by opus 2026-07-06):

- **(a) `first_circle_hit` stays kernel-private.** It is load-bearing
  interior of the momentum kernel (`surface.rs`: `SurfaceChain` +
  `resolve_surface` intimacy, on the no-pushout/OOB path). It is NOT
  extracted and NOT re-exported. `cast`'s doc header names it as the
  kernel's internal swept-circle tier; a PUBLIC swept-circle query is minted
  in `cast` only when a consumer outside the kernel lands (AJ13 discipline:
  real pressure, not symmetry-lust). Until then, "no system rolls its own
  cast" is satisfied — no system outside the kernel needs a circle cast.
- **(b) `ray_aabb` + `raycast_solids` + `SolidWorldQuery` move DOWN into
  `engine_core::cast`.** They are pure geometry / world-query with zero
  platformer semantics; `ambition_platformer_primitives` already depends on
  engine_core, so the move is with-the-grain (and the orphan-rule note in
  `world_query.rs` dissolves: the `impl SolidWorldQuery for World` lands
  beside `World`). Consumers repoint in the same arc; no re-export shim
  stays behind (D2 rule). [opus, mechanical]
- **(c) The portal-aware cast lands in `cast` WITH CC5.** It needs the
  engine-level aperture vocabulary (§7); once `PortalFrame`/`PortalAperture`
  live in engine_core, `cast::ray_through_apertures` is correctly layered
  and `ambition_portal`'s `raycast_through_portals` becomes the gameplay
  wrapper that supplies apertures from `PlacedPortal`s (channels, tuning,
  recursion budget stay portal-side). The dependency inversion GPT-5.5
  flagged is broken exactly here: **engine_core owns the GEOMETRY of an
  aperture pair; ambition_portal owns the GAMEPLAY of portals** (placement,
  channels, cooldowns, carve policy, transit machine).

The family registry (which APIs exist, where, and which slice needs them):

| Family | API | Home | Needed by |
|---|---|---|---|
| Swept AABB vs AABB | `AabbExt::sweep_hit` | geometry, re-exported by cast | CC1 ✓ (exists) |
| Swept AABB vs world | `cast::body_sweep(world, body, delta, predicate)` | cast (delegates to `World`'s privileged block access) | CC1 ✓ (exists) |
| Trigger path contact | `cast::aabb_path_contacts(center, half, delta, target)` | cast | CC2 ✓ (exists) |
| Segment ray vs AABB | `ray_aabb` | cast after ruling (b) | CC1-completion |
| Ray vs solid world | `raycast_solids` over `SolidWorldQuery` | cast after ruling (b) | CC1-completion |
| Portal-aware ray | `cast::ray_through_apertures` | cast, with CC5 | CC5 |
| Swept circle vs surfaces | kernel-internal (`first_circle_hit`) | surface.rs, ruling (a) | — (public form: future-only) |
| Swept trigger vs MOVING target | relative-sweep form of `aabb_path_contacts` | cast | CC6 (moving portals) |
| Convex/OBB sweep | parry-backed shape cast | cast | future-only (P3b/P4, combat sweeps) |

CC1's exit restates as: rulings (a)/(b) executed, the registry above true in
code, `cast`'s module doc IS this table.

### 3.5 Portal-aware cast semantics (pinned for CC5)

What "a cast continues through a portal" means, exactly:

1. **Continuation:** a ray that reaches an aperture's plane, within its
   opening, entering from the FRONT (`dir · normal < 0`), before any solid
   hit, re-anchors at the mapped point on the exit and continues along the
   mapped direction. (This is today's behavior, kept.)
2. **TOI/distance is cumulative:** one `max_dist` budget decremented across
   hops; the recursion bound (`max_depth`) guards mutual-facing loops.
3. **Chart of the result:** hit point + normal are returned in the FINAL
   chart (where the ray landed). Callers that need the path's provenance
   (which pairs were crossed) get it only when a consumer demands it —
   v1 returns `(point, normal)` as today; do not speculatively grow the
   return type.
4. **Aperture clipping:** the crossing point must lie within
   `aperture_half` along the tangent; a ray hitting the PLANE outside the
   opening ignores the portal (hits whatever is behind).
5. **Swept SHAPES through portals (AABB/circle/body) are NOT promised.**
   Body transit is the transit machine's job (piece decomposition +
   centroid transfer), not a cast-family feature. The only shape-through-
   aperture query the engine commits to is the transit trigger's segment
   test (centroid path vs. plane-within-opening). P3b/P4 revisit this; a
   cast API is not the vehicle.

### 3.6 Geometry identity — `GeoId` (RULED, fable 2026-07-06 night; answers the GPT-5.5 identity questions)

**The question this closes:** what durably names a piece of geometry — for
`WorldDelta` ops, the CC6 portal host ref, save overlays, and debug
traces? Today `Block.name` is an informal display string and nothing else
exists. The ruling:

```rust
// ambition_engine_core (beside World/Block/SurfaceChain):

/// Durable identity of one piece of ROOM geometry. Two-level: WHERE it
/// came from + its deterministic ordinal within that source's emission.
pub struct GeoId { pub source: GeoSource, pub index: u16 }

pub enum GeoSource {
    /// Entity-authored geometry (a Solid/OneWay/SurfaceChain LDtk entity,
    /// or any backend's placement): the placement id IS the identity
    /// (the [W-d] `PlacementId` — LDtk iid / bake-synth).
    Placement(PlacementId),
    /// Grid/tile-derived geometry (IntGrid merge → solid rects): keyed by
    /// layer name; `index` = the merge ordinal. The merger MUST iterate
    /// deterministically (row-major over the grid) so the same map always
    /// yields the same ids — that determinism is part of this contract.
    TileLayer { layer: String },
    /// Output of a parameterized generator marker (`SurfaceLoop`,
    /// `SurfaceRamp`): the MARKER's placement id + the emission ordinal
    /// (segment k of the arc). Regenerating from the same marker params
    /// yields the same ids.
    Generator(PlacementId),
    /// Geometry ADDED by a WorldDelta op (a dug tunnel's new wall): the
    /// op's sequence number in the room's delta list is durable because
    /// it is IN the save.
    Delta { op_index: u32 },
    /// Test/fixture geometry. The authoring pipeline NEVER emits this;
    /// the delta/save layer REJECTS ops naming it (validator).
    Anon,
}

/// A face + position on identified geometry — the "host face" vocabulary
/// moving portals, deltas, and traces share.
pub struct GeoFaceRef {
    pub geo: GeoId,
    pub face: Face,     // AABB blocks: Top/Bottom/Left/Right (world-axis,
                        // +y-down: Top = the min.y face). Chains/polygons:
                        // Face::Segment(u16) — the polyline segment index.
    pub along: f32,     // px offset from the face's CENTER, tangent-signed.
                        // (px, not normalized — geometry doesn't resize;
                        // px is what placement math uses today.)
}
```

**The rules that make it work:**

1. **`Block` gains `id: GeoId`;** `name` stays the human label, derived
   from the id (entity-id-matches-label). Constructors used by tests
   default to `Anon` so the fixture surface doesn't churn; the
   IR emission paths always assign real sources.
2. **Only AUTHORED-tier geometry has durable identity.** Carve pieces,
   split blocks, and every product of per-frame composition are DERIVED
   state (the same rule as N3.1/W-c: derived is never persisted). Their
   working identity is `(parent GeoId, derivation ordinal)` and is valid
   for ONE frame only — nothing may store it across frames or into a
   save. A `WorldDelta` op therefore names authored `GeoId`s
   (`RemoveBlock(GeoId)`, `AddBlock { .. } → GeoSource::Delta`); the
   composition then re-derives.
3. **Runtime resolution is a lookup, not a pointer.** The composed
   collision world carries each block/chain's `GeoId`; consumers resolve
   `GeoId → &Block` per frame through the room's geometry index. Blocks
   are NOT entities; `Entity` never appears in geometry identity (the
   N3.1 rule generalizes).
4. **Introduction is INCREMENTAL:** mint the types with CC6 (its host
   ref is the first consumer, below); the `TileLayer`/`Generator`
   sources land with the IR paths that emit them (W2); the validator
   rule (no `Anon` in deltas) lands with the first delta op. Do not
   sweep the codebase converting `name` usages speculatively.

**✅ SUBSTRATE MINTED (opus 2026-07-06 night).** The types are in code:
`ambition_engine_core::geo_id` — `PlacementId`, `GeoSource`, `GeoId`
(+ `anon`/`placement`/`tile_layer` constructors, `Default = Anon`), `Face`
(Top/Bottom/Left/Right | Segment(u16)), `GeoFaceRef` — all re-exported at the
crate root beside `World`/`Block`. **`Block` gained `id: GeoId`;** the six
`Block` constructors + the ~10 struct-literal sites default to `GeoSource::Anon`
so the fixture/composition surface is byte-parity (the id is inert — no logic
reads it yet). Real `Placement`/`TileLayer` sources are NOT assigned: they need
the iid threaded through the emission paths + a constructor variant, which is
W2's `RoomEmission` reshaping (rule 4 above) — assigning them now would be the
speculative sweep this rule forbids. The FIRST consumer is CC6's `PortalHostRef`
(= `GeoFaceRef`); the validator (no `Anon` in deltas) lands with the first
`WorldDelta` op. engine_core 252 (+3 geo_id tests), gameplay_core lib 1175,
full app rl_sim suite green.

## 4. Non-axis-aligned geometry

The end state: **a room may be built from arbitrary polyline/polygon
geometry**, with axis-aligned tiles as the fast common case.

- **S1. Chains are already the answer for actors on the momentum path.**
  `SurfaceChain` (open/closed, one-sided, validated winding) + the follower
  solver handle slopes, valleys, loops, moving surfaces. Nothing new needed.
- **S2. The AABB kernel gets a bounded slope vocabulary, not a rewrite**
  (CC8, gated on a demo needing it). Rules pinned BEFORE implementation:
  - *Foot sampling:* two probe points on the feet edge (±0.5·half-width
    minus a small inset), ground height = max of the two chain projections;
    the body stands on the higher.
  - *Support threshold:* surface slope ≤ 45° from the gravity-perpendicular
    counts as ground; steeper reads as WALL (the axis-swept step treats it
    as a side contact — no sliding pseudo-physics on the AABB kernel).
  - *Snap distance:* downhill ground-follow snaps only within
    `is_contact_range_snap` (the mockingbird rule) along gravity; never
    lateral.
  - *No-pushout guarantee:* rising ground under horizontal motion is
    resolved by the SWEPT step as a ramp contact (the body rides up as part
    of TOI resolution); if one frame's rise exceeds the step budget it is a
    wall hit, never a lift-teleport.
  - *One-way slopes:* admission by feet-side plane crossing, same rule as
    flat one-ways (C4-tested).
  - *Joints:* the chain's height function is continuous across segment
    joints; the walker never sees a seam (reuse the follower solver's joint
    rules).
  - *Moving slopes:* inherit `surface_velocity` exactly like flat floors
    (the Contact vocabulary already carries it).
  The AABB kernel NEVER learns general polygons — bodies that need loops
  ride the momentum kernel (that's what `MotionModel` per-body policy is
  FOR).
- **S3. `SurfacePolygon` — solidity defined per consumer** (mint when a demo
  needs true rotated solids, not before). A closed, validated-winding
  boundary chain + `solid: true`. What "solid polygon" means to each
  consumer, pinned now so the mint is mechanical:
  | Consumer | Meaning |
  |---|---|
  | Momentum kernel | rides the boundary chain (existing chain rules; the interior is unreachable because the boundary is one-sided solid) |
  | AABB kernel | NOT support, NOT collision in v1 — a room mixing AABB-kernel bodies with polygon floors is an authoring-validator ERROR until CC8's slope vocabulary opens (then: ≤45° faces per S2) |
  | Combat/triggers | parry convex decomposition (CombatVolume already speaks OBB/convex) |
  | Blink/spawn validity | point-in-polygon (interior = invalid) + the existing body sweep against the boundary chain |
  | Portal carving | polygon hosts REJECTED by the placement validator until P3b (carve is AABB subtraction today) |
  | Broadphase | conservative AABB hull (the IR emits it alongside the chain) |
  | OOB oracle | center strictly inside a solid polygon = illegal, same as blocks |
- **S4. Broadphase honesty.** Casting against every segment in a big room is
  the momentum kernel's current behavior; before Sanic-scale zones land, add
  a uniform grid/interval index over `World.blocks + chains` keyed by the
  swept path's AABB. Bounded, mechanical, measured (profile first — the
  current N is small). NOT a precondition for CC1–CC3.

Authoring: LDtk `SurfaceChain` entities + generated markers (`SurfaceLoop`,
`SurfaceRamp` — Q27) exist/planned; parameterized generator entities are how
LDtk stays sufficient without a second backend.

## 5. Portals become physical objects

Today: portals are static wall-mounted apertures; transit is swept; the
carve is static per placement. The arc, with the object model pinned:

- **P1 (=CC5). The `PortalFrame` type** — ✅ landed; conventions in §7.
- **P2 (=CC6). Moving portals (translation).** ✅ **LANDED 2026-07-09.** The
  object model, as built:
  - **Portals are HOST-ATTACHED apertures, never free entities.** The host ref
    is the §3.6 vocabulary exactly: `GeoFaceRef { geo: GeoId, face, along }` —
    NOT a Bevy entity (blocks aren't entities), NOT a raw index into the
    composed world (recomposed per frame), NOT a bare placement id (a placement
    may emit several blocks; the portal needs the face). The frame's `origin`
    re-derives each frame by resolving `geo → &Block` and evaluating `face +
    along`; `velocity` IS the host block's authoritative `Block.velocity` —
    **never finite-differenced from positions.** Static-wall portals get the
    same ref with zero host velocity: one representation, no static/moving
    split.
  - **Update order (one frame), pinned:** (1) hosts/platforms integrate; (2)
    portal frames re-derive from hosts; (3) carve re-composes; (4) body kernels
    sweep the re-composed world; (5) the transit trigger runs the RELATIVE
    sweep (body segment − portal segment — one subtraction, both linear over
    the frame); (6) transfers apply `map_velocity` (Galilean composition, §7);
    (7) pieces re-evaluate from the new frame transform; (8) presentation reads.
  - **Edge cases, ruled:** a portal sweeping over a STATIONARY body transits it
    (the relative segment is nonzero — the aperture moved over the body; this
    is correct physics and a designed capability: a descending portal "scoops" a
    standing actor). `min_exit_speed` composes in the EXIT FRAME's rest frame —
    the floor applies to `v_out − exit.velocity` along the exit normal, THEN the
    frame velocity adds back (otherwise a fast-moving exit portal could never
    satisfy, or would trivially satisfy, the floor). Host ROTATION is out of
    scope (translation only). Carves recompute discretely per frame — acceptable
    because the carve is a solidity assist sized with reach margins; the
    RELATIVE swept trigger is the correctness backstop.
- **P3. Angled portals — scope split (answering "not just authoring+math"):**
  - **P3a (authoring + math):** apertures at arbitrary tangents for
    point/ray/velocity mapping and CENTROID transit of fully-contained
    bodies. The frame math (§7) is already angle-general; P3a relaxes the
    cardinal restriction for the map, the transit trigger (plane test is
    angle-general already), and placement/authoring. The C4 harness extends
    to arbitrary-angle conjugation (transit through a θ-portal == rotate,
    transit through cardinal, rotate back).
  - **P3b (real geometry):** STRADDLING bodies at arbitrary angles. This is
    where AABB pieces stop working: piece geometry becomes convex polygons.
    Ruling on piece geometry: **render pieces = exact clipped convex
    polygons; collision pieces = conservative AABB hulls of those polygons**
    (collision correctness stays with the transit machine + carve, which
    are plane-based and angle-general; the hull only feeds broadphase/
    overlap queries). Angled CARVE needs polygon subtraction from hosts —
    gated on `SurfacePolygon` (S3) landing. P3b is post-demo and does NOT
    block P3a.
- **P4. Portal-carried bodies & dynamic straddle.** P2 makes the straddle
  state dynamic (aperture moves under a straddling body): the piece map
  re-evaluates per frame from the frame transform; eviction stays the ONLY
  pushout in the engine and only on CLOSE.

**§5-P2a — CC6 as-built amendments (fable, 2026-07-09):**
- **`host` is `Option<GeoFaceRef>`, attribution is LAZY.** The placement law
  ("a portal cannot exist without a host face") is enforced where geometry is
  identified: the content adapter attributes every placed portal one-shot
  against the UNCARVED authored+movers view (never the carved composition —
  carve pieces are derived/anon). `None` remains the fixture tier (anon
  geometry can't host) and is byte-identical to the pre-CC6 static portal.
  This kept ~40 fixture sites honest instead of inventing fake host refs.
- **Units, pinned:** `Block.velocity` is the kernels' PER-TICK displacement
  convention (platform carry applies it undtd). `PlacedPortal.vel` is px/s
  (derived at refresh as `anchor.velocity / scaled_dt`) and feeds the frame
  map; the relative trigger uses `pos − prev_pos` (exact displacement, no
  dt round-trip).
- **P4 partially emerges free:** host-carried motion under a straddler does
  NOT evict (the eviction diff allows displacement == the host's frame
  delta), and pieces already re-evaluate per frame from the live aperture.
  Dynamic-straddle piece correctness at speed remains P4's to verify.
- **Host rotation stays out (translation only)** per the P2 ruling.
- **Gate discovery:** the content portal suite compiles only under
  `--features portal` — the default `cargo test -p ambition_content` does
  NOT run it. The parity-gate suite list must name
  `-p ambition_content --features portal` explicitly (D2 gate list updated).

Execution grades: CC5 [fable — landed]; CC6 [fable — landed 2026-07-09];
P3a [opus after CC6]; P3b/P4 [opus, post-demo, gated on S3].

## 6. The OOB endgame: guarantee, not vigilance

### 6.1 The fuzz oracle (CC3) — exact illegal-state definition

**Status (CC3 largely EXECUTED, opus 2026-07-10).** Four of the six
invariants are live in `ambition_app/tests/collision_invariant_oracle.rs`,
the pinned §6.2 minimum payload is attached, and the run matrix is every
shipped room. The diagnostic-only posture and the repro-line format are
unchanged, as ruled.

| Invariant | Status |
|---|---|
| 1 embed-in-solid | ✅ **carve-aware.** `solid_blocks` now composes the world through `ambition_world::collision::world_with_portal_carves` before testing, and includes `BlinkWall`. **The transit exemption falls out of the geometry**: a straddling body's center sits in a hole that no longer contains a block, so no `PortalTransit` special case is needed. |
| 2 straddle-outside-carve | ⏳ **remains.** Needs the transiting body's straddled-portal identity exposed to a test — the aperture volume is knowable, the *which portal* is not, without a read-model row. |
| 3 out-of-bounds | ✅ pre-existing (by side, with the authored-exit suppression + the through-wall classifier). |
| 4 NaN/inf | ✅ **folded in and CATALOGED**, pos and vel. It short-circuits: every geometric test is meaningless on a non-finite body. |
| 5 one Class-B remap per frame | ⏳ **remains, and is the one that needs new machinery** — the Class-B writers must emit a countable event (§3.2). Not fakeable from outside. |
| 6 one-way fall-through | ✅ **live.** Tracks the one-way a body was supported by at the end of last tick; fires when this tick's center ends below its top with no drop-through intent (held descend axis) and no Class-B remap (room load / respawn). |

**THE MEASUREMENT (2026-07-10): 72 rooms × 3 seeds × 300 ticks = 64,800
stepped frames, 15 violations, and NOT ONE of them is a collision bug.**

```
  gap_run             OOB-BELOW-FLOOR (open edge)   x1
  intro_escape_shaft  OOB-SIDE (open edge)          x2
  tiny_chamber        OOB-SIDE (open edge)          x3
  under_town_pipes    OOB-SIDE (open edge)          x8
  portal_lab          TELEPORT                      x1   (290px — a portal transit)
```

- **Zero `EMBEDDED-IN-SOLID`.** Zero `ONE-WAY-FALL-THROUGH`. Zero
  `NON-FINITE`. Zero OOB that ended past a Solid at the crossed edge
  (the `[past-solid?]` suspect class is empty).
- All 14 OOB are open-edge walk-offs — level authoring, which §6.1 already
  calls legal.
- **The one TELEPORT is a false positive**, and a known one: §6.1's
  "Explicitly legal" list says *"the transfer frame's position jump"* is
  legal provided invariants 1–4 hold at frame end, and they do. The
  teleport probe predates the numbering and does not yet consult the portal
  crossing channel. **Fix rides invariant 2's slice** (both want the same
  read-model row); until then it is one known, named line in the catalog.

So the OOB class §6 exists to kill is, on this evidence, **already dead in
every shipped room** — which is what turns "vigilance" into "guarantee".
CC3 stays diagnostic-only per Jon's ruling; what the numbers argue is that
promoting it to a hard gate would now cost only the authored-exit
allowlist, not a bug hunt.

The rig (per shipped room: random spawns, random high-speed impulses incl.
through portals, N seconds stepped headlessly) asserts, at the END of every
stepped frame, per body:

**Illegal (any ⇒ failure):**
1. Body CENTER strictly inside a `Solid`/`BlinkWall` block's AABB **after**
   carve subtraction (the composed world is the truth — a carved hole is
   not solid) — UNLESS the body carries an active `PortalTransit` (a
   straddling body's center legitimately sits behind the plane inside the
   carve; its legality is the pieces', tested by the next rule).
2. A transit-straddling body whose center is NOT within the straddled
   portal's carve volume (embedded outside the aperture = the §7.6 class).
3. Body center outside the room's world AABB inflated by margin M (one body
   height), without a death/reset/room-transition event for that body this
   frame or the K=3 frames prior.
4. `NaN`/`inf` in pos or vel.
5. Two Class-B remaps applied to one body in one frame (§3.2 — the ordering
   invariant, asserted structurally).
6. One-way violation: a body that ended BELOW a one-way it was supported by
   last frame without a drop-through intent or a Class-B remap (admission
   is one-directional; silent fall-through is the historical bug).

**Explicitly legal:** overlap with one-way blocks (always); the transfer
frame's position jump (must still satisfy 1–4 at frame END); temporary
hazard overlap (hazards damage, they don't eject); anything during the
frames a body is dead/despawned.

**Determinism note:** the rig is seeded; a failure REPRODUCES from
`(seed, room)` alone. That property is load-bearing — it is what makes the
trace (below) optional-to-read rather than the only evidence.

### 6.2 Required failure trace

On violation the rig dumps (reusing the `debug_traces/` OOB tooling — same
format, extended): seed, room id, actor archetype + `MotionModel` kernel,
and for the last ~120 frames per involved body: pos, vel, the sweep sample,
kernel contacts, trigger events fired (hazard/pickup/zone), portal
crossings (channel, TOI, mapped pos/vel), active transit/cooldown state,
and the specific invariant number violated. The existing OOB trace hook is
the implementation seed; CC3 adds the event channels.

**The MINIMUM payload while CC3 stays diagnostic-only (pinned — ship this
much even before the event channels exist, because it is already nearly
free):** `(seed, room id, tick, invariant #, body SimId + archetype +
MotionModel)` + the existing per-body `BodyKinematics` ring
(`debug_traces/` OOB tooling — pos/vel history is already recorded) + the
`GeoId` of the geometry involved where the invariant names one (the
embedding block, the violated one-way). Each richer channel (sweep
samples once §3.1 mints, Class-B events once invariant 5's counter
exists, portal crossings) JOINS the dump in the same slice that creates
it — the payload grows with the machinery, never speculatively. The
format contract that must hold from v1: a dump reproduces from
`(seed, room)` alone, and every field it names uses durable ids
(SimId/GeoId), never `Entity` values.

### 6.3 Guard deletion + snap discipline

- **Delete the guards the law obsoletes.** Fixed "guard windows" sized to
  worst-case per-frame steps (`APPROACH_CARVE_REACH`, `CARVE_DEPTH`
  compensations) shrink to geometric truths once every trigger is swept;
  each deletion cites the sweeping slice that made it safe.
- **`is_contact_range_snap` discipline** (the mockingbird lesson, twice):
  landing snaps only within contact range along the contact normal — the
  cast module owns the ONE implementation; audited in CC1-completion.

## 7. `PortalFrame` — the exact conventions (CC5, ✅ landed; this is what the code does)

The old `ambition_portal::pieces::PortalFrame { pos, normal, half_extent }` was
the frame PLUS aperture extent in cardinal-AABB clothing. CC5 SPLIT it and moved
the split DOWN (reorganize-don't-adapt; no wrapper, no bridge — the old struct
was deleted in the same arc). The shapes below are live vocabulary:

```rust
// ambition_engine_core::frame (new module; bevy_math only)

/// A portal endpoint IS a frame. World-frame fields (AJ13 naming).
pub struct PortalFrame {
    pub origin: Vec2,    // world-space center of the doorway, on the host face
    pub normal: Vec2,    // unit, OUT of the wall into the room
    pub velocity: Vec2,  // the aperture's own motion, px/s (ZERO until CC6)
}
impl PortalFrame {
    /// Tangent is DERIVED, never stored: normal rotated +90°
    /// (`Vec2::new(-n.y, n.x)`) — the existing `portal_tangent`. Handedness
    /// is thereby pinned; an inconsistent frame is unrepresentable.
    pub fn tangent(&self) -> Vec2;
    /// Local coords: (along, front) = (tangent·(p−origin), normal·(p−origin)).
    /// front > 0 = room side (this IS `front_distance`).
    pub fn to_local(&self, p: Vec2) -> Vec2;
    pub fn from_local(&self, l: Vec2) -> Vec2;
}

/// Frame + opening extent. THE aperture vocabulary the portal-aware cast
/// consumes. Carve depth / capture margins are NOT here — they are
/// ambition_portal gameplay policy.
pub struct PortalAperture {
    pub frame: PortalFrame,
    pub half_length: f32,   // opening half-extent along tangent()
}

/// The pair map. `convention` is an EXPLICIT parameter at this layer —
/// the global flag (`portal_map_rotation()`) is ambition_portal's wrapper
/// concern, never engine_core's.
pub enum MapConvention { Reflection /* det −1, today's default */, Rotation /* det +1 */ }
pub fn map_point(a: &PortalFrame, b: &PortalFrame, c: MapConvention, p: Vec2) -> Vec2;
pub fn map_vec(a: &PortalFrame, b: &PortalFrame, c: MapConvention, v: Vec2) -> Vec2;
/// Galilean velocity composition — THE moving-portal rule (CC6):
///   v_out = map_vec(a, b, c, v − a.velocity) + b.velocity
pub fn map_velocity(a: &PortalFrame, b: &PortalFrame, c: MapConvention, v: Vec2) -> Vec2;
```

**The map, in local coordinates (pinned):** with entry-local `(s, d)` =
`a.to_local(p)`, the image is `b.from_local(s', d')` where depth ALWAYS
flips (`d' = −d`: depth sunk INTO the entry emerges OUT of the exit) and
the along-aperture coordinate depends on convention: **Reflection
(default): `s' = s`** (along-surface preserved — falling right through two
floor portals exits still moving right); **Rotation: `s' = −s`** (the bare
rotation taking `−a.normal` onto `b.normal`; opposite-facing thin-wall
pairs become the identity map). These are exactly today's
`portal_map_vec_reflection`/`_rotation` re-expressed; for CARDINAL normals
every product is by 0/±1, so the frame formulation is bit-identical to the
existing arithmetic — which is why the parity gate is achievable, not
aspirational.

**Parity tolerance was zero** and the entire portal suite passed
byte-identically. If a future change drifts a portal test, the resolution is
exact-op matching, **not** tolerance-loosening.

Carve depth and capture margins are deliberately NOT in the aperture type —
they are `ambition_portal` gameplay policy. The capture box builds from
`half_length` + `TRANSIT_BEGIN_MARGIN` explicitly, which is what the old
`half_extent`'s through-thickness component always meant.

## 8. Slices (executor-graded)

| # | Slice | Grade |
|---|---|---|
| CC1 | ✅ **COMPLETE (fable, 2026-07-06).** All three §3.4 rulings executed: (a) circle stays kernel-private (documented in cast's header); (b) `ray_aabb`/`raycast_solids`/`SolidWorldQuery` moved down into `cast`, `world_query.rs` deleted, consumers repointed; (c) `cast::ray_through_apertures` landed with CC5 — §3.5 segment semantics (incl. the flush-mount tie-break: `t == solid_t` → aperture wins), `raycast_through_portals` is now the gameplay wrapper supplying aperture pairs + the game-wide convention | done |
| CC2 | ⏳ FIRST PASS (opus, 2026-07-06): `aabb_path_contacts` + hazards converted + tunneling test. **Completion = the §3.3 table:** auto-collect → ANY-HIT; mid-room Doors → FIRST-TOI; water/climb thin-region validator rule; ledge audit; the `AMBITION_REVIEW(discrete_ok)` markers; migrate hazard delta onto the §3.1 sample when it mints | [opus — the table is the checklist] |
| CC3 | Fuzz invariant rig — the §6.1 oracle verbatim, §6.2 traces, seeded-reproducible. **DIAGNOSTIC-ONLY for now (Jon, 2026-07-06): it detects + reports illegal states + emits reproducible seeds/traces; it is NOT wired as a hard CI gate yet** (that would RED on the deferred embed/OOB bugs). SHAPE the seeds/traces so a staged hard gate can be switched on later without redesign (stable seed → replayable trace; a `--deny` mode is a flag flip, not a rewrite). | [opus — oracle written; no design freedom; the GATE-vs-diagnostic switch is Jon's, deferred] |
| CC4 | Broadphase grid for chains+blocks casts (profile first) | [opus; NOT a CC1–CC3 precondition] |
| CC5 | ✅ **LANDED (fable, 2026-07-06).** `engine_core::frame` minted (`PortalFrame {origin, normal, velocity}`, tangent DERIVED, `PortalAperture {frame, half_length}`, explicit `MapConvention`, `map_vec/map_point/map_velocity` incl. Galilean composition); platformer math delegates to the ONE implementation; `pieces::PortalFrame` REPLACED (no shim) — frame-only consumers take `&PortalFrame`, opening-aware take `&PortalAperture` (`PlacedPortal::{frame, aperture}`). Full parity suite green (portal 46, presentation 45, gameplay 1167, app rl_sim). CC6 may now read non-zero `velocity` | done |
| CC6 | ✅ **LANDED (fable, 2026-07-09).** Host-attached frames (`PlacedPortal.host: Option<GeoFaceRef>` + `host_lift`/`vel`/`prev_pos` — the aperture's own sweep sample), engine_core `FaceAnchor` + `World::{block_by_id, resolve_face, attribute_face}`, platforms stamp `GeoSource::Placement`, the RELATIVE swept trigger (body sample shifted by the aperture's frame delta — the scoop works), Galilean `map(v−v_enter)+v_exit` with the min-exit floor in the EXIT REST frame, host-carried motion exempt from eviction (close-only pushout preserved), lazy content-side attribution + per-frame re-derivation (portal closes with its host face). Full parity gate green incl. the `--features portal` content suite. **Amendments recorded below (§5-P2a).** | done |
| CC7 | P3a angled math/authoring → then P3b straddle-pieces + P4 dynamic straddle (post-demo, P3b gated on S3) | [opus] |
| CC8 | AABB slope vocabulary (S2 rules, pinned) — only when a demo/content demands it | [opus] |

**The minimum-slice separation (explicit):** CC1–CC3 are the CCD doctrine's
exit and depend on NOTHING in §4/§5 — not moving portals, not angled
portals, not `SurfacePolygon`, not broadphase, not slopes. Those are
capability tracks that BUILD ON the contracts; an executor on CC1–CC3 who
finds themself blocked by one of them has mis-read a dependency — stop and
re-check §3.

Exit for the doctrine: CC1–CC3 landed and the fuzz rig runs green over all
shipped rooms **as a diagnostic** (Jon 2026-07-06 — hard CI gating is a
deferred, deliberate switch, not a CC3 precondition); §7.6-style bugs
become impossible to write without failing review (an unswept
path-dependent reader without a `discrete_ok` marker is a flagged pattern,
same tier as `AMBITION_REVIEW(spatial)`).
