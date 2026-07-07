"""Path and image helpers for LDtk projects."""

from __future__ import annotations

import os
import struct
from pathlib import Path


def repo_root_from_ldtk(ldtk: Path) -> Path:
    """Best-effort Ambition repo root discovery from an LDtk file path."""
    p = ldtk.resolve()
    for parent in [p.parent, *p.parents]:
        if (parent / "crates").exists() and (parent / "tools").exists():
            return parent
    return Path.cwd().resolve()


def default_sandbox_ldtk(anchor: Path | None = None) -> Path:
    """Return the default sandbox LDtk path for the current repo checkout."""
    if anchor is None:
        # ldtk/paths.py -> ambition_ldtk_tools -> tools/ambition_ldtk_tools -> tools -> repo
        root = Path(__file__).resolve().parents[4]
    else:
        root = repo_root_from_ldtk(anchor)
    return root / "crates" / "ambition_actors" / "assets" / "ambition" / "worlds" / "sandbox.ldtk"


def rel_to_ldtk(ldtk: Path, path: Path) -> str:
    """Return a forward-slash relative path from the LDtk file to ``path``."""
    return str(Path(os.path.relpath(path.resolve(), ldtk.resolve().parent))).replace("\\", "/")


def path_from_ldtk(ldtk: Path, rel: str) -> Path:
    """Resolve an LDtk-relative path."""
    return (ldtk.resolve().parent / rel).resolve()


def png_dimensions(path: Path) -> tuple[int, int] | None:
    """Return PNG dimensions without depending on Pillow."""
    try:
        with path.open("rb") as fh:
            if fh.read(8) != b"\x89PNG\r\n\x1a\n":
                return None
            fh.read(8)  # IHDR length + tag
            return tuple(map(int, struct.unpack(">II", fh.read(8))))  # type: ignore[return-value]
    except OSError:
        return None
