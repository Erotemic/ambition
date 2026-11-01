# Underwater audio

## Current shape

`AudioEnvironment` (`crates/ambition_platformer2d_audio/src/environment.rs`)
reads `WaterContact.submersion` and smooths a `wetness` value. Music receives
its channel gain. SFX publishes user and environment gain through `SfxMix`;
the shared voice mixer combines it with distance attenuation. A Kira SFX
channel-volume command would replace the per-voice gain, so do not use one.

The audible result is a **volume duck**, not a low-pass filter:

- music drops about 8 dB;
- SFX drop about 5 dB;
- the spectrum is unchanged.

Do not describe the current result as a muffle. A real muffle needs a tweenable
low-pass filter handle. The `bevy_kira_audio` wrapper does not expose Kira's
effect handles, so it needs a direct-Kira backend or an upstream seam.

## Invariants

- `AudioEnvironment` is ECS state; presentation and audio backends consume it.
- User mixer settings compose with environment wetness.
- Mute wins over environment effects.
- A track switch must not drop the environment effect chain.

## Validation

```bash
cargo test -p ambition_platformer2d_audio --lib environment
cargo test -p ambition_audio --features kira --lib one_shots_and_held_loops
```

For web audio behaviour, use
[`../recipes/web-audio-manual-test.md`](../recipes/web-audio-manual-test.md).
