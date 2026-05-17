# ADR 0005: Mark spatial/geometry code for extra review

## Status

Accepted.

## Context

Ambition has many spatial systems: collision, camera transforms, loading zones, spawn repair, blink shape casts, moving hazards, room graphs, and future non-Euclidean seams. These are easy to get subtly wrong, especially when patches are authored without local compile/run access or rich visual inspection.

## Decision

Use explicit review markers in spatially delicate code:

```rust
// AMBITION_REVIEW(spatial): explain the assumption and what should be checked later.
```

These comments should mark code that is good enough to proceed but deserves future visualization, tests, or a stronger spatial-reasoning pass.

## Consequences

Future agents can search for `AMBITION_REVIEW` and systematically improve high-risk geometry areas. This marker is not a substitute for fixing known bugs.

## Current implications for agents

- Use `AMBITION_REVIEW(spatial): ...` for risky geometry seams.
- Search for existing review markers before broad spatial refactors.
- Treat markers as follow-up hooks, not as excuses to ship known-bad behavior.
