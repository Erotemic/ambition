# Ambition Engine architecture

Ambition Engine is not trying to replace Bevy. Bevy is the host engine for the
current executable: windowing, rendering, input plumbing, audio playback, ECS,
scheduling, and debug drawing.

Ambition Engine is the backend-neutral simulation crate. It should remain useful
without a window. The ideal shape is:

```text
input frame + world state -> ambition_engine step -> simulation events
simulation events + state -> Bevy adapter -> visuals/audio/debug UI
```

## Module map

`crates/ambition_engine/src/` is now split by responsibility:

- `lib.rs` — public crate surface and re-exports.
- `math.rs` — small renderer-independent `Vec2` plus `approach()` easing.
- `geometry.rs` — `Aabb` collision primitive.
- `world.rs` — generated room blocks, block kinds, and `build_endgame_sandbox()`.
- `movement.rs` — `Player`, `InputState`, movement tuning, combo trace, and player stepping.
- `enemy.rs` — sandbox dummy target simulation: HP, stun, knockback, death, respawn.
- `music.rs` — symbolic music placeholders for future generated music work.

The Bevy sandbox can still import convenience names from the crate root, for
example `ambition_engine::Player` or `ambition_engine::spawn_dummies`, because
`lib.rs` re-exports the main public types.

## What moved from the sandbox into the engine

The dummy/enemy test fixtures moved from `ambition_sandbox` into
`ambition_engine::enemy`.

That logic belongs in the engine because it is simulation, not presentation:

- whether a dummy is alive;
- how much HP it has;
- hit stun duration;
- knockback velocity;
- death and respawn timers;
- how dummy gravity/friction update.

The Bevy layer should only decide how those dummies look, which sounds/effects to
play when the engine state changes, and how to draw their debug overlays.

## What should move next

The next good candidates for engine ownership are:

1. `AttackSpec` and slash hitbox generation.
   - Today, the sandbox hard-codes slash dimensions.
   - Better: engine defines attacks and emits hit events; Bevy renders previews.

2. Room/spec data structures.
   - Today, `build_endgame_sandbox()` directly pushes blocks.
   - Better: define `RoomSpec`, `BlockSpec`, and maybe procedural room builders.

3. Story/world-state events.
   - The engine should not know the plot, but it can own generic event logs:
     `AbilityUnlocked`, `RouteOpened`, `WorldTransformApplied`, etc.

4. Replay inputs.
   - Because `InputState` is backend-neutral, we can record and replay movement
     without Bevy once the simulation state is fully represented in the engine.

## What should stay in the sandbox

Keep these Bevy-side for now:

- keyboard/gamepad keycodes and presets;
- Bevy sprites, gizmos, UI text, and cameras;
- generated WAV playback details;
- CPU particle entities;
- debug overlay drawing;
- window size and world-to-Bevy coordinate conversion.

## Commenting style

Engine comments should explain why a mechanic exists and what assumptions it
relies on. Avoid comments that simply restate a line of code. Good examples:

- why coyote time exists;
- why `Vec2` is not Bevy's vector type;
- why AABB collision is sufficient for the first sandbox;
- why player movement is kinematic rather than rigid-body physics;
- why dummy logic lives in the engine rather than Bevy.

## Ability and testability update

The engine now owns `AbilitySet`, which makes movement verbs explicit and
optional. The sandbox still enables everything by default, but future tests,
story states, and generated challenge rooms can run with reduced or unusual
ability sets.

The engine also owns `combat::slash_hitbox`. Bevy still renders the slash preview
and particles, but hitbox shape is now testable without a window.

This is the intended direction: if a behavior affects simulation, reachability,
combat, resources, or deterministic replay, prefer moving it to
`ambition_engine`. If a behavior only affects presentation, keep it in the Bevy
sandbox adapter.

## Enemy collision note

Dummies now use engine-side room collision via `Dummy::update_in_world`; see `docs/enemy_collision.md`.
