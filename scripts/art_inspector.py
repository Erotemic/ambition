#!/usr/bin/env python3
"""Build the offline art inspector: every art file, the generator that made it
and where the game uses it (`tools/art_inspector/README.md`).

    python3 scripts/art_inspector.py build [--open] [--jobs N] [--no-previews] [--prune]

Writes `.agent/art_inspector/index.html` (git-ignored). Needs Pillow and
PyYAML; without them it runs itself in the sprite renderer's interpreter.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOOL = ROOT / "tools" / "art_inspector"


def _has_dependencies() -> bool:
    try:
        import PIL  # noqa: F401
        import yaml  # noqa: F401
    except ImportError:
        return False
    return True


def main() -> int:
    if not _has_dependencies() and not os.environ.get("ART_INSPECTOR_REEXEC"):
        done = subprocess.run(
            ["bash", "-c", 'source "$1/scripts/lib/tool_python.sh"; ambition_select_tool_python "$1/tools/ambition_sprite2d_renderer" AMBITION_SPRITE_PYTHON', "_", str(ROOT)],
            capture_output=True, text=True,
        )
        python = done.stdout.strip()
        if not python:
            print("art_inspector: needs Pillow and PyYAML, and found no sprite renderer interpreter", file=sys.stderr)
            return 2
        env = dict(os.environ, ART_INSPECTOR_REEXEC="1")
        return subprocess.call([python, __file__, *sys.argv[1:]], env=env)
    sys.path.insert(0, str(TOOL))
    from art_index.build import main as build_main

    return build_main(sys.argv[1:])


if __name__ == "__main__":
    sys.exit(main())
