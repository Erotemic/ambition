# gen3d blender lab

Blender-first procedural 3D character sprite generation for Ambition.

This package is intended to live here:

```text
/home/joncrall/code/ambition/tools/generators/gen3d/
  pyproject.toml
  gen3d_blender_lab/
    cli.py
    configs/
      goblin.yaml
      robot.yaml
    blender_backend/
      driver.py
      scene_builder.py
  assets/
  draw_all_character_spritesheets.py
  draw_character_canonicals.py
```

The Python module is named `gen3d_blender_lab` to make it clear that this is a
3D Blender-based generator. Blender is the single character-rendering backend.
Pillow is used only for packing rendered frames into sprite sheets, drawing row
labels, and building contact sheets.

## Relative-directory behavior

The tool does **not** depend on `/home/joncrall/code/ambition` being the current
working directory. By default it resolves paths relative to the directory that
contains the package:

```text
gen3d_blender_lab/cli.py -> parent parent -> tools/generators/gen3d
```

Default paths are therefore:

```text
configs: <gen3d>/gen3d_blender_lab/configs
assets:  <gen3d>/assets
```

You can override these with environment variables:

```bash
export GEN3D_BLENDER_LAB_ROOT=/path/to/tools/generators/gen3d
export GEN3D_BLENDER_LAB_CONFIG_DIR=/path/to/configs
export GEN3D_BLENDER_LAB_ASSET_DIR=/path/to/assets
export GEN3D_BLENDER_LAB_BLENDER_BIN=/path/to/blender
```

`BLENDER_BIN` is also accepted as a fallback for the Blender executable.

## Setup

From the `gen3d` directory:

```bash
cd /home/joncrall/code/ambition/tools/generators/gen3d
python -m pip install -e .
```

Or without installing, use:

```bash
cd /home/joncrall/code/ambition/tools/generators/gen3d
PYTHONPATH=$PWD:$PYTHONPATH python -m gen3d_blender_lab.cli list-targets
```

## Commands

Render every configured spritesheet into `<gen3d>/assets`:

```bash
cd /home/joncrall/code/ambition/tools/generators/gen3d
PYTHONPATH=$PWD:$PYTHONPATH python -m gen3d_blender_lab.cli draw-all
```

Equivalent top-level script:

```bash
cd /home/joncrall/code/ambition/tools/generators/gen3d
PYTHONPATH=$PWD:$PYTHONPATH python draw_all_character_spritesheets.py
```

Render canonical review images:

```bash
cd /home/joncrall/code/ambition/tools/generators/gen3d
PYTHONPATH=$PWD:$PYTHONPATH python -m gen3d_blender_lab.cli canonical-all
PYTHONPATH=$PWD:$PYTHONPATH python draw_character_canonicals.py
```

Render a single sheet explicitly:

```bash
cd /home/joncrall/code/ambition/tools/generators/gen3d
PYTHONPATH=$PWD:$PYTHONPATH python -m gen3d_blender_lab.cli spritesheet \
  gen3d_blender_lab/configs/goblin.yaml \
  assets/goblin_spritesheet.png
```

After editable install, the console command is also available:

```bash
gen3dlab draw-all
gen3dlab canonical-all
```

`pcglab` is retained as a temporary compatibility alias.

## Pipeline

```text
YAML config
  -> sampled procedural spec
  -> Blender scene builder constructs character in 3D
  -> Blender renders per-frame PNGs with side-scroller camera
  -> Pillow packs frames and draws labels
  -> PNG sprite sheet + YAML manifest
```

## Notes

- Canonicals and sprite sheets use the same Blender backend and camera.
- The current characters are primitive-built cel-shaded prototypes intended for
  quick visual iteration on your machine with Blender installed.
- This environment cannot run Blender renders, so visual validation should be
  done locally.


## v0.3.1 notes

- Removed the inverted-hull Solidify outline pass that caused black silhouette renders on some Blender versions.
- Enabled Blender Freestyle outlines instead.
- Simplified material nodes for stable colored renders across Blender 3.x/4.x.
- Moved robot visor and goblin eyes onto the camera-facing side plane while preserving side-scroller travel direction.
- Removed top in-image canonical labels; the contact sheet labels are now the canonical labels.
