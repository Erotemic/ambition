# Bevy-native engine math and geometry

The engine is now allowed to depend on Bevy-adjacent crates when they provide battle-tested primitives that story crates should not have to reimplement. `ambition_engine` uses `bevy_math::Vec2` for vector math and `bevy_math::bounding::Aabb2d` as its public AABB representation, re-exported as `ambition_engine::Vec2` and `ambition_engine::Aabb`.

The engine no longer maintains a custom `Vec2d` or custom AABB data type. Any remaining helpers in `geometry.rs` are Ambition semantics layered on top of Bevy/Parry primitives: constructing an AABB from min+size, strict platformer overlap where edge-touching does not count as overlap, and Parry-backed swept-box time-of-impact queries.

`build_endgame_sandbox()` has also been removed from the public engine API. The playable sandbox now owns room layout through RON data, and engine tests define small purpose-built fixture worlds inline. This avoids a legacy hard-coded room becoming a hidden dependency for movement tests.

The intended layering is now:

```text
ambition_engine
  reusable Bevy-native mechanics, movement, collision semantics, abilities, combat, enemies, audio/music specs

ambition_actors / future story crates
  data manifests, presentation, Bevy app wiring, input bindings, story-specific content
```

This keeps reusable mechanics in one place while letting sandbox and story crates remain thin clients of the engine.
