# Body modes

Body modes describe traversal states that change collision shape, movement affordances, or player posture: crouch, crawl, slide, compact/morph-like movement, climb, swim/sink variants, and future specialized traversal modes.

## Current status

- Reusable vocabulary lives in `crates/ambition_engine_core/src/player_state.rs`: `LocomotionState`, `BodyMode`, `BodyShape`, and `ResourceMeter`.
- The sandbox has body-mode modules under `crates/ambition_sandbox/src/body_mode/`.
- Player-authoritative runtime state lives on ECS player components under `crates/ambition_sandbox/src/player/` and the engine `Player` state.
- Authored traversal examples should be LDtk rooms/specs, not hard-coded one-off checks.

## Engine contract

### `LocomotionState`

`LocomotionState` is an explicit movement-mode enum for HUD, trace, AI, and future state machines. It replaces ad-hoc inference from `on_ground`, `dash_timer`, `blink_aiming`, `wall_clinging`, and similar booleans. `LocomotionState::from_clusters(...)` is the current projection from the existing player struct; future mechanics with dedicated state machines may drive the value more directly.

### `BodyMode` and `BodyShape`

`BodyMode` is the stance enum. Current variants include:

```text
Standing, Crouching, Crawling, Sliding, MorphBall, Climbing
```

`BodyMode::shape(base_size)` returns the AABB size for each mode. `BodyShape::fits_at(center, world, predicate)` is the collision-safe resize primitive. Callers choose which blocks count as blockers, typically `Solid` and sometimes `OneWay` for stand-up checks under one-way ceilings.

Current shape policy:

- Standing uses the base player size.
- Crouching keeps width and uses roughly 55% height.
- Crawling is shorter and slightly narrower.
- Sliding is low and slightly wider.
- MorphBall is compact and symmetric.
- Climbing currently keeps the standing silhouette so ladder/climbable intersection remains stable.

### `ResourceMeter`

`ResourceMeter` is the generic clamped meter for stamina, mana, ammo, charge, hover fuel, oxygen, rage/super, and similar mechanics. Use `try_spend`, `tick_regen`, `tick_decay`, `fraction`, `is_full`, and `is_empty` rather than hand-rolling one-off meters.

## Driver contract

A body-mode driver should:

1. Interpret input as a requested mode change.
2. Refuse to take ownership when another locomotion mechanic owns posture, such as dash, blink aim, wall cling/climb, ledge grab, or water policy.
3. For shrink transitions, commit the smaller shape and let the next movement tick continue normally.
4. For expansion transitions, call the engine fit probe before changing shape so the simulator never sees the player penetrating a ceiling.
5. Keep feet planted when changing height unless the mechanic explicitly says otherwise.
6. Let the trace recorder observe `body_mode` changes rather than manually duplicating trace events in every driver.

Current examples:

- Down-held while grounded can enter Crouching.
- Releasing Down attempts a stand-up and remains crouched under a low ceiling.
- Double-tap-down can enter MorphBall where enabled.
- Jump or Up attempts to exit MorphBall, subject to stand-up collision checks.
- Climbing is entered from climbable contact and should be exited by jump, dash/push-off policy, losing contact, or explicit drop-through rules.

## Design rules

- A body mode must define its collision shape and the rules for entering/exiting that shape safely.
- Shape changes must fail gracefully when the expanded shape would overlap collision.
- Input interpretation should produce a requested body mode; collision and movement systems decide whether the transition is legal.
- Presentation should reflect mode changes but not be the source of truth.
- Keep water, iron-boots, swim, sink, and murky variants as mode policy layered on top of the same shape/affordance vocabulary where practical.
- Avoid adding a new `BodyMode` variant when an existing shape plus a separate gameplay policy would be clearer.

## Current and likely modes

| Mode | Backend status | Notes |
|---|---:|---|
| Standing/running | Available | Default platforming shape. |
| Crouch/crawl | Available vocabulary | Needs more authored traversal rooms. |
| Slide | Available vocabulary | Tune around collision-safe transitions and momentum. |
| Compact/morph-like traversal | Available vocabulary | Needs stronger showcase coverage. |
| Climbing/ladder | Available vocabulary | Needs movement-speed, jump-off, dash-off, and ladder-top polish. |
| Swim/surface swim/sink/iron-boots water modes | Design direction | Should reuse body-mode + volume policy where possible. |
| Spider-ball/spring-ball/bomb traversal | Future | Needs explicit backend semantics before content authoring. |

## How to add a body-mode-driven mechanic

1. Add or update a mechanic entry if the HUD/registry should advertise it.
2. Reuse an existing `BodyMode` where possible.
3. Implement input -> requested mode -> collision-safe commit in a focused sandbox module.
4. Use `BodyShape::fits_at` before expansion.
5. Add trace/replay coverage if the mechanic can affect collision, movement, or room transitions.
6. Add or update an LDtk showcase room once the backend behavior is stable.

## Validation anchors

```bash
cargo test -p ambition_sandbox --lib engine_core::player_state
cargo test -p ambition_sandbox --lib engine_core::movement
cargo test -p ambition_sandbox body_mode
cargo test -p ambition_app --test scripted_gameplay --features "rl_sim portal"
```
