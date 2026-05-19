# Player singleton audit

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

| File | Line | Today | Migration target |
|---|---|---|---|
| `presentation/rendering/camera.rs` | 87, 154 | `camera.single()` / `player.single()` | `PrimaryPlayerOnly` filter |
| `presentation/rendering/parallax.rs` | 112 | `camera.single()` | `PrimaryPlayerOnly` filter (via camera-follows-primary) |
| `presentation/rendering/foreground.rs` | 85, 89 | `camera.single()` | `PrimaryPlayerOnly` filter |
| `presentation/rendering/health.rs` | 39 | `player.single()` | `PrimaryPlayerOnly` filter for the HUD health bar; per-player overlays should iterate when added |
| `app/hud.rs` | 132, 149 | `camera_params.player.single()` | `PrimaryPlayerOnly` filter; HUD already comments that it's primary-only |
| `audio/environment.rs` | 157, 401, 417 | `Query<..., With<PrimaryPlayer>>` ✓ | Already uses `With<PrimaryPlayer>` (correct) |
| `dev/trace/systems.rs` | 123 | `player_q.single()` | `PrimaryPlayerOnly` filter — trace dump should follow primary |
| `dev/debug_overlay.rs` | 103 | `player_q.single()` | `PrimaryPlayerOnly` filter — debug overlay is primary-only |
| `dev/fps_overlay.rs` | 161, 184 | `query.single_mut()` | One UI text node — single() is correct (not a player query) |
| `dev/dev_tools.rs` | 563, 704, 724 | `player_q.single_mut()` / `health_q.single_mut()` | `PrimaryPlayerOnly` filter; dev hotkeys target the primary's player by intent |
| `body_mode/morph_ball.rs` | 148, 154 | `ball_query.single_mut()` / `player_q.single()` | `PrimaryPlayerOnly` filter for the player; the ball is per-player visual (should become per-entity) |
| `player/bubble_shield.rs` | 115, 116 | `player_q.single()` / `shield_q.single_mut()` | Bubble shield is per-player gameplay state — should become a per-player component, not a singleton |
| `time/time_control.rs` | 277 | `primary.single()` ✓ | Already filtered by `With<PrimaryPlayer>` |
| `app/input_systems.rs` | 72, 94, 133 | `player_input.single()` | Today's local-input ActionState is global; per-player input is OVERNIGHT-TODO #17.5 |
| `pause_menu/input.rs` | 136 | `dev_toggles.player_q.single_mut().ok()` | Dev-pause input — primary-only is fine |
| `host/mobile_input/bevy_plugin.rs` | 562, 577 | `windows.single()` | Window query (not player) — single() is correct |
| `host/mobile_input/menu_bridge.rs` | 184 | `windows.single()` | Window query — single() is correct |
| `persistence/settings/model.rs` | 1247 | `windows.single_mut()` | Window query — single() is correct |

### B — Should iterate ALL players (hazards, pickups, enemy attacks, world interactions)

These currently use `single()` and assume "the player" but the semantics
demand "every player that overlaps / interacts." Migration cost is the
highest here because each site needs to drop the singleton, iterate, and
typically filter by faction/state. Today single-player parity is preserved.

| File | Line | Today | Should become |
|---|---|---|---|
| `content/features/ecs/mod.rs` | 910, 967, 1027, 1097, 1177, 1280, 1490 | `player.single()` for pickup/breakable/chest/interactable overlap and switch toggles | `for player_body in players.iter()` — any overlapping player triggers; pickups/chests probably award the entity that touched them |
| `content/features/ecs/damage.rs` | 281 | `player_combat_q.single_mut()` | Per-player damage routing (OVERNIGHT-TODO #17.6) |
| `enemy_projectile/systems.rs` | 32 | `player_body_q.single().ok()` for enemy aim target | Target nearest hostile actor (OVERNIGHT-TODO #17.8) |
| `encounter/systems.rs` | 130 | `player_body_q.single()` for encounter trigger overlap | Iterate players; any player triggers |
| `projectile/systems.rs` | 164 | `player_body_q.single()` for player projectile spawn | Should be per-player owner (OVERNIGHT-TODO #17.7) |
| `projectile/visuals.rs` | 62 | `player_body_q.single()` for projectile recolor based on player state | Per-projectile owner reference |
| `presentation/fx.rs` | 480 | `player_authority.single()` for screen FX origin | Primary-player FX is acceptable; multi-player split-screen would need per-player FX |

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
| `CurrentPlayerAttack` | `Resource(Option<PlayerAttackState>)` | `ActivePlayerAttack` component on the player entity (OVERNIGHT-TODO #17.4) — already commented in `lib.rs` |
| `SandboxSimState::last_safe_player_pos` | `Vec2` on a shared `Resource` | `PlayerSafetyState` component (OVERNIGHT-TODO #17.9) — already commented in `lib.rs` |
| `SandboxSimState::time_scale` | global | Stays global (hitstop/bullet-time apply to the whole world) |
| `SandboxSimState::room_transition_cooldown` | global | Stays global today; would need per-room/per-player if rooms ever diverge |
| `KeyboardPreset` selection (`SandboxDevState.preset_index`) | global | Per-player input mapping is OVERNIGHT-TODO #17.5 |

## Recommended ordering for migration

1. **A-bucket migration (cosmetic, no behavior change)** — Adopt the
   `PrimaryPlayerOnly` filter at the camera/HUD/dev-tool/audio sites listed
   above. Mechanical, behavior-preserving, makes the intent visible without
   risk to single-player play. Good agent-sized first patch.
2. **Per-entity attack state (OVERNIGHT-TODO #17.4)** — Move
   `CurrentPlayerAttack` onto the player entity as
   `ActivePlayerAttack`. Smoke test: spawn two players with different
   active attacks and assert they tick independently.
3. **Per-entity safety state (OVERNIGHT-TODO #17.9)** — Move
   `last_safe_player_pos` onto the player entity as
   `PlayerSafetyState`. Smoke test: per-player respawn anchors.
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
