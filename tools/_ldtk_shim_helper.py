"""Helper for LDtk legacy script shims.

The legacy LDtk scripts at ``tools/<name>.py`` moved into the
``ambition_ldtk_tools`` package. Each shim forwards its argv to the
appropriate subcommand and prints a deprecation note.
"""
from __future__ import annotations

import os
import sys
from pathlib import Path


def forward(subcommand: list[str], deprecation_note: str) -> int:
    print(f"[deprecated] {deprecation_note}", file=sys.stderr)
    repo_root = Path(__file__).resolve().parents[1]
    pkg_root = repo_root / "tools" / "ambition_ldtk_tools"
    cmd = [sys.executable, "-m", "ambition_ldtk_tools", *subcommand, *sys.argv[1:]]
    env = dict(os.environ)
    env["PYTHONPATH"] = str(pkg_root) + os.pathsep + env.get("PYTHONPATH", "")
    os.execvpe(cmd[0], cmd, env)
    return 0  # pragma: no cover
