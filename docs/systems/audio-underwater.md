# Underwater audio

## Current status

The sandbox has an ECS `AudioEnvironment` layer that reads `WaterContact.submersion`, smooths a `wetness` value, and writes environment mix changes to the music and SFX channels.

Current audible behavior is a **volume duck**, not a true underwater low-pass filter:

- music drops about 8 dB,
- SFX drop about 5 dB,
- the spectrum is unchanged.

Do not describe the current result as a real muffle. The plumbing is useful, but the backend effect is still incomplete.

## Desired backend

A real underwater muffle needs a tweenable low-pass filter that moves roughly between transparent high cutoff and muffled low cutoff while preserving user mixer settings.

The current `bevy_kira_audio` wrapper does not expose the Kira effect handles needed for that directly. Viable future routes are:

1. add a thin direct-Kira backend owned by Ambition,
2. use an upstream/forked `bevy_kira_audio` seam if one becomes available,
3. add a web-only WebAudio backend only if web audio becomes urgent enough to justify divergence.

## Invariants

- `AudioEnvironment` is ECS state; presentation/audio backends consume it.
- User mixer settings compose with environment wetness.
- Mute wins over environment effects.
- Track switches must not drop the environment effect chain once a real filter exists.

## Acceptance criteria for true underwater muffle

- The audio backend owns a real filter cutoff handle, not only channel volume.
- Wetness changes tween cutoff smoothly over a short interval.
- Music and SFX both respond.
- Browser/manual test confirms audible high-frequency reduction on submerge and recovery on surface.

## Validation

```bash
cargo test -p ambition_actors --lib audio
cargo test -p ambition_actors --lib water
```

For web audio behavior, also use [`../recipes/web-audio-manual-test.md`](../recipes/web-audio-manual-test.md).
