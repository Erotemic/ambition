#!/usr/bin/env python3
"""Generate and publish Ambition placeholder parallax background assets.

Run from any directory:

    python scripts/generate_background_assets.py

The script writes PNG layers into the repository's `assets/backgrounds` tree.
"""

from __future__ import annotations

import sys
from pathlib import Path


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def main() -> int:
    root = repo_root()
    tool_root = root / "tools" / "ambition_background_renderer"
    sys.path.insert(0, str(tool_root))

    from ambition_background_renderer.cli import main as renderer_main

    return renderer_main(
        ["--out", str(root / "assets" / "backgrounds"), "--profile", "all"]
    )


if __name__ == "__main__":
    raise SystemExit(main())
