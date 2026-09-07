# Capability and runtime composition

**State:** ACTIVE — the prerequisite authority work is far enough along for
capability/plugin/crate composition to be the primary architecture workstream.
The remaining task is to move genuine capability installation to its owner while
preserving irreducible composition where only the composition layer can name
both sides.

This page is a **current composition contract and execution guide**. Historical
censuses, individual carve diaries, corrected counts and superseded candidate
lists live in git history.

## Scope

This page owns:

- who installs a capability's private systems;
- how capabilities participate in runtime phases without naming foreign private
  systems;
- optional-capability compile/runtime contracts;
- rollback declaration participation;
- active-ruleset policy borrowing/restoration;
- where cross-capability composition legitimately remains centralized;
- the public facade boundary after composition is sound.

The actor-monolith SCC/package split is owned by
[`actor-monolith-decomposition.md`](actor-monolith-decomposition.md) and the
current execution frontier it links to.

## Current architecture checkpoint

At the current reviewed baseline, measured by the repository tools:

```text
foreign private-system ordering
    capability/ruleset       1
    composition             73
    total                   74

foreign system installations
                           174

carveability upper bound
    reducible                2
    irreducible             39
```

Re-measure instead of copying these numbers forward:

```bash
python scripts/measure_foreign_system_ordering.py
python scripts/measure_carveable_installations.py
python scripts/check_declared_system_packages.py
```

The one capability-written ordering at this baseline is the body-clock
contribution naming a concrete `sim_view` reset system. It is a local regression
against the C1 invariant and should return to zero through published scheduling
vocabulary.

## Prerequisite status

### A/F1 — construction/actor inversion: crossed

Construction no longer depends upward on feature implementation. Continue the
residual monolith split through the SCC plan; do not reopen the old F1 migration
unless the direct dependency returns.

### B — control/custody authority: crossed

`ControlClaims` retains simultaneous claims and deterministically projects the
effective `TemporaryControl`. Mount custody and actual control masking are
separate facts, ordinary dismount retracts its claim, and rollback restores the
claim state before projection.

Do not return to multiple features overwriting `TemporaryControl` directly.

### C1 — scheduling vocabulary: invariant is zero private foreign ordering

A capability may place its systems in published semantic phases/sets. It may not
order itself against another crate's concrete private function.

Current action: repair the one body-clock edge and keep the ratchet at zero.

### D — optional rollback composition: crossed for the current model

Optional simulation capabilities are compile-time composition choices. Their
rollback declarations participate only when the capability is compiled/installed.

If Ambition later requires one binary to dynamically load/unload arbitrary
rollback-owning capabilities, reopen this decision explicitly rather than
stretching the current registrar model.

### E — scoped ruleset policy: crossed

A game/ruleset may temporarily override engine policy by:

1. snapshotting the prior value;
2. applying the active-ruleset policy;
3. restoring the exact prior value on departure.

Do not make a demo plugin permanently own process-global engine policy.

## Composition model

### Capability ownership

A reusable capability should own:

- its authoritative state and semantic messages;
- its private systems;
- its rollback declaration when applicable;
- its installation function/plugin;
- its public scheduling sets/milestones;
- its absence behavior.

The host/composition layer decides **which capabilities exist together**, not
how each capability internally works.

### Runtime ownership

The runtime owns global simulation phases and cross-domain ordering vocabulary.
It may compose published capability sets into a deterministic pipeline.

The runtime should not accumulate a list of private functions merely because it
is the central place where they historically happened to be registered.

### Irreducible composition is legitimate

A block is irreducible when its job is genuinely to coordinate two independent
capabilities that cannot name each other without reversing dependency direction.

Do not create an artificial “installer” whose only purpose is to make a foreign
installation count decrease. Preserve explicit composition when composition is
the owner.

### Optional means absent-capable

For a capability advertised as optional:

- removing it from the compile/composition graph must not require its resource,
  message or rollback state elsewhere;
- a minimal host must not panic because an unrelated system assumes it exists;
- tests should include a tiny app/absence contract rather than only a full game.

### Content requirements are explicit

A capability that requires authored data should declare the requirement through
catalog/validation vocabulary. Missing content should be diagnosed before the
runtime reaches a private implementation panic.

## C2 execution procedure

For every remaining foreign installation block:

1. **Name the owner.** Is the block one capability, shared runtime vocabulary, or
   genuine cross-capability composition?
2. **Inspect chaining.** A syntactically single-capability block may still be
   semantically entangled by `.chain()` or private ordering.
3. **Move only owned internals.** If one capability owns the block, provide an
   installer/plugin in that capability and let composition call it.
4. **Publish milestones, not functions.** If another domain needs ordering,
   expose a `SystemSet`/phase fact.
5. **Prove population.** A source test that counts systems is insufficient when
   the bug can be “installed in the wrong host.” Include the supported minimal
   and assembled populations.
6. **Prove absence.** Optional capabilities need a no-capability composition
   test/contract.
7. **Remeasure.** Record only the current command/result in the owning queue
   item; do not append a carve diary here.

## Current C2 work

### C2.1 — restore the C1 zero-ordering invariant

Replace the Smash body-clock contribution's direct ordering against
`sim_view::rebuild_body_clocks_view` with a published body-clock/reset/contribute
milestone owned by the read-model/presentation vocabulary.

Acceptance:

```bash
python -m pytest -q scripts/tests/test_foreign_system_ordering.py
```

must return the capability/ruleset count to zero.

### C2.2 — inspect the remaining reducible blocks

The current carveability tool reports only a very small reducible tail. For each
row, verify semantic ownership before moving it. If a row crosses a real lane
boundary, reclassify it as composition rather than manufacturing a package to
absorb it.

### C2.3 — preserve the irreducible composition ledger

For the remaining cross-capability blocks, maintain an explicit reason the
composition layer must name both sides. A future carve requires a new authority
fact, not another wrapper function.

### C2.4 — narrow the public facade after ownership is stable

The public facade should export stable semantic types, capability plugins and
supported host composition—not the actor monolith's internal organization.

Do not perform broad facade churn ahead of the crate/SCC work. Compatibility
adapters can disappear only after their consumers have moved to durable owners.

## Rollback composition contract

The domain that owns authoritative rewind state also owns the declaration that it
must be registered. The backend implements a backend-neutral registration
vocabulary.

Do not centralize a growing list of optional domain components inside the GGRS
backend merely because the backend performs the final registration call.

Stable wire names/encoded shapes are compatibility state. Moving a type between
crates does not authorize changing those IDs.

## Ruleset/session/host separation

Use this vocabulary consistently:

- **capability** — reusable mechanism;
- **ruleset/game** — policy values and feature selection;
- **session** — active lifetime/instance;
- **host** — platform/window/input/audio/network composition.

A capability should not infer game policy from which demo plugin happened to be
installed. A game should not permanently mutate host-global policy to activate a
session rule.

## Target shape

```text
game / experience
    -> selects ruleset policy + capability composition

runtime
    -> owns global phases and deterministic cross-domain schedule

capability plugin/installer
    -> owns private systems + state + rollback declarations
    -> installs into published phases/sets

host
    -> adds window/input/audio/network/presentation services

public facade
    -> exports supported semantic composition surfaces
```

## Acceptance

C2 is complete enough for Engine 1.0 when:

1. capability/ruleset code has zero foreign-private-system ordering;
2. capability-owned private systems are installed by their capability;
3. remaining centralized registrations are documented cross-capability
   composition rather than historical leftovers;
4. optional capabilities are absent-capable and rollback-gated;
5. ruleset policy restores prior engine policy exactly;
6. supported minimal/headless/windowed compositions install only the populations
   they claim;
7. the public facade exposes durable semantic boundaries rather than monolith
   internals.

## Standing prohibitions

- do not chase a zero foreign-install count;
- do not add public sets merely to make a metric smaller;
- do not order a capability against a foreign concrete system;
- do not move cross-capability policy into one participant to make composition
  disappear;
- do not call compile-time optionality runtime dynamism;
- do not preserve dated carve transcripts in this live page.

Use git history for the removed 2026-09-03 through 2026-09-07 measurement and
carve narrative.
