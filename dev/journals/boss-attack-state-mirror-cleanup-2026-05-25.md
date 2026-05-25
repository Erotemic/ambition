# Boss attack state mirror cleanup — `BossAttackState` is the single source of truth (2026-05-25)

Completion journal for the **final** step of the "move boss policy
out of `BossRuntime`" migration. The user spec'd 7 cleanup steps:

1. Move boss attack volume calculation out of `BossRuntime`
2. Replace `BossRuntime::player_damage`
3. Move debug overlay + telegraph rendering to `BossAttackState`
4. Move boss vulnerability logic off mirrored profiles
5. Eliminate `sync_runtime_mirror_from_attack_state`
6. Move `pattern_timer` fully into `BossPatternState`
7. Reduce `BossRuntime` method surface

All seven landed in commit `583bc7b`.

## Estimated vs actual time

| Plan estimate (implicit, ~4-6h for a 7-step cross-module migration) | Actual wall-clock | Ratio (plan / actual) |
| ------------------------------------------------------------------- | ----------------- | --------------------- |
| ~4–6 h                                                              | **~24 min**       | **10–15× faster**     |

Fourth datapoint on the trend log:

| Task | Plan | Actual | Ratio |
| --- | --- | --- | --- |
| Enemy melee hitbox lifecycle | 2–3 h | ~22 min | 5.5–8.2× |
| gnu_ton apple_rain consumer | 4–6 h | ~20 min | 12–18× |
| Boss policy out of BossRuntime | 4–6 h | ~26 min | 9–14× |
| BossAttackState mirror cleanup | 4–6 h | ~24 min | 10–15× |
| **Trend** | | | **~10×** |

The trend continues at ~10×.

### Why this stayed fast

* **Survey-first paid off.** Inventoried every caller of the
  deleted methods (`grep \.attack_volumes\|\.player_damage\|\.damageable_aabbs`)
  before touching anything. Found 3 source-of-truth callers
  (debug_overlay, ecs/damage.rs's apply + predicate,
  ecs/bosses.rs's update + brain tick) + 4 presentation readers
  (view_index, presentation/actors.rs ×2, anim_helpers). Knew
  exactly what would break before starting.
* **One new pure module did the heavy lifting.**
  `content/features/boss_attack_geometry.rs` (366 lines) owns every
  volume / damage computation as pure free functions over
  `BossVolumeContext = (pos, size, combat_size, is_gnu_ton, &BossBehaviorProfile, &BossAttackState)`.
  Every caller becomes `volumes_for_profile(profile, ...)` or
  `damageable_volumes(&ctx)` — uniform shape, no methods, no
  mirrors.
* **The `cycle_attacks: Vec<BossAttackProfile>` plumbing change
  unblocked Cycle-mode uniformity.** Pre-cleanup the cycle brain
  wrote `FullBodyPulse` as a placeholder into `BossAttackState`
  because the brain didn't have the `behavior.attacks` list. One
  field on `BossPatternCfg` + `cycle_attacks: boss.behavior.attacks.clone()`
  at spawn → cycle and scripted bosses now both populate
  `BossAttackState.active_profile` with the real per-rotation
  profile, so volume math has one path instead of two.

## What landed

### Files (14 total)

* **NEW** `content/features/boss_attack_geometry.rs` — pure volume +
  damage helpers (366 lines).

* `brain/boss_pattern.rs` — added `cycle_attacks: Vec<BossAttackProfile>`
  to `BossPatternCfg`. `advance_cycle` picks `attacks[idx]` based on
  `pattern_timer / cycle_attack_cooldown` and writes it into
  `BossAttackState.active_profile` / `telegraph_profile`.

* `brain/mod.rs` — added `Brain::boss_pattern_state() -> Option<&BossPatternState>`
  accessor so presentation / debug code can read `pattern_timer`
  without match-deconstructing the variant by hand.

* `content/features/bosses.rs` — deleted fields:
  `pattern_timer`, `movement_timer`, `attack_windup_timer`,
  `attack_timer`, `attack_cooldown`, `active_strike_profile`,
  `telegraph_profile`. Deleted methods: `attack_volumes`,
  `attack_telegraph_volumes`, `cycle_pattern_volumes`,
  `volumes_for`, `damageable_aabbs`, `player_damage`,
  `body_damage_aabb`, `gnu_ton_part_aabb`, `tick_runtime_clocks`.
  Deleted constants: `GNU_TON_COLLISION_SCALE`,
  `GNU_TON_FRAME_HEIGHT`, helper `gnu_ton_sprite_scale` (all moved
  to `boss_attack_geometry.rs`). Surviving surface: `new`,
  `integrate_body`, `aabb`, `combat_size`, `render_size`,
  `bark_anchor`, `is_gnu_ton`, `is_mockingbird`,
  `apply_behavior_profile`.

* `content/features.rs` — added module + re-exports
  (`active_attack_volumes`, `body_damage_aabb`, `boss_attack_damage`,
  `damageable_volumes`, `telegraph_volumes`, `volumes_for_profile`,
  `BossVolumeContext`).

* `content/features/ecs/bosses.rs` — `sync_runtime_mirror_from_attack_state`
  deleted; `update_ecs_bosses` queries `&BossAttackState` + `&Brain`,
  builds `BossVolumeContext::from_runtime`, calls `boss_attack_damage`.
  `BossPatternTimer` mirror populated from
  `brain.boss_pattern_state().pattern_timer` instead of
  `boss.pattern_timer`.

* `content/features/ecs/spawn.rs` — populates
  `BossPatternCfg.cycle_attacks` from `behavior.attacks.clone()`;
  initial `BossPatternTimer(0.0)`.

* `content/features/ecs/damage.rs` — `apply_feature_damage_events`
  and `ecs_damage_event_hits_boss` queries extended with
  `&BossAttackState`; call `damageable_volumes(&ctx)` instead of
  `boss.damageable_aabbs()`.

* `content/features/ecs/reset.rs` — `reset_ecs_room_features`
  queries `&mut Brain` + `&mut BossAttackState` + `&mut ActorControl`;
  resets `BossPatternState` + clears `BossAttackState` + zeros
  `ActorControl` instead of zeroing the deleted runtime mirror
  fields.

* `content/features/ecs/anim_helpers.rs` — `ecs_boss_anim_state` +
  `ecs_boss_name` query `&BossAttackState` (+ `&Brain` for the
  former). Reads `attack_active = attack_state.active_profile.is_some()`,
  `attack_windup = attack_state.telegraph_profile.is_some()`,
  `pattern_timer = brain.boss_pattern_state().map(|s| s.pattern_timer).unwrap_or(0.0)`.

* `content/features/ecs/view_index.rs` — `bosses` query extended;
  `flash` flag reads `BossAttackState` profiles + `boss.hit_flash`.

* `dev/debug_overlay.rs` — `FeatureDebugQueries.bosses` extended;
  draws via the new helpers.

* `presentation/rendering/actors.rs` — `upgrade_boss_sprites` +
  `animate_bosses` queries extended with `&BossAttackState` (+
  `&Brain` for `animate_bosses`). Boss `flash` flag and
  `BossAnimState` populated from the new source.

* `projectile/systems.rs` — `ecs_bosses` query for the
  damage-hit predicate extended.

### Acceptance criteria

| Spec grep | Result |
| --- | --- |
| `BossRuntime is the single intent producer` in src/ | 0 hits |
| `active_strike_profile\|telegraph_profile\|attack_timer\|attack_windup_timer` in `bosses.rs` / `ecs/bosses.rs` | only documentation comments explaining the deletion |
| Method surface check: `attack_volumes / attack_telegraph_volumes / cycle_pattern_volumes / volumes_for / player_damage / damageable_aabbs / body_damage_aabb / tick_runtime_clocks` in `bosses.rs` | all deleted |
| `boss.pattern_timer\|boss.movement_timer\|boss.attack_cooldown` source-wide | 0 hits |

## Final BossRuntime surface

```rust
pub struct BossRuntime {
    pub id: String,
    pub name: String,
    pub pos: Vec2,
    pub spawn: Vec2,
    pub size: Vec2,
    pub health: Health,
    pub brain: BossBrain,
    pub behavior: BossBehaviorProfile,
    pub alive: bool,
    pub hit_flash: f32,
    pub encounter_phase: BossEncounterPhase,
}

impl BossRuntime {
    pub(crate) fn new(id, name, aabb, brain) -> Self;
    pub fn integrate_body(&mut self, world, desired_vel, dt);
    pub fn aabb(&self) -> Aabb;
    pub fn combat_size(&self) -> Vec2;
    pub fn render_size(&self) -> Vec2;
    pub fn bark_anchor(&self) -> Vec2;
    pub fn is_gnu_ton(&self) -> bool;
    pub fn is_mockingbird(&self) -> bool;
    pub fn apply_behavior_profile(&mut self, behavior: BossBehaviorProfile);
}
```

Body + HP + integration only. Zero attack state. Zero policy.

## Test status

803 lib + 28 integration tests pass (unchanged from before this
commit — no new tests were added; the existing tests in
`bosses.rs::scripted_pattern_tests` were ported to the new helpers
and the brain-side `boss_pattern::tests` already cover the policy
invariants from the previous migration).
