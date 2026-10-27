# ADR 0036: Synchronization and rollback are optional game capabilities

## Status

**Proposed architecture (2026-10-26); product requirements recorded.** Deterministic gameplay
remains a priority; the Smash demo needs rollback; other engine consumers must not be required
to use or link a rollback backend.

## Context

`SimulationHost::{RenderFrame, Fixed60Hz, Rollback}` currently combines stepping policy with
synchronization policy. `ambition_platformer2d_rollback_ggrs` is a separate crate, but the
flagship composition selects rollback and the facade's default `all_capabilities` includes the
feature. A game's simulation should not be forced to save and replay a large persistent world
solely to gain deterministic fixed-step gameplay.

## Proposed decision

- **Deterministic, fixed-step simulation is the base capability**, independent of network
  transport, GGRS, snapshot history, or presentation frame rate. Retain the same-build guarantee
  in [ADR 0023](0023-same-build-determinism.md).
- Model **tick scheduling**, **synchronization backend**, and **rollback population/extent** as
  distinct configuration choices. Do not multiply implementations of gameplay laws.
- A new game's minimum profile defaults to deterministic fixed-step gameplay with **no rollback
  backend installed or required in its resolved Cargo closure**. A convenience/full-content
  profile may explicitly opt into more.
- Supported synchronization candidates include offline local stepping, bounded input lockstep,
  GGRS P2P rollback, and peer-hosted authoritative state replication. A given game selects only
  what it needs.
- **Smash** is the primary bounded-rollback consumer. **Mary-O/Sanic** should evaluate
  level-scope synchronization with the level ending after every required participant reaches its
  goal. **Ambition** needs persistent, multi-room co-op where ordinary travel need not rebase an
  entire world history.
- A simulation owner publishes confirmed, externally visible effects according to its
  synchronization mode; an offline fixed-step host does not have to manufacture GGRS
  confirmation objects.
- Domain-level stable identity, deterministic event ordering, and explicit state ownership
  remain useful without rollback. Backend-neutral rewind declarations may exist but must not
  cause mandatory GGRS dependency or per-tick snapshot work in non-rollback profiles.
- Neither the `GgrsSchedule` nor the currently shipped all-world GGRS session is the universal
  execution contract for all games.

## Tradeoffs and unresolved implementation

A configurable tick frequency changes gameplay semantics unless mechanical durations and
integration rules are audited. Host-authoritative replication needs a different state-transfer
path from GGRS input exchange. Runtime switching between synchronization modes, especially when
players join or leave a region, is **not approved or promised** by this ADR.

## Acceptance experiments

- Headless and windowed minimum games run deterministic fixed-step simulation and compile
  without GGRS, renderer optional where relevant.
- Smash uses a real two-peer rollback game without loading Ambition's persistent-world
  infrastructure.
- Same gameplay kernel executes under at least a fixed-step host and a bounded GGRS host with
  equivalent mechanical outcomes for controlled test inputs.
- Measure resolved Cargo closure, active systems, CPU, memory, saved state, resimulated ticks
  and latency instead of inferring costs from the presence of schema declarations.

See [ADR 0027](0027-ggrs-is-the-sole-rollback-authority.md), [ADR 0037](0037-causally-isolated-simulation-islands.md), and [capability planning](../planning/engine/capability-and-runtime-composition.md).
