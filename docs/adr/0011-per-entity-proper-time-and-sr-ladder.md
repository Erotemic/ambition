# ADR 0011: Per-entity proper time and the Galilean-to-SR ladder

## Status

Accepted research direction; current gameplay implementation is partial.

## Decision

Use per-entity proper-time vocabulary as the long-term way to represent local time manipulation. Global clock scaling remains a useful early/single-player behavior, but it is not the only conceptual model.

The long-term ladder is:

1. Galilean/single-clock gameplay for current simple mechanics.
2. Per-entity local clocks for coherent multi-observer slow/fast effects.
3. More relativistic or non-Euclidean time rules only when a concrete mechanic needs them.

## Context

For one player, slowing the world and boosting the player's proper time can feel equivalent. For multiple observers, replay, AI training, or story mechanics about who controls time, those operations differ. The project wants the vocabulary without overbuilding the full system before gameplay requires it.

## Consequences

- Proper-time language is valid design vocabulary.
- The current implementation may still use simpler clock handling.
- Future local-clock mechanics should integrate with ADR 0010 regime policies.

## Current implications for agents

- Do not implement speculative relativity infrastructure without a concrete gameplay use.
- Do not delete the vocabulary just because the full ladder is not landed.
- When adding time-affecting mechanics, say whether it is global sim time, entity proper time, presentation-only time, or wall-clock behavior.
