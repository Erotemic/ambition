"""GNU Ton boss character — multi-file tack-on target.

A multi-file character package. Same shape as
[`mockingbird_boss`](../mockingbird_boss/__init__.py): the package
ships its own :mod:`.sprite_generator` and the ``__init__.py``
exposes the discovery API on top of it.
"""

from __future__ import annotations

from pathlib import Path
from typing import List

from . import sprite_generator

TARGET_NAME = sprite_generator.TARGET_NAME
SHEET_FILES = list(sprite_generator.OUTPUT_FILES)
ACTOR_METADATA = sprite_generator.ACTOR_METADATA


def render(out_dir: str | Path, **opts) -> List[Path]:
    return list(
        sprite_generator.render_outputs(
            outdir=Path(out_dir),
            quick=bool(opts.get("quick", False)),
        )
    )


def install(render_dir: str | Path, dest_root: str | Path) -> List[Path]:
    render_dir = Path(render_dir)
    install_dir = Path(dest_root) / TARGET_NAME
    return list(
        sprite_generator.install_outputs(
            render_dir=render_dir,
            install_dir=install_dir,
        )
    )
