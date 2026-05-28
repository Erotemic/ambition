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

## What Phase 3d still leaves open (multi-day work)

The legacy `ae::Player` aggregate still exists in the engine. Sandbox
no longer references it; engine internals still operate on it for the
inner movement helpers that haven't been migrated to cluster refs.
Deleting it requires:

## What Phase 3d still leaves open

The legacy `ae::Player` aggregate still exists inside the engine.
Sandbox doesn't see it, but it remains the storage shape that the
inner movement code mutates each tick. Deleting it requires:

1. **Engine entry point refactor.** Replace
   `update_player_control_with_tuning(&mut Player, …)` and
   `update_player_simulation_with_tuning(&mut Player, …)` with
   versions whose internals operate directly on cluster refs (no
   tick-local `Player` scratchpad). This cascades through ~21
   inner helpers in `movement/{control,simulation,collision,integration,blink}.rs`
   and `ledge_grab.rs` (~2000 lines of engine code, ~300 `player.<field>`
   accesses).

2. **Engine test migration.** ~50 movement test cases in
   `crates/ambition_engine/src/movement/tests/` construct
   `Player::new(...)` + call the legacy entry points. They need to
   migrate to cluster-component fixtures. A `Player::into_clusters` /
   `Player::from_clusters` pair plus a small `run_player_simulation`
   test helper can absorb most of the boilerplate.

3. **Delete `ae::Player` + `_with_tuning` legacy entry points.** The
   `Player` struct, `Player::new`, `Player::reset_to`,
   `Player::refresh_movement_resources`, the `update_player_*_with_tuning`
   functions, and the bridge methods (`PlayerClustersMut::to_player`,
   `write_from_player`, `with_player_scratchpad`) all become dead and
   delete in the same commit.

4. **Update `PlayerSimulationBundle::new(player, health)`.** It
   currently takes `ae::Player` and decomposes it into cluster
   components. With `Player` gone, it should take an `AbilitySet` +
   `Vec2 spawn` + `Health` directly and build the cluster components
   inline.

5. **RL / trace fixtures.** A few sandbox call sites still construct a
   tick-local `ae::Player` via `clusters.to_player()` for read-only
   trace/RL observation:
   `crates/ambition_sandbox/src/bin/headless.rs`,
   `crates/ambition_sandbox/src/dev/trace/systems.rs`,
   `crates/ambition_sandbox/src/dev/debug_overlay.rs`,
   `crates/ambition_sandbox/src/rl_sim/runtime.rs` (already on cluster
   reads). Each needs to either read the clusters directly or use a
   sandbox-side snapshot type instead of `ae::Player`.

Phase 3d is **safely separable**. The branch as it stands is mergeable
— `rl_smoke 42/42`, engine `265/0` lib tests, `headless 120 ok`. The
engine-internals refactor is a follow-up that doesn't change the
sandbox surface; it only deletes legacy engine code.

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
