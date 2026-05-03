# Procedural Tune Authoring

Ambition sandbox music is authored as RON data and rendered procedurally at startup. A tune preview file should usually be a `MusicTrackSpec`:

```ron
(
    id: "new_tune",
    display_name: "New Tune",
    arrangement: (
        bpm: 76.0,
        total_beats: 32.0,
        root_hz: 220.0,
        bass_root_hz: 55.0,
        key_root_hz: 220.0,
        master_gain: 0.72,
        lowpass_alpha: 0.070,
        tape_hiss: 0.0020,
        gains: (chord_pad: 0.010, lead: 1.0, soft_keys: 0.016, bass: 0.047, drums: 0.78),
        lead: [
            (start: 4.0, duration: 1.2, semitone: 0, volume: 0.018),
            (start: 12.0, duration: 1.0, semitone: 7, volume: 0.016),
        ],
        chords: [
            (0, 3, 7, 10),
            (-5, -2, 2, 7),
            (-3, 0, 3, 7),
            (-7, -4, 0, 5),
            (0, 3, 7, 12),
            (-5, -2, 2, 7),
            (-8, -5, -1, 3),
            (-7, -4, 0, 5),
        ],
        bass_roots: [0, -5, -3, -7, 0, -5, -8, -7],
    ),
)
```

## Format

- `id`: stable machine name used by `default_music_track` and pause-menu switching.
- `display_name`: human-facing name shown in the pause menu.
- `bpm`: beats per minute. Current music likes `68.0` to `84.0`.
- `total_beats`: loop length in beats. Four beats make one bar, so `32.0` is 8 bars and `64.0` is 16 bars.
- `root_hz`: pitch root for lead and pad voices. `220.0` is A3.
- `bass_root_hz`: bass root. `55.0` is A1.
- `key_root_hz`: soft-key root. Usually match `root_hz`.
- `master_gain`: final output gain. Safe range: `0.55` to `0.82`.
- `lowpass_alpha`: warmth/brightness. Safe range: `0.045` to `0.100`; lower is darker.
- `tape_hiss`: procedural noise bed. Safe range: `0.0` to `0.004`.

## Notes And Harmony

All note pitches are semitone offsets from the relevant root. Examples: `0` is the root, `3` is a minor third, `7` is a fifth, `10` is a minor seventh, `12` is one octave up, and negative values go down.

- `lead`: sparse melody notes. Each note has `start` and `duration` in beats, a `semitone`, and a low `volume`. Good lead volumes are usually `0.012` to `0.030`.
- `chords`: one four-note chord per bar when you want full control. If the loop has more bars than chord entries, the renderer cycles the chord list.
- `bass_roots`: one bass root semitone per bar. Match its length to `chords` for predictable phrases.
- `gains.chord_pad`: pad level, usually `0.008` to `0.014`.
- `gains.lead`: multiplier for lead note volumes, usually `1.0`.
- `gains.soft_keys`: off-beat key stabs, usually `0.012` to `0.022`.
- `gains.bass`: bass level, usually `0.035` to `0.060`.
- `gains.drums`: dusty drum level, usually `0.60` to `1.0`.

To avoid monotonous loops, author 8 to 16 bars for short previews, vary the last two bars of each phrase, keep lead notes sparse, and make bass roots move across the whole phrase. For longer loops, prefer writing the full chord and bass phrase instead of relying on a tiny repeated set.

## Preview CLI

Render a tune to a WAV file:

```bash
cargo run -p ambition_sandbox --bin tune_preview -- \
    --track crates/ambition_sandbox/assets/ambition/tune_examples/example_drift.ron \
    --out /tmp/ambition_tune_preview.wav
```

Optional sample rate:

```bash
cargo run -p ambition_sandbox --bin tune_preview -- \
    --track crates/ambition_sandbox/assets/ambition/tune_examples/example_drift.ron \
    --out /tmp/ambition_tune_preview.wav \
    --sample-rate 44100
```

The CLI accepts either a full `MusicTrackSpec` or a bare `MusicSpec` arrangement. Full track specs are preferred because they can be pasted into `sandbox.ron`.

## Using A Tune In Game

To add a tune to the sandbox, paste the full `MusicTrackSpec` into `audio.music_tracks` in:

```text
crates/ambition_sandbox/assets/ambition/sandbox.ron
```

To make it the startup tune, change:

```ron
default_music_track: "new_tune",
```

The pause menu can switch between tracks already listed in `music_tracks`.
