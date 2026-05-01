# 0007: Use Avian2D for secondary physics while keeping the player controller custom

## Status

Accepted.

## Context

Ambition's core movement identity depends on a custom, highly tuned platformer controller. The player needs coyote time, buffered jumps/dashes, blink semantics, pogo refreshes, wall behavior, bullet-time aiming, and future mathematical movement operations that are easier to reason about as explicit kinematic gameplay code than as a fully dynamic rigid body.

At the same time, the sandbox is starting to need physical secondary motion: breakable platforms should throw shards, defeated enemies should leave ragdoll-like chunks, boss defeats should feel heavier, and later props or optional physics-player experiments should be possible without rewriting the engine.

## Decision

Use `avian2d` as the Bevy sandbox's secondary physics backend for dynamic props, debris, and ragdoll-like bodies.

Do not move the primary player controller to Avian. The player remains custom/kinematic by default.

Add a backend-neutral physics vocabulary to `ambition_engine` so game data can describe physical intent without depending directly on Avian component names. The sandbox maps that vocabulary and runtime events into Avian rigid bodies.

## Consequences

- Room solids can be mirrored as Avian static colliders for debris and props.
- Breakables, defeated enemies, and bosses can spawn dynamic debris/ragdoll pieces.
- The current player collision path remains unchanged.
- A future `PhysicsControlledPlayerPrototype` marker can be used for experimental alternate modes without changing the default controller.
- Spatial and coordinate conversion code around physics should be treated as review-sensitive because Ambition simulation space uses top-left +Y-down coordinates while Bevy/Avian uses centered +Y-up rendering/physics coordinates.

## Follow-ups

- Add richer data-driven physics specs to room objects when the first effect layer settles.
- Consider static colliders for intact breakable platforms, but only if they can be despawned when the breakable changes state.
- Consider joints and articulated bodies once enemies have more stable ECS state machines.
- Keep player physics optional and experimental until a physics-controlled mode proves it can preserve Ambition's movement feel.
