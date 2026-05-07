# Ambition 2D Sprite Renderer

Procedural 2D sprite generator for Ambition. Targets are registered Python
modules that draw a spritesheet PNG plus a YAML manifest.

Each target generates local files first, under `generated/<target>/`. Runtime
assets are only updated when you explicitly run `install` or `render-publish`.

## Modal CLI

```
python -m ambition_sprite2d_renderer list
python -m ambition_sprite2d_renderer render <target>
python -m ambition_sprite2d_renderer preview <target>
python -m ambition_sprite2d_renderer install <target>
python -m ambition_sprite2d_renderer render-publish <target>
```

`render` writes the sheet into `tools/ambition_sprite2d_renderer/generated/<target>/`.

`install` copies the canonical sheet files into
`crates/ambition_sandbox/assets/sprites/`.

`render-publish` does both.

## Targets

### sandbag

Procedural pale cloth sandbag character. Sparse output (only `idle`, `hit`,
`death`). Runtime support for missing animations is provided by
`character_sprites.rs` resolving them to `idle` at load time.

Pass `--legacy-aliases` to also emit the 11-row alias sheet
(`sandbag_legacy_11row_spritesheet.*`) for old-runtime compatibility.

```bash
python -m ambition_sprite2d_renderer render sandbag
python -m ambition_sprite2d_renderer render-publish sandbag
```

Output:

```
generated/sandbag/sandbag_spritesheet.png
generated/sandbag/sandbag_spritesheet.yaml
```

After install:

```
crates/ambition_sandbox/assets/sprites/sandbag_spritesheet.png
crates/ambition_sandbox/assets/sprites/sandbag_spritesheet.yaml
```

## Adding a new target

1. Drop a module under `ambition_sprite2d_renderer/targets/<name>.py`.
2. Export `TARGET_NAME`, `SHEET_FILES`, and a `render(out_dir, **opts)`
   function that returns the list of paths it wrote.
3. Register the module in `ambition_sprite2d_renderer/targets/__init__.py`.

## Conventions

- Generated outputs live under `generated/` and are gitignored.
- Targets must be deterministic for a given input (same code → same bytes).
- Runtime assets are written only by explicit `install` / `render-publish`.
- Do not commit `.png`, `.yaml`, etc., from `generated/`.
