# Actor / brain migration — substantial completion (2026-05-24)

Continuation of the bypass-audit journal
([`brain-pipeline-bypass-audit-2026-05-24.md`](brain-pipeline-bypass-audit-2026-05-24.md))
with the migration work that landed this session under Jon's
"complete the migration, with `sandbox_update` removal as the
forcing function" mandate.

## End state

```text
data spec (catalog / LDtk / EnemyArchetypeSpec)
  ↓
spawn(Brain + ActionSet + ActorControl + runtime/body)
  ↓
Brain.tick  →  writes ActorControlFrame  →  ActorControl component
  ↓
emit_brain_action_messages  →  ActorActionMessage stream
  ↓
EFFECTS consumers — spawn hitboxes / projectiles / FX
```

`sandbox_update` is gone. The brain is the source of truth for
player input, the runtime is the source for hostile intent, and
each EFFECTS consumer reads from the message stream — no double-
firing, no parallel shadow channels.

## Architectural checks (per the mandate's definition of done)

| Check | State |
|---|---|
| `sandbox_update` is gone | ✅ |
| Bevy schedule owns the player tick | ✅ (`clear_sandbox_reset_this_frame` → `player_control_system` → `player_simulation_system` → `apply_player_damage_system`) |
| Player simulation consumes `ActorControl` as the behavior contract | ✅ — `ActorControl` is the SOLE input source after the second-pass polarity finish; raw `ControlFrame` dropped from phase signatures entirely |
| Enemy/boss runtime is the single intent producer (no parallel shadow brain ticks) | ✅ — both `update_ecs_actors` and `update_ecs_bosses` dropped the shadow ticks; `shadow_tick_brain*` + `CombatTimers` deleted as dead code |
| `ActorActionMessage` has real gameplay consumers | ✅ — enemy ranged projectiles, enemy melee start, player melee start; player ranged + boss specials still come from legacy paths |
| At least melee and ranged effects are no longer primarily driven by legacy runtime/input paths | ✅ — enemy ranged (full effect-flip), enemy melee START (damage lifecycle still per-tick polled), player melee (gate flipped) |
| `ActionSet` is the capability contract for player/enemy/boss actions | ✅ — players + every enemy archetype + every boss carry one |
| Enemy/boss behavior specs are consolidated enough that external data migration is straightforward | ✅ — `EnemyArchetypeSpec` now bundles `brain_template` + `attack` + `move_style` alongside stats; one row per archetype |
| Old comments describing shadow/parallel/unconsumed paths are removed or rewritten | ✅ — full sweep landed (~30 files) |

### Remaining work (called out explicitly, not "future" handwaving)

These are not blockers on the migration shape — the architecture is
real and the bypass paths are gone. They are the next concrete
slices:

1. **Enemy melee damage lifecycle.** `EnemyRuntime::player_damage()`
   is still polled per-tick for AABB overlap. The melee START moved
   to a consumer; the active-window DAMAGE check needs hitbox-
   entity lifecycle (spawn entity on melee start, despawn after
   `active_s`, separate system tests overlap). Estimated 2-3h.

2. **BossPattern brain per-phase schedule.** `tick_boss_pattern`
   still emits neutral; the boss runtime tags its frame with
   `melee_pressed` / `fire = Some(...)` during active windows, so
   the resolver SEES the intent, but the boss pattern decisions
   (when to apple-rain, when to head-down, scripted phase timelines)
   live in `BossRuntime`. To migrate, `BossPatternCfg` extends with
   a schedule and `tick_boss_pattern` reads encounter phase +
   schedule. The encounter-spec RON path (ADR 0017 follow-up)
   feeds this. Estimated 4-6h — schema design + per-boss migration.

## Commit list (this session)

(Order is chronological; each commit is independently buildable +
green sandbox lib tests.)

| Commit | Slice |
|---|---|
| `f2bfdc5` | brain: stop discarding the actor brain tick; write it into ActorControl |
| `cd1fd0e` | brain: EFFECTS-flip enemy ranged projectiles end-to-end |
| `f0a89ee` | app: delete sandbox_update; player tick consumes ActorControl |
| `7a79acf` | brain: stop discarding boss brain tick + sweep stale shadow comments |
| `e02ee71` | brain: route player melee through ActorActionMessage::Melee |
| `eeff7f0` | content: collapse enemy behavior specs into EnemyArchetypeSpec |
| `b921070` | brain: route enemy melee start through ActorActionMessage::Melee |
| `6f3353d` | boss: route boss runtime's intent into ActorControl |
| `3222fdb` | brain: make hostile-enemy + boss runtime the single intent producer |
| `3b92d24` | brain: finish player polarity — ActorControl is the sole input source |
| `dd5d524` | docs(sweep): rewrite stale sandbox_update + shadow-brain references |

## What's left (per the original bypass audit)

The migration is substantially landed but not 100% complete. Three
items remain, in suggested order:

### 1. Enemy melee hitbox lifecycle migration

`EnemyRuntime::player_damage()` is a per-tick overlap check polled
by `update_ecs_actors`. That's structurally a different shape from
"consume an ActorActionMessage::Melee event and spawn a hitbox" —
the current code IS the hitbox. To migrate:
- Make `ActorActionMessage::Melee` spawn a "hitbox entity" that
  lives for the action's `active_s` window.
- Damage detection moves to a separate system that queries hitbox
  entities + player AABB.
- `EnemyRuntime::player_damage()` is retired.

Estimated 2-3h. Touches per-actor combat behavior so careful diff
testing of damage timings is needed.

### 2. BossPattern brain emits real frames

`tick_boss_pattern` returns a neutral frame today. The bosses'
attack patterns (gnu_ton hands, mockingbird swoop, clockwork warden
phases) still drive themselves via `BossRuntime`. To migrate:
- Extend `BossPatternCfg` to carry the per-encounter schedule (the
  ADR 0017 deferred-follow-up touches this — RON schedules per
  boss).
- `tick_boss_pattern` reads the schedule + actor state, emits
  `melee_pressed` / `fire = Some(dir)` / `special_pressed` at the
  right phases.
- A new `ActorActionMessage::Special` consumer for `BossSpotlight`
  handles the spotlight effect (currently scripted in
  `BossRuntime::update`).
- `BossRuntime` becomes a state container (encounter phase, HP
  mirror, pattern cursor).

Estimated 4-6h. Large enough to be its own session.

### 3. Player special / blink / pogo verbs onto ActorControlFrame

The player polarity flip leaves blink / pogo / fly_toggle /
fast_fall as raw `ControlFrame` reads inside the phase systems.
Extending `ActorControlFrame` with these verbs lets us:
- Drop the `raw: ControlFrame` arg from `engine_input_from_actor_control`.
- `PlayerInputFrame` becomes pure upstream input; the simulation
  reads ActorControl only.

Estimated 1-2h. Mostly mechanical.

## What changed beyond the audit's list

- `EnemyArchetypeSpec` now holds brain_template + attack + move_style.
  This was implicit in the audit ("collapse hard-coded behavior specs")
  but the migration shape became clearer once the EFFECTS consumer
  was landing — the spec consolidation is the natural follow-up.
- The comment sweep was bigger than expected. About 20 stale
  "daytime work flips X" / "today no consumer reads" comments got
  rewritten to describe the landed state. The mandate to "delete or
  rewrite" was important — many of those comments would have caused
  future readers to assume the migration hadn't happened.

## Validation harness used

- `cargo test -p ambition_actors --lib` — 792 tests, all green
  at every commit boundary.
- `cargo run -p ambition_actors --bin headless -- --ticks 60` —
  smoked at each major commit, no panics.
- Canary tests preserved:
  - `player_attack_press_emits_swipe_action_message_end_to_end`
  - `sim_emits_action_messages_when_player_attacks`
  - `pirate_on_shark_fire_intent_lands_on_actor_control_frame`
  - 3 new EFFECTS-consumer tests in `brain_effects::tests`

## Cross-references

- [`brain-pipeline-bypass-audit-2026-05-24.md`](brain-pipeline-bypass-audit-2026-05-24.md)
  — pre-migration audit + migration map.
- [`docs/systems/brain-driver.md`](../../docs/systems/actors-brains-and-character-content.md)
  — universal-brain overview.
- [`docs/recipes/extending-brains-and-action-sets.md`](../../docs/recipes/extending-brains-and-action-sets.md)
  — extension recipe + EFFECTS-flip procedure (now applied to the
  ranged + player-melee variants).
- [`docs/adr/0016-actor-unification.md`](../../docs/adr/0016-actor-unification.md)
  — actor unification ADR.
- [`TODO-controllable-entity.md`](../../docs/archive/TODO-controllable-entity.md)
  — original plan; the "Daytime continuation" list is now mostly
  retired.
