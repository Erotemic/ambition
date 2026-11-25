"""Packages use the accepted publication."""
import json
from pathlib import Path
import sys

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from scripts.package import package_asset_guard as guard
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
        {"audio/sfx_registry.ron"}, {}, [manifest], [guard.LibraryPackageFile(
            "shared", "audio/sfx_registry.ron", "content://shared/audio/sfx_registry.ron",
            library / "runtime/audio/sfx_registry.ron",
        )]))
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


@pytest.mark.parametrize("profile", ["android", "steamdeck"])
def test_a_package_uses_the_selected_bundle_and_excludes_old_loose_pages(tmp_path, monkeypatch, profile):
    monkeypatch.delenv("AMBITION_ART_STORE", raising=False)
    (tmp_path / guard.CONTENT_ASSET_ROOT).mkdir(parents=True)
    library = tmp_path / "library"
    library.mkdir()
    manifest = library / "library.json"
    manifest.write_text('{"schema":"ambition.content_library.v1","id":"shared","version":"1","asset_roots":{"generated":"generated/runtime"},"publication":"generated/art_publication"}')
    assets = library / "generated/runtime"
    (assets / "sprites").mkdir(parents=True)
    (assets / "sprites/body.png").write_bytes(b"old pixels")
    (assets / "sprites/body.9.png").write_bytes(b"retired page")
    source = tmp_path / "new.png"
    source.write_bytes(b"accepted pixels")
    sheet = tmp_path / "body_spritesheet.ron"
    sheet.write_text('(image: "body.png")')
    store = Store(assets.parent / "art_publication")
    key = store.accept("target:body", {"sprites/body.png": (source, "runtime", "full")}, None)
    sheet_key = store.accept("target:sheet", {"sprites/body_spritesheet.ron": (sheet, "runtime", "full")}, None)
    store.publish({"target:body": key, "target:sheet": sheet_key}, None)
    selected = store.selected()["files"]
    inputs = guard.PackInputs(set(), {}, [manifest], [
        guard.LibraryPackageFile("shared", logical, f'content://shared/__art/{row["path"]}', store.root / row["path"])
        for logical, row in selected.items()
    ])
    packaged = "__libraries/shared/__art/" + selected["sprites/body.png"]["path"]
    inputs.assets[packaged] = {"probe:music_cue/selected"}
    monkeypatch.setattr(guard, "read_pack_inputs", lambda *_: inputs)
    contract, files = guard.build_contract(tmp_path, profile, strict_pack_assets=True)
    assert set(files) == {guard.normalize_package_path(file.address) for file in inputs.library_files}
    assert files[packaged].source.read_bytes() == b"accepted pixels"
    out = tmp_path / "package"
    guard.materialize_composed_tree(files, out, mode="copy")
    guard.audit_tree(contract, out, reject_extras=True)
    assert (out / packaged).read_bytes() == b"accepted pixels"
    assert not (out / "__libraries/shared/sprites/body.png").exists()
    assert not (out / "__libraries/shared/sprites/body.9.png").exists()


def test_published_music_products_resolve_from_the_manifest_directory(tmp_path, monkeypatch):
    (tmp_path / guard.CONTENT_ASSET_ROOT).mkdir(parents=True)
    library = tmp_path / "library"
    library.mkdir()
    manifest = library / "library.json"
    manifest.write_text('{"schema":"ambition.content_library.v1","id":"shared","version":"1",'
                        '"asset_roots":{"generated":"generated"},"publication":"publication"}')
    cue = "audio/music/generated/example"
    product_path = "playback/intro/intro.band.ogg"
    description = tmp_path / "description.json"
    description.write_text(json.dumps({
        "music_playback": {"sections": [{"products": [{"path": product_path}]}]},
        "diagnostics": {"path": "preview/review.ogg"},
    }))
    stem = tmp_path / "stem.ogg"
    stem.write_bytes(b"accepted stem")
    store = Store(library / "publication")
    key = store.accept("producer:music:cue:example", {
        f"{cue}/example.adaptive_manifest.json": (description, "runtime", "full"),
        f"{cue}/{product_path}": (stem, "runtime", "full"),
    }, None)
    store.publish({"producer:music:cue:example": key}, None)
    inputs = guard.PackInputs(set(), {}, [manifest], [
        guard.LibraryPackageFile("shared", logical, f'content://shared/__art/{row["path"]}', store.root / row["path"])
        for logical, row in store.selected()["files"].items()
    ])
    monkeypatch.setattr(guard, "read_pack_inputs", lambda *_: inputs)
    contract, _ = guard.build_contract(tmp_path, "android")
    product = next(file for file in inputs.library_files if file.logical.endswith(product_path))
    packaged = guard.normalize_package_path(product.address)
    assert any(origin.startswith("manifest:") for origin in contract.entries[packaged].origins)

    inputs.library_files.remove(product)
    with pytest.raises(guard.AssetContractError, match=f"{cue}/{product_path}"):
        guard.build_contract(tmp_path, "android")
