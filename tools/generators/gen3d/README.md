
## v0.4.10

- Refined goblin side-pose facial registration so the nose sits more centrally relative to the rotated eye line.
- Pushed the dagger arm farther out from the torso silhouette so the right arm reads cleanly in the side sprite instead of merging into the body.
- Kept the side-pose head derived from the same canonical construction layout instead of introducing a separate side-view head description.

## v0.4.9

- Reworked the goblin side-pose head to reuse the same canonical head layout as the construction view and then rotate it into the gameplay pose.
- Head attachments (ears, eye sockets, eyes, nose, mouth, teeth) are now positioned from canonical local coordinates via a shared head transform instead of a separate ad hoc side-view layout.
- This should keep the near ear attached behind the face plane and preserve a more principled 3/4 face read in the side sprite.

## v0.4.8

- Corrected goblin ear diagnosis: the close-up showed the ear was not disappearing; it was pushed too far into the face plane and overlapping the eye/nose region.
- Moved the near ear higher and behind the visible face plane, reduced its thickness, and added a small inner-ear accent so it reads as an ear instead of a horn.
- Kept the goblin head / face visible while preserving the sprite-first v0.4.6+ color pipeline.
## v31 notes

- Switched rendering toward a sprite-first setup: Standard view transform instead of a washed-out filmic look.
- Disabled bloom and reduced fill/rim lighting so the characters read with more contrast as standalone pre-rendered sprites.
- Materials now push color through a hue/saturation node and stronger texture mix so accents and patterns should survive downsampling.

## v30 notes

- Boosted palette saturation and contrast so the goblin purples and robot cyan/purple accents read more clearly.
- Increased texture influence and material contrast so generated textures should be more visible.
- Darkened the world lighting slightly so character colors do not wash out as much.

## v29 notes

- Added an explicit rendered version badge (for example `v0.4.4`) to canonical images, spritesheets, and contact sheets.
- Added CLI startup version printing and manifest metadata version to help debug whether a new package was actually unpacked and executed.
- This release is primarily for iteration/debug visibility rather than character design changes.

## v28 notes

- Based on v27 / v15 track.
- Goblin side pose is turned further toward the camera (yaw 30 degrees instead of 40) so the face reads more clearly in motion.
- Goblin side head-face pieces and front ear were pushed further into view to improve facial readability and ear silhouette.
- Accent colors are less pastel and more saturated, with slightly stronger texture mixing so materials read a bit richer.

## v27 notes

- Based directly on v15.
- Robot remains on the v15 side-pose code path.
- Goblin side pose yaw reduced from 56 degrees to 40 degrees so the face stays more visible in side movement renders.

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

## CLI output and logs

Blender stdout/stderr is hidden by default to keep `draw-all` and `canonical-all` readable.
If Blender fails, the tool writes `_blender_render.log` beside the active output frames and prints the last part of the log.
Set `GEN3D_BLENDER_LAB_VERBOSE=1` to show Blender's raw output while debugging.

The canonical commands print a Rich `file://` link to `assets/character_canonicals.png` so supported terminals can open the review image directly.

## v0.4.7

- Adjusted goblin side-pose presentation to preserve clearer 3/4 face readability in sprite renders.
- Moved the near goblin ear forward and outward so it remains visible instead of collapsing into the head silhouette.
- Reduced goblin side-pose yaw slightly to keep the face readable for 2D pre-rendered sprite use.
