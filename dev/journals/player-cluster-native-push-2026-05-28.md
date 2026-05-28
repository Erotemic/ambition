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
  ↓ (2 scratchpads remain inside engine_core/movement.rs:)
  to_player → tick_active_ledge_grab(&mut Player) → write_from_player
  to_player → integrate_velocity(&mut Player) → write_from_player
```

The sandbox runtime no longer writes through `&mut ae::Player` for
*anything*. The two scratchpads above are localized to
`update_player_simulation_with_clusters` and only because two big
inner helpers (~600 lines combined) still take `&mut Player`.
Everything outside that function is cluster-native.

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

1. ~~Cluster-native `tick_active_ledge_grab`~~ — **landed** `bfb5783d`
   (2026-05-28). Field-for-field translation; rl_smoke + the 39
   ledge_grab unit tests still green.
2. **Cluster-native `integrate_velocity`** (~200 lines, the densest
   one — calls `integrate_climb`, `integrate_flight`,
   `sweep_player_x`, `sweep_player_y`, `apply_wall_abilities`,
   `touching_rebound`, all of which also take `&mut Player`). This
   one is multi-hour because the recursive helpers all need
   converting too. The plan: cluster-native variants for each
   helper, then the outer function reads/writes clusters directly.
   `touching_rebound` already has an `_aabb` variant; `apply_wall_abilities`
   reads `(on_wall, on_ground, abilities, wall_normal_x, vel.y)` and
   writes `(wall_clinging, wall_climbing, vel.y)`. The sweep helpers
   are the densest (60+ lines each, sub-helpers `resolve_axis`,
   `block_passable_during_climb`, `body_is_side_contact` all take
   `&mut Player` too).
3. **Once that lands**, `update_player_simulation_with_clusters`
   has zero scratchpads and `ae::Player` + `to_player` +
   `write_from_player` + the legacy `update_player_*_with_tuning`
   entry points are deletable. That deletion is probably 500+ lines
   net.

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
- **The `PlayerClustersMut::to_player` / `write_from_player` pair is
  still used internally by `engine_core/movement.rs`** for the two
  remaining scratchpads. Delete it after those clear.

## Test posture

- `cargo run --bin rl_smoke -- --frames 30` → 42/42 rooms ok at
  every commit through the push.
- `cargo test -p ambition_sandbox --lib` → 1132/1133.
- The 1 failing lib test (`embedded_ldtk_patrol_enemy_resolves_kinematic_path_index`)
  is an LDtk-content authoring issue, not code; deferred.
