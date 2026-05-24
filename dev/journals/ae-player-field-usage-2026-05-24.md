# `ae::Player` field-usage audit (2026-05-24)

## Why this exists

Chunk 4 of the universal-brain unification (see
`TODO-controllable-entity.md`) decomposes `ae::Player`'s 49-field
god-struct into ECS sibling components on the player entity. The
audit names every reader so the decomposition can move them in
clusters instead of blindly.

`ae::Player` is defined in
`crates/ambition_engine/src/movement/player.rs:26`.

## Field inventory (49 fields)

Grouped by intended sibling component, with the count of read sites
across `crates/ambition_sandbox/src`.

### `PlayerVelocity` — movement (per-tick)
- `pos: Vec2` (6)
- `vel: Vec2` (3)
- `facing: f32` (1)
- `on_ground: bool` (4)
- `max_speed: f32`

### `PlayerSize` — body envelope
- `size: Vec2`
- `base_size: Vec2` (1)

### `PlayerWallState` — wall interactions
- `on_wall: bool` (2)
- `wall_normal_x: f32`
- `wall_clinging: bool` (2)
- `wall_climbing: bool` (1)
- `pre_wall_vel: Vec2`
- `pre_wall_vel_age: f32`
- `ledge_release_cooldown: f32`

### `PlayerJumpState` — jump + coyote buffer
- `air_jumps_available: u8`
- `jump_buffer_timer: f32`
- `coyote_timer: f32`
- `rebound_cooldown: f32`
- `drop_through_timer: f32`

### `PlayerDashState` — dash + dodge
- `dash_charges_available: u8`
- `dash_timer: f32` (2)
- `dash_cooldown: f32`
- `dash_buffer_timer: f32`
- `dodge_roll_timer: f32`
- `dodge_roll_cooldown: f32`

### `PlayerFlightState` — fly + glide
- `fly_enabled: bool`
- `flight_phase: f32`
- `gliding: bool`
- `fast_falling: bool`

### `PlayerBlinkState` — blink ability
- `blink_cooldown: f32`
- `blink_hold_active: bool`
- `blink_hold_timer: f32`
- `blink_aiming: bool` (1)
- `blink_aim_offset: Vec2`
- `blink_grace_timer: f32`

### `PlayerShieldState` — bubble shield + parry
- `shield_active: bool`
- `parry_window_timer: f32`

### `PlayerLedgeState` — ledge grab
- `ledge_grab: Option<LedgeGrabState>`

### `PlayerBodyMode`
- `body_mode: BodyMode`

### `PlayerContact` — world contact (water + climbable)
- `water_contact: Option<WaterContact>` (4)
- `climbable_contact: Option<ClimbableContact>` (2)

### `PlayerResources` — mana
- `mana: ResourceMeter` (5)

### `PlayerCombo` — attack combo state
- `combo: Vec<ComboMark>` (2 reads of `combo_symbols`)

### `PlayerHealthMods`
- `damage_multiplier: i32` (1)
- `invincible: bool` (1)

### `PlayerLifetime`
- `time_alive: f32`
- `resets: u32`

### `abilities` — already pulled into ECS land
- `abilities: AbilitySet` (3 reads via `authority.player.abilities`)
  — this is already accessed through the authority; staying on the
  struct for now.

## File-by-file reader inventory

Files that read `ae::Player` or `PlayerMovementAuthority` (39 total):

```
app/dev_runtime.rs           runtime/setup.rs
app/hud.rs                   runtime/reset.rs
app/phases.rs                player.rs
app/sim_systems.rs           player/affordances/intent.rs
app/update.rs                player/affordances/mod.rs
app/world_flow.rs            player/bundles.rs
audio/environment.rs         player/components.rs
bin/headless.rs              player/ledge_grab.rs
body_mode/mechanics.rs       player/swim.rs
body_mode/morph_ball.rs      player/systems.rs
body_mode/tests.rs           presentation/character_sprites/anim.rs
content/features/bosses.rs   presentation/fx.rs
content/features/conversion_tests.rs    presentation/rendering/actors.rs
content/features/ecs/actors.rs          rl_sim/runtime.rs
content/features/ecs/bosses.rs          pause_menu/model.rs
dev/debug_overlay.rs         time/time_control.rs
dev/dev_tools.rs             world/platforms.rs
dev/trace/detect.rs          world/rooms/graph.rs
dev/trace/systems.rs         falling_sand.rs
dev/trace/tests.rs           lib.rs
```

Reader-cluster breakdown for sequencing decomposition:

| Cluster                  | Files                                              | Risk |
|--------------------------|----------------------------------------------------|------|
| Sim core (the writer)    | `app/sim_systems.rs`, `player/systems.rs`,         | High |
|                          | `app/update.rs`, `runtime/reset.rs`                |      |
| Movement adjuncts        | `body_mode/*`, `player/{swim,ledge_grab}.rs`,      | Med  |
|                          | `world/platforms.rs`                               |      |
| Combat/ability           | `player/affordances/*`, `content/features/*`,      | Med  |
|                          | `audio/environment.rs`                             |      |
| HUD / FX / dev / trace   | `app/hud.rs`, `dev/*`, `presentation/*`,           | Low  |
|                          | `pause_menu/model.rs`                              |      |

## Decomposition strategy chosen for Chunk 4

Following the plan and the "stale-component-after-sync-removal"
journal lesson (link in the plan), the migration is overlap-then-
delete:

- **4b**: Add per-cluster components (`PlayerVelocity`,
  `PlayerWallState`, `PlayerJumpState`, `PlayerDashState`,
  `PlayerContact`, etc.) as siblings on the player entity.
  `PlayerMovementAuthority` keeps writing `ae::Player`; a new
  sync system mirrors the relevant fields into the new
  components after every authority write.
- **4c**: Same pattern for combat/ability state — `PlayerCombo`,
  `PlayerShieldState`, `PlayerBlinkState`, `PlayerHealthMods`,
  `PlayerResources`.
- **4d**: Add a `Brain::Player` driver that builds an
  `ActorControlFrame` from the player's `PlayerInputFrame`. The
  frame is written into the entity's `ActorControl` component
  but nothing reads it yet — proves the brain seam ticks
  cleanly each frame.
- **4e+**: Reverse polarity — readers consume the new
  components instead of `ae::Player`. One reader-cluster per
  commit. Last cluster's commit also deletes `ae::Player` +
  `PlayerMovementAuthority`.

## Stopping rule

If 4e bogs down on broad call-site churn (the
[stale-component journal](../benchmark-candidates/bevy-ecs-stale-component-after-sync-removal-2026-05-15.md)
warns about exactly this), stop at the last green commit. The
overlap-then-delete pattern means the game still runs with both
shapes in place; finishing the polarity flip is a daytime
follow-up.

## What "done" looks like

`crates/ambition_engine/src/movement/player.rs::Player` deleted.
`PlayerMovementAuthority` deleted. Every reader in the file list
above queries one of the new components instead.

## Update (2026-05-24): overnight session outcome

Stopped at end of Chunk 4f after additional consolidation work.
Captured here so the next daytime session picks up cleanly.

**Landed:**
- ActorControlFrame extended with player verbs (`6d04715`).
- crates/ambition_sandbox/src/brain/ scaffolded with Brain enum,
  ActionSet, 7 brain templates, BrainSnapshot, player brain
  translator (`8e06032`).
- NpcRuntime ticks through Brain::StateMachine — bespoke
  NpcRuntime::update gone, brain-built per NPC at spawn
  (`0aa526a`).
- Player entity carries Brain::Player + ActionSet + ActorControl;
  tick_player_brains fills ActorControl each frame via the
  PlayerInput phase (`c41997b`, `32c37e3`).
- PlayerBody expanded to cover wall state, water/climbable
  contact, dash timer, blink_aiming (`506b06c`).
- audio/environment.rs reader migrated off authority onto
  PlayerBody (`923ad65`).
- ECS actor + boss tick systems drop unused PlayerMovementAuthority
  reads (`f9bf1fe`, `40dc3b4`).
- Brain components attach at spawn for hostile actors (`4518a41`)
  + bosses (`61fd1a0`). Shadow brain tick runs each frame for
  enemies + bosses; BossRuntime / EnemyRuntime still drive
  behavior, but the brain output populates ActorControl as a
  parallel shape.
- shadow_tick_brain helper extracted (`3b3a147`) so the per-actor
  snapshot construction lives in one place.
- Regression test asserts encounter-mob spawns carry Brain +
  ActionSet + ActorControl (`46c5f29`).
- Per-archetype tunings threaded through enemy MeleeBrute brain
  cfg so daytime EFFECTS-flip preserves behavior (`540c401`).
- Per-archetype ActionSet defaults wired (Swipe / Lunge / Bite /
  Bolt / PunchWeak / Slither) — every enemy carries a concrete
  attack spec ready for the consumer flip (`83ac40b`).
- NPC hostile-flip also swaps the brain to MeleeBrute via
  commands.insert so the shadow shape tracks disposition
  (`47564fa`).
- ActionSet resolver wired with ActorActionMessage stream —
  emit_brain_action_messages runs in PlayerInput after
  tick_player_brains and writes one message per resolved
  ActionRequest. Nothing consumes the stream yet but daytime
  EFFECTS-flip plugs in here (`4a7efc5`).
- BossPatternCfg encounter_id is a String mirroring
  boss_encounter::encounter_id_from_name (`fd1f63b`).
- End-to-end test: player attack press emits a Swipe
  ActorActionMessage through the full input → brain → resolver
  pipeline (`9587b23`).
- End-to-end test: player projectile release emits a Bolt
  ActorActionMessage (`a67f553`).
- Per-archetype brain + ActionSet mapping tests pin the
  PuppySlug → Wanderer, Sandbag → StandStill, Brute → Lunge,
  PirateOnShark → Float+Ranged assignments (`705d21d`).
- NPC_PATROL_SPEED wired through PatrolCfg::NPC_DEFAULT.speed
  so the brain-side default tracks the legacy const (`86e7b66`).
- Brain module clippy-clean with explicit reason-tagged
  #[allow]s for the intentionally-unused daytime surface
  (`d5c4d54`).
- Comprehensive docs: docs/systems/brain-driver.md overview,
  docs/recipes/extending-brains-and-action-sets.md (`eb38a2f`,
  `de39167`). Existing docs (character-ai-refactor.md,
  universal-brain-interface.md, OVERNIGHT-TODO.md, TODO.md,
  FEATURES.md) updated to reflect what landed
  (`f86b7a7`, `551ec8a`, `5c83587`).

**Remaining for daytime:**
- Reader-side polarity flip: 38 `authority.player.*` reads still
  in the sandbox, most of them co-located with writes. Walk the
  dev_tools / debug_overlay / runtime/reset.rs / body_mode/tests
  clusters per the table above.
- Enemy EFFECTS consumer flip: today the brain's ActorControl
  output is discarded for hostile actors. Replace EnemyRuntime's
  inline choreography → integration call with one that consumes
  brain.tick + ActionSet.resolve. Per-archetype attack specs
  (Swipe / Lunge / Bite / Arrow / Pistol / etc.) need authoring.
- Boss EFFECTS consumer flip: same shape. BossPatternCfg
  already carries the real `encounter_id: String` mirroring
  `encounter_id_from_name(boss.name)` — daytime work threads it
  through `BossPattern.tick` to drive per-phase schedules from
  the brain rather than from `BossRuntime`.
- update_player consumes ActorControl: today the player brain
  fills the frame but update_player still reads PlayerInputFrame
  directly. Flipping the consumer is the biggest single risk in
  the remaining work — overlap-then-delete per the journal.
- Delete `ae::Player` once no reader remains.
- Narrow `ActorControlFrame::fire` to `Option<Vec2>` once
  ActionSet's RangedActionSpec is the speed source.

Test counts at session end: 753+ sandbox lib tests + 265 engine
lib tests + 55/4/5 in other workspace crates = 1082+ total
tests green. Headless / rl_smoke / rl_random_walker binaries all
clean (verified at 60, 100, 200, 300, 500, 800, 1000 ticks at
various points). Brain module clippy-clean. Doc-link check
passes. Agent KB check fails on a pre-existing missing path
unrelated to brain work.

Session-end note: the dev environment hit EMFILE pressure during
intense test-runner phases (parallel cargo test workers opening
many bevy crate FDs). Mitigation: run `cargo test
-p ambition_sandbox --lib -- --test-threads=2` instead of
default parallelism; documented in
docs/recipes/extending-brains-and-action-sets.md.

Additional polish past the initial Chunk 4f wrap (all
parallel-shape / additive, no behavior change):
- Brain.is_player / .player_slot / .label / .stand_still /
  .npc_patrol convenience helpers
- ActorActionMessage.is_melee / .is_ranged / .is_special filters
  + .label() diagnostic string
- ActionRequest.label() diagnostic string
- MeleeActionSpec uniform .damage() / .reach_px() /
  .total_duration_s() accessors
- ActorControlFrame.wants_any_action() / .clear_edges() helpers
- BrainPlugin (message + counter registration in one plugin)
- BrainActionCounter resource + observe_brain_action_counter
  system + observe test
- CombatTimers POD + shadow_tick_brain_with_timers variant
- Brain ABI determinism + dead-actor + 100-tick smoke tests
- Headless integration test (full plugin) for player spawn +
  attack press → ActorActionMessage
- Per-archetype enemy_default_brain + enemy_default_action_set
  coverage lints
- Player ActionSet derived from AbilitySet at spawn

Late-session stability round (catches future-regression
land-mines without changing today's behavior):
- `tick_state_machine` dead-actor branch now explicitly writes
  `ActorControlFrame::neutral()` instead of returning early —
  catches a pre-poisoned frame leaking through dead-actor ticks.
- Sniper + Skirmisher pin the `target_alive=false` branch
  (dead target in aggro must not emit fire).
- Hostile Patrol (aggressiveness > 0) pin tests: Chase emits
  movement, Attack emits melee with cooldown clear, Attack
  honors `attack_cooldown_remaining > 0` and holds off.
- MeleeBrute attack gate table-driven test exercises each of
  the four phase timers (cooldown / windup / active / recover)
  individually — drops in one would have slipped past the
  single-windup test.
- `RangedActionSpec::damage()` per-variant pin mirrors the
  existing `speed()` accessor test.
- `BrainActionCounter` headless test now asserts
  `last_frame ≤ total` instead of a no-op `let _ = …`.
