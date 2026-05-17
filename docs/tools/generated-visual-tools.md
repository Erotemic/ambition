
# Generated visual tools

## Sprite renderer

Location: `tools/ambition_sprite2d_renderer/`

Purpose: generate and publish 2D character/entity spritesheets and rig assets.

```bash
cd tools/ambition_sprite2d_renderer
python -m ambition_sprite2d_renderer --help
python -m pytest tests
```

## Background and parallax renderers

Locations:

- `tools/ambition_background_renderer/`
- `tools/ambition_parallax_renderer/`

Purpose: generate background images and parallax layers.

## Promo/vanity tools

Location: `tools/vanity_card_prep/` when present.

Purpose: prepare promotional card material, not gameplay runtime data.

## Experimental visual tools

`tools/experimental/` contains reference or in-progress work, including 3D sprite experiments, procedural fit experiments, and component extraction prototypes. Promote a tool out of `experimental/` and document its workflow before using it as a runtime asset source.
