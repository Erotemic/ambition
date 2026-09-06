---
id: engine-architecture
aliases: []
status: current
authority: durable-architecture
last_verified: 2026-08-30
related_docs:
  - docs/concepts/engine-mental-model.md
  - docs/concepts/content-and-provider-boundaries.md
  - docs/architecture/package-and-capability-boundaries.md
  - docs/adr/0027-ggrs-is-the-sole-rollback-authority.md
  - docs/planning/engine/engine-1.0-architecture-program.md
---

# Engine architecture

This page owns the durable current architecture: responsibilities, dependency
direction, authority boundaries, and composition rules. It does not own the
migration backlog. Current gaps live in `docs/planning/`.

The design oracle is:

> Could another platformer be built by adding provider/content packages and a
> thin host without editing reusable engine packages?

Named product content and policy enter through provider packages, Bevy plugins,
and supported Ambition seams. Reusable platformer capability belongs in the
engine.

## Repository shape

```text
crates/   reusable engine vocabulary, kernels, services, presentation, and hosts
game/     Ambition/demo providers, content, apps, and game-owned extensions
tools/    author-time generators, importers, validators, and publishing tools
docs/     durable architecture/concepts/systems plus forward planning and history
```

Directory placement is secondary to Cargo dependency direction and semantic
ownership.

## Architectural layers

The exact package list will keep changing. These responsibilities should not.

### 1. Authored and interchange vocabulary

Backend-neutral schemas describe what may be authored or exchanged. They do not
execute gameplay and do not own a provider's closed named roster.

Examples include entity/action/placement contracts, sprite metadata, sound-bank
formats, and gameplay-trace formats.

### 2. Mathematical and simulation foundations

Content-free geometry, movement, time, stable identity, reference frames, and
shared platformer vocabulary form the trusted simulation foundation.

Foundation packages do not reach upward for product policy or presentation.

### 3. Reusable domain services

Characters/brains, combat, input, projectiles, world composition, encounters,
items, dialogue, persistence, audio, portals, and other reusable domains own
their vocabulary, mutation authority, systems, and plugin-local installation.

A domain should expose typed commands/facts or narrow adapters at its boundary.
The global runtime orders domain sets; it should not become the installation
site for every leaf implementation.

### 4. Live simulation composition

The platformer simulation composes body state, accepted control/intent,
movement/contact integration, world mechanics, action execution, and
simulation facts.

`ambition_platformer2d_actor_monolith` remains a migration-era composition hub,
not the desired permanent package boundary. Decomposition is chosen by semantic
ownership and dependency closure rather than source-line count. The target is a
small residual actor kernel containing only tightly coupled actor/body
simulation responsibilities.

See `package-and-capability-boundaries.md` and the active actor-kernel planning
files for the remaining migration.

### 5. Observation and presentation

Simulation publishes stable read models and semantic presentation facts.
Rendering, animation, audio, camera, HUD, and menus consume those facts without
becoming competing simulation authority.

Presentation may read immutable authored content directly when doing so does
not hide observer-dependent or mutable gameplay truth.

### 6. Runtime, provider, session, and host composition

The reusable runtime owns headless-safe schedule composition and global phase
ordering. Provider/session layers own prepared content, activation, and
construction. Windowed hosts add devices, windows, rendering, and platform
policy. Programmatic harnesses supply explicit caller-selected composition.

The SDK facade exposes semantic engine facilities; it should not require
consumers to know the historical internal package graph.

### 7. Games and providers

Games/providers own named worlds, characters, items, art, audio, encounters,
quests, rules, and product presentation. Composition roots explicitly register
the provider plugins they ship. Lower reusable packages do not depend on game
packages.

Game-owned extension packages are justified after a coherent optional feature
can depend exclusively on supported reusable surfaces. Do not create extension
packages speculatively.

## Dependency direction

The preferred direction is:

```text
provider/game policy
        ↓
host and provider/session composition
        ↓
observation/presentation      reusable domain services
              ↘               ↙
               simulation composition
                       ↓
              foundations / IR
```

Cross-layer convenience re-exports do not change ownership. A facade may make
composition convenient; it must not make a lower package depend upward.

## Content and construction

The durable content flow is:

```text
authoring backend
    -> backend-neutral authored records
    -> validation/lowering
    -> immutable provider content
    -> prepared construction plan
    -> authorized transaction
    -> live authoritative population
```

Import, validation, lowering, preparation, commit, and publication are distinct
phases. Preflight does not mutate the live world. A room/session replacement
remains authoritative until the replacement is ready to commit.

New-session construction, room transition, same-room replay, and new-game reset
consume this one model. They differ in three values — which room they target,
which population they retire, and whether they read or forget durable occurrence
facts — and in nothing else. Retention is decided by declared lifetime: an
object in a body's custody rides through a rebuild, room residents do not.

Durable restoration is the remaining exception: a loaded save adopts its facts
into an already-built world rather than informing that world's construction. The
active migration is owned by
`docs/planning/engine/construction-and-reconstitution.md`.

## Gameplay-session and rollback authority

Three lifetimes are distinct:

```text
Process
  └── Gameplay session        SessionScopeId
        └── Rollback timeline RollbackTimelineGeneration
              └── historical frames
```

A rollback timeline rebase within one gameplay session is continuity. Starting
a different gameplay session is not.

Current rollback authority is installed for a gameplay-session owner. Health or
diagnosis carries across timeline generations only for that same owner. A
foreign session cannot consume another session's rollback health as live
authority. Historical diagnostic evidence may outlive a session, but it does
not gate gameplay.

See ADR 0027 for the implemented ownership contract.

Rollback correctness has several independent dimensions:

1. **state codec** — which authoritative bytes rewind;
2. **population participation** — which authoritative entities/lifetimes rewind;
3. **stable semantic identity** — which logical simulation objects are being
   reconstructed;
4. **deterministic composition** — how several valid peers are selected or
   ordered when the operation is not commutative;
5. **lifetime ownership** — which gameplay session may treat the state as live
   authority.

Passing one dimension does not imply the others.

## Identity and ordering

Bevy `Entity` is an allocator handle, not durable simulation identity.
Authoritative selection or ordering that can affect outcomes uses a stable
semantic key such as `SimId`, authored occurrence identity, or another explicit
domain key.

Raw ECS iteration/spawn order is not semantic precedence. Where several effects
combine noncommutatively, the domain defines a deterministic composition rule.

Dynamic authoritative construction must establish the same identity,
participation, provenance, and lifetime requirements as boot-time construction.
A startup census alone cannot prove this for populations that appear only after
runtime events.

## Simulation and presentation authority

Anything that can change gameplay outcome belongs to simulation authority and
must follow deterministic time/order/lifetime rules. Presentation state is
derived and disposable unless explicitly promoted into gameplay authority.

External effects that cannot rewind must cross an explicit confirmation or
lifecycle boundary rather than firing from speculative rollback frames.

The headless host and visible host should execute the same simulation contracts;
a simplified host is valid only when the omitted composition is irrelevant to
the property being tested.

## Capability composition and package boundaries

Capabilities are semantic composition units, not frame-time flags. Their value
is dependency closure, ownership, minimal consumers, test isolation, platform
composition, and a legible public SDK.

Measurements did not show broad capability removal improving representative
frame time or plugin-registration startup. Do not use those claims to justify
package work without new evidence.

A package extraction is valuable when it creates a coherent owner and removes
real dependency/change fanout. Moving files while the old composition hub must
still rebuild and know the same domains is not architectural closure.

## World existence, residency, simulation, and visibility

These are separate properties:

```text
world occurrence exists
    != its room is resident
    != it is fully simulated
    != it is visible to a local observer
```

Persistent-world architecture builds on session/lifetime ownership and canonical
reconstitution. It should not assume that one resident entity instance is the
same thing as durable occurrence identity.

## Architectural enforcement

Use types, package dependencies, domain-owned plugins, explicit schedules, and
construction APIs to make invalid ownership difficult. Workspace policy scans
are useful regression guards, but they should defend architecture rather than
serve as the primary representation of it.

When changing a boundary:

1. identify the semantic owner;
2. identify the authority/lifetime being moved;
3. preserve or improve headless and real-host coverage;
4. remove obsolete dependency edges and compatibility paths;
5. update durable architecture and the active focused plan in the same change.

## Planning relationship

This document states what the architecture is. Forward work belongs in:

- `docs/planning/status.md` for current orientation;
- `docs/planning/roadmap.md` for strategic ordering;
- `docs/planning/queue.md` for executable work;
- `docs/planning/tracks.md` for deferred work and promotion triggers;
- focused plans for unresolved design.

Completed execution history does not remain here or in active planning merely
because it was expensive to discover. Git history and measurement/review
artifacts retain the evidence when it is worth revisiting.
