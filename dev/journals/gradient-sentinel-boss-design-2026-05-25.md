# Gradient Sentinel boss design — 2026-05-25

Polished metroidvania boss fight authored for the Gradient Sentinel
(internal id `clockwork_warden`, audit name `gradient_sentinel`). This
document is the design + tuning sheet; the implementation lives in:

- `src/brain/boss_pattern.rs`        — new BossAttackProfile variants
- `src/brain/action_set.rs`          — new SpecialActionSpec variants
- `src/content/features/bosses.rs`   — Scripted phase 1 / transition / phase 2 / enrage
- `src/content/features/ecs/brain_effects.rs` — EFFECTS consumers
- `src/content/features/boss_attack_geometry.rs` — volume math
- `src/content/features/ecs/bosses.rs` — direct-write Special path

## Narrative & theme

The Gradient Sentinel is a guardian AI that descended too far into a
local minimum. It defends its bad solution with mathematical-themed
attacks. Every move is an *optimization failure* the player must
read and exploit. The fight is the player's first encounter with a
boss that **respects the room geometry**: the arena's drop platforms
and read ledges are part of the puzzle, not scenery.

## Arena (first_system_boss_area)

```text
1280 × 768. Floor y=736. Ceiling y=0-32.
Player start (120, 690). BossSpawn (608, 656). Boss center ~ (640, 696).
Drop-throughs   y=512 (left 288–480, right 800–992).
Upper ledges    y=288 (left 128–352, right 928–1152).
P5 ledge (solid) y=416–432, x=512–768 (slightly left of boss).
```

The boss uses `BossMovementProfile::AnchorSway` — sways ±130 px around
the spawn anchor and chases the player horizontally by 0.18 × distance
(clamped ±70 px). Brain reads `target_pos` from `BrainSnapshot`.

## Attack vocabulary (NEW BossAttackProfile variants)

| Profile           | Telegraph | Strike  | Kind     | Player response                                  |
|-------------------|-----------|---------|----------|--------------------------------------------------|
| `GradientLane`    | 1.4 s     | 1.0 s   | ordinary | Tall vertical column at boss x; jump-over or leave |
| `OverfitVolley`   | 1.4 s     | 0.3 s   | special  | Boss memorizes 5 positions; keep moving           |
| `MinimaTrap`      | 1.0 s     | 0.6 s   | special  | Pit forms at player pos; spawns 1 puppy_slug      |
| `SaddlePoint`     | 1.4 s     | 4.8 s   | special  | Rotating axis cross around boss; perpendicular safe |
| `GradientCascade` | 1.2 s     | 0.4 s   | special  | Spawns 2 small_lurker ("slop") at top of arena    |

Plus existing: `FloorSlam`, `SideSweep`, `FullBodyPulse` (used in phase 1
as familiar baseline patterns).

## Phase script

### Phase 1 (~16 s loop) — fundamentals
```
Telegraph FloorSlam     1.2 s   → ground-pounce wind-up
Strike    FloorSlam     0.40 s  → thin floor slap
Rest                    1.4 s   ← reliable damage window
Telegraph GradientLane  1.4 s   → vertical column outline
Strike    GradientLane  1.0 s   → full-height hazard
Rest                    1.0 s
Telegraph OverfitVolley 1.4 s   → markers track player
Strike    OverfitVolley 0.30 s  → bolts fire at all markers
Rest                    1.5 s   ← deal with bolts then attack
Telegraph SideSweep     0.9 s
Strike    SideSweep     0.40 s
Rest                    2.0 s   ← loop-close breather
```

### Transition (3 s pure rest, music swap)

### Phase 2 (~22 s loop) — hazards + minions
```
Telegraph MinimaTrap    1.0 s   → pit outline at player pos
Strike    MinimaTrap    0.6 s   → pit live + puppy_slug spawns
Rest                    1.4 s   ← kill slug or wait it out
Telegraph SaddlePoint   1.4 s   → cross hazard glows
Strike    SaddlePoint   4.8 s   → rotating arm; long boss exposure
Rest                    1.2 s   ← big punish window
Telegraph GradientCascade 1.2 s → top of arena flashes
Strike    GradientCascade 0.4 s → 2 small_lurkers fall in
Rest                    2.4 s   ← clear minions
Telegraph OverfitVolley 1.2 s   ← faster than phase 1
Strike    OverfitVolley 0.30 s
Rest                    1.4 s
Telegraph FullBodyPulse 1.1 s
Strike    FullBodyPulse 0.5 s
Rest                    1.0 s
```

### Enrage (~10 s loop) — desperate
```
Telegraph MinimaTrap    0.7 s
Strike    MinimaTrap    0.5 s
Telegraph OverfitVolley 0.7 s   ← barely time to react
Strike    OverfitVolley 0.3 s
Rest                    0.6 s
Telegraph SaddlePoint   1.0 s   ← faster rotation period
Strike    SaddlePoint   3.0 s
Telegraph GradientLane  0.7 s
Strike    GradientLane  0.8 s
Rest                    1.2 s
```

## Architectural choices

### Boss-side direct-write of Special messages

Existing `ActionSet.special` carries a single `SpecialActionSpec` —
fine when each boss has one signature move (GNU-ton's apple rain).
Gradient Sentinel has **four** specials, so we bypass the
single-slot ActionSet:

- `tick_boss_brains_system` writes `ActorActionMessage::Special { spec }`
  directly via `MessageWriter` when `BossAttackState.active_profile`
  is `Some(profile)` and `BossAttackProfile::is_special()` is true.
- `boss_special_for_profile(&profile, &boss)` maps the profile to its
  `SpecialActionSpec` (with per-boss tuning).
- The boss's `ActionSet.special` is set to `None` for multi-special
  bosses so the generic resolver doesn't fire a duplicate.
- GNU-ton migrates to the same path (single direct emission), keeping
  the consumer `spawn_gnu_apple_rain_from_special_messages` unchanged.

This keeps `ActorControlFrame` lean (no slot field), keeps `ActionSet`
unchanged (no multi-slot refactor), and isolates the "boss has many
specials" concern in one place — the brain tick — which is the only
place that knows the active_profile anyway.

### Per-boss state components for stateful specials

`OverfitVolley` needs to sample positions across telegraph + fire at
strike start. `SaddlePoint` needs to rotate the active axis over time.
Each gets a component that the EFFECTS consumer mutates:

- `OverfitVolleyState { samples, last_sample_t, fired_this_strike }`
- `SaddlePointState { active_axis, axis_remaining_s, hitbox_h, hitbox_v }`

Both follow the `AppleRainSpawnState` pattern: defaulted-attached to
every boss; only the boss whose schedule fires the matching profile
ever advances it.

### Minion spawning

`spawn_minima_trap_from_special_messages` and
`spawn_gradient_cascade_minions_from_special_messages` call
`spawn_runtime_enemy(commands, archetype, pos)` — a new helper next
to `spawn_enemy` that builds `EnemyRuntime` + components from an
`EnemyArchetype` enum (rather than authored `Authored<EnemyBrain>`).

Puppy slugs are pacifist crawlers (`Wanderer` brain, no attacks) —
they serve as harmless obstacles the player has to walk around or
kill for tempo. Small lurkers ("slop" stand-in until we have real
slop art) are aggressive `MeleeBrute` skitters.

### Why this is data-driven

Adding a new attack flavor for a *future* boss is now four edits:

1. New `BossAttackProfile` variant
2. New `SpecialActionSpec` variant + `is_special` arm if special
3. New entry in `boss_special_for_profile` (one match arm)
4. New EFFECTS consumer system (one file, one schedule registration)

The phase script itself is pure data — no callbacks, no behavior in
the schedule. An AI agent (or designer) can author a new boss by
editing `BossBehaviorProfile::clockwork_warden`'s `BossPattern::steps`
without touching any system code.

## What's deferred

- **Boss-schedule RON migration (ADR 0017 follow-up)**: `BossPattern`
  + `BossAttackProfile` + `SpecialActionSpec` need
  `Serialize + Deserialize` derives before the schedule can come from
  `assets/data/boss_encounters/<id>.ron`. The schedule is a Rust
  constant for now — same shape, different transport.
- **Sprite regeneration**: the existing clockwork_warden sprite reads
  fine. New attack telegraph art (lane glow, saddle cross) would
  improve the read but is presentation-layer work — not part of the
  gameplay slice.
- **Minion archetype "slop"**: using `small_lurker` as stand-in. When
  a real slop archetype is authored (`EnemyArchetype::Slop`), swap
  the spawn target in `GradientCascade`'s consumer.

## Cost estimate vs actual

Estimated 6–8 h for the full slice including tests + commit hygiene.
Actual reported in journal commit messages.
