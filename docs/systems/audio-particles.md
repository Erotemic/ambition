# Audio and particle notes

## Generated audio

The sandbox creates short sound effects at startup from symbolic synth recipes. Each recipe defines waveform, frequency sweep, duration, envelope, volume, and optional noise.

The code renders stereo frames directly into Kira `StaticSoundData` and registers those as `bevy_kira_audio` assets. Playback uses typed Kira channels: `SfxChannel` for one-shot feedback and `MusicChannel` for looping generated tracks, fades, and track switching. Bevy's built-in audio feature is intentionally not enabled for the sandbox.

This follows the same architectural shape as the older pygame prototype:

1. identify the event by a small sound id;
2. render or load a cached/generated frame representation;
3. play the resulting runtime sound object;
4. keep sound design as data, not imported assets.

For the current sandbox, the useful cues are jump, double jump, dash, slash, hit, pogo, reset, dummy death, and dummy respawn.

## Generated music

Music remains procedural/declarative in `crates/ambition_actors/assets/ambition/sandbox.ron`. The manifest now has `default_music_track` and a `music_tracks` list; each track has an id, display name, and arrangement body. The original 32-beat loop was ported as `original_lofi_loop`, and a longer 128-beat generated track is currently the default.

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

## Kira audio feature note

The sandbox depends on `bevy_kira_audio` with Kira's `wav` feature enabled because the crate's public re-exports expect a file-decoder error type behind Kira's Symphonia integration. Generated SFX and music do not write or check in audio files; they are still built in memory from data.
