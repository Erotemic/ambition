# Capability and runtime composition

**State:** OPEN successor program.

## Goal

Make engine composition reflect what a game actually chooses to use.

A consumer building a small platformer should not inherit portal rendering, boss
orchestration, networking integration, persistence, debug presentation or
Ambition-only content merely because a broad historical crate sits in the middle
of the dependency graph.

## Why this program still matters

Current measurements changed the rationale.

Removing several non-Smash experiences from a measured Smash composition did not
materially improve representative frame time, and the associated plugin/system
removal did not improve plugin-registration startup in the measured probe.
Therefore capability composition is **not currently a funded generic runtime CPU
or startup optimization**.

Its demonstrated value is:

- dependency closure;
- coherent ownership;
- smaller/minimal consumers;
- compile/change and test isolation;
- host/platform composition;
- public SDK quality;
- making optional domains actually optional.

## Principles

- capability selection is a semantic engine API, not a Cargo-feature illusion;
- dependency closure and installed runtime behavior should agree;
- the easy default may install a broad useful engine while narrow composition
  remains real and tested;
- internal implementation crates are not public capability names;
- a capability owns its data/schema/install declarations close to its domain;
- headless, rendered, desktop and mobile hosts compose from one capability
  vocabulary with host-specific services layered on top;
- domain-owned rollback/content declarations compose through backend-neutral
  registrars/catalogs; the generic runtime does not own concrete domain-type
  censuses.

## Current pressure points

- `ambition_platformer2d_actor_monolith` still owns several unrelated domains
  and therefore acts as a dependency/composition hub;
- `ambition_platformer2d_shared_tangle` still has high fan-in and mixed ownership;
- the public facade can expose semantic APIs beside historical implementation
  topology;
- some optional domains remain reachable through dependency closure even when a
  consumer did not ask for them;
- construction/content capability installation can still have compile-time and
  runtime-install assumptions that need explicit closure proofs.

Rollback registration itself is no longer the earlier central-census problem:
concrete gameplay declarations are federated by domain and the GGRS backend is
separate from the generic runtime. Do not use that completed migration as the
justification for another capability layer.

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

The plan is not a service locator. It records what is installed, validates
requirements/conflicts and lowers to ordinary Bevy plugins/resources.

## Phases

### C1 — inventory from actual consumers

Use Ambition, Mary-O, Sanic, TwinTrack, Smash and the external-consumer fixture to
identify capability families that have independent customers.

### C2 — choose one leaky capability

Pick a capability whose absence still drags in unrelated crates/runtime behavior.
Carve its declaration/installation boundary and prove a minimal consumer no
longer inherits the unwanted dependency.

Choose the slice for dependency/ownership value, not because system count is
expected to move frame time.

### C3 — align content/construction declarations

A capability that contributes authored schema/construction lanes must have a
coherent installation contract. Avoid states where compile-time support says a
room may build a feature while the runtime fingerprint says that capability is
absent.

### C4 — separate host services where substitution is real

Audio, persistence, networking transport, window/input devices and renderer
services may become explicit host/service contracts where actual consumers need
substitution. Do not abstract all of them preemptively.

### C5 — narrow the facade

Expose semantic capability names and stable game-author APIs. Keep internal crate
moves behind the facade.

## Acceptance

- a minimal external game selects a small capability set and its dependency tree
  reflects that choice;
- Ambition composes the rich engine without privileged hidden paths;
- adding an optional domain does not require edits in unrelated runtime/game
  crates merely to register its state/content;
- capability conflicts/missing requirements fail during preparation with useful
  diagnostics;
- internal decomposition can continue without forcing external game code to
  follow crate topology;
- any claimed performance/startup benefit is backed by a new comparable
  measurement rather than inferred from fewer plugins/crates.
