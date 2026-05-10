# Mockingbird boss sprite generator

This overlay adds a standalone script:

- `tools/ambition_sprite2d_renderer/mockingbird_boss_sprite_generator.py`

It is designed to behave more like the other Ambition tool renderers:

- outputs into `tools/ambition_sprite2d_renderer/generated/mockingbird_boss/`
- exposes `render`, `preview`, `install`, and `render-publish` commands
- writes installable assets under `crates/ambition_sandbox/assets/sprites/mockingbird_boss/`
- emits a manifest JSON alongside the spritesheet

## Commands

```bash
python tools/ambition_sprite2d_renderer/mockingbird_boss_sprite_generator.py render
python tools/ambition_sprite2d_renderer/mockingbird_boss_sprite_generator.py preview
python tools/ambition_sprite2d_renderer/mockingbird_boss_sprite_generator.py install
python tools/ambition_sprite2d_renderer/mockingbird_boss_sprite_generator.py render-publish
```

## Outputs

Generated files:

- `mockingbird_boss_spritesheet.png`
- `mockingbird_boss_spritesheet_manifest.json`
- `mockingbird_boss_preview_labeled.png`
- `mockingbird_boss_canonical.png`
- `mockingbird_boss_canonical_transparent.png`
- `sources_and_inspirations.md`

## Animation rows

The spritesheet is organized by animation row and frame column.
Current rows:

- `rest`
- `floor_slam`
- `side_sweep`
- `spike_halo`
- `dash_echo`
- `hit`
- `death`

## Inspiration links

- https://archive.org/download/htkam/TKAM%28www.albinoblacksheep.com%29.swf
- https://archive.org/download/how-to-kill-a-mockingbird/how-to-kill-a-mockingbird.swf

## Notes

- The boss is intentionally larger and more aggressive than the earlier draft.
- The reconstruction is still primitive / PIL-based and does not depend on extracted source art assets.
- This is a standalone tool-side script for later integration into the fuller `ambition_sprite2d_renderer` package.
