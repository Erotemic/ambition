# Actor residual-kernel decomposition

> **Current topology measured at `625fa79af45e6eff40cbefabd8cdae33c5b5e9db`
> on 2026-09-07.**
>
> Historical carve-by-carve investigation lives in Git history. This document
> keeps the durable rules, current topology and exit criteria only.

**State:** ACTIVE, no longer a prerequisite to begin C2 capability composition.
The remaining actor-kernel decomposition continues in parallel because it still
controls package isolation, compile fanout and ownership clarity.

Executable next steps live in
[`actor-monolith-work-frontier.md`](actor-monolith-work-frontier.md).

## Goal

Reduce `ambition_platformer2d_actor_monolith` to the tightly integrated actor/body
simulation kernel described by
[`controlled-character-actor-kernel.md`](controlled-character-actor-kernel.md).

A successful carve moves an authority to its real owner and leaves a narrow,
explicit dependency behind. Source shrinkage is useful only when that happens.

The residual kernel may own approximately:

```text
body state and actor-local lifecycle
accepted intent / control projection
movement/contact integration
core reaction/action seams
narrow observation/decision interfaces
```

It should not remain the composition root for:

```text
session/world lifecycle
persistence/save mirrors
items/projectiles as independent domains
boss/encounter/dialogue orchestration
presentation/UI/audio
host/dev policy
named product content
```

## Current measured graph

Run:

```bash
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
```

Current nontrivial SCCs:

```text
11: abilities, actor_spawn, character_runtime, construction, control,
    features, items, projectile, session, shrine, world

 2: assets, character_sprites
```

The current largest SCC has five single-edge cuts that make it smaller:

```text
->  9    1 ref   character_runtime -> features
-> 10    1 ref   features -> projectile
-> 10    2 refs  actor_spawn -> character_runtime
-> 10    2 refs  projectile -> features
-> 10    6 refs  shrine -> session
```

The executable frontier deliberately does **not** choose the cheapest edge in
all cases. Edge direction and ownership outrank reference count.

## What the SCC means

A strongly connected component says the current source graph contains paths in
both directions among every member. It is a warning against extracting one
module in isolation while preserving all current dependencies.

It does **not** say:

- every member belongs in one final crate;
- every member must become its own crate;
- the smallest reference count is the correct cut;
- two modules in a small SCC must first be made acyclic before they can move
  together.

The useful questions are:

1. which subsets are coherent ownership groups;
2. which directions are legitimate consumption of lower vocabulary;
3. which reverse edges are policy/registration/lifetime authority leaking
   upward or sideways.

## Durable carve rules

### 1. Crates follow ownership, not file size

A 2,000-line module with one wrong authority edge can be a better carve than a
20,000-line module with correct dependencies. Do not choose by LOC.

### 2. A type filed beside its first consumer can manufacture a false cycle

Several prior cuts were unlocked by moving a dependency-free marker/config/value
type to the lower domain that actually owned its meaning.

Before inventing an interface, inspect whether the dependency is only a data type
whose current file location is historical.

### 3. Re-exports preserve dependency paths

Moving a definition and leaving all consumers on the old re-export can leave the
module graph unchanged and keep the old module as the discovery authority.

After moving a type:

```text
consumer -> new semantic owner
```

should be visible in source. Keep compatibility exports only at a stable public
facade where they do not rebuild the internal dependency.

### 4. Scheduling belongs to the capability that owns the systems

A carved domain installs/configures its own private systems against published
semantic sets. Composition may order public phases; one domain should not order
another crate's concrete private function.

### 5. Registrars and codecs are ledgers, not semantic owners

A rollback-registration module naming many domain types is a dependency ledger.
Do not carve a domain merely to reduce that list. Move registration ownership
only when the destination can own the full rollback contract coherently.

### 6. Lifetime is part of authority

Session, match, attempt, stock and process lifetimes are different. A resource or
component that crosses one of those boundaries needs an explicit owner and
retraction/reconstruction rule before moving crates.

### 7. Production acceptance moves with the authority

A helper test proving a moved function still computes the same number is not
sufficient. Keep or add the production poison that exercises the real install,
schedule, rollback/lifetime and host path affected by the cut.

### 8. Prefer one-way downward vocabulary over callbacks/service locators

When a high-level module needs a lower fact, move/publish the fact at the lower
owner. Do not replace a Rust dependency with a process-global registry or dynamic
callback unless substitution is a real product requirement.

## Current decomposition sequence

The current graph supports a bounded peel before another design pass:

```text
P1  character_runtime -> features
    move stocks-match settlement value vocabulary downward
    expected largest SCC: 11 -> 9

P2  projectile -> features
    stop projectile simulation calling feature-specific breakable/boss helpers
    expected largest SCC: 9 -> 8

P3  shrine -> session
    move rollback-safe lifecycle intent/slot vocabulary below session executor
    expected largest SCC: 8 -> 7

P4  construction -> world
    move actor-specific placement-lowering specialization to construction
    expected largest SCC: 7 -> 6
```

Exact instructions and acceptance are in the executable frontier.

After P4, the expected hard core is:

```text
abilities, control, features, items, session, world
```

No single edge currently splits that six-module SCC. That is the point at which
mechanical peeling stops and authority design resumes.

## The six-module design problem

At the current head, after applying the four planned cuts conceptually, the
remaining direct two-way pairs are:

```text
abilities <-> control
features  <-> control
features  <-> world
items     <-> session
world     <-> session
```

Do not decide those boundaries from this summary. The next design pass must
classify the concrete edges by ownership:

```text
DATA/VOCABULARY
POLICY
SCHEDULING
CONSTRUCTION
LIFETIME
```

The likely hypotheses worth testing are:

- `abilities + control` may be one coherent actor-local kernel;
- `world + session` may be one lifecycle package until a real seam appears;
- `features` is a residual orchestration bucket and should shrink by returning
  responsibilities to owners rather than becoming a permanent public crate;
- `items` should be audited as residual adapters/policy because the physical and
  held-item domains have already been carved out.

Those are hypotheses, not a package map. Record exact evidence before carving the
six-module core.

## Satellite SCCs

After P1, `actor_spawn <-> character_runtime` should become a two-module SCC
outside the central knot. That is a successful boundary discovery, not a demand
to split the pair immediately. It may be a coherent grouped construction/runtime
package. Revisit its internal cycle only when a package boundary requires it.

`assets <-> character_sprites` is already outside the central actor SCC. Treat it
the same way: it may be a coherent grouped extraction or it may need a
catalog/loader inversion. The existing external `ambition_character_sprites`
crate owns pose/geometry semantics and is not automatically the destination for
loading/catalog code.

## What has already converged

The historical D33 campaign has already moved substantial independent authority
out of the actor monolith, including domains such as:

- developer tools;
- world-item physics;
- held-item mechanics;
- body seed/construction vocabulary;
- match vocabulary;
- encounter features;
- reusable abilities;
- mount installation/lifecycle ownership;
- several presentation/FX responsibilities.

The exact commit-by-commit story is intentionally absent here. Git history keeps
it. Re-open a prior carve only when current production evidence shows the
ownership decision was wrong.

## Measurement rules

The module graph intentionally excludes file-level tests and inline
`#[cfg(test)]` blocks. A test reaching across modules is a fixture dependency,
not a production boundary.

The graph counts textual `crate::<module>` paths. Therefore:

- it undercounts dependencies imported through globs/unqualified aliases;
- it can preserve a stale edge through a re-export;
- it cannot distinguish a legitimate vocabulary dependency from an authority
  inversion;
- it cannot tell whether two modules should move together.

Always read the concrete references before acting on a graph edge.

## Post-carve acceptance

Every carve owes:

```bash
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
python3 scripts/modules_md.py
python3 scripts/check_doc_links.py
python3 scripts/check_planning_citations.py
```

Plus:

- focused production tests for the moved authority;
- rollback/lifetime tests when canonical state moves;
- source/manifest absence guards when a new crate boundary is created;
- capability/compile-cost ratchets required by their owner pages.

Do not re-freeze a failing baseline merely because topology changed. Attribute
and accept only the metric whose movement is an intentional price of the carve.

## Exit

D33 is complete when:

1. the residual actor package matches the controlled-character kernel target;
2. optional domains install and own their own implementation;
3. world/session/persistence/product policy no longer lives in the actor kernel;
4. remaining internal SCCs correspond to deliberate package units rather than
   accidental reverse dependencies;
5. external consumers use semantic facade APIs rather than following internal
   crate moves.

The goal is a coherent kernel with clear package boundaries. A DAG of tiny files
is not required.
