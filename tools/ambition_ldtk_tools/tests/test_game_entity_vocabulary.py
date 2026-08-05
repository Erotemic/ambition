#!/usr/bin/env python3
"""A game may EXTEND the LDtk vocabulary; it may not extend it by accident.

`MaryOBlock` is Mary-O's own noun — the game installs a converter for it
through `install_ldtk_entity_converters`, and no other world has one. Making
validation accept it went through a wrong answer first, and this file pins the
right one.

The wrong answer was to accept any identifier the project itself DEFINES::

    if ident not in KNOWN_ENTITIES and ident not in entity_defs:
        error

An editor definition is not evidence that anything can convert it, and `defs`
is written by the same generator that writes the instances — so the check was
comparing the file against itself. A GPT 5.6 review reproduced the hole
directly: a `BogusEntity` definition plus an instance of it validated clean
with no converter anywhere in the engine or the game.

The right answer is a DECLARED manifest, which is what these tests pin:

- an identifier the engine knows is accepted, as always;
- an identifier a game manifest declares is accepted;
- an identifier with a full editor definition and NO manifest entry is
  REJECTED — the reproduction above, now red;
- a manifest that shadows an engine entity is refused, because "extend" and
  "override" are not the same permission.

The other half of the contract lives in Rust:
`ldtk_vocabulary::tests::the_declared_manifest_matches_the_converters_actually_installed`
asserts the manifest matches the converters the game really installs. Neither
list is derived from the other, so a lie in either one is a red test.
"""

from __future__ import annotations

import copy
import json
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
PKG_ROOT = REPO_ROOT / "tools" / "ambition_ldtk_tools"
sys.path.insert(0, str(PKG_ROOT))

from ambition_ldtk_tools.ldtk.paths import default_game_worlds_dir  # noqa: E402
from ambition_ldtk_tools.validate import validate  # noqa: E402

MARY_O = default_game_worlds_dir("ambition_demo_mary_o") / "mary_o.ldtk"
MANIFEST = MARY_O.with_name("mary_o.entities.json")


def unsupported_entity_errors(errors, identifier):
    return [e for e in errors if "unsupported entity" in e and identifier in e]


def write_project(directory: Path, project, *, manifest=None) -> Path:
    path = directory / "mary_o.ldtk"
    path.write_text(json.dumps(project))
    if manifest is not None:
        path.with_name("mary_o.entities.json").write_text(json.dumps(manifest))
    return path


def load_mary_o():
    return json.loads(MARY_O.read_text()), json.loads(MANIFEST.read_text())


def clone_entity(project, source_identifier, new_identifier, *, iid):
    """Copy a live instance under a new identifier, defs and all.

    Cloning rather than authoring from scratch means the copy inherits geometry
    the validator already accepts, so the only thing under test is whether the
    identifier is supported.
    """
    defs = project["defs"]["entities"]
    template = next(e for e in defs if e["identifier"] == source_identifier)
    new_def = copy.deepcopy(template)
    new_def["identifier"] = new_identifier
    new_def["uid"] = max(e["uid"] for e in defs) + 1000
    new_def["fieldDefs"] = []
    defs.append(new_def)

    for level in project["levels"]:
        for layer in level.get("layerInstances") or []:
            instances = layer.get("entityInstances") or []
            source = next(
                (i for i in instances if i["__identifier"] == source_identifier),
                None,
            )
            if source is None:
                continue
            clone = copy.deepcopy(source)
            clone["__identifier"] = new_identifier
            clone["defUid"] = new_def["uid"]
            clone["iid"] = iid
            clone["fieldInstances"] = []
            instances.append(clone)
            layer["entityInstances"] = instances
            return project
    raise AssertionError(f"no {source_identifier} instance to clone")


def test_a_declared_game_entity_is_accepted():
    """The level ships a `MaryOBlock`; its sidecar manifest declares one."""
    project, manifest = load_mary_o()
    with tempfile.TemporaryDirectory() as tmp:
        path = write_project(Path(tmp), project, manifest=manifest)
        errors, _ = validate(path)
    assert not unsupported_entity_errors(errors, "MaryOBlock"), errors


def test_an_undeclared_game_entity_is_rejected():
    """The same level, same file, with the manifest absent.

    This is what makes the acceptance above mean something: `MaryOBlock` is
    supported because it was DECLARED, not because it appears in the file.
    """
    project, _ = load_mary_o()
    with tempfile.TemporaryDirectory() as tmp:
        path = write_project(Path(tmp), project)
        errors, _ = validate(path)
    assert unsupported_entity_errors(errors, "MaryOBlock"), errors


def test_a_defined_entity_with_no_converter_is_still_rejected():
    """GPT 5.6's reproduction, kept red.

    `BogusEntity` gets a complete editor definition AND a valid instance, and
    the real manifest is present — everything the old check looked at says
    "fine". Nothing can convert it, so validation must refuse.
    """
    project, manifest = load_mary_o()
    clone_entity(
        project,
        "MaryOBlock",
        "BogusEntity",
        iid="bogus000-0000-0000-0000-000000000001",
    )
    with tempfile.TemporaryDirectory() as tmp:
        path = write_project(Path(tmp), project, manifest=manifest)
        errors, _ = validate(path)
    assert unsupported_entity_errors(errors, "BogusEntity"), errors


def test_a_manifest_may_not_shadow_an_engine_entity():
    """Extending the vocabulary is a game's business; redefining it is not.

    A game that claims `EnemySpawn` would be quietly asserting its converter
    wins, which is not a thing the manifest can grant — so say so at the
    manifest rather than letting the level look valid.
    """
    project, manifest = load_mary_o()
    manifest["entities"].append({"identifier": "EnemySpawn", "fields": []})
    with tempfile.TemporaryDirectory() as tmp:
        path = write_project(Path(tmp), project, manifest=manifest)
        errors, _ = validate(path)
    assert any(
        "redeclares 'EnemySpawn'" in e for e in errors
    ), errors


def test_an_explicit_manifest_flag_overrides_the_sidecar():
    """`--game-entities` is how a world validates against a manifest elsewhere."""
    project, manifest = load_mary_o()
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        path = write_project(tmp_path, project)  # no sidecar
        elsewhere = tmp_path / "somewhere-else.json"
        elsewhere.write_text(json.dumps(manifest))
        errors, _ = validate(path, game_entity_manifests=[elsewhere])
    assert not unsupported_entity_errors(errors, "MaryOBlock"), errors
