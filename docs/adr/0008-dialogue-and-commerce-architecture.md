# ADR 0008: Dialogue and commerce use interactable actor data

## Status

Accepted direction; current implementation is partial.

## Decision

Dialogue, merchants, tutorials, doors/locks, save terminals, and common interaction shells should be authored as interactable actor-like data and projected into Bevy ECS. They should not become a pile of unrelated UI special cases.

`bevy_yarnspinner` remains the likely authored-dialogue direction, but the current runtime may use lightweight code-owned registries or simple dialogue data while the UI/event contract settles.

Merchant rows are treated as dialogue choices with transaction data: price, requirement, preview text, reward/effect, persistence policy, and consequence text.

## Context

The sandbox already has authored interactables and NPC hooks. The durable idea is not a specific first dialogue overlay; it is that common interactions share actor identity, dialogue/choice flow, game-mode pause behavior, and persistence hooks.

## Consequences

- Dialogue and commerce should compose with `GameMode::Dialogue` / pause behavior.
- Merchants should reuse dialogue-capable interactables instead of becoming a separate UI path.
- Content can later migrate to Yarn source files without changing core gameplay semantics.

## Current implications for agents

- Treat this ADR as architecture direction, not proof that a full Yarn/merchant pipeline is complete.
- Before adding a one-off interaction UI, check actor/interactable vocabulary and game-mode pause docs.
- Update `docs/systems/game-mode-pause.md`, `docs/systems/menu-navigation.md`, or concept docs if interaction flow changes durably.
