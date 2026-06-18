---
id: multi-file-character-target
status: current
aliases:
  - multi-file tack-on
  - mockingbird tack-on reference
  - rich rig target
last_verified: 2026-05-24
implemented_by:
  - tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/mockingbird_boss/
---

# Multi-file character target (reference: `mockingbird_boss`)

The `mockingbird_boss` sprite generator is the project's prototype
for a **rich, multi-file character target** — the kind of authoring
surface that future bone-based / scenegraph-driven characters will
likely use. The detail decisions below will probably change once a
proper rig system lands, but the *shape* of the integration is
intended to generalize. Treat this as a worked example, not a
contract.

## When to author as a multi-file target

A character probably needs the multi-file layout when:

- The sprite has more than one independently-animated piece (head +
  body + hands for `gnu_ton_boss`, body + wings for
  `mockingbird_boss`).
- The pose system uses a scenegraph or rig that doesn't reduce
  cleanly to a single frame-grid per animation row.
- A part editor or tuning UI lives next to the renderer so authors
  can iterate on the rig without re-running the full sheet build.

Single-file tack-on targets (`targets/characters/<name>.py`) stay
the right answer for everything else.

## File layout

```
tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/
    targets/characters/<character_name>/
        __init__.py                    # tack-on API: render() + install()
        sprite_generator.py            # the actual renderer
        part_editor.py                 # optional rig-tuning UI
        <character>_scene.yaml         # scene composition
        <character>_parts.yaml         # part definitions
        <character>_legacy_parts.yaml  # frozen pre-rewrite snapshot (optional)
```

The runtime asset layout matches:

```
crates/ambition_gameplay_core/assets/sprites/<character_name>/
    <character_name>_spritesheet.png
    <character_name>_spritesheet.ron       # consumed by `SheetRegistry`; canonical runtime metadata
    <character_name>_actor.ron             # actor catalog sidecar
    <character_name>_canonical.png         # auto-crop reference
    <character_name>_canonical_transparent.png
    <character_name>_preview_labeled.png

Generated-only review images may also be written under
`tools/ambition_sprite2d_renderer/generated/<character_name>/`. For example,
`gnu_ton_boss_hitboxes_debug.png` overlays the exact `body_metrics` hit/hurt
boxes that GNU-ton emits into its `.ron` file.
```

## The tack-on API the multi-file target exposes

`__init__.py` is the seam the rest of the renderer interacts with.
Four pieces:

```python
from . import sprite_generator

TARGET_NAME = sprite_generator.TARGET_NAME
SHEET_FILES = list(sprite_generator.OUTPUT_FILES)


def render(out_dir, **opts) -> list[Path]:
    """Build every output file in ``out_dir``. Returns absolute paths."""
    ...


def install(render_dir, dest_root) -> list[Path]:
    """Copy the runtime-needed subset of files into
    ``<dest_root>/<TARGET_NAME>/``. Multi-file targets stage into a
    subdir; single-file tack-ons stage at the dest root."""
    ...
```

The `target_registry.discover_tackon_targets()` walker picks up the
package via the `render` function, and `publish_catalog_sprites.py`
routes catalog entries whose `spritesheet:` field points into
`sprites/<TARGET_NAME>/` to the unified `publish <target>` command.

## Catalog wiring

A multi-file character's `character_catalog.ron` entry uses the
subdir path:

```ron
"npc_mockingbird_boss": (
    display_name: "Mockingbird Boss",
    spritesheet: "sprites/mockingbird_boss/mockingbird_boss_spritesheet.png",
    manifest: "sprites/mockingbird_boss/mockingbird_boss_spritesheet.ron",
    tier: Basement,
    body_kind: Wide,
    ...
),
```

The runtime's `record_index()` scans one level deep into subdirs
(see `character_sprites/sheets/mod.rs::record_index`) so
the manifest reaches the catalog → sprite chain without special-
casing each multi-file target by name.

## Open questions for the future rig system

These are intentionally unresolved — write down what felt awkward
about the current pattern, not what to do about it yet:

- **Scene / parts YAML schema** — `mockingbird_boss_scene.yaml` and
  `mockingbird_boss_parts.yaml` evolved organically. A proper rig
  system would have a typed schema (probably Rust-backed) so the
  scenegraph can be validated at author time.
- **Part editor integration** — `part_editor.py` is a Tk app that
  edits the YAML directly. A real authoring tool would round-trip
  through the engine's runtime so iteration is WYSIWYG.
- **Animation rows vs scene takes** — the current setup builds a
  rectangular spritesheet (rows = animations, columns = frames).
  A bone-based system might prefer per-animation atlases or
  skeleton-keyed sparse storage; the manifest schema would need to
  grow.
- **Tuning** — `sheet_tuning` (collision_scale + frame_sample_inset,
  per ADR 0017's V3/D4 migration) is per-target. Multi-file
  characters might want per-part tuning (the head's collision is
  smaller than the body's), which the current `SheetTuningSpec`
  doesn't model.

## Cross-references

- [`docs/systems/sprite-rendering-surface.md`](../systems/sprite-rendering-surface.md)
  — three publish patterns + unification plan.
- [`docs/systems/character-catalog.md`](../systems/character-catalog.md)
  — how characters reach the runtime.
- [`docs/adr/0017-rust-behavior-ron-content-ldtk-space.md`](../adr/0017-rust-behavior-ron-content-ldtk-space.md)
  — the architectural posture that makes "renderer-only authoring"
  the goal for new characters.
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/mockingbird_boss/`
  — the live reference.
