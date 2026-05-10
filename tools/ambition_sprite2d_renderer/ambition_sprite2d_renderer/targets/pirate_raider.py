from __future__ import annotations

import subprocess
import sys
from pathlib import Path

TARGET_NAME = "pirate_raider"
SHEET_FILES = [f"{TARGET_NAME}_spritesheet.png", f"{TARGET_NAME}_spritesheet.yaml"]


def render(out_dir: str | Path, **opts):
    out_dir = Path(out_dir)
    script = Path(__file__).resolve().parents[2] / "render_pirate_spritesheets.py"
    cmd = [sys.executable, str(script), "--target", TARGET_NAME, "--out-root", str(out_dir.parent)]
    subprocess.run(cmd, check=True)
    return [out_dir.parent / TARGET_NAME / name for name in SHEET_FILES]
