# Gameplay effects, damage messages, and ECS messages

The sandbox routes cross-system gameplay side effects through typed Bevy messages. The main progression/save/audio effect stream is `features::GameplayEffect`, consumed by focused systems in `features::bus`. Feature-local interactions use additional typed messages such as `DamageEvent`, `PogoBounceEvent`, `PlayerDamageEvent`, `ResetRoomFeaturesEvent`, and `GameplayBannerRequested`.

**Review date:** 2026-05-27. Reviewed against source archive `ambition-source-2026-05-26T222032-5-3e93516618a5`.

The old `FeatureEventBus`, `FeatureEvents`, and `FeatureEcsQueues` bridge layers have been removed. New producers should write typed messages directly instead of adding ad-hoc vectors or custom resource queues.

## Current boundary

Use `GameplayEffect` for effects that cross into progression, save, encounter, boss, quest, or standalone audio routing:

- `SetFlag { id, on }`
- `AdvanceQuest(QuestAdvanceEvent)`
- `ActivateSwitch { payload, pos }`
- `DamageBoss { boss_id, amount }`
- `StrikeNpc { npc_id, pos }`
- `PlaySfx { id, pos }` for standalone audio-only effects

Use domain-specific messages when the consumer is known and the payload is more specific:

- `DamageEvent` for slash/projectile damage against ECS feature targets.
- `PogoBounceEvent` for pogo-refresh breakables.
- `PlayerDamageEvent` for hazards, enemy attacks, and boss attacks damaging the player.
- `ActorActionMessage` for resolved brain/action requests that spawn or start concrete effects.
- `GameplayBannerRequested` for HUD banner text from systems whose parameter list is already large.
- `ResetRoomFeaturesEvent` for same-room feature reset.

Presentation facts that already have a concrete presentation type should use the existing presentation messages directly, for example `SfxMessage`, `VfxMessage`, and `DebrisBurstMessage`.

## Damage and hit state today

Combat works, but there is no single canonical per-hit object yet.

| Current shape | Producer / consumer role | Limitation |
|---|---|---|
| `DamageEvent` | Player slash and player projectile hits against feature targets | Outgoing-only shape; does not carry full per-target reaction metadata. |
| `PogoBounceEvent` | Pogo-refresh breakables | Specialized refresh/damage event, not a generic hit result. |
| Hostile `Hitbox` entities | Enemy/boss melee active windows | Good explicit lifecycle, but payload is still small and target-specific resolution happens downstream. |
| `PlayerDamageEvent` | Hazards, hostile hitboxes, enemy projectiles, boss attacks damaging the player | Incoming-only shape; separate from outgoing damage. |
| `BossDamageOutcome` | Boss HP/invulnerability/kill result | Useful outcome object, but only boss-specific. |
| `GameplayEffect::DamageBoss` | Cross-domain boss damage observation seam | The active damage path applies boss damage inline before this bus seam. |

The next durable cleanup is a `HitSpec` -> `HitInstance` -> `HitResult` pipeline:

```text
HitSpec      authored/produced by attack, projectile, hazard, or tool
  -> overlap / target resolution
HitInstance  one concrete source-target contact with impact geometry
  -> defense, armor, invulnerability, scaling, stagger, reaction
HitResult    applied or rejected outcome plus VFX/SFX/hitstop/resource facts
```

That pipeline should unify incoming and outgoing damage instead of growing more one-off events. It should carry health damage, stagger/poise damage, damage kind/elements, source/target identity, impact position/normal, knockback, hitstop/hitstun, pogo/resource rewards, VFX/SFX policy, and rejection reasons.

## Scheduling contract

Producers write messages during the simulation phase. Focused readers then consume them before progression systems that depend on the resulting save, quest, encounter, boss, or presentation state. Keep this shape when adding new producers:

1. Simulation systems emit typed messages.
2. Brain/action consumers resolve `ActorActionMessage` into concrete hitboxes, projectiles, boss specials, and related effects.
3. Feature-damage systems resolve `DamageEvent` / `PogoBounceEvent` / hostile `Hitbox` overlaps against ECS feature components.
4. Gameplay-effect readers apply save, quest, switch, boss, NPC-strike, and SFX side effects.
5. Progression systems observe the updated state in the same `Update` frame.

Do not make each producer manually reach into save, quest, boss, switch, or audio resources unless the behavior is truly local to that producer.

## Design intent

The immediate goal is not to remove every string id. Authored content still uses human-readable ids, and some effects still carry authored payload strings. The important shift is that the *kind* of side effect is explicit and typed. That gives future systems one place to inspect, route, trace, validate, or serialize gameplay side effects.

When adding new gameplay behavior, prefer one of these options in order:

1. Reuse an existing domain-specific message.
2. Add a new domain-specific message and focused consumer.
3. Add a new `GameplayEffect` variant for cross-domain progression/save/audio routing.
4. For combat-hit behavior, prefer extending the future `HitSpec`/`HitInstance` shape over adding a new parallel damage event.

Do not add another custom bridge resource or parallel side-effect vector.
