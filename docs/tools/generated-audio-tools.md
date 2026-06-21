
# Generated audio tools

## Music renderer

Location: `tools/ambition_music_renderer/`

Purpose: render, publish, audit, and package generated/adaptive music cues and transition labs.

Common entry points:

```bash
cd tools/ambition_music_renderer
python -m ambition_music_renderer --help
python -m ambition_music_renderer cue bundle <cue_id> --backend pretty-midi --force --zip
python -m ambition_music_renderer cue bundle <cue_id> --backend pretty-midi --runtime-stem-gain-mode shared --force --zip
python -m ambition_music_renderer cue bundle <cue_id> --backend pretty-midi --runtime-stem-gain-mode shared --zip-report --force
python -m pytest tests
```

The `cue bundle` subcommand is the preferred one-cue handoff path: it runs the renderer with retained debug stems, executes useful diagnostics, writes manifest-scoped reports/plots, prints clickable output paths, and optionally writes a full zip or compact report zip. Use `--zip-report` for small chat/agent uploads: it excludes OGG/WAV/NPY binaries while keeping YAML, manifests, numeric reports, LLM-friendly spectral fingerprints, and JPEG spectrograms. Use `--runtime-stem-gain-mode shared` when auditing layered dynamic music; shared gain is capped so the exporter does not hide quiet-source problems by amplifying noise floors. Generated bundles and runtime audio remain ignored by git.

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
