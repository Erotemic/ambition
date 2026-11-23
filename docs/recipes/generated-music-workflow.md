---
status: current
last_verified: 2026-11-10
related_docs:
  - docs/concepts/generated-assets-audio.md
  - docs/tools/generated-audio-tools.md
  - docs/systems/audio-and-vfx.md
---

# Generated music workflow

Music source specifications and renderer code are the source of truth. Rendered
files are review/runtime artifacts produced by an explicit render or publish
step; they are not hand-edited.

## Inspect the current CLI

```bash
uv run --project tools/ambition_music_renderer \
  python -m ambition_music_renderer --help
uv run --project tools/ambition_music_renderer \
  python -m ambition_music_renderer cue --help
```

Prefer CLI help and the tool-local README over old copied flag lists.

## Inspect native playback policy

```bash
cargo run -p ambition_content_check --bin ambition_content_check -- \
  game/ambition_content/assets --inspect-music --advisory-assets
```

The JSON joins expanded game policy with the selected producer identity,
products, clocks, section ends, gains, exit wait bounds, and required voices.
A null wait bound needs decode facts or has no exit. Musical bounds exclude
preparation and frame observation latency. Inspection does not prove native
voice admission or speaker output. The native owner checks decoded layer
rates and frame counts before it resumes a group. Each voice retains its
source facts through asset reload.

A whole-asset section uses `AssetEnd` without a score clock. For example,
a cue file can declare this loop:

```ron
(
    cues: [(
        id: "ambient",
        media: Assets(sections: [(
            id: "song", looped: true,
            sources: [(layer_id: "full", path: "audio/ambient.ogg")],
        )]),
        relative_volume: 1.0,
        layers: [(id: "full", slot: 0)],
        states: [(id: "song", section_id: "song", gains: [(layer_id: "full", gain: 1.0)])],
    )],
)
```

Set `looped: false` for a one-shot. An intro state can name a loop state
through `next_state`. An ending uses `outro_state` and the existing scoped
completion binding. Layered states can name the same section with different
gains. All these forms use the same bounded playback owner.

An optional `playback: (...)` sets overrides, such as
`playback: (section_crossfade_seconds: Some(0.4))`. Omitted fields retain the
recipe defaults. `section_exit` and
`ending_exit` accept `Some(Immediate)`, `Some(AssetEnd)`, `Some(MusicalEnd)`,
`Some(Bars(n))`, or
`Some(Markers(["exit_id", ...]))`. Preparation checks exit references and clock
support. Whole assets default to immediate state changes. Clocked programs
default to one-bar changes and two-bar ending exits.

The policy also sets section, ending, and continuation fades; return overlap;
and the ending-media timeout. `exhausted: Some(Finish)` stops automatic late entry
after a one-shot reaches its file end. The default is `StartReadyTarget`.
`FrameDriven` timing is supported. `SampleAccurate` and `PhasePreserving`
requirements fail preparation because the current native adapter cannot
promise them. Pickup scheduling and independent tail release are not admitted
by this policy surface.

## One-cue loop

```bash
CUE=<cue_id>

uv run --project tools/ambition_music_renderer \
  python -m ambition_music_renderer cue bundle "$CUE" \
  --backend=pretty-midi --force --zip_report
```

A report bundle is the normal review handoff: source/manifest, numeric reports,
spectral/dissonance diagnostics, and compact plots without large audio binaries.
Use a full bundle when the reviewer needs audio. Add the current explicit
publish option only when the cue should update provider runtime assets.

## Diagnose before changing runtime

1. Render only the affected cue.
2. Inspect arrangement/dissonance/stem-balance reports.
3. Listen to adjacent sections outside the game.
4. Reproduce through the provider room/encounter and capture semantic music
   director logs.
5. Change runtime transition policy only if the generated sources are already
   coherent and logs show a state/queue/gain error.

Do not use runtime gain to hide an under-authored source, clipped master, noisy
stem, or bad phrase boundary.

## Provider registration

Named cue IDs and encounter/room bindings belong to provider assets/registries.
The reusable audio crate owns App-local catalogs, state selection, loading,
mixing, web unlock, and output. Simulation emits semantic music/SFX intent; it
must not know filenames or renderer internals.

## Check the result

For a cue edit, render and listen to the affected sections, inspect its bundle
report, and hear it in the owning room or encounter. Use the level audit when
cue bindings or coverage changed:

```bash
uv run --project tools/ambition_music_renderer \
  python -m ambition_music_renderer audit levels --check
```

Run the renderer's tests when its code or format changes. Run focused audio or
content tests when their loading, registration, or transition contracts change.
Use a browser check only for web audio/device changes.

### Native software capture on Linux

Use the opt-in device test to check the locked Kira mixer and the native music
executor. The test delays decode beyond the intro, selects the current phase,
then checks two fresh cue handoffs and an explicit stop. It stalls the host
for one second across a prepared join. The source has a half-second tail;
the native clock must silence it and start the next section at the prepared end.
The test also replaces a source after submission and checks final native resource
release.

```bash
CARGO_INCREMENTAL=0 cargo test -p ambition_audio --features kira,content_pack --lib --no-run
python3 scripts/measure/capture_native_audio.py \
  --out .cache/music-native-capture/run.raw --seconds 30 -- \
  cargo test -p ambition_audio --features kira,content_pack --lib \
  native_output_survives_cold_decode_and_repeated_fresh_handoffs -- --ignored --nocapture
```

Select a new output path for each run. The command uses a temporary ALSA
configuration for its child process. A pipe reader limits the null device to
48 kHz stereo float output and caps the capture duration. It writes raw
`FLOAT_LE` PCM and a JSON summary. It changes no system audio configuration.
Listen to the PCM or inspect its spectrum to check the two source tones per
section. Backend admission and a nonzero file alone do not prove continuity.
This capture does not verify physical speakers or the complete game route.

The opt-in `native_canceling_a_prepared_entry_cancels_its_predecessors_stop`
test checks cancellation. Run it in a separate capture. The old source must
continue past its withdrawn exit while the replacement is not ready, including
a host stall immediately after withdrawal. Check section levels as well as
frequency content; a nearly silent source can still produce the expected tones.
Native starts and fades use Kira's processing resolution. The preparer still
rejects `SampleAccurate` and `PhasePreserving` requests.

The opt-in `a_native_loop_exit_runs_after_several_passes_during_a_host_stall`
test checks a prepared loop exit. Run it in a separate capture. The target tones
must replace the source tones during the host stall, after several source loop
passes. A missed admission deadline must select another allowed exit instead of
adding a delay to the selected boundary.

The opt-in `a_native_short_continuation_finishes_once_between_host_observations`
test admits a 5 ms continuation, then stalls across its full lifetime. Its
completion must advance the cue once without retrying the finished audio.
