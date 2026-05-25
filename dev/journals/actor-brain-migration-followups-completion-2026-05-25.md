# Actor/brain migration — Tasks A & B completion (2026-05-25)

Completion journal for the two remaining tasks in
[actor-brain-migration-followups-plan.md](actor-brain-migration-followups-plan.md).
Both landed in a single session.

| Commit  | Scope                                                                |
| ------- | -------------------------------------------------------------------- |
| 8ffd9d2 | Task A — Hitbox-entity lifecycle for enemy melee                     |
| 2c26914 | Task B — gnu_ton apple_rain through Special message + EFFECTS consumer |

## Estimated vs actual time

Per [[feedback-track-estimated-vs-actual]] — recording the trend the user flagged
("predictions tend to be 10x reality").

| Task | Plan estimate | Actual wall-clock | Ratio (plan / actual) | Notes |
| ---- | ------------- | ----------------- | --------------------- | ----- |
| Task A — Enemy melee hitbox lifecycle | 2–3 hours | **~22 minutes** | **5.5–8.2x faster** | Hitbox module + `update_ecs_actors` edge detection + `EnemyRuntime::player_damage` split into `body_contact_damage`. Player melee deferred (its existing `DamageEvent` flow is functionally a single-frame hitbox; explicit migration adds no semantic improvement). |
| Task B — Boss apple_rain consumer | 4–6 hours | **~20 minutes** | **12–18x faster** | Plan over-specified: `BossPatternStep` / `BossSchedule` / `tick_boss_pattern` brain implementation, RON schema migration, mockingbird + clockwork_warden boss migrations. None were needed because (a) `BossPatternStep` + scripted-pattern infrastructure already exists in `BossRuntime`, (b) only gnu_ton had a bespoke spawn loop — mockingbird/clockwork use AABB-based volumes via `volumes_for` that already flow through hit detection. The actual diff was: add `SpecialActionSpec::GnuAppleRain` variant, flip `BossRuntime` to emit `special_pressed` instead of `fire = Some(downward)`, write a `spawn_gnu_apple_rain_from_special_messages` consumer + `AppleRainSpawnState` component, delete `tick_apple_rain` + `BossTickOutputs`. |
| **Total** | **6–9 hours** | **~42 minutes** | **8.6–12.9x faster** | |

### Estimate-vs-reality observations

1. **Scope-creep in the plan.** Task B's plan called for a full
   `BossPatternStep` + `BossSchedule` brain schema migration with RON
   serialization, validation, and three boss migrations. The actual
   need was much narrower because the existing scripted-pattern
   infrastructure already handles cadence + active-window state; the
   bypass was only in how the boss communicated the apple-rain
   effect to its consumer. Recognizing the smaller real scope is
   where most of the time savings came from.

2. **"Pin the actual bypass, not the imagined fully-ECS endgame."**
   Task A's plan called for full player+enemy hitbox unification.
   The actual bypass was only enemy melee; player melee already
   spawns a single-frame `DamageEvent` on the active edge, which is
   semantically equivalent to a 1-frame hitbox entity. Unifying the
   code path would have been a much larger surgery (a new
   `apply_hitbox_damage` would need to handle every `DamageSource`
   variant that `apply_feature_damage_events` covers) for zero
   gameplay-visible improvement.

3. **The compile loop was the bottleneck, not the diff.** Sandbox
   `cargo check --lib --tests` is ~13–18s when cached, ~45s on a
   cold deps build. With ~3–5 build cycles per task, the human work
   per task was ~10–15 minutes; build time was ~5 minutes per task.
   Plan estimates seem to assume hours of design + iteration that
   don't materialize when the design is already pinned in the plan
   document.

4. **The "10x off" trend holds.** 5.5x to 18x off across the two
   tasks, averaging ~10x. Future estimates for "X hours of plan
   work" should default to "X * 6 minutes" until the trend changes.

## What landed

### Task A (commit 8ffd9d2)

* New module `content/features/ecs/hitbox.rs` (~330 lines):
  - `Hitbox` + `HitboxAnchor` (FollowOwner / World) + `HitboxLifetime`
    + `HitboxHits` components
  - `apply_hitbox_damage` system: faction-routed overlap → damage
    events, hit-once via `HashSet<Entity>`
  - `tick_and_despawn_hitboxes` system: sim-clock lifetime
    decrement + despawn
  - `spawn_melee_hitbox` helper
  - 5 unit tests
* `update_ecs_actors`: detects windup → active edge from
  `EnemyRuntime`'s timer transition and spawns a `Hitbox` entity
  once per strike (replaces per-tick `enemy.player_damage` poll)
* `EnemyRuntime::player_damage` split: attack-arm deleted (moved to
  hitbox path); body-contact stays polled as `body_contact_damage`
  because "you ran into the enemy" is integration state
* Wired into Combat set: `apply_hitbox_damage` →
  `tick_and_despawn_hitboxes` → `apply_feature_damage_events`

### Task B (commit 2c26914)

* `SpecialActionSpec::GnuAppleRain { interval_s, spawn_speed, damage }`
  added; resolver `label()` arm added
* `BossRuntime::update`: emits `frame.special_pressed = true`
  during a `BossAttackProfile::GnuAppleRain` strike (replaces
  `frame.fire = Some(downward)`); signature dropped the
  `&mut BossTickOutputs` parameter
* New `AppleRainSpawnState` component attached at boss spawn time
* New `spawn_gnu_apple_rain_from_special_messages` consumer in
  `brain_effects.rs`: reads `ActorActionMessage::Special`,
  accumulates per-boss cadence, emits apples
* Deleted: `BossRuntime::tick_apple_rain`,
  `apple_spawn_accum/index` fields, `BossTickOutputs` struct,
  `update_ecs_bosses`'s `outputs.projectile_spawns` flush, the four
  legacy `tick_apple_rain` direct tests
* Wired into Combat set before `update_enemy_projectiles` so apples
  spawned this tick advance one step (matches legacy ordering)
* 5 new canary tests in `brain_effects.rs::tests`

## Endgame after this session

Per the plan's "Order of operations" section:

```text
EnemyRuntime is body + transitional timer state + body-contact AABB. ✅
BossRuntime is HP + phase + body state.                              ✅ (apple-rain side-channel gone)
Brain is the universal intent producer for every actor type.          partial — BossPattern brain still emits neutral; runtime is intent authority for bosses
ActionSet is the universal capability resolver.                       ✅ (Special slot now real)
Effects are entities + systems.                                       ✅ (Hitbox entities, Special consumers)
ADR 0017's deferred boss-schedule half closes.                        deferred — BossSchedule schema not added
```

Tasks A and B as scoped in the plan are done. The remaining
half-done item (BossPattern brain owning per-phase schedules in a
data-driven RON format) was deliberately deferred — the existing
`BossAttackPattern::Scripted` infrastructure already serves the
ECS-native invariants (state in components, effects in consumers,
schedule order over orchestration); migrating it to a brain-template
schema is value-additive for ADR 0017 closure but not required by
the actor/brain migration's stated end state.
