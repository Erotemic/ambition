#!/usr/bin/env python3
"""Compatibility shim for the legacy ``tools/audio/render_sandbox_music.py`` script.

The renderer moved to ``tools/ambition_music_renderer`` and exposes a modal
CLI. Prefer:

    python -m ambition_music_renderer sandbox render-publish

This shim forwards arguments through to that command and prints a short
deprecation note.
"""
from __future__ import annotations

import os
import sys
from pathlib import Path


def main(argv: list[str] | None = None) -> int:
    print(
        "[deprecated] tools/audio/render_sandbox_music.py — use "
        "`python -m ambition_music_renderer sandbox render-publish` instead.",
        file=sys.stderr,
    )
    repo_root = Path(__file__).resolve().parents[2]
    pkg_root = repo_root / "tools" / "ambition_music_renderer"
    venv_python = pkg_root / ".venv" / "bin" / "python"
    python = str(venv_python) if venv_python.exists() else sys.executable
    cmd = [python, "-m", "ambition_music_renderer", "sandbox", "render-publish"]
    if argv is None:
        argv = sys.argv[1:]
    cmd.extend(argv)
    env = dict(os.environ)
    env["PYTHONPATH"] = str(pkg_root) + os.pathsep + env.get("PYTHONPATH", "")
    os.execvpe(cmd[0], cmd, env)
    return 0  # pragma: no cover


if __name__ == "__main__":
    raise SystemExit(main())
