"""Canonical repository paths and image helpers for LDtk tooling.

Ambition keeps durable game content under ``game/ambition_content/assets`` and
publishes generated character sprite products under
``crates/ambition_actors/assets/sprites``.  Keep that split explicit here so
individual commands do not recreate stale repository-layout assumptions.
"""

from __future__ import annotations

import os
import struct
from pathlib import Path


def repo_root_from_ldtk(ldtk: Path) -> Path:
    """Best-effort Ambition repo root discovery from an LDtk or repo path."""
    p = ldtk.resolve()
    if p.is_file():
        p = p.parent
    for parent in [p, *p.parents]:
        if (parent / "crates").exists() and (parent / "tools").exists():
            return parent
    return Path.cwd().resolve()


def _repo_root(anchor: Path | None = None) -> Path:
    if anchor is not None:
        return repo_root_from_ldtk(anchor)
    # ldtk/paths.py -> ambition_ldtk_tools -> tools/ambition_ldtk_tools -> tools -> repo
    return Path(__file__).resolve().parents[4]


def default_content_assets_dir(anchor: Path | None = None) -> Path:
    """Return the authoritative durable-content asset directory."""
    return _repo_root(anchor) / "game" / "ambition_content" / "assets"


def default_worlds_dir(anchor: Path | None = None) -> Path:
    """Return the authoritative LDtk world directory."""
    return default_content_assets_dir(anchor) / "worlds"


def default_sandbox_ldtk(anchor: Path | None = None) -> Path:
    """Return the authoritative sandbox LDtk path for this checkout."""
    return default_worlds_dir(anchor) / "sandbox.ldtk"


def default_hall_ldtk(anchor: Path | None = None) -> Path:
    """Return the generated Hall-of-Characters LDtk path."""
    return default_worlds_dir(anchor) / "hall_of_characters.ldtk"


def default_character_catalog(anchor: Path | None = None) -> Path:
    """Return the authoritative character-catalog path."""
    return default_content_assets_dir(anchor) / "data" / "character_catalog.ron"


def default_sprite_assets_dir(anchor: Path | None = None) -> Path:
    """Return the published generated character-sprite directory."""
    return (
        _repo_root(anchor)
        / "crates"
        / "ambition_actors"
        / "assets"
        / "sprites"
    )


def _as_posix_relpath(path: Path, start: Path) -> str:
    return str(Path(os.path.relpath(path, start))).replace("\\", "/")


def rel_to_ldtk(ldtk: Path, path: Path) -> str:
    """Return the runtime-safe LDtk path for ``path``.

    Authored worlds are loaded through the ``game://`` source rooted at
    ``game/ambition_content/assets``. That source falls back to the shared
    ``crates/ambition_actors/assets`` tree, so generated sprites must be
    addressed through the virtual ``game://sprites`` mount rather than by a
    filesystem traversal into another crate. Bevy rejects those ``../../..``
    traversals before the fallback reader gets a chance to resolve them.
    """
    # Use lexical absolute paths here. ``game/ambition_content/assets/sprites``
    # may itself be a symlink in a developer checkout; resolving it before we
    # recognize the virtual mount would erase the ``game://sprites`` identity.
    ldtk = Path(os.path.abspath(ldtk))
    path = Path(os.path.abspath(path))
    shared_sprites = Path(os.path.abspath(default_sprite_assets_dir(ldtk)))
    try:
        sprite_rel = path.relative_to(shared_sprites)
    except ValueError:
        pass
    else:
        virtual_path = (
            Path(os.path.abspath(default_content_assets_dir(ldtk)))
            / "sprites"
            / sprite_rel
        )
        return _as_posix_relpath(virtual_path, ldtk.parent)
    return _as_posix_relpath(path, ldtk.parent)


def path_from_ldtk(ldtk: Path, rel: str) -> Path:
    """Resolve an LDtk path through the same virtual fallback as ``game://``."""
    ldtk = Path(os.path.abspath(ldtk))
    direct = Path(os.path.abspath(ldtk.parent / rel))
    virtual_sprites = Path(os.path.abspath(default_content_assets_dir(ldtk))) / "sprites"
    try:
        sprite_rel = direct.relative_to(virtual_sprites)
    except ValueError:
        return direct
    return Path(os.path.abspath(default_sprite_assets_dir(ldtk) / sprite_rel))


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
