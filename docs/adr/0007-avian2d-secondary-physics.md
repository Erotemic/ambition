# ADR 0007: Use Avian2D for secondary physics while keeping the player controller custom

## Status

Accepted direction; implementation remains incremental.

## Decision

Avian2D is an **optional** secondary physics backend for dynamic props, debris, ragdoll-like chunks, breakable effects, and experiments; an engine consumer that needs only placeable props or deterministic kinematic movement must not be forced to install it.

Do **not** move the primary player controller to Avian by default. The player controller remains custom kinematic gameplay code because Ambition's movement identity depends on explicit platformer semantics: coyote time, buffered input, dash, blink, pogo, wall behavior, body modes, and collision-safe resizing.

Expose backend-neutral physics intent in `crates/ambition_platformer2d_core/src/` only where it helps gameplay data describe physical effects without depending directly on sandbox presentation adapters.

## Context

Ambition needs physical secondary motion, but the core player controller is not a generic rigid-body problem. Earlier notes about Avian and Parry were patch-era scaffolding; the current trusted system entry point is `docs/systems/collision-geometry-and-secondary-physics.md`.

## Consequences

- Room solids may be mirrored as static colliders for debris/props where useful.
- Breakables, enemies, bosses, and props may spawn dynamic secondary physics bodies.
- The default player collision path remains custom/kinematic.
- Coordinate conversion between Ambition, Bevy, LDtk, and Avian is spatial-review-sensitive.

## 2026-10-26 scope amendment: physics is not object identity

[ADR 0038](0038-persistent-objects-and-optional-physics.md) proposes a distinct persistent-object contract for carrying, placing, custody, and location. A simple solver can service those interactions without Avian. Avian rigid-body props and articulated ragdolls may be authoritative *only when their motion affects gameplay*, including switches, collision, carry or platforming; otherwise local ragdoll/debris motion is presentation and may be disabled or simplified per device.

This is a clarification of the accepted custom player-controller and optional secondary-physics direction, not a claim that the generalized object or authoritative ragdoll protocol has shipped. A client quality setting may not disable authoritative physics independently of the host or other peers. Region-specific physics and rollback isolation still require explicit evidence.

## Current implications for agents

- Do not replace player movement with Avian unless a future ADR explicitly changes this decision.
- Keep secondary physics presentation separate from primary collision correctness.
- Search `dev/` for movement/collision lessons before editing geometry code.
- Add `AMBITION_REVIEW(spatial): ...` near coordinate conversions that are plausible but hard to prove.
