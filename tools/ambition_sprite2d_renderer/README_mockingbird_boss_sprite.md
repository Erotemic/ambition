# Mockingbird boss sprite generator

This overlay adds a standalone script:

- `tools/ambition_sprite2d_renderer/mockingbird_boss_sprite_generator.py`

It is designed to behave more like the other Ambition tool renderers:

- outputs into `tools/ambition_sprite2d_renderer/generated/mockingbird_boss/`
- exposes `render`, `preview`, `install`, and `render-publish` commands
- writes installable assets under `crates/ambition_gameplay_core/assets/sprites/mockingbird_boss/`
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

- `mockingbird_boss_spritesheet.png` — 576×216 frames by default
- `mockingbird_boss_spritesheet_manifest.json` — includes frame size and alpha-bbox fill metrics
- `mockingbird_boss_preview_labeled.png`
- `mockingbird_boss_canonical.png`
- `mockingbird_boss_canonical_transparent.png`
- `sources_and_inspirations.md`

## Animation rows

The spritesheet is organized by animation row and frame column.
Current rows, in manifest / PNG order:

- `hover`
- `thrust`
- `bite`
- `slash`
- `hit`
- `death`

The Rust `MOCKINGBIRD_SHEET` maps those rows onto the shared boss animation vocabulary.

## Inspiration links

- https://archive.org/download/htkam/TKAM%28www.albinoblacksheep.com%29.swf
- https://archive.org/download/how-to-kill-a-mockingbird/how-to-kill-a-mockingbird.swf

## Notes

- The boss is intentionally larger and more aggressive than the earlier draft.
- The default frame is wide rather than square so the bird fills ~82% of the frame height instead of ~32% while preserving safer edge margin for the nose/flame silhouettes; this avoids Rust-side texture upscaling.
- The reconstruction is still primitive / PIL-based and does not depend on extracted source art assets.
- This is a standalone tool-side script for later integration into the fuller `ambition_sprite2d_renderer` package.
