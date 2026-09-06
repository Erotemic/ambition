---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/mechanics/blink.md
  - docs/concepts/movement-collision.md
---

# Blink motion policy

The blink mechanism should not bake in one character's post-blink feel. A policy
selects the reusable choices around path resolution and retained motion while the
body/collision domain owns the actual move.

## Policy dimensions

A blink policy may define:

- requested range and shortening behavior;
- surfaces/volumes that block, permit, or terminate transit;
- candidate safe-placement search order;
- whether pre-blink tangential/gravity velocity is preserved, clamped, replaced,
  or projected;
- post-blink lock/recovery/cancel behavior;
- resource, cooldown, and rejection semantics.

Provider capabilities select and tune a policy. The implementation remains
content-free and shared by every body.

## Commit contract

Resolution produces either:

- a complete result containing destination, resulting body motion/state, and
  semantic outcome; or
- a rejection reason with no authoritative mutation.

Apply position, motion, costs, and state together. Do not relocate first and then
attempt to repair collision or resources in later systems.

## Determinism

- Candidate generation and tie-breaking are stable.
- Queries use the same world/collision semantics as ordinary movement.
- Gravity/orientation transforms are explicit.
- Results do not depend on render frame rate, wall clock, query iteration order,
  particles, or camera state.

## Validation

```bash
python scripts/agent_query.py "blink motion policy destination"
python scripts/agent_query.py tests "post blink velocity"
./run_tests.sh -k blink
```
