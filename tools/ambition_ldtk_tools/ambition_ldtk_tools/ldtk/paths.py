"""Canonical repository paths and image helpers for LDtk tooling.

Ambition keeps durable game content under ``game/ambition_content/assets`` and
publishes generated sprites through its selected content library. Read accepted
products with ``published_runtime_files``; use ``default_sprite_assets_dir``
for loose generator output.
"""

from __future__ import annotations

import os
import struct
import sys
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


def game_worlds_dir(game: str, anchor: Path | None = None) -> Path:
    """Return the game's authoring worlds or its served LDtk worlds.

    A game that compiles LDtk into room data keeps its source in `authoring`.
    A game that loads LDtk through the platform asset reader uses `assets`.
    """
    root = _repo_root(anchor) / "game" / game
    authored = root / "authoring" / "worlds"
    runtime = root / "assets" / "worlds"
    return authored if authored.is_dir() or not runtime.is_dir() else runtime


def game_world_ldtk(game: str, stem: str, anchor: Path | None = None) -> Path:
    """Return one game-owned world file, e.g. ``mary_o.ldtk``."""
    return game_worlds_dir(game, anchor) / f"{stem}.ldtk"


def game_entity_manifest(game: str, stem: str, anchor: Path | None = None) -> Path:
    """Return the sidecar manifest declaring a game-owned world's own entities.

    The LDtk validator reads this to know which identifiers a game installs
    converters for; see ``validate.game_entity_manifest_for``.
    """
    return game_worlds_dir(game, anchor) / f"{stem}.entities.json"


def default_entity_contract(anchor: Path | None = None) -> Path:
    """Return the LDtk authoring contract the Rust converters PROVE.

    ⭐ **This is the one file that stops the Python loop reporting green on
    content the runtime refuses.** It lives beside the crate that enforces it —
    `crates/ambition_platformer2d_ldtk` — because that crate's `contract::prover`
    runs every claim in it against the real converters, in both directions. A
    copy kept here would be a second authority, which is the whole defect.
    """
    return (
        _repo_root(anchor)
        / "crates"
        / "ambition_platformer2d_ldtk"
        / "ldtk_entity_contract.json"
    )


def default_character_catalog(anchor: Path | None = None) -> Path:
    """Return the authoritative character-catalog path."""
    return default_content_assets_dir(anchor) / "data" / "character_catalog.ron"


def default_sprite_assets_dir(anchor: Path | None = None) -> Path:
    """Return the selected library's loose sprite output directory."""
    return _library_tools().generated_root(_repo_root(anchor)) / "sprites"


def _library_tools():
    root = str(_repo_root())
    if root not in sys.path:
        sys.path.insert(0, root)
    from scripts.lib import content_libraries
    return content_libraries


def published_runtime_files(anchor: Path | None = None) -> dict[str, Path]:
    """Read accepted products, including their qualified library references."""
    tools = _library_tools()
    libraries = tools.read_libraries([tools.selected_manifest(_repo_root(anchor))])
    files: dict[str, Path] = {}
    for name, library in libraries.items():
        for logical, path in library.files().items():
            files[logical] = path
            files[f"content://{name}/{logical}"] = path
    return files


def _as_posix_relpath(path: Path, start: Path) -> str:
    return str(Path(os.path.relpath(path, start))).replace("\\", "/")


def _in_repo_spelling(path: Path, root: Path) -> Path:
    """``path`` under the resolved repo root ``root``, keeping each symlink
    inside the repo as it is spelled.

    A checkout can be reached through an alias (a symlink to the repo). The
    repo root is resolved, so a path through the alias must be re-rooted, or a
    relative path between the two spellings climbs to ``/``.
    """
    for parent in [path, *path.parents]:
        try:
            same = parent.resolve() == root
        except OSError:
            continue
        if same:
            return root / path.relative_to(parent)
    return path


def rel_to_ldtk(ldtk: Path, path: Path) -> str:
    """Return the runtime-safe LDtk path for ``path``.

    Authored worlds are loaded through the ``game://`` source rooted at
    ``game/ambition_content/assets``. That source falls back to the shared
    ``game/ambition_content_library/generated/runtime`` tree, so generated sprites must be
    addressed through the virtual ``game://sprites`` mount rather than by a
    filesystem traversal into another crate. Bevy rejects those ``../../..``
    traversals before the fallback reader gets a chance to resolve them.
    """
    # Use lexical absolute paths here. ``game/ambition_content/assets/sprites``
    # may itself be a symlink in a developer checkout; resolving it before we
    # recognize the virtual mount would erase the ``game://sprites`` identity.
    root = _repo_root(Path(os.path.abspath(ldtk)))
    ldtk = _in_repo_spelling(Path(os.path.abspath(ldtk)), root)
    path = _in_repo_spelling(Path(os.path.abspath(path)), root)
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
    logical = "sprites/" + sprite_rel.as_posix()
    return published_runtime_files(ldtk).get(logical, default_sprite_assets_dir(ldtk) / sprite_rel).absolute()


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
