"""Music, sound, fonts and worlds index from the selected library and pack, and never block each other."""

import json
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from asset_index import kinds, reviews
from asset_index.scan import HashCache


def write_bank(path: Path, clips: dict[str, bytes]) -> None:
    """A version 1 `AMBNDSFX` bank, as `tools/ambition_sfx_pack/pack.py` writes it."""
    names = list(clips)
    entries_at = 40
    payloads_at = entries_at + 64 * len(names)
    offset, table, payload = payloads_at, b"", b""
    for name in names:
        data = clips[name]
        table += struct.pack("<QQIBBHIIfffI16s", 0, offset, len(data), 1, 2, 0, 48000, 500, 0.0, -6.0, -18.0, 0, b"")
        payload += data
        offset += len(data)
    names_blob = b"".join(struct.pack("<H", len(n.encode())) + n.encode() for n in names)
    header = struct.pack("<8sIIQQQ", b"AMBNDSFX", 1, len(names), entries_at, payloads_at, payloads_at + len(payload))
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(header + table + payload + names_blob)


def game(tmp_path: Path) -> Path:
    library = tmp_path / "game/ambition_content_library"
    (library / "library.json").write_text(json.dumps({
        "schema": "ambition.content_library.v1", "id": "fixture", "version": "1",
        "asset_roots": {"generated": "generated/runtime"}, "publication": "generated/art_publication",
        "producers": {
            "music": {"tool": "music_tool", "registry": "audio/music_registry.ron", "roots": ["audio/music"]},
            "sfx": {"tool": "sfx_tool", "roots": ["audio/sfx.bank"]},
            "fonts": {"tool": "font_download", "policy": "authored/fonts.json", "roots": ["fonts"]}}}))
    runtime = library / "generated/runtime"
    (runtime / "audio").mkdir(parents=True)
    (runtime / "audio/music_registry.ron").write_text('(tracks: [\n(id: "theme", display_name: "Main Theme"),\n(id: "gone", display_name: "Gone"),\n])')
    (runtime / "audio/music/generated/theme").mkdir(parents=True)
    (runtime / "audio/music/generated/theme/full.ogg").write_bytes(b"OggS theme")
    scores = tmp_path / "tools/music_tool/scores/active"
    scores.mkdir(parents=True)
    (scores / "sketch.music.yaml").write_text("schema: music\n")
    (scores / "theme.music.yaml").write_text("""schema: music
title: Main Theme
description: The hub's morning theme.
notes:
- Kept the original melody.
- Lowered the bass an octave.
tempo: {bpm: 96}
meter: {beats_per_bar: 3, beat_unit: 4}
sections: [{name: intro}, {name: main}]
instruments:
- {name: lead, group: lead, program: clean_guitar, instrument_backend: {kind: sfz, library_ref: guitar.archtop}}
- {name: pad, group: pad, instrument_backend: {kind: surge, patch: Pads/Glass}}
- {name: drums, group: drums, program: standard, is_drum: true}
""")
    write_bank(runtime / "audio/sfx.bank", {"ui.click": b"OggS click"})
    sounds = tmp_path / "tools/sfx_tool/sounds/active"
    sounds.mkdir(parents=True)
    (sounds / "ui.click.sfx.yaml").write_text("duration_ms: 500\nlayers:\n- name: tick\n")
    (sounds / "ui.unpacked.sfx.yaml").write_text("duration_ms: 100\n")
    (library / "authored").mkdir()
    (library / "authored/fonts.json").write_text(json.dumps([{"name": "Mono", "license_name": "OFL", "extracts": [["a.ttf", "Mono.ttf"], ["b.ttf", "Missing.ttf"]]}]))
    (runtime / "fonts").mkdir()
    (runtime / "fonts/Mono.ttf").write_bytes(b"font")
    pack = tmp_path / "game/pack"
    (pack / "worlds").mkdir(parents=True)
    (pack / "worlds/hub.ldtk").write_text(json.dumps({"levels": [{"identifier": "lobby", "pxWid": 320, "pxHei": 180,
        "fieldInstances": [{"__identifier": "music_track", "__value": "theme"}, {"__identifier": "parallax_theme", "__value": "dawn"}]}]}))
    return pack


def test_each_kind_reports_what_exists_what_is_missing_and_where_it_is_used(tmp_path):
    pack = game(tmp_path)
    out = tmp_path / "out"
    result = kinds.collect(tmp_path, out, HashCache(out / "hashes.json"), pack=pack, log=lambda _m: None)
    items = result["items"]
    assert items["music:theme"]["state"] == "present"
    score = items["music:theme"]["score"]
    assert score["description"] == "The hub's morning theme." and score["notes"][1] == "Lowered the bass an octave."
    assert (score["bpm"], score["meter"], score["sections"]) == (96, "3/4", ["intro", "main"])
    assert [(i["sound"], i["source"]) for i in score["instruments"]] == [
        ("guitar.archtop", "sampled (SFZ)"), ("Pads/Glass", "synth (Surge XT)"), ("standard", "drum kit (General MIDI)")]
    assert items["music:gone"]["state"] == "missing" and items["music:gone"]["fix"]
    assert items["music:sketch"]["state"] == "missing" and "newer than the last music publish" in items["music:sketch"]["reasons"][0]
    assert items["sfx:ui.click"]["state"] == "present"
    assert (out / items["sfx:ui.click"]["media"]["href"]).read_bytes() == b"OggS click"
    assert items["sfx:ui.unpacked"]["state"] == "missing"
    assert items["font:Mono.ttf"]["state"] == "present" and items["font:Missing.ttf"]["state"] == "missing"
    assert items["world:hub"]["details"]["rooms"][0]["music_track"] == "theme"
    assert items["music:theme"]["related"] == ["world:hub"]
    assert all(not row["notices"] for row in result["kinds"].values())


def test_a_failing_kind_is_a_notice_and_the_others_still_index(tmp_path):
    pack = game(tmp_path)
    (tmp_path / "game/ambition_content_library/generated/runtime/audio/sfx.bank").write_bytes(b"not a bank")
    out = tmp_path / "out"
    result = kinds.collect(tmp_path, out, HashCache(out / "hashes.json"), pack=pack, log=lambda _m: None)
    assert result["kinds"]["sfx"]["notices"]
    assert result["items"]["music:theme"]["state"] == "present"


def test_a_review_note_on_any_item_records_the_version_it_reviewed(tmp_path):
    pack = game(tmp_path)
    out = tmp_path / "out"
    catalog = kinds.collect(tmp_path, out, HashCache(out / "hashes.json"), pack=pack, log=lambda _m: None)
    subject = reviews.snapshot(tmp_path, catalog)["subjects"]["music:theme"]
    assert reviews.all_subjects(catalog)["music:theme"] == subject  # what the review service accepts
    reviews.check_products(tmp_path, subject)
    reviews.Store(tmp_path).add(subject, "The bridge drags.")
    audio = tmp_path / "game/ambition_content_library/generated/runtime/audio/music/generated/theme/full.ogg"
    audio.write_bytes(b"OggS a longer theme")
    try:
        reviews.check_products(tmp_path, subject)
        raise AssertionError("A changed asset must refuse a note for its old version")
    except reviews.ConflictError:
        pass
    later = kinds.collect(tmp_path, out, HashCache(out / "hashes.json"), pack=pack, log=lambda _m: None)
    note = reviews.snapshot(tmp_path, later)["notes"][0]
    assert note["generation_relation"] == "older"


def test_a_music_id_counts_as_used_only_where_the_line_names_music(tmp_path):
    import subprocess
    (tmp_path / "game").mkdir(exist_ok=True)
    (tmp_path / "game/moves.ron").write_text('clip: "theme",\n')
    (tmp_path / "game/rooms.ron").write_text('music_track: "theme",\n')
    subprocess.run(["git", "init", "-q"], cwd=tmp_path, check=True)
    subprocess.run(["git", "add", "game"], cwd=tmp_path, check=True)
    uses = __import__("collections").defaultdict(list)
    kinds.text_uses(tmp_path, [{"id": "music:theme", "kind": "music", "name": "theme"}], uses)
    assert [row["file"] for row in uses["music:theme"]] == ["game/rooms.ron"]


def test_the_content_binary_is_rebuilt_only_when_a_rust_source_is_newer(tmp_path, monkeypatch):
    import os
    from asset_index import sources
    binary = tmp_path / "target/debug/ambition_content_check"
    binary.parent.mkdir(parents=True)
    binary.write_text("old binary")
    source = tmp_path / "crates/schema/src/lib.rs"
    source.parent.mkdir(parents=True)
    source.write_text("pub struct Tactics;")
    os.utime(binary, (1, 1))
    assert sources.newer_source(tmp_path, binary) == source
    os.utime(binary, None)
    os.utime(source, (1, 1))
    assert sources.newer_source(tmp_path, binary) is None


def test_the_newer_of_the_release_and_debug_binaries_is_used(tmp_path, monkeypatch):
    import os
    from asset_index import sources
    monkeypatch.delenv("AMBITION_ART_CONTENT_BIN", raising=False)
    assert sources.default_binary(tmp_path) == tmp_path / "target/release/ambition_content_check"
    for profile, when in (("release", 100), ("debug", 200)):
        path = tmp_path / "target" / profile / "ambition_content_check"
        path.parent.mkdir(parents=True)
        path.write_text(profile)
        os.utime(path, (when, when))
    assert sources.default_binary(tmp_path).parent.name == "debug"
    os.utime(tmp_path / "target/release/ambition_content_check", (300, 300))
    assert sources.default_binary(tmp_path).parent.name == "release"


def test_a_music_review_from_the_page_rates_only_an_offered_unchanged_render(tmp_path, monkeypatch):
    from types import SimpleNamespace
    import pytest
    from asset_index import music_review
    saved = []
    version = SimpleNamespace(cue_id="theme", render_hash="abc", preview_sha256="p1", version_id="abc:p1")
    class Store:
        def __init__(self, tool): pass
        def save(self, v, **fields):
            saved.append((v, fields))
            return SimpleNamespace(path=tmp_path / "tools/music/reviews/music/theme/abc--p1.review.yaml",
                                   data={"current": {"score": fields["score"], "label": "Strong", "notes": fields["notes"], "issues": fields["issues"], "reviewed_at": "now"},
                                         "subject": {"score_sha256_at_review": "s"}})
    module = SimpleNamespace(discover_render_versions=lambda *a, **k: [version], ReviewStore=Store)
    monkeypatch.setattr(music_review, "music_reviews_module", lambda tool: module)
    item = {"id": "music:theme", "kind": "music", "details": {"music_tool": "tools/music", "folder": "theme"},
            "review_targets": [{"version_id": "abc:p1", "render_hash": "abc", "preview_sha256": "p1", "roles": ["what the game plays"]}]}
    catalog = {"items": {"music:theme": item}}
    result = music_review.save(tmp_path, catalog, {"item": "music:theme", "version_id": "abc:p1", "score": 7.5, "notes": "Good", "issues": ["mix-master"]})
    assert result["review"]["score"] == 7.5 and saved[0][1]["issues"] == ["mix-master"]
    with pytest.raises(ValueError):
        music_review.save(tmp_path, catalog, {"item": "music:theme", "version_id": "other", "score": 5})
    version.preview_sha256 = "rerendered"
    with pytest.raises(LookupError):
        music_review.save(tmp_path, catalog, {"item": "music:theme", "version_id": "abc:p1", "score": 5})


def test_a_world_page_has_a_map_and_room_previews(tmp_path, monkeypatch):
    tool = Path(__file__).resolve().parents[2] / "ambition_ldtk_tools"
    monkeypatch.syspath_prepend(str(tool))
    pack = game(tmp_path)
    out = tmp_path / "out"
    world = kinds.collect(tmp_path, out, HashCache(out / "hashes.json"), pack=pack, log=lambda _m: None)["items"]["world:hub"]
    preview = world["preview"]
    assert (out / preview["world"]).read_text().startswith("<svg")
    assert (out / preview["rooms"]["lobby"]).is_file()
    assert preview["bounds"][2] == 320 + 2 * kinds._MARGIN
    assert world["details"]["rooms"][0]["at"] == [0, 0]
