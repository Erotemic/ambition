#!/usr/bin/env python3
"""A vocabulary that can change after the level is authored, or it cannot change.

`sync_mary_o_ldtk_defs.py` promised to be the re-runnable half of Mary-O's
authoring workflow — `author_mary_o_ldtk.py` rebuilds the level and must never
run twice, so definitions needed a path that could. It shelled out to
`def register-entity`, which refuses an identifier the project already has, and
then matched that refusal's own error text and reported success. After the
first run the manifest was decorative: a new field, a moved default, a wider
block, all silently discarded, all announced as synchronized.

`def upsert-entity` is the real operation, and the thing it has to get right is
that an LDtk file is a graph held together by uids. Every `entityInstance`
points at its definition by `defUid` and every `fieldInstance` points at its
field definition the same way, so the naive fix — delete the def, register it
again — parses fine and empties the level. So these pin the uids, and they pin
the values that hang off them: a `MaryOBlock` authored as a `Brick` is still a
`Brick` after the block grows a field.

⚠ **the assertions here are derived from the level, not spelled out.** Three
literals in this file have gone stale as Mary-O's block vocabulary changed
underneath them — `contents` arriving, `Question`/`Hidden` arriving, and
`Quasar` leaving to become a `contents` value. Each red cost a triage that found
nothing wrong with the tool. What the tool owes is a property; which kinds a
level authors is the level's business.

The other half is what the tool must NOT do quietly. Retiring a field and
changing a field's type both delete the `fieldInstance` records that carried
values — a def-less `fieldInstance` fails validation outright, and one whose
`__type` no longer matches its `__value` is a lie `repair` would go on to
launder into the file. When instances actually hold values, that is a decision
about authored content, so the tool stops and names what dies. The opt-in is
per field path and is refused once there is nothing left to lose, which is what
keeps a one-time migration from becoming a flag that lives in a script forever.
"""

from __future__ import annotations

import copy
import json
import shutil
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
PKG_ROOT = REPO_ROOT / "tools" / "ambition_ldtk_tools"
sys.path.insert(0, str(PKG_ROOT))

from ambition_ldtk_tools.edit.upsert_entity import (  # noqa: E402
    apply_upsert,
    main as upsert_main,
    plan_losses,
)
from ambition_ldtk_tools.ldtk.paths import game_world_ldtk  # noqa: E402
from ambition_ldtk_tools.validate import validate  # noqa: E402

MARY_O = game_world_ldtk("ambition_demo_mary_o", "mary_o")
MANIFEST = MARY_O.with_name("mary_o.entities.json")


def load_mary_o():
    return json.loads(MARY_O.read_text()), json.loads(MANIFEST.read_text())


def block_def(project):
    return next(
        e for e in project["defs"]["entities"] if e["identifier"] == "MaryOBlock"
    )


def block_instances(project):
    return [
        inst
        for level in project["levels"]
        for layer in level.get("layerInstances") or []
        for inst in layer.get("entityInstances") or []
        if inst["__identifier"] == "MaryOBlock"
    ]


def kind_values(project):
    return [
        field["__value"]
        for inst in block_instances(project)
        for field in inst["fieldInstances"]
        if field["__identifier"] == "kind"
    ]


@pytest.fixture
def staged(tmp_path):
    """The real level and its real manifest, copied somewhere destructible.

    Working from the shipped file rather than a synthetic one means the uids
    and the authored `kind` values under test are the ones that would actually
    be lost.
    """
    ldtk = tmp_path / "mary_o.ldtk"
    shutil.copy2(MARY_O, ldtk)
    shutil.copy2(MANIFEST, tmp_path / "mary_o.entities.json")
    return ldtk


def test_an_upsert_keeps_the_uids_every_placement_references():
    """The whole reason this cannot be a delete-and-re-register.

    The spec grows a field and moves the default — an ordinary vocabulary
    edit — and afterwards the entity def, the field def, and every instance's
    `defUid` must still be the same numbers, because the level references
    nothing else.
    """
    project, manifest = load_mary_o()
    before = block_def(project)
    entity_uid, kind_uid = before["uid"], before["fieldDefs"][0]["uid"]

    manifest["entities"][0]["fields"][0]["default"] = "Brick"
    manifest["entities"][0]["fields"].append(
        {"name": "respawns", "type": "Bool", "default": True}
    )
    changes = apply_upsert(project, manifest["entities"])

    after = block_def(project)
    assert after["uid"] == entity_uid, changes
    kind = next(f for f in after["fieldDefs"] if f["identifier"] == "kind")
    assert kind["uid"] == kind_uid, changes
    assert kind["defaultOverride"] == {"id": "V_String", "params": ["Brick"]}
    # Two of us fixed this independently and the same way, which is a decent sign it was the fix.
    expected = [f["name"] for f in manifest["entities"][0]["fields"]]
    assert [f["identifier"] for f in after["fieldDefs"]] == expected, (
        "every field the manifest declares survives the upsert, in order"
    )
    assert all(inst["defUid"] == entity_uid for inst in block_instances(project))


def test_a_compatible_change_leaves_the_authored_values_alone():
    """A compatible schema change must preserve authored field values.

    The fixture only requires multiple distinct authored kinds; it must not pin a
    particular content vocabulary value.
    """
    project, manifest = load_mary_o()
    original = kind_values(project)
    assert len(set(original)) > 1, (
        "the fixture no longer exercises what it claims: every block authors the "
        "same kind, so an upsert that collapsed them all would still pass"
    )

    manifest["entities"][0]["width"] = 48
    manifest["entities"][0]["fields"][0]["default"] = "Brick"
    apply_upsert(project, manifest["entities"])

    assert kind_values(project) == original
    assert block_def(project)["width"] == 48


def test_retiring_a_field_the_level_still_uses_is_reported_not_done():
    """The tool names the field and the distinct values that would die."""
    project, manifest = load_mary_o()
    manifest["entities"][0]["fields"] = []

    losses = plan_losses(project, manifest["entities"])

    assert [loss.path for loss in losses] == [
        "MaryOBlock.kind",
        "MaryOBlock.contents",
    ]
    assert losses[0].count == len(kind_values(project))
    # derived from the level, not spelled out. The literal set here has
    # gone stale twice — `Question`/`Hidden` joined when 1-2 got its own block
    # row, and `Quasar` LEFT when it became a `contents` value on a Brick. What
    # the tool owes is that it reports every distinct authored value, quoted;
    # which values a level happens to author is the level's business.
    assert set(losses[0].values) == {f"'{value}'" for value in set(kind_values(project))}
    assert len(set(losses[0].values)) > 1, (
        "one distinct value means this assertion cannot tell a tool that reports "
        "all of them from one that reports the first"
    )


def test_retyping_a_field_the_level_still_uses_is_reported_not_done():
    """A type change is the other way a `fieldInstance` stops being valid.

    Nothing coerces an authored kind string into an `Int`, and inventing a value
    would be worse than saying so.
    """
    project, manifest = load_mary_o()
    manifest["entities"][0]["fields"][0]["type"] = "Int"
    manifest["entities"][0]["fields"][0]["default"] = 0

    losses = plan_losses(project, manifest["entities"])

    assert [loss.path for loss in losses] == ["MaryOBlock.kind"]
    # the DESTINATION type, not the source's spelling.
    assert "to Int" in losses[0].reason


def test_retiring_a_field_nothing_ever_set_is_silent():
    """The gate is on data at risk, not on how alarming the diff looks.

    A field added to the manifest and never authored onto an instance can be
    taken away again without a ceremony, and demanding one would train the
    operator to reach for the opt-in reflexively.
    """
    project, manifest = load_mary_o()
    manifest["entities"][0]["fields"].append(
        {"name": "unused", "type": "Float", "default": 1.0}
    )
    apply_upsert(project, manifest["entities"])
    assert "unused" in [f["identifier"] for f in block_def(project)["fieldDefs"]]

    manifest["entities"][0]["fields"].pop()
    assert plan_losses(project, manifest["entities"]) == []
    apply_upsert(project, manifest["entities"])
    assert "unused" not in [f["identifier"] for f in block_def(project)["fieldDefs"]]


def test_an_authorized_retype_keeps_the_uid_and_drops_the_records():
    """What the opt-in actually buys, so the report is not a bluff.

    The field def survives with its uid — it is the same field, differently
    typed — and the stale `fieldInstance` records are gone rather than left to
    claim an `Int` that reads a kind string.
    """
    project, manifest = load_mary_o()
    kind_uid = block_def(project)["fieldDefs"][0]["uid"]
    manifest["entities"][0]["fields"][0]["type"] = "Int"
    manifest["entities"][0]["fields"][0]["default"] = 0

    apply_upsert(project, manifest["entities"])

    kind = next(f for f in block_def(project)["fieldDefs"] if f["identifier"] == "kind")
    assert kind["uid"] == kind_uid
    assert kind["__type"] == "Int"
    assert kind_values(project) == []
    assert block_instances(project), "the instances themselves must survive"


def test_a_stale_opt_in_is_refused(staged, capsys):
    """The flag cannot outlive the migration it was written for.

    A blanket `--force` written for one change silently authorizes the next
    one. A path-scoped opt-in that is refused when nothing at that path is at
    risk cannot: leave it in a script and the next run goes red.
    """
    rc = upsert_main(
        [
            str(staged.with_name("mary_o.entities.json")),
            "--ldtk",
            str(staged),
            "--game-owned",
            "--dry-run",
            "--drop-instance-values",
            "MaryOBlock.kind",
        ]
    )
    assert rc == 2
    assert "no instance value would be lost" in capsys.readouterr().err


def test_the_synced_file_still_opens_and_re_running_is_a_no_op(staged):
    """The two promises the script makes, end to end through the real command.

    The run swaps `kind` for `respawns`, which is the shape that touches both
    sides: a field def appears, another goes, and nine `fieldInstance` records
    have to go with it. A `fieldInstance` whose def is gone is not a cosmetic
    leftover — validation rejects it outright and LDtk would be opening a file
    that references nothing — so the post-pass runs and the result is validated
    again here.

    Then it runs a second time and the bytes have to be identical, because
    being re-runnable is the entire distinction from `author_mary_o_ldtk.py`.
    """
    manifest_path = staged.with_name("mary_o.entities.json")
    manifest = json.loads(manifest_path.read_text())
    manifest["entities"][0]["fields"] = [
        {"name": "respawns", "type": "Bool", "default": True}
    ]
    manifest_path.write_text(json.dumps(manifest))
    argv = [
        str(manifest_path),
        "--ldtk",
        str(staged),
        "--game-owned",
        "--in-place",
        "--drop-instance-values",
        "MaryOBlock.kind",
        # `contents` carries authored values now too (the cammo blocks), so
        # retiring the whole field list costs two fields rather than one.
        "--drop-instance-values",
        "MaryOBlock.contents",
    ]

    assert upsert_main(argv) == 0
    errors, _ = validate(staged)
    assert not errors, errors

    synced = staged.read_bytes()
    # The opt-in is spent now, so the re-run is the bare command.
    assert upsert_main(argv[:-4]) == 0
    assert staged.read_bytes() == synced


def test_the_manifest_the_repo_ships_is_already_synced():
    """Mary-O's committed level and manifest agree right now.

    This is the state the fix had to reach and the one a future manifest edit
    is measured against: run the script, get no diff. It is also the cheapest
    place to notice that someone changed the manifest without syncing.
    """
    project, manifest = load_mary_o()
    assert plan_losses(project, manifest["entities"]) == []
    assert apply_upsert(copy.deepcopy(project), manifest["entities"]) == []
