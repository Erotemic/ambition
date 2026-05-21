from __future__ import annotations

from pathlib import Path

from ..pirates.render import render_target

TARGET_NAME = "pirate_corsair"
SHEET_FILES = [f"{TARGET_NAME}_spritesheet.png", f"{TARGET_NAME}_spritesheet.yaml"]


def render(out_dir: str | Path, **opts):
    out_dir = Path(out_dir)
    frame_size = opts.get("frame_size")
    outputs = render_target(TARGET_NAME, out_dir, frame_size=frame_size or (128, 128))
    return [outputs["spritesheet"], outputs["yaml"]]
