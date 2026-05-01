# Crate Strategy

Ambition should stay code-first, but it should not rebuild common infrastructure unless doing so is part of the game idea.

## Prefer crates for solved infrastructure

- `glam` / `bevy_math`: vector and matrix math. The current custom `Vec2` was useful early, but it should be migrated to `glam::Vec2` or `bevy_math::Vec2` once we do a deliberate math pass. This keeps the core close to Bevy without depending on all of Bevy.
- `serde` + `ron` or `toml`: future room specs, ability presets, input bindings, and generated content should become data rather than hand-authored Rust constructors.
- `kira`: future audio should use a proper game-audio layer when we need fades, layered adaptive music, buses, effects, or precise clocks. The current Bevy audio path is fine for simple generated WAV playback.
- Bevy ECS/plugins: keep rendering, camera, UI, audio playback, and windowing in Bevy systems rather than reimplementing a second engine.

## Keep bespoke code where it defines Ambition

- movement feel, blink rules, wall/climb/fly tuning;
- ability compatibility and unlock policy;
- deterministic simulation tests;
- story/world-state semantics;
- procedural content contracts that are specific to Ambition.

## Near-term refactor target

The highest-value next data-driven migration is room authoring:

```text
RoomSpec
  size
  blocks[]
  loading_zones[]
  enemy_spawns[]
  moving_platforms[]
  ambience_id
```

Once that exists, generated rooms and hand-authored test rooms can use the same validation pipeline.
