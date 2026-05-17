# Music generation balance notes

Status: goblin encounter transition polish pass, 2026-05-10.

## Symptom

Runtime transition work improved the intro -> wave1 handoff (`gain_start=target`),
but the level and texture still changed enough that the listener could tell two
rendered assets were being switched. The log showed the runtime compensating for
wave sections with a very large gain:

```text
state=intro ... gains=0.31,0.00,0.00,0.00,0.00,0.00
state=wave1 ... gains=3.83,0.00,0.00,0.00,0.00,0.00 gain_start=target
```

That is a content/mastering smell. A +12 dB runtime lift on a rendered OGG also
raises any SoundFont, reverb, codec, or normalized-tail noise by the same amount.

## Policy

For the current `first_goblin_tune_v2` full-mix approach:

- Generated full-section files should arrive at roughly matched perceived
  loudness.
- Runtime section gains should stay near unity.
- The music director should own timing/crossfade/de-click behavior, not loudness
  mastering.
- Long baked section fade-outs are usually counterproductive for adaptive
  music. Render section boundaries loop/cut-friendly with tiny de-click tails;
  let runtime crossfade decide the musical transition.

## Changes in this pass

- Reduced master and group reverb wetness / decay to reduce white-noise tails.
- Lowered limiter drive and peak target to leave more headroom.
- Added section-specific `postprocess.gain_db` to wave sections so wave1/2/3 are
  generated closer to encounter loudness without requiring +14 dB runtime gain.
- Reduced first-goblin runtime `wave_state_gain` from 5.0 to 1.0.
- Made legacy first-goblin runtime balance overrides a no-op when the cue is in
  full-mix mode (`layers == [full]`).

## Required workflow

This pass changes generation inputs. Re-render and reinstall the OGG assets:

```bash
./generate_audio_assets.sh --force
```

Then run the encounter room:

```bash
cargo run -p ambition_sandbox --bin ambition_sandbox --features dev_hot_reload --release -- --start-room mob_lab
```

Expected runtime log shape after re-render:

```text
state=intro ... gains=<near user-music-volume>,0,0,0,0,0
state=wave1 ... gains=<near user-music-volume>,0,0,0,0,0 gain_start=target
```

If the transition still feels like a new track appears, audit the rendered OGGs
rather than adding more runtime gain. The useful report is per-section integrated
LUFS/RMS/peak plus a short spectrogram/noise-floor check for intro.full.ogg and
wave1.full.ogg.

## 2026-05-10 follow-up: full-mix iteration path

The first goblin cue still exhibited an obvious section boundary after runtime
handoff fixes. The next iteration moved more responsibility into the generated
assets:

- intro full mix peaks lower than wave1 so it no longer dominates the handoff;
- master/section reverb is much drier to reduce broadband "white noise" tails;
- synthetic hat/noise-prone percussion is quieter and darker;
- `generate_audio_assets.sh` defaults to full-mix-only rendering, skipping
  per-stem OGGs because the Rust cue spec currently plays `*.full.ogg` files.

The renderer still uses temporary `.npy` scratch buffers between isolated worker
processes, but deletes them after successful renders unless
`--keep-debug-stems` is passed.
