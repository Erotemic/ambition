# Character AI refactor

Status: enemy intent migration landed, boss/data-table migration pending.

This is the companion doc referenced from
`crates/ambition_engine/src/character_ai.rs`. It captures the current
state of the shared character-AI vocabulary and the path forward for
making it the single source of truth for hostile NPCs, enemies, and
bosses.

## Why this exists

The sandbox grew three nearly-parallel "AI loops":

- `EnemyRuntime` (sandbox `features.rs`) — striker / brute /
  fastfall / ranger / sandbag brains, each with their own ad-hoc
  timer fields and per-brain match arms.
- `BossRuntime` (sandbox `features.rs`) — boss patterns mostly run
  off `BossPatternStep` schedules but still hand-roll the
  surrounding aggro / telegraph / recover logic.
- Hostile NPC conversion (`features::apply_save`) — once an NPC
  is flagged hostile, the save layer replaces it with an
  `EnemyRuntime` so it inherits the enemy AI by construction.

The third path is the desired shape: one AI loop, parameterized by
data, that every combatant actor consumes. The first two paths are
still timer-field-driven. That means a behavior change like "telegraph
now flashes a ring" or "stunned actors don't accept pogo bounces"
has to be re-implemented in N places, and the headless / RL test
harness can't drive any of it without spinning up the full sandbox.

## What landed

`ambition_engine::character_ai` is the pure-data evaluator:

- `CharacterAiSnapshot` — the read-only view (positions, ranges,
  attack-window timers, stun, alive, `patrol_enabled`).
- `CharacterAiMode` — the canonical mode the actor should be in
  this tick (`Idle | Patrol | Chase | Telegraph | Attack | Recover
  | Stunned | Dead`), with helpers `label`, `is_dangerous`,
  `is_committed`.
- `CharacterAiIntent` / `CharacterAiOutput` — the coarse hold / patrol /
  chase / attack intent paired with the mode. Sandbox code still supplies
  speeds and collision, but this output is the authority for which coarse
  behavior branch runs.
- `evaluate_character_ai(snapshot) -> mode` and
  `evaluate_character_ai_output(snapshot) -> output` — deterministic,
  Bevy-free functions with unit tests that exercise the priority order
  (dead > stunned > active attack > windup > recover > in-range > aggro >
  patrol > idle).

The seldom_state component vocabulary in `state_machines`
(`EnemyIdle / EnemyPatrol / EnemyTelegraph / EnemyAttack /
EnemyRecover / EnemyStunned / EnemyDead`) is the *per-entity* mirror
of the same shape, so when migration happens the component types
already exist.

## What hasn't landed

`EnemyRuntime` now builds a `CharacterAiSnapshot` and consumes
`CharacterAiOutput` for its coarse hold / patrol / chase / attack branch.
That makes the shared engine evaluator authoritative for standard enemy
intent. Remaining enemy work is data-table cleanup: archetype-specific
speeds, aggro ranges, attack ranges, cooldown multipliers, and damage still
live in sandbox enum matches.

`BossRuntime` still has its own pattern/state loop. See the open tech-debt
entry "`EnemyRuntime` and `BossRuntime` carry their own ad-hoc state
machines" for the boss half.

## Migration target

The eventual shape:

1. Each combatant runtime exposes `snapshot(&self, player) ->
   CharacterAiSnapshot` and `apply(mode: CharacterAiMode, dt) ->
   AiActionEvents` (move-toward, start-windup, do-attack,
   start-recover, …).
2. `evaluate_character_ai` chooses the mode; per-brain data
   (chase speed, attack hitbox, telegraph tint, sound id) is read
   from a shared `EnemyArchetype` / `BossArchetype` table rather
   than an enum match in the update fn.
3. Boss-specific patterns stay layered on top: `BossPatternStep`
   becomes a *driver* that overrides the snapshot's
   `attack_windup_remaining` / `attack_active_remaining` /
   `attack_recover_remaining` fields, and `evaluate_character_ai`
   consumes the override naturally.
4. seldom_state components in `state_machines` get written from
   the evaluator's output once per tick so HUD / animation
   pickers can query by component type.

That refactor is meaningful surgery — it touches every enemy
behavior test plus the boss encounter integration test — so it is
intentionally not scoped to a single patch. Doing it in two steps:

- Step A: route `EnemyRuntime::update` through
  `evaluate_character_ai_output` and assert the resulting intent drives
  chase / patrol / attack behavior. **Done for standard enemies.**
- Step B: move per-brain knobs (`chase_speed`, `attack_radius`,
  `telegraph_seconds`, …) from the brain/archetype match arms into a small
  data table; delete the duplicate match arms.

Step B unlocks data-driving new enemies without code changes,
which is the whole point of the refactor.

## Until then

When you add a new enemy or tune an existing one:

- Read `ai_mode` first to figure out the actor's intent for the
  tick. Don't add new bool flags that re-derive that intent.
- Mirror any new mode/transition to `evaluate_character_ai` so the
  pure evaluator stays accurate.
- If you need a per-brain knob the evaluator doesn't expose,
  prefer adding it to `CharacterAiSnapshot` (as input) or
  `CharacterAiMode` (as output) over wiring a parallel field
  through `EnemyRuntime`.

This keeps the eventual migration mechanical instead of a rewrite.
