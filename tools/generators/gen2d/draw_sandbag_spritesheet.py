#!/usr/bin/env python3
"""Compatibility shim for the legacy sandbag generator.

The sandbag renderer moved to ``tools/ambition_sprite2d_renderer`` as a
proper target. Prefer:

    python -m ambition_sprite2d_renderer render sandbag
    python -m ambition_sprite2d_renderer render-publish sandbag
"""
from __future__ import annotations

import os
import sys
from pathlib import Path


def main(argv: list[str] | None = None) -> int:
    print(
        "[deprecated] tools/generators/gen2d/draw_sandbag_spritesheet.py — use "
        "`python -m ambition_sprite2d_renderer render sandbag` (add `-publish` "
        "to also install) instead.",
        file=sys.stderr,
    )
    if argv is None:
        argv = sys.argv[1:]

    legacy_aliases = "--legacy-aliases" in argv
    copy_to_sandbox = "--copy-to-sandbox" in argv
    sub = "render-publish" if copy_to_sandbox else "render"
    forward = ["sandbag"]
    if legacy_aliases:
        forward.append("--legacy-aliases")

    repo_root = Path(__file__).resolve().parents[2]
    pkg_root = repo_root / "tools" / "ambition_sprite2d_renderer"
    venv_python = (
        repo_root / "tools" / "generators" / "gen2d" / ".venv" / "bin" / "python"
    )
    python = str(venv_python) if venv_python.exists() else sys.executable
    cmd = [python, "-m", "ambition_sprite2d_renderer", sub, *forward]
    env = dict(os.environ)
    env["PYTHONPATH"] = str(pkg_root) + os.pathsep + env.get("PYTHONPATH", "")
    os.execvpe(cmd[0], cmd, env)
    return 0  # pragma: no cover


if __name__ == "__main__":
    raise SystemExit(main())
