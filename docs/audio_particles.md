# Audio and particle notes

## Generated audio

The sandbox creates short sound effects at startup from symbolic synth recipes. Each recipe defines waveform, frequency sweep, duration, envelope, volume, and optional noise.

The code renders stereo 16-bit PCM into an in-memory WAV buffer and registers that buffer as a Bevy `AudioSource` asset. Playback uses `AudioPlayer` plus `PlaybackSettings::DESPAWN`, so one-shot sound entities clean themselves up after playback.

This follows the same architectural shape as the older pygame prototype:

1. identify the event by a small sound id;
2. render or load a cached/generated PCM/WAV representation;
3. play the resulting runtime sound object;
4. keep sound design as data, not imported assets.

For the current sandbox, the useful cues are jump, double jump, dash, slash, hit, pogo, reset, dummy death, and dummy respawn.

## Particle system

The current particle system is intentionally a tiny CPU-side ECS implementation. Each particle is an entity with position, velocity, age, lifetime, radius, color, gravity, drag, and kind. The renderer draws each particle as a small colored Bevy sprite.

This is enough for movement feedback and early feel testing. It is not meant to be the final visual effects architecture.

## Framework decision

The sandbox is now on Bevy 0.18.1. This gives Ambition an ECS-native place for particles, enemies, input resources, audio assets, renderer plugins, and future tool/debug systems.

A framework upgrade beyond Bevy is not necessary for small generated SFX or hundreds of simple particles. Reconsider the renderer/engine layer only if Ambition needs massive GPU particle fields, shader graphs, complex animation graphs, or custom procedural material pipelines.

## Future particle path

The next serious VFX step is to replace the current CPU sprite particle system with either:

- a Bevy GPU particle plugin, or
- a custom wgpu-backed particle renderer inside Ambition Engine.

## Bevy audio format feature note

Generated sound effects are synthesized into in-memory WAV bytes and inserted as `AudioSource` assets. Bevy's audio decoder requires the matching Cargo feature for encoded formats, so the sandbox enables the `wav` feature on the `bevy` dependency. Without it, Bevy may panic with `UnrecognizedFormat` when an SFX is played.
