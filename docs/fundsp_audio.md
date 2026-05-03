# FunDSP audio rendering

Ambition uses FunDSP for generated-audio synthesis and `bevy_kira_audio` for visible playback.

The current pipeline is:

```text
RON audio specs
  -> FunDSP-backed startup rendering
  -> in-memory stereo frame buffers
  -> Kira StaticSoundData assets
  -> typed Kira music/SFX channels
```

The renderer stays data-driven and testable without an audio device. Kira owns runtime playback, channel-level fades, looping, and track switching.

## What FunDSP owns now

`crates/ambition_sandbox/src/audio.rs` uses FunDSP for:

- bandlimited one-shot SFX oscillators: sine, square, triangle, and soft saw;
- SFX white noise and low-pole filtering;
- lo-fi music low-pole filtering;
- pink-noise tape hiss;
- white-noise drum dust;
- waveform helpers for procedural notes;
- smooth envelope shaping and soft clipping helpers.

## What stays data-driven

The following values are still authored in `crates/ambition_sandbox/assets/ambition/sandbox.ron`:

- sample rate;
- SFX cue frequencies, envelopes, durations, gains, noise amounts, and waveforms;
- default music track id;
- lo-fi track ids/display names, BPM, loop length, note pattern, chords, bass roots, gains, low-pass warmth, and tape hiss.

FunDSP is the renderer; RON remains the authoring layer.

## Why not live FunDSP graphs?

Live graphs are attractive for adaptive music, bullet-time pitch/filtering, room-specific ambience, and layered theorem-field effects. The current path deliberately renders static procedural tracks at startup, then lets Kira handle playback. A future audio patch can add live graph scheduling or adaptive stems if the game design needs it.

## Compatibility note

Use `fundsp::prelude` as the sandbox DSP namespace. Some FunDSP docs and examples refer to `hacker32` / `hacker64` style preludes, but the crate version resolved by Cargo for this workspace exposes the stable prelude namespace instead. Keeping the import on `fundsp::prelude` avoids compile churn while still using FunDSP oscillators, filters, noise sources, and math helpers for startup WAV generation.

The stable prelude keeps oscillator and pink-noise constructors generic over sample type, so the sandbox explicitly asks for `f32` nodes such as `dsp::sine::<f32>()` and `dsp::pink::<f32>()`. The rest of the renderer is already `f32`, so making the sample type explicit avoids inference failures without changing the audio path.

Compatibility note: In FunDSP 0.23, `sine` and `pink` are generic and should be called with `::<f32>()`, while `square`, `triangle`, and `soft_saw` are not generic and should be called without type parameters.
