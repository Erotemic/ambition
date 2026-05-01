# ADR 0004: Support multiple game modes from one reusable engine

## Status

Proposed.

## Context

Ambition may support a semi-linear metroidvania, a pure platformer, a pure roguelike, and a hybrid mode where generated runs feed a persistent world. A new brainstorm introduced a data-sharing choice: opting in helps later generations build on the player's run, but enemies or hostile systems may learn from the same data.

## Decision

Design `ambition_engine` around reusable verbs and data specs that can serve multiple modes. Do not hardwire the engine to exactly one campaign structure. Treat roguelike/data-sharing as a candidate game mode until the first vertical slice proves the core.

## Consequences

Progression systems should distinguish engine mechanics from campaign policy. Data-sharing, metaprogression, and enemy-learning systems should be implemented as game/story policy on top of reusable recording, replay, ability, and world-state primitives.
