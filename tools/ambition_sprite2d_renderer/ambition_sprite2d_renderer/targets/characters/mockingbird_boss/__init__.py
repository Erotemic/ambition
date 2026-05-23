"""Mockingbird boss character — multi-file tack-on target.

A multi-file character package: the boss ships a manifest + per-part
frames assembled by :mod:`.sprite_generator`, with part-config YAML
files alongside the renderer (`mockingbird_boss_parts.yaml`,
`mockingbird_boss_scene.yaml`, `mockingbird_boss_legacy_parts.yaml`)
and a Tk-based part editor (:mod:`.part_editor`) for tuning the rig.

The package layout exists so adding the next multi-file character is a
copy-this-directory operation: drop ``targets/characters/<name>/`` with
the same ``__init__.py`` shape, and discovery picks it up.
"""
from __future__ import annotations

from pathlib import Path
from typing import List

from . import sprite_generator

TARGET_NAME = sprite_generator.TARGET_NAME
SHEET_FILES = list(sprite_generator.OUTPUT_FILES)


def render(out_dir: str | Path, **opts) -> List[Path]:
    return list(
        sprite_generator.render_outputs(
            outdir=out_dir,
            quick=bool(opts.get("quick", False)),
        )
    )


def install(render_dir: str | Path, dest_root: str | Path) -> List[Path]:
    return list(
        sprite_generator.install_outputs(
            render_dir=render_dir,
            install_dir=Path(dest_root) / TARGET_NAME,
        )
    )
