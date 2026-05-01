# Engine and Bevy boundary

The boundary has changed: `ambition_engine` is no longer trying to be a renderer-neutral core. It is now the reusable Bevy-native mechanics crate for Ambition, while sandbox/story crates are thin content and presentation crates.

That means the engine may depend on Bevy-adjacent crates when they replace bespoke infrastructure:

- `bevy_math::Vec2` replaces the old custom vector type.
- `bevy_math::bounding::Aabb2d` replaces the old custom AABB data type.
- `parry2d` handles low-level geometry queries and swept collision.

The important separation is not “no Bevy in the engine.” The important separation is:

```text
ambition_engine
  movement, collision semantics, abilities, combat, enemies, reusable mechanics

ambition_sandbox / story crates
  room/story data, Bevy app setup, rendering, input bindings, debug UI, presentation
```

This lets future stories assemble engine features without copying details out of the sandbox.
