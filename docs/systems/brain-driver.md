# Brain driver — controllable-entity unification

The universal-brain interface, as it stands at the end of the
2026-05-24 overnight session.

**Source:** [`docs/planning/universal-brain-interface.md`](../planning/universal-brain-interface.md)
(design doc), [`TODO-controllable-entity.md`](../../TODO-controllable-entity.md)
(plan), and [`dev/journals/ae-player-field-usage-2026-05-24.md`](../../dev/journals/ae-player-field-usage-2026-05-24.md)
(decomposition map).

## Why brains exist

Every controllable entity in the sandbox — players, NPCs, enemies,
bosses, and (future) RL agents / remote co-op players — needs to
answer one question each tick: *what does this actor want to do?*
Pre-brain, three nearly-parallel update paths each answered that
question internally (NpcRuntime, EnemyRuntime, BossRuntime, plus
`update_player`). A behavior change like "telegraphs flash a ring"
had to be repeated in all four.

A brain is the single seam where "what does this actor want" gets
decided. The integration stage (collision, cooldowns, EFFECTS) then
reads the same shape regardless of who filled it.

## Vocabulary

```
                          ┌────────────────────────┐
                          │      Brain (enum)      │
                          │  - Player(slot)        │
                          │  - StateMachine(cfg)   │
                          │  - (future Remote/RL)  │
                          └───────────┬────────────┘
                                      │ tick()
                                      ▼
                          ┌────────────────────────┐
                          │      BrainSnapshot     │
                          │  pos, vel, target,     │
                          │  timers, wall_contact, │
                          │  player_input          │
                          └───────────┬────────────┘
                                      │
                                      ▼
                          ┌────────────────────────┐
                          │  ActorControlFrame     │
                          │  desired_vel, facing,  │
                          │  melee_pressed, fire,  │
                          │  jump/dash/interact…   │
                          └───────────┬────────────┘
                                      │ stored on the actor entity
                                      │ as ActorControl(frame)
                                      ▼
                          ┌────────────────────────┐
                          │   EFFECTS-stage        │
                          │   - integration        │
                          │     (step_kinematic)   │
                          │   - cooldowns / fire   │
                          │   - ActionSet resolve  │
                          │     to concrete spec   │
                          └────────────────────────┘
```

Three sibling components on every controllable entity:

- **`Brain`** — the policy. An enum dispatched via match (not trait
  objects). `Brain::Player(slot)` translates inputs.
  `Brain::StateMachine(cfg)` runs one of 7 brain templates.
- **`ActionSet`** — the per-entity capability. Resolves abstract
  brain intent (`melee_pressed = true`) into concrete effects
  (`spawn a Swipe hitbox` vs `spawn a Lunge hitbox`). Two enemies
  with the same brain template but different ActionSets play
  differently.
- **`ActorControl`** — the brain's last-tick output. Read by the
  EFFECTS stage (or, today, read by nobody — the shadow shape
  populates it but EnemyRuntime / BossRuntime / update_player
  still drive behavior).

## Brain templates (small fixed set)

`crates/ambition_sandbox/src/brain/state_machine.rs`:

| Template          | Use today                          | Knobs                                                             |
| ----------------- | ---------------------------------- | ----------------------------------------------------------------- |
| `StandStill`      | Sandbags, dialogue-only NPCs       | none                                                              |
| `Patrol`          | Peaceful NPCs, gated patrollers    | spawn_x, radius, speed, aggressiveness, aggro_radius              |
| `Wanderer`        | Puppy slug (planned)               | speed, climb_walls, chatter_threshold, window, pause              |
| `MeleeBrute`      | Striker / Brute / Striker variants | aggressiveness, aggro_radius, attack_range, chase_speed           |
| `Skirmisher`      | Ranger / future ranged variants    | aggressiveness, aggro_radius, standoff_px, strafe_speed, cooldown |
| `Sniper`          | Stationary turrets                 | aggressiveness, aggro_radius, fire_cooldown_s                     |
| `BossPattern`     | Boss encounter schedules           | aggressiveness, encounter_id, phase, phase_elapsed                |

Per-entity variety lives in `ActionSet`, not in new templates. Two
enemies with the same `MeleeBrute` brain are different in the world
because their ActionSets have different `MeleeActionSpec` variants
(Swipe vs Lunge vs Slam vs Bite).

Each `*ActionSpec` carries its own windup → active → recover
animation timing. There is *no separate `TelegraphSpec`* — the
windup phase of an attack is its telegraph.

## What's wired today

Every controllable entity carries a `Brain` + `ActionSet` +
`ActorControl` sibling component:

- **Players** spawn with `Brain::Player(PlayerSlot::PRIMARY)` +
  the default player `ActionSet` (Swipe melee + Bolt ranged). The
  `tick_player_brains` system (runs in the `PlayerInput` phase
  after `sync_local_player_input_frame`) translates the
  per-player `PlayerInputFrame` into the actor's `ActorControl`
  frame each tick; `emit_brain_action_messages` then runs the
  resolver and writes `ActorActionMessage`s for each concrete
  request. Nothing consumes those messages yet — `update_player`
  still drives combat / projectile spawns from `PlayerInputFrame`
  directly.
- **NPCs** carry `Brain::StateMachine(Patrol{NPC_DEFAULT})` or
  `Brain::StateMachine(StandStill)` per their authored fields.
  `NpcRuntime::tick_via_brain` builds a snapshot, calls
  `brain.tick`, and applies the resulting frame to the NPC's
  body via the same engine kinematic sweep as before. The
  bespoke `NpcRuntime::update` is gone.
- **Enemies** carry `Brain::StateMachine(MeleeBrute{archetype-keyed})`
  or `StandStill` for sandbags or `Wanderer{PUPPY_SLUG_DEFAULT}` for
  puppy slugs. The brain's chase_speed / aggro_radius / attack_range
  are read off `EnemyArchetype` so the brain matches the archetype's
  pre-flip tunings. The matching `ActionSet` carries the archetype's
  concrete attack spec — Striker family gets `Swipe`, Brute /
  Colossus get `Lunge`, BurningFlyingShark gets `Bite + Float`,
  PirateOnShark family gets `Bolt + Float`, Sandbag gets a weak
  `PunchWeak` counter, PuppySlug and peaceful PirateHeavy get
  no melee. `update_ecs_actors` shadow-ticks the brain alongside
  the existing `EnemyRuntime::update`; the frame is produced and
  the resolver emits matching `ActorActionMessage`s, but
  EnemyRuntime still drives behavior — the messages are an
  observation channel until daytime EFFECTS-flip wires combat
  spawns to consume them.
- **Bosses** carry `Brain::StateMachine(BossPattern{encounter_id})`
  where `encounter_id` is the same `String` the boss-encounter
  registry uses (computed via `encounter_id_from_name(boss.name)`
  at spawn). `update_ecs_bosses` shadow-ticks similarly. BossRuntime
  still drives behavior; daytime work threads the registry through
  `BossPattern.tick` to drive the phase schedule from the brain.

When a peaceful NPC turns hostile (strike-threshold flip in
`damage.rs`), the entity's `ActorRuntime::Peaceful → Hostile` swap
also swaps the brain to `Brain::StateMachine(MeleeBrute::STRIKER_DEFAULT)`
so the shadow shape stays aligned.

## What's NOT wired (daytime continuation)

Three big chunks remain:

### 1. EFFECTS consumer flip for enemies + bosses

Today `EnemyRuntime::update` builds its own ActorControlFrame
internally via `build_control_frame` and immediately consumes it.
Daytime work removes that internal build and instead reads the
brain's already-built frame off `ActorControl`. The choreography
state machine moves into the brain's per-template state — Striker
choreography becomes part of `MeleeBruteState`, boss scripted
patterns become part of `BossPatternState`.

Once the EFFECTS stage reads ActorControl + resolves
`ActionRequest`s through the actor's `ActionSet`, per-entity
attack variety (Swipe vs Lunge vs Bite) lights up — currently
the resolver works in unit tests but no spawn system consumes
its output.

### 2. update_player consumes ActorControl

Same pattern as #1 but for the player. Today
`tick_player_brains` fills the frame; `update_player` ignores it
and reads `PlayerInputFrame` directly. Flipping the consumer is
the biggest single risk in the remaining work — overlap-then-
delete per
[`dev/benchmark-candidates/bevy-ecs-stale-component-after-sync-removal-2026-05-15.md`](../../dev/benchmark-candidates/bevy-ecs-stale-component-after-sync-removal-2026-05-15.md).

### 3. `ae::Player` decomposition completes

The remaining 38 `authority.player.*` reads in the sandbox are
mostly co-located with writes. PlayerBody covers the read model;
PlayerInputFrame covers the input read model. The work is to
walk each reader cluster (debug/overlay, dev_tools, runtime/
reset, body_mode/tests) and replace authority access with
component reads + an explicit write component for the few sites
that mutate engine state.

When the last reader is gone, delete `ae::Player` and
`PlayerMovementAuthority`. Per-cluster components
(`PlayerVelocity`, `PlayerWallState`, `PlayerJumpState`, …) may
or may not be needed depending on how far PlayerBody can stretch
— the audit doc captures the full field map.

## Possession and multi-player

Possession is cheap because of brain + ActionSet decomposition:

```rust
// Player presses "possess" on a goblin entity.
commands.entity(goblin).insert(Brain::Player(PlayerSlot::PRIMARY));
// Goblin's ActionSet is unchanged.
// Player input → brain.tick → goblin's ActionSet resolves it as a Leap.
```

Two-player co-op with different bodies is the same operation:

```rust
commands.entity(player2_body).insert((
    Brain::Player(PlayerSlot(1)),
    fast_fragile_skirmisher_action_set,
));
```

Both pending until the EFFECTS consumer flip lands — the brain
seam exists today, but nothing reads its output for combat
effects.

## Performance

- Brain dispatch is enum-match, not trait objects: one switch per
  actor per tick. Branch-predictor friendly.
- Snapshot construction is per-actor per-tick. ~80 bytes of
  POD on the stack; no allocations.
- Shadow-tick adds a free function call + snapshot build for
  every enemy + boss; measurable as a flat ~1-2µs per actor per
  tick, well under frame budget at the 10s-of-actors scale.
- The "unbrained" optimization (parallel ECS path for trivial
  actors like puppy slugs) is documented as an escape hatch but
  not implemented — measurement first.

## File map

```
crates/ambition_sandbox/src/brain/
├── mod.rs              # Brain enum, shadow_tick_brain helper, ActorControl
├── snapshot.rs         # BrainSnapshot, WallContact, to_character_ai_snapshot
├── state_machine.rs    # 7 brain templates + tick_state_machine
├── action_set.rs       # ActionSet, ActionRequest, resolve
└── player.rs           # tick_player_brain + tick_player_brain_from_control
```

Integration sites:

```
crates/ambition_sandbox/src/player/bundles.rs           # bundle attaches brain
crates/ambition_sandbox/src/player/systems.rs           # tick_player_brains
crates/ambition_sandbox/src/app/plugins.rs              # scheduling
crates/ambition_sandbox/src/content/features/ecs/spawn.rs   # NPC/enemy/boss spawn
crates/ambition_sandbox/src/content/features/ecs/actors.rs  # shadow tick (enemies)
crates/ambition_sandbox/src/content/features/ecs/bosses.rs  # shadow tick (bosses)
crates/ambition_sandbox/src/content/features/ecs/damage.rs  # hostile-flip brain swap
crates/ambition_sandbox/src/content/features/npcs.rs        # tick_via_brain
```

Tests:

```
crates/ambition_sandbox/src/brain/{mod,snapshot,state_machine,action_set,player}.rs::tests
crates/ambition_sandbox/src/content/features/conversion_tests.rs  # NPC patrol via brain
crates/ambition_sandbox/src/content/features/ecs/spawn.rs::tests  # spawn regression
crates/ambition_sandbox/src/player/systems.rs::tests              # player seam end-to-end
crates/ambition_sandbox/src/audio/environment.rs::tests           # PlayerBody migration
crates/ambition_sandbox/src/headless.rs::tests                    # full plugin integration
```

## Helper API

Convenience methods exposed for daytime work:

| Type                  | Helper                                  | Returns                            |
| --------------------- | --------------------------------------- | ---------------------------------- |
| `Brain`               | `stand_still()`                         | `Brain::StateMachine(StandStill)`  |
| `Brain`               | `npc_patrol(spawn_x, radius)`           | `Brain::StateMachine(Patrol{...})` |
| `Brain`               | `is_player()`                           | `bool`                             |
| `Brain`               | `player_slot()`                         | `Option<PlayerSlot>`               |
| `Brain`               | `is_hostile()`                          | `bool`                             |
| `Brain`               | `label()`                               | `&'static str`                     |
| `Brain` Display       | `format!("{}", brain)`                  | `"Player(slot=N)"` / `"StateMachine(label)"` |
| `ActorActionMessage`  | `is_melee()` / `is_ranged()` / `is_special()` | `bool`                       |
| `ActionRequest`       | `label()`                               | `"melee_swipe"`, `"ranged_bolt"`, …|
| `ActionRequest` Display | `format!("{}", req)`                  | `"melee_swipe(at … facing +1)"`    |
| `MeleeActionSpec`     | `damage()` / `reach_px()` / `total_duration_s()` | `i32` / `f32` / `f32`     |
| `RangedActionSpec`    | `speed()` / `damage()`                  | `f32` / `i32`                      |
| `ActionSet`           | `peaceful()` / `can_attack()`           | `Self` / `bool`                    |
| `BrainSnapshot`       | `idle()` / `to_character_ai_snapshot(...)` | `Self` / `ae::CharacterAiSnapshot` |
| `ActorControlFrame`   | `neutral()` / `wants_any_action()` / `clear_edges()` | `Self` / `bool` / `()`   |
| `shadow_tick_brain` / `shadow_tick_brain_with_timers` | free fn | `ae::ActorControlFrame` |
| `CombatTimers`        | `CLEAR` const                            | `Self` (all zeros)                 |
| `log_brain_action_messages` | Bevy system (optional)             | debug! log per message              |

## Quick reference

| Thing | Where |
| ----- | ----- |
| Brain enum + ActorControl + ActorActionMessage | [`crates/ambition_sandbox/src/brain/mod.rs`](../../crates/ambition_sandbox/src/brain/mod.rs) |
| Per-template state machines | [`crates/ambition_sandbox/src/brain/state_machine.rs`](../../crates/ambition_sandbox/src/brain/state_machine.rs) |
| Per-entity ActionSet + resolver | [`crates/ambition_sandbox/src/brain/action_set.rs`](../../crates/ambition_sandbox/src/brain/action_set.rs) |
| Player input → frame translator | [`crates/ambition_sandbox/src/brain/player.rs`](../../crates/ambition_sandbox/src/brain/player.rs) |
| BrainSnapshot definition | [`crates/ambition_sandbox/src/brain/snapshot.rs`](../../crates/ambition_sandbox/src/brain/snapshot.rs) |
| Player spawn bundle | [`crates/ambition_sandbox/src/player/bundles.rs`](../../crates/ambition_sandbox/src/player/bundles.rs) |
| Enemy / boss spawn brain attach | [`crates/ambition_sandbox/src/content/features/ecs/spawn.rs`](../../crates/ambition_sandbox/src/content/features/ecs/spawn.rs) |
| Enemy shadow tick | [`crates/ambition_sandbox/src/content/features/ecs/actors.rs`](../../crates/ambition_sandbox/src/content/features/ecs/actors.rs) |
| Boss shadow tick | [`crates/ambition_sandbox/src/content/features/ecs/bosses.rs`](../../crates/ambition_sandbox/src/content/features/ecs/bosses.rs) |
| NPC brain-driven tick | [`crates/ambition_sandbox/src/content/features/npcs.rs`](../../crates/ambition_sandbox/src/content/features/npcs.rs) |
| Player tick_player_brains + resolver scheduling | [`crates/ambition_sandbox/src/app/plugins.rs`](../../crates/ambition_sandbox/src/app/plugins.rs) |
| ae::Player decomposition audit | [`dev/journals/ae-player-field-usage-2026-05-24.md`](../../dev/journals/ae-player-field-usage-2026-05-24.md) |
| Extension recipe | [`docs/recipes/extending-brains-and-action-sets.md`](../recipes/extending-brains-and-action-sets.md) |
| Multi-chunk plan | [`TODO-controllable-entity.md`](../../TODO-controllable-entity.md) |
