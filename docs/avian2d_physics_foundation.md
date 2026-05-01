# Avian2D physics foundation

This patch introduces Avian2D as a secondary physics layer for Ambition.

The default player controller remains custom and kinematic. Avian is used for physical secondary bodies:

- static room colliders that dynamic debris can hit,
- breakable shards,
- defeated enemy ragdoll-like chunks,
- boss defeat debris,
- future props and optional physics-controlled player experiments.

## Current architecture

```text
ambition_engine::physics
  backend-neutral intent types:
  PhysicsBodySpec, PhysicsShape, PhysicsMaterial, RagdollSpec

ambition_sandbox::physics
  Avian2D plugin and runtime adapter:
  static colliders for room blocks,
  dynamic debris/ragdoll bursts,
  debris lifetime cleanup,
  PhysicsControlledPlayerPrototype marker
```

The sandbox maps Ambition's top-left +Y-down room coordinates into Bevy/Avian's centered +Y-up transform space through the existing `world_to_bevy` conversion. This is review-sensitive spatial code.

## What this does not do yet

- It does not make the player an Avian body.
- It does not replace Ambition's movement/collision engine.
- It does not give enemies fully articulated joint ragdolls yet.
- It does not add physics-authored RON fields yet.
- It does not make intact breakable platforms into Avian colliders yet; the player collision system already handles those via `FeatureRuntime`.

## Next steps

1. Compile-test the Avian version/API against Bevy 0.18.
2. Tune debris lifetime, gravity, and impulses.
3. Add optional physics specs to room objects when the runtime behavior is stable.
4. Add joints for enemy/boss ragdolls after enemy state machines are less transient.
5. Add a separate physics-player prototype room if we want to compare custom vs dynamic player control.
