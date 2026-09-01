# 0034: Perception is bounded by attention, and belief is the only durable part

## Status

Proposed (2026-09-01). Direction set by Jon; this ADR settles only the
rollback/replay question that blocks the first increment. The design lives in
`docs/planning/engine/bounded-perception-and-attention.md`.

## Context

Jon, on a room full of tactical fighters:

> Increasing the number of actors in a region should increase the fidelity of
> the crowd summary, not linearly increase the amount of cognition supplied to
> every brain.

Today every perceiving body builds an exact view of every peer. The first
increment — let a brain **declare** the resolution it needs, and build only that
— reads like a local optimization, and it is not, because one of the things it
would stop writing is canonically checksummed state:

```text
Perception         rollback_component_canonical, "actor.perception"
PerceptionMemory   rollback_component_canonical, "actor.perception_memory"
```

### These are two different classes, and only one of them is durable

`Perception` is a **policy**: `Omniscient | Sighted { viewport_half }`. The only
runtime construction of a value is `perception.rs:449`, always
`Sighted { DEFAULT_VIEWPORT_HALF }`; `Omniscient` is reached exclusively through
`Default` on an absent component. Every instance in the world holds the same
value, and its presence is *by construction* the same fact as
`PerceptionMemory`'s — `ensure_perception` inserts both together behind
`Without<PerceptionMemory>`, and says so: *"Missing memory ⟺ missing
perception"*. The per-body override its doc comment advertises has no user.

`PerceptionMemory` is **accumulated history**. It cannot be re-derived from the
current tick — surviving the loss of sight of a foe is the entire point — so it
is genuinely durable state.

⇒ A declaration gate changes the VALUE of the second and nothing about the
first.

### What the gate is NOT justified by

⛔ It is not justified by a per-frame saving. A previous draft of this ADR and
of the planning docs claimed the shipped host saves and checksums every
registered component every frame under its zero-distance `SyncTestSession`.
ggrs skips the entire save path at `check_distance: 0`
(`sync_test_session.rs:155`), which is what `local_session.rs:40` configures.
Ordinary play saves nothing.

The measured cognition cost is also small today: `Decide` is 0.234 ms/tick at
the hall's 130 bodies, of which the peer-independent remainder is 0.039 — so the
whole declaration is worth ~0.2 ms. (And that number came from the direct
sandbox; see `performance-and-iteration.md`.)

**This ADR exists because the change is a wire decision, not because it is
fast.**

## Decision

**A brain declares the perceptual resolution it needs; the engine supplies at
most that. Belief is the only durable part of perception, and changing what is
written into it is a replay-compatibility change.**

1. **Resolution is declared, not inferred.** A brain asks for `None`,
   `TargetBelief`, or `TacticalWorld`. The engine builds no more than is asked
   for. A brain that reads nothing is supplied nothing.

2. **The gate reads rollback-authoritative state only** (the brain's own
   configuration), so two peers gating identically cannot diverge.

3. **`PerceptionMemory` remains rollback state, and narrowing what it records
   moves the schema baseline.** It is a canonically checksummed value: a save or
   replay made before the change decodes to a different world than one made
   after. This component has desynced a match before, on a subtler change than
   this. It gets the ADR 0023 treatment — a baseline move, recorded — never a
   benchmark and a merge.

4. **`Perception` stays as it is.** Its value is invariant and its presence is
   already implied by `PerceptionMemory`'s, so it is redundant on the wire — but
   it is the seam a per-character sense override would use, and deleting a
   checksummed name to save nothing measurable is the wrong trade. Do not
   remove it as an optimization; revisit only if the override stays unused when
   attention lands.

5. **The crowd summary is derived, and is NOT rollback state.** The aggregate is
   rebuilt every tick from rollback-authoritative state, so it must not be
   registered. Deriving it keeps the wire flat while the fidelity of the summary
   grows with density.

## Consequences

- The first increment is schedulable: it needs a baseline move, and the baseline
  move is the whole of its risk.
- Cognition cost stops tracking room population, which is the point. Wire cost
  was never tracking it.
- A brain that declares `None` gets an empty belief store rather than one
  tracking peers it never reads. That is a visible change to a checksummed
  value, and it is intended.

## Current implications for agents

- ⛔ Do not gate perception construction as a performance change. It moves the
  schema baseline; land it as one.
- ⛔ Do not argue for or against this work from a per-frame rollback cost. There
  isn't one at `check_distance: 0`. Argue from cognition cost and from the
  attention seam.
- Do not register the crowd summary, the spatial grid, or any aggregate for
  rollback. They are derived every tick; registering them puts a derived value
  on the wire and invites the desync class this ADR is being careful about.
- `Perception` and `PerceptionMemory` are not interchangeable. Policy is
  invariant; belief accumulates. A change that touches one usually does not
  touch the other.
- ⛔ Do not make dormancy the answer. Sleeping distant actors is a legitimate
  policy later; it does not solve the representation problem, and applied to the
  hall it deletes the benchmark that finds these defects.
