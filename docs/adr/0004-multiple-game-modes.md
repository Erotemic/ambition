# ADR 0004: Support multiple modes from reusable mechanics

## Status

Accepted as a design constraint.

## Context

Ambition may support a semi-linear metroidvania, pure platformer labs, roguelike/procedural runs, hybrid generated worlds, and debug/simulation modes. The exact product shape is still developing in `docs/brainstorms/` and `docs/vision/`.

## Decision

Design reusable mechanics and data vocabularies so game modes compose on top of them instead of forking the controller or collision model.

Examples:

- movement, combat, body modes, and projectiles belong in reusable mechanics;
- campaign progression, story flags, enemy-learning policy, and mode-specific tuning belong in game/story/sandbox policy;
- recording/replay/test hooks should help multiple modes validate behavior.

## Consequences

Do not hardwire the engine to one campaign structure. Do not let speculative modes block the first playable vertical slice. Use Bevy ECS and data specs so mode-specific policies can be added without rewriting core mechanics.
