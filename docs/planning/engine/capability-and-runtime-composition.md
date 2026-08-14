# Capability and runtime composition

**State:** OPEN successor program.

## Goal

Make engine composition reflect what a game actually chooses to use.

A consumer building a small platformer should not inherit portal rendering,
boss orchestration, networking integration, persistence, debug presentation or
Ambition-only content because a broad historical crate happens to sit in the
middle of the dependency graph.

## Principles

- capability selection is a semantic engine API, not a Cargo-feature illusion;
- dependency closure and installed runtime behavior should agree;
- the easy default may install a broad useful engine, while narrow composition
  remains real and tested;
- internal implementation crates are not public capability names;
- a capability owns its data/schema/install declarations close to the domain;
- headless, rendered, desktop and mobile hosts compose from the same capability
  vocabulary with host-specific presentation/services layered on top.

## Current pressure points

- `ambition_platformer2d_actor_monolith` still acts as a composition root for
  several unrelated domains;
- `ambition_platformer2d_shared_tangle` has high fan-in and mixed ownership;
- the runtime still knows domain-specific rollback registrations;
- the public facade can expose both semantic APIs and historical implementation
  topology;
- some optional features are reachable through dependencies even when a
  consumer did not request them.

## Target shape

```text
Game / Experience definition
    + authored content providers
    + capability declarations
    + host services
    + local presentation policy
          |
          v
Prepared capability plan
          |
          +--> headless runtime host
          +--> desktop rendered host
          +--> mobile rendered host
```

The plan is not a second service locator. It records what is being installed,
validates requirements/conflicts and lowers to ordinary Bevy plugins/resources.

## Phases

### C1 — capability inventory from actual consumers

Use Ambition, Mary-O, Sanic, TwinTrack, Smash and the external consumer fixture
to identify capability families that have real independent consumers.

### C2 — choose one leaky capability

Pick a capability whose absence still drags in unrelated crates or runtime
behavior. Carve its declaration/installation boundary and prove a minimal
consumer no longer inherits the unwanted dependency.

### C3 — align rollback/content declarations

Capability composition should collect domain-owned authored schemas and rollback
fragments without a central runtime census.

### C4 — separate host services

Audio, persistence, networking transport, window/input devices and renderer
services should be explicit host/service contracts where real consumers need
substitution. Do not abstract them all preemptively.

### C5 — narrow the facade

Expose semantic capability names and stable game-author APIs. Keep internal crate
moves behind the facade.

## Acceptance

- a minimal external game selects a small capability set and its dependency tree
  reflects that choice;
- Ambition composes the rich engine without special privileged paths;
- adding a new optional domain does not require edits in unrelated runtime or
  game crates;
- capability conflicts/missing requirements fail during preparation with useful
  diagnostics;
- internal crate decomposition can continue without forcing external game code
  to follow it.
