"""Save a music review from the page through the music tool's own review owner.

The review rates one exact render version: its render hash and the bytes of its
preview. The page may only rate a version that its index offered, and only if
the preview bytes are unchanged since the index was built.
"""

from __future__ import annotations

from pathlib import Path

from .kinds import music_reviews_module


def save(repo: Path, catalog: dict, data: dict) -> dict:
    item = (catalog.get("items") or {}).get(str(data.get("item", "")))
    if not item or item["kind"] != "music":
        raise ValueError("Name a music item of this index.")
    target = next((t for t in item.get("review_targets", []) if t["version_id"] == data.get("version_id")), None)
    if not target:
        raise ValueError("The index does not offer this render version for review. Rebuild the inspector.")
    tool = repo / item["details"]["music_tool"]
    module = music_reviews_module(tool)
    version = next((v for v in module.discover_render_versions(tool, include_agent_bundles=False)
                    if v.cue_id == item["details"]["folder"] and v.render_hash == target["render_hash"]), None)
    if version is None or version.preview_sha256 != target["preview_sha256"]:
        raise LookupError("This render changed or is gone. Reload the inspector before reviewing it.")
    document = module.ReviewStore(tool).save(
        version, score=float(data["score"]), notes=str(data.get("notes", "")), issues=list(data.get("issues") or []),
        furthest_played_seconds=float(data.get("furthest_played_seconds") or 0.0),
        furthest_played_fraction=float(data.get("furthest_played_fraction") or 0.0))
    current = document.data["current"]
    return {"item": item["id"], "version_id": version.version_id, "path": str(document.path.relative_to(repo)),
            "review": {"score": current["score"], "label": current["label"], "notes": current["notes"], "issues": current["issues"],
                       "reviewed_at": current["reviewed_at"], "version": version.version_id,
                       "score_sha256": document.data["subject"].get("score_sha256_at_review")}}
