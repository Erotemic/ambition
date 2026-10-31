"""A cue replacement keeps other accepted cues and rejects a stale render."""
import json
from pathlib import Path
import sys
from types import SimpleNamespace

import pytest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import generate_library_content as producer


@pytest.fixture
def music(tmp_path, monkeypatch):
    manifest = tmp_path / "library.json"
    manifest.write_text("{}")
    source = tmp_path / "producer.py"
    source.write_text("source")
    selected = SimpleNamespace(path=manifest, manifest={"publication": "publication", "producers": {"music": {
        "registry": "audio/music_registry.ron", "roots": ["audio/music", "audio/music_registry.ron"]}}})
    store = producer.Store(tmp_path / "publication")
    expected = {"alpha": "a", "beta": "b"}
    cache = tmp_path / "renders"
    for cue, fingerprint in expected.items():
        folder = cache / cue
        folder.mkdir(parents=True)
        (folder / "manifest.json").write_text(json.dumps({"render_dependencies": {"fingerprint": fingerprint}}))
    monkeypatch.setitem(sys.modules, "ambition_music_renderer", SimpleNamespace())
    monkeypatch.setitem(sys.modules, "ambition_music_renderer.cli", SimpleNamespace(
        generated_root=lambda: cache, resolve_latest_generated_dir=lambda path: path,
        find_latest_manifest=lambda path, cue: path / "manifest.json"))
    monkeypatch.setattr(producer, "input_files", lambda family, selected: [manifest, source])
    monkeypatch.setattr(producer, "owner_dependencies", lambda family, cues=None: {
        cue: expected[cue] for cue in (expected if cues is None else cues)})
    monkeypatch.setattr(producer, "library", lambda: selected)
    monkeypatch.setattr(producer, "selected_music_cues", lambda arguments: (
        [arguments[1]] if arguments else sorted(expected)))

    def publish(cue, data, extra=False):
        cues = [cue] if isinstance(cue, str) else cue
        staging = tmp_path / ("-".join(cues) + data.decode())
        root = staging / "runtime/full"
        for selected_cue in cues:
            folder = root / "audio/music/generated" / selected_cue
            folder.mkdir(parents=True)
            (folder / "full.ogg").write_bytes(data)
            (folder / "manifest.json").write_text("review")
            if extra:
                (folder / "intro.ogg").write_bytes(b"intro")
        (root / "audio/music_registry.ron").write_text("authored registry")
        hashes = producer.Digests()
        before = producer.evidence("music", selected, hashes, scope={"cues": cues})
        producer.accept_outputs("music", selected, store, staging, before, hashes, ["--cue", ",".join(cues)])

    return SimpleNamespace(store=store, publish=publish, expected=expected, cache=cache)


def test_replacing_one_cue_keeps_another_and_a_stale_render_keeps_the_selection(music):
    music.publish("alpha", b"first", extra=True)
    music.publish("beta", b"other")
    before = music.store.selected()
    beta = music.store.paths()["audio/music/generated/beta/full.ogg"]
    music.expected["alpha"] = "new render"
    with pytest.raises(ValueError, match="no current render"):
        music.publish("alpha", b"stale")
    assert music.store.selected() == before
    assert music.store.paths()["audio/music/generated/alpha/full.ogg"].read_bytes() == b"first"
    (music.cache / "alpha/manifest.json").write_text(json.dumps({
        "render_dependencies": {"fingerprint": "new render"}}))
    music.publish("alpha", b"second")
    paths = music.store.paths()
    assert paths["audio/music/generated/alpha/full.ogg"].read_bytes() == b"second"
    assert "audio/music/generated/alpha/intro.ogg" not in paths
    assert paths["audio/music/generated/beta/full.ogg"] == beta
    assert beta.read_bytes() == b"other"


def test_a_scoped_check_does_not_claim_a_complete_library(music):
    music.publish("alpha", b"first")
    assert "audio/music_registry.ron" not in music.store.paths()
    assert producer.check("music", ["--cue", "alpha"]) == 0
    assert producer.check("music") == 1
    music.publish("beta", b"second")
    assert producer.check("music") == 1
    music.publish(["alpha", "beta"], b"complete")
    assert "audio/music_registry.ron" in music.store.paths()
    assert producer.check("music") == 0
    music.expected["beta"] = "changed"
    assert producer.check("music", ["--cue", "alpha"]) == 0
    assert producer.check("music") == 1
