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
crates/ambition_actors/assets/backgrounds/default/sky.png
crates/ambition_actors/assets/backgrounds/default/far.png
crates/ambition_actors/assets/backgrounds/default/mid.png
crates/ambition_actors/assets/backgrounds/default/near.png
crates/ambition_actors/assets/backgrounds/default/manifest.txt
```

## Direct CLI

```bash
python -m ambition_background_renderer \
  --out crates/ambition_actors/assets/backgrounds --profile default
```

Later LDtk room metadata can select a profile per area/room.
