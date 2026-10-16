"""The publish re-applies the sprite tileset to each LDtk world, and the
worlds are the pack's own list (`worlds.ron`).

A hand list in `scripts/regen/sprites.sh` left out the Math world when it
was added, so its tileset def kept a sheet height the renderer no longer
published (3774 against 3769, found 2026-10-15)."""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
REGEN = REPO / "scripts/regen/sprites.sh"
CONTENT = REPO / "game/ambition_content/assets"


def _ldtk_world_files() -> list[str]:
    text = REGEN.read_text()
    function = re.search(r"^ldtk_world_files\(\) \{\n.*?^\}\n", text, re.S | re.M).group(0)
    script = f"content_assets_dir={CONTENT}\n{function}ldtk_world_files\n"
    return subprocess.run(["bash", "-c", script], capture_output=True, text=True, check=True).stdout.split()


def test_the_tileset_phase_applies_to_every_world_the_game_ships() -> None:
    shipped = sorted(path.name for path in (CONTENT / "worlds").glob("*.ldtk"))
    assert shipped, "no world files: the fixture is not the shipped tree"
    assert sorted(_ldtk_world_files()) == shipped
    assert "for world in $(ldtk_world_files); do" in REGEN.read_text()
