# Standalone pirate spritesheet generators

This overlay adds standalone scripts that generate two new pirate spritesheets in
an Ambition-compatible PNG + YAML layout.

## Targets

- `pirate_admiral` — gaunt eyepatch captain with blue coat / rapier
- `pirate_raider` — loud redcoat raider with beard / cutlass

## Render both locally

```bash
python tools/ambition_sprite2d_renderer/render_pirate_spritesheets.py
```

Outputs are written under:

```text
tools/ambition_sprite2d_renderer/generated/
```

Each target gets:

```text
<target>_canonical.png
<target>_canonical_transparent.png
<target>_preview_labeled.png
<target>_spritesheet.png
<target>_spritesheet.yaml
```

## Render one target

```bash
python tools/ambition_sprite2d_renderer/render_pirate_spritesheets.py --target pirate_admiral
python tools/ambition_sprite2d_renderer/render_pirate_spritesheets.py --target pirate_raider
```

## Publish the runtime spritesheets

```bash
python tools/ambition_sprite2d_renderer/publish_pirate_spritesheets.py
```

or

```bash
./regen_sprites.sh
```

This copies only the runtime pair for each target into:

```text
crates/ambition_gameplay_core/assets/sprites/
```

## Included animations

Rows emitted in each sheet:

- idle
- walk
- slash
- taunt
- hurt
- death

Death includes X-eyes. The sheets are rendered procedurally from vector-like
PIL primitives and downsampled, so transforms are applied to the underlying
shapes rather than post-transforming already rasterized sprites.
