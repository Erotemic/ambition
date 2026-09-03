# Gameplay effects, damage messages, and ECS messages

The sandbox routes cross-system gameplay side effects through typed Bevy messages. The progression/save/audio effect streams are **four focused messages** consumed by per-effect systems in `features::bus`. Feature-local interactions use additional typed messages such as the unified combat `HitEvent`, `RoomReplayAdmitted`, and `GameplayBannerRequested`.

**Review date:** 2026-06-02. The single mixed-purpose `GameplayEffect` enum was split into the four typed messages below (ecs-cleanup-plan #5); the earlier no-op `DamageBoss` / `StrikeNpc` variants were already deleted (boss damage applies inline; NPC strikes route through `ActorStimulus`).

The old `FeatureEventBus`, `FeatureEvents`, and `FeatureEcsQueues` bridge layers have been removed. New producers should write typed messages directly instead of adding ad-hoc vectors or custom resource queues.

> **RE-VERIFIED 2026-09-03, and this page had drifted twice.** Its **Review
> date: 2026-06-02** is the date the `GameplayEffect` split happened, not a
> currency stamp — and two of the commits that invalidated rows below it landed
> after: `de22be956` (2026-06-22) merged `apply_npc_stimuli` away, and
> `ea55d8023` (2026-08-30) deleted `ResetRoomFeaturesEvent`. Both are corrected <!-- cite-ok: names the deleted message this note is about -->
> in place with their successors.
>
> ✔ What was checked: every CamelCase type and every `snake_case` name this page
> cites, against all Rust and RON in the tree. **12 of 12 current-boundary types
> now resolve** and 49 of 49 function names do. ⚠ What was NOT checked: whether
> the boundary this page describes is the one the code actually enforces — that
> the four typed messages are the ONLY route for their effects. Existence is a
> cheaper question than exclusivity, and only the first was answered.

## Current boundary

Use the focused progression/save/audio messages for effects that cross into save, quest, encounter, or standalone audio routing. Each has a single consumer system in `features::bus`:

- `SetFlagRequested { id, on }` — save flag + same-frame `QuestAdvanceEvent::FlagSet` mirror (`apply_flag_effects`).
- `QuestAdvanceRequested(QuestAdvanceEvent)` — structured quest events (`apply_quest_effects`).
- `SwitchActivated { activation, pos }` — switch activation → encounter queue + click SFX (`apply_switch_effects`).
- `GameplaySfxRequested { id, pos }` — standalone audio-only effects (`apply_gameplay_sfx_effects`).

(Boss damage is applied inline in the hit path; NPC strike/aggression flows through `ActorStimulus` → `apply_actor_stimuli` (`actor_monolith/src/features/ecs/aggression.rs:21`). ⚠ This named a PAIR — `apply_npc_stimuli` / `apply_actor_stimuli` — until 2026-09-03. `de22be956` (2026-06-22) merged them, and its subject says so: *"one actor cluster + in-place provoke"*. The doc kept both halves of a split the commit removed, which is the readable shape of this rot: a slash between two names where the point of the change was that there is now one.)

Use domain-specific messages when the consumer is known and the payload is more specific:

- `HitEvent` (carrying a `HitSource` + `HitTarget`) for **all** combat damage — player slash/projectile against feature targets, pogo-bounce orbs, and hazard/enemy/boss damage against the player. This replaces the old split damage-message family.
- `ActorActionMessage` for resolved brain/action requests that spawn or start concrete effects.
- `GameplayBannerRequested` for HUD banner text from systems whose parameter list is already large.
- ⛔ **`ResetRoomFeaturesEvent` IS DELETED — this row named it as current until <!-- cite-ok: names the deleted message this correction replaces -->
  2026-09-03.** Removed 2026-08-30 in `ea55d8023` (*"the replay reset the world,
  then found out it was not allowed to happen"*), which is also why: the message
  let a reset be seen while no body existed and re-read several frames later
  against a different world. Same-room reset now goes through
  `ambition_combat::events::RoomReplayAdmitted`, written by
  `actor_monolith/src/shrine.rs` and **drained unconditionally** for exactly that
  reason. ⇒ The successor is not a rename: it is a message with a different
  lifetime rule, so a producer copying the old row would reintroduce the defect
  the deletion fixed.

Presentation facts that already have a concrete presentation type should use the existing presentation messages directly, for example `SfxMessage`, `VfxMessage`, and `DebrisBurstMessage`.

## Damage and hit state today

The legacy split damage-message shapes have been **unified into one `HitEvent`** carrying a `HitSource` (who/what
caused it), a `HitTarget` (broadcast `Volume` vs a specific `Body(Entity)`), plus
`volume`, `damage`, `HitMode`, and optional `HitKnockback`.

⛔ **THIS PARAGRAPH LISTED THE WRONG VARIANTS UNTIL 2026-09-03, and the error was
the one the unification existed to fix.** It said `HitSource` carries
`PlayerSlash` / `PlayerProjectile` / `PogoBounce` / `EnemyBody` / <!-- cite-ok: lists the WRONG old variants the correction is refuting -->
`EnemyProjectile` / `BossBody` / `BossAttack` — seven names encoding WHO struck <!-- cite-ok: lists the WRONG old variants the correction is refuting -->
into the cause. None of them exists. The real variants
(`ambition_combat/src/events.rs:253`) are **`Melee`, `Projectile`, `Contact`,
`Hazard`, `LeftTheWorld`, `Pogo`** — six, and every one is a KIND of harm rather
than a role.

⇒ The type's own doc says why: *"attacker identity comes from
`HitEvent::attacker`, victim routing from the named target"*, and of the
melee/projectile split, *"a victim genuinely wants to know whether it took a
contact swing or a ranged shot — that is a real difference in the world, unlike
who fired it."* ⇒ **So the page was describing the pre-unification shape while
claiming to describe the unification**, and anyone matching on
`HitSource::PlayerSlash` would not compile. Both outgoing
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
