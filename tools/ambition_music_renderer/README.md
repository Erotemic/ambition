# Ambition Music Renderer

Code-only Python tool that renders Ambition MusicIR YAML scores into adaptive
OGG stems and a set of preview mixes.

This package is the canonical audio generator for the project. **No rendered
`.ogg`, `.wav`, or `.mid` is committed.** Generate assets locally under
`generated/<cue>/`, then publish explicitly into the bevy asset tree.

## Modal CLI

```
python -m ambition_music_renderer render <cue>
python -m ambition_music_renderer publish <cue>
python -m ambition_music_renderer render-publish <cue>

# sandbox cues (lofi_study_loop, long_lofi_drift, pulse_drift_voyage):
python -m ambition_music_renderer sandbox render-publish
python -m ambition_music_renderer sandbox render-publish --cue lofi_study_loop
python -m ambition_music_renderer sandbox publish --skip-render
```

`<cue>` resolves against `scores/active/`, `scores/examples/`, then
`scores/archive/`. You can also pass an explicit YAML path.

`render-publish` skips the render step when the YAML mtime is older than the
latest preview. Pass `--force-render` to re-render.

`publish` copies the newest hashed `<cue>_<hash>.full_soundtrack_preview.ogg`
to `crates/ambition_sandbox/assets/audio/music/generated/<cue>/full.ogg`.

## Installing dependencies

One-shot setup:

```bash
cd tools/ambition_music_renderer
./setup.sh                # apt + pip dependencies (FluidSynth, SoundFonts, etc.)
uv venv .venv             # if not already created
source .venv/bin/activate
```

`setup.sh` installs FluidSynth, several General MIDI SoundFonts (MuseScore
General, FluidR3, TimGM6mb), `ffmpeg`, and the Python deps `numpy scipy
pretty_midi PyYAML soundfile pyfluidsynth`.

## Backends

Three rendering paths, in order of audio quality:

| Backend | What it is | When to use |
|---|---|---|
| `pretty-midi` | pyFluidSynth + SoundFont, internal reverb/chorus disabled | **Default for production cues.** |
| `fluidsynth-cli` | the `fluidsynth` binary + SoundFont, `-R 0 -C 0` | When pyFluidSynth has trouble. |
| `fallback` | hand-rolled additive synth, isolated in `fallback_backend.py` | When no SoundFont is available — sounds noticeably synthetic. |
| `auto` | tries `fluidsynth-cli` first, falls back to `fallback` | CI / portable contexts. |

The `fallback` backend is genuinely the *fallback* — it doesn't sound like
real instruments, just like a competent additive synth.

## SoundFont selection

`DEFAULT_SOUNDFONTS` in `musicir_renderer.py` is the preference list,
highest-quality first:

1. `/usr/share/sounds/sf3/MuseScore_General_Full.sf3` (recommended; ~140 MB)
2. `/usr/share/sounds/sf3/MuseScore_General.sf3`
3. `/usr/share/sounds/sf2/FluidR3_GM.sf2`
4. `/usr/share/sounds/sf2/TimGM6mb.sf2`
5. system default

Override per-cue with `render.soundfont` in the YAML, or per-invocation with
`--soundfont /path/to/file.sf2`.

## Rendering a cue

The repo-root convenience script:

```bash
./generate_audio_assets.sh
# AMBITION_MUSIC_BACKEND=fallback ./generate_audio_assets.sh   # overrideable
```

Renders `first_goblin_tune_v2`, runs the loudness audit, and installs the
hash-stripped filenames into the Bevy asset tree under
`crates/ambition_sandbox/assets/audio/music/generated/first_goblin_tune_v2/`.

For the three sandbox single-track cues (lofi_study_loop / long_lofi_drift
/ pulse_drift_voyage), one wrapper does render + publish:

```bash
python tools/audio/render_sandbox_music.py            # all three
python tools/audio/render_sandbox_music.py --cue lofi_study_loop
python tools/audio/render_sandbox_music.py --skip-render  # just republish
python tools/audio/render_sandbox_music.py --force-render # ignore mtime cache
```

Skips re-rendering when the cue's YAML mtime is older than the latest
preview. Publish copies the newest hashed
`<cue>_<hash>.full_soundtrack_preview.ogg` to the stable
`crates/ambition_sandbox/assets/audio/music/generated/<cue>/full.ogg`
that `MusicTrackSpec.asset_path` references in `sandbox.ron`.

The Rust side falls back to the legacy procedural `render_lofi_theme`
synth path on any track whose asset_path file is missing, so partial
renders still load the game.

For any cue manually:

```bash
cd tools/ambition_music_renderer
source .venv/bin/activate
python -m ambition_music_renderer.render_isolated \
    scores/active/<cue>.music.yaml \
    --outdir output/<cue> \
    --backend pretty-midi
```

Each render writes a `regen.sh` into the output directory containing the
exact CLI invocation that produced it. To re-render the same cue with the
same settings:

```bash
bash output/<cue>/regen.sh
```

## Output files

For a cue rendered into `output/<cue>/`:

```
output/<cue>/
├── adaptive/
│   ├── intro/
│   │   ├── <cue>_<hash>.intro.<stem>.ogg     # one per stem group
│   │   └── <cue>_<hash>.intro.full.ogg       # mastered slice of master mix
│   ├── verse/   ...
│   └── outro/   ...
├── preview/
│   ├── <cue>_<hash>.full_soundtrack_preview.ogg    # mastered full mix
│   ├── <cue>_<hash>.in_game_minimal.ogg            # bridge stems only
│   ├── <cue>_<hash>.in_game_maximal.ogg            # all stems at gain 1.0
│   └── <cue>_<hash>.in_game_state_<name>.ogg       # one per state_map state
├── <cue>_<hash>.adaptive_manifest.json
└── regen.sh                                  # re-render with same settings
```

The mastered preview applies the YAML `postprocess` chain (master EQ +
reverb + limiter). The in-game previews apply only soft-limit normalization,
because that's what the runtime mixer actually does — they're an honest
approximation of what playback in-engine sounds like.

## Score file format (high level)

Each cue is a YAML file under `scores/`. Layout:

- `scores/active/` — cues actively used by the sandbox runtime.
- `scores/examples/` — reference / example cues kept for development.
- `scores/archive/` — historical cues kept for reference.

The structure is:

- **`tempo` / `meter`** — BPM and beats-per-bar.
- **`render`** — sample rate, OGG quality, default backend, SoundFont pin.
- **`postprocess` / `stem_postprocess` / `group_postprocess`** — three
  layers of EQ + reverb + limiter settings, applied at master, per-stem,
  and per-instrument-group respectively. The full chain in order is:
  `gain_db → highpass_hz → transient_tame → presence_db → high_shelf_db →
  lowpass_hz → compressor → reverb → post_reverb_high_shelf_db →
  stereo_width → soft_limit`.
  - **Compressor** is opt-in: set `compressor_threshold_db` to enable.
    Tunable via `compressor_ratio` (default 3), `compressor_attack_ms`
    (10), `compressor_release_ms` (100), `compressor_makeup_db` (0),
    `compressor_knee_db` (6, soft knee). Useful for master bus glue.
  - **Reverb** is a Schroeder/Freeverb-style algorithm: 4 parallel
    lowpass-feedback combs into 2 series allpass diffusers. `reverb_decay_seconds`
    is the approximate RT60; `reverb_damping_hz` controls tail brightness.
- **`constraints`** (optional, top-level or per-layer) — opt-in voicing
  rules. See *Constraint flags* below.
- **`instruments`** — list of instrument specs. Each gets a `name`, `group`
  (one of `strings`, `winds`, `brass`, `mallets`, `percussion`, `choir_pad`),
  GM `program` or `is_drum: true`, plus initial MIDI volume / pan /
  expression / modulation values.
- **`motifs`** — named melodic patterns. Each has a `root`, an `intervals`
  array (semitones from root), a `rhythm` array (beats per note), and a
  `velocities` array (0..1 multipliers).
- **`layer_templates`** — reusable layer definitions referenced from sections
  via `template:`. Each is one of these `kind` values:
  - `pad_chords` — sustained chord pads
  - `chord_hits` — short chord hits at specific bars/beats
  - `bassline` — bass figures with explicit beat/duration patterns
  - `motif` — applies a named motif to one or more instruments
  - `arpeggio` — arpeggiates chord tones at a step rate
  - `pedal` — single sustained note for the section's duration
  - `root_hits` — hits on the chord root (timpani, low brass)
  - `drums` — drum patterns with named drum hits
  - `automation` — CC ramps (volume, expression, modulation) with curve
    options: `linear`, `smooth`, `exp`, `lfo` (sine sweep with `cycles`)

  Every layer kind that produces notes accepts:
  - `humanize_ms` — Gaussian time jitter per note (sigma in ms).
  - `humanize_velocity_pct` — Gaussian velocity jitter per note (sigma in
    percent of nominal velocity). 2-4% gives a natural ensemble feel.

  The `motif` kind additionally accepts:
  - `pitch_bend_curve` (or `instrument_pitch_bend_curves`) — list of
    `[beat_offset, cents]` waypoints applied across each note. Use for
    sustained guitar bends or slide effects. Example:
    `[[0.0, 0], [0.1, 100], [0.5, 100], [0.7, 0]]` rises a semitone, holds,
    releases.
- **`playback`** — runtime crossfade rules: `loop_components`, `bridge_stems`,
  fade behavior on exit.
- **`state_map`** — named gameplay states each pointing at a section and
  optional per-stem gains. Used by the runtime to dynamically remix.
- **`sections`** — the actual cues. Each has a `bars` count, an `intensity`
  multiplier, a `harmony` array (one chord symbol per bar), and a `layers`
  array. Layers can reference templates (`{template: foo, ...overrides}`)
  or be inline. A section may also define its own **`postprocess`** block
  to override the master ambience for that section's `.full.ogg` slice
  (intimate intro vs cathedral climax without remixing every stem).

See `scores/examples/violin_boss_relentless.music.yaml` for a thoroughly-commented
production example.

## Existing example cues

| File | Style |
|---|---|
| `first_goblin_tune_v2.music.yaml` | Active goblin-encounter cue (production) |
| `first_goblin_encounter.music.yaml` | Earlier goblin score (kept for reference) |
| `lofi_study_loop.music.yaml` | Default sandbox music — chill study lofi (32-bar Fm9 loop) |
| `long_lofi_drift.music.yaml` | Extended sandbox lofi (64-bar Am loop, 4 phrase regions) |
| `pulse_drift_voyage.music.yaml` | Sandbox water_world music (48-bar A major / Lydian, 92 BPM) |
| `moonlit_canal.music.yaml` | Sample non-combat cue |
| `violin_boss_relentless.music.yaml` | D-minor boss battle, violin-led, 4/4 |
| `crooked_ascent_boss.music.yaml` | A-Phrygian-Dominant boss, 5/4, klezmer flavor |
| `military_iron_resolve.music.yaml` | G-minor military march, brass-led |
| `env_advocacy_solace.music.yaml` | G-Mixolydian pastoral, 6/8, woodwind-led |
| `tech_bros_disruption.music.yaml` | F#-minor synthwave, electronic instruments |
| `dinosaur_liberators.music.yaml` | Southern-rock anthem (~3 min) |
| `dinosaur_liberators_long.music.yaml` | Same anthem at full 9:08 length |

## Music-theory reference

Concepts to reach for when composing new YAML cues. Some are already
honored by the renderer; others are author discipline.

### Modes and color tones

The defining "color tone" of each mode — write motifs that prominently use
these notes to make the modal flavor unambiguous:

| Mode | Color tone | Used in |
|---|---|---|
| Phrygian | b2 | violin_boss_relentless (D Phrygian) |
| Phrygian Dominant | b2 + raised 3 (augmented 2nd between b2 and 3) | crooked_ascent_boss |
| Mixolydian | b7 | env_advocacy_solace (G Mixolydian) |
| Lydian | #4 | not yet used |
| Harmonic minor | natural 7 + augmented 2nd between b6 and 7 | dinosaur_liberators (E minor pentatonic with leading tone in solos) |
| Whole tone | no half steps; ambiguous | not yet used |
| Locrian | b5 | not yet used (very unstable) |

### Chord-progression vocabulary by genre

- **Pop / rock**: I — V — vi — IV; vi — IV — I — V; i — VII — VI — V (Andalusian)
- **Jazz**: ii7 — V7 — Imaj7 — VI7 (turnaround); rhythm changes
  Imaj7 — IV7 — iii7 — VI7
- **Classical**: I — IV — V — I; chord substitution `V7 → bII7` (tritone
  sub); deceptive cadence `V → vi`
- **Cinematic / electronic**: i — VII — VI — VII; modal mixture
  I — III — IV — iv (borrowed iv from parallel minor)

### Voice-leading discipline

When writing chord layers (`pad_chords`, `chord_hits`):

- **Smallest motion wins.** When the harmony moves from one chord to the
  next, each voice should move to the *nearest* chord tone of the new
  chord, not jump to the new root.
- **No parallel perfect fifths or octaves** between adjacent voices. If two
  voices are already a fifth or octave apart, they must move in *contrary*
  or *oblique* motion (or use a different interval), never in parallel.
- **Common-tone preservation.** If two adjacent chords share a note,
  hold that note in the same voice rather than re-attacking it.
- **Doubling rule.** In four-voice writing, double the root or fifth, not
  the third or seventh. Never double the leading tone (b7 in major,
  raised 7 in minor).
- **Cluster avoidance.** Voices should not be a minor 2nd apart (semitone)
  unless the dissonance is a deliberate suspension awaiting resolution.

The renderer's current `add_chord` voicing is naive — it just stacks chord
tones above the root. Fixing this on the renderer side is a future
improvement; for now, write motifs with explicit voicing in mind, and use
slash-bass notation (e.g. `Bm/F#`) to control the bass line.

### Counterpoint and independent lines

A single melody plus chord pads is the simplest texture. To get richer
sound:

- Add a **countermelody** that moves in *contrary motion* to the lead.
  Example: lead ascends G → A → B, countermelody descends D → C → B.
- Use **suspensions** at phrase boundaries: hold a chord tone over a
  harmony change so it becomes a non-chord tone, then resolve it down by
  step. The standard suspensions are 4-3, 7-6, 9-8.
- Begin and end multi-line passages on a perfect consonance (unison,
  fifth, or octave). Use imperfect consonances (thirds, sixths) inside.

### Orchestration

- **Frequency separation.** Bass instruments live ~50–250 Hz, rhythmic
  middleground 250–500 Hz, harmonic body 500–2000 Hz, melodic lead 2000–
  8000 Hz. Two instruments fighting for the same band sound muddy.
- **Doubling for color.** Violin + flute reads as a single bright
  instrument. Cello + bassoon reads as a single dark instrument.
  Strings + brass at a unison reads as "epic block" rather than two
  voices. Use these to thicken without adding harmonic complexity.
- **Bass-melody gap.** Keep the bass line at least two octaves below the
  melody. If they're in adjacent octaves the mix loses clarity.
- **Dynamic layering.** Build texture progressively. A common pattern:
  solo (pp–p) → woodwinds (p–mp) → strings (mp–mf) → brass (mf–f) → tutti
  + percussion (f–ff).

### Rhythmic ideas not yet used

- **3-against-2 polyrhythm.** Two voices, one in 3/8 feel and one in 2/4
  feel, occupying the same bar. Creates harmonic-rhythmic tension.
- **Hemiola.** In 6/8, one voice articulates the natural 2-group-of-3
  beat, another articulates 3-group-of-2. Same notes, different felt
  meter.
- **Anticipation / lay-back.** Notes intentionally early or late by ~10–
  30 ms. Already supported via `humanize_ms`.
- **Walking bass.** Quarter-note line that connects each chord change
  with stepwise motion. Easy to express as a `bassline` template with
  per-bar custom patterns.

### Constraint flags (opt-in)

The renderer supports opt-in voicing constraints. Each is **off by default** —
there are legitimate musical reasons to break each rule, so the renderer
will not enforce them silently. Set at the spec level (applies to every
chord layer) or on a per-layer basis (overrides spec-level):

```yaml
constraints:
  voice_leading: minimize_motion   # find permutation/octave shift of
                                    # each new chord that minimizes total
                                    # voice motion from the previous chord
                                    # on the same instrument
  no_clusters: true                # if any two voices end up a minor 2nd
                                    # apart, raise the higher one an octave
                                    # until no clusters remain
```

Constraints currently apply only inside `add_chord` (so they affect
`pad_chords` and `chord_hits` layers). Per-layer override:

```yaml
chorus_block:
  kind: chord_hits
  ...
  constraints:
    voice_leading: minimize_motion
    no_clusters: true
```

## Stable filenames

`render_isolated` writes hash-suffixed filenames so re-rendering does not
silently replace assets. The repo installer
(`tools/audio/install_first_goblin_tune_v2_assets.py`) copies those files
into the Bevy asset tree under stable names like
`adaptive/wave1/wave1.strings.ogg`, which the Rust loader targets directly.

## Renderer version

The current renderer version is recorded in `RENDERER_VERSION` at the top
of `musicir_renderer.py`. The version is part of the cue hash, so bumping
it invalidates all stored hashes — useful when changing rendering behavior
in a way that affects audio output.
