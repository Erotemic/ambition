# Ambition music renderer

Author-time renderer for generated Ambition music assets. Generated outputs are local until explicitly installed or published into runtime assets.

This package is the canonical code-only music generator for the project. Do not commit ad-hoc rendered `.ogg`, `.wav`, `.mid`, or temporary stem buffers unless a task explicitly says to publish runtime assets.

## Common commands

Run from the repo root unless noted:

```bash
PYTHONPATH=tools/ambition_music_renderer python -m ambition_music_renderer --help
./generate_audio_assets.sh --force
```

From the tool directory:

```bash
cd tools/ambition_music_renderer
python -m ambition_music_renderer --help
./render_first_goblin_transition_lab.sh
python transition_audit.py --help     # two-file transition seam
python audit_cue_balance.py --help    # sections within one cue
python level_report.py --help         # inter-cue catalog levels; --check fails on clipping
```

Use the package CLI and scripts in this directory for current music-renderer work. Older docs may mention retired paths under `tools/audio/`; those paths are stale and should not be copied into new instructions.

## Useful files

- `ambition_music_renderer/cli.py` - package CLI.
- `ambition_music_renderer/musicir_renderer.py` - main MusicIR renderer and renderer version.
- `scores/active/` - cues actively used or being prepared for runtime.
- `scores/examples/` - reference/example cues.
- `scores/archive/` - historical cues kept for reference.
- `render_first_goblin_transition_lab.sh` - local transition-lab helper.
- `install_first_goblin_tune_v2.py` - installer for the first-goblin tune asset path.
- `audit_cue_balance.py`, `transition_audit.py`, `level_report.py`, `spectral_compare.py`, `spectral_localize.py` - analysis helpers (`level_report.py` is the diff-friendly cross-catalog loudness/clipping report; `--check` gates clipping).
- `goals.md` - design/planning notes for renderer direction.

## Dependencies and backends

The renderer can use multiple backends depending on local setup:

| Backend | What it is | When to use |
|---|---|---|
| `pretty-midi` | pyFluidSynth + SoundFont, internal reverb/chorus disabled | Preferred for production-quality local renders when available. |
| `fluidsynth-cli` | the `fluidsynth` binary + SoundFont | Useful when Python FluidSynth bindings misbehave. |
| `fallback` | additive synth fallback | Portable/CI-ish fallback; sounds synthetic. |
| `auto` | backend selection/fallback policy | Good for scripts that should run on many machines. |

SoundFont preference is defined in the renderer code. Prefer high-quality MuseScore/FluidR3 style General MIDI SoundFonts when available. Override per-cue with `render.soundfont` in YAML or per invocation with a backend-specific CLI flag when supported.

## Output and publish model

Rendering is a staging step. Publishing/installing is a separate decision.

Typical generated output for a cue includes:

```text
generated/<cue>/
  adaptive/<section>/
    <section>.full.ogg
    <section>.<stem>.ogg
  preview/
    full_soundtrack_preview.ogg
    in_game_minimal.ogg
    in_game_maximal.ogg
    in_game_state_<name>.ogg
  <cue>.adaptive_manifest.json
```

Runtime assets live under:

```text
crates/ambition_sandbox/assets/audio/music/generated/<cue>/
```

For `first_goblin_tune_v2`, the top-level wrapper currently renders/installs the active cue path used by the sandbox. By default, prefer full-mix render/install for the cue the game actually loads. Use stem rendering when auditing or reviving stem-driven runtime playback.

## Score file format

Music scores are YAML files under `scores/`. At a high level:

- `tempo` / `meter` - BPM and beats per bar.
- `render` - sample rate, OGG quality, backend, SoundFont pin, and render-specific settings.
- `postprocess`, `stem_postprocess`, `group_postprocess` - EQ, reverb, limiter/compressor, stereo width, and related mastering controls at different mix levels.
- `constraints` - optional voicing rules such as minimizing motion or avoiding clusters.
- `instruments` - named instruments with group, GM program/drum settings, MIDI volume/pan/expression/modulation.
- `motifs` - reusable melodic/rhythmic patterns.
- `layer_templates` - reusable layer definitions.
- `playback` - runtime crossfade/loop behavior.
- `state_map` - gameplay states mapped to sections and optional stem gains.
- `sections` - cue sections with bar count, intensity, harmony, layers, and optional section postprocess.

Common layer kinds include:

- `pad_chords`
- `chord_hits`
- `bassline`
- `motif`
- `arpeggio`
- `pedal`
- `root_hits`
- `drums`
- `automation`

Most note-producing layers accept timing and velocity humanization. Motif layers can also carry pitch-bend curves for slides or bends.

## Constraint flags

The renderer supports opt-in voicing constraints. They are off by default because there are legitimate musical reasons to break each rule.

Example:

```yaml
constraints:
  voice_leading: minimize_motion
  no_clusters: true
```

Per-layer overrides can use the same shape. Constraints currently apply to chord construction paths; do not assume every layer kind enforces them.

## Debugging transitions and balance

For adaptive cues, distinguish runtime problems from generated-audio problems before changing code:

1. Render/regenerate the cue.
2. Audit generated and installed OGGs with `audit_cue_balance.py`.
3. Run the game in the relevant room and capture music logs.
4. Confirm whether the runtime starts the next state at target gain or fades from silence.
5. Listen to adjacent generated files outside the game to decide if the seam exists before runtime touches them.

Useful future improvements tracked in `TODO.md`:

- level reports with LUFS / peak / RMS / duration,
- live in-engine gain HUD,
- equal-power crossfade experiments,
- mastered per-stem outputs if stem-driven playback returns,
- clearer staging vs production publish flow.

## Music-theory reference

When composing new YAML cues, prefer explicit, inspectable musical choices:

- Preserve common tones and minimize voice motion between adjacent chords when the texture wants smoothness.
- Avoid accidental parallel perfect fifths/octaves in classical-ish writing unless the style wants it.
- Keep bass instruments, harmonic body, and lead instruments separated enough that the mix remains readable.
- Use dynamic layering to intensify gameplay states: sparse intro -> loop body -> denser combat -> recap/outro.
- Modal color tones help cues feel intentional: Phrygian b2, Lydian #4, Mixolydian b7, harmonic-minor leading tone, etc.
- Humanization and anticipation/lay-back can make generated parts feel less mechanical; keep values small and deliberate.

## Agent rules

- Keep generated audio out of runtime assets unless the task explicitly installs/publishes it.
- Preserve conservative gain ranges in tune specs; the runtime renderer can clip if stems are too hot.
- Treat `first_goblin_tune_v2` as the current active adaptive-music lab, not as the final abstraction for all encounters.
- Update `docs/recipes/generated-music-workflow.md` and `docs/tools/generated-audio-tools.md` when the workflow changes.
