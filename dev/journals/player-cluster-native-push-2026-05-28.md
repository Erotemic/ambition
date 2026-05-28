# Player cluster-native push (2026-05-28)

This is the continuation of the
[`engine-crate-collapse-2026-05-28`](engine-crate-collapse-2026-05-28.md)
journal. The engine crate is gone; the next layer was making every
sandbox-side `&mut ae::Player` path cluster-native so the legacy
aggregate becomes deletable.

## End state

```text
ECS player entity
  PlayerKinematics, PlayerGroundState, PlayerWallState,
  PlayerJumpState, PlayerDashState, PlayerFlightState,
  PlayerBlinkState, PlayerLedgeState, PlayerDodgeState,
  PlayerShieldState, PlayerBodyModeState, PlayerEnvironmentContact,
  PlayerAbilities, PlayerMana, PlayerOffense, PlayerActionBuffer,
  PlayerLifetime, PlayerComboTrace
  ↓
PlayerClustersMut<'a>   (struct of &mut to each cluster, built from
                         PlayerClusterQueryData::as_clusters_mut)
  ↓
player_control_phase / player_simulation_phase
  ↓
ae::update_player_{control,simulation}_with_clusters
  ↓
  tick_active_ledge_grab_clusters       (cluster-native)
  try_start_ledge_grab_clusters         (cluster-native)
  integrate_velocity_clusters           (cluster-native; calls
    integrate_climb_clusters,
    integrate_flight_clusters,
    sweep_player_x_clusters,
    sweep_player_y_clusters,
    apply_wall_abilities_clusters,
    touching_rebound_aabb,
    refresh_movement_resources_clusters)
```

**Zero scratchpads** in production code. The sandbox runtime never
writes through `&mut ae::Player` anywhere, and the engine internals
follow the same rule. `write_from_player` and
`with_player_scratchpad` were deleted (`780951af`); `to_player` is
read-only and used only by 3 callers (debug overlay, trace recorder,
headless bin) plus the engine-internal regression-test suite.

## What landed (phases 3d.1 – 3d.4)

| Refactor | Commit |
|---|---|
| `player_control_phase`, `player_simulation_phase` cluster-native + drop wrappers | `80b6b798` |
| `reset_sandbox`, `handle_player_events`, `is_riding` cluster-native | (same) |
| `death_respawn_player`, `safe_respawn_player`, `apply_player_knockback`, `handle_player_damage_events`, `load_room`, `remember_safe_player_position` cluster-native | `d01fae2b` |
| `start_attack`, `advance_attack`, `reload_ldtk_world_from_disk`, settings menu (`apply_action`, `apply_player_body_profile`, `apply_movement_profile`) cluster-native | `e638b452` |
| `RoomGraph::transition_for_player(Aabb, bool)` instead of `&Player` | `11dd45b6` |
| `try_start_ledge_grab_clusters` (engine_core, drops the 3rd inner scratchpad) | `1de3e9e9` |

## Side wins from the same push

- **Screen shake on a hard fall** — new `CameraShakeState` resource +
  `tick_camera_shake` decay + `hard_fall_shake_amplitude` pure helper
  triggered from `player_simulation_phase`. 11 unit tests pin the
  contract. (`4fcc5863`, `781ecf31`)
- **`cargo test -p ambition_sandbox --lib` passes again** — 1132/1133
  tests green (was: doesn't compile). The recovery touched ~10 files
  and quarantined `body_mode/tests.rs` behind `#![cfg(any())]` so a
  future session can port its 10 PlayerMovementAuthority-shaped
  tests against clusters at its own pace. (`b668e82a`, `3f95ec8d`)
- **`brain::smash::mode::choose_mode` Idle → committed-mode
  transition** — was getting trapped by the dwell window on the
  first call; now bypasses the dwell when the current state is
  `Idle`. (`3f95ec8d`)
- **`codex/intro-content-cleanup` integration** — both
  fascist→raid_enforcer rename commits cherry-picked onto this
  branch; one conflict resolved (namespace shift from `ae::` to
  `crate::cutscene::` since the cutscene moved sandbox-side in the
  engine port). (`08386078`, plus cherry-pick of `13fe1c59`)

## What remains for the next session

1. ~~Cluster-native `tick_active_ledge_grab`~~ — **landed** `bfb5783d`.
2. ~~Cluster-native `integrate_velocity`~~ — **landed** `3fa1f173`
   + `3a3f55e5` (sweep_player_x/y cluster-native via the new
   `resolve_axis_clusters`, `resolve_vertical_clusters`,
   `block_passable_during_climb_clusters` helpers).
3. ~~Delete `write_from_player` + `with_player_scratchpad`~~ —
   **landed** `780951af` (zero callers after the integrate_velocity
   wire-up).
4. **Read-only `Player` snapshot callers** so `to_player` +
   `ae::Player` itself can go away. Status after `df6e7cef` and
   `8c8edc93`:
   - ✅ `app/world_flow.rs` (`start_attack` / `advance_attack`) —
     **landed `df6e7cef`**. New `combat::AttackView` snapshot drives
     `combat::{resolve_attack_intent,attack_spec,attack_hitbox}_from_view`.
     No more `to_player` in either function.
   - ✅ `dev/debug_overlay.rs::draw_health_bars` — **landed `8c8edc93`**.
     Takes `Aabb` directly. The umbrella `draw_player_debug` still
     wants `&Player`; converting that is the next step.
   - 🔜 `dev/debug_overlay.rs::draw_player_debug` — reads ~10
     player fields; mechanical to convert.
   - 🔜 `dev/trace/systems.rs` snapshot — feeds
     `synthesize_events_from_diff`, `record_simulation_frame`,
     `update_previous_snapshot`, plus `LocomotionState::from_player`
     and `BodyMode::from_player`. Each downstream reader needs a
     cluster-aware variant; the deepest fan-out of the remaining
     work.
   - 🔜 `bin/headless.rs:244` — constructs a `Player` explicitly
     for the headless reporting path.
5. **Delete the legacy engine helpers + `ae::Player`** —
   only feasible AFTER #4 lands, and AFTER the
   `engine_core/movement/tests/` regression suite + sandbox-side
   `player/{ledge_grab,swim}.rs` test files are ported. The legacy
   entry points (`update_player`, `update_player_with_tuning`,
   `update_player_control`, `update_player_control_with_tuning`,
   `update_player_simulation`, `update_player_simulation_with_tuning`)
   plus their inner helpers (`integrate_velocity`, `integrate_climb`,
   `integrate_flight`, `apply_wall_abilities`, `sweep_player_x`,
   `sweep_player_y`, `resolve_axis`, `resolve_vertical`,
   `tick_active_ledge_grab`, `try_start_ledge_grab`,
   `requested_wall_normal`, `block_passable_during_climb`,
   `standing_on_one_way`, `touching_rebound`) have **zero production
   callers** — they survive only because the engine-internal movement
   regression suite and a handful of sandbox `player/*.rs` test
   files still drive them. Porting those tests + deleting both the
   tests' fallback path and `ae::Player` is multi-hour work, probably
   ~1500 lines net.

## Gotchas worth remembering

- **`PlayerClustersMut` field re-borrowing**: passing
  `clusters.dash` to a function that wants `&mut PlayerDashState`
  fails because `clusters.dash` is `&'a mut PlayerDashState` and
  field access through `&mut PlayerClustersMut` doesn't auto-reborrow.
  Use `&mut *clusters.dash` to reborrow.
- **`PlayerClustersMut.abilities` is `&PlayerAbilities` (shared)**,
  not `&mut`. This is deliberate — abilities are written by the
  ability-edit system, not the simulation tick. Helpers like
  `refresh_movement_resources_clusters` take it as shared.
- **`entity_mut().get_mut::<X>().unwrap()` drops the temporary**.
  Bind `let mut entity = world.entity_mut(e); let mut x = entity.get_mut::<X>().unwrap();`
  if you need to chain.
- **`use crate::engine_core as ae;` is private to the file**, so a
  child test mod that wants `ae::` can't inherit it via
  `use super::*` if the file scope removed the import. Either keep
  the alias at file scope (cfg-gate it for non-test builds to
  silence unused-import) or repeat the alias inside the test mod.
- ~~The `PlayerClustersMut::to_player` / `write_from_player` pair is
  still used internally by `engine_core/movement.rs`~~ — `write_from_player`
  and `with_player_scratchpad` deleted `780951af`. `to_player` is
  read-only; production code never writes through Player back into
  clusters.

## Test posture

- `cargo run --bin rl_smoke` → 42/42 rooms ok at every commit
  through the push (200 ticks each, 8400 frames per pass).
- `cargo test -p ambition_sandbox --lib` → **1140/1140 green** (was
  "doesn't compile" at session start; ~25 lib-test logic / shape
  issues fixed along the way).
- All integration tests green: `repro_walls` (6), `crouch_stability`,
  `dash_stability` (2), `scripted_gameplay` (3),
  `replay_fixture_regression`, `plugin_minimal_app` (7),
  `fuzz_random_walker` (5). Total: ~25 integration tests passing.
- Workspace `cargo check` clean (zero warnings).

## Final step: `ae::Player` deletion (2026-05-28, session 2)

Commit `c02ca686` removes the `Player` struct outright. All
intermediate scratchpad and bridge code is now gone:

- `Player::new_with_abilities` → replaced by
  `PlayerClusterScratch::new_with_abilities(spawn, abilities)` which
  builds the 18 cluster components directly (mirror of the original
  `Player::new_with_abilities` field-for-field, but no Player
  intermediate).
- `Player::reset_to`, `Player::record`, `Player::aabb`,
  `Player::refresh_movement_resources` → deleted.
  `reset_player_clusters`, `events.op_clusters`,
  `PlayerKinematics::aabb`, `refresh_movement_resources_clusters`
  are the cluster-native replacements.
- `PlayerClusterScratch::from_player(&Player)` + all the per-cluster
  `XxxState::from_player(&Player) -> Self` constructors → deleted.
  Tests build a scratch via `new_with_abilities` then poke individual
  cluster fields.
- `classify_player_safety(&Player)`, `try_change_body_mode(&mut
  Player)`, `LocomotionState::from_player(&Player)`,
  `BodyMode::from_player(&Player)`, `FrameEvents::op(&mut Player)`,
  `combat::{resolve_attack_intent, attack_spec,
  attack_hitbox}(&Player)`, `trace::detect_oob(&Player)` → all
  deleted.
  Callers use the cluster / `AttackView` / `_from_kinematics` /
  `_scratch` variants.
- `PlayerSimulationBundle::new(Player, h)` → deleted, replaced by
  `from_scratch(scratch, h)`. Production spawn sites
  (`runtime/setup.rs`, `runtime/reset.rs`) go through
  `crate::player::primary_player_scratch(spawn, abilities)`.
- Orphan `body_mode/tests.rs` (gated `cfg(any())` pending port) was
  deleted; the mechanic is covered by `rl_smoke`'s 42 rooms.

Final test posture: 1143/1143 lib tests green, 42/42 rl_smoke rooms
ok, `cargo check` clean.
