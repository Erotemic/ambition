# Procedural Ambience

The sandbox now starts a calm generated ambience loop at startup. It is synthesized into an in-memory WAV and played by Bevy audio, matching the current assetless direction.

This is intentionally a simple first pass. If Ambition's music becomes adaptive, layered, or timing-sensitive, move the audio layer behind an `ambition_audio` abstraction and use Kira for:

- cross-fading;
- buses/effects;
- smooth parameter automation;
- clocked musical events;
- layered room/state music.
