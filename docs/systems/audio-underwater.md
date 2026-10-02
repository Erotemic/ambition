# Underwater audio

## Current shape

`AudioEnvironment` (`crates/ambition_platformer2d_actor_monolith/src/audio/environment.rs`)
reads `WaterContact.submersion`, smooths a `wetness` value, and writes
environment mix changes to the music and SFX channels.

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
cargo test -p ambition_platformer2d_actor_monolith --lib audio
cargo test -p ambition_platformer2d_actor_monolith --lib water
```

For web audio behaviour, use
[`../recipes/web-audio-manual-test.md`](../recipes/web-audio-manual-test.md).
