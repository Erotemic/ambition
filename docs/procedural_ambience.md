# Procedural Lo-Fi Music

The sandbox starts a generated lo-fi music track at startup instead of the earlier drone/pad ambience or the brighter SNES-style pass. Tracks are synthesized into in-memory Kira static sound data and played through `bevy_kira_audio`, matching the assetless direction: no prerecorded files, no imported samples, and no external asset pipeline.

The current loop is intentionally low-key and unintrusive. It is built from code-owned voices:

- a slow warm chord pad;
- a sparse lower-register triangle melody;
- lazy off-beat soft-key stabs;
- a simple subby sine bass;
- dusty, low-volume kick/snare/hat noise;
- a tiny amount of procedural tape hiss plus low-pass filtering and soft clipping.

The goal is closer to "lo-fi beats to study or relax to" than arcade energy. Keep future background music melodic, tonal, low in the mix, and gentle enough that movement SFX remain readable. If a future patch adds bus control, the music volume should be user-tunable separately from SFX.

The manifest now supports multiple generated tracks via `default_music_track` and `music_tracks`. The old 32-beat loop is still available as data, and the current default track is longer and less repetitive for sandbox iteration.

For a compact tune-format guide and WAV preview CLI workflow, see `docs/procedural_tune_authoring.md`.

If Ambition's music becomes adaptive, layered, or timing-sensitive, move the audio layer behind an `ambition_audio` abstraction and extend the current Kira path with:

- cross-fading;
- buses/effects;
- smooth parameter automation;
- clocked musical events;
- layered room/state music.

## FunDSP renderer note

The arrangements are still authored as compact RON data, and the startup renderer uses FunDSP nodes and helpers for oscillators, filters, noise, envelopes, and soft clipping. Kira receives generated stereo frame buffers directly, so playback no longer depends on Bevy's built-in audio or encoded WAV buffers.
