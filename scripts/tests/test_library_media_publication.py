"""Media replacement uses accepted files and keeps the last valid selection."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from lib.content_libraries import read_libraries
from lib.art_publication import Store


def test_font_replacement_retires_old_paths_and_a_failure_keeps_accepted_bytes(tmp_path):
    library = tmp_path / "library"
    (library / "authored/runtime/fonts").mkdir(parents=True)
    (library / "authored/runtime/fonts/own.ttf").write_bytes(b"authored")
    archive = tmp_path / "fixture-font.zip"
    with zipfile.ZipFile(archive, "w") as output:
        output.writestr("face.ttf", b"accepted face")
        output.writestr("OFL.txt", "fixture license")
    source = {"name": "Fixture", "url": archive.as_uri(), "homepage": "https://example.invalid",
              "license_name": "Fixture", "extracts": [["face.ttf", "first.ttf"]],
              "license_candidates": ["OFL.txt"], "archive_sha256": hashlib.sha256(archive.read_bytes()).hexdigest()}
    policy = library / "authored/fonts.json"
    policy.write_text(json.dumps([source]))
    manifest = library / "library.json"
    manifest.write_text(json.dumps({"schema": "ambition.content_library.v1", "id": "fixture", "version": "1",
        "asset_roots": {"authored": "authored/runtime", "generated": "generated/runtime/full"},
        "publication": "generated/publication", "provenance": "generated/provenance",
        "producers": {"fonts": {"policy": "authored/fonts.json", "roots": ["fonts"]}}}))
    env = dict(os.environ, AMBITION_CONTENT_LIBRARY=str(manifest))
    command = [sys.executable, str(ROOT / "scripts/regen/grab_font_assets.py"), "--cache-dir", str(tmp_path / "cache")]
    def run():
        return subprocess.run(command, env=env, cwd=ROOT, text=True, capture_output=True)
    first = run()
    assert first.returncode == 0, first.stdout + first.stderr
    selected = lambda: next(iter(read_libraries([manifest]).values())).files()
    held = selected()["fonts/bundled/first.ttf"]
    source["extracts"] = [["face.ttf", "second.ttf"]]
    policy.write_text(json.dumps([source]))
    second = run()
    assert second.returncode == 0, second.stdout + second.stderr
    files = selected()
    assert "fonts/bundled/first.ttf" not in files
    assert files["fonts/bundled/second.ttf"].read_bytes() == held.read_bytes() == b"accepted face"
    assert files["fonts/own.ttf"].read_bytes() == b"authored"
    assert not any("FONT_ASSET_MANIFEST" in name for name in files)
    store = Store(library / "generated/publication")
    accepted = store.selected()["generation"]
    source["archive_sha256"] = "0" * 64
    policy.write_text(json.dumps([source]))
    assert run().returncode != 0
    assert store.selected()["generation"] == accepted
    assert selected()["fonts/bundled/second.ttf"].read_bytes() == b"accepted face"


def test_bank_scope_keeps_authored_audio_and_other_media(tmp_path):
    (tmp_path / "authored/audio").mkdir(parents=True)
    (tmp_path / "generated/audio/music").mkdir(parents=True)
    (tmp_path / "authored/audio/sfx_registry.ron").write_text("authored registry")
    (tmp_path / "generated/audio/music/track.ogg").write_bytes(b"music")
    (tmp_path / "generated/audio/sfx.bank").write_bytes(b"old bank")
    bank = tmp_path / "new.bank"
    bank.write_bytes(b"accepted bank")
    manifest = tmp_path / "library.json"
    manifest.write_text(json.dumps({"schema": "ambition.content_library.v1", "id": "fixture", "version": "1",
        "asset_roots": {"authored": "authored", "generated": "generated"}, "publication": "publication"}))
    store = Store(tmp_path / "publication")
    key = store.accept("producer:sfx", {"audio/sfx.bank": (bank, "runtime", "full")}, None, roots=["audio/sfx.bank"])
    store.publish({"producer:sfx": key}, None)
    files = next(iter(read_libraries([manifest]).values())).files()
    assert files["audio/sfx.bank"].read_bytes() == b"accepted bank"
    assert files["audio/music/track.ogg"].read_bytes() == b"music"
    assert files["audio/sfx_registry.ron"].read_text() == "authored registry"


def test_all_media_check_refuses_a_source_only_sprite_library():
    (ROOT / ".cache").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="sprite-check-", dir=ROOT / ".cache") as name:
        library = Path(name)
        (library / "authored").mkdir()
        (library / "authored/sprites.yaml").write_text(json.dumps({
            "collections": {"runtime": ["goblin"]}, "static_props": {}}))
        manifest = library / "library.json"
        manifest.write_text(json.dumps({"schema": "ambition.content_library.v1", "id": "probe", "version": "1",
            "asset_roots": {"generated": "generated/runtime/full"}, "publication": "generated/art_publication",
            "producers": {"sprites": {"collection": "runtime", "policy": "authored/sprites.yaml", "sources": ["authored/sprites.yaml"]}}}))
        result = subprocess.run([sys.executable, str(ROOT / "scripts/regen/generate_library_content.py"), "--check"],
            cwd=ROOT, env=dict(os.environ, AMBITION_CONTENT_LIBRARY=str(manifest)), capture_output=True, text=True)
        assert result.returncode == 1, result.stdout + result.stderr
        assert "target:goblin: missing" in result.stdout, result.stdout + result.stderr
        assert not (library / "generated").exists()


def test_music_plan_and_registry_write_work_outside_the_checkout(tmp_path):
    (tmp_path / "authored").mkdir()
    (tmp_path / "authored/music.json").write_bytes(
        (ROOT / "game/ambition_content_library/authored/music.json").read_bytes())
    manifest = tmp_path / "library.json"
    manifest.write_text(json.dumps({
        "schema": "ambition.content_library.v1", "id": "fixture", "version": "1",
        "asset_roots": {"generated": "generated/runtime/full"},
        "publication": "generated/art_publication",
        "producers": {"music": {"policy": "authored/music.json",
            "registry": "audio/music_registry.ron", "collection": "radio"}},
    }))
    result = subprocess.run(
        [sys.executable, str(ROOT / "scripts/checks/check_regeneration_coverage.py"),
            "--family", "music", "--plans-only"],
        cwd=ROOT, env=dict(os.environ, AMBITION_CONTENT_LIBRARY=str(manifest)),
        capture_output=True, text=True,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert not (tmp_path / "generated").exists()
    written = subprocess.run(
        [sys.executable, str(ROOT / "scripts/regen/regen_music_registry.py")],
        cwd=ROOT, env=dict(os.environ, AMBITION_CONTENT_LIBRARY=str(manifest)),
        capture_output=True, text=True,
    )
    assert written.returncode == 0, written.stdout + written.stderr
    assert (tmp_path / "generated/runtime/full/audio/music_registry.ron").is_file()


def test_missing_background_tiers_use_the_selected_source_bundle(tmp_path):
    from PIL import Image

    library = tmp_path / "library"
    library.mkdir()
    manifest = library / "library.json"
    manifest.write_text(json.dumps({
        "schema": "ambition.content_library.v1", "id": "fixture", "version": "1",
        "asset_roots": {"generated": "generated/runtime/full", "half": "generated/runtime/half",
                        "quarter": "generated/runtime/quarter", "potato": "generated/runtime/potato"},
        "publication": "generated/publication", "provenance": "generated/provenance",
        "producers": {"backgrounds": {"roots": ["backgrounds"]},
                      "variants": {"roots": ["backgrounds/parallax_layers_0_5x",
                                            "backgrounds/parallax_layers_0_25x", "backgrounds/parallax_layers_potato"]}},
    }))
    full = tmp_path / "sky.png"
    half = tmp_path / "native-half.png"
    Image.new("RGBA", (32, 32), "red").save(full)
    Image.new("RGBA", (2, 2), "blue").save(half)
    store = Store(library / "generated/publication")
    key = store.accept("theme:fixture", {
        "backgrounds/parallax_layers/sky.png": (full, "runtime", "full"),
        "backgrounds/parallax_layers_0_5x/sky.png": (half, "runtime", "half"),
    }, {})
    store.publish({"theme:fixture": key}, None)
    held = store.paths()["backgrounds/parallax_layers_0_5x/sky.png"]
    env = dict(os.environ, AMBITION_CONTENT_LIBRARY=str(manifest), AMBITION_SPRITE_PYTHON=sys.executable,
               PYTHONPATH=str(ROOT / "tools/ambition_sprite2d_renderer"))
    command = [sys.executable, str(ROOT / "scripts/regen/generate_library_content.py"), "variants", "--backgrounds-only"]
    for options in (["--only-missing"], ["--only-missing"], []):
        before = store.selected()
        result = subprocess.run([*command, *options], env=env, cwd=ROOT, text=True, capture_output=True)
        assert result.returncode == 0, result.stdout + result.stderr
        assert store.selected()["bundles"]["theme:fixture"] == key
        assert "producer:backgrounds" not in store.selected()["bundles"]
        assert store.paths()["backgrounds/parallax_layers_0_5x/sky.png"] == held
        assert held.read_bytes() == half.read_bytes()
        with Image.open(store.paths()["backgrounds/parallax_layers_0_25x/sky.png"]) as image:
            assert image.size == (8, 8)
        if options and len(before["bundles"]) > 1:
            assert store.selected() == before
