"""Read the renderer publication selected by a library manifest."""
from pathlib import Path
import json
import os
import sys

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "tools/ambition_sprite2d_renderer"))
from ambition_sprite2d_renderer.publication_store import Store, MANAGED_ROOTS, relative


def store(manifest: Path) -> Store | None:
    data = json.loads(manifest.read_text())
    declared = data.get("publication")
    return Store(Path(os.environ.get("AMBITION_ART_STORE", manifest.parent / relative(declared)))) if declared else None


def files(manifest: Path) -> dict[str, Path]:
    publication = store(manifest)
    return publication.paths() if publication else {}


def resolve(manifest: Path, logical: str) -> Path | None:
    data = json.loads(manifest.read_text())
    publication = store(manifest)
    if publication:
        selected = publication.selected()
        if logical.split("/", 1)[0] in set(MANAGED_ROOTS) | set(selected.get("roots", [])):
            item = selected["files"].get(logical)
            return publication.root / item["path"] if item else None
    candidates = [manifest.parent / relative(root) / relative(logical) for root in data["asset_roots"].values()]
    existing = [path for path in candidates if path.is_file()]
    if len(existing) > 1:
        raise ValueError(f"The library declares {logical!r} twice: {existing}")
    return existing[0] if existing else None
