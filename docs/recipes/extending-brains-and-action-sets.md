# Extending brains and ActionSets

A practical recipe for current work on the universal-brain interface. See `docs/systems/brain-driver.md` for the overview.

**Review date:** 2026-05-27. Reviewed against source archive `ambition-source-2026-05-26T222032-5-3e93516618a5`.

## Current status to remember

The brain/action pipeline is live. Do not follow old instructions that say the message stream is only observed.

Live consumers today include:

- player melee-start gating from this player's `ActorActionMessage::Melee`;
- hostile enemy ranged projectiles from `ActorActionMessage::Ranged`;
- hostile enemy melee windup starts from `ActorActionMessage::Melee`;
- GNU-ton apple rain and Gradient Sentinel specials from `ActorActionMessage::Special`.

Still-direct paths include:

- player projectile charge/motion-input handling in `update_projectiles` reading `PlayerInputFrame`;
- player-specific pogo input inside the attack lifecycle;
- runtime-owned windup/active/recover timers for enemy hitbox spawning;
- parts of boss/body runtime state.

## Three places work usually lands

1. **Brain template** (`crates/ambition_sandbox/src/brain/state_machine.rs`) — when an actor needs a new policy or state graph. Add a new `StateMachineCfg` variant only when existing templates cannot express the behavior.
2. **ActionSet spec** (`crates/ambition_sandbox/src/brain/action_set.rs`) — when an actor needs a new concrete capability: melee, ranged, move style, or special.
3. **Effect consumer** (`crates/ambition_sandbox/src/content/features/ecs/brain_effects.rs` or another focused module) — when an `ActionRequest` is emitted but not yet translated into hitboxes, projectiles, VFX/SFX, boss hazards, or other world effects.

Per-entity mapping usually lives in `crates/ambition_sandbox/src/content/features/ecs/spawn.rs` or the relevant boss/profile setup code.

## Adding a new brain template

A brain template is reusable policy. Two enemies sharing the template share state-machine code but can still look different because their ActionSets resolve abstract intent differently.

1. Add the variant, config, and state to `state_machine.rs`.
2. Add a `tick_<template>` function that always starts by writing `ActorControlFrame::neutral()`.
3. Add the dispatch arm in `tick_state_machine` and, if needed, `tick_state_machine_with_actions`.
4. Extend `StateMachineCfg::is_hostile()` and `Brain::label()` coverage.
5. Re-export the new types from `brain/mod.rs` when they are part of the public surface.
6. Add pure unit tests for state transitions before wiring an archetype.
7. Map the relevant actor/archetype to the new template at spawn.

Rules:

- Do not allocate in per-tick snapshot/tick logic unless there is a measured reason.
- Do not make `Brain` a trait object; enum dispatch is intentional.
- Do not add a brain template just to vary attack shape. Use `ActionSet` for that.

## Adding a new ActionSet spec

An ActionSpec is the concrete effect an actor performs when its brain emits abstract intent.

1. Extend the relevant enum in `action_set.rs` (`MeleeActionSpec`, `RangedActionSpec`, `SpecialActionSpec`, or `MoveStyleSpec`).
2. Add a small spec struct when the variant needs timings/damage/reach/costs.
3. Implement helper methods used by consumers (`damage()`, `speed()`, `total_duration_s()`, etc.) if the enum already exposes them.
4. Add resolver tests proving `ActionSet::resolve` emits the expected `ActionRequest` when the frame asks for the verb.
5. Wire the actor/archetype/boss profile to the spec.
6. Add or extend an EFFECTS consumer if no current consumer handles the new spec.

Do not add separate telegraph specs. Telegraphs are the windup phase of the action spec unless a real system needs a separate concept.

## Adding or extending an EFFECTS consumer

Use this when an `ActorActionMessage` is already emitted and the missing part is the real-world effect.

1. Pick the focused request/spec to consume.
2. Add a Bevy system that reads `MessageReader<ActorActionMessage>`.
3. Filter by request kind and actor ownership/faction.
4. Look up the components/resources needed to spawn or start the effect.
5. Produce the existing effect shape when possible: hostile `Hitbox`, `EnemyProjectileSpawn`, `PlayerDamageEvent`, `DamageEvent`, boss-special state, VFX/SFX messages, etc.
6. Schedule after `emit_brain_action_messages` and before the downstream tick that should observe the effect.
7. Add a focused integration test in the consumer module.

Current scheduling examples are in `crates/ambition_sandbox/src/app/plugins.rs::register_combat_systems`.

### Consumer skeleton

Use this as a starting point when a new `ActionRequest` needs a focused effect consumer:

```rust
use bevy::prelude::*;

use crate::brain::{ActorActionMessage, ActionRequest};

pub fn spawn_example_from_brain_actions(
    mut messages: MessageReader<ActorActionMessage>,
    mut commands: Commands,
    actors: Query<&Transform>,
) {
    for msg in messages.read() {
        let ActionRequest::Special { spec } = msg.request else {
            continue;
        };

        let Ok(transform) = actors.get(msg.actor) else {
            continue;
        };

        // Filter to the concrete spec variant, faction, phase, or actor
        // component this consumer owns. Then spawn/start exactly one
        // effect shape.
        let _origin = transform.translation.truncate();
        commands.spawn(/* effect components */);
    }
}
```

Schedule the consumer in the set where the downstream effect should first be visible. For combat effects, current examples live in `register_combat_systems`: ranged and boss projectile consumers run before projectile ticking so spawned projectiles advance on the same frame; enemy melee start runs before hitbox damage application.

### Replacement discipline

When replacing an old direct path, avoid double-spawning by using an overlap-then-delete sequence:

1. Add the new consumer and targeted tests while the old path still exists.
2. Add a parity assertion or debug counter where practical.
3. Flip one concrete spec/archetype/profile to the new consumer.
4. Delete the old producer for that concrete case in the same patch.
5. Search for stale comments that still call the message stream “shadow,” “observed only,” or “next.”

## When not to add another consumer

Some behavior is really hit/damage metadata, not a new message stream. If the feature needs raw damage, final damage, stagger, elemental tags, hitstop, knockback, resource gain, VFX/SFX policy, or rejection reasons, prefer designing the canonical `HitSpec` / `HitInstance` / `HitResult` path instead of adding another parallel damage event.

## Common pitfalls

- **Brain swap must drag ActionSet with it.** If an entity changes from peaceful to hostile, update both policy and capability.
- **Early-return tick branches must write neutral.** Pre-poison tests should set an output frame to an action and verify dead/idle paths clear it.
- **The resolver returns multiple requests.** A frame can emit melee and ranged/special in one tick; consumers must filter, not assume exclusivity.
- **Runtime state is not always policy.** It is acceptable for a runtime component to own windup/active timers or spawn accumulators when those are integration state. The policy decision should still come from the brain/action path.
- **Player-specific exceptions should be named.** Projectile charging and pogo are still direct; do not accidentally duplicate them with a second brain consumer without disabling or reconciling the legacy path.
- **Use overlap-then-delete.** When replacing an old producer, add the new consumer with tests, verify parity, then remove the old producer for that variant.

## Validation gates

After every brain/action change:

```bash
cargo check -p ambition_engine
cargo check -p ambition_sandbox
cargo test -p ambition_sandbox --lib engine_core
cargo test -p ambition_sandbox --lib brain::
cargo test -p ambition_sandbox --lib content::features::ecs::brain_effects
cargo test -p ambition_sandbox --lib
cargo run -p ambition_sandbox --bin headless -- --ticks 30
```

If the full sandbox lib test hits EMFILE under high parallelism on shared dev VMs, run single-threaded:

```bash
cargo test -p ambition_sandbox --lib -- --test-threads=2
```
