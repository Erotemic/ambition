# Crate Strategy

Ambition should stay code-first, but it should not rebuild common infrastructure unless doing so is part of the game idea.

## Prefer crates for solved infrastructure

- `bevy_math`: vector math and bounding primitives. The former custom `Vec2` and custom AABB were useful early, but the engine now re-exports Bevy math primitives as `ambition_engine::Vec2` and `ambition_engine::Aabb`.
- `parry2d`: collision and geometry queries such as swept boxes, future raycasts, and spawn validation.
- `serde` + `ron`: room specs, ability presets, input bindings, tuning, and generated-content specs should be data rather than hand-authored Rust constructors.
- `fundsp`: procedural/generated audio synthesis.
- `kira` / `bevy_kira_audio`: future audio playback and mixing when we need fades, layered adaptive music, buses, effects, or precise clocks.
- Bevy ECS/plugins: keep rendering, camera, UI, audio playback, and windowing in Bevy systems rather than reimplementing a second engine.

## Keep bespoke code where it defines Ambition

- movement feel, blink rules, wall/climb/fly tuning;
- ability compatibility and unlock policy;
- deterministic simulation tests;
- story/world-state semantics;
- procedural content contracts that are specific to Ambition.

## Crate roles

`ambition_engine` should own reusable mechanics and can depend on Bevy math/geometry primitives. `ambition_sandbox` and future story crates should own content, data manifests, presentation, and input mappings.
