from __future__ import annotations

"""Portal gun (blue mode) — see `_portal_gun_art` for the shared geometry."""

from pathlib import Path
from typing import List

from . import _portal_gun_art as art

TARGET_NAME = "portal_gun_blue"
SHEET_FILES = (
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
)


def render(out_dir: str | Path, **opts) -> List[Path]:
    del opts
    return art.build(
        out_dir,
        TARGET_NAME,
        glow=(150, 205, 255, 255),
        core=(39, 167, 255, 255),
        accent=(46, 174, 255, 255),
        actor_id="prop_portal_gun_blue",
        display="Portal Gun (Blue)",
    )
