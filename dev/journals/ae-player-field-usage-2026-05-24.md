# `ae::Player` field-usage audit (2026-05-24)

## Why this exists

Chunk 4 of the universal-brain unification (see
`TODO-controllable-entity.md`) decomposes `ae::Player`'s 49-field
god-struct into ECS sibling components on the player entity. The
audit names every reader so the decomposition can move them in
clusters instead of blindly.

`ae::Player` is defined in
`crates/ambition_engine/src/movement/player.rs:26`.

## Field inventory (49 fields)

Grouped by intended sibling component, with the count of read sites
across `crates/ambition_sandbox/src`.

### `PlayerVelocity` — movement (per-tick)
- `pos: Vec2` (6)
- `vel: Vec2` (3)
- `facing: f32` (1)
- `on_ground: bool` (4)
- `max_speed: f32`

### `PlayerSize` — body envelope
- `size: Vec2`
- `base_size: Vec2` (1)

### `PlayerWallState` — wall interactions
- `on_wall: bool` (2)
- `wall_normal_x: f32`
- `wall_clinging: bool` (2)
- `wall_climbing: bool` (1)
- `pre_wall_vel: Vec2`
- `pre_wall_vel_age: f32`
- `ledge_release_cooldown: f32`

### `PlayerJumpState` — jump + coyote buffer
- `air_jumps_available: u8`
- `jump_buffer_timer: f32`
- `coyote_timer: f32`
- `rebound_cooldown: f32`
- `drop_through_timer: f32`

### `PlayerDashState` — dash + dodge
- `dash_charges_available: u8`
- `dash_timer: f32` (2)
- `dash_cooldown: f32`
- `dash_buffer_timer: f32`
- `dodge_roll_timer: f32`
- `dodge_roll_cooldown: f32`

### `PlayerFlightState` — fly + glide
- `fly_enabled: bool`
- `flight_phase: f32`
- `gliding: bool`
- `fast_falling: bool`

### `PlayerBlinkState` — blink ability
- `blink_cooldown: f32`
- `blink_hold_active: bool`
- `blink_hold_timer: f32`
- `blink_aiming: bool` (1)
- `blink_aim_offset: Vec2`
- `blink_grace_timer: f32`

### `PlayerShieldState` — bubble shield + parry
- `shield_active: bool`
- `parry_window_timer: f32`

### `PlayerLedgeState` — ledge grab
- `ledge_grab: Option<LedgeGrabState>`

### `PlayerBodyMode`
- `body_mode: BodyMode`

### `PlayerContact` — world contact (water + climbable)
- `water_contact: Option<WaterContact>` (4)
- `climbable_contact: Option<ClimbableContact>` (2)

### `PlayerResources` — mana
- `mana: ResourceMeter` (5)

### `PlayerCombo` — attack combo state
- `combo: Vec<ComboMark>` (2 reads of `combo_symbols`)

### `PlayerHealthMods`
- `damage_multiplier: i32` (1)
- `invincible: bool` (1)

### `PlayerLifetime`
- `time_alive: f32`
- `resets: u32`

### `abilities` — already pulled into ECS land
- `abilities: AbilitySet` (3 reads via `authority.player.abilities`)
  — this is already accessed through the authority; staying on the
  struct for now.

## File-by-file reader inventory

Files that read `ae::Player` or `PlayerMovementAuthority` (39 total):

```
app/dev_runtime.rs           runtime/setup.rs
app/hud.rs                   runtime/reset.rs
app/phases.rs                player.rs
app/sim_systems.rs           player/affordances/intent.rs
app/update.rs                player/affordances/mod.rs
app/world_flow.rs            player/bundles.rs
audio/environment.rs         player/components.rs
bin/headless.rs              player/ledge_grab.rs
body_mode/mechanics.rs       player/swim.rs
body_mode/morph_ball.rs      player/systems.rs
body_mode/tests.rs           presentation/character_sprites/anim.rs
content/features/bosses.rs   presentation/fx.rs
content/features/conversion_tests.rs    presentation/rendering/actors.rs
content/features/ecs/actors.rs          rl_sim/runtime.rs
content/features/ecs/bosses.rs          pause_menu/model.rs
dev/debug_overlay.rs         time/time_control.rs
dev/dev_tools.rs             world/platforms.rs
dev/trace/detect.rs          world/rooms/graph.rs
dev/trace/systems.rs         falling_sand.rs
dev/trace/tests.rs           lib.rs
```

Reader-cluster breakdown for sequencing decomposition:

| Cluster                  | Files                                              | Risk |
|--------------------------|----------------------------------------------------|------|
| Sim core (the writer)    | `app/sim_systems.rs`, `player/systems.rs`,         | High |
|                          | `app/update.rs`, `runtime/reset.rs`                |      |
| Movement adjuncts        | `body_mode/*`, `player/{swim,ledge_grab}.rs`,      | Med  |
|                          | `world/platforms.rs`                               |      |
| Combat/ability           | `player/affordances/*`, `content/features/*`,      | Med  |
|                          | `audio/environment.rs`                             |      |
| HUD / FX / dev / trace   | `app/hud.rs`, `dev/*`, `presentation/*`,           | Low  |
|                          | `pause_menu/model.rs`                              |      |

## Decomposition strategy chosen for Chunk 4

Following the plan and the "stale-component-after-sync-removal"
journal lesson (link in the plan), the migration is overlap-then-
delete:

- **4b**: Add per-cluster components (`PlayerVelocity`,
  `PlayerWallState`, `PlayerJumpState`, `PlayerDashState`,
  `PlayerContact`, etc.) as siblings on the player entity.
  `PlayerMovementAuthority` keeps writing `ae::Player`; a new
  sync system mirrors the relevant fields into the new
  components after every authority write.
- **4c**: Same pattern for combat/ability state — `PlayerCombo`,
  `PlayerShieldState`, `PlayerBlinkState`, `PlayerHealthMods`,
  `PlayerResources`.
- **4d**: Add a `Brain::Player` driver that builds an
  `ActorControlFrame` from the player's `PlayerInputFrame`. The
  frame is written into the entity's `ActorControl` component
  but nothing reads it yet — proves the brain seam ticks
  cleanly each frame.
- **4e+**: Reverse polarity — readers consume the new
  components instead of `ae::Player`. One reader-cluster per
  commit. Last cluster's commit also deletes `ae::Player` +
  `PlayerMovementAuthority`.

## Stopping rule

If 4e bogs down on broad call-site churn (the
[stale-component journal](../benchmark-candidates/bevy-ecs-stale-component-after-sync-removal-2026-05-15.md)
warns about exactly this), stop at the last green commit. The
overlap-then-delete pattern means the game still runs with both
shapes in place; finishing the polarity flip is a daytime
follow-up.

## What "done" looks like

`crates/ambition_engine/src/movement/player.rs::Player` deleted.
`PlayerMovementAuthority` deleted. Every reader in the file list
above queries one of the new components instead.
