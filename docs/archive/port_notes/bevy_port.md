# Bevy port notes

The sandbox shell has been ported to Bevy 0.18.1.

## What moved into Bevy

- Window creation and main loop now use `DefaultPlugins` and `WindowPlugin`.
- Static room blocks are spawned as colored `Sprite` entities.
- Player and dummies are Bevy sprite entities synchronized from `SandboxRuntime`.
- Particles, impact flashes, and slash previews are transient ECS entities.
- The debug HUD is Bevy UI text.
- Generated sound effects and music are Kira static sound assets played through typed `bevy_kira_audio` channels.

## What stayed backend-neutral

- `ambition_engine` still owns the deterministic platformer movement and collision core.
- `InputState` remains a compact semantic input packet, independent of keyboard/gamepad backends.
- Room geometry is still generated in code, not imported from maps or assets.

## Why keep the engine core separate?

The Bevy app is now the professional runtime shell, but Ambition Engine should not become tightly coupled to Bevy. Keeping movement/collision backend-neutral makes it easier to add deterministic tests, headless reachability checks, replay systems, and future renderer/audio experiments.

## Kira audio note

The visible Bevy app installs `bevy_kira_audio::AudioPlugin` and registers separate music and SFX channels. The built-in Bevy audio feature is not part of the sandbox's Bevy feature set. Generated audio is still synthesized at startup from RON data, but it now becomes Kira `StaticSoundData` instead of encoded WAV bytes.

## Camera framing note

The Tangent Space sandbox uses a fixed orthographic projection sized to the generated world. This keeps the whole single-room sandbox visible instead of clipping the floor or ceiling when the window size differs from the world dimensions.
