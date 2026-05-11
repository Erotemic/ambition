# Gameplay effects table

The sandbox now has a first-pass typed gameplay effect table in
`features::GameplayEffect`. This is the migration path away from adding one
stringly typed vector to `FeatureEvents` for every new cross-system side effect.

## Current boundary

`FeatureEvents` still owns presentation-shaped vectors for things that are
already concrete visual/audio facts:

- impact positions
- burst positions
- physics debris bursts
- chest-open positions
- pickup-collected positions
- breakable-destroyed positions

Anything that changes progression state, save state, or another simulation
system should prefer `GameplayEffect`:

- `SetFlag { id, on }`
- `AdvanceQuest(QuestAdvanceEvent)`
- `ActivateSwitch { payload, pos }`
- `DamageBoss { boss_id, amount }`
- `StrikeNpc { npc_id, pos }`
- `PlaySfx { id, pos }` for standalone audio-only effects

## Design intent

The immediate goal is not to remove every string id. Authored content still uses
human-readable ids, and some effects still carry authored payload strings. The
important shift is that the *kind* of side effect is now explicit and typed.
That gives future systems one place to inspect, route, trace, validate, or
serialize gameplay side effects.

When adding new gameplay behavior, prefer adding a new `GameplayEffect` variant
or a helper method on `FeatureEvents` instead of adding another parallel vector
such as `foo_happened: Vec<(String, ...)>`.

## Future direction

The next step is to move effect payloads away from raw strings when the target
vocabulary is stable enough. For example, switch activation can eventually carry
a parsed `SwitchActivation` or a `SwitchId`, and save flags can grow a typed
`GameFlagId` wrapper. This patch deliberately stops short of that so existing
authoring remains compatible while the event-routing shape improves.
