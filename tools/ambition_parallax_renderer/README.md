# Ambition Parallax Renderer

`ambition_parallax_renderer` generates biome-specific background plates and
parallax atmosphere layers for the sandbox. This is the background/parallax art
pipeline, not the sprite pipeline. Generated files install under
`crates/ambition_gameplay_core/assets/backgrounds/parallax_layers/`.

The package supersedes the earlier background-renderer experiments while
keeping their scheme: every biome gets an opaque sky/backdrop color field plus
sparser transparent parallax plates for silhouettes, haze, cables, reeds, cave
lips, and other non-gameplay atmosphere. Runtime keeps the sky singular so
sun/moon/star details do not repeat, while the transparent plates may tile so
edge-framing silhouettes remain visible in large rooms.

## Usage

From the repo root, prefer:

```bash
./regen_backgrounds.sh
```

Direct renderer call:

```bash
cd tools/ambition_parallax_renderer
python -m ambition_parallax_renderer draw-backgrounds \
  --out-dir ../../crates/ambition_gameplay_core/assets/backgrounds/parallax_layers
```

The output directory is ignored by git; regenerate locally when you want to
refresh the background assets.
