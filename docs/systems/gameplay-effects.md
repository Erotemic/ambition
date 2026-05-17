# Gameplay effects and ECS messages

The sandbox routes cross-system gameplay side effects through typed Bevy
messages. The main progression/save/audio effect stream is
`features::GameplayEffect`, consumed by focused systems in `features::bus`.
Feature-local interactions use additional typed messages such as
`DamageEvent`, `PogoBounceEvent`, `PlayerDamageEvent`,
`ResetRoomFeaturesEvent`, and `GameplayBannerRequested`.

The old `FeatureEventBus`, `FeatureEvents`, and `FeatureEcsQueues` bridge
layers have been removed. New producers should write typed messages directly
instead of adding ad-hoc vectors or custom resource queues.

## Current boundary

Use `GameplayEffect` for effects that cross into progression, save, encounter,
boss, quest, or standalone audio routing:

- `SetFlag { id, on }`
- `AdvanceQuest(QuestAdvanceEvent)`
- `ActivateSwitch { payload, pos }`
- `DamageBoss { boss_id, amount }`
- `StrikeNpc { npc_id, pos }`
- `PlaySfx { id, pos }` for standalone audio-only effects

Use domain-specific messages when the consumer is known and the payload is more
specific:

- `DamageEvent` for slash/projectile damage against ECS feature targets.
- `PogoBounceEvent` for pogo-refresh breakables.
- `PlayerDamageEvent` for hazards, enemy attacks, and boss attacks damaging the
  player.
- `GameplayBannerRequested` for HUD banner text from systems whose parameter
  list is already large.
- `ResetRoomFeaturesEvent` for same-room feature reset.

Presentation facts that already have a concrete presentation type should use
the existing presentation messages directly, for example `SfxMessage`,
`VfxMessage`, and `DebrisBurstMessage`.

## Scheduling contract

Producers write messages during the simulation phase. Focused readers then
consume them before progression systems that depend on the resulting save,
quest, encounter, boss, or presentation state. Keep this shape when adding new
producers:

1. Simulation systems emit typed messages.
2. Feature-damage systems resolve `DamageEvent` / `PogoBounceEvent` against ECS
   feature components.
3. Gameplay-effect readers apply save, quest, switch, boss, NPC-strike, and SFX
   side effects.
4. Progression systems observe the updated state in the same `Update` frame.

Do not make each producer manually reach into save, quest, boss, switch, or
audio resources unless the behavior is truly local to that producer.

## Design intent

The immediate goal is not to remove every string id. Authored content still uses
human-readable ids, and some effects still carry authored payload strings. The
important shift is that the *kind* of side effect is explicit and typed. That
gives future systems one place to inspect, route, trace, validate, or serialize
gameplay side effects.

When adding new gameplay behavior, prefer one of these options in order:

1. Reuse an existing domain-specific message.
2. Add a new domain-specific message and focused consumer.
3. Add a new `GameplayEffect` variant for cross-domain progression/save/audio
   routing.

Do not add another custom bridge resource or parallel side-effect vector.
