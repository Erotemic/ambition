# First Goblin Transition Lab

This workflow is for debugging the first-goblin intro -> wave1 transition
outside the game, without editing the active score or installing game assets.
It is intended for cases where the runtime transition already looks correct in
logs but the rendered sections still sound like two different tracks.

## Why this exists

The in-game cue currently plays full-mix section files for
`first_goblin_tune_v2`. Runtime changes can control scheduling, crossfades, and
bank gains, but they cannot make two separately rendered files feel continuous
if the score/mastering creates a loudness, spectrum, noise-floor, or phrase
boundary.

The transition lab lets us work independently from other agents by deriving a
separate experiment score from the current active score:

```text
tools/ambition_music_renderer/scores/active/first_goblin_tune_v2.music.yaml
    -> tools/ambition_music_renderer/scores/experiments/first_goblin_transition_lab.music.yaml
```

It renders to an isolated output directory:

```text
tools/ambition_music_renderer/generated/transition_lab/first_goblin_transition_lab/
```

It does **not** install into:

```text
crates/ambition_sandbox/assets/audio/music/generated/first_goblin_tune_v2/
```

## Quick start

```bash
cd ~/code/ambition
./tools/ambition_music_renderer/render_first_goblin_transition_lab.sh --force
```

The script:

1. reads the current active first-goblin score;
2. writes an experiment score with quieter/drier intro and darker/noise-reduced
   postprocess settings;
3. renders the experiment with `render_isolated.py`;
4. runs the existing peak/RMS cue audit;
5. runs `transition_audit.py` for intro -> wave1;
6. writes audition WAV previews.

Useful options:

```bash
# Parallel render workers
./tools/ambition_music_renderer/render_first_goblin_transition_lab.sh --force --jobs 4

# Try another backend
AMBITION_MUSIC_BACKEND=fluidsynth-cli \
  ./tools/ambition_music_renderer/render_first_goblin_transition_lab.sh --force

# Keep a hand-edited experiment score and only rerender it
./tools/ambition_music_renderer/render_first_goblin_transition_lab.sh \
  --force --keep-existing-score

# Clean experiment output first
./tools/ambition_music_renderer/render_first_goblin_transition_lab.sh \
  --clean --force
```

## Files to listen to

After rendering, listen to:

```text
tools/ambition_music_renderer/generated/transition_lab/first_goblin_transition_lab/transition_audit/intro_to_wave1_runtime_preview.wav
tools/ambition_music_renderer/generated/transition_lab/first_goblin_transition_lab/transition_audit/intro_to_wave1_level_matched_preview.wav
```

Interpretation:

- If the **level-matched** preview sounds smooth but the raw runtime preview is
  obvious, focus on generation/mastering loudness.
- If both previews still sound obvious, focus on arrangement/timbre continuity:
  intro cadence, wave1 first bar density, shared instruments, and reverb/noise
  floor.

## Transition audit metrics

`transition_audit.py` prints and writes:

```text
peak_db
rms_db
head_rms_db
tail_rms_db
tail_to_full_db
high_band_ratio
tail_high_band_ratio
```

The CSV lands here:

```text
generated/transition_lab/first_goblin_transition_lab/transition_audit/intro_to_wave1_metrics.csv
```

The high-band ratio is a simple FFT-based proxy for hiss/air/noise. It is not a
proper LUFS or psychoacoustic noise metric, but it is useful for quickly seeing
whether an intro tail has a lot more high-frequency energy than wave1.

## What the experiment score changes

The generated experiment score is intentionally conservative:

- lowers intro gain / peak target;
- shortens and dries intro reverb;
- darkens winds, percussion, mallets, choir pad, and high-frequency shelves;
- reduces hat/crash velocities in later templates;
- makes wave1 enter a bit more assertively on bar 1;
- adds a small low-goblin pickup near the end of the intro.

The goal is not to produce final music in one pass. The goal is to create a
safe, isolated loop for testing hypotheses about why intro -> wave1 feels like a
track boundary while wave2 -> wave3 feels natural.

## About `.npy` files

`render_isolated.py` uses per-group `.npy` stem buffers as scratch data while it
assembles full mixes, adaptive sections, and previews. These scratch buffers are
not installed into the game and should not be checked in. The current renderer
removes `debug_stems/*.npy` after a successful render.

Skipping `.npy` generation entirely would require a different render path. For
adaptive cues, the renderer still needs intermediate group audio to assemble
full-section files and diagnostics. The practical speed wins for now are:

- use `--jobs N`;
- render the isolated transition-lab score rather than all active scores;
- avoid installing/regenerating unrelated cues;
- only keep debug stems when adding an explicit renderer option for that.

## Promotion path

When the experiment sounds better:

1. compare the experiment YAML against the active YAML;
2. manually port only the accepted musical/postprocess changes into
   `scores/active/first_goblin_tune_v2.music.yaml`;
3. regenerate the real cue with `./regen_music.sh --force`;
4. test in `mob_lab`;
5. check in/IPFS-track accepted installed audio assets separately.

Do not point the game directly at `first_goblin_transition_lab` unless we add a
separate Rust cue spec for that experiment.

## Visual transition audit

`transition_audit.py` now writes visual reports in addition to WAV previews. After
running the lab script, inspect:

```text
tools/ambition_music_renderer/generated/transition_lab/first_goblin_transition_lab/transition_audit/intro_to_wave1_report.md
tools/ambition_music_renderer/generated/transition_lab/first_goblin_transition_lab/transition_audit/intro_to_wave1_tail_head_envelope.png
tools/ambition_music_renderer/generated/transition_lab/first_goblin_transition_lab/transition_audit/intro_to_wave1_runtime_envelope.png
tools/ambition_music_renderer/generated/transition_lab/first_goblin_transition_lab/transition_audit/intro_to_wave1_runtime_spectrogram.png
```

The plots are intended to answer questions that are hard to judge by ear:

- **Tail/head envelope**: compares the last few seconds of the intro to the
  first few seconds of wave1. A large jump at zero means the rendered files are
  not level-matched at the boundary.
- **Runtime envelope**: shows the RMS and peak envelope of the runtime-style
  crossfade preview, with the crossfade window shaded.
- **Runtime spectrogram**: shows whether the seam has a spectral cliff, e.g. a
  noisy/reverby high-frequency intro tail followed by a darker wave1, or the
  reverse.
- **Markdown report**: summarizes head/tail RMS deltas and embeds the plots.

Useful direct command:

```bash
python tools/ambition_music_renderer/transition_audit.py \
  tools/ambition_music_renderer/generated/transition_lab/first_goblin_transition_lab \
  --sections intro wave1
```

Skip plots if matplotlib is not available or if you only need the CSV/WAVs:

```bash
python tools/ambition_music_renderer/transition_audit.py \
  tools/ambition_music_renderer/generated/transition_lab/first_goblin_transition_lab \
  --sections intro wave1 \
  --no-plots
```

Interpretation rule of thumb:

```text
level-matched preview smooth + runtime preview rough => gain/mastering issue
both previews rough + spectrogram cliff => arrangement/timbre/noise-floor issue
tail/head envelope jump at 0s => section boundary level mismatch
high-frequency tail visible before seam => reverb/hiss/noise tail problem
```

## Iteration: reduce intro-tail dip and add component plots

After the first visual audit, the key measurement was that `wave1` head RMS was
about 2.9 dB lower than the `intro` tail. The transition-lab generator now moves
closer by:

- replacing the intro's rising `expression_gentle` automation with a held/slight
  fade-down automation, so the last seconds of the intro no longer crest into
  the boundary;
- lowering and drying the intro's tail-oriented postprocess;
- moving the last low-goblin pickup earlier in the last bar and reducing its
  velocity;
- adding a stronger but still controlled wave1 downbeat anchor;
- adding a low-goblin echo on wave1 beat 1 so wave1 shares material with the
  intro instead of feeling like an unrelated track;
- slightly raising wave1's first-bar energy while keeping the overall loop from
  becoming much louder.

The audit now also writes:

```text
transition_audit/intro_to_wave1_component_envelope.png
```

This plot separates the outgoing intro contribution, incoming wave1 contribution,
and summed runtime preview. If the summed line dips while both components look
reasonable, tune the crossfade/gain law. If the incoming wave1 component is low
or sparse before the crossfade ends, tune the score/generator.


## Iteration 3 notes

The second visual audit moved the source tail/head level delta close to target
(`wave1 head - intro tail` around -0.9 dB), but the component plot showed a
deep summed-RMS dip inside the 0.35s crossfade. That means the average source
levels are now close, but wave1's first few hundred milliseconds still do not
enter quickly enough to support the fade-out of the intro.

This iteration tests two isolated hypotheses before changing the active game
cue:

- Make the intro tail recede more intentionally by removing the late bar-3
  low-goblin restart and adding a stronger expression fade-down.
- Front-load wave1 with low, non-hissy energy on beat 1: bass drum/floor tom,
  low-goblin echo, tuba root, timpani root, and stronger low-string pulse.

The wrapper now audits with a 0.65s transition preview by default. This is a
lab-only preview setting. If the visualization improves, port the same idea to
the runtime intro-to-loop crossfade constant later; otherwise keep tuning the
source content.

Target metrics:

```text
wave1 head - intro tail: roughly -1 dB to +1 dB
crossfade dip vs pre-window median: no worse than about -3 dB
component plot: incoming wave1 should be visibly present before the outgoing intro reaches silence
```
