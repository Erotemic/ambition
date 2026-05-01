# ADR 0009: World composition and level authoring

## Status

Accepted as architecture direction. Implementation is pending.

## Context

Ambition is intended to be more than a single game prototype. It should become a
package capable of supporting large 2D platformers/metroidvanias, generated
worlds, curated campaign spaces, and sandbox proofs-of-concept. The world system
therefore needs professional-scale composition instead of relying on a single
flat room format forever.

A recent basement-design clarification exposed the issue. A basement below the
central hub should be a physically connected space that the player can drop into,
not a new loading-zone room. In general, room boundaries used by designers or
procedural generators should not necessarily be the same as boundaries perceived
by the player.

Professional games usually distinguish authoring units from runtime traversal
units:

```text
Authoring unit:  room chunk, section, prefab, level, screen, generated module
Runtime unit:    active area, loaded region, world composition, streamed set
Player feel:     continuous space unless there is an intentional transition
```

Ambition currently uses RON-authored sandbox rooms and loading zones. That is a
good proving ground, but it is not enough as the only long-term authoring model
for massive games.

## Decision

Create an Ambition-owned world-composition layer. The project should not treat
"one room file" as the fundamental world unit. Instead, use these concepts:

```text
RoomChunk
  A locally authored piece of playable space. A chunk may come from RON, LDtk,
  generated data, tests, or another importer. It owns local geometry, actors,
  hazards, pickups, labels, camera hints, and metadata.

PlacedRoomChunk
  A RoomChunk with a transform/offset inside a larger composed area.

ActiveArea
  The runtime world region currently loaded and simulated as one continuous
  space. It may contain one or many PlacedRoomChunks.

LoadingZone
  An intentional transition between ActiveAreas, doors, portals, entrances,
  exits, or story transitions. A LoadingZone is not the mechanism for ordinary
  adjacent stitched chunks.
```

The central hub basement should be represented as a composed active area, not a
separate loading-zone destination:

```text
ActiveArea: central_hub_complex
  chunks:
    central_hub_main      at (0, 0)
    central_hub_basement  at (0, -N)
    optional side sections/labs at authored offsets
```

The player should be able to move continuously across stitched chunk boundaries.
No transition state, fade, teleport, spawn repair, or loading-zone prompt should
occur at a stitch seam. Collision, hazards, actors, pickups, debug labels,
platforms, camera constraints, and room metadata should be transformed into the
active-area coordinate frame.

## External authoring tools

Ambition should remain code-first and data-driven, but it should not force all
large authored spaces to be edited directly as hand-written RON. Use external
level editors when they reduce friction without surrendering the engine's schema.

Adopt this policy:

1. **Ambition schema is canonical.**
   The engine/sandbox should own semantic concepts such as `DamageVolume`,
   `Actor`, `Interactable`, `Pickup`, `Breakable`, `RespawnPolicy`, and future
   `RoomChunk`/`ActiveArea` data. External editors feed this schema through
   adapters; they do not define engine semantics directly.

2. **RON remains the stable test and generated-data format.**
   RON is still useful for tests, fixtures, generated rooms, diffs, examples,
   and small handcrafted sandbox cases. Do not delete or abandon RON.

3. **LDtk is the first external editor integration candidate.**
   LDtk is a strong fit for 2D platformer/metroidvania authoring because it has
   grid/tile/entity workflows, custom entity fields, enum fields, and auto-layer
   rules. Evaluate it first for human-authored chunks and a sandbox POC.

4. **Tiled remains a secondary candidate.**
   Tiled is mature and widely used. Keep it available as a future adapter if LDtk
   is insufficient for teams, asset pipelines, or user expectations.

5. **Bevy plugins are adapters, not architecture.**
   `bevy_ecs_ldtk`, `bevy_ecs_tiled`, and `bevy_ecs_tilemap` may be useful, but
   Ambition should avoid hard-coding the world model to any single plugin's
   entity hierarchy. Treat these crates as import/render/load helpers behind
   feature gates or sandbox integration layers until their role is proven.

6. **Generated and authored chunks must share one runtime contract.**
   A generated mathematical room and a hand-authored LDtk chunk should both
   become `RoomChunk`/`PlacedRoomChunk`/`ActiveArea` data before simulation.

## Required proof of concept

The sandbox should demonstrate this architecture, not merely describe it.

The first POC should implement a `central_hub_complex` composed active area where:

- the player starts near the middle of the hub;
- a floor opening allows the player to drop into a basement below;
- the basement is continuously traversable from the hub;
- no loading zone is used between hub and basement;
- debug rendering clearly shows chunk names, local origins, active-area bounds,
  collision, loading zones, and stitch seams;
- a debug overview/zoom-out mode can show the whole composed area;
- spatial assumptions are marked with `AMBITION_REVIEW(spatial)` where needed.

The first implementation may keep the source data in RON while proving the
composition model. LDtk import should follow once the internal runtime contract is
clear enough to avoid cargo-culting an editor plugin's shape.

## Implementation notes

Prefer a load-time flattening/composition step before building streaming.
Streaming and distance-based activation are important for massive games, but they
are not required for the first professional POC.

A practical sequence is:

1. Define pure data types for `RoomChunk`, `PlacedRoomChunk`, `ActiveArea`, bounds,
   chunk origins, and stitch metadata.
2. Convert the existing single-room sandbox format into one `RoomChunk` inside
   one `ActiveArea` so current behavior remains possible.
3. Add a composed central hub active area with a basement chunk below the main hub.
4. Transform all chunk-local objects into active-area/world coordinates during
   loading.
5. Add debug overview camera controls and chunk/stitch visualization.
6. Add validation tests for chunk bounds, duplicate IDs, invalid loading zones,
   bad transforms, overlapping seams where not allowed, and finite/no-NaN
   geometry.
7. Evaluate LDtk import as a sandbox-only or feature-gated adapter.
8. Only add streaming/activation when area size or performance justifies it.

## Consequences

This supersedes any interpretation of "basement" as a separate room entered via a
loading zone. A basement may still be authored as a separate chunk, but it should
be part of the same active area when the intended player experience is dropping
or walking there continuously.

Future agents should not patch around world scale by adding more loading zones
for physically adjacent spaces. Use loading zones for intentional transitions;
use stitched chunks or large active areas for continuous traversal.

This also means the camera system and debug tooling must evolve. Camera bounds
should come from active-area metadata and camera zones, not only a single room's
rectangle. Debug zoom-out/overview is required infrastructure for large or
stitched spaces.

## Alternatives considered

### One huge room only

This is simple and may still be useful for small areas, but it does not scale well
for professional authoring, team workflows, generated modules, reuse, or local
reasoning. It also conflates an editing convenience with the runtime world model.

### Loading-zone rooms only

This is already available, but it produces the wrong player experience for spaces
that are physically adjacent. It should remain for intentional doors, portals,
scene changes, and graph transitions.

### Directly adopt LDtk or Tiled as the canonical format

This would speed up authoring but would make Ambition's engine semantics depend
on an external editor. That is risky for a reusable package intended to support
massive games, generated content, mathematical topology, and custom world models.
Use external tools through importers/adapters instead.

### Full streaming system immediately

This is premature. Ambition needs a correct composition model, validation, debug
visualization, and a sandbox POC before investing in streaming complexity.
