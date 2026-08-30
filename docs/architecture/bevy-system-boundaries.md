---
id: bevy-system-boundaries
aliases: []
status: current
authority: durable-architecture
last_verified: 2026-08-30
related_docs:
  - docs/architecture/engine-architecture.md
  - docs/planning/engine/simulation-authority-and-determinism.md
  - docs/architecture/package-and-capability-boundaries.md
---

# Bevy ECS system boundaries

Bevy's top-level system-parameter limit is useful design feedback. It is not an
architecture primitive and it should not be defeated by hiding unrelated access
inside tuples or giant custom parameters.

The question is not "how do we fit under sixteen parameters?" It is:

> What semantic entity view, world capability, phase, or mutation authority is
> this system actually expressing?

## First distinguish systems from ordinary kernels

Only registered Bevy systems are constrained by Bevy's system-parameter shape.
A pure/ordinary Rust function with many value/reference arguments may have a
legibility problem, but wrapping it in `SystemParam` or `QueryData` does not solve
that problem.

Use ordinary structs for pure facts, policy, requests, inputs and outcomes when
that makes a kernel contract clearer.

## Name stable entity roles with `QueryData`

Use derived `QueryData` when a tuple repeatedly describes one semantic entity
role, for example a damageable body, controlled actor, projectile row, or
presented subject.

A query-data type should:

- name one entity role;
- make required versus optional components explicit;
- be reusable when the role genuinely recurs;
- avoid becoming one system's entire world encoded as a row type.

The value is semantic consistency and reduced drift, not parameter-count
cosmetics.

## Name cohesive world access with `SystemParam`

A custom `SystemParam` is appropriate when several resources/queries/messages
form one domain-owned capability or service from the caller's perspective.

A good parameter has a sentence explaining its responsibility. A bad parameter
exists only to make Bevy count several unrelated dependencies as one slot.

Do not reuse a broad parameter merely because another system needs half its
fields. That recreates hidden coupling and unnecessarily constrains Bevy's
access graph.

## Split at phase or mutation-authority boundaries

A large system should split when the pieces have distinct semantic phases or
mutation authorities, for example:

```text
observe / gather
    -> decide / resolve pure outcome
    -> apply authoritative mutation
    -> publish derived facts
```

Do not split one globally ordered/noncommutative authority into several writers
merely to obtain smaller signatures. Deterministic ordering and single mutation
authority outrank local function size.

Likewise, do not merge unrelated systems merely to reduce scheduler count.
Current runtime measurements do not justify system-count reduction as a generic
performance program.

## Packing is not decomposition

These are warning signs:

- tuple parameters created only because the system reached Bevy's ceiling;
- one `SystemParam` spanning unrelated domains;
- a parameter type whose fields are mostly unused by each caller;
- a system gathering observation, making policy decisions and mutating several
  authorities in one body;
- `#[allow(too_many_arguments)]` treated as evidence that the function is a
  Bevy-system problem without checking whether it is actually a system.

The correct repair may be a query view, a domain service, a value struct, a phase
split, or no refactor at all.

## Determinism

For simulation systems, ECS access shape must not become an implicit ordering
contract.

- query iteration order is not semantic order;
- noncommutative peers require stable keys and explicit precedence/composition;
- phase ordering belongs in named schedule sets/data flow;
- a refactor that changes schedule topology must preserve deterministic behavior
  intentionally rather than relying on incidental topological order.

## How to choose a refactor slice

Choose a slice because it names a recurring concept or removes mixed authority,
not because it is easiest to make smaller.

A useful slice can state:

1. the system/kernel being changed;
2. the semantic role/capability/phase being named;
3. the old duplicate or mixed authority being deleted;
4. the deterministic behavior that must remain unchanged;
5. the tests/measurements proving the refactor did not invent a new cost or
   ordering dependency.

Do not maintain a permanent census target such as "N QueryData types" or "zero
systems at the parameter ceiling". Counts are investigation aids; architectural
closure is semantic.
