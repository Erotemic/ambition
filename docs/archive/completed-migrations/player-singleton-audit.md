# Player singleton audit

> **Status (2026-05-28):** **HISTORICAL.** This audit predates the
> `ae::Player` deletion and the 18-cluster decomposition. Code/file
> references to `PlayerMovementAuthority` / `PlayerBody` / the
> monolithic `ae::Player` aggregate describe the as-of-2026-05-19
> shape; the post-migration shape uses the 18 cluster components on
> the player entity directly. The singleton-call-site classification
> remains useful — the cluster components carry `PlayerEntity` /
> `PrimaryPlayer` / `LocalPlayer` markers just like the legacy
> aggregate did. See `docs/current/state.md` for the post-migration
> player state.

Source-of-truth for OVERNIGHT-TODO item #17 (Player/enemy actor unification +
multiplayer-readiness). Step #17.1 calls for an audit and classification of
every singleton call site that assumes "exactly one player." This file is
that audit.

Validated 2026-05-19 by running:

```bash
rg -n '\.single\(\)|\.single_mut\(\)|\.get_single\(\)|\.get_single_mut\(\)' \
   --type rust -g '!target' crates/ambition_sandbox/src
rg -n 'With<.*PlayerEntity>' --type rust -g '!target' crates/ambition_sandbox/src
rg -n 'PrimaryPlayer' --type rust -g '!target' crates/ambition_sandbox/src
```

## Existing scaffolding (already in place)

- `PrimaryPlayer` marker component — set on the local primary player.
- `LocalPlayer` marker component — set on every player driven by this client.
- `PlayerSlot(u8)` component — orders multi-player iteration deterministically.
- `PrimaryPlayerOnly` filter (`(With<PlayerEntity>, With<PrimaryPlayer>)`) —
  use as the filter parameter on a `Query` when you intend "the camera/HUD
  target, not just any player."
- `primary_player_entity(&Query<Entity, PrimaryPlayerOnly>) -> Option<Entity>`
  helper.
- `sort_players_by_slot` helper for deterministic multi-player iteration.

These live in [`player/queries.rs`](../../crates/ambition_sandbox/src/player/queries.rs)
and [`player/components.rs`](../../crates/ambition_sandbox/src/player/components.rs).

## Singleton call sites by classification

### A — Intentionally primary-player-only (camera, HUD, dev tools, audio listener)

These are safe to keep as `single()` today; they should migrate to
`PrimaryPlayerOnly` filters so the singleton intent is visible at the call
site rather than implied by `With<PlayerEntity>`. No behavior change.

**Status (2026-05-19):** All player-query A-bucket sites have been migrated
to `PrimaryPlayerOnly`. Window-only and UI-root `single()` sites are
intentionally singleton and need no change.

| File | Line | Today | Status |
|---|---|---|---|
| `presentation/rendering/camera.rs` | 87, 154 | `camera.single()` / `player.single()` | ✓ migrated 257aca4..0902de6 |
| `presentation/rendering/parallax.rs` | 112 | `camera.single()` | camera filter — drives off primary-followed camera; no player query |
| `presentation/rendering/foreground.rs` | 85, 89 | `camera.single()` | camera filter — drives off primary-followed camera; no player query |
| `presentation/rendering/health.rs` | 39 | `player.single()` | ✓ migrated 0902de6 |
| `app/hud.rs` | 132, 149 | `camera_params.player.single()` | ✓ migrated 0902de6 (HudCameraParams uses PrimaryPlayerOnly) |
| `audio/environment.rs` | 157, 401, 417 | `Query<..., With<PrimaryPlayer>>` ✓ | Already correct before audit |
| `dev/trace/systems.rs` | 123 | `player_q.single()` | ✓ migrated 0902de6 |
| `dev/debug_overlay.rs` | 103 | `player_q.single()` | ✓ migrated 0902de6 |
| `dev/fps_overlay.rs` | 161, 184 | `query.single_mut()` | UI text node — singleton is intentional |
| `dev/dev_tools.rs` | 563, 704, 724 | `player_q.single_mut()` / `health_q.single_mut()` | ✓ migrated 0902de6 |
| `body_mode/morph_ball.rs` | 148, 154 | `ball_query.single_mut()` / `player_q.single()` | ✓ migrated 0902de6 (player_q); the ball singleton stays per-presentation today |
| `player/bubble_shield.rs` | 112, 115, 116 | `player_q.single()` / `shield_q.single_mut()` | ✓ migrated d183a9d (player_q); shield visual singleton stays primary-only today |
| `time/time_control.rs` | 277 | `primary.single()` ✓ | Already correct before audit |
| `app/input_systems.rs` | 72, 94, 133 | `player_input.single()` | Today's local-input ActionState is global; per-player input is OVERNIGHT-TODO #17.5 |
| `pause_menu/input.rs` | 136 (via model.rs `DevToggleParams::player_q`) | `dev_toggles.player_q.single_mut().ok()` | ✓ migrated fff5829 |
| `presentation/fx.rs::update_blink_preview` | 480 | `player_authority.single()` | ✓ migrated fff5829 |
| `host/mobile_input/bevy_plugin.rs` | 562, 577 | `windows.single()` | Window query (not player) — singleton is intentional |
| `host/mobile_input/menu_bridge.rs` | 184 | `windows.single()` | Window query — singleton is intentional |
| `persistence/settings/model.rs` | 1247 | `windows.single_mut()` | Window query — singleton is intentional |

### B — Should iterate ALL players (hazards, pickups, enemy attacks, world interactions)

These currently use `single()` and assume "the player" but the semantics
demand "every player that overlaps / interacts." Migration cost is the
highest here because each site needs to drop the singleton, iterate, and
typically filter by faction/state. Today single-player parity is preserved.

| File | Line | Today | Should become |
|---|---|---|---|
| ~~`content/features/ecs/pickups.rs`~~ ✓ DONE 2026-05-19 (a086f07) | Was `player.single()` | Now iterates every player; first player to overlap collects. Heal/banner is still implicitly primary until #17.6 lands target fields |
| ~~`content/features/ecs/chests.rs`~~ ✓ DONE 2026-05-19 (1ba01b0) | Was `player.single_mut()` | Now iterates every player's interact buffer; `Opened` marker is the source-of-truth for "this chest is taken" so concurrent-frame reaches race deterministically |
| ~~`content/features/ecs/breakables.rs`~~ ✓ DONE 2026-05-19 (a086f07) | Was `player_body_q.single()` | Now any player standing triggers the break |
| ~~`content/features/ecs/hazards.rs`~~ ✓ DONE 2026-05-19 (c626d35) | Was `player.single()` | Now iterates every overlapping player — co-op-ready for the "no targeting needed" pattern |
| ~~`content/features/ecs/bosses.rs`~~ ✓ DONE 2026-05-19 (f0a4e08) | Was `player_query.single()` | Now `PrimaryPlayerOnly` filter documents the "boss targets primary player" decision until #17.8 lands per-target AI |
| ~~`content/features/ecs/actors.rs`~~ ✓ DONE 2026-05-19 (f0a4e08) | Was `player_query.single()` | Now `PrimaryPlayerOnly`; enemy targeting decision is visible at the query |
| ~~`content/features/ecs/interact.rs`~~ ✓ DONE 2026-05-19 (0a569dd) | Was `player.single_mut()` | Now iterates every player. Dialogue stays global (one GameMode::Dialogue); switch activation is per-target so different players can flip different switches in the same frame |
| `content/features/ecs/damage.rs` | 288 | `player_combat_q.single_mut()` | Per-player damage routing (OVERNIGHT-TODO #17.6) |
| ~~`enemy_projectile/systems.rs`~~ ✓ DONE 2026-05-19 (bd306f0) | Was `player_body_q.single().ok()` | Now iterates every player; the first vulnerable overlapping player takes the hit |
| ~~`encounter/systems.rs`~~ ✓ DONE 2026-05-19 (4ece6ad) | Was `player_body_q.single()` for encounter trigger overlap | Now iterates every player; the first overlapping player fires the trigger |
| ~~`projectile/systems.rs`~~ ✓ DONE 2026-05-19 | Was `player_body_q.single()` for player projectile spawn | Now `PrimaryPlayerOnly` — spawn anchored to the primary local player whose `ControlFrame` pressed fire. Per-player input + projectile-owner is OVERNIGHT-TODO #17.5 + #17.7 |
| ~~`projectile/visuals.rs`~~ ✓ DONE 2026-05-19 | Was `player_body_q.single()` for charge indicator anchor | Now `PrimaryPlayerOnly` — charge indicator follows the primary local player; per-player charge UI is #17.5 |
| ~~`presentation/fx.rs`~~ ✓ DONE | `player_authority.single()` already uses the `PrimaryPlayerOnly` filter — screen FX is intentionally tied to the primary player. Split-screen FX is a future-multi-camera concern, not a B-bucket migration. |

**B-bucket pattern split (post 2026-05-19):**

- **No targeting needed (hazards, projectiles)** — iterate every player, hit every overlapping player. Already done for `ecs/hazards.rs` (c626d35) and `enemy_projectile/systems.rs` (bd306f0).
- **Targeting decision deliberate (bosses, enemy AI)** — `PrimaryPlayerOnly` filter until per-target selection lands. Already done for `ecs/bosses.rs` and `ecs/actors.rs` (f0a4e08). Real multi-target boss AI is OVERNIGHT-TODO #17.8.

### C — Should target a specific player slot / entity (input, attack state, damage, healing)

These already conceptually take a target. The fix is to move global
resources / per-frame state onto per-player components.

| File | Line | Today | Should become |
|---|---|---|---|
| `app/update.rs` | 83 | `player_q.single_mut()` driving the main sim tick | Iterate players; each gets its own sim step (OVERNIGHT-TODO #17.4/#17.5) |
| `app/sim_systems.rs` | 42, 100, 159, 220, 279, 347, 425, 496 | `player_q.single_mut()` across input/movement/animation/death systems | Per-player components and iteration |
| `app/world_flow.rs` | 218 | `player_q.single_mut()` on room transition | Iterate players or operate on the transitioning slot |
| `app/dev_runtime.rs` | 146 | `player_q.single_mut()` during LDtk hot-reload | Iterate; reload preserves every player's position relative to room |
| `runtime/reset.rs` | 178 | `player_q.single_mut()` in sandbox reset | Iterate; reset every player to spawn |
| `body_mode/mechanics.rs` | 55 | `player_q.single_mut()` for body-mode transitions | Per-player body-mode |
| `player/systems.rs` | 26, 38 | `players.single_mut()` for combat/health sync | Per-entity sync (already a query over `PlayerEntity`, just iterate) |
| `map_menu/ui.rs` | 147, 154, 161, 185, 200 | UI root `single()` | Singleton intent (one map menu) — single() is correct; if per-player HUD lands, the map UI may need a primary filter |

## Global resources that implicitly model "the player"

These predate the per-player audit. Each is a candidate for becoming a
per-`PlayerEntity` component.

| Resource | Current shape | Per-player target |
|---|---|---|
| ~~`CurrentPlayerAttack`~~ ✓ DONE 2026-05-19 (2aab57e) | Was `Resource(Option<PlayerAttackState>)` | Now `ActivePlayerAttack` component on the player entity (OVERNIGHT-TODO #17.4) |
| ~~`SandboxSimState::last_safe_player_pos`~~ ✓ DONE 2026-05-19 (ea12dee) | Was a field on a shared `Resource` | Now `PlayerSafetyState { last_safe_pos }` component (OVERNIGHT-TODO #17.9) |
| `SandboxSimState::time_scale` | global | Stays global (hitstop/bullet-time apply to the whole world) |
| `SandboxSimState::room_transition_cooldown` | global | Stays global today; would need per-room/per-player if rooms ever diverge |
| `KeyboardPreset` selection (`SandboxDevState.preset_index`) | global | Per-player input mapping is OVERNIGHT-TODO #17.5 |

## Recommended ordering for migration

1. **A-bucket migration (cosmetic, no behavior change)** — ✓ DONE
   (2026-05-19). All A-bucket sites use `PrimaryPlayerOnly` now.
2. **Per-entity attack state (OVERNIGHT-TODO #17.4)** — ✓ DONE
   (2026-05-19 commit 2aab57e). `ActivePlayerAttack` component
   replaces `CurrentPlayerAttack` resource; multiplayer smoke tests
   `two_players_have_independent_active_attacks` and
   `clear_is_per_entity` cover the per-entity invariant.
3. **Per-entity safety state (OVERNIGHT-TODO #17.9)** — ✓ DONE
   (2026-05-19 commit ea12dee). `PlayerSafetyState { last_safe_pos }`
   component replaces `SandboxSimState::last_safe_player_pos`;
   `two_players_have_independent_safety_anchors` smoke test pins
   the invariant.
4. **B-bucket: iterate-all-players for pickups / hazards / interactions**
   (OVERNIGHT-TODO #17.6, #17.7, #17.8) — These are the meaty behavior
   changes. Stage one feature kind at a time (pickups first; they have the
   least cross-system entanglement).
5. **Per-player input (OVERNIGHT-TODO #17.5)** — Touch this last; input is
   wired through `leafwing-input-manager` and several mobile/touch seams.

## Open questions surfaced by the audit

- **`PlayerBody` vs. `PlayerMovementAuthority`** — Many call sites read
  `PlayerBody` for position/size/facing snapshots, but `PlayerMovementAuthority`
  owns the authoritative `ae::Player`. Per-player iteration should pick the
  read-only `PlayerBody` path where possible to avoid contention.
- **`PlayerSlot` ordering** — `sort_players_by_slot` exists but is not used
  by any system yet. Once iteration starts, all multi-player loops should
  go through it to keep frame-to-frame ordering stable.
- **`LocalPlayer` vs. `PrimaryPlayer`** — In a future multi-client build,
  every client's local players carry `LocalPlayer`; only one of those is
  the camera/HUD `PrimaryPlayer`. The audit above keeps "camera target" =
  `PrimaryPlayer`; remote players never get it. Check this assumption when
  networking lands.
