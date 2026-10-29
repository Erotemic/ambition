"""The music roster comes from authored scores, without local render state."""

import importlib.util
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("regen_music_registry", ROOT / "scripts/regen_music_registry.py")
registry = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(registry)


def test_unrendered_scores_are_registered_with_their_playback_policy(tmp_path, monkeypatch):
    scores = tmp_path / "scores"
    scores.mkdir()
    (scores / "loop.music.yaml").write_text('id: loop\none_shot: false\n')
    (scores / "sting.music.yaml").write_text('id: sting\none_shot: true\n')
    generated = tmp_path / "generated"
    generated.mkdir()
    (generated / "retired").mkdir()
    (generated / "retired/full.ogg").write_bytes(b"old render")
    monkeypatch.setitem(registry._policy, "EXTRA_CUES", [])
    monkeypatch.setattr(registry, "ACTIVE_SCORES_DIR", scores)
    monkeypatch.setattr(registry, "GENERATED_DIR", generated)
    monkeypatch.setattr(registry, "SPECIAL_ENTRIES", [])

    content = registry.render_registry()

    assert registry.tracked_ids(content) == {"loop", "sting"}
    assert '(id: "loop", display_name: "Loop")' in content
    assert '(id: "sting", display_name: "Sting", one_shot: true)' in content
