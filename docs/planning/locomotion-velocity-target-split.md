# Split `ActorControlFrame::desired_vel` → `locomotion` + `velocity_target`

Status: **designed, not yet landed.** Must land as ONE atomic change (removing a
field is all-or-nothing) — do it in a window when no other agent is editing
`ambition_characters` or `ambition_gameplay_core`.

## Why

`desired_vel` is overloaded with three meanings, split across actor types:

| Producer | meaning today | unit | frame |
|---|---|---|---|
| player / player_demo / possession | normalized stick intent | dimensionless `[-1,1]` | local |
| grounded enemy / NPC brain | velocity-valued side speed | px/s | local-side |
| boss pattern / aerial steering | exact 2D velocity | px/s | world |

The grounded enemy/NPC consumer re-packs the velocity into the spine's
`axis_x * max_run_speed` model by setting `max_run_speed = |desired|`,
`axis_x = sign` per tick — a hack that exists only because player (normalized)
and enemy (velocity) drive the *same* shared `integrate_normal_spine` through the
*same* field with *different* units. That player/enemy divergence is the smell.

## Target design (the principles it must satisfy)

- **Players and enemies unified** — one field, one meaning, one consumer line.
- **Correct behavior emergent from structure**, not preserved by per-tick hacks.
- **Jitter is intent, not capability** — a character's `max_run_speed` is fixed
  ("what it can do"); an enemy's per-spawn speed jitter is the brain *choosing*
  to throttle, encoded in the intent.

Two fields:

```rust
/// Normalized controlled-body-local locomotion intent. |·| ≤ 1 throttle —
/// "how hard, of what this body is capable", NOT a velocity. Every self-
/// locomoting actor (player, possession, grounded AI) writes this. The
/// integrator resolves velocity uniformly as `locomotion * max_run_speed`,
/// with NO per-actor-type branch.
pub locomotion: Vec2,
/// Exact world-space velocity (px/s) for the free-mover / choreography
/// modality: boss patterns that snap to a scripted velocity, AI flyers that
/// steer a 2D velocity directly. `None` ⇒ locomotes via `locomotion`.
/// Mode-based, not actor-type-based — so it does NOT reintroduce a split.
pub velocity_target: Option<Vec2>,
```

The capability channel:

- `EnemyTuning.max_run_speed: f32` (and the NPC equivalent) = the body's ground
  run capability. Set at catalog-resolve = `patrol_speed.max(chase_speed)`.
- `BrainSnapshot.max_run_speed: f32` = same capability, surfaced to the brain.
  Real builders: `features/npcs.rs:56`, `features/ecs/actors/update.rs:756`,
  `player/systems.rs:77`, plus `BrainSnapshot::idle()` and the smash test builder.

## Producer migration

Grounded AI brains throttle by capability (jitter rides along *as intent*):
```rust
// was: out.desired_vel = ae::Vec2::new(facing * cfg.speed, 0.0);
out.locomotion = ae::Vec2::new(facing * cfg.speed / snapshot.max_run_speed, 0.0);
```
Sites in `brain/state_machine/mod.rs`: tick_patrol (289, 297), tick_wanderer
(413, 420), tick_melee_brute (475), tick_player_demo (1180/1184/1191/1202 — these
are *already* normalized `run_axis`, just rename field). `brain/smash/emit.rs`
(50/54/61/101 — walk/dash throttles of capability).

Player brain (`brain/player.rs:85`) and possession
(`features/ecs/actors/update.rs:292,568`) are already normalized — rename field;
possession's `* POSSESSED_MOVE_SPEED` stays (that is the possessed body's
capability).

Aerial / boss / choreography brains → `velocity_target = Some(world_vel)`:
`state_machine/mod.rs` tick_skirmisher (612-613), tick_shark (759/774/780),
tick_aerial_* (948/980/1026/1045/1060/1078), `brain/boss_pattern/tick.rs:590`,
`content/.../cut_rope/arena.rs:276`.

## Consumer migration (the payoff: uniform, branch-free)

`features/enemies/integration.rs` grounded branch (124-165) and
`features/npcs.rs` `integrate_velocity` (160-208) collapse to the SAME shape the
player already uses — delete the `max_run_speed = |desired|` / `axis = sign`
decomposition:
```rust
let spine_tuning = ae::MovementTuning { max_run_speed: tuning.max_run_speed, .. };
ae::integrate_normal_spine(.., ae::InputState { axis_x: frame.locomotion.x, .. }, ..);
```
Aerial branch + `step_floating_body` (`features/mod.rs:58`, param rename
`desired_vel`→`velocity_target`) + boss `integrate_body`
(`combat/boss_clusters.rs`, `features/bosses.rs:909`, `ecs/bosses/tick.rs:346`)
read `frame.velocity_target.unwrap_or(ZERO)`.

## Tests / trace to update

`conversion_tests.rs` (576/646/727 — set `locomotion=(1,0)` + rely on the
enemy's `max_run_speed`), `state_machine/tests.rs` + `smash/mod.rs` pre-poison
sites (99/999), `brain/mod.rs:367,645-646`, `player/clone_probe_tests.rs:33-34`,
`app/world_flow/attack.rs:27-28`, `player/systems.rs:423`, `control.rs:260`.

## Acceptance

- `integrate_normal_spine` is fed identically for player and enemy — grep proves
  no actor-type branch remains in the run path.
- `conversion_tests` (wall-collision) green; brain + smash + boss tests green.
- Follow-on (NOT this change): step 6 — give the spine a 2D-accel path so
  `to_world(locomotion)` enables arbitrary-angle acceleration; and unify AI
  flight onto normalized intent too.
