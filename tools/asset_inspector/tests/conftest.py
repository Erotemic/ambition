"""Explicit library selection for synthetic inspector repositories."""
import json
import pytest


@pytest.fixture(autouse=True)
def selected_library(tmp_path, monkeypatch):
    monkeypatch.delenv("AMBITION_CONTENT_LIBRARY", raising=False)
    root = tmp_path / "game/ambition_content_library"
    root.mkdir(parents=True, exist_ok=True)
    (root / "library.json").write_text(json.dumps({
        "schema":"ambition.content_library.v1", "id":"fixture", "version":"1",
        "asset_roots":{"generated":"generated/runtime"},
        "publication":"generated/art_publication", "provenance":"generated/asset_provenance"
    }))
