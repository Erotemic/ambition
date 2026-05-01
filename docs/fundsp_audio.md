# FunDSP audio rendering

Ambition still uses Bevy's built-in audio playback for now, but the generated-audio renderer now depends on FunDSP for synthesis and filtering primitives.

The current pipeline is:

```text
RON audio specs
  -> FunDSP-backed startup rendering
  -> in-memory WAV bytes
  -> Bevy AudioSource handles
  -> AudioPlayer entities
```

This is intentionally conservative. It avoids introducing a second runtime audio plugin while replacing the most fragile handmade DSP code with a dedicated audio library.

## What FunDSP owns now

`crates/ambition_sandbox/src/audio.rs` uses FunDSP for:

- bandlimited one-shot SFX oscillators: sine, square, triangle, and soft saw;
- SFX white noise and low-pole filtering;
- lo-fi music low-pole filtering;
- pink-noise tape hiss;
- white-noise drum dust;
- waveform helpers for procedural notes;
- smooth envelope shaping and soft clipping helpers.

The sandbox still writes a tiny WAV header itself because Bevy's built-in `AudioSource` path already accepts WAV bytes, and keeping that boundary stable makes this patch much lower risk.

## What stays data-driven

The following values are still authored in `crates/ambition_sandbox/assets/ambition/sandbox.ron`:

- sample rate;
- SFX cue frequencies, envelopes, durations, gains, noise amounts, and waveforms;
- lo-fi BPM, loop length, note pattern, chords, bass roots, gains, low-pass warmth, and tape hiss.

FunDSP is the renderer; RON remains the authoring layer.

## Why not live FunDSP graphs yet?

Live graphs are attractive for adaptive music, bullet-time pitch/filtering, room-specific ambience, and layered theorem-field effects. The current goal is smaller: improve the generated asset renderer without changing gameplay, scheduling, or Bevy audio playback. A future audio patch can route these generated sounds through Kira or another Bevy audio backend for buses, fades, pitch, and room transitions.

## Compatibility note

Use `fundsp::prelude` as the sandbox DSP namespace. Some FunDSP docs and examples refer to `hacker32` / `hacker64` style preludes, but the crate version resolved by Cargo for this workspace exposes the stable prelude namespace instead. Keeping the import on `fundsp::prelude` avoids compile churn while still using FunDSP oscillators, filters, noise sources, and math helpers for startup WAV generation.

The stable prelude keeps oscillator and pink-noise constructors generic over sample type, so the sandbox explicitly asks for `f32` nodes such as `dsp::sine::<f32>()` and `dsp::pink::<f32>()`. The rest of the renderer is already `f32`, so making the sample type explicit avoids inference failures without changing the audio path.

Compatibility note: In FunDSP 0.23, `sine` and `pink` are generic and should be called with `::<f32>()`, while `square`, `triangle`, and `soft_saw` are not generic and should be called without type parameters.
