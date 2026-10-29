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
    candidates = []
    managed = False
    if publication:
        selected = publication.selected()
        managed = logical in selected["files"] or any(logical == prefix or logical.startswith(prefix + "/")
                      for prefix in set(MANAGED_ROOTS) | set(selected.get("roots", [])))
        if item := selected["files"].get(logical):
            candidates.append(publication.root / item["path"])
    for role, root in data["asset_roots"].items():
        if role != "authored" and managed:
            continue
        path = manifest.parent / relative(root) / relative(logical)
        if path.is_file():
            candidates.append(path)
    if len(candidates) > 1:
        raise ValueError(f"The library declares {logical!r} twice: {candidates}")
    return candidates[0] if candidates else None
