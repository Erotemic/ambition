"""Where the art is: the roots the index scans, by family and quality tier."""

from __future__ import annotations

import os
import json
from dataclasses import dataclass
from pathlib import Path


def repo_root() -> Path:
    override = os.environ.get("AMBITION_REPO_ROOT")
    if override:
        return Path(override).resolve()
    return Path(__file__).resolve().parents[3]


@dataclass(frozen=True)
class Root:
    """One scanned directory. `name` is the path the generators' records use
    for it (`sprites`, `backgrounds/parallax_layers`); `tier` names the
    quality tier of a reduced copy and `of` the root it is a copy of."""

    name: str
    path: Path
    tier: str = "full"
    of: str | None = None
    #: `generated` (made by a generator) or `reference` (authored or imported
    #: art that no generator makes).
    origin: str = "generated"


def library_path(repo: Path) -> Path:
    return repo / os.environ.get("AMBITION_CONTENT_LIBRARY", "game/ambition_content_library/library.json")


_LIBRARY: dict[str, tuple] = {}


def library(repo: Path) -> dict:
    """The library manifest, parsed once per file version (a build asks thousands of times)."""
    path = library_path(repo)
    st = path.stat()
    stamp = (st.st_size, st.st_mtime_ns)
    cached = _LIBRARY.get(str(path))
    if cached is None or cached[0] != stamp:
        cached = _LIBRARY[str(path)] = (stamp, path.read_text())
    return json.loads(cached[1])


def asset_root(repo: Path) -> Path:
    return library_path(repo).parent / library(repo)["asset_roots"]["generated"]


def provenance_root(repo: Path) -> Path | None:
    path = library(repo).get("provenance")
    return library_path(repo).parent / path if path else None


TIERS = {"_0_5x": "half", "_0_25x": "quarter", "_potato": "potato"}


_ROOTS: dict[tuple, list] = {}


_DIRS: dict[str, str] = {}


def forget_roots() -> None:
    """Drop the roots and resolved directories read for an earlier build in this process."""
    _ROOTS.clear()
    _DIRS.clear()


def real(path: Path) -> Path:
    """`path.resolve()`, resolving each directory once per build: on a shared
    filesystem every path component's lstat is a round trip, and a build
    resolves tens of thousands of paths in a few hundred directories."""
    path = os.path.abspath(path)
    parent, name = os.path.split(path)
    real_parent = _DIRS.get(parent)
    if real_parent is None:
        real_parent = _DIRS[parent] = os.path.realpath(parent)
    candidate = os.path.join(real_parent, name)
    return Path(os.path.realpath(candidate) if os.path.islink(candidate) else candidate)


def roots(repo: Path) -> list[Root]:
    """The scanned roots, read once per build (`forget_roots` starts a build)."""
    key = (str(repo), str(library_path(repo)))
    if key not in _ROOTS:
        _ROOTS[key] = _read_roots(repo)
    return list(_ROOTS[key])


def _read_roots(repo: Path) -> list[Root]:
    a = asset_root(repo)
    import sys
    sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts"))
    from lib.art_publication import files as art_files
    out = [Root("sprites", a / "sprites", origin="unmanaged")]
    out += [Root("sprites"+suffix, a / ("sprites"+suffix), tier, "sprites", "unmanaged") for suffix, tier in TIERS.items()]
    out.append(Root("sprite_packs", a / "sprite_packs", origin="unmanaged"))
    published = art_files(library_path(repo))
    managed = {"sprites", "sprite_packs"} | {"sprites" + suffix for suffix in TIERS}
    for name in sorted({logical.split("/", 1)[0] for logical in published} - managed):
        for base in (a, repo / "game/ambition_content/assets"):
            out.append(Root(name, base / name, origin="unmanaged"))
    seen = set()
    for logical, physical in published.items():
        name, rel = logical.split("/", 1)
        base = physical
        for _ in Path(rel).parts:
            base = base.parent
        if (name, base) in seen:
            continue
        seen.add((name, base))
        suffix = next((suffix for suffix in TIERS if name == "sprites" + suffix), None)
        out.append(Root(name, base, TIERS[suffix] if suffix else "full", "sprites" if suffix else None))
    out.append(Root("backgrounds/parallax_layers", a / "backgrounds" / "parallax_layers"))
    out += [
        Root(f"backgrounds/parallax_layers{suffix}", a / "backgrounds" / f"parallax_layers{suffix}", tier, "backgrounds/parallax_layers")
        for suffix, tier in TIERS.items()
    ]
    out.append(Root("backgrounds/sky_parts", a / "backgrounds" / "sky_parts"))
    out.append(Root("room_dressing", a / "room_dressing"))
    # The placeholder profiles of `tools/ambition_background_renderer`.
    for profile in sorted(p.name for p in (a / "backgrounds").iterdir() if p.is_dir()) if (a / "backgrounds").is_dir() else []:
        if not profile.startswith(("parallax_layers", "sky_parts")):
            out.append(Root(f"backgrounds/{profile}", a / "backgrounds" / profile))
    out.append(Root("assets", repo / "assets", origin="reference"))
    return out


#: The file kinds the index lists. Other files (`.gitignore`, `.partial`) are not art.
LISTED = {".png", ".webp", ".jpg", ".jpeg", ".gif", ".svg", ".ron", ".yaml", ".yml", ".json", ".txt", ".xcf"}
IMAGES = {".png", ".webp", ".jpg", ".jpeg", ".gif", ".svg"}
