---
status: current
last_verified: 2026-11-10
related_docs:
  - docs/mechanics/abilities.md
  - docs/systems/input-control-and-ui.md
---

# Projectiles and motion inputs

Motion-gesture recognition and projectile physics are independent reusable
systems. A game may connect a named gesture to a projectile technique without
putting device parsing in combat or projectile contact in the input layer.

## Motion input

`ambition_input` re-exports these reusable motion-input types:

- `MotionInputBuffer`: a bounded, time-windowed history of quantized
  `MotionDirection` samples. Repeated held directions are collapsed.
- `MotionTechnique`: one or more ordered direction patterns, optionally
  inverting the resulting facing.
- `MotionTechniqueCatalog` and `MotionTechniqueAppExt`: registration of
  game/provider-owned named techniques. The input crate defines no mandatory
  quarter-circle or half-circle action.

The current `detect_sequence` is an **ordered-subsequence** match: intermediate
samples are allowed. It returns a matched direction/facing; it does not spend
a resource, choose an animation, or spawn an attack. Content and the action
system decide what the recognized technique means.

## Projectile ownership

A projectile has an authoritative owner/source, physics and lifetime policy,
contact shape, collision filtering, hit payload, and an eventual despawn
reason. Projectile movement/contact uses shared physical-world semantics;
damage, knockback, and hurt acceptance use the combat authority. Visual trails
and sprites observe the result.

```text
semantic direction/action input
    -> named motion technique (when relevant)
    -> body action request and capability checks
    -> projectile spawn
    -> projectile simulation and contact
    -> combat hit resolution
    -> presentation
```

In particular, a failed geometric contact query is not a valid miss. The
projectile contact authority must preserve its refusal/recovery semantics.
See [collision geometry](../systems/collision-geometry-and-secondary-physics.md)
for physical material and portal crossings.

Use a focused gameplay scenario for the action under development. Add a
permanent test only when it captures an important distinct failure mode.
