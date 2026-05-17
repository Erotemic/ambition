
# Generated audio tools

## Music renderer

Location: `tools/ambition_music_renderer/`

Purpose: render and audit generated/adaptive music cues and transition labs.

Common entry points:

```bash
cd tools/ambition_music_renderer
python -m ambition_music_renderer --help
python -m pytest tests
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
