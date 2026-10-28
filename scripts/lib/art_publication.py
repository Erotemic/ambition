"""Read the renderer's accepted publication from repository tools."""
from pathlib import Path
import os
import sys

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "tools/ambition_sprite2d_renderer"))
from ambition_sprite2d_renderer.publication_store import Store, MANAGED_ROOTS


def store(asset_root: Path) -> Store:
    return Store(Path(os.environ.get("AMBITION_ART_STORE", asset_root.parent / "art_publication")))


def files(asset_root: Path) -> dict[str, Path]:
    return store(asset_root).paths()


def resolve(asset_root: Path, logical: str) -> Path:
    publication = store(asset_root)
    selected = publication.selected()
    if logical.split("/", 1)[0] in set(MANAGED_ROOTS) | set(selected.get("roots", [])):
        item = selected["files"].get(logical)
        return publication.root / item["path"] if item else publication.root / "unpublished" / logical
    return asset_root / logical
