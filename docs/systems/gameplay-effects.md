# Gameplay effects, damage messages, and ECS messages

The sandbox routes cross-system gameplay side effects through typed Bevy messages. The progression/save/audio effect streams are **four focused messages** consumed by per-effect systems in `features::bus`. Feature-local interactions use additional typed messages such as `DamageEvent`, `PogoBounceEvent`, `PlayerDamageEvent`, `ResetRoomFeaturesEvent`, and `GameplayBannerRequested`.

**Review date:** 2026-06-02. The single mixed-purpose `GameplayEffect` enum was split into the four typed messages below (ecs-cleanup-plan #5); the earlier no-op `DamageBoss` / `StrikeNpc` variants were already deleted (boss damage applies inline; NPC strikes route through `ActorStimulus`).

The old `FeatureEventBus`, `FeatureEvents`, and `FeatureEcsQueues` bridge layers have been removed. New producers should write typed messages directly instead of adding ad-hoc vectors or custom resource queues.

## Current boundary

Use the focused progression/save/audio messages for effects that cross into save, quest, encounter, or standalone audio routing. Each has a single consumer system in `features::bus`:

- `SetFlagRequested { id, on }` — save flag + same-frame `QuestAdvanceEvent::FlagSet` mirror (`apply_flag_effects`).
- `QuestAdvanceRequested(QuestAdvanceEvent)` — structured quest events (`apply_quest_effects`).
- `SwitchActivated { activation, pos }` — switch activation → encounter queue + click SFX (`apply_switch_effects`).
- `GameplaySfxRequested { id, pos }` — standalone audio-only effects (`apply_gameplay_sfx_effects`).

(Boss damage is applied inline in the hit path; NPC strike/aggression flows through `ActorStimulus` → `apply_npc_stimuli` / `apply_actor_stimuli`.)

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
4. The focused effect readers apply save (`SetFlagRequested`), quest (`QuestAdvanceRequested`), switch (`SwitchActivated`), and SFX (`GameplaySfxRequested`) side effects; NPC strike/aggression is handled by the `ActorStimulus` readers.
5. Progression systems observe the updated state in the same `Update` frame.

Do not make each producer manually reach into save, quest, boss, switch, or audio resources unless the behavior is truly local to that producer.

## Design intent

The immediate goal is not to remove every string id. Authored content still uses human-readable ids, and some effects still carry authored payload strings. The important shift is that the *kind* of side effect is explicit and typed. That gives future systems one place to inspect, route, trace, validate, or serialize gameplay side effects.

When adding new gameplay behavior, prefer one of these options in order:

1. Reuse an existing domain-specific message.
2. Add a new focused typed message + its own consumer system (the `SetFlagRequested` / `SwitchActivated` / … pattern) for cross-domain progression/save/audio routing.
3. For combat-hit behavior, prefer extending the future `HitSpec`/`HitInstance` shape over adding a new parallel damage event.

Do not add another custom bridge resource or parallel side-effect vector.
