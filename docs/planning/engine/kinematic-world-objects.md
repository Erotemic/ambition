# Kinematic world objects

**State:** RESTING — **K2–K6 are all closed** (K5's native `path_ref` landed
2026-08-15). ⛔ reopen only for a real kinematic customer; ⛔⛔ **and split
`Block::velocity` into displacement + surface drag BEFORE any conveyor-like one**.
**Sole kinematic customer: Ambition moving platforms, and that is MEASURED rather
than assumed** — see K6 for the census that closed the second-customer question.

## Why this exists

Moving platforms work today, but they still expose transitional architecture.
The engine should treat deterministic moving world geometry as a first-class
world/simulation capability, not as a special feature hanging off the actor
monolith.

## Source-backed current state

The current implementation is more capable than some old docs imply:

- `MovingPlatformSpec` and `MovingPlatformState` live in
  `ambition_platformer2d_world::platforms`;
- LDtk emits `MovingPlatformSpec` and room preparation resolves optional
  `KinematicPathSpec` references;
- runtime state is serializable and participates in the live
  `MovingPlatformSet`;
- platforms advance once per frame and expose `last_delta` so riders/ledge
  contacts can be carried without advancing the platform per actor;
- collision composition inserts current moving-platform AABBs as solids;
- portals can attach to identified moving platform faces;
- the Bevy visual is a read-model projection of authoritative platform state.

But the module comment still calls the feature a design experiment, and several
boundaries are not engine-1.0 quality:

- the Bevy visual adapter remains under the actor monolith;
- provider lifecycle reaches through that monolith adapter to obtain platform
  state even though the world crate owns the model;
- `world_with_moving_platforms` builds a temporary collision world around the
  static room representation;
- motion authoring is an implicit precedence of `path > loop > sweep` across a
  bag of optional fields;
- the path relation is string-based even though LDtk `EntityRef` tooling exists;
- `KinematicPath` point authoring is a coordinate string;
- crush, one-way, passenger and moving-surface interaction policy is not yet a
  deliberate general contract.

## Target boundary

The engine needs a narrow concept such as **moving/kinematic world geometry**.
Do not generalize every dynamic object into it. A platform belongs here because
its authoritative identity is world geometry whose transform follows a
deterministic motion driver.

Conceptually:

```text
Authored moving solid
    stable spatial identity
    shape / collision policy
    motion specification
    presentation reference
        |
        v preparation
Resolved kinematic solid
    resolved path / typed refs
    validated motion
        |
        v simulation
Kinematic world state
    transform / previous transform / velocity-delta
        |
        +--> collision/contact query
        +--> passenger/ledge carry
        +--> portal host transform
        +--> sim-view / renderer
```

Exact type names should follow the carve, not precede it.

## Motion model

The current three useful behaviors are real product semantics:

- ping-pong sweep;
- path traversal (`Once`, `Loop`, `PingPong`);
- discontinuous wrapping vertical loop for paternoster/infinite-elevator effects.

Make them explicit variants or lower simple authoring sugar into a canonical
motion representation. Do not keep semantic precedence as "which optional field
happened to be filled in".

A discontinuous wrap is not necessarily the same thing as a continuously closed
path; preserve that distinction.

## Contact semantics to settle

### Riding / surface velocity

A supported body resting on a moving solid should inherit the surface delta via
one contact model. Existing carrying tests are useful evidence; consolidate the
behavior rather than adding actor-family exceptions.

### Ledge contacts

A latched ledge contact should remain attached to the same moving spatial
feature while valid. Prefer stable `GeoId`/feature identity over rediscovering by
position.

### One-way platforms

If moving one-way platforms are supported, one-way is a contact policy on the
moving surface rather than a second movement kernel.

### Crushing

Specify what happens when a kinematic solid closes space against another solid.
Possible rules include lethal crush, forced displacement where a legal path
exists, or capability-specific behavior. The engine must not silently depend on
pushout iteration order.

### Portals and attached objects

Portals already ride moving platform faces. Preserve this as a first-class
consumer of stable moving-geometry identity rather than a special name lookup.

## Ownership carve

1. Keep authored/runtime kinematic state in the world/simulation domain.
2. Move visual spawning/sync to presentation/render ownership when dependency
   boundaries permit it.
3. Remove actor-monolith dependency from provider/world lifecycle paths that only
   need world-owned platform state.
4. Give collision an explicit dynamic-geometry overlay/query rather than
   repeatedly treating moving geometry as an ad-hoc reconstructed static world
   if measurement shows that is the cleaner boundary.
5. Keep kinematic state scoped to its world/room partition so future multi-room
   residency can simulate the moving geometry needed by each resident partition
   without promoting one room's platform set to process-global authority.

## LDtk authoring slice

Use [`ldtk-authoring-and-world-tools.md`](ldtk-authoring-and-world-tools.md):

- native/typed path reference;
- visible path editing;
- explicit motion mode;
- validation for mode-specific fields;
- semantic room preview showing path and direction.

## Phases

### K1 — re-measure and write the contract

Inventory every production consumer of `MovingPlatformSet`, platform
`last_delta`, `world_with_moving_platforms` and platform identity. Record which
behaviors are authoritative versus presentation.

**Measured 2026-08-14.** Two results worth carrying forward:

- **Authoring census: 7 platforms across all six worlds** — 4 sweeps, 3 anchored
  vertical loops, and **zero** authoring `path_id`. The path-following mode has
  no content customer, so `MovingPlatform -> KinematicPath` is a weak choice for
  L2's "first proof". The live string relation is `EnemySpawn.brain =
  "Patrol:<id>"`, resolved against a slug of the path's authored name — that is
  where typed references would pay. Authored motion is now a validated
  `MovingPlatformMotionSpec` rather than a precedence over optional fields.
- **The platforms did not move at all in a session with no home avatar.**
  `advance_moving_platforms` read the primary player's hitstop through a
  `single()` that returned early when there was none, so every match froze its
  moving geometry. Fixed under D117 (the hitstop was a duplicate of the global
  clock the same body already drives), which is the shape of the coupling this
  plan predicts: platform ownership questions keep resolving into actor/body
  questions, and are cheaper to answer after the actor kernel is coherent.

### K2 — typed authored motion — CLOSED (marker corrected 780295052, 2026-09-02)

The ambiguous optional-field precedence is now gone: LDtk conversion classifies
into a validated `MovingPlatformMotionSpec` and rejects conflicting motion fields.

⚠ **This section said PARTIAL and contradicted the page header, which says
K2-K6 are all closed. The header was right and this marker was stale.** The
"remaining half" it described — *"path motion still carries a string `path_id`;
move the relationship to typed/native reference authoring"* — was satisfied at
the layer that matters. Authoring is a NATIVE LDtk `EntityRef`: `path_ref`, read
through `LdtkEntityCtx::kinematic_path_ref` and used by
`conversion/entity_converters.rs`, with the path index built before any
conversion so a reference may name a path authored later in the file.

⛔ `MovingPlatformMotionSpec::Path` does still hold a `String`, and that is not
the same defect. The reference is typed where it is AUTHORED and resolved where
it is CONSUMED, which is the ordinary shape of a resolved reference — not the
"ambiguous optional field" this item existed to remove. Do not reopen K2 for the
runtime string alone.

### K3 — isolate dynamic geometry ownership — CLOSED 2026-08-14

The monolith no longer re-exports world-owned platform state; five consumers
(provider room lifecycle, room-transition commit, `sim_view` facts, portal host
adapter, app-side call sites) were repointed at
`ambition_platformer2d_world::platforms` directly. What legitimately stays in
the monolith: `MovingPlatformVisual` and its spawn/sync systems (they name Bevy
sprite/lifecycle types).

⚠ the visual carve is blocked by a transaction, not a dependency:
`sync_moving_platform_visuals`'s own doc (renamed since this row was written;
`ambition_render/src/rendering/moving_platforms.rs:46`) records a prior bug where a hidden second
authority clobbered freshly restored platform state after a room-change reset,
so making the spawn reactive naively would reintroduce it. The fix: platform
rows are now PUBLISHED like every other room feature, so
`ambition_render::rendering::moving_platforms` reconciles visuals from
`MovingPlatformSet` (spawn/retire/move) instead of construction spawning
visuals directly — a pure reconcile has nothing to remember, so it cannot
clobber a restored set. Pinned by a test that moves the platform set to a new
position with no event and asserts the visual follows.

Step 4, the explicit dynamic-geometry query, closed the same day: `CollisionWorld`
already was that query; three readers had not adopted it, one was a live bug
(`sim_view::rebuild_blink_preview_fact` composed a stale collision world under a
comment that no longer matched what the body actually integrates against, so
the blink reticle could point through a wall). All three now read
`CollisionWorld::solids()`; the portal host adapter's distinct need is
`CollisionWorld::hostable_surfaces()`.

✔ No consumer composes a collision world by hand anymore. `CollisionWorld`'s
shape is the four questions the game asks: `solids`, `carves_only`,
`hostable_surfaces`, `base`.

### K4 — contact completeness — CLOSED 2026-08-15 (marker added 780295052, 2026-09-02)

Consolidate ride/ledge behavior and make crush/one-way policy explicit.

⚠ **This section carried NO status marker at all while the page header counted
it among the closed items.** Closed by `7f6c9a6a4`, "K4: riding a ledge was the
one contact rule only the player could get", whose own message states the item
verbatim and records that censusing HEAD dissolved most of it: passenger carry
was already unified and explicit through `Block::velocity`. Verified at HEAD —
one-way policy is explicit in the type system (`BlockKind::OneWay`, with its
struck-from-below mirror), crush is an explicit reported event
(`movement/events.rs`: the body is over-constrained between two surfaces;
`movement/integration.rs` and `adhesive_crawler.rs` report rather than resolve
it), and rider carry is pinned by
`a_wrapping_platform_carries_a_rider_by_its_travel_not_by_its_teleport`.

### K5 — authoring polish

Make the vertical slice pleasant in LDtk and the tooling, including visible path
and semantic diagnostics.

### K6 — second consumer test — CLOSED 2026-08-15

**There IS a second customer — the door — and it is not kinematic.** It is
sealed by appearing (an existence toggle in `gate_solids`), not by sliding: 3
authored `LockWall` entities, with two independent contributors already
deriving it from encounter/save-flag state.

A census of every dynamic world-geometry producer at HEAD (`crates/`, `game/`)
found `MovingPlatformState::as_collision_block` is the **only** site anywhere
that originates a non-zero `Block::velocity`. Every other dynamic solid (lock
walls, falling sand/liquid, breakables, portal carves) toggles existence/kind
at a fixed place; everything that genuinely moves per frame (patrolling hazard
volumes, the cut-rope anvil, gravity zones, hosted portal apertures) is not
solid. One apparent exception is not a bug: `SettledSandLedger` emits a taller
`Block::one_way` as a pile fills — a differently-sized block at the same tile,
not one block moving. **Geometry can move without being kinematic** — this is
a shortage of KIND, not of instances.

Three named candidates were rejected:
- **moving door/wall** — real content (3 `LockWall` entities), but it
  *appears*, it does not *slide*; adopting the mover would promote a per-frame
  derived resource into serde snapshot state for no motion gained.
- **conveyor-like solid** — the word "conveyor" appears twice in doc comments,
  neither naming an authored belt; a stationary solid that drags what stands
  on it is authored nowhere.
- **Smash stage platform** — `smash_stage()` is a single static block; a
  moving stage is a design decision nobody has made.

Falling sand (`sandbox.ldtk falling_sand_room`) is real dynamic solid content
but is a FIELD, not a BODY — a per-tile lattice re-derived from a particle grid
each frame, with no tile identity across frames — so forcing it into the mover
contract would be the false universal this phase warns against.

#### ⛔⛔ The falsifier, recorded so nobody adds a `bool` instead

`Block::velocity` means two things at once — the moving solid's per-frame
DISPLACEMENT, and the DRAG it imparts to a rider — and only a belt (zero
displacement, non-zero drag) can tell them apart: the sweep reads it as rider
carry, and `ledge_grab::ledge_carry_for_frame` selects a carrier by
`velocity != ZERO` and recovers the previous pose as
`block.aabb.translated(-block.velocity)`. Authored as `Block { velocity: drag
}` today, a belt would be wrongly selected as a ledge carrier and assigned a
previous pose it never occupied. **The day a belt is authored, `velocity`
splits into `displacement` (defines the previous pose, drives the carrier
test) and `surface_drag` (what a supported body inherits)** — before any new
`BlockKind` or authoring field.

The motion DRIVER (`KinematicPath`/`PathMotion`) is a separate abstraction
from the moving-SOLID contact contract and already has three consumers (moving
platforms, damage volumes, enemy patrol brains); its only AUTHORED customer is
a brain (2 `KinematicPath` entities, both from `EnemySpawn.path_ref`). The
moving-solid contract (`Block::velocity` and everything derived from it) has
exactly one consumer, and the census found no candidate in kind — that is K6's
answer.

⚠ `WorldDelta` DOES NOT EXIST in code — only the reserved `GeoSource::Delta {
op_index }` variant. The runtime substitute is the immutable authored base plus
per-frame recomposition; nothing mutates a `Block::aabb` or a `SurfaceChain`'s
points after room construction. Do not plan against a delta-op road that has no
traveller.

What K6 did prove is the **collision composition seam**: `FeatureEcsWorldOverlay`
plus `CollisionWorld`'s four named views already carry a moving transform,
three kinds of existence gate, a particle field and two subtractions, with no
producer editing the authored base. The kinematic representation has one
customer and should keep exactly one until a moving door, a belt, or a
rotating stage is AUTHORED — at which point the belt case above is already
scoped.

⇒ **no further work is owed here.** ⛔ do not re-open K6 to go find a customer;
re-open it when content arrives.

## Acceptance

- an Ambition level author can build a complex moving platform entirely through
  supported LDtk/tooling surfaces;
- runtime collision, passenger carry, attached portal behavior and rollback use
  one authoritative moving transform;
- no actor-family special case is needed to ride it;
- the world/provider path does not depend on the actor monolith merely to obtain
  moving-platform state;
- invalid path/motion authoring fails during preparation with useful provenance;
- the capability remains usable by another game without Ambition-specific code.
