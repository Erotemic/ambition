# Gameplay effects and event bus

The sandbox has a typed gameplay event stream in `features::GameplayEffect` and
one Bevy resource, `features::FeatureEventBus`, that routes those events across
simulation systems.

This is the migration path away from adding one stringly typed vector to
`FeatureEvents` for every new cross-system side effect.

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
system should prefer `GameplayEffect` and the bus:

- `SetFlag { id, on }`
- `AdvanceQuest(QuestAdvanceEvent)`
- `ActivateSwitch { payload, pos }`
- `DamageBoss { boss_id, amount }`
- `StrikeNpc { npc_id, pos }`
- `PlaySfx { id, pos }` for standalone audio-only effects

## Bus contract

Producers enqueue effects with `FeatureEventBus::ingest` or
`FeatureEventBus::emit`. The `drain_feature_event_bus` system is the central
routing table.

Scheduling matters:

1. `sandbox_update` emits feature effects.
2. `update_projectiles` emits projectile-hit effects.
3. `drain_feature_event_bus` drains the bus.
4. encounter, boss, quest, save, and audio consumers see those effects in the
   same `Update` frame.

Keep this order when adding new producers. Do not make each producer manually
reach into save, quest, boss, switch, or audio resources unless the behavior is
truly local to that producer.

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
