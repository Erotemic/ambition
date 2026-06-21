# Ambition 2D Sprite Renderer

Procedural 2D sprite renderer for Ambition. Two surfaces share the package:

1. **Adapter targets** — YAML-driven characters built on a `BaseAdapter`
   class (robot / goblin / ninja / boss / toon / sandbag / …). Jobs in
   `configs/*.yaml` describe the spec; `adapters.TARGETS` wires each
   target id to its rig. This is the "character lab" surface formerly
   published as `proc2d_character_lab`.
2. **Tack-on targets** — per-target modules under
   `targets/<category>/`. Each module exposes a `render()` function;
   discovery walks the tree and registers them automatically. No
   central registration list — dropping a file in the right category
   subdir is the entire integration step.

Tack-on targets are split across four category subdirs:

| Category | What goes here | Examples |
|---|---|---|
| [`targets/characters/`](ambition_sprite2d_renderer/targets/characters/) | Anything controllable by a brain — characters, bosses, tiny enemies | `ghoul_skulker`, `weird_hermit`, `mockingbird_boss/` (multi-file) |
| [`targets/props/`](ambition_sprite2d_renderer/targets/props/) | Items, weapons, gates, scene dressing, batched entity sheets | `lasersword`, `interdimensional_gate`, `entities` |
| [`targets/tiles/`](ambition_sprite2d_renderer/targets/tiles/) | LDtk tileset atlases | `intro_lab_tileset`, `town_tileset` |
| [`targets/icons/`](ambition_sprite2d_renderer/targets/icons/) | UI ability/item icons | `item_icons` |

## Package layout

The package is organized by role; the import boundary `core` ← `authoring`
← `targets` keeps the render heart dependency-light.

| Dir | Role | Deps |
|---|---|---|
| [`core/`](ambition_sprite2d_renderer/core/) | Rendering primitives — draw, pipeline, measure, frameset, the single RON emitter | **Pillow + stdlib only** |
| [`authoring/`](ambition_sprite2d_renderer/authoring/) | Render spines + per-paradigm helpers (`sheet`, `tackon_sheet`, `adapters`, `skeleton`, `rigdoc`, …) | +PIL |
| [`targets/`](ambition_sprite2d_renderer/targets/) | The actual sprite content (characters/props/tiles/icons) | +authoring |
| [`registry/`](ambition_sprite2d_renderer/registry/) | Target discovery (`discovery`) + adapter render config (`config`) | — |
| [`cli/`](ambition_sprite2d_renderer/cli/) | Command-line surface — `commands` (logic), `parser` (argparse + `main`), `console` | — |
| [`gui/`](ambition_sprite2d_renderer/gui/) | PySide6 rig editor | +PySide6 |
| [`devtools/`](ambition_sprite2d_renderer/devtools/) | Author-facing inspection (`debug_hitboxes`) | — |
| `configs/` · `data/` | YAML adapter jobs · rig templates | data |

## Modal CLI

**Unified Target commands** (take an optional `<TARGET>` from `list`; no arg = bulk):

```
python -m ambition_sprite2d_renderer list                   # every registered target, grouped by category
python -m ambition_sprite2d_renderer canonical [<target>]   # one canonical, or the full gallery
python -m ambition_sprite2d_renderer sheet     [<target>]   # one full sheet, or every tack-on sheet
python -m ambition_sprite2d_renderer install   [<target>]   # one install, or every tack-on install
python -m ambition_sprite2d_renderer publish   [<target>]   # sheet + install (one, or every tack-on)
```

**Adapter-pipeline commands** (take config paths or have unique semantics):

```
python -m ambition_sprite2d_renderer draw-all                # render every config in configs/
python -m ambition_sprite2d_renderer draw-review             # render every config in configs/review/
python -m ambition_sprite2d_renderer draw-character <cfg>    # one config: canonical + spritesheet + YAML
python -m ambition_sprite2d_renderer draw-factions           # music-faction lineup review render
python -m ambition_sprite2d_renderer draw-runtime-npcs       # render+install the curated runtime-NPC subset
python -m ambition_sprite2d_renderer spritesheet <cfg> <out> # one config's sheet to a specific path
python -m ambition_sprite2d_renderer single <cfg> <out>      # one frame from a config
python -m ambition_sprite2d_renderer regenerate-all          # draw-all + publish + draw-runtime-npcs
```

`sheet` writes to `tools/ambition_sprite2d_renderer/generated/<target>/`.
`install` copies the canonical sheet files into
`crates/ambition_gameplay_core/assets/sprites/`. `publish` does both.

The bulk forms of `sheet` / `install` / `publish` are scoped to the tack-on
surface (characters/props/tiles/icons); main YAML configs and review NPCs
have their own bulk paths (`draw-all` / `draw-runtime-npcs`).

## Adding a new sprite

Run `python -m ambition_sprite2d_renderer list-targets` to see what's
already registered before adding a new one.

The **canonical spec for the tack-on API** lives in the module
docstring of
[`ambition_sprite2d_renderer/registry/discovery.py`](ambition_sprite2d_renderer/registry/discovery.py).
That's the source of truth — this section is the practical walkthrough.

### 1. Pick a category

- **characters/** — anything controllable by a brain (state machine,
  RL agent, player input). Normal characters, bosses, tiny enemies.
- **props/** — items, weapons, gates, scene dressing, batched entity
  sheets.
- **tiles/** — LDtk tileset atlases (cells designed to repeat).
- **icons/** — UI ability/item icons.

A target can move between categories with a plain `git mv` — discovery
is path-agnostic beyond the category dir, and the relative-import
depth (`...tackon_sheet` etc.) is the same in every category. No
registration diff is involved.

### 2. Single-file target (most common)

Drop `targets/<category>/<name>.py`:

```python
"""Standalone generator for the my_new_enemy character."""
from __future__ import annotations
from pathlib import Path
from typing import List

from ...tackon_sheet import build_sheet

TARGET_NAME = "my_new_enemy"
# Optional — defaults to `{TARGET_NAME}_spritesheet.{png,yaml,ron}`
# which matches what `build_sheet` writes.
SHEET_FILES = [
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
]
ROWS = [("idle", 6, 120), ("walk", 8, 90), ("hurt", 4, 90), ("death", 8, 110)]


def _render_frame(anim, frame_idx, nframes):
    ...  # PIL rendering


def render(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=_render_frame,
        out_dir=out_dir,
    )
    return [outputs["spritesheet"], outputs["yaml"], outputs["ron"],
            outputs["preview"], outputs["canonical"], outputs["canonical_transparent"]]
```

That's it. `list-targets` will show it; `render-publish my_new_enemy`
will write + install it.

[`targets/characters/ghoul_skulker.py`](ambition_sprite2d_renderer/targets/characters/ghoul_skulker.py)
is a good copy-paste starting point for single-file characters.

### 3. Multi-file target (for big characters with helpers/configs)

If a target needs its own helper modules, part-config YAML files, or
part-editor scripts, ship it as a package directory:

```
targets/characters/my_boss/
  __init__.py            # exposes TARGET_NAME, SHEET_FILES, render(), install()
  sprite_generator.py
  part_editor.py
  my_boss_parts.yaml
  my_boss_scene.yaml
```

The `__init__.py` exposes the same tack-on API as a single-file
target. See
[`targets/characters/mockingbird_boss/__init__.py`](ambition_sprite2d_renderer/targets/characters/mockingbird_boss/__init__.py)
for the canonical example.

### 4. Multiple targets in one module

If one file naturally produces several related sheets (e.g. an entity
batch), expose a `TARGETS` dict instead of a single `render`:

```python
TARGETS = {
    "alpha": {"render": render_alpha, "sheet_files": [...]},
    "beta": {"render": render_beta},  # sheet_files defaults from the name
}
```

Each entry becomes its own registry key.

### Custom install (optional)

The default installer copies each path in `SHEET_FILES` from
`generated/<name>/` into `crates/ambition_gameplay_core/assets/sprites/`. If
your target ships a subdirectory of part files (or otherwise needs
non-flat install behavior), expose `install(render_dir, dest_root) ->
Iterable[Path]` and it'll be used instead.

### Helpers (drawing primitives, shared rigs)

- **Generic drawing + spritesheet building** — lives under
  [`authoring/`](ambition_sprite2d_renderer/authoring/):
  [`tackon_sheet.py`](ambition_sprite2d_renderer/authoring/tackon_sheet.py)
  (`build_sheet` + math + draw primitives) and
  [`common_draw.py`](ambition_sprite2d_renderer/authoring/common_draw.py)
  (adapter-helper drawing primitives). The RON emitter and core draw/measure
  primitives now live in [`core/`](ambition_sprite2d_renderer/core/). Use these
  from any target.
- **Character-family helpers** (shared by several characters in a
  family) live under `targets/characters/` with a leading underscore
  so discovery skips them — see
  [`targets/characters/_pirate_common.py`](ambition_sprite2d_renderer/targets/characters/_pirate_common.py)
  for the pirate-family rig (Palette + draw_character + animation_pose).

### Adapter target instead of tack-on

If your character fits the YAML-driven adapter shape (one parametric
rig with many archetype variants), the adapter path is preferred:

1. Drop a generator class under
   `targets/characters/<name>_side.py` (`Generator` with
   `sample_spec` + `render_animation_frame` methods).
2. Add an `XxxAdapter(BaseAdapter)` to `adapters.py` and register it
   in the module-level `TARGETS` dict.
3. Add `configs/<name>.yaml` (or a review config under
   `configs/review/`) describing the render parameters.
4. Add the file's stem to
   [`ADAPTER_HELPER_STEMS`](ambition_sprite2d_renderer/registry/discovery.py)
   in `registry/discovery.py` so discovery skips it.

[`targets/characters/robot_side.py`](ambition_sprite2d_renderer/targets/characters/robot_side.py)
+ [`configs/robot.yaml`](ambition_sprite2d_renderer/configs/robot.yaml) is
the canonical adapter target.

## Character specs and review casts

`CharacterJob` accepts optional `name`, `output_name`, and `spec` fields.
The `toon` target uses those `spec` overrides to author silhouette-first
characters without inventing a brand new renderer per NPC. Review presets can
be intentionally trope-heavy: `absurd_general` is the shouting-general pass with
a giant star cap, epaulets, medals, awards, baton, and irate yell face. Example:

```yaml
target: toon
name: Merchant Prototype
output_name: merchant_prototype
archetype: merchant_prototype
spec:
  torso_w: 31.5
  leg_upper: 10.5
```

The curated review pass lives in `ambition_sprite2d_renderer/configs/review/`
and is meant to answer the question, “do these feel like different characters?”
Use `draw-review` to regenerate the current cast along with a canonical contact
sheet. The `raid_enforcer` preset is a fictional raid-enforcer enemy pass: severe
cap, charcoal tunic, red armband with an invented black sigil, collar skull
tabs, and a long rifle so it reads as a distinct villain rather than a variant
of `absurd_general`.

## Conventions

- Generated outputs live under `generated/` and are gitignored.
- Targets must be deterministic for a given input (same code → same bytes).
- Runtime assets are written only by explicit `install` / `render-publish`
  / `render-publish-all` / `regenerate-all` (or `draw-all` /
  `draw-canonicals` / `draw-entities` / `draw-icons` for adapter targets).
- Do not commit `.png`, `.yaml`, etc., from `generated/`.

See [`docs/design.md`](docs/design.md) for the architecture rationale
and [`docs/ENTITY_TODOS.md`](docs/ENTITY_TODOS.md) for outstanding
entity-sprite work.
