"""A cue replacement keeps other accepted cues and rejects a stale render."""
import json
from pathlib import Path
import sys
from types import SimpleNamespace

import pytest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from scripts.regen import generate_library_content as producer


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
    def selected_cues(arguments):
        if "--cue" in arguments:
            return arguments[arguments.index("--cue") + 1].split(",")
        return sorted(expected)
    monkeypatch.setattr(producer, "selected_music_cues", selected_cues)

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


@pytest.mark.parametrize("concurrent", [False, True])
@pytest.mark.parametrize("existing_registry", [False, True])
def test_only_missing_music_keeps_existing_cues_and_updates_the_registry(music, monkeypatch, concurrent, existing_registry):
    music.publish("alpha", b"accepted alpha")
    if existing_registry:
        source = music.cache / "old-registry.ron"
        source.write_text("old registry")
        unit = "producer:music:registry"
        key = music.store.accept(unit, {"audio/music_registry.ron": (source, "runtime", "full")}, {})
        music.store.publish({unit: key}, music.store.selected())
    alpha = music.store.paths()["audio/music/generated/alpha/full.ogg"]
    music.expected["alpha"] = "stale but retained"
    calls = []

    def generate(command, *, cwd, env):
        calls.append(command)
        assert command[command.index("--cue") + 1] == "beta"
        root = Path(env["AMBITION_GENERATION_ROOT"])
        folder = root / "audio/music/generated/beta"
        folder.mkdir(parents=True)
        (folder / "full.ogg").write_bytes(b"new beta")
        (folder / "adaptive/intro").mkdir(parents=True)
        (folder / "adaptive/intro/intro.full.ogg").write_bytes(b"intro")
        (root / "audio/music_registry.ron").write_text("registry with alpha and beta")
        if concurrent:
            music.publish("beta", b"concurrent beta")
        return 0

    monkeypatch.setattr(producer.subprocess, "call", generate)
    assert producer.run("music", ["--only-missing"]) == 0
    paths = music.store.paths()
    assert paths["audio/music/generated/alpha/full.ogg"] == alpha
    assert alpha.read_bytes() == b"accepted alpha"
    assert paths["audio/music/generated/beta/full.ogg"].read_bytes() == (b"concurrent beta" if concurrent else b"new beta")
    assert paths["audio/music_registry.ron"].read_text() == "registry with alpha and beta"
    if not concurrent:
        assert paths["audio/music/generated/beta/adaptive/intro/intro.full.ogg"].read_bytes() == b"intro"
    assert producer.run("music", ["--only-missing"]) == 0
    assert len(calls) == 1


def test_only_missing_can_restore_just_the_music_registry(music, monkeypatch):
    music.publish("alpha", b"accepted alpha")
    music.publish("beta", b"accepted beta")
    held = music.store.paths().copy()
    def generate(command, *, cwd, env):
        assert command[-1].endswith("regen_music_registry.py")
        root = Path(env["AMBITION_GENERATION_ROOT"])
        (root / "audio").mkdir(parents=True)
        (root / "audio/music_registry.ron").write_text("restored registry")
        return 0
    monkeypatch.setattr(producer.subprocess, "call", generate)
    assert producer.run("music", ["--only-missing"]) == 0
    paths = music.store.paths()
    assert all(paths[name] == path for name, path in held.items())
    assert paths["audio/music_registry.ron"].read_text() == "restored registry"


@pytest.mark.parametrize("family", ["fonts", "music", "sfx", "variants"])
def test_only_missing_refuses_forced_replacement(family):
    with pytest.raises(ValueError, match="cannot be combined"):
        producer.run(family, ["--only-missing", "--force"])


@pytest.mark.parametrize("family", ["fonts", "sfx"])
def test_only_missing_keeps_an_existing_media_publication(tmp_path, monkeypatch, family):
    source = tmp_path / "asset"
    source.write_bytes(b"accepted")
    selected = SimpleNamespace(path=tmp_path / "library.json", manifest={"publication": "publication", "producers": {family: {}}})
    store = producer.Store(tmp_path / "publication")
    unit = "producer:" + family
    key = store.accept(unit, {family + "/asset": (source, "runtime", "full")}, {})
    before = store.publish({unit: key}, None)
    monkeypatch.setattr(producer, "library", lambda: selected)
    def refuse(*args, **kwargs):
        raise AssertionError("Existing products must not need a freshness check or a renderer")
    monkeypatch.setattr(producer, "current", refuse)
    monkeypatch.setattr(producer.subprocess, "call", refuse)
    assert producer.run(family, ["--only-missing"]) == 0
    assert store.selected() == before
