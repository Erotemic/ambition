# Body modes & locomotion state

Three Tier-1 backend primitives from `docs/mechanics_checklist.md` are now
landed in the engine. They are the foundation for the planned crouch /
crawl / morph-ball / projectile / hover / charge / etc. mechanics.

## `LocomotionState`

```rust
ambition_engine::player_state::LocomotionState
```

Explicit movement-mode enum. Replaces "infer from on_ground / dash_timer /
blink_aiming / wall_clinging" booleans for HUD, trace, and AI consumers.

Variants cover the shipping verbs and the planned-but-not-yet-wired ones
(Crouching, Crawling, Sliding, MorphBall, GrappleAiming, GrapplePulling,
CurveRiding, Hitstun) so adding a new mechanic does not require also
adding a new state variant in a downstream crate.

`LocomotionState::from_player(&Player)` is the minimum-viable projection
from the existing `Player` struct: it inspects the same fields the old
ad-hoc code already uses. Mechanics that own dedicated state machines
(future `seldom_state` `PlayerLocomotionMachine`, etc.) should bypass
this and drive the resource directly.

## `BodyMode` and `BodyShape`

```rust
ambition_engine::player_state::{BodyMode, BodyShape}
```

`BodyMode` is the stance enum: `Standing`, `Crouching`, `Crawling`,
`Sliding`, `MorphBall`. `BodyMode::shape(base_size)` returns a
`BodyShape { mode, size }` with the per-mode AABB size. The size table
lives in `shape()` and intentionally does not adapt to player tuning —
shape constants are part of the gameplay contract.

`BodyShape::fits_at(center, world, predicate)` is the collision-safe
resize primitive. Pass the predicate that defines what "blocks" a body
swap; typically `|b| matches!(b.kind, BlockKind::Solid)`.

This is enough to build, in this order:

1. **Crouch** *(Prototype — landed)* — see `crate::body_mode`. Down held
   while grounded swaps to `Crouching`; releasing Down attempts a
   stand-up via the engine helper `try_change_body_mode`, which adjusts
   `pos.y` to keep feet planted, runs `BodyShape::fits_at` on the
   target shape, and rejects the transition under a low ceiling. The
   trace recorder auto-detects the `body_mode` field change and emits
   a `PlayerModeChanged` event each frame the stance flips, so the
   driver does not push events itself.
2. **Crawl through low tunnel** — author a sandbox station with a
   `Solid` ceiling that has a one-tile-tall gap below it. `Crawling`
   shape fits; `Standing` does not. The collision-safe resize check is
   what gates re-entry.
3. **Morph ball** *(Prototype — landed)* — same `crate::body_mode`
   driver. Double-tap-down on the ground curls the player into
   `BodyMode::MorphBall`. The signal is
   `PlayerInteractionState::double_tap_down_pending`, set by
   `input_timer_system` when `register_down_tap` fires the second tap
   of a double-tap inside `feel.down_double_tap_window`; the body-mode
   driver in the progression chain consumes it via `mem::take`. Routing
   the edge through the `PlayerInteractionState` ECS component (rather
   than `ControlFrame`) is necessary because `sandbox_update` consumes
   its `ControlFrame` as a local copy that doesn't reach later systems
   — engine-side fast-fall sees the local-copy edge inline, but
   post-update mutators need a separate channel. Jump-pressed (or
   Up-pressed) inside the morph ball calls `try_change_body_mode` to
   try Standing; a low ceiling rejects the transition and the ball
   stays curled. The MorphBall AABB is smaller than Crouching on both
   axes (~55% of base on x and y) so it fits through tighter gaps.

Each of those becomes a sandbox proof for the underlying primitive
without each verb needing its own bespoke collision code.

### Crouch driver contract

`crate::body_mode::update_body_mode` still runs in the progression chain
after `sandbox_update`, but ledge grab and swim no longer live on that shelf:
both are owned by `ambition_engine::movement`. Body-mode resize remains a
sandbox-side mutator on `runtime.player`; the shrink case is a strict subset of
the previous AABB so collision repair on the next sim tick has nothing new to
fix, and the stand-up case calls `fits_at` itself before mutating, so the
simulator never sees a body penetrating a ceiling. The driver explicitly skips
the resize while `dash_timer > 0`, `blink_aiming`, `wall_clinging`,
`wall_climbing`, `ledge_grab`, or any `water_contact` is set so those mechanics
keep ownership of
the player posture.

Tests live in `crate::body_mode::tests` and cover: down-held-grounded
enters crouch, down-released stand-up succeeds in open space, low
ceiling blocks stand-up, airborne does not crouch, mid-dash blocks
crouch, wall-clinging blocks crouch. Engine-side tests in
`ambition_engine::player_state::tests` cover `try_change_body_mode`
directly: feet stay planted, base size restored on Standing, blocked
stand-up under a low ceiling, idempotent same-mode call.

## `ResourceMeter`

```rust
ambition_engine::player_state::ResourceMeter
```

Generic clamped meter with regen and decay rates. Use it for stamina,
mana, ammo, charge, hover fuel, oxygen, rage / super, or any other
"fills up over time, drains on use, has a hard cap" mechanic.

Key methods:

- `try_spend(cost) -> bool` — consumes if affordable, leaves the
  meter unchanged otherwise. Always honors the floor at zero.
- `tick_regen(dt)` / `tick_decay(dt)` — independent so mechanics that
  should regen only when idle (or decay only while in use) can call
  the matching half. `tick(dt)` runs both.
- `fraction()` / `is_full()` / `is_empty()` — HUD bar helpers.

The meter is intentionally serde-`Serialize` so a future save system
or RL observation can include it without an adapter.

## What this enables next

These primitives unlock the following mechanic checklist boxes (from
`docs/mechanics_checklist.md`) once the corresponding sandbox station
is wired:

- crouch / crawl / morph ball (body-state mechanics)
- collision-safe resize / unmorph validation
- compact hitbox mode
- alternate hurtbox by stance
- stamina / mana / ammo / hover fuel meters

The grapple, projectile, parry, and curve-motion mechanics from the
checklist need additional backends (targeting / shape-cast API,
projectile spawner, curve sampler) that are deliberately not in this
patch. They are listed in `MechanicsRegistry` with maturity `Planned`
so the HUD shows them and the next agent can pick them up.

## How to surface a new body-mode-driven mechanic

1. Add a `MechanicEntry` to `crate::mechanics::default_entries` with
   the right category and `Planned` / `Prototype` maturity.
2. Add a `BodyMode` variant if needed (rare — the existing five
   cover most ground-locomotion shapes).
3. Implement the input → state → resize logic in a new sandbox
   module. Call `BodyShape::fits_at` before committing the swap.
4. Emit a `GameplayTraceEvent::PlayerModeChanged` so the trace
   recorder captures the transition.
5. Bump the registry entry's maturity once the mechanic is playable
   in a sandbox station.

## Tests

`crates/ambition_engine/src/player_state.rs` tests cover:

- `LocomotionState::from_player` for grounded / airborne / dashing /
  blink-aiming defaults,
- `BodyMode::shape` produces smaller shapes for crouch and morph,
- `BodyShape::fits_at` returns true in open space, false against a
  solid block,
- `ResourceMeter::try_spend` succeeds / fails correctly,
- regen clamps to max, decay clamps to zero,
- `fraction` handles zero `max` without dividing by zero.

Run with `cargo test -p ambition_engine player_state::`.
