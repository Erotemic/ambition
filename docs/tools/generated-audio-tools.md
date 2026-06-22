
# Generated audio tools

## Music renderer

Location: `tools/ambition_music_renderer/`

Purpose: render, publish, audit, and package generated/adaptive music cues and transition labs.

Common entry points:

```bash
cd ~/code/ambition
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer --help
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer cue_bundle <cue_id> --backend=pretty-midi --force --zip
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer cue_bundle <cue_id> --backend=pretty-midi --runtime_stem_gain_mode=shared --force --zip
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer cue_bundle <cue_id> --backend=pretty-midi --runtime_stem_gain_mode=shared --zip_report --force
uv run --project ~/code/ambition/tools/ambition_music_renderer pytest -q tools/ambition_music_renderer/tests
```

The `cue_bundle` subcommand is the preferred one-cue handoff path: it runs the renderer with retained debug stems, executes useful diagnostics, writes manifest-scoped reports/plots, prints clickable output paths, and optionally writes a full zip or compact report zip. Use `--zip_report` for small chat/agent uploads: it excludes OGG/WAV/NPY/MIDI binaries while keeping YAML, manifests, numeric reports, LLM-friendly spectral fingerprints, dissonance hotspot reports, state mix reports, and JPEG spectrograms. When plotting is available it also writes `dissonance_hotspots.md`, `plots/dissonance_timeline.<fmt>`, `plots/dissonance_layer_pairs.<fmt>`, and stem-amplitude balance/timeline/stack plots. Use `--runtime_stem_gain_mode=shared` when auditing layered dynamic music; shared gain is capped so the exporter does not hide quiet-source problems by amplifying noise floors. Generated bundles and runtime audio remain ignored by git.

Standalone report helpers are also useful while editing:

```bash
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer tools arrangement_audit scores/active/<cue_id>.music.yaml --outdir=/tmp/<cue>_arrangement
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer tools dissonance_audit scores/active/<cue_id>.music.yaml --outdir=/tmp/<cue>_dissonance --plots=/tmp/<cue>_dissonance/plots
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer tools spectral_localize tools/ambition_music_renderer/generated/<cue_id> --window 0 -1
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer tools reference_audio_audit path/to/reference.mp3 --outdir=/tmp/reference_audio_audit
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer tools level_report --check
```

See [`../recipes/generated-music-workflow.md`](../recipes/generated-music-workflow.md) for the current recipe.

## SFX renderer

Location: `tools/ambition_sfx_renderer/`

Purpose: render/audit generated sound effects and banks. Some checkouts contain this as a nested tool checkout; do not delete it accidentally just because it looks self-contained.

Common entry points:

```bash
cd tools/ambition_sfx_renderer
python -m ambition_sfx_renderer --help
python -m pytest tests
```

## SFX packer

Location: `tools/ambition_sfx_pack/`

Purpose: pack sound effects into runtime banks.

## Policy

Generated audio becomes runtime input only after an explicit publish/install step. Update asset catalog docs when a cue/bank becomes a required runtime asset.
