---
status: current
last_verified: 2026-07-18
---

# Mechanics documentation

Mechanics pages describe reusable gameplay contracts: the vocabulary an engine
must expose so many characters and games can express different behavior without
adding parallel player/enemy implementations.

They are deliberately not catalogs of current content, tuning values, or a list
of every implemented move. Those facts change quickly and belong in provider
data, source, tests, or planning.

## Core contracts

- [`abilities.md`](abilities.md) — how an authored capability becomes one shared
  actor action and body effect.
- [`body-modes.md`](body-modes.md) — shape, locomotion, and traversal modes.
- [`blink.md`](blink.md) — discontinuous movement and safe-placement policy.
- [`projectiles-and-motion-inputs.md`](projectiles-and-motion-inputs.md) —
  command recognition separated from projectile execution.
- [`expressibility-checklist.md`](expressibility-checklist.md) — questions that
  reveal when a mechanic requires a missing engine primitive or a fork.

## Reading rule

A mechanic is engine-ready when it can be:

1. authored as data or composed from reusable capabilities;
2. requested by a human, brain, RL controller, replay, or script through the
   same actor-local action seam;
3. simulated headlessly without presentation assets;
4. observed and rendered through read models/messages;
5. reconstructed across reset, room replacement, save/load, and snapshot restore;
6. used by a second provider without editing Ambition-specific content.

Use `python scripts/agent_query.py "<mechanic>"` to find the current owner and
`python scripts/agent_query.py tests "<invariant>"` to find validation anchors.
