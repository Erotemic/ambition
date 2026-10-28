"""Packages use accepted sprites and omit old loose files."""
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import package_asset_guard as guard
from lib.art_publication import Store


def test_a_package_uses_the_selected_bundle_and_excludes_old_loose_pages(tmp_path, monkeypatch):
    monkeypatch.delenv("AMBITION_ART_STORE", raising=False)
    (tmp_path / guard.CONTENT_ASSET_ROOT).mkdir(parents=True)
    assets = tmp_path / guard.ACTOR_ASSET_ROOT
    (assets / "sprites").mkdir(parents=True)
    (assets / "sprites/body.png").write_bytes(b"old pixels")
    (assets / "sprites/body.9.png").write_bytes(b"retired page")
    source = tmp_path / "new.png"
    source.write_bytes(b"accepted pixels")
    store = Store(assets.parent / "art_publication")
    key = store.accept("target:body", {"sprites/body.png": (source, "runtime", "full")}, None)
    store.publish({"target:body": key}, None)
    files = guard.collect_source_files(tmp_path, "web")
    assert set(files) == {"sprites/body.png"}
    assert files["sprites/body.png"].source.read_bytes() == b"accepted pixels"
    out = tmp_path / "package"
    guard.materialize_composed_tree(files, out, mode="copy")
    assert (out / "sprites/body.png").read_bytes() == b"accepted pixels"
    assert not (out / "sprites/body.9.png").exists()
