# Procedural Lo-Fi Music

The sandbox starts a generated lo-fi background loop at startup instead of the earlier drone/pad ambience or the brighter SNES-style pass. The track is synthesized into an in-memory WAV and played by Bevy audio, matching the assetless direction: no prerecorded files, no imported samples, and no external asset pipeline.

The current loop is intentionally low-key and unintrusive. It is built from code-owned voices:

- a slow warm chord pad;
- a sparse lower-register triangle melody;
- lazy off-beat soft-key stabs;
- a simple subby sine bass;
- dusty, low-volume kick/snare/hat noise;
- a tiny amount of procedural tape hiss plus low-pass filtering and soft clipping.

The goal is closer to "lo-fi beats to study or relax to" than arcade energy. Keep future background music melodic, tonal, low in the mix, and gentle enough that movement SFX remain readable. If a future patch adds bus control, the music volume should be user-tunable separately from SFX.

This is still a first pass. If Ambition's music becomes adaptive, layered, or timing-sensitive, move the audio layer behind an `ambition_audio` abstraction and use Kira for:

- cross-fading;
- buses/effects;
- smooth parameter automation;
- clocked musical events;
- layered room/state music.
