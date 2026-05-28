# Player ECS bandaid — Phase 0 baseline & field ledger

> **Status (2026-05-28):** **HISTORICAL.** The migration this baseline
> drives toward landed in commit `c02ca686`. `ae::Player` is deleted;
> the 18 cluster components on the player entity are the only player
> state. This document is preserved as the as-it-stood field-by-field
> baseline. See
> [`player-ecs-bandaid-plan.md`](player-ecs-bandaid-plan.md) and
> [`player-ecs-bandaid-phase3.md`](player-ecs-bandaid-phase3.md)
> (both marked COMPLETE) plus the migration journals in
> `dev/journals/`.

Phase 0 artifact for [`player-ecs-bandaid-plan.md`](player-ecs-bandaid-plan.md).
Branch: `player-ecs-bandaid` (cut from `main` at `04d4294f Add player ECS
bandaid plan`).

Date: 2026-05-28.

This document records:

1. The baseline gate (what compiled / passed before any code change so the
   refactor branch knows which failures are pre-existing).
2. The smoke route the branch should use to detect regressions.
3. The field-by-field destination ledger for every `ae::Player` field, so
   no movement/combat state silently disappears when the aggregate is
   deleted.

Sources read for the ledger:

- [`crates/ambition_sandbox/src/engine_core/movement/player.rs`](../../crates/ambition_sandbox/src/engine_core/movement/player.rs) (the `ae::Player` struct definition).
- [`crates/ambition_sandbox/src/engine_core/player_state.rs`](../../crates/ambition_sandbox/src/engine_core/player_state.rs) (`BodyMode`, `ResourceMeter`, `LocomotionState`, `try_change_body_mode`, `classify_player_safety`).
- [`crates/ambition_sandbox/src/player/components.rs`](../../crates/ambition_sandbox/src/player/components.rs) (existing ECS shape).
- [`crates/ambition_sandbox/src/player/bundles.rs`](../../crates/ambition_sandbox/src/player/bundles.rs) (current spawn shape).

## Baseline gate (commit `04d4294f`)

Run on `player-ecs-bandaid` immediately after branching:

| Command | Result |
| --- | --- |
| `cargo check -p ambition_engine` | clean (`Finished … in 1.10s`) |
| `cargo test -p ambition_engine --lib` | **265 passed; 0 failed** |
| `cargo check -p ambition_sandbox` | clean (`Finished … in 17.75s`) |
| `cargo test -p ambition_sandbox --lib` | **pre-existing compile fail (9 errors)** — see below |
| `cargo run -p ambition_sandbox --bin headless -- 120` | 120 ticks ok, quest log populated |
| `cargo run -p ambition_sandbox --bin rl_smoke -- 200 1` | **42 / 42 rooms ok** |

### Pre-existing sandbox lib-test compile failures

`cargo test -p ambition_sandbox --lib` does **not** compile on `main`. These
breaks predate the branch and MUST NOT be used as a regression gate:

- `crates/ambition_sandbox/src/brain/smash/action.rs:214` — missing field
  `self_air_jumps_remaining` in `smash::observation::ObservationFrame`.
- `crates/ambition_sandbox/src/brain/smash/emit.rs:108` — same field.
- `crates/ambition_sandbox/src/brain/smash/mode.rs:116` — same field.
- `crates/ambition_sandbox/src/content/features/enemies.rs:849` —
  `enemy.update(...)` takes 7 arguments, 6 supplied (the
  `Option<ActorControlFrame>` override slot is missing in five test call
  sites at `conversion_tests.rs:411 / 457 / 600 / 663 / 710`).
- `crates/ambition_sandbox/src/content/features/boss_attack_geometry.rs:920`
  — lifetime escape in a `BossSpriteMetrics`-borrowing closure.

These need their own fix branch. They are tracked here only so that, at
merge time, the player ECS branch can prove it didn't *introduce* any new
sandbox-lib-test errors.

### Refactor regression gate

Use this on the player ECS branch:

```bash
CARGO_TARGET_DIR=/home/agent/ambition-target \
  cargo check -p ambition_engine \
    && cargo test -p ambition_engine --lib \
    && cargo check -p ambition_sandbox \
    && cargo run -p ambition_sandbox --bin headless -- 120 \
    && cargo run -p ambition_sandbox --bin rl_smoke -- 200 1
```

The `CARGO_TARGET_DIR` override is required for the agent user — the
checked-in `.cargo/config.toml` points at `/home/joncrall/ambition-target/`
which is not writable from the agent VM (root-owned parent). Jon's host
shell does not need the override.

## Smoke route

Two layers:

1. **Automated**: `rl_smoke 200 1` runs a random-walker policy through every
   one of the 42 sandbox rooms for 200 simulation ticks and asserts that
   HP stays in `[0, hp_max]` and position stays finite + bounded. This is
   the cheapest "did movement break in any room" canary.

2. **Manual** (windowed binary, run by Jon on host): walk through the plan's
   checklist in [`player-ecs-bandaid-plan.md`](player-ecs-bandaid-plan.md)
   §"Manual smoke checklist" — spawn, walk, jump (short/full/coyote/double),
   dash, wall cling/jump, ledge grab, attack (forward/up/down), pogo,
   projectile charge/release, blink (quick/precision), shield/parry, dodge
   roll, take damage / hitstun / invuln / reset, interact (door/NPC/chest/
   switch), water swim, morph/crouch, room transition.

## Live consumer inventory

`PlayerMovementAuthority` is referenced by **29** files and `ae::Player`
is named directly in **30** files inside `crates/ambition_sandbox/src/`.
The categorical map of where the refactor will bleed:

| Category | Files |
| --- | --- |
| Player tick / authority | [`app/player_tick.rs`](../../crates/ambition_sandbox/src/app/player_tick.rs), [`app/sim_systems.rs`](../../crates/ambition_sandbox/src/app/sim_systems.rs), [`app/world_flow.rs`](../../crates/ambition_sandbox/src/app/world_flow.rs), [`app/hud.rs`](../../crates/ambition_sandbox/src/app/hud.rs), [`app/dev_runtime.rs`](../../crates/ambition_sandbox/src/app/dev_runtime.rs) |
| Player module | [`player.rs`](../../crates/ambition_sandbox/src/player.rs), [`player/components.rs`](../../crates/ambition_sandbox/src/player/components.rs), [`player/bundles.rs`](../../crates/ambition_sandbox/src/player/bundles.rs), [`player/systems.rs`](../../crates/ambition_sandbox/src/player/systems.rs), [`player/affordances/mod.rs`](../../crates/ambition_sandbox/src/player/affordances/mod.rs), [`player/affordances/intent.rs`](../../crates/ambition_sandbox/src/player/affordances/intent.rs) |
| Body mode | [`body_mode/mechanics.rs`](../../crates/ambition_sandbox/src/body_mode/mechanics.rs), [`body_mode/morph_ball.rs`](../../crates/ambition_sandbox/src/body_mode/morph_ball.rs) |
| Brain seam | [`brain/mod.rs`](../../crates/ambition_sandbox/src/brain/mod.rs) |
| Runtime / lifecycle | [`runtime/setup.rs`](../../crates/ambition_sandbox/src/runtime/setup.rs), [`runtime/reset.rs`](../../crates/ambition_sandbox/src/runtime/reset.rs), [`rl_sim/runtime.rs`](../../crates/ambition_sandbox/src/rl_sim/runtime.rs) |
| Presentation / FX | [`presentation/fx.rs`](../../crates/ambition_sandbox/src/presentation/fx.rs), [`presentation/rendering/actors.rs`](../../crates/ambition_sandbox/src/presentation/rendering/actors.rs), [`audio/environment.rs`](../../crates/ambition_sandbox/src/audio/environment.rs) |
| Dev / trace / debug | [`dev/dev_tools.rs`](../../crates/ambition_sandbox/src/dev/dev_tools.rs), [`dev/debug_overlay.rs`](../../crates/ambition_sandbox/src/dev/debug_overlay.rs), [`dev/trace/systems.rs`](../../crates/ambition_sandbox/src/dev/trace/systems.rs), [`bin/headless.rs`](../../crates/ambition_sandbox/src/bin/headless.rs) |
| Time / world / pause | [`time/time_control.rs`](../../crates/ambition_sandbox/src/time/time_control.rs), [`falling_sand.rs`](../../crates/ambition_sandbox/src/falling_sand.rs), [`pause_menu/model.rs`](../../crates/ambition_sandbox/src/pause_menu/model.rs), [`lib.rs`](../../crates/ambition_sandbox/src/lib.rs) |

Phase 2 of the plan removes `PlayerMovementAuthority` from the bundle and
relies on the compiler to surface every reader in this list.

## `ae::Player` field ledger

Every field on [`ae::Player`](../../crates/ambition_sandbox/src/engine_core/movement/player.rs#L26)
gets a destination. "Cluster" names follow the target vocabulary in the
plan. "Helper-local" means the value lives only as a local variable inside
a movement helper after the refactor — it should not survive as a
component.

### Identity / abilities

| Field | Type | Destination | Notes |
| --- | --- | --- | --- |
| `abilities` | `AbilitySet` | **stays** as its own `Component` on the player entity (`PlayerAbilities(AbilitySet)`) | Already used to derive `ActionSet`; lift it off the aggregate so AbilitySet can drive ECS gating without going through the authority. |

### Kinematics cluster → `PlayerKinematics`

| Field | Type | Destination | Notes |
| --- | --- | --- | --- |
| `pos` | `Vec2` | `PlayerKinematics::pos` | Center of AABB. Stays the authoritative simulation position; presentation `Transform.translation` mirrors it. |
| `vel` | `Vec2` | `PlayerKinematics::vel` | |
| `size` | `Vec2` | `PlayerKinematics::size` | Currently morph/crouch shrinks this. |
| `base_size` | `Vec2` | `PlayerKinematics::base_size` | Stand-up target shape; read by `BodyMode::shape`. |
| `facing` | `f32` | `PlayerKinematics::facing` | `±1.0`. |

### Ground cluster → `PlayerGroundState`

| Field | Type | Destination | Notes |
| --- | --- | --- | --- |
| `on_ground` | `bool` | `PlayerGroundState::on_ground` | |
| `coyote_timer` | `f32` | `PlayerGroundState::coyote_timer` | Counts down after losing ground contact. |
| `drop_through_timer` | `f32` | `PlayerGroundState::drop_through_timer` | Brief window where vertical sweep ignores one-way platforms. |
| `rebound_cooldown` | `f32` | `PlayerGroundState::rebound_cooldown` | Pogo/bounce re-entry guard. Keep here; the pogo verb itself reads it. |

### Wall cluster → `PlayerWallState`

| Field | Type | Destination | Notes |
| --- | --- | --- | --- |
| `on_wall` | `bool` | `PlayerWallState::on_wall` | |
| `wall_normal_x` | `f32` | `PlayerWallState::wall_normal_x` | |
| `wall_clinging` | `bool` | `PlayerWallState::wall_clinging` | |
| `wall_climbing` | `bool` | `PlayerWallState::wall_climbing` | |
| `pre_wall_vel` | `Vec2` | `PlayerWallState::pre_wall_vel` | Pre-collision airborne velocity, read by the ledge-grab momentum boost. |
| `pre_wall_vel_age` | `f32` | `PlayerWallState::pre_wall_vel_age` | Staleness counter for `pre_wall_vel`. |

### Jump cluster → `PlayerJumpState`

| Field | Type | Destination | Notes |
| --- | --- | --- | --- |
| `air_jumps_available` | `u8` | `PlayerJumpState::air_jumps_available` | |
| `jump_buffer_timer` | `f32` | `PlayerJumpState::jump_buffer_timer` | Plan §"Action buffers" wants this folded into a generic `PlayerActionBuffer.jump`; keep on `PlayerJumpState` for the first cut and migrate when the rest of the action buffer lands. |

### Dash cluster → `PlayerDashState`

| Field | Type | Destination | Notes |
| --- | --- | --- | --- |
| `dash_charges_available` | `u8` | `PlayerDashState::charges_available` | |
| `dash_timer` | `f32` | `PlayerDashState::timer` | `> 0` while a dash is mid-execution. |
| `dash_cooldown` | `f32` | `PlayerDashState::cooldown` | |
| `dash_buffer_timer` | `f32` | `PlayerDashState::buffer_timer` | Same migration plan as `jump_buffer_timer` → `PlayerActionBuffer.dash`. |

### Flight / glide / fast-fall cluster → `PlayerFlightState`

| Field | Type | Destination | Notes |
| --- | --- | --- | --- |
| `fly_enabled` | `bool` | `PlayerFlightState::fly_enabled` | Toggleable free-flight mode. |
| `flight_phase` | `f32` | `PlayerFlightState::flight_phase` | Idle hover bob accumulator. |
| `gliding` | `bool` | `PlayerFlightState::gliding` | Held-jump glide flag. |
| `fast_falling` | `bool` | `PlayerFlightState::fast_falling` | Set after double-tap-down. |

### Blink cluster → `PlayerBlinkState`

| Field | Type | Destination | Notes |
| --- | --- | --- | --- |
| `blink_cooldown` | `f32` | `PlayerBlinkState::cooldown` | |
| `blink_hold_active` | `bool` | `PlayerBlinkState::hold_active` | |
| `blink_hold_timer` | `f32` | `PlayerBlinkState::hold_timer` | |
| `blink_aiming` | `bool` | `PlayerBlinkState::aiming` | |
| `blink_aim_offset` | `Vec2` | `PlayerBlinkState::aim_offset` | Precision-blink cursor relative to player position. |
| `blink_grace_timer` | `f32` | `PlayerBlinkState::grace_timer` | Post-blink fall suppression. |

Camera-side presentation state (`PlayerBlinkCameraState`) already exists and
keeps its current shape; only the simulation half moves.

### Ledge cluster → `PlayerLedgeState`

| Field | Type | Destination | Notes |
| --- | --- | --- | --- |
| `ledge_grab` | `Option<LedgeGrabState>` | `PlayerLedgeState::grab` | Whole `LedgeGrabState` stays a pure-engine value type; only the optional-on-player wrapping moves. |
| `ledge_release_cooldown` | `f32` | `PlayerLedgeState::release_cooldown` | Smash-style re-grab guard. |

### Dodge / shield / parry → `PlayerDodgeState` + `PlayerShieldState`

| Field | Type | Destination | Notes |
| --- | --- | --- | --- |
| `dodge_roll_timer` | `f32` | `PlayerDodgeState::roll_timer` | i-frame countdown. |
| `dodge_roll_cooldown` | `f32` | `PlayerDodgeState::cooldown` | |
| `shield_active` | `bool` | `PlayerShieldState::active` | |
| `parry_window_timer` | `f32` | `PlayerShieldState::parry_window_timer` | |

`PlayerBody::shielding` / `parrying` (derived booleans) become read-only
helpers on `PlayerShieldState`, or just inlined `state.active && state.parry_window_timer > 0.0`.

### Body mode → `PlayerBodyModeState`

| Field | Type | Destination | Notes |
| --- | --- | --- | --- |
| `body_mode` | `BodyMode` | `PlayerBodyModeState::body_mode` | `BodyMode` itself stays as an engine value type. `try_change_body_mode` becomes a helper that takes `&mut PlayerKinematics, &mut PlayerBodyModeState, &World, predicate`. |

### Environment contact → `PlayerEnvironmentContact`

| Field | Type | Destination | Notes |
| --- | --- | --- | --- |
| `water_contact` | `Option<WaterContact>` | `PlayerEnvironmentContact::water` | |
| `climbable_contact` | `Option<ClimbableContact>` | `PlayerEnvironmentContact::climbable` | |

### Resources / offense

| Field | Type | Destination | Notes |
| --- | --- | --- | --- |
| `mana` | `ResourceMeter` | `PlayerMana(ResourceMeter)` | Standalone component so per-resource ticks can run in a dedicated system. `ResourceMeter` stays an engine value type. |
| `damage_multiplier` | `i32` | `PlayerOffense::damage_multiplier` | New `PlayerOffense` component (room for future `crit`, `mana_efficiency` knobs). |
| `invincible` | `bool` | `PlayerOffense::invincible` *or* `PlayerHealth::invincible` | The flag actually drops incoming damage, not outgoing. **Decision: put on `PlayerHealth`** to keep the damage gate co-located with HP math. The existing `ae::Health::invulnerable` field is already there; this just makes the engine flag agree with it. |

### Combat state (already ECS)

| Field | Type | Destination | Notes |
| --- | --- | --- | --- |
| n/a | — | `PlayerCombatState` already owns `flash_timer`, `hitstop_timer`, `damage_invuln_timer`, `hitstun_timer`, `attacking`. | No change beyond removing the `PlayerBody::shielding` / `parrying` mirrors. |

### Combo / diagnostics → helper-local or deleted

| Field | Type | Destination | Notes |
| --- | --- | --- | --- |
| `combo` | `Vec<ComboMark>` | **Delete from the player entity.** Kept *inside the movement debug helper* if anything still consumes `combo_symbols()`; otherwise drop. | Aging the marks happens in `update_simulation_timers`. The combo HUD is debug-only; if it's still in the dev overlay, give it its own `PlayerComboTrace` component owned by the dev path. |
| `max_speed` | `f32` | Helper-local (or drop). | Used by the dev/HUD speedometer. If kept, becomes `PlayerSpeedometer { peak: f32 }` on the dev path. |
| `time_alive` | `f32` | `PlayerLifetime { time_alive: f32, resets: u32 }` *or* delete | Consumed by the trace recorder. Bundle with `resets` if kept. |
| `resets` | `u32` | Same as `time_alive`. | Reset path increments this; check whether anything outside the dev/trace path reads it before deciding to delete. |

### Implicit fields that disappear

These have no analogue on the ECS side because they encode "the aggregate is
the entity":

- `Player::new(spawn)` / `Player::new_with_abilities(spawn, abilities)` →
  becomes `PlayerSimulationBundle::new_at(spawn, abilities, health)` (the
  bundle already exists; the constructor body changes from "build the
  aggregate, then mirror it" to "build the components directly").
- `Player::reset_to(spawn)` → becomes a reset *system* (or helper fn that
  takes `&mut PlayerKinematics, &mut PlayerJumpState, … , spawn`) so the
  reset path explicitly enumerates which components it touches. The
  existing `crate::reset_player` site in `runtime/reset.rs` is the
  consolidation point.
- `Player::refresh_movement_resources(tuning)` → free function in
  `ambition_engine::movement` taking `&AbilitySet, MovementTuning, &mut
  PlayerJumpState, &mut PlayerDashState`. Resource refresh is grid-cell-
  scoped, not Player-scoped, so a free function is the right shape.
- `Player::aabb()` → method on `PlayerKinematics` (already mirrored by
  `PlayerBody::aabb`).
- `Player::spend_dash_charge()` / `Player::record(MovementOp)` /
  `Player::combo_symbols()` → free helpers in `engine::movement` that
  take only the dash component / combo trace they need.

## Updated component vocabulary (informed by the ledger)

Diff vs. the plan's §"Target component vocabulary":

- **Add `PlayerAbilities`** to the identity cluster (the plan didn't call
  this out separately).
- **Confirm `PlayerJumpState` and `PlayerDashState` own their buffer
  timers for the first cut**, and only fold them into a generic
  `PlayerActionBuffer` once attack/pogo/projectile/blink buffering lands.
  This matches the plan's §"Action buffers" first-pass scope.
- **Put `invincible` on `PlayerHealth`**, not on the offense cluster.
- **Drop `combo`/`max_speed`/`time_alive`/`resets` from the live
  component set** unless a current reader explicitly needs them; if so,
  isolate to `PlayerLifetime` / `PlayerComboTrace` / `PlayerSpeedometer`
  on the dev path.

## Timer-tick home decisions

Required by plan §"Triage / Timer tick home". After the cut, these systems
own the per-frame countdown of the fields in their bucket:

| System | Decrements |
| --- | --- |
| `tick_jump_dash_buffers` (new, in `app/sim_systems.rs`) | `jump_buffer_timer`, `dash_buffer_timer`, `dash_cooldown`, `coyote_timer`, `drop_through_timer`, `rebound_cooldown` |
| `tick_blink_state` (new) | `blink_cooldown`, `blink_hold_timer` (only while `hold_active`), `blink_grace_timer` |
| `tick_dodge_shield` (new) | `dodge_roll_timer`, `dodge_roll_cooldown`, `parry_window_timer` |
| `tick_wall_pre_vel` (new) | `pre_wall_vel_age` |
| `tick_ledge_release` (new) | `ledge_release_cooldown` |
| `cleanup_timers_system` (existing) | `flash_timer`, `slash_anim_timer`, `land_anim_timer`, `dash_startup_timer`, `blink_in_timer`, `camera_snap_timer` — already ECS-owned, no change. |
| `input_timer_system` (existing) | `hitstop_timer`, `damage_invuln_timer`, `hitstun_timer`, interaction tap timers — already ECS-owned, no change. |

The new tick systems can be one wide `tick_player_movement_timers` if it
keeps the per-entity query small. Splitting only matters once a system
needs to gate on something other than `(PlayerEntity, alive)`.

## Helper boundary

What stays in `ambition_engine` after `ae::Player` is gone:

- `geometry.rs` (`Aabb`, `AabbExt`, sweep helpers) — pure value math.
- `movement/collision.rs`, `movement/ops.rs` — sweep / one-way / wall
  collision. Refactor: take `&mut Vec2 pos, &mut Vec2 vel, Vec2 size, &World`
  instead of `&mut Player`.
- `movement/blink.rs` — blink destination math; takes `Vec2 origin,
  Vec2 aim, &World`.
- `movement/tuning.rs` — `MovementTuning` constants.
- `movement/integration.rs` — gravity / drag / glide / fast-fall
  velocity integration. Takes the component cluster references it needs,
  not `&mut Player`.
- `ledge_grab.rs` — `LedgeGrabState` value type + the `try_start_ledge_grab`
  predicate. Refactor: takes the kinematics / wall / ledge component
  references.
- `player_state.rs` — `LocomotionState`, `BodyMode`, `BodyShape`,
  `ResourceMeter`, `classify_player_safety`, `try_change_body_mode`. All
  shed their `&Player` arguments in favor of the component cluster
  references.
- `abilities.rs`, `combat.rs`, `combat_slots.rs`, `attack_choreography.rs`,
  `boss_encounter.rs`, `interaction.rs`, `quest.rs`, `cutscene.rs`,
  `save.rs`, `world.rs`, `actor.rs`, `actor_control.rs`, `kinematic.rs`,
  `projectile.rs`, `character_ai.rs`, `scalar.rs`, `debug.rs` — engine
  value types and helpers that don't depend on the player aggregate.

What gets deleted:

- `crates/ambition_sandbox/src/engine_core/movement/player.rs` (the `Player` struct).
- The high-level `update_player_control_with_tuning` /
  `update_player_simulation_with_tuning` entry points if they only operate
  on `Player` (they do today). Their bodies move into the sandbox ECS
  systems that own each component cluster.

## Open questions

These do not block Phase 1, but should be resolved before the cut lands:

1. **Where do `LocomotionState` projections live?**
   Today `LocomotionState::from_player(&Player)` is the sole projection
   point. After the cut, the natural shape is `LocomotionState::from(
   &PlayerGroundState, &PlayerWallState, &PlayerDashState, …)` — but the
   call sites are dev/HUD/trace, not the simulation loop. Probably becomes
   a free function in the sandbox's `dev/` or `presentation/` layer that
   takes whatever query data is convenient.

2. **Should `PlayerKinematics` carry `aabb()` directly or stay as a free
   helper?**
   Either works; preference is a method (`PlayerKinematics::aabb()`) so
   call sites read `kinematics.aabb()` like they do today on `PlayerBody`.

3. **Is `PlayerBody` worth keeping as a read-model?**
   Plan §"Target component vocabulary" suggests it can either disappear or
   become a small generated read-model. Current call sites of `PlayerBody`
   (camera follow, HUD, trace) read 3–5 fields; switching them to query
   `(&PlayerKinematics, &PlayerGroundState)` is roughly as ergonomic as
   keeping the read-model around. Recommendation: **delete `PlayerBody`**
   after Phase 3 lands; query the authoritative clusters directly. This
   keeps the architecture honest about where the truth lives.
