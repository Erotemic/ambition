# Actor/brain migration — follow-up plan for the two remaining tasks

Written 2026-05-25 after the multi-session migration push that
landed: `sandbox_update` deletion, player polarity flip,
single-producer-of-intent for enemies + bosses,
`EnemyArchetypeSpec` consolidation, enemy ranged + enemy melee
start + player melee migrations.

Two architectural items remain. This document is the concrete
attack plan for both, written while the migration context is
still fresh. Read the **principles** first — they constrain
*how* both tasks are done, not just *what* is done.

---

## Principles (do not violate these, even under time pressure)

1. **Systems do the work.** Per-tick logic is a Bevy system, not a
   method on a runtime struct.
2. **Components / resources hold state.** Timers, HP mirrors, phase
   cursors, hit-once sets — all components on the entity, not
   fields buried inside a god-runtime. Transitional use of existing
   runtime timers is acceptable only when it is clearly being used
   to preserve semantics during migration.
3. **Events / messages represent edges.** "Attack requested",
   "attack became active", "hitbox hit something" — these are
   messages or one-shot spawns, not field flips polled next frame.
4. **Specs describe capabilities (data, not code).** No imperative
   callbacks inside `MeleeActionSpec` / `RangedActionSpec` /
   `SpecialActionSpec` / `BossPatternStep`. If you find yourself
   wanting `Box<dyn Fn>` in a spec, stop and design a
   registry/plugin instead.
5. **Effects are entities/systems, not hidden runtime side
   effects.** A boss apple-rain is N hitbox/projectile entities, not
   a private loop inside `BossRuntime::tick_apple_rain`. The boss's
   *intent* to apple-rain is a message; the *effect* is what a
   consumer system spawns.
6. **Schedule order replaces hand-written orchestration.** No
   single mega-system that calls phases sequentially. The
   `clear_reset → control → simulation → damage` chain replaced
   `sandbox_update` by trusting Bevy's `.chain()` — same pattern
   here.

### Explicit anti-patterns to NOT introduce

* **Do not** let `ActorControlFrame` grow into a 50-field "everything
  about the actor" bag. The polarity flip needed those player verbs;
  most future fields belong in messages or per-actor components,
  not the frame. If you're about to add a 4th `*_pressed` for a
  one-off enemy/boss action, push it into an `ActionRequest`
  variant via `melee_pressed` / `fire` / `special_pressed` instead.
* **Do not** keep `EnemyRuntime` / `BossRuntime` as the real AI
  forever. The current state — "runtime is the single intent
  producer, brain is a placeholder" — is a halfway-house. The
  endgame is: brain decides; runtime holds physical/combat state
  the brain reads via snapshot.
* **Do not** invent a `tick_all_actors_system` that switches on
  archetype and dispatches. That's `sandbox_update` generalized.
  The actor pipeline is already a *schedule* of small systems,
  one per concern.
* **Do not** make `BossPatternStep` carry a `Box<dyn Fn(&mut BossRuntime)>`
  or any other callback. Steps describe what the boss *wants*
  this tick, and the EFFECTS consumer system decides what to spawn
  for each resolved action.
* **Do not** duplicate boss special identity in both the boss schedule
  and the actor `ActionSet`. Pick one ownership rule. The preferred
  rule here is: boss schedules emit abstract special slots, and
  `ActionSet` resolves those slots into concrete `SpecialActionSpec`s.

---

## Task A — Enemy melee damage as a hitbox-entity lifecycle

### Current state (what we have)

* `ActorActionMessage::Melee { spec, origin, facing, attack_axis }`
  arrives at `start_enemy_melee_from_brain_actions` (Combat set).
* That consumer calls `EnemyRuntime::begin_melee_attack(tuning)`
  which sets `attack_windup_timer`, `attack_cooldown`, and
  `ai_mode = Telegraph` on the runtime.
* `update_ecs_actors` ticks the windup → active → cooldown phases
  on the runtime each frame and calls `enemy.player_damage(player_body)`
  to *poll* a per-tick AABB overlap during the active window.
  On hit, emits a `PlayerDamageEvent`.

The polling-and-overlap-test inside the runtime is the bypass.

### Target state (what we want)

A melee strike becomes an explicit attack lifecycle. The melee
message starts the attack; the attack lifecycle owns windup →
active → recovery timing; the **windup → active edge** spawns one
or more **Hitbox entities** that live for `spec.active_s` seconds.
A separate system tests overlap against targets each tick and emits
damage events on hit.

The runtime may keep transitional timers for animation / AI gating,
but it stops doing the damage check.

Important semantic distinction:

```text
ActorActionMessage::Melee means "start/commit this melee action".
It does not necessarily mean "spawn active damage on this exact frame".
```

Active damage should be spawned when the attack lifecycle reaches
its active phase.

### Concrete components / messages / systems

```rust
// In content/features/combat/hitbox.rs (new module)

#[derive(Component, Clone, Copy, Debug)]
pub struct Hitbox {
    /// Spawned-by entity. Used to skip self-hits and to look up
    /// source context for knockback origin, faction routing, etc.
    pub owner: Entity,
    /// Whose damage events does this fire? Enemy hitboxes hit
    /// players; player hitboxes hit enemy actors. Bosses can hit
    /// players via this too once their attacks migrate.
    pub source: ActorFaction,
    /// Where is this hitbox anchored?
    pub anchor: HitboxAnchor,
    pub half_extent: Vec2,
    pub damage: i32,
    pub knockback_strength: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum HitboxAnchor {
    /// Default for melee swings. The hitbox follows the owner's
    /// current authoritative position each frame.
    FollowOwner {
        local_offset: Vec2,
    },
    /// Default for arena hazards, apple-rain impacts, traps, etc.
    /// This is a fixed world-space rectangle.
    World {
        center: Vec2,
    },
}

#[derive(Component, Clone, Copy, Debug)]
pub struct HitboxLifetime {
    pub remaining_s: f32,
}

#[derive(Component, Default, Debug)]
pub struct HitboxHits {
    /// Entities already hit by this hitbox this strike. Stops a
    /// long active window from double-hitting the same target.
    pub hit: std::collections::HashSet<Entity>,
}
```

Preferred attack lifecycle component:

```rust
#[derive(Component, Clone, Debug)]
pub struct MeleeAttackInstance {
    pub spec: MeleeActionSpec,
    pub phase: MeleeAttackPhase,
    pub elapsed_s: f32,
    pub spawned_active_hitbox: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeleeAttackPhase {
    Windup,
    Active,
    Recovery,
}
```

Transitional acceptable shape:

```text
ActorActionMessage::Melee
  -> call EnemyRuntime::begin_melee_attack(...)
  -> runtime/component lifecycle advances windup/active/recovery
  -> active edge spawns Hitbox
```

Preferred final shape:

```text
ActorActionMessage::Melee
  -> spawn/update MeleeAttackInstance
  -> advance_melee_attack_instances
  -> active edge spawns Hitbox
  -> hitbox systems apply damage
```

Systems, all Bevy systems with narrow params, registered in the
Combat set, **chained** in this order:

```text
start_enemy_melee_attacks_from_brain_actions
  reads MessageReader<ActorActionMessage>
  filter Melee + Hostile + attacks_player
  start a MeleeAttackInstance OR call EnemyRuntime::begin_melee_attack
    as a transitional compatibility bridge
  do NOT spawn active damage immediately unless the spec has no windup

advance_melee_attack_lifecycles
  advances windup / active / recovery state
  on Windup -> Active edge:
    spawn (Hitbox, HitboxLifetime { spec.active_s }, HitboxHits::default())
    with HitboxAnchor::FollowOwner { local_offset }

apply_hitbox_damage
  query: (Entity, &Hitbox, &mut HitboxHits) + actor/player positions
  for each hitbox:
    compute world AABB from HitboxAnchor:
      FollowOwner => owner pos + local_offset
      World => center
    test overlap against valid targets
    skip targets already in HitboxHits.hit
    emit PlayerDamageEvent / FeatureDamageEvent on hit
    insert hit-target Entity into HitboxHits.hit

advance_hitbox_lifetimes
  for &mut HitboxLifetime: remaining_s -= dt

despawn_expired_hitboxes
  despawn entities with remaining_s <= 0
```

Ordering note: `apply_hitbox_damage` intentionally runs before
lifetime cleanup so a hitbox spawned this frame always receives at
least one damage pass, even if its lifetime is very short.

### What to delete

* `EnemyRuntime::player_damage(player_body)` — caller in
  `update_ecs_actors` is gone, polling overlap moves to the
  hitbox system.
* The `if player_vulnerable && enemy.alive { if let Some(damage) = enemy.player_damage(...) ... }`
  block in `update_ecs_actors`.
* `EnemyRuntime::attack_aabb()` — replaced by
  `HitboxAnchor::FollowOwner { local_offset } + half_extent`
  baked into the active hitbox spawn from `MeleeActionSpec`.
  Keep `attack_telegraph_aabb()` if the debug overlay still uses
  it for telegraph visualization.

### What stays in `EnemyRuntime`

* Transitional acceptable:

  * `attack_windup_timer`, `attack_timer`, `attack_cooldown` —
    these may stay briefly to preserve animation / AI gating while
    the hitbox lifecycle is introduced.
* Preferred final:

  * active attack lifecycle moves to `MeleeAttackInstance`
  * runtime exposes only the combat/body state the brain snapshots
    to decide whether `melee_pressed` should be emitted.
* `body_damage_aabb()` + body-contact damage — this is "you ran
  into the enemy", not "the enemy swung at you". Keep as a
  per-tick contact check for now.

### Hit-once semantics

`HitboxHits` is the natural ECS shape for "this strike already hit
target X — don't re-hit." Pre-migration the runtime's
`attack_timer > 0` test could double-hit a player who stayed
inside the AABB across multiple ticks. The HashSet inside the
hitbox component fixes that without the runtime needing per-strike
hit-tracking.

### Tests to add (canaries)

```text
melee_message_starts_attack_lifecycle
melee_attack_spawns_hitbox_on_active_edge_not_request_edge
melee_hitbox_lifetime_matches_spec_active_s
hitbox_despawns_after_active_s
hitbox_overlap_emits_player_damage_event
hitbox_hits_each_target_at_most_once_per_strike
follow_owner_hitbox_world_aabb_follows_owner_each_frame
world_hitbox_uses_fixed_world_center
hitbox_with_dead_owner_despawns_cleanly
```

The body-contact damage test (`enemy.body_damage_aabb()` overlap)
stays as-is — it's not part of this migration.

### Estimated cost

2–3 hours. Most of the time is in the attack lifecycle edge,
`apply_hitbox_damage` query shape, target-filter logic, faction
routing, and the canary tests.

### Player melee (parallel slice — recommended to land in the same session)

The player's `ActivePlayerAttack` lifecycle is structurally
identical to enemy melee: windup → active → cooldown timers
inside a runtime-like struct, with hit detection done by a polled
overlap. Once enemy melee uses `Hitbox` entities, **do the same
for player melee in the same session.** Player melee already
flows through `ActorActionMessage::Melee` via the
`attack_advance_system` gate; the missing piece is replacing
`attack_advance_system`'s own per-tick overlap check with a
hitbox-entity spawn on the attack active edge.

If the unified `Hitbox` component carries a `source: ActorFaction`
field, `apply_hitbox_damage` handles both directions with one
system. The faction tag picks the target query.

---

## Task B — BossPattern brain emits real per-phase intent

### Current state (what we have)

* `tick_boss_pattern` returns neutral.
* `BossRuntime::build_control_frame` only sets `desired_vel`.
* The bespoke attack timelines (`update_scripted_attacks`,
  `tick_apple_rain`, etc.) live inside `BossRuntime`.
* `update_ecs_bosses` calls `BossRuntime::update`, which returns
  a frame tagged with `melee_pressed` / `fire = Some(...)` *iff*
  the runtime decided to attack this tick. The frame lands in
  `ActorControl`; the resolver emits the matching
  `ActorActionMessage`s. **The boss INTENT is visible in the
  message stream; the EFFECT is still bespoke inside the runtime.**

### Target state (what we want)

The BossPattern brain reads the encounter's per-phase schedule
(data, RON-authored per ADR 0017), the boss's current target
position, and the boss's body state, and emits the right
`ActorControlFrame` flags each tick. `BossRuntime` keeps HP /
phase / body state the brain reads; apple-rain etc. spawn loops
move into EFFECTS consumers driven by resolved
`ActorActionMessage::Special`.

Boss schedule state belongs in a `BossPatternState` /
brain-state component, not as bespoke scripted-step fields buried
inside `BossRuntime`.

### Concrete schema (data, no callbacks)

```rust
// In brain/state_machine.rs (extend BossPatternCfg)

#[derive(Clone, Debug)]
pub struct BossPatternCfg {
    pub aggressiveness: f32,
    pub encounter_id: String,
    /// Per-phase scripted schedule. The brain advances a cursor
    /// through these per tick.
    pub schedule: BossSchedule,
}

#[derive(Clone, Debug)]
pub struct BossSchedule {
    pub phase1: Vec<BossPatternStep>,
    pub phase2: Vec<BossPatternStep>,
    pub enrage: Vec<BossPatternStep>,
    /// Intro/Stagger/Death/Dormant — what does the brain emit
    /// during these? Default to "neutral" (no intent).
}

#[derive(Clone, Copy, Debug)]
pub enum BossPatternStep {
    /// Hold position for N seconds. Emits desired_vel=0.
    Hold { seconds: f32 },
    /// Move toward target at speed; advance when within radius.
    ApproachTarget { speed: f32, settle_radius: f32 },
    /// Telegraph a melee strike for windup_s, then emit
    /// melee_pressed for one tick. Cursor advances after
    /// windup_s + 1 frame.
    Melee { windup_s: f32 },
    /// Telegraph a projectile volley: N shots fired at the
    /// target with cadence. Each "fire" tick emits fire=Some(dir).
    ProjectileVolley { shots: u32, cadence_s: f32, dir_strategy: DirStrategy },
    /// Ask the actor to perform its configured special slot.
    /// ActionSet owns the concrete mapping from this slot to
    /// SpecialActionSpec::GnuAppleRain / MockingbirdSwoop /
    /// ClockworkSpotlight / etc.
    Special { slot: SpecialSlot, seconds: f32 },
}

#[derive(Clone, Copy, Debug)]
pub enum DirStrategy {
    AtTarget,
    Downward,           // apple-rain style if represented as ranged/projectile
    Spread { count: u32, arc_deg: f32 },
    // Add more when a step needs them — no callbacks.
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpecialSlot {
    Primary,
    Secondary,
    Tertiary,
}
```

`BossPatternState` extends with `step_index: usize`,
`step_elapsed: f32`, and the last-seen phase so phase transitions
can reset cursor state deliberately.

```rust
#[derive(Component, Clone, Debug, Default)]
pub struct BossPatternState {
    pub step_index: usize,
    pub step_elapsed: f32,
    pub last_phase: Option<BossEncounterPhase>,
}
```

### Per-tick brain logic

```text
tick_boss_pattern(cfg, state, snapshot, out):
  phase = snapshot.encounter_phase or BossPhase component
  if state.last_phase != Some(phase):
    state.step_index = 0
    state.step_elapsed = 0
    state.last_phase = Some(phase)

  steps = schedule.steps_for_phase(phase)
    // The brain needs encounter_phase. Either:
    //   (a) Add encounter_phase to BrainSnapshot
    //   (b) Add a per-actor `BossPhase` component the brain reads
    //       through the boss-brain ticking system
    // (b) is more ECS-native and avoids snapshot bloat.

  if steps.is_empty(): return  // phase has no script, e.g. Stagger

  step = steps[state.step_index]
  state.step_elapsed += snapshot.dt
  duration = step_duration(step)

  if state.step_elapsed >= duration:
    state.step_elapsed -= duration
    state.step_index = (state.step_index + 1) % steps.len()
    return  // next tick fills the new step

  match step:
    Hold => out.desired_vel = 0

    ApproachTarget { speed, settle_radius } =>
      delta = snapshot.target_pos - snapshot.actor_pos
      if delta.length() > settle_radius:
        out.desired_vel = delta.normalize_or_zero() * speed

    Melee { windup_s } =>
      // Emit melee_pressed ONCE at the end of windup.
      if state.step_elapsed >= windup_s &&
         state.step_elapsed < windup_s + snapshot.dt:
        out.melee_pressed = true

    ProjectileVolley { shots, cadence_s, dir_strategy } =>
      // Emit fire=Some(dir) at each shot tick.
      shot_index = (state.step_elapsed / cadence_s) as u32
      if shot_index < shots && fires_this_tick(state.step_elapsed, cadence_s):
        out.fire = Some(ActorFireRequest {
          dir: resolve_dir(dir_strategy, snapshot),
          speed: 0.0,  // resolved by ActionSet
        })

    Special { slot, .. } =>
      // Emit special_pressed / selected special slot for this
      // step's first tick. ActionSet resolves the slot to a
      // concrete SpecialActionSpec.
      if state.step_elapsed < snapshot.dt:
        out.special_pressed = true
        out.special_slot = Some(slot)  // or equivalent non-frame-side channel
```

If `ActorControlFrame` currently has only `special_pressed`, avoid
growing it into a one-off boss bag. Prefer either:

```text
ActionSet has only one special for the actor, so special_pressed is enough.
```

or:

```text
BossPattern emits an ActorActionMessage / ActionRequest with a selected slot
through the resolver without adding many special-case booleans to the frame.
```

### EFFECTS consumers per `SpecialActionSpec`

One small Bevy system per concrete `SpecialActionSpec` variant — in
`brain_effects.rs` alongside `spawn_enemy_projectiles_from_brain_actions`:

```text
spawn_gnu_apple_rain_from_special_messages
  reads MessageReader<ActorActionMessage>
  filter Special with spec == SpecialActionSpec::GnuAppleRain
  for each: spawn N world-anchored or velocity-driven hitbox/projectile
    entities with lifetime/gravity as appropriate

spawn_mockingbird_swoop_from_special_messages
  reads MessageReader<ActorActionMessage>
  filter Special with spec == SpecialActionSpec::MockingbirdSwoop
  spawn the corresponding effect entities

spawn_clockwork_spotlight_from_special_messages
  reads MessageReader<ActorActionMessage>
  filter Special with spec == SpecialActionSpec::ClockworkSpotlight
  spawn the corresponding effect entities
```

These consumers replace `BossRuntime::tick_apple_rain` etc. The
runtime keeps HP + phase + body state but stops the bespoke spawn
loops.

Ownership rule:

```text
BossPatternStep says "use special slot Primary".
ActionSet says "Primary means GnuAppleRain for gnu_ton".
Effect consumers execute SpecialActionSpec::GnuAppleRain.
```

Do not also encode `GnuAppleRain` inside the schedule step unless
you intentionally decide boss scripts should bypass ActionSet.

### What to delete (incrementally — one boss at a time)

1. Migrate `GnuAppleRain` first (smallest, most pattern-isolated):

   * Author `gnu_ton`'s `BossSchedule` (RON or Rust constant).
   * Configure `gnu_ton`'s `ActionSet` so its primary special maps
     to `SpecialActionSpec::GnuAppleRain`.
   * Wire `spawn_gnu_apple_rain_from_special_messages` consumer.
   * Delete `BossRuntime::tick_apple_rain` + `apple_spawn_*` fields.
   * Delete any `frame.fire = Some(downward)` or bespoke special
     tagging from `BossRuntime::update` that existed only to drive
     apple rain.
2. Mockingbird + clockwork_warden follow the same recipe.
3. When all 3 are migrated, delete:

   * `BossRuntime::update_cycle_attacks`
   * `BossRuntime::update_scripted_attacks`
   * The `outputs.projectile_spawns` flush in `update_ecs_bosses`
   * `BossTickOutputs` struct, if it has become only a mirror of
     the old runtime-driven effect path.

### What stays in `BossRuntime`

* `health`, `rider_health` — HP state.
* `encounter_phase` — phase tracking.
* `pos`, `vel`, `combat_size`, `is_active_visible_player_pos` —
  body state, used by `step_kinematic`.
* Transitional acceptable:

  * `attack_windup_timer`, `attack_timer`, `attack_cooldown` may
    stay while melee attack lifecycles are being unified.
* Preferred final:

  * phase cursor / scripted-step state lives in `BossPatternState`
  * effect spawning lives in effect consumers
  * runtime is body + HP + phase, not behavior policy.

### Phase reset semantics

Define cursor reset explicitly:

```text
When BossEncounterPhase changes, reset BossPatternState step_index
and step_elapsed unless that transition explicitly opts into
preserving the cursor.
```

Reset on:

```text
phase1 -> phase2
phase2 -> enrage
active phase -> stagger
stagger -> active phase
encounter reset
boss despawn/respawn
boss schedule hot reload
```

This prevents phase transitions from entering a new phase halfway
through an unrelated step or firing a special immediately after a
stagger unless the schedule explicitly wants that behavior.

### Schema → RON migration (ADR 0017's deferred half)

Once `BossSchedule` is a Rust type with `Serialize + Deserialize`
derives, extend the existing
`assets/data/boss_encounters/<id>.ron` files (currently numeric
fields only) with a `schedule:` field. The `BossEncounterSpec` /
`BossPattern` cfg can then come entirely from RON. ADR 0017's
"per-phase brain schedules" follow-up closes when this lands.

Add validation before treating RON-authored schedules as trusted:

```text
validate_boss_schedule:
  active phases are non-empty unless intentionally neutral
  all Hold/Special durations > 0
  Melee windup_s >= 0
  ProjectileVolley shots > 0
  ProjectileVolley cadence_s > 0
  Spread count > 0
  referenced SpecialSlot is bound in the boss ActionSet
  referenced SpecialActionSpec has a registered effect consumer
```

### Tests to add (canaries)

```text
boss_pattern_brain_with_empty_schedule_emits_neutral
boss_pattern_brain_advances_step_cursor_after_duration
boss_pattern_state_resets_on_phase_change
boss_pattern_brain_emits_melee_at_end_of_windup
boss_pattern_brain_emits_fire_per_shot_in_volley
boss_pattern_brain_emits_special_once_per_step_start
boss_special_slot_resolves_through_action_set
gnu_apple_rain_consumer_spawns_n_hitboxes
mockingbird_schedule_round_trips_through_ron
invalid_boss_schedule_reports_validation_error
```

### Estimated cost

4–6 hours total:

* 1–2h: `BossPatternStep` + `BossSchedule` types + `tick_boss_pattern`
  implementation + brain canary tests.
* 1h: `BossPhase` / `BossEncounterPhase` routing +
  `update_ecs_bosses` integration.
* 2h: First boss (`gnu_ton`) full migration — author schedule,
  configure special slot, wire `spawn_gnu_apple_rain_from_special_messages`
  consumer, delete `tick_apple_rain`, write end-to-end test.
* 1h: Mockingbird + clockwork_warden following the same pattern.
* 0.5h: ADR 0017 update + RON schema extension + validation.

---

## Common architectural notes

### Schedule placement

Both consumers (`start_*_melee_attacks_from_brain_actions`,
`spawn_*_from_special_messages`) belong in the **Combat set**,
chained AFTER `emit_brain_action_messages` has produced the action
stream. The existing ranged + melee-start consumers are already
there — same pattern.

Recommended Combat order:

```text
start attack lifecycles from ActorActionMessage
advance attack lifecycles and spawn hitboxes on active edges
spawn ranged/special effect entities from ActorActionMessage
apply hitbox damage
advance hitbox lifetimes
despawn expired hitboxes
```

This keeps message consumption, active-edge spawning, damage, and
cleanup deterministic.

### Faction routing

Both player and enemy hitboxes use the same `Hitbox` component
with a `source: ActorFaction` tag. `apply_hitbox_damage` filters
by faction:

```text
source == Enemy
  -> target query is PlayerBody / player hurtbox
  -> emit PlayerDamageEvent

source == Player
  -> target query is hostile actor/body/runtime hurtboxes
  -> emit FeatureDamageEvent or equivalent hostile-damage event

source == Boss
  -> target query is PlayerBody / player hurtbox
  -> emit PlayerDamageEvent

source == Npc
  -> no-op for now
```

Peaceful NPCs should not spawn hitboxes unless they have explicitly
flipped to a hostile actor/action set.

### "Hitbox follows owner" vs "Hitbox is a world rectangle"

The follow-owner pattern is the right default for melee swings —
the swing tracks the actor's position. Some attacks, such as
apple-rain apples, scripted arena hazards, or traps, are better as
world-anchored or projectile-like entities with their own velocity
/ lifetime.

Represent this distinction in components, not only in comments:

```text
HitboxAnchor::FollowOwner { local_offset }
HitboxAnchor::World { center }
optional Velocity / Gravity components for projectile-like hazards
```

Do not force every effect through the follow-owner shape.

### When you find yourself wanting a callback in a spec

If `BossPatternStep` needs to do something genuinely custom for
one boss — don't add a callback. Instead:

1. Emit an abstract slot/verb from the schedule.
2. Resolve that through `ActionSet` to a concrete `SpecialActionSpec`.
3. Add a new EFFECTS consumer for that `SpecialActionSpec`.

The cost is one match arm / consumer per unique attack flavor,
which is strictly better than `Box<dyn Fn>` for trace logging, save
serialization, RL determinism, and human-readable debugging.

### Sizing `ActorControlFrame` deliberately

`ActorControlFrame` now carries enough for player input + abstract
combat verbs. **Resist adding fields for per-actor specialness.**
If a future actor, boss, possessed body, or RL agent needs to
express something `melee_pressed` / `fire` / `special_pressed`
can't cover, the right move is usually:

```text
Add a new ActionRequest variant richer than the abstract verb.
```

or:

```text
Add a component on the actor entity that the EFFECTS consumer
queries alongside the message.
```

Not:

```text
Add a new boolean to ActorControlFrame for one boss.
```

---

## Optional tracing/debug hook

This migration is also laying groundwork for a future reusable
Bevy brain/editor crate. Add a tiny trace hook if it is cheap:

```rust
pub struct BrainTraceEvent {
    pub actor: Entity,
    pub brain_kind: BrainTraceKind,
    pub state_or_step: Option<String>,
    pub emitted_intent: Option<String>,
    pub resolved_action: Option<String>,
}
```

Use it to record:

```text
boss step selected
intent emitted
ActionSet resolved special slot
hitbox spawned
hitbox hit target
```

This is not a full editor task. It is just enough trace data to
debug the migration and later feed a visual brain-state inspector.

---

## Order of operations

1. **Task A first.** Hitbox lifecycle is the smaller surgery and
   unlocks player melee using the same component. Land it +
   player melee in the same session.
2. **Task B second.** BossPattern schedule is bigger, and benefits
   from having `Hitbox` already in the toolbox. The special
   consumers will spawn hitboxes/projectiles too.

After both land:

```text
EnemyRuntime is body + transitional timer state + body-contact AABB.
BossRuntime is HP + phase + body state.
Brain is the universal intent producer for every actor type.
ActionSet is the universal capability resolver.
Effects are entities + systems.
ADR 0017's deferred boss-schedule half closes.
The migration is fully done.
```

## Where this document lives + how to find it

`dev/journals/actor-brain-migration-followups-plan.md` — paired
with `dev/journals/actor-brain-migration-completion-2026-05-24.md`
(the completion journal for the work already landed). If you're
picking up the migration in a future session, read the completion
journal first for context, then this plan for the next two
slices.