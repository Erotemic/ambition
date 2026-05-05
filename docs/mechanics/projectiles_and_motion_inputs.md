# Player projectiles + motion-input recognizer

## Overview

The sandbox now has a player projectile mechanic with two variants:

- **Fireball** — cheap, mostly horizontal travel with mild downward
  arc (Mario-fireball style), 1 damage, ~1.2s lifetime.
- **Hadouken** — strong, straight, no gravity, 3 damage, ~1.6s
  lifetime, costs 3 resource units.

Press **F** (keyboard) or the **gamepad West face button** to fire a
Fireball. Perform a **half-circle motion (forward → down → back)**
ending while pressing fire to upgrade to a Hadouken; the motion
buffer recognizes both right-to-left and left-to-right gestures and
is tolerant of intermediate samples.

## Where it lives

```
crates/ambition_engine/src/projectile.rs
  ProjectileKind { Fireball, Hadouken }
  ProjectileSpec, ProjectileBody (per-tick state)
  ProjectileSpawner (cooldown + ResourceMeter)
  MotionDirection / MotionInputBuffer (recognizer)

crates/ambition_sandbox/src/projectile.rs
  PlayerProjectileState (Bevy resource)
  update_projectiles (Bevy system)
  ProjectileTraceEvent → GameplayTraceEvent::Projectile
```

The reusable primitives live in the engine; sandbox owns the input
sampling, world-collision check, and trace plumbing.

## Resource model

Both projectile kinds spend from a shared `ResourceMeter`:

| Variant   | Cost | Damage | Cooldown | Speed | Arc          |
| --------- | ---: | -----: | -------: | ----: | ------------ |
| Fireball  | 1.0  | 1      | 0.30 s   | 360   | gravity 360  |
| Hadouken  | 3.0  | 3      | 0.55 s   | 520   | straight     |

The default meter is 8.0 max with a 1.5/sec regen — hold-to-spam
fireball is fine, but Hadoukens have a real cost. The
`gameplay.player_damage_multiplier` setting scales outgoing damage
on top of the base values.

## Motion-input recognizer

`MotionInputBuffer` is a deque of recent `MotionDirection` samples
with a sliding window (default 0.45s in the sandbox). Each frame
the sandbox quantizes the deadzoned `axis_x / axis_y` into one of
9 directions (`Neutral` + 8 cardinals) and pushes it into the
buffer; held directions collapse to a single sample with the most
recent timestamp.

`detect_quarter_circle` looks for `Down → DownRight → Right` (or
mirror) in order; `detect_half_circle` looks for the longer
`Right → DownRight → Down → DownLeft → Left` (or mirror). Extra
or noisy samples between key directions are tolerated, which keeps
the recognizer usable for both keyboard arrow-key gestures and
analog stick rolls.

**Why a separate buffer:** the `update_projectiles` system never
parses raw input; it just asks `motion_buffer.detect_half_circle()`
on the same frame the player presses fire. Recognition logic can
grow (charge, dragon-punch, double-quarter) without touching the
spawner or input system.

## Sandbox wiring

`update_projectiles` runs after `sandbox_update` in the gameplay
chain:

1. Tick the spawner (cooldown decay + meter regen).
2. Sample motion direction from `ControlFrame.axis_x/y` (already
   deadzone-filtered) and push into the motion buffer.
3. Tick existing projectile bodies; drop on collision against
   `Solid` / `BlinkWall` blocks or on lifetime expiry.
4. If `control_frame.projectile_pressed`, decide kind (Hadouken
   when the recognizer fires) and call `spawner.try_spawn`.
5. Push trace events for fired / blocked / hit / expired phases.

Bevy entities are intentionally **not** spawned per projectile —
sandbox owns a `Vec<PlayerProjectile>` in the resource so headless
tests can observe motion / collision without rendering. A future
patch can layer sprites/VFX on top by querying the resource.

## Trace events

`GameplayTraceEvent::Projectile { tick, kind, event, damage }`. The
`event` string is one of `fired`, `blocked_by_resource`, `hit`,
`expired` so traces remain greppable.

## Tests

Engine (`cargo test -p ambition_engine --lib projectile::`):

- quarter-circle right / left recognition,
- half-circle recognition (mirror form returns opposite facing),
- tolerance against extra / noise samples,
- window pruning drops old samples,
- spawner blocks during cooldown,
- spawner blocks when out of resource,
- Fireball arcs downward, Hadouken travels straight,
- damage multiplier scales the spawned spec.

Sandbox (`cargo test -p ambition_sandbox --lib projectile::`):

- one fireball spawns on press,
- pre-loaded half-circle motion + press = Hadouken,
- second press during cooldown blocks,
- empty resource meter blocks the spawn.

## Future work

- VFX / SFX hookup (today the projectile is data-only).
- Damage to enemies / breakables (today only collides with solids).
- Per-projectile Bevy entities for mid-air visual rendering.
- Charge-up shot mechanic (hold fire to grow the projectile).
- Motion buffer extensions: dragon punch, double-quarter, charge
  motion (down-held + up).
