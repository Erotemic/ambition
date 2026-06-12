# Gameplay effects, damage messages, and ECS messages

The sandbox routes cross-system gameplay side effects through typed Bevy messages. The progression/save/audio effect streams are **four focused messages** consumed by per-effect systems in `features::bus`. Feature-local interactions use additional typed messages such as the unified combat `HitEvent`, `ResetRoomFeaturesEvent`, and `GameplayBannerRequested`.

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

- `HitEvent` (carrying a `HitSource` + `HitTarget`) for **all** combat damage — player slash/projectile against feature targets, pogo-bounce orbs, and hazard/enemy/boss damage against the player. This replaces the old split damage-message family.
- `ActorActionMessage` for resolved brain/action requests that spawn or start concrete effects.
- `GameplayBannerRequested` for HUD banner text from systems whose parameter list is already large.
- `ResetRoomFeaturesEvent` for same-room feature reset.

Presentation facts that already have a concrete presentation type should use the existing presentation messages directly, for example `SfxMessage`, `VfxMessage`, and `DebrisBurstMessage`.

## Damage and hit state today

The legacy split damage-message shapes have been **unified into one `HitEvent`** carrying a `HitSource` (who/what
caused it — `PlayerSlash` / `PlayerProjectile` / `PogoBounce` /
`EnemyBody` / `EnemyProjectile` / `BossBody` / `BossAttack` / hazard / …),
a `HitTarget` (broadcast `Volume` vs a specific player/actor entity), plus
`volume`, `damage`, `HitMode`, and optional `HitKnockback`. Both outgoing
(player → feature) and incoming (hazard/enemy/boss → player) damage flow
through it. What's still missing is a full per-hit *lifecycle* object —
`HitEvent` is the canonical transport, but reaction/poise/stagger/armor/
scaling metadata isn't modeled yet.

| Current shape | Role | Remaining limitation |
|---|---|---|
| `HitEvent { source, target, volume, damage, mode, knockback }` | The single attacker-side + victim-side hit message (`apply_feature_hit_events` applies it to actors/bosses/breakables; the player-damage reader applies victim-side sources to players) | Transports the contact but carries no defense/armor/stagger/reaction state — those are resolved ad hoc downstream. |
| Hostile `Hitbox` entities | Enemy/boss melee active windows that emit `HitEvent`s on overlap | Good explicit lifecycle; the emitted event is still the small `HitEvent` shape. |
| `BossDamageOutcome` | Boss HP/invulnerability/kill result returned by `record_boss_damage` | Useful outcome object, but only boss-specific. |

The next durable cleanup is a `HitSpec` -> `HitInstance` -> `HitResult` pipeline that adds the missing lifecycle metadata on top of the existing `HitEvent` transport:

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
3. Feature-damage systems resolve `HitEvent`s (by `HitSource`) and hostile `Hitbox` overlaps against ECS feature components.
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
