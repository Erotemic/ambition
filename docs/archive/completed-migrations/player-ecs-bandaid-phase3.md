# Player ECS bandaid — Phase 3 status

Companion to [`player-ecs-bandaid-plan.md`](player-ecs-bandaid-plan.md)
and [`player-ecs-bandaid-phase0.md`](player-ecs-bandaid-phase0.md).

Date: 2026-05-28. Branch: `player-ecs-bandaid`.

## What's done

The plan's stated merge posture is met:

> This removes `ae::Player` / `PlayerMovementAuthority` as the live
> player runtime authority. Player state is now owned by ECS
> components on the player entity. The existing movement/combat
> feature set is preserved as closely as possible.

Commits delivering this:

| Stage | Commit | Notes |
| --- | --- | --- |
| Phase 0 | `b84c2619` | Baseline + `ae::Player` field ledger |
| Phase 1 | `c045d19a` | Cluster components in sandbox |
| Phase 2 scaffold | `2871f9b4` | Tick-local `ae::Player` bridge |
| Phase 2a | `f8dca5a3` | Live writers migrated to bridge |
| Phase 2b/c | `0727db24` | Readers migrated; `PlayerMovementAuthority` + `PlayerBody` deleted |
| Phase 3a | `4636b154` | Cluster components moved into engine (engine is Bevy-native per ADR 0002) |
| Phase 3b.1 | `a0b84abc` | `try_change_body_mode_clusters` + body_mode bridge-free |
| Phase 3b.2 | `ebd235ce` | `blink_destination_clusters` + fx blink-preview bridge-free |
| Phase 3b.3 | `9fe1ce8c` | `sync_live_ability_edits_clusters` + dev-edits bridge-free |
| Phase 3b/c | `42782254` | Sandbox `engine_player_bridge` module **deleted** |

## Current architectural state

```text
[ Sandbox systems  (Bevy ECS) ]
       |  cluster component refs
       v
[ engine cluster API  (ae::PlayerClustersMut, _with_clusters helpers) ]
       |  tick-local round-trip via to_player / write_from_player
       v
[ legacy engine helpers  (ae::Player, _with_tuning, …) ]
```

- **Sandbox layer**: idiomatic Bevy. Cluster components are the
  source of truth. No `engine_player_bridge` module. Systems take
  cluster refs in their `Query` signatures.
- **Engine cluster API**: `ae::PlayerClustersMut`,
  `ae::PlayerClusterQueryData`, `update_player_*_with_clusters`,
  `blink_destination_clusters`, `try_change_body_mode_clusters`,
  `sync_live_ability_edits_clusters`. These accept cluster refs and
  internally bridge to the legacy `&mut Player` helpers via
  `PlayerClustersMut::to_player` / `write_from_player`.
- **Legacy engine helpers**: `update_player_control_with_tuning`,
  `update_player_simulation_with_tuning`, and ~21 inner functions
  still take `&[mut] Player`. They are no longer called directly
  from sandbox code — only from engine tests and from the cluster
  wrapper entry points.

## Phase 3d progress

Cluster-native engine entry points + several inner helpers refactored
during the multi-session push:

- `ae::PlayerClustersMut` aggregate (engine-side) + `to_player` /
  `write_from_player` / `with_player_scratchpad` round-trip helpers
- `ae::PlayerClusterQueryData` Bevy QueryData (engine-side)
- `update_player_simulation_with_clusters` — operates on cluster refs
  natively for setup/age/timers/jump_buffer/hazard; uses a localized
  scratchpad only for `tick_active_ledge_grab`, `integrate_velocity`,
  `try_start_ledge_grab` (inner helpers not yet refactored)
- `update_player_control_with_clusters` — operates on cluster refs
  natively for reset/facing/buffers/fly-toggle/dodge/dash/shield/jump-release;
  uses a localized scratchpad only for `handle_blink` + `handle_attacks`
- `ae::reset_player_clusters(&mut clusters, spawn)` — replaces
  `Player::reset_to`
- `ae::refresh_movement_resources_clusters(...)` — replaces
  `Player::refresh_movement_resources`
- `ae::try_change_body_mode_clusters(...)` — replaces
  `try_change_body_mode(&mut Player, ...)` for sandbox callers
- `ae::blink_destination_clusters` / `blink_destination_to_point_clusters`
- `ae::dev_tools::sync_live_ability_edits_clusters`
- `ae::movement::handle_jump_buffer_clusters` — cluster-ref jump-buffer
  handler with full feature parity
- `ae::touching_hazard_aabb(world, aabb) -> bool`
- `ae::movement::collision::standing_on_one_way_aabb(world, aabb) -> bool`
- `FrameEvents::op_clusters(combo_trace, op)` — cluster-side combo-trace
  push without going through `Player::record`

## Phase 3d — COMPLETE (2026-05-28, commit `c02ca686`)

The follow-ups listed below have all landed:

1. **Engine entry point refactor.** ✓
   `update_player_with_tuning_clusters`,
   `update_player_control_with_clusters`,
   `update_player_simulation_with_clusters` (plus their `_scratch`
   test wrappers) are the only entry points; the legacy
   `_with_tuning` Player-shaped chain is deleted along with every
   inner Player-shaped helper.

2. **Engine test migration.** ✓
   All movement tests build a `PlayerClusterScratch` via
   `PlayerClusterScratch::new_with_abilities(spawn, abilities)` and
   call the `_scratch` entry points. The previous `Player::new`
   call sites are gone.

3. **Delete `ae::Player` + `_with_tuning` legacy entry points.** ✓
   `Player` struct, `Player::new`, `Player::new_with_abilities`,
   `Player::reset_to`, `Player::record`,
   `Player::refresh_movement_resources`, `Player::combo_symbols`,
   the `update_player_*_with_tuning` chain, and the bridge methods
   `PlayerClustersMut::to_player`, `write_from_player`,
   `with_player_scratchpad` — all deleted.

4. **Update `PlayerSimulationBundle::new(Player, health)`.** ✓
   Replaced by `PlayerSimulationBundle::from_scratch(scratch,
   health)`. Production spawn sites (`runtime/setup.rs`,
   `runtime/reset.rs`) go through
   `crate::player::primary_player_scratch(spawn, abilities)`.

5. **RL / trace fixtures.** ✓ `bin/headless.rs` queries
   `PlayerClusterQueryData::as_clusters_mut()` directly;
   `dev/trace/{systems,detect,tests}.rs` reads cluster components
   natively; `dev/debug_overlay.rs::draw_player_debug` consumes
   `&PlayerClustersMut`. No `ae::Player` materializations remain
   anywhere.

Final test posture: 1143 / 1143 lib tests, 28 / 28 integration
tests, 42 / 42 rl_smoke rooms green. See
`dev/journals/player-cluster-native-push-2026-05-28.md` for the
commit-by-commit map.

## Regression gate status

Verified at every Phase 3 commit:

- `cargo check -p ambition_engine`: clean
- `cargo check -p ambition_sandbox`: clean
- `cargo test -p ambition_engine --lib`: **265 passed / 0 failed**
- `cargo run -p ambition_sandbox --bin headless -- 120`: ok
- `cargo run -p ambition_sandbox --bin rl_smoke -- 200 1`: **42 / 42 rooms ok**

The pre-existing sandbox lib-test compile failures documented in
Phase 0 remain pre-existing. A handful of additional test fixtures
that referenced `PlayerMovementAuthority` / `PlayerBody` directly need
spawning rewrites; they grew alongside the pre-existing failures and
remain out-of-gate.
