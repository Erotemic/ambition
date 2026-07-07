# 0021: Authoring-backend-agnostic world IR

## Status

Accepted; IMPLEMENTED first cut (2026-07-07). The W3/W4 crate split minted
`ambition_world` for room/placement IR and `ambition_ldtk_map` for the LDtk
backend, with architecture-boundary tests ratcheting the dependency direction.

## Context

Ambition's room data was historically authored in LDtk and materialized by
`gameplay_core::world`, which mixed four concerns: backend parsing, room graph
IR, sim-side room transition systems, and Bevy/LDtk runtime-spine adapters.
That shape made LDtk feel like the world model instead of one authoring backend,
and it made a generated/RON/test backend look like a special case.

The W-track ruling in `docs/planning/engine/decomposition.md` closed the design:
world IR is pure authored input; LDtk and future backends convert into it; the
sim/content layer lowers authored placement records into live entities at room
load.

## Decision

1. **`ambition_world` owns the backend-agnostic IR.** Room graph types
   (`RoomSpec`, `RoomSet`, links/transitions), metadata, loading zones, authored
   placement records, debug labels, and moving-platform math live there.
2. **`ambition_ldtk_map` owns LDtk.** JSON/project parsing, LDtk validation,
   entity conversion, manifest/file loading, hot-reload state, and the
   `bevy_ecs_ldtk` runtime spine live in the backend crate.
3. **Dependency direction is one-way.** LDtk converts into `ambition_world`;
   `ambition_world` never imports LDtk, gameplay-core, app, render, runtime, or
   content. Boundary tests enforce this.
4. **Simulation lowers, it does not author.** The sim heart registers
   `PlacementKind` interpreters and lowers `PlacementRecord` values during room
   load. Unknown placement kinds hard-error with room/id/kind diagnostics.

## Current implications for agents

Add new authored world facts to `ambition_world` first, then teach each backend
to emit them. Do not put LDtk types in world IR. Do not make gameplay-core parse
backend entities directly. If a live entity needs behavior, add or extend a
placement schema and lower it through the registry rather than adding a parallel
spawn channel.
