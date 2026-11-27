"""Save edits through the served inspector."""

import json
import threading
from http.server import ThreadingHTTPServer
from pathlib import Path
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, urlopen
import sys

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from asset_index.server import make_handler


@pytest.fixture
def editor(tmp_path):
    pack = tmp_path / "pack"
    (pack / "data").mkdir(parents=True)
    (pack / "worlds").mkdir()
    (pack / "pack.ron").write_text('''(
        id: "fixture", namespace: "fixture",
        sources: [
            (path: "data/character_catalog.ron", schema: "character_catalog", version: 1),
            (path: "data/music.ron", schema: "music_registry", version: 1),
        ],
    )''')
    (pack / "data/music.ron").write_text('''(tracks: [
        (id: "calm", display_name: "Calm"),
        (id: "fight", display_name: "Fight"),
    ])''')

    with ThreadingHTTPServer(("127.0.0.1", 0), make_handler(tmp_path, {}, profile=str(pack))) as http:
        thread = threading.Thread(target=http.serve_forever)
        thread.start()
        origin = f"http://127.0.0.1:{http.server_port}"

        def request(resource, keys, update=None):
            url = origin + "/api/" + resource
            if update is None:
                req = Request(url + "?" + urlencode(keys))
            else:
                req = Request(url, data=json.dumps(dict(keys, **update)).encode(),
                              headers={"Content-Type": "application/json", "Origin": origin})
            with urlopen(req, timeout=5) as response:
                return json.load(response)

        try:
            yield pack, request
        finally:
            http.shutdown()
            thread.join()


def test_character_save_changes_one_field_and_refuses_a_stale_edit(editor):
    pack, request = editor
    path = pack / "data/character_catalog.ron"
    original = '''(
    presets: { "guide": (cast: (importance: Primary)) },
    characters: {
        "guide": (
            /* comment with ) }, /* nested comment */ and a comma */
            display_name: "Guide",
            authoring_description: r#"This is not a field: cast: (importance: Background), "#,
        ),
        "other": (display_name: "Other", cast: (importance: Background)),
    },
)'''
    path.write_text(original)
    keys = {"id": "fixture:guide"}
    opened = request("character-cast", keys)
    assert opened["cast"] == {"importance": "Unclassified", "roles": {}}
    update = {"revision": opened["revision"], "cast": {"importance": "Secondary", "roles": {"story": ["mentor", "ally"]}}}
    saved = request("character-cast", keys, update)
    assert saved["cast"]["roles"] == {"story": ["ally", "mentor"]}
    expected = '            cast: (importance: Secondary, roles: {"story": ["ally", "mentor"]}),\n'
    assert path.read_text().replace(expected, "", 1) == original
    with pytest.raises(HTTPError) as error:
        request("character-cast", keys, update)
    assert error.value.code == 409
    update = {"revision": saved["revision"], "cast": {"importance": "Primary", "roles": {}}}
    request("character-cast", keys, update)
    assert path.read_text().replace('            cast: (importance: Primary, roles: {}),\n', "", 1) == original


def test_room_music_writes_the_source_for_a_shared_room(editor):
    pack, request = editor
    source = pack / "library.ldtk"
    mount = pack / "worlds/library.ldtk"
    mount.symlink_to(source)
    fields = ["activeArea", "music_track", "fight_music_track"]
    defs = [{"identifier": name, "__type": "String", "uid": i, "canBeNull": True}
            for i, name in enumerate(fields, 1)]
    def level(name, uid, area, music=None):
        return {"identifier": name, "uid": uid, "pxWid": 64, "pxHei": 64, "layerInstances": [],
                "fieldInstances": [{"__identifier": field, "__type": "String", "__value": value, "defUid": i}
                                   for i, (field, value) in enumerate(zip(fields, [area, music, None]), 1)]}
    source.write_text(json.dumps({"nextUid": 7, "defs": {"levelFields": defs},
                                  "levels": [level("entry", 4, "hall", "calm"), level("balcony", 5, "hall"),
                                             level("attic", 6, "attic", "calm")]}))
    source.with_suffix(".generated.json").write_text(json.dumps({
        "schema": "ambition.ldtk_generated_record.v1", "generated": {"attic": "baseline"}, "ldtk_owned": []}))
    keys = {"world": "world:library", "room": "balcony"}
    opened = request("room-music", keys)
    assert opened["area"] == "hall"
    assert opened["tracks"] == {"calm": "Calm", "fight": "Fight"}
    assert opened["fields"]["music_track"]["value"] == "calm"
    original = source.read_bytes()
    with pytest.raises(HTTPError) as error:
        request("room-music", keys, {"revision": opened["revision"], "tracks": {"music_track": "unknown"}})
    assert error.value.code == 400
    assert source.read_bytes() == original

    saved = request("room-music", keys, {"revision": opened["revision"],
                                         "tracks": {"music_track": "", "fight_music_track": "fight"}})
    assert mount.is_symlink() and saved["levels"] == {
        "entry": {"music_track": "", "fight_music_track": "fight"},
        "balcony": {"music_track": "", "fight_music_track": ""}}
    assert request("room-music", keys)["fields"] == saved["fields"]
    generated = request("room-music", dict(keys, room="attic"))
    assert generated["fields"]["music_track"]["value"] == "calm"
    with pytest.raises(HTTPError) as error:
        request("room-music", dict(keys, room="attic"),
                {"revision": generated["revision"], "tracks": {"music_track": "fight"}})
    assert error.value.code == 400
    with pytest.raises(HTTPError) as error:
        request("room-music", keys, {"revision": opened["revision"], "tracks": {"music_track": "calm"}})
    assert error.value.code == 409
