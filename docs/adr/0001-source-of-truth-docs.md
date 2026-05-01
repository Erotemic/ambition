# ADR 0001: Maintain source-of-truth docs separately from historical notes

## Status

Accepted.

## Context

Ambition has accumulated many focused patch docs and historical design notes. These are valuable, but older notes can make it look like superseded constraints still apply. The README had also become stale by trying to describe too much current implementation detail.

## Decision

Use a documentation hierarchy:

1. README as a stable project portal.
2. `docs/CURRENT_STATE.md` for active state and transient areas.
3. `docs/GOAL_STATE.md` for long-term direction.
4. ADRs for durable decisions and supersessions.
5. Focused subsystem docs for implementation details.
6. Historical docs preserved with supersession pointers when needed.

## Consequences

The README should be easier to keep correct. Future agents should update `CURRENT_STATE.md` and ADRs instead of editing scattered historical notes when architecture changes.
