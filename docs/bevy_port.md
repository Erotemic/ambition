# Bevy port notes

The sandbox shell has been ported to Bevy 0.18.1.

## What moved into Bevy

- Window creation and main loop now use `DefaultPlugins` and `WindowPlugin`.
- Static room blocks are spawned as colored `Sprite` entities.
- Player and dummies are Bevy sprite entities synchronized from `SandboxRuntime`.
- Particles, impact flashes, and slash previews are transient ECS entities.
- The debug HUD is Bevy UI text.
- Generated sound effects are `AudioSource` assets played by spawning `AudioPlayer` entities with `PlaybackSettings::DESPAWN`.

## What stayed backend-neutral

- `ambition_engine` still owns the deterministic platformer movement and collision core.
- `InputState` remains a compact semantic input packet, independent of keyboard/gamepad backends.
- Room geometry is still generated in code, not imported from maps or assets.

## Why keep the engine core separate?

The Bevy app is now the professional runtime shell, but Ambition Engine should not become tightly coupled to Bevy. Keeping movement/collision backend-neutral makes it easier to add deterministic tests, headless reachability checks, replay systems, and future renderer/audio experiments.

## Bevy audio format feature note

Generated sound effects are synthesized into in-memory WAV bytes and inserted as `AudioSource` assets. Bevy's audio decoder requires the matching Cargo feature for encoded formats, so the sandbox enables the `wav` feature on the `bevy` dependency. Without it, Bevy may panic with `UnrecognizedFormat` when an SFX is played.

## Camera framing note

The Tangent Space sandbox uses a fixed orthographic projection sized to the generated world. This keeps the whole single-room sandbox visible instead of clipping the floor or ceiling when the window size differs from the world dimensions.
