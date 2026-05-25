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
   fields buried inside a god-runtime.
3. **Events / messages represent edges.** "Attack started",
   "hitbox hit something" — these are messages or one-shot
   spawns, not field flips polled next frame.
4. **Specs describe capabilities (data, not code).** No imperative
   callbacks inside `MeleeActionSpec` / `RangedActionSpec` /
   `SpecialActionSpec` / `BossPatternStep`. If you find yourself
   wanting `Box<dyn Fn>` in a spec, stop and design a
   registry/plugin instead.
5. **Effects are entities/systems, not hidden runtime side
   effects.** A boss apple-rain is N hitbox entities, not a
   private loop inside `BossRuntime::tick_apple_rain`. The boss's
   *intent* to apple-rain is a message; the *effect* is what a
   consumer system spawns.
6. **Schedule order replaces hand-written orchestration.** No
   single mega-system that calls phases sequentially. The
   `clear_reset → control → simulation → damage` chain replaced
   `sandbox_update` by trusting Bevy's `.chain()` — same pattern
   here.

### Explicit anti-patterns to NOT introduce

- **Do not** let `ActorControlFrame` grow into a 50-field "everything
  about the actor" bag. The polarity flip needed those player verbs;
  most future fields belong in messages or per-actor components,
  not the frame. If you're about to add a 4th `*_pressed` for a
  one-off enemy/boss action, push it into an `ActionRequest`
  variant via `melee_pressed` / `fire` / `special_pressed` instead.
- **Do not** keep `EnemyRuntime` / `BossRuntime` as the real AI
  forever. The current state — "runtime is the single intent
  producer, brain is a placeholder" — is a halfway-house. The
  endgame is: brain decides; runtime holds physical/combat state
  the brain reads via snapshot.
- **Do not** invent a `tick_all_actors_system` that switches on
  archetype and dispatches. That's `sandbox_update` generalized.
  The actor pipeline is already a *schedule* of small systems,
  one per concern.
- **Do not** make `BossPatternStep` carry a `Box<dyn Fn(&mut BossRuntime)>`
  or any other callback. Steps describe what the boss *wants*
  this tick (intent), and the EFFECTS consumer system decides
  what to spawn for each variant.

---

## Task A — Enemy melee damage as a hitbox-entity lifecycle

### Current state (what we have)

- `ActorActionMessage::Melee { spec, origin, facing, attack_axis }`
  arrives at `start_enemy_melee_from_brain_actions` (Combat set).
- That consumer calls `EnemyRuntime::begin_melee_attack(tuning)`
  which sets `attack_windup_timer`, `attack_cooldown`, and
  `ai_mode = Telegraph` on the runtime.
- `update_ecs_actors` ticks the windup → active → cooldown phases
  on the runtime each frame and calls `enemy.player_damage(player_body)`
  to *poll* a per-tick AABB overlap during the active window.
  On hit, emits a `PlayerDamageEvent`.

The polling-and-overlap-test inside the runtime is the bypass.

### Target state (what we want)

A melee strike spawns a **Hitbox entity** that lives for
`spec.active_s` seconds. A separate system tests overlap against
players each tick and emits `PlayerDamageEvent` on hit. The
runtime keeps timers for animation / AI gating but stops doing
the damage check.

### Concrete components / messages / systems

```rust
// In content/features/combat/hitbox.rs (new module)

#[derive(Component, Clone, Copy, Debug)]
pub struct Hitbox {
    /// Spawned-by entity. Used to skip self-hits and to look up
    /// the source for knockback origin.
    pub owner: Entity,
    /// Whose damage events does this fire? Enemy hitboxes hit
    /// players; player hitboxes hit enemy actors. Bosses can hit
    /// players via this too once their attacks migrate.
    pub source: ActorFaction,
    /// AABB offset from the owner's pos each frame. The hitbox
    /// FOLLOWS the owner — it isn't a frozen world rectangle.
    pub local_offset: Vec2,
    pub half_extent: Vec2,
    pub damage: i32,
    pub knockback_strength: f32,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct HitboxLifetime { pub remaining_s: f32 }

#[derive(Component, Default, Debug)]
pub struct HitboxHits {
    /// Entities already hit by this hitbox this strike. Stops a
    /// long active window from double-hitting the same target.
    pub hit: bevy::utils::HashSet<Entity>,
}
```

Systems (all Bevy systems with narrow params, registered in the
Combat set, **chained** in this order):

```text
spawn_enemy_melee_hitboxes_from_brain_actions
  reads MessageReader<ActorActionMessage>
  filter Melee + Hostile + attacks_player
  call EnemyRuntime::begin_melee_attack (still — that owns the
    runtime windup/cooldown timers + animation cues)
  spawn (Hitbox, HitboxLifetime { spec.active_s }, HitboxHits::default())
    parented logically to the actor entity via Hitbox.owner

advance_hitbox_lifetimes
  for &mut HitboxLifetime: remaining_s -= dt
  despawn entities with remaining_s <= 0

apply_hitbox_damage
  query: (Entity, &Hitbox, &mut HitboxHits) + actor positions + player body
  for each hitbox:
    compute world AABB from Hitbox.owner pos + local_offset
    test overlap against valid targets (player for enemy hitboxes,
      enemy actors for player hitboxes)
    skip targets already in HitboxHits.hit
    emit PlayerDamageEvent / FeatureDamageEvent on hit
    insert hit-target Entity into HitboxHits.hit
```

### What to delete

- `EnemyRuntime::player_damage(player_body)` — caller in
  `update_ecs_actors` is gone, polling overlap moves to the
  hitbox system.
- The `if player_vulnerable && enemy.alive { if let Some(damage) = enemy.player_damage(...) ... }`
  block in `update_ecs_actors`.
- `EnemyRuntime::attack_aabb()` — replaced by `Hitbox.local_offset
  + half_extent` baked into the spawn from `MeleeActionSpec`.
  Keep `attack_telegraph_aabb()` if the debug overlay still uses
  it for telegraph visualization.

### What stays in `EnemyRuntime`

- `attack_windup_timer`, `attack_timer`, `attack_cooldown` —
  these are runtime/integration state the brain (current or
  future) snapshots into `CombatTimers` to gate its `melee_pressed`.
  AI mode (Telegraph / Attack / Recover) reads them. Don't delete.
- `body_damage_aabb()` + body-contact damage — this is "you ran
  into the enemy", not "the enemy swung at you". Keep as a
  per-tick contact check (still inside `update_ecs_actors`).

### Hit-once semantics

`HitboxHits` is the natural ECS shape for "this strike already hit
target X — don't re-hit." Pre-migration the runtime's
`attack_timer > 0` test could double-hit a player who stayed
inside the AABB across multiple ticks. The HashSet inside the
hitbox component fixes that without the runtime needing per-strike
hit-tracking.

### Tests to add (canaries)

```text
melee_message_spawns_hitbox_with_lifetime_matching_spec
hitbox_despawns_after_active_s
hitbox_overlap_emits_player_damage_event
hitbox_hits_each_target_at_most_once_per_strike
hitbox_world_aabb_follows_owner_each_frame
hitbox_with_dead_owner_despawns_cleanly
```

The body-contact damage test (`enemy.body_damage_aabb()` overlap)
stays as-is — it's not part of this migration.

### Estimated cost

2–3 hours. Most of the time is in `apply_hitbox_damage` (query
shape + target-filter logic + faction routing) and the canary
tests.

### Player melee (parallel slice — recommended to land in the same session)

The player's `ActivePlayerAttack` lifecycle is structurally
identical to enemy melee: windup → active → cooldown timers
inside a runtime-like struct, with hit detection done by a polled
overlap. Once enemy melee uses `Hitbox` entities, **do the same
for player melee in the same session.** Player melee already
flows through `ActorActionMessage::Melee` (via the
`attack_advance_system` gate); the missing piece is replacing the
attack_advance_system's own per-tick overlap check with a
hitbox-entity spawn.

If the unified `Hitbox` component carries a `source: ActorFaction`
field, `apply_hitbox_damage` handles both directions with one
system. The faction tag picks the target query.

---

## Task B — BossPattern brain emits real per-phase intent

### Current state (what we have)

- `tick_boss_pattern` returns neutral.
- `BossRuntime::build_control_frame` only sets `desired_vel`.
- The bespoke attack timelines (`update_scripted_attacks`,
  `tick_apple_rain`, etc.) live inside `BossRuntime`.
- `update_ecs_bosses` calls `BossRuntime::update`, which returns
  a frame tagged with `melee_pressed` / `fire = Some(...)` *iff*
  the runtime decided to attack this tick. The frame lands in
  `ActorControl`; the resolver emits the matching
  `ActorActionMessage`s. **The boss INTENT is visible in the
  message stream; the EFFECT (apple spawns, etc.) is still
  bespoke inside the runtime.**

### Target state (what we want)

The BossPattern brain reads the encounter's per-phase schedule
(data, RON-authored per ADR 0017), the boss's current target
position, and the boss's body state, and emits the right
`ActorControlFrame` flags each tick. `BossRuntime` keeps HP /
phase / pattern-cursor state the brain reads; the apple-rain etc.
spawn loops move into EFFECTS consumers driven by
`ActorActionMessage::Special`.

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
    /// Boss-specific special — the variant tag picks the EFFECTS
    /// consumer. New boss attack flavors add a variant here AND a
    /// matching SpecialActionSpec / consumer.
    Special { kind: BossSpecialKind, seconds: f32 },
}

#[derive(Clone, Copy, Debug)]
pub enum DirStrategy {
    AtTarget,
    Downward,           // apple-rain style
    Spread { count: u32, arc_deg: f32 },
    // Add more when a step needs them — no callbacks.
}

#[derive(Clone, Copy, Debug)]
pub enum BossSpecialKind {
    GnuAppleRain,
    MockingbirdSwoop,
    ClockworkSpotlight,
    // One variant per distinct boss attack flavor. Each variant
    // gets a SpecialActionSpec mirror (or fold into SpecialActionSpec
    // directly) + a per-variant EFFECTS consumer that reads
    // ActorActionMessage::Special and spawns the appropriate
    // hitbox/projectile pattern.
}
```

`BossPatternState` extends with `step_index: usize` and
`step_elapsed: f32` (replacing the equivalent fields currently
inside `BossRuntime.scripted_step_*`).

### Per-tick brain logic

```text
tick_boss_pattern(cfg, state, snapshot, out):
  steps = schedule.steps_for_phase(snapshot.encounter_phase)
    // The brain needs encounter_phase. Either:
    //   (a) Add encounter_phase to BrainSnapshot (extend the
    //       snapshot once, then NPCs/enemies ignore it)
    //   (b) Add a per-actor `BossPhase` component the brain reads
    //       via a new system param
    // (b) is more ECS-native and avoids snapshot bloat.

  if steps.is_empty(): return  // phase has no script (e.g. Stagger)
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
    Special { kind, .. } =>
      // Emit special_pressed for this step's first tick;
      // the SpecialActionSpec stored on the actor's ActionSet
      // carries the matching `kind`.
      if state.step_elapsed < snapshot.dt:
        out.special_pressed = true
```

### EFFECTS consumers per `BossSpecialKind`

One small Bevy system per `SpecialActionSpec` variant — in
`brain_effects.rs` alongside `spawn_enemy_projectiles_from_brain_actions`:

```text
spawn_gnu_apple_rain_from_special_messages
  reads MessageReader<ActorActionMessage>
  filter Special with spec == BossSpotlight (or a new
    SpecialActionSpec::GnuAppleRain variant)
  for each: spawn N apple Hitbox entities (with HitboxLifetime,
    gravity-driven via a new HitboxGravity component or by
    reusing EnemyProjectileSpawn with the apple parameters)

spawn_mockingbird_swoop_from_special_messages
  ...
```

These consumers replace `BossRuntime::tick_apple_rain` etc. The
runtime keeps timers + phase tracking but stops the bespoke
spawn loops.

### What to delete (incrementally — one boss at a time)

1. Migrate `GnuAppleRain` first (smallest, most pattern-isolated):
   - Author `gnu_ton`'s `BossSchedule` (RON or Rust constant).
   - Wire `spawn_gnu_apple_rain_from_special_messages` consumer.
   - Delete `BossRuntime::tick_apple_rain` + `apple_spawn_*` fields.
   - Delete the `frame.fire = Some(downward)` tagging from
     `BossRuntime::update`.
2. Mockingbird + clockwork_warden follow the same recipe.
3. When all 3 are migrated, delete:
   - `BossRuntime::update_cycle_attacks`
   - `BossRuntime::update_scripted_attacks`
   - The `outputs.projectile_spawns` flush in `update_ecs_bosses`
   - `BossTickOutputs` struct (mirror of the enemy cleanup)

### What stays in `BossRuntime`

- `health`, `rider_health` — HP state.
- `encounter_phase` — phase tracking.
- `attack_windup_timer`, `attack_timer`, `attack_cooldown` — the
  brain reads these via snapshot to gate the `Melee` step.
- `pos`, `vel`, `combat_size`, `is_active_visible_player_pos` —
  body state, used by `step_kinematic`.

### Schema → RON migration (ADR 0017's deferred half)

Once `BossSchedule` is a Rust type with `Serialize + Deserialize`
derives, extend the existing
`assets/data/boss_encounters/<id>.ron` files (currently numeric
fields only) with a `schedule:` field. The `BossEncounterSpec` /
`BossPattern` cfg can then come entirely from RON. ADR 0017's
"per-phase brain schedules" follow-up closes when this lands.

### Tests to add (canaries)

```text
boss_pattern_brain_with_empty_schedule_emits_neutral
boss_pattern_brain_advances_step_cursor_after_duration
boss_pattern_brain_emits_melee_at_end_of_windup
boss_pattern_brain_emits_fire_per_shot_in_volley
boss_pattern_brain_emits_special_once_per_step_start
gnu_apple_rain_consumer_spawns_n_hitboxes
mockingbird_schedule_round_trips_through_ron
```

### Estimated cost

4–6 hours total:
- 1–2h: `BossPatternStep` + `BossSchedule` types + `tick_boss_pattern`
  implementation + 6 brain canary tests.
- 1h: `BossPhase` snapshot routing + `update_ecs_bosses` integration
  (drop runtime intent tagging now that brain owns intent).
- 2h: First boss (gnu_ton) full migration — author schedule,
  wire `spawn_gnu_apple_rain_from_special_messages` consumer,
  delete `tick_apple_rain`, write end-to-end test.
- 1h: Mockingbird + clockwork_warden following the same pattern.
- 0.5h: ADR 0017 update + RON schema extension.

---

## Common architectural notes

### Schedule placement

Both consumers (`spawn_*_hitboxes_from_brain_actions`,
`spawn_*_from_special_messages`) belong in the **Combat set**,
chained AFTER `emit_brain_action_messages` (which lives in
PlayerInput). The existing ranged + melee-start consumers are
already there — same pattern.

Hitbox lifetime + damage systems also go in Combat, chained after
the spawn systems so a hitbox spawned this frame can still hit
something this frame (matching the pre-migration latency).

### Faction routing

Both player and enemy hitboxes use the same `Hitbox` component
with a `source: ActorFaction` tag. `apply_hitbox_damage` filters
by faction:
- `source == Enemy` → target query is `Query<&PlayerBody, …>`,
  emit `PlayerDamageEvent`.
- `source == Player` → target query is `Query<(Entity, &mut
  ActorRuntime), …>`, emit `FeatureDamageEvent`.
- `source == Npc` → no-op for now (peaceful NPCs don't spawn
  hitboxes; the hostile-flip changes their ActionSet).

### "Hitbox follows owner" vs "Hitbox is a world rectangle"

The follow-owner pattern (compute world AABB from owner pos +
local offset each frame) is the right default for melee swings —
the swing tracks the actor's position. Some attacks (apple-rain
apples, scripted arena hazards) are better as **independent
projectile-like entities** with their own velocity / lifetime —
those should use `EnemyProjectileSpawn` (already the pattern) or
a new world-anchored hitbox variant. **Don't** force every effect
through the follow-owner shape.

### When you find yourself wanting a callback in a spec

If `BossPatternStep` needs to do something genuinely custom for
one boss — don't add a callback. Instead:
1. Add a new variant to `BossSpecialKind`.
2. Add a new EFFECTS consumer for that variant.
3. The spec stays pure data.

The cost is one match arm per unique attack flavor, which is
strictly better than `Box<dyn Fn>` for trace logging, save
serialization, RL determinism, and human-readable debugging.

### Sizing `ActorControlFrame` deliberately

`ActorControlFrame` now carries enough for player input + abstract
combat verbs. **Resist adding fields for per-actor specialness.**
If a future actor (boss, possessed body, RL agent) needs to
express something `melee_pressed` / `fire` / `special_pressed`
can't cover, the right move is usually:
- Add a new `ActionRequest` variant (richer than the abstract
  verb), OR
- Add a new component on the actor entity that the EFFECTS
  consumer queries alongside the message.

Not: a new boolean on the frame.

---

## Order of operations

1. **Task A first.** Hitbox lifecycle is the smaller surgery and
   unlocks player melee using the same component. Land it +
   player melee in the same session.
2. **Task B second.** BossPattern schedule is bigger, and benefits
   from having `Hitbox` already in the toolbox (the special
   consumers will spawn hitboxes too).

After both land:
- `EnemyRuntime` is just body + timer state + body-contact AABB.
- `BossRuntime` is just HP + phase + pattern cursor + body state.
- The brain is the universal intent producer for every actor type.
- Effects are entities + systems.
- ADR 0017's deferred boss-schedule half closes.
- The migration is fully done.

## Where this document lives + how to find it

`dev/journals/actor-brain-migration-followups-plan.md` — paired
with `dev/journals/actor-brain-migration-completion-2026-05-24.md`
(the completion journal for the work already landed). If you're
picking up the migration in a future session, read the completion
journal first for context, then this plan for the next two
slices.
