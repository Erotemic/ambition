"""Where the art is: the roots the index scans, by family and quality tier."""

from __future__ import annotations

import os
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


def asset_root(repo: Path) -> Path:
    return repo / "crates" / "ambition_platformer2d_actor_monolith" / "assets"


def provenance_root(repo: Path) -> Path:
    return repo / "crates" / "ambition_platformer2d_actor_monolith" / "asset_provenance"


TIERS = {"_0_5x": "half", "_0_25x": "quarter", "_potato": "potato"}


def roots(repo: Path) -> list[Root]:
    a = asset_root(repo)
    out = [Root("sprites", a / "sprites")]
    out += [Root(f"sprites{suffix}", a / f"sprites{suffix}", tier, "sprites") for suffix, tier in TIERS.items()]
    out.append(Root("sprite_packs", a / "sprite_packs"))
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
