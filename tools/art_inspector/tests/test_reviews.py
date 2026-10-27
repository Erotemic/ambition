"""Review notes retain their exact art subject across regeneration."""

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

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from art_index import reviews, server


@pytest.fixture
def catalog(tmp_path):
    path = tmp_path / reviews.ASSETS / "sprites/body.png"
    path.parent.mkdir(parents=True)
    path.write_bytes(b"first published pixels")
    return {"products": {"sprites/body.png": {"sha256": hashlib.sha256(path.read_bytes()).hexdigest()}},
            "inventory": {"profile": {"fingerprint": "first catalog"}, "identities": {
                "game:person": {"key": "game:person", "display": "Person", "units": ["target:body"], "catalog": {"manifest": "sprites/body.ron"}}},
                "units": {"target:body": {"id": "target:body", "root": "sprites", "required": ["body.png", "body.ron"],
                    "lineages": ["person"], "generator": {"source": "body.py"}, "repair": "publish body"}}}}


def test_generation_tracks_art_bytes_and_missing_products_not_unrelated_content(catalog):
    original = reviews.subjects(catalog)["game:person"]
    catalog["inventory"]["profile"]["fingerprint"] = "unrelated gameplay edit"
    catalog["inventory"]["identities"]["game:person"]["display"] = "New label"
    assert reviews.subjects(catalog)["game:person"]["generation_id"] == original["generation_id"]
    catalog["products"]["sprites/body.ron"] = {"sha256": "now published"}
    assert reviews.subjects(catalog)["game:person"]["generation_id"] != original["generation_id"]
    catalog["products"]["sprites/body.png"]["sha256"] = "new pixels"
    assert original["products"][0]["sha256"] != reviews.subjects(catalog)["game:person"]["products"][0]["sha256"]
    before_parts = reviews.subjects(catalog)["game:person"]["generation_id"]
    catalog["inventory"]["units"]["target:body"]["optional"] = ["body_parts.png"]
    catalog["products"]["sprites/body_parts.png"] = {"sha256": "published parts"}
    assert reviews.subjects(catalog)["game:person"]["generation_id"] != before_parts


def test_regeneration_keeps_open_notes_and_the_reviewed_subject(tmp_path, catalog):
    store = reviews.Store(tmp_path)
    subject = reviews.subjects(catalog)["game:person"]
    note = store.add(subject, "The hat clips during death.", priority="high", animation="death")
    catalog["products"]["sprites/body.png"]["sha256"] = "new pixels"
    result = reviews.snapshot(tmp_path, catalog)
    assert result["backlog"]["open_count"] == 1
    assert result["notes"][0]["generation_relation"] == "older"
    assert result["notes"][0]["subject"] == subject
    store.update(note["id"], "resolved", "Moved the cap joint.")
    assert reviews.snapshot(tmp_path, catalog)["backlog"]["open_count"] == 0
    saved = reviews.Store(tmp_path).load()[0]
    assert saved["resolution"] == "Moved the cap joint."
    assert saved["text"] == note["text"] and saved["subject"] == subject
    store.update(note["id"], "open")
    assert reviews.snapshot(tmp_path, catalog)["backlog"]["open_count"] == 1
    del catalog["inventory"]["identities"]["game:person"]
    assert reviews.snapshot(tmp_path, catalog)["notes"][0]["generation_relation"] == "unavailable"


def test_shared_art_keeps_character_feedback_distinct_and_sorts_priority(tmp_path, catalog):
    actor = copy.deepcopy(catalog["inventory"]["identities"]["game:person"])
    actor.update(key="game:other", display="Other")
    catalog["inventory"]["identities"]["game:other"] = actor
    subjects = reviews.subjects(catalog)
    assert subjects["game:person"]["generation_id"] == subjects["game:other"]["generation_id"]
    store = reviews.Store(tmp_path)
    store.add(subjects["game:person"], "Low priority.", priority="low")
    store.add(subjects["game:other"], "High priority.", priority="high")
    queue = reviews.snapshot(tmp_path, catalog)["backlog"]
    assert [group["character"] for group in queue["characters"]] == ["game:other", "game:person"]
    assert queue["open_count"] == 2


def test_save_rejects_changed_pixels_or_missing_products_that_appeared(tmp_path, catalog):
    subject = reviews.subjects(catalog)["game:person"]
    reviews.check_products(tmp_path, subject)
    (tmp_path / reviews.ASSETS / "sprites/body.ron").write_text("new manifest")
    with pytest.raises(reviews.ConflictError):
        reviews.check_products(tmp_path, subject)
    (tmp_path / reviews.ASSETS / "sprites/body.ron").unlink()
    (tmp_path / reviews.ASSETS / "sprites/body.png").write_bytes(b"new pixels")
    with pytest.raises(reviews.ConflictError):
        reviews.check_products(tmp_path, subject)


def test_note_updates_reject_an_older_edit(tmp_path, catalog):
    store = reviews.Store(tmp_path)
    note = store.add(reviews.subjects(catalog)["game:person"], "A note.")
    store.update(note["id"], "resolved", expected_updated_at=note["updated_at"])
    with pytest.raises(reviews.ConflictError):
        store.update(note["id"], "open", expected_updated_at=note["updated_at"])


def test_http_save_survives_reload_and_exposes_the_same_backlog(tmp_path, catalog):
    with ThreadingHTTPServer(("127.0.0.1", 0), server.make_handler(tmp_path, catalog)) as http:
        thread = threading.Thread(target=http.serve_forever)
        thread.start()
        base = f"http://127.0.0.1:{http.server_port}/api/art-reviews"
        def request(method="GET", path="", data=None):
            body = json.dumps(data).encode() if data is not None else None
            with urlopen(Request(base + path, data=body, method=method, headers={"Content-Type": "application/json"})) as response:
                return json.load(response)
        try:
            subject = request()["subjects"]["game:person"]
            payload = {"character": "game:person", "generation_id": subject["generation_id"], "text": "Please fix the hat.", "priority": "high"}
            saved = request("POST", data=payload)
            note = saved["notes"][0]
            assert request()["backlog"]["open_count"] == 1
            assert reviews.Store(tmp_path).load()[0]["id"] == note["id"]
            request("PATCH", "/" + note["id"], {"status": "resolved", "resolution": "Fixed the cap.", "updated_at": note["updated_at"]})
            assert request()["backlog"]["open_count"] == 0
            with pytest.raises(HTTPError) as error:
                request("POST", data=dict(payload, generation_id="stale page"))
            assert error.value.code == 409
            assert len(reviews.Store(tmp_path).load()) == 1
        finally:
            http.shutdown()
            thread.join()
