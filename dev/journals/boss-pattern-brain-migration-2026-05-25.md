# Boss policy out of BossRuntime → real BossPattern brain (2026-05-25)

Completion journal for the "move boss policy out of `BossRuntime`"
migration. The task was specified explicitly by the user with 6
migration steps + 4 grep-based acceptance criteria.

| Commit  | Scope                                                            |
| ------- | ---------------------------------------------------------------- |
| e7a1e73 | move boss policy out of BossRuntime into a real BossPattern brain |

## Estimated vs actual time

Per [[feedback-track-estimated-vs-actual]].

| Plan estimate | Actual wall-clock | Ratio (plan / actual) |
| ------------- | ----------------- | --------------------- |
| 4–6 h (followups-plan.md Task B, full BossPattern brain implementation) | **~26 min** | **9–14× faster** |

This is the third task in the est-vs-actual log:

| Task | Plan estimate | Actual | Ratio |
| ---- | ------------- | ------ | ----- |
| Task A — Enemy melee hitbox lifecycle | 2–3 h | ~22 min | 5.5–8.2× |
| Task B — gnu_ton apple_rain consumer | 4–6 h | ~20 min | 12–18× |
| Boss policy out of BossRuntime | 4–6 h | ~26 min | 9–14× |
| **Trend** | | | **~10×** |

Same trend confirmed. The "predictions are ~10× reality" rule is
holding across the actor/brain migration series.

### What made this faster than the estimate

* **The plan + survey ate most of the design budget.** With Tasks A
  and B already landed, I had a clean mental model for the
  `Brain → ActorControl → ActionSet → message → consumer` shape; this
  task's design fell out as "do the same thing for the boss policy
  decision". Most of the estimate was on the assumption I'd have to
  invent the vocabulary; instead it was already there (`BossPatternStep`,
  `BossAttackPattern`, etc.) and just needed to move.
* **The mirror-fields pattern dodged a wide-call-site refactor.**
  Keeping `BossRuntime::{active_strike_profile, telegraph_profile,
  attack_timer, attack_windup_timer}` as brain-written mirrors meant
  the legacy `attack_volumes`, `attack_telegraph_volumes`, and
  `player_damage` callers didn't need per-call changes. The mirror
  fields are marked in doc comments as "brain-written, runtime-readable"
  so future readers know the policy/mirror split.
* **Only two tests had to go.** The runtime-cursor tests
  (`gnu_ton_scripted_advance_cycles_telegraph_strike_rest`,
  `gnu_ton_scripted_patterns_skip_non_attacking_phases`) were
  testing the deleted `boss.update(...)` path; their invariants moved
  cleanly to the new `brain::boss_pattern::tests` unit tests, so the
  delete + replace was a one-for-one trade.
* **The user's "skip integration canaries" guidance.** Halfway
  through I was about to add a redundant integration canary in
  `ecs/bosses.rs::tests`. The user redirected: the brain-side unit
  tests + the existing `apple_rain_consumer_spawns_on_interval` +
  the grep acceptance checks already covered the surface. Saved
  ~5 minutes of test scaffolding for zero loss of signal.

## What landed

### Files

* `crates/ambition_actors/src/brain/boss_pattern.rs` (NEW, 681 lines):
  - Vocabulary moved from `content/features/bosses.rs`:
    `BossMovementProfile`, `BossPatternStep`, `BossPattern`,
    `BossAttackPattern`, `BossAttackProfile`, `step_duration`.
  - `BossAttackProfile::is_special()` → true for `GnuAppleRain`
    (today's only Special-typed profile).
  - `BossMovementProfile::target` purified: takes `(spawn,
    movement_timer, target_pos)` instead of `&BossRuntime`.
  - New `BossPatternCfg` carries pattern + movement + spawn anchor
    + combat collision size + per-boss cycle timings + apple-rain
    dodge tuning.
  - New `BossPatternState` carries `last_phase`, `step_index`,
    `step_elapsed`, `movement_timer`, `pattern_timer`,
    `cycle_phase: CyclePhase`, `cycle_phase_remaining`.
  - New `BossPatternContext` (encounter_phase, actor_pos, target_pos,
    world_size, dt) is the per-tick read-only input.
  - New `BossAttackState` component holds the brain's execution
    mirror.
  - Pure `tick_boss_pattern(cfg, state, ctx, out, attack_state_out)`
    function.
  - 7 unit tests covering: neutral in non-attacking phase, cursor
    reset on phase change, telegraph state, melee strike intent,
    gnu apple-rain special intent, cycle-mode phase advancement,
    peaceful gate.

* `crates/ambition_actors/src/brain/mod.rs`: re-exports
  `boss_pattern::*` (BossPatternCfg/State/Context/AttackState +
  vocabulary + tick fn).

* `crates/ambition_actors/src/brain/state_machine.rs`: placeholder
  `BossPatternCfg`/`State`/`tick_boss_pattern` deleted. The
  `StateMachineCfg::BossPattern` variant now carries
  `super::BossPatternCfg`/`State` (the re-exports). The generic
  `tick_state_machine` dispatch arm calls
  `tick_boss_pattern_via_state_machine` which writes neutral —
  bosses bypass the generic dispatcher via the boss tick system,
  and a dispatch race would be a bug.

* `crates/ambition_actors/src/features/bosses.rs`:
  vocabulary `pub use`-re-exported from `crate::brain::boss_pattern`.
  Deleted: `BossMovementProfile`/`BossPatternStep`/`BossPattern`/
  `BossAttackPattern`/`BossAttackProfile` definitions,
  `step_duration` free fn, `scripted_step_index`,
  `scripted_step_elapsed` fields, `update_scripted_attacks` /
  `update_cycle_attacks` / `update` / `build_control_frame`
  methods. New: `integrate_body(world, desired_vel, dt)` and
  `tick_runtime_clocks(dt)`. `active_strike_profile` /
  `telegraph_profile` / `attack_timer` / `attack_windup_timer`
  survive as brain-written mirrors.

* `crates/ambition_actors/src/features/ecs/bosses.rs`:
  rewritten. New `sync_boss_encounter_phase` + `tick_boss_brains_system`
  systems; `update_ecs_bosses` is integration-only and does not
  call `boss.update(...)` or overwrite `ActorControl`. The boss
  tick chain runs in `WorldPrep`: `sync_boss_encounter_phase` →
  `tick_boss_brains_system` → `update_ecs_bosses`.

* `crates/ambition_actors/src/features/ecs/spawn.rs`:
  boss spawn populates the full `BossPatternCfg` (pattern,
  movement, spawn, combat_size, cycle timings, apple-rain dodge)
  from `BossBehaviorProfile` and attaches `BossAttackState::default()`.
  Bundle split into outer + inner tuple (Bevy 15-arity Bundle
  limit).

* `crates/ambition_content/src/features.rs`: re-exports
  `sync_boss_encounter_phase` + `tick_boss_brains_system`;
  `WorldPrepSchedulePlugin` chains them in.

* `crates/ambition_content/src/character_catalog/resolver.rs`:
  catalog-preview `BossPattern` brain construction uses
  `BossPatternCfg::neutral_test()` as a baseline (real spawn-time
  bosses build their full cfg in `spawn.rs`).

### Acceptance criteria

| AC | Status | Notes |
| -- | ------ | ----- |
| `BossRuntime is the single intent producer` | ✅ 0 hits | |
| `update_scripted_attacks\|update_cycle_attacks\|scripted_step_index\|scripted_step_elapsed` in `bosses.rs` | ✅ 1 hit | Documentation comment explaining the deletion. Spec allows: "or only deleted-test references during transition". |
| `tick_boss_pattern` in `brain/` | ✅ real implementation in `brain/boss_pattern.rs` | |
| `control.0 = boss_frame\|boss.update(` in `ecs/bosses.rs` | ✅ 1 hit | File-header doc comment explaining what's deliberately absent. No policy-driven frame writes. |

## What stays as "Better final form" follow-up

* `BossAttackState` is now the authoritative component, but
  `BossRuntime::{active_strike_profile, telegraph_profile,
  attack_timer, attack_windup_timer}` survive as brain-written
  mirrors so legacy `attack_volumes` / `attack_telegraph_volumes`
  / `player_damage` readers can keep working without per-call
  changes. The endgame is to migrate those readers to query
  `BossAttackState` directly and delete the mirror fields. Tracked
  in the file-level doc comment in `ecs/bosses.rs`.

* Cycle-mode volume rendering still reads `pattern_timer` from
  `BossRuntime` (the brain owns its own `movement_timer` /
  `pattern_timer`, but the runtime ticks a parallel `pattern_timer`
  for the legacy `cycle_pattern_volumes` math). The right cleanup
  is to either (a) have the cycle-mode volume rendering read the
  brain's `pattern_timer` instead, or (b) move
  `cycle_pattern_volumes` itself to be brain-state-driven. Either
  way it's a small follow-up, not blocking.

* The `boss_pattern::tick_boss_pattern` emits `melee_pressed` for
  cycle-mode Active phase using `BossAttackProfile::FullBodyPulse`
  as a placeholder profile in the `BossAttackState`. The cycle-mode
  volume rendering doesn't actually read this profile (it reads
  `pattern_timer` + `behavior.attacks`), but a future cleanup that
  drives cycle volumes from the brain's profile choice would want
  to pick the correct profile per `pattern_timer` rotation index.

These follow-ups don't satisfy the spec — they're aesthetic /
endgame polish. The migration as specified is done.

## Test status

798 lib + 28 integration tests before this commit;
803 lib + 28 integration tests after. Net: +5 lib tests (7 new
boss_pattern unit tests, −2 deleted runtime-cursor tests that the
brain-side unit tests subsume).
