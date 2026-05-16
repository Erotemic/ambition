from __future__ import annotations

"""Filesystem locations used by the sprite renderer package.

Keep path discovery here so renderers, legacy wrappers, and CLI commands agree
on where generated assets and sandbox sprite installs live.
"""

from pathlib import Path


def package_root() -> Path:
    return Path(__file__).resolve().parent


def tool_root() -> Path:
    # tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/paths.py
    return package_root().parent


def repo_root(start: str | Path | None = None) -> Path:
    start_path = Path(start).resolve() if start is not None else tool_root().resolve()
    if start_path.is_file():
        start_path = start_path.parent
    for path in [start_path, *start_path.parents]:
        if (path / "crates" / "ambition_sandbox").exists() and (path / "tools" / "ambition_sprite2d_renderer").exists():
            return path
    # Repository-layout fallback for source checkouts.
    return tool_root().parents[1]


def generated_root() -> Path:
    return tool_root() / "generated"


def sandbox_sprites_dir() -> Path:
    return repo_root() / "crates" / "ambition_sandbox" / "assets" / "sprites"
