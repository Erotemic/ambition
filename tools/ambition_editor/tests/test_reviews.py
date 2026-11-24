"""Asset reviews: one format for every kind, keyed by the exact version reviewed."""

import copy
import hashlib
import json
import sys
import threading
from http.server import ThreadingHTTPServer
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen

import pytest
import yaml

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from asset_index import reviews, server

MUSIC_TOOL = Path(__file__).resolve().parents[2] / "ambition_music_renderer"


@pytest.fixture
def catalog(tmp_path):
    path = tmp_path / reviews.ASSETS / "sprites/body.png"
    path.parent.mkdir(parents=True)
    path.write_bytes(b"first published pixels")
    prop = tmp_path / reviews.ASSETS / "props/crate.png"
    prop.parent.mkdir(parents=True)
    prop.write_bytes(b"crate pixels")
    clip = tmp_path / "out/audio/sfx/ui.confirm.ogg"
    clip.parent.mkdir(parents=True)
    clip.write_bytes(b"OggS confirm")
    digest = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
    return {"products": {"sprites/body.png": {"sha256": digest(path)}, "props/crate.png": {"sha256": digest(prop)}},
            "assets": {"prop:crate": {"display": "Crate", "products": ["props/crate.png"], "family": "prop", "category": "props"},
                       "sheet:body": {"display": "Person", "products": ["sprites/body.png"], "identities": ["game:person"]}},
            "items": {"sfx:ui.confirm": {"kind": "sfx", "display": "ui.confirm", "sources": [],
                                         "version_files": [["out/audio/sfx/ui.confirm.ogg", digest(clip)]]}},
            "inventory": {"profile": {"fingerprint": "first catalog"}, "identities": {
                "game:person": {"key": "game:person", "display": "Person", "units": ["target:body"], "catalog": {"manifest": "sprites/body.ron"}}},
                "units": {"target:body": {"id": "target:body", "root": "sprites", "required": ["body.png", "body.ron"],
                    "lineages": ["person"], "generator": {"source": "body.py"}, "repair": "publish body"}}}}


def test_the_format_is_the_music_tools_scale_and_rubric():
    if not (MUSIC_TOOL / "ambition_music_renderer" / "music_reviews.py").is_file():
        pytest.skip("the music tool is not checked out")
    from asset_index.kinds import music_reviews_module
    music = music_reviews_module(MUSIC_TOOL)
    assert (reviews.MIN_SCORE, reviews.MAX_SCORE, reviews.POLISH_THRESHOLD) == (music.MIN_SCORE, music.MAX_SCORE, music.POLISH_THRESHOLD)
    assert reviews.RUBRIC_BANDS == music.RUBRIC_BANDS
    assert reviews.MUSIC_SCHEMA == music.REVIEW_SCHEMA and reviews.MUSIC_SCHEMA_V1 == music.REVIEW_SCHEMA_V1
    for score in (1, 2.99, 3, 5.5, 7, 9.5, 10):
        assert reviews.score_label(score) == music.score_label(score)


def test_a_character_version_tracks_art_bytes_not_unrelated_content(catalog):
    original = reviews.character_subjects(catalog)["game:person"]
    catalog["inventory"]["profile"]["fingerprint"] = "unrelated gameplay edit"
    catalog["inventory"]["identities"]["game:person"]["display"] = "New label"
    assert reviews.character_subjects(catalog)["game:person"]["version_id"] == original["version_id"]
    catalog["products"]["sprites/body.ron"] = {"sha256": "now published"}
    assert reviews.character_subjects(catalog)["game:person"]["version_id"] != original["version_id"]
    before_parts = reviews.character_subjects(catalog)["game:person"]["version_id"]
    catalog["inventory"]["units"]["target:body"]["optional"] = ["body_parts.png"]
    catalog["products"]["sprites/body_parts.png"] = {"sha256": "published parts"}
    assert reviews.character_subjects(catalog)["game:person"]["version_id"] != before_parts


def test_every_kind_is_a_subject_and_a_character_sheet_is_its_character(catalog):
    subjects = reviews.all_subjects(catalog)
    assert {key: subject["kind"] for key, subject in subjects.items()} == {
        "game:person": "character", "prop:crate": "image", "sfx:ui.confirm": "sfx"}


def test_a_review_keeps_its_version_and_asks_for_a_re_review_after_a_change(tmp_path, catalog):
    subject = reviews.all_subjects(catalog)["prop:crate"]
    saved = reviews.save(tmp_path, catalog, {"id": "prop:crate", "version_id": subject["version_id"], "score": 4,
                                             "notes": "Reads as a barrel.", "issues": ["silhouette-readability"]})
    document = yaml.safe_load((tmp_path / saved["path"]).read_text())
    assert document["schema"] == reviews.SCHEMA and document["score_scale"] == {"min": 1.0, "max": 10.0}
    assert document["subject"]["id"] == "prop:crate" and document["subject"]["version_id"] == subject["version_id"]
    assert document["current"]["label"] == "Major polish" and document["history"] == []
    report = reviews.snapshot(tmp_path, catalog)
    assert [(row["action"], row["id"]) for row in report["backlog"]["rows"]] == [("fix", "prop:crate")]
    catalog["products"]["props/crate.png"]["sha256"] = "redrawn"
    report = reviews.snapshot(tmp_path, catalog)
    assert report["reviews"][0]["relation"] == "older"
    assert report["backlog"]["rows"][0]["action"] == "re-review"


def test_an_edit_replaces_the_review_of_that_version_and_part(tmp_path, catalog):
    version = reviews.all_subjects(catalog)["game:person"]["version_id"]
    first = reviews.save(tmp_path, catalog, {"id": "game:person", "version_id": version, "score": 3, "part": "death"})
    reviews.save(tmp_path, catalog, {"id": "game:person", "version_id": version, "score": 8, "part": "death", "notes": "Fixed."})
    reviews.save(tmp_path, catalog, {"id": "game:person", "version_id": version, "score": 9})
    rows = reviews.snapshot(tmp_path, catalog)["reviews"]
    assert sorted((row["part"], row["score"]) for row in rows) == [("", 9.0), ("death", 8.0)]
    document = yaml.safe_load((tmp_path / first["path"]).read_text())
    assert document["current"]["created_at"] <= document["current"]["updated_at"]
    assert reviews.snapshot(tmp_path, catalog)["backlog"]["count"] == 0


def test_a_save_refuses_a_stale_page_changed_bytes_a_bad_score_and_unknown_tags(tmp_path, catalog):
    version = reviews.all_subjects(catalog)["sfx:ui.confirm"]["version_id"]
    with pytest.raises(reviews.ConflictError):
        reviews.save(tmp_path, catalog, {"id": "sfx:ui.confirm", "version_id": "an older page", "score": 5})
    for score in (0, 11, "loud", float("nan")):
        with pytest.raises(ValueError):
            reviews.save(tmp_path, catalog, {"id": "sfx:ui.confirm", "version_id": version, "score": score})
    with pytest.raises(ValueError):
        reviews.save(tmp_path, catalog, {"id": "sfx:ui.confirm", "version_id": version, "score": 5, "issues": ["composition"]})
    reviews.save(tmp_path, catalog, {"id": "sfx:ui.confirm", "version_id": version, "score": 5, "issues": ["mix-level"],
                                     "furthest_played_seconds": 0.4, "furthest_played_fraction": 1.0})
    assert reviews.snapshot(tmp_path, catalog)["reviews"][0]["furthest_played_fraction"] == 1.0
    (tmp_path / "out/audio/sfx/ui.confirm.ogg").write_bytes(b"OggS louder")
    with pytest.raises(reviews.ConflictError):
        reviews.save(tmp_path, catalog, {"id": "sfx:ui.confirm", "version_id": version, "score": 6})
    with pytest.raises(ValueError):
        reviews.save(tmp_path, catalog, {"id": "sfx:nothing", "version_id": version, "score": 6})


def test_shared_art_keeps_character_reviews_distinct(tmp_path, catalog):
    actor = copy.deepcopy(catalog["inventory"]["identities"]["game:person"])
    actor.update(key="game:other", display="Other")
    catalog["inventory"]["identities"]["game:other"] = actor
    subjects = reviews.character_subjects(catalog)
    assert subjects["game:person"]["version_id"] == subjects["game:other"]["version_id"]
    reviews.save(tmp_path, catalog, {"id": "game:person", "version_id": subjects["game:person"]["version_id"], "score": 2})
    reviews.save(tmp_path, catalog, {"id": "game:other", "version_id": subjects["game:other"]["version_id"], "score": 9})
    assert {row["id"]: row["score"] for row in reviews.snapshot(tmp_path, catalog)["reviews"]} == {"game:person": 2.0, "game:other": 9.0}


def test_music_reviews_are_read_from_the_music_tool_in_the_same_shape(tmp_path, catalog):
    folder = tmp_path / "tools/music/reviews/music/theme"
    folder.mkdir(parents=True)
    common = {"subject": {"cue_id": "theme", "version_id": "abc:p1"}}
    (folder / "abc--p1.review.yaml").write_text(yaml.safe_dump(dict(common, schema=reviews.MUSIC_SCHEMA_V1,
        current={"score": 2, "notes": "Old scale", "issues": ["composition"], "reviewed_at": "2026-01-01T00:00:00Z"})))
    (folder / "def--p2.review.yaml").write_text(yaml.safe_dump({"schema": reviews.MUSIC_SCHEMA, "subject": {"cue_id": "theme", "version_id": "def:p2"},
        "current": {"score": 8.5, "notes": "", "issues": [], "updated_at": "2026-02-01T00:00:00Z"}}))
    catalog["items"]["music:theme"] = {"kind": "music", "display": "Theme", "details": {"music_tool": "tools/music", "folder": "theme"},
        "review_targets": [{"version_id": "def:p2", "roles": ["what the game plays"]}]}
    rows = reviews.snapshot(tmp_path, catalog)["reviews"]
    assert [(row["version_id"], row["score"], row["label"], row["relation"]) for row in rows] == [
        ("def:p2", 8.5, "Strong", "current"), ("abc:p1", 4.0, "Major polish", "older")]
    assert reviews.snapshot_versions(catalog)["music:theme"] == "def:p2"


def test_an_unpublished_item_has_nothing_to_review_and_an_unpublished_track_rates_its_newest_render(tmp_path, catalog):
    catalog["items"]["sfx:ui.unpacked"] = {"kind": "sfx", "display": "ui.unpacked", "sources": [], "version_files": []}
    assert "sfx:ui.unpacked" not in reviews.all_subjects(catalog)
    folder = tmp_path / "tools/music/reviews/music/theme"
    folder.mkdir(parents=True)
    (folder / "new.review.yaml").write_text(yaml.safe_dump({"schema": reviews.MUSIC_SCHEMA, "subject": {"version_id": "new:p"},
        "current": {"score": 3, "updated_at": "2026-02-01T00:00:00Z"}}))
    catalog["items"]["music:theme"] = {"kind": "music", "display": "Theme", "details": {"music_tool": "tools/music", "folder": "theme"},
        "review_targets": [{"version_id": "new:p", "roles": ["newest render"]}]}
    report = reviews.snapshot(tmp_path, catalog)
    assert report["reviews"][0]["relation"] == "current" and report["backlog"]["rows"][0]["action"] == "fix"


def test_a_file_that_does_not_read_is_a_problem_not_a_failure(tmp_path, catalog):
    bad = tmp_path / "tools/ambition_editor/reviews/image/prop_crate/broken.review.yaml"
    bad.parent.mkdir(parents=True)
    bad.write_text("schema: something.else\n")
    report = reviews.snapshot(tmp_path, catalog)
    assert report["reviews"] == [] and "broken.review.yaml" in report["problems"][0]


def test_http_save_returns_the_snapshot_and_refuses_a_stale_page(tmp_path, catalog):
    with ThreadingHTTPServer(("127.0.0.1", 0), server.make_handler(tmp_path, catalog)) as http:
        thread = threading.Thread(target=http.serve_forever)
        thread.start()
        base = f"http://127.0.0.1:{http.server_port}/api/asset-reviews"
        def request(method="GET", data=None):
            body = json.dumps(data).encode() if data is not None else None
            with urlopen(Request(base, data=body, method=method, headers={"Content-Type": "application/json"})) as response:
                return json.load(response)
        try:
            version = request()["versions"]["game:person"]
            payload = {"id": "game:person", "version_id": version, "score": 5.5, "notes": "The hat clips.", "part": "death"}
            saved = request("POST", payload)
            assert saved["saved"]["review"]["label"] == "Acceptable"
            assert request()["backlog"]["rows"][0]["part"] == "death"
            with pytest.raises(HTTPError) as error:
                request("POST", dict(payload, version_id="stale page"))
            assert error.value.code == 409
            assert len(request()["reviews"]) == 1
        finally:
            http.shutdown()
            thread.join()
