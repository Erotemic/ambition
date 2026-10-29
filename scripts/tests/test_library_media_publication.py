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
    command = [sys.executable, str(ROOT / "scripts/grab_font_assets.py"), "--cache-dir", str(tmp_path / "cache")]
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
        result = subprocess.run([sys.executable, str(ROOT / "scripts/generate_library_content.py"), "--check"],
            cwd=ROOT, env=dict(os.environ, AMBITION_CONTENT_LIBRARY=str(manifest)), capture_output=True, text=True)
        assert result.returncode == 1, result.stdout + result.stderr
        assert "target:goblin: missing" in result.stdout, result.stdout + result.stderr
        assert not (library / "generated").exists()
