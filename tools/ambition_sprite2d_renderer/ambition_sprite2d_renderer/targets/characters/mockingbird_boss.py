from __future__ import annotations

from pathlib import Path
from typing import List

from ...mockingbird import sprite_generator

TARGET_NAME = sprite_generator.TARGET_NAME
SHEET_FILES = list(sprite_generator.OUTPUT_FILES)


def render(out_dir: str | Path, **opts) -> List[Path]:
    return list(sprite_generator.render_outputs(outdir=out_dir, quick=bool(opts.get("quick", False))))


def install(render_dir: str | Path, dest_root: str | Path) -> List[Path]:
    return list(
        sprite_generator.install_outputs(
            render_dir=render_dir,
            install_dir=Path(dest_root) / TARGET_NAME,
        )
    )
