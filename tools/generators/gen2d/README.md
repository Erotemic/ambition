# Procedural 2D Character Lab

A small Python package for generating deterministic, game-ready procedural 2D
character sprite sheets for Ambition prototypes.

The package currently includes two side-scroller targets:

- **robot**: a cute right-facing robot that reuses the older 2.5D robot head
  treatment, but stages the whole body as a side-view game character.
- **goblin**: a right-facing goblin enemy with a compact side profile and simple
  weapon variations.

## Highlights

- YAML-driven jobs for reproducible art generation.
- Deterministic seeds and archetypes.
- Single-frame render and full sprite-sheet export.
- Quick canonical look-dev render generation.
- Side-scroller friendly animation set shared across targets.
- No Blender dependency.

## Package layout

```text
proc2d_character_lab/
  adapters.py              # target-agnostic rendering facade
  canonical.py             # canonical frame rendering helpers
  cli.py                   # Typer CLI entrypoint
  config.py                # pydantic YAML job models
  sheet.py                 # labeled sprite-sheet + manifest writer
  rig.py                   # shared future rig primitives
  configs/
    robot.yaml             # default side-view robot job
    goblin.yaml            # default side-view goblin job
  targets/
    robot25d.py            # historical robot renderer primitives
    robot_side.py          # current side-view robot generator
    goblin_side.py         # current side-view goblin generator
```

## Quick start

Install editable from this directory:

```bash
python -m pip install -e .
```

List available targets:

```bash
pcg2d list-targets
```

Render the default robot sprite sheet:

```bash
pcg2d spritesheet proc2d_character_lab/configs/robot.yaml assets/robot_spritesheet.png --manifest-out assets/robot_spritesheet.yaml
```

Render one slash frame:

```bash
pcg2d single proc2d_character_lab/configs/robot.yaml assets/robot_slash.png --animation slash --frame-index 3
```

Equivalent module form:

```bash
python -m proc2d_character_lab.cli spritesheet proc2d_character_lab/configs/robot.yaml assets/robot_spritesheet.png --manifest-out assets/robot_spritesheet.yaml
```

## YAML jobs

A job describes the character target, seed, archetype, animation rows, and render
settings:

```yaml
target: robot
seed: 7
archetype: cute_scout
animations:
  - idle
  - walk
  - run
  - jump
  - fall
  - slash
  - hit
  - death
render:
  frame_width: 128
  frame_height: 128
  supersample: 6
  downsample: lanczos
  background: transparent
  sheet_background: transparent
  border: 0
  label_width: 100
```

## Project-root helper scripts

These are intended to work when run from the relative project root.

Render all sheets:

```bash
python draw_all_character_spritesheets.py
```

Render all canonicals:

```bash
python draw_character_canonicals.py
```

By default these read configs from `./proc2d_character_lab/configs` and write to
`./assets`.
