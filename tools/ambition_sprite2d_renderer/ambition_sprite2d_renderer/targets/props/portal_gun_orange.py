from __future__ import annotations

"""Portal gun (orange mode) — see `_portal_gun_art` for the shared geometry."""

from pathlib import Path
from typing import List

from . import _portal_gun_art as art

TARGET_NAME = "portal_gun_orange"
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
        glow=(255, 198, 138, 255),
        core=(255, 148, 22, 255),
        accent=(255, 150, 58, 255),
        actor_id="prop_portal_gun_orange",
        display="Portal Gun (Orange)",
    )
