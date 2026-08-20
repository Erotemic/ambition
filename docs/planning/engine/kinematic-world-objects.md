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

### K2 — typed authored motion — PARTIAL

The ambiguous optional-field precedence is now gone: LDtk conversion classifies
into a validated `MovingPlatformMotionSpec` and rejects conflicting motion fields.
The remaining half is identity: path motion still carries a string `path_id`;
move the relationship to typed/native reference authoring when a real authored
path customer justifies that slice.

### K3 — isolate dynamic geometry ownership — CLOSED 2026-08-14

The monolith no longer re-exports world-owned platform state. `world::platforms`
handed out `MovingPlatformSpec`, `MovingPlatformState`,
`moving_platforms_for_room` and `world_with_moving_platforms` under an
actor-monolith path, so five consumers (provider room lifecycle, room-transition
commit, `sim_view` facts, portal host adapter, and app-side call sites) reached
through the monolith for world-owned state; deleting the re-export found and
repointed all of them at `ambition_platformer2d_world::platforms` directly. What
legitimately stays in the monolith: `MovingPlatformVisual` and its spawn/sync
systems (they name Bevy sprite/lifecycle types). No Cargo edge disappeared —
every repointed crate already depended on the world crate — this removed a false
authority, not a compile unit.

The visual carve was blocked by a transaction, not a dependency:
`spawn_moving_platforms` ran inside room-construction's commit, installing state
and visuals together on purpose — `sync_moving_platform`'s own doc records a
prior bug where a hidden second authority clobbered freshly restored platform
state after a room-change reset. Making the spawn reactive naively would have
reintroduced that bug. The actual fix: every other room feature is already drawn
reactively from a published view (`ambition_render::rendering::features` even
draws a marker for any unclaimed published id), and the moving platform was the
only authored room geometry spawned directly by construction instead of
publishing a row. Publishing platform rows the same way let
`ambition_render::rendering::moving_platforms` reconcile visuals from
`MovingPlatformSet` (spawn/retire/move), and `MovingPlatformVisual`,
`spawn_moving_platform(s)`, and `sync_moving_platform` were deleted from the actor
monolith, along with the spawn call in `spawn_contents`, the app-side dressing
call, and the rollback-coverage waiver that existed only to excuse the component.
A pure reconcile has nothing to remember, so it cannot clobber a restored set —
the guarding test drives the platform set to a new position with no event (what a
rollback restore or room change looks like from here) and asserts the visual
follows rather than sitting at its authored start.

Step 4, the explicit dynamic-geometry query, closed the same day as an adoption:
`CollisionWorld` already was that query; three readers had not adopted it, and
one was a live bug. The blink preview (`sim_view::rebuild_blink_preview_fact`)
composed `world_with_moving_platforms` under a comment claiming that was what
blink resolves against — true when written, false at measurement time, since the
body actually integrates against `world_with_sandbox_solids` (ECS overlay +
portal carves too), so the reticle could point through a lock wall the blink
stops at. Both now read `CollisionWorld::solids()`. The portal host adapter's
distinct real need (uncarved authored + movers, no ECS solids) is now named
`CollisionWorld::hostable_surfaces()`; `hostable_view` is deleted.

✔ No consumer composes a collision world by hand anymore. `CollisionWorld`'s
shape is the four questions the game asks: `solids`, `carves_only`,
`hostable_surfaces`, `base`.

### K4 — contact completeness

Consolidate ride/ledge behavior and make crush/one-way policy explicit.

### K5 — authoring polish

Make the vertical slice pleasant in LDtk and the tooling, including visible path
and semantic diagnostics.

### K6 — second consumer test — CLOSED 2026-08-15

**There IS a second customer. It is the door, it has been shipping for months,
and it is not kinematic** — which is the answer this phase existed to get, so K6
closes on the evidence rather than on an adoption.

#### The census: every dynamic world-geometry producer at HEAD

Read across `crates/` and `game/`, production sites only. "Channel" is how the
geometry reaches a collision read-path; "changes" is what varies frame to frame.

| producer | channel | changes | position owner | `Block::velocity` | rollback |
| --- | --- | --- | --- | --- | --- |
| moving platforms (`MovingPlatformState::as_collision_block`) | composited into `CollisionWorld` | TRANSFORM | `MovingPlatformState` | `last_delta` | serde snapshot state |
| encounter lock walls (`contribute_encounter_lock_walls`) | `FeatureEcsWorldOverlay::gate_solids` | EXISTENCE | authored `LockWallSpec` | `ZERO` | derived per frame |
| intro flag-gated lock walls (`sync_intro_flag_gated_lock_walls`) | `gate_solids` | EXISTENCE | authored `LockWall` placement | `ZERO` | derived per frame |
| falling sand / liquid (`project_particles_to_movement_world`, `falling_sand_sim`) | `gate_solids` + `water_regions` | EXISTENCE per TILE | the particle grid | `ZERO` | grid is sim state, projection derived |
| breakables + world-pogo targets (`rebuild_feature_ecs_world_overlay`) | `overlay.blocks` | EXISTENCE | authored `CenteredAabb` | `ZERO` | derived per frame |
| broken bricks / monitors, gnu_ton ladder-floor gate, Mary-O discovered hidden blocks | `removed_block_names` (+ a replacement `Solid`) | EXISTENCE / KIND | authored placement | `ZERO` | derived per frame |
| portal apertures / gnu_ton climbable carve | `portal_carves` / `climbable_carves` | EXISTENCE (subtraction) | portal placement | n/a | derived per frame |
| struck-block flinch (`block_nudge`) | none — render only | drawn quad offset | nothing; geometry is static | n/a | explicitly not sim state |

And the things that genuinely MOVE a world-owned box per frame and are **not
solid**, so none of them wants the moving-solid contract:

| producer | what moves | driver | solid? |
| --- | --- | --- | --- |
| patrolling hazard volumes (`update_ecs_hazards`) | `CenteredAabb` centre AND half-size, every frame | `PathMotion` over a `KinematicPath` | no — damage only |
| the cut-rope falling anvil (`FallingHazard` in `encounter_script`) | a falling world volume | integration | no — damage only |
| oscillating gravity zones (`oscillate_gravity_zones`) | `zone.aabb` | authored oscillation | no — a field region |
| hosted portal apertures (`refresh_hosted_portal_frames`) | aperture `pos`, and `vel` from the host's `anchor.velocity / dt` | the host block's authoritative displacement | no — a subtraction |

⚠ **and one solid whose surface moves while its `velocity` is `ZERO`:**
`SettledSandLedger::blocks()` emits a `Block::one_way` per dense tile whose HEIGHT
is proportional to fill, so a growing pile's top face rises every frame. It is not
a mixed frame and not a bug — the rising face is a *differently sized block at the
same tile*, not one block moving — but it is the sharpest statement of the
distinction this phase found: **geometry can move without being kinematic.**

⭐ **`MovingPlatformState` is the ONLY site that originates a non-zero
`Block::velocity` anywhere in the repo** (`platforms/mod.rs`, one line;
`boundary_chain` and the portal carve only propagate it). Every other dynamic
SOLID toggles existence or kind at a fixed place, and everything that genuinely
moves per frame is not solid. That is not a shortage of instances — it is a
shortage of KIND, and it is why a second kinematic customer cannot be found by
looking harder.

#### The three candidates the plan named

- ⛔ **moving door/wall — REJECTED, and it is the one with real content.** The door
  exists: 3 authored `LockWall` entities across `intro.ldtk` and `sandbox.ldtk` —
  `goblin_encounter_lock` sealed by the encounter lifecycle, and
  `alice_private_return_lock` / `gate_alice_private_lock` sealed by save flags
  through `INTRO_FLAG_GATED_LOCK_WALLS` — so TWO independent contributors already
  derive it. It **appears**; it does not **slide**. ⚠ and the LDtk `LockWall`
  entity is inert on its own: each game wires the condition, which is the
  authoring gap, not a kinematic one. Adopting the mover would give it a driver
  for no motion, promote a per-frame *derived* resource into serde snapshot state
  (a rollback regression bought with nothing), and delete nothing — the whole
  feature is `desired_lock_wall_blocks`, ten lines. The sliding door that WOULD be
  kinematic has zero authored instances.
- ⛔ **conveyor-like solid — REJECTED for want of content, but it is the
  FALSIFIER.** The word appears twice at HEAD and neither is a new customer:
  `VerticalLoop::anchor_y`'s doc calls a run of anchored loops *"a conveyor of
  lifts"* (3 authored — Mary-O's lifts A/B/C, already this representation), and
  `ForceZone`'s doc calls a wind volume a *"conveyor updraft"* (an acceleration
  field on bodies, not geometry). A **belt** — a stationary solid that drags what
  stands on it — is authored nowhere. See the falsifier below before anyone
  authors one.
- ⛔ **Smash stage platform — REJECTED as an invented customer.** `smash_stage()`
  is a single static `ae::Block`, and that crate's own doc argues the stage is four
  numbers whose one interesting fact is the blast margin. Nothing in the demo wants
  a moving platform, and a stage that moves is a design decision nobody has made.
- ⭐ **falling sand — the real second customer the plan did not name, and it must
  stay separate.** Real authored content (`sandbox.ldtk falling_sand_room` plus 4
  authored `Switch` spouts), genuinely dynamic solid geometry, and materially
  different in every respect that matters: it is a FIELD, not a BODY. Its solid is
  a per-tile lattice re-derived each frame from a particle grid, so a tile has no
  identity across frames and `last_delta` / `previous_aabb` / stable portal host /
  ledge carry are all meaningless for it. Its authoritative position owner is the
  grid; the geometry is a pure projection. Forcing it into a mover would be the
  false universal K6 warns about.

#### ⛔⛔ The falsifier, recorded so nobody adds a `bool` instead

**`Block::velocity` means two things at once, and only a belt can tell them
apart.** For a moving platform the solid's per-frame DISPLACEMENT and the DRAG it
imparts to a rider are the same vector, so one field has carried both:

- the sweep reads it as rider carry (`Contact::surface_velocity`);
- `ledge_grab::ledge_carry_for_frame` selects a carrier by `velocity != ZERO` and
  recovers the previous pose as `block.aabb.translated(-block.velocity)`;
- `MovingPlatformState::previous_aabb` is the world-side sibling of that line.

A belt has displacement `ZERO` and drag non-zero. Authored as
`Block { velocity: drag }` today it would be selected as a ledge CARRIER (dragging
a body merely hanging off its lip) and be assigned a previous pose it never
occupied. ⇒ **the day a belt is authored, `velocity` splits into `displacement`
(defines the previous pose, drives the carrier test) and `surface_drag` (what a
supported body inherits)** — before any new `BlockKind` or authoring field.

#### ⭐ That is TWO abstractions, and only one lacks a second customer

The census splits the thing this plan has been calling "the kinematic
representation" at a seam already visible in the content:

- **the motion DRIVER — `KinematicPath` / `PathMotion` — is already proven by
  three consumers**: moving platforms (`MovingPlatformMotionSpec::Path`), damage
  volumes (`HazardFeature::new_with_paths` resolves a `path_id` into `PathMotion`),
  and enemy patrol brains. ⛔ **and its only AUTHORED customer is a brain**: the
  corpus holds 2 `KinematicPath` entities, both reached from `EnemySpawn.path_ref`;
  no `MovingPlatform` and no `DamageVolume` authors a path anywhere. So the driver
  needs no second customer — it needs authored content for the two geometry
  consumers it already has.
- **the moving-SOLID contact contract — `Block::velocity` plus everything derived
  from it (rider carry, ledge carry, previous pose, portal host velocity) — has
  exactly ONE consumer**, and the census above says there is no candidate in kind,
  not merely no second instance.

⇒ K6's question was only ever about the second bullet, and the answer is no.

⚠ **also measured, because the plan's pipeline diagram could be read to assume
it: `WorldDelta` DOES NOT EXIST in code.** Only the reserved
`GeoSource::Delta { op_index }` variant and aspirational doc references; the
runtime substitute is the immutable authored base plus per-frame recomposition,
and nothing anywhere mutates a `Block::aabb` or a `SurfaceChain`'s points after
room construction. Do not plan against a delta-op road that has no traveller.

#### What K6 actually proved

The abstraction that two materially different uses DO prove is not
`MovingPlatformState` — it is the **collision composition seam**:
`FeatureEcsWorldOverlay` plus `CollisionWorld`'s four named views carry a moving
transform, three kinds of existence gate, a particle field and two subtractions,
with no producer editing the authored base. That seam is proven. The kinematic
representation has one customer and should keep exactly one until a moving door,
a belt, or a rotating stage is AUTHORED — at which point the belt case above is
already scoped.

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
