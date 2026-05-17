# ADR 0007: Use Avian2D for secondary physics while keeping the player controller custom

## Status

Accepted direction; implementation remains incremental.

## Decision

Use Avian2D as a secondary physics backend for dynamic props, debris, ragdoll-like chunks, breakable effects, and experiments.

Do **not** move the primary player controller to Avian by default. The player controller remains custom kinematic gameplay code because Ambition's movement identity depends on explicit platformer semantics: coyote time, buffered input, dash, blink, pogo, wall behavior, body modes, and collision-safe resizing.

Expose backend-neutral physics intent in `ambition_engine` only where it helps gameplay data describe physical effects without depending directly on sandbox presentation adapters.

## Context

Ambition needs physical secondary motion, but the core player controller is not a generic rigid-body problem. Earlier notes about Avian and Parry were patch-era scaffolding; the current trusted system entry point is `docs/systems/collision-geometry-and-secondary-physics.md`.

## Consequences

- Room solids may be mirrored as static colliders for debris/props where useful.
- Breakables, enemies, bosses, and props may spawn dynamic secondary physics bodies.
- The default player collision path remains custom/kinematic.
- Coordinate conversion between Ambition, Bevy, LDtk, and Avian is spatial-review-sensitive.

## Current implications for agents

- Do not replace player movement with Avian unless a future ADR explicitly changes this decision.
- Keep secondary physics presentation separate from primary collision correctness.
- Search `dev/` for movement/collision lessons before editing geometry code.
- Add `AMBITION_REVIEW(spatial): ...` near coordinate conversions that are plausible but hard to prove.
