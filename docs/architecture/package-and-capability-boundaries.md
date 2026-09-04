# Package and capability boundaries

This page is durable architecture. Active extraction work belongs in
`docs/planning/`, especially
[`actor-monolith-decomposition.md`](../planning/engine/actor-monolith-decomposition.md)
and
[`capability-and-runtime-composition.md`](../planning/engine/capability-and-runtime-composition.md).

## Purpose

Ambition decomposes by **semantic ownership and dependency direction**, not by
file size or crate count. A package boundary is useful when it makes one domain
independently understandable, installable, testable, or reusable and removes a
real authority/dependency knot from its former owner.

The public `ambition_platformer2d` facade is the compatibility boundary.
Internal packages may move while consumers keep semantic APIs.

## Layering

The intended direction is:

```text
foundations / value types
        ↓
domain services and reusable capabilities
        ↓
unified authoritative simulation
        ↓
observation / presentation
        ↓
runtime, provider and host composition
        ↓
game-specific named content
```

A lower layer does not reach upward to register a higher domain, name a game
asset, or drive a presentation policy merely because one executable needs the
combination.

## What earns a package

A new crate or independently owned plugin should satisfy several of these at
once:

- one coherent authority or capability can be named without mentioning the old
  container;
- its plugin installs the systems/resources/messages owned by that domain;
- dependency direction becomes simpler;
- an existing direct dependency edge or change-fanout path can disappear;
- it can be exercised in a small Bevy `App` without importing Ambition game
  content;
- tests can target the domain without assembling unrelated capabilities;
- another real consumer could plausibly use the boundary;
- the public facade can expose a semantic API instead of an implementation path.

Moving files into a new crate that is immediately imported wholesale by the old
crate is not decomposition. Neither is creating a forwarding crate that retains
all of the old authority.

## Package maturity

Use three practical levels rather than assuming every extraction must become a
standalone ecosystem project.

### Internal domain/plugin

The code has one owner and installs itself coherently, but may still live inside
a larger package while the boundary settles.

### Ambition workspace crate

The domain has stable ownership, useful independent tests, and a dependency
boundary worth enforcing inside the workspace.

### Independently consumable Bevy crate

Promote only after a real consumer demonstrates that the API, configuration,
error model and feature surface make sense without Ambition's repository
context. Do not externalize a crate merely because its name sounds generic.

## Two goals, and the second is not implied by the first

⭐⭐ **AUTHORITY DECOMPOSITION IS NECESSARY. INDEPENDENTLY INSTALLABLE CAPABILITY
COMPOSITION IS A SEPARATE SUCCESS CRITERION.** A domain can satisfy every rule on
this page about ownership and dependency direction and still be mandatory in
every supported composition. Both are required; neither substitutes for the
other, and authority decomposition comes first.

⇒ **The doctrine — the two dimensions, the ordering, the absence criterion, the
intended layering, the runtime and shared-schedule risks, the stronger meaning of
"decomposed", and the minimum-host tests that would prove it — lives in
[`../planning/engine/decomposition.md`](../planning/engine/decomposition.md).**
It is not restated here, so that there is one home for it.

⚠ Read it beside the rest of this page rather than instead of it: this page's
"what earns a package" list is the threshold for CREATING a boundary; capability
composability is the further question asked of a major reusable capability once
it has one.

## Capability composition

Optional capability composition is primarily about architecture:

- a minimal consumer should not inherit unrelated gameplay domains;
- providers install the capabilities they require rather than editing a central
  application census;
- domain registration travels with the domain;
- platform/product hosts choose the capability set;
- tests can assemble the smallest host that still exercises the property under
  test;
- the public SDK can explain requirements in semantic terms.

Recent measurements did **not** show generic capability removal to be a useful
frame-time or plugin-registration optimization. That does not weaken the
ownership, dependency, compile-isolation or SDK case. Do not justify a carve by
promised runtime savings unless a current measurement demonstrates them.

## Bevy plugin rule

A capability plugin owns its own registrations when that does not invert the
dependency graph. The generic runtime provides common schedules, lifecycle
hooks, registrars and substrate; domain crates contribute domain-specific state
and systems through those seams.

A host may compose several domain plugins. It should not become a hand-written
registry of every type inside every domain.

## Dependency-closure rule

Before extracting or gating a dependency for footprint, inspect all paths:

```bash
cargo tree -i <crate>
```

Removing one direct edge does not reduce the resolved closure when another
package already supplies the dependency. That can still be a worthwhile
ownership or compile-isolation change, but record the benefit accurately.

Measure at least the distinction among:

- a manifest dependency line;
- a direct edge in the resolved product graph;
- the total resolved closure.

Do not use one count as a proxy for all three.

## Actor residual kernel

The actor decomposition has a concrete destination. The residual actor package
should contain the tightly coupled, reusable actor/body simulation that really
needs to move together, approximately:

- body state and actor-local authoritative state;
- control/intent acceptance;
- movement/contact integration;
- actor-local lifecycle semantics;
- narrow action/body integration interfaces.

The following do not belong in the residual kernel merely because they were
historically implemented there:

- named game content or provider catalogs;
- dialogue/conversation and encounter orchestration;
- persistence and process/session policy;
- UI, menus, audio or presentation effects;
- developer tooling;
- optional items, portals, projectiles or other capabilities that have coherent
  independent ownership;
- host/platform composition.

The active frontier and measured dependency graph live in the focused actor
plan, not in this durable doctrine.

## Validation

Prefer structural proof over source-text policy when the package graph or type
system can express the boundary.

Useful evidence includes:

- `cargo tree` closure checks;
- minimal-consumer compilation/tests;
- domain plugin tests in a small `App`;
- public-facade consumer fixtures;
- absence of the retired direct dependency/import;
- touched-crate rebuild measurements when compile isolation is part of the
  rationale.

Policy scans remain useful for migration residue or properties Rust/Cargo cannot
express directly, but they should not substitute permanently for a real
boundary.
