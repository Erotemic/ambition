# ADR 0010: Time domains and regime policies

## Status

Accepted direction; implementation is incremental.

## Decision

Gameplay code should express time manipulation through a uniform clock-domain vocabulary and policy layer, not ad-hoc direct mutation of global Bevy time from arbitrary systems.

Core vocabulary:

```text
ClockDomain       = SimClock | PlayerClock(entity/player) | WallClock
ClockScaleRequest = requester + domain + scale + reason
RegimePolicy      = permission table over requests
```

Initial regimes are conceptual targets:

- Solo: permissive single-player default.
- RLDeterministic: fixed, seeded, no gameplay clock mutation.
- Cinematic: script-owned authority during sequences.
- Coop/competitive variants: future policy shapes, not current runtime promises.

## Context

Bullet-time, boss freeze, cutscene pause, replay/CI determinism, and future multi-observer gameplay all need clearer time authority. Scaling one global clock works for early single-player feel but is not a durable vocabulary.

## Consequences

- Time-control abilities should become requests evaluated by policy.
- Direct time mutation from gameplay systems should be treated as legacy or transitional.
- Deterministic/headless modes should choose stricter policy rather than forking gameplay.

## Current implications for agents

- Do not assume the full time-regime system is implemented everywhere.
- When touching bullet-time, pause, replay, or headless determinism, look for direct `Time` mutation and decide whether the code is transitional.
- Keep time-domain docs clear about implemented subset versus future vocabulary.
- Cross-check ADR 0011 when adding per-entity or local-clock behavior.
