# ambition_background_renderer

Procedural placeholder renderer for Ambition parallax background layers.

The goal is not final art. It gives the game a simple layered background pipeline
now, with deterministic assets that can be replaced by hand-painted transparent
PNG layers later.

## Generate all shipped placeholder layers

From the repository root:

```bash
python scripts/generate_background_assets.py
```

Generated files are written to:

```text
game/ambition_content_library/generated/runtime/backgrounds/default/sky.png
game/ambition_content_library/generated/runtime/backgrounds/default/far.png
game/ambition_content_library/generated/runtime/backgrounds/default/mid.png
game/ambition_content_library/generated/runtime/backgrounds/default/near.png
game/ambition_content_library/generated/runtime/backgrounds/default/manifest.txt
```

## Direct CLI

```bash
python -m ambition_background_renderer \
  --out game/ambition_content_library/generated/runtime/backgrounds --profile default
```

Later LDtk room metadata can select a profile per area/room.
