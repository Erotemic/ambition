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
- It does not make intact breakable platforms into Avian colliders yet; the player collision system already handles those through the ECS feature collision overlay.

## Fidelity tiers and mobile power

Reward chests currently use a cheap settle-until-first-contact helper instead of a continuously simulated rigid body. That means a falling chest stops on the first blocking object it reaches; if that blocker is a moving platform, the chest does not automatically resume falling when the platform moves away. Treat that as an intentional low-cost shortcut for now, not as the desired long-term model.

Future physics features should make fidelity explicit. A phone/battery-focused build may choose kinematic or one-shot settling for props that rarely need continuous simulation, while desktop/dev/physics-lab builds should be able to opt entities into full dynamic simulation so gravity, moving supports, force fields, and other systemic interactions can produce fun emergent behavior. Prefer data/build-feature/runtime-setting seams over one-off forks so the same authored entity can select between cheap settling and full physics depending on platform and mode.

## Next steps

1. Compile-test the Avian version/API against Bevy 0.18.
2. Tune debris lifetime, gravity, and impulses.
3. Add optional physics specs to room objects when the runtime behavior is stable.
4. Add joints for enemy/boss ragdolls after enemy state machines are less transient.
5. Add a separate physics-player prototype room if we want to compare custom vs dynamic player control.
