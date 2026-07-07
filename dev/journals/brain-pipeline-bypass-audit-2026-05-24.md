# Brain-pipeline bypass audit (2026-05-24)

**Context:** Jon flagged during the character-catalog overnight run
that the brain → ActionSet → consumer pipeline is being bypassed
by several systems, with `sandbox_update` facilitating the player-
side bypass. This is an audit + migration map for the next session;
the actual EFFECTS-flip work didn't fit in the remaining budget.

## Intended state (per universal-brain ADR 0016 + recipe)

```
data spec (catalog / LDtk / boss profile)
  ↓
spawn(Brain + ActionSet + ActorControl + runtime/body components)
  ↓
Brain tick    →  writes ActorControlFrame (per-tick intent)
  ↓
ActionSet     →  resolves frame into ActorActionMessage(s)
  ↓
shared consumers — read ActorActionMessage, spawn hitboxes /
                   projectiles / FX / timers
```

This is the chunks 1–4 scaffolding from the overnight universal-
brain run (commits `c41997b`, `32c37e3`, `506b06c`, etc.).

## Current bypass surface

### Player bypass — `sandbox_update` / `update_player_*`

**File:** `crates/ambition_app/src/app/phases.rs`

- `player_control_phase` (line ~63) calls
  `ae::update_player_control_with_tuning(&control_world, player, input, ...)`
  with `input: ae::InputState` derived from `ControlFrame::engine_input`.
- `player_simulation_phase` (line ~195) calls
  `ae::update_player_simulation_with_tuning(&collision_world, player, input, ...)`
  with the same `InputState`.

Both calls consume `InputState`, not `ActorControlFrame`. The
brain pipeline already attaches `Brain::Player(slot)` + `ActionSet` +
`ActorControl` to the player entity (per chunk 2). `tick_player_brains`
fills the player's `ActorControlFrame`, but `update_player_*`
discards it and reads `InputState` instead.

**Migration target:** `update_player_control` / `update_player_simulation`
should consume `ActorControlFrame` directly. The two clocks
(control_dt + sim_dt) stay; only the input type changes. Touches
the engine boundary (`ambition_engine::player::*`).

Daytime continuation #3 in [`TODO-controllable-entity.md`](../../TODO-controllable-entity.md):
> *`update_player` consume `ActorControl` frame instead of `PlayerInputFrame` directly.*

### Enemy bypass — `EnemyRuntime::update`

**File:** `crates/ambition_actors/src/features/enemies.rs`

- `EnemyRuntime::update` (line ~891) is a 600+-line tick that bakes
  AI + choreography + projectile spawn into a single method. The
  final projectile-spawn block (line ~1144) reads
  `frame.fire: Option<ActorFireRequest>` (from the legacy snapshot
  the runtime maintains internally), pushes an `EnemyProjectileSpawn`
  into `outputs.projectile_spawns`, and the caller flushes that
  into the world.
- Caller: `update_ecs_actors` (`content/features/ecs/actors.rs:188`)
  shadow-ticks `Brain` alongside `EnemyRuntime`, but it's
  `EnemyRuntime.update`'s outputs that actually shape combat —
  `Brain` is read-only parallel surface today.

**Migration target:** drop the `outputs.projectile_spawns`
write inside `EnemyRuntime::update`; instead have a new Bevy
system read `MessageReader<ActorActionMessage>`, filter
`ActionRequest::Ranged { spec, dir }`, look up the actor's
position via the `ActorRuntime::Hostile(enemy)` query, and call
the same `EnemyProjectileSpawn` constructor. Overlap-then-delete
per the [stale-component journal](../benchmark-candidates/bevy-ecs-stale-component-after-sync-removal-2026-05-15.md).

Suggested first variant (per the recipe's "start small"): one
specific archetype's `Bolt` / `Arrow` ranged action — e.g. the
`PirateOnShark` pistol shot. Smaller surface, harder to miss in
QA, exercises projectile spawn + recoil.

### Boss bypass — `BossRuntime` + bespoke pattern state-machine

**Files:**
- `crates/ambition_actors/src/features/bosses.rs`
- `crates/ambition_actors/src/features/ecs/bosses.rs`

Boss attack patterns drive themselves through `BossBehaviorProfile`
+ per-boss runtime state (e.g. `apple_rain`, `spike_halo`, head-
descent windows for gnu_ton). The `Brain::StateMachine(BossPattern
{ encounter_id })` brain is a *placeholder* (`brain/state_machine.rs:524`
returns `ae::ActorControlFrame::neutral()` — the brain doesn't
fight). Combat still flows through the bespoke runtime.

**Migration target:** larger; the bespoke pattern state-machine
needs to become a real `BossPattern` brain producing
`ActorActionMessage::BossSpotlight` / similar. The per-encounter
schedules belong in `boss_encounters/<id>.ron` per ADR 0017's
deferred follow-up (the *numeric* fields half landed today; the
schedule is the second half).

### Player ledge/swim sub-ticks — also call `update_player_*`

**Files:**
- `crates/ambition_actors/src/player/ledge_grab.rs:41,51,65`
- `crates/ambition_actors/src/player/swim.rs:47,70,73,97,122`

These are sub-tick simulations (`0.016` literal) inside the player
modules — used for probing what would happen if the player swims
or ledge-grabs in a specific configuration. They call
`update_player_simulation` directly with a synthetic `InputState`.

When the engine flips to consume `ActorControlFrame`, these
internal probes need to construct a synthetic frame instead.
Mechanical translation; tests pin the behavior.

## Suggested migration order (smallest → largest)

1. **PirateOnShark `Bolt` ranged consumer.** One archetype, one
   action variant, well-scoped tests already in
   `content/features/conversion_tests.rs`. Estimated ~1–2 hr for
   Phase A (write new consumer side-by-side), another ~1 hr for
   Phase B verify + Phase C disable.
2. **Sandbag `PunchWeak` counter-punch.** Even smaller scope; the
   sandbag is a debug entity so visual regressions are easy to
   catch.
3. **Striker family `Swipe` melee.** Multiple archetypes share
   `MeleeBrute`; flipping the consumer flips them all at once.
4. **Boss `Bolt` / `Rock` ranged.** Lifts gnu_ton's hand-spawn
   actions into the message stream.
5. **`update_player_*` consumes `ActorControlFrame`.** Touches the
   engine boundary; do after enemy/boss consumers are stable so
   the player can ride the same pipeline.
6. **Delete `sandbox_update`.** Split into two systems with the
   `SandboxResetThisFrame` resource short-circuit; both consume
   `ActorControlFrame` via the (by-then-migrated) `update_player_*`.
7. **`BossRuntime` retirement.** Largest scope; per-boss bespoke
   pattern state-machines move into `BossPattern` brain + per-
   encounter RON schedules.

## Validation harness already in place

- `BrainActionCounter` resource (per `brain/mod.rs:300`) tracks
  per-frame `ActorActionMessage` counts. Non-zero `last_frame` ==
  resolver firing.
- Player end-to-end tests in `player/systems.rs::tests`:
  `player_attack_press_emits_swipe_action_message_end_to_end`,
  `player_projectile_release_emits_ranged_bolt_action_message_end_to_end`.
- Enemy / boss pin tests in
  `content/features/conversion_tests.rs` (200+ lines, all archetype
  combinations).

## What NOT to do

- Don't try to delete `sandbox_update` until `update_player_*`
  consumes `ActorControlFrame`. The current orchestration is the
  one place that derives `InputState` from `ControlFrame`; deleting
  it without an upstream migration moves the bypass elsewhere.
- Don't migrate every consumer in one PR. The
  overlap-then-delete pattern explicitly wants per-variant churn so
  trace + parity comparisons are bounded.
- Don't add new `MessageReader<ActorActionMessage>` consumers
  without first writing the parity test (`fork the canary`).

## Cross-references

- [`docs/recipes/extending-brains-and-action-sets.md`](../../docs/recipes/extending-brains-and-action-sets.md) §"Daytime EFFECTS-consumer flip" — concrete procedure with consumer skeleton.
- [`TODO-controllable-entity.md`](../../TODO-controllable-entity.md) — overall plan + chunks 1–4f checklist.
- [`docs/adr/0016-actor-unification.md`](../../docs/adr/0016-actor-unification.md) — universal-brain section.
- [`dev/journals/ae-player-field-usage-2026-05-24.md`](ae-player-field-usage-2026-05-24.md) — 38-call-site `authority.player.*` audit (the `ae::Player` decomposition input).
- [`dev/benchmark-candidates/bevy-ecs-stale-component-after-sync-removal-2026-05-15.md`](../benchmark-candidates/bevy-ecs-stale-component-after-sync-removal-2026-05-15.md) — overlap-then-delete pattern.
