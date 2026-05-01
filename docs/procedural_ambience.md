# Procedural Retro Music

The sandbox now starts a generated SNES-style background loop at startup instead of the earlier drone/pad ambience. The track is synthesized into an in-memory WAV and played by Bevy audio, matching the assetless direction: no prerecorded files, no imported samples, and no external asset pipeline.

The current loop is intentionally simple and readable. It is built from four code-owned voices:

- a soft pulse-wave lead melody;
- a quiet pulse-wave arpeggio;
- a triangle bass line;
- low-volume procedural noise percussion.

The goal is calming retro energy rather than atmospheric horror. Keep future background music melodic, tonal, and low enough in the mix that movement SFX remain readable.

This is still a first pass. If Ambition's music becomes adaptive, layered, or timing-sensitive, move the audio layer behind an `ambition_audio` abstraction and use Kira for:

- cross-fading;
- buses/effects;
- smooth parameter automation;
- clocked musical events;
- layered room/state music.
