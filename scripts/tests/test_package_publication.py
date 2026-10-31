"""Packages use accepted sprites and omit old loose files."""
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import package_asset_guard as guard
from lib.art_publication import Store


def test_a_pack_source_keeps_its_scope_when_a_library_has_the_same_name(tmp_path, monkeypatch):
    pack = tmp_path / "game" / "demo" / "assets"
    (pack / "audio").mkdir(parents=True)
    (pack / "pack.ron").write_text("()")
    (pack / "audio/sfx_registry.ron").write_text("game cues")
    library = tmp_path / "library"
    (library / "runtime/audio").mkdir(parents=True)
    (library / "runtime/audio/sfx_registry.ron").write_text("library cues")
    manifest = library / "library.json"
    manifest.write_text('{"schema":"ambition.content_library.v1","id":"shared","version":"1","asset_roots":{"authored":"runtime"}}')
    monkeypatch.setattr(guard, "read_pack_inputs", lambda *_: guard.PackInputs(
        {"audio/sfx_registry.ron"}, {}, [manifest]))
    contract, _ = guard.build_contract(tmp_path, "web", pack_root=pack)
    assert guard.PACK_SOURCE_ORIGIN in contract.entries["audio/sfx_registry.ron"].origins
    assert guard.PACK_SOURCE_ORIGIN not in contract.entries["__libraries/shared/audio/sfx_registry.ron"].origins


def test_a_selected_game_packages_its_own_source_declarations(tmp_path, monkeypatch):
    pack = tmp_path / "game" / "demo" / "assets"
    pack.mkdir(parents=True)
    (pack / "own.png").write_bytes(b"image")
    source = pack.parent / "src"
    source.mkdir()
    (source / "lib.rs").write_text('const OWN_ASSET: &str = "own.png";')
    other = tmp_path / "game" / "other" / "src"
    other.mkdir(parents=True)
    (other / "lib.rs").write_text('const OTHER_ASSET: &str = "unrelated.png";')
    monkeypatch.setattr(guard, "read_pack_inputs", lambda *_: None)
    contract, _ = guard.build_contract(tmp_path, "web", pack_root=pack)
    assert set(contract.entries) == {"own.png"}


def test_a_package_uses_the_selected_bundle_and_excludes_old_loose_pages(tmp_path, monkeypatch):
    monkeypatch.delenv("AMBITION_ART_STORE", raising=False)
    (tmp_path / guard.CONTENT_ASSET_ROOT).mkdir(parents=True)
    library = tmp_path / "library"
    library.mkdir()
    manifest = library / "library.json"
    manifest.write_text('{"schema":"ambition.content_library.v1","id":"shared","version":"1","asset_roots":{"generated":"generated/runtime"},"publication":"generated/art_publication"}')
    monkeypatch.setattr(guard, "read_pack_inputs", lambda *_: guard.PackInputs(set(), {}, [manifest]))
    assets = library / "generated/runtime"
    (assets / "sprites").mkdir(parents=True)
    (assets / "sprites/body.png").write_bytes(b"old pixels")
    (assets / "sprites/body.9.png").write_bytes(b"retired page")
    source = tmp_path / "new.png"
    source.write_bytes(b"accepted pixels")
    store = Store(assets.parent / "art_publication")
    key = store.accept("target:body", {"sprites/body.png": (source, "runtime", "full")}, None)
    store.publish({"target:body": key}, None)
    files = guard.collect_source_files(tmp_path, "web")
    assert set(files) == {"__libraries/shared/sprites/body.png"}
    assert files["__libraries/shared/sprites/body.png"].source.read_bytes() == b"accepted pixels"
    out = tmp_path / "package"
    guard.materialize_composed_tree(files, out, mode="copy")
    assert (out / "__libraries/shared/sprites/body.png").read_bytes() == b"accepted pixels"
    assert not (out / "__libraries/shared/sprites/body.9.png").exists()
