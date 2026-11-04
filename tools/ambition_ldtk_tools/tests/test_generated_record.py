"""A generator does not overwrite a level that was edited in LDtk.

`area create --replace-existing` builds a level again from its spec. These
tests make a level in a copy of the sandbox world, record it as its generator
would, edit it as an author would in LDtk, and check that the next replace is
refused, that `--overwrite-hand-edits` still replaces it, and that an adopted
(LDtk-owned) level is refused too.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from ambition_ldtk_tools import generated_record
from ambition_ldtk_tools.ldtk.paths import default_sandbox_ldtk

PKG_ROOT = Path(__file__).resolve().parents[1]
LEVEL = "generated_record_test_level"
SPEC = {
    "id": "generated_record_test_area",
    "level_id": LEVEL,
    "world_x": 22000,
    "world_y": 1024,
    "px_wid": 512,
    "px_hei": 256,
    "fill_collision": "solid_floor",
    "entities": [
        {"type": "PlayerStart", "px": [32, 32], "size": [28, 46], "fields": {"name": "record_start"}},
        {"type": "Solid", "px": [0, 240], "size": [512, 16], "fields": {"name": "record_floor"}},
    ],
}


def area_create(world: Path, spec: Path, *extra: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        # No repair pass: the sandbox world's own validation findings would
        # fail the run, and the guard is what is under test.
        [sys.executable, "-m", "ambition_ldtk_tools", "area", "create", str(spec), "--ldtk", str(world), "--no-repair", *extra],
        cwd=PKG_ROOT, capture_output=True, text=True,
    )


def level_of(world: Path) -> dict:
    return next(lv for lv in json.loads(world.read_text())["levels"] if lv["identifier"] == LEVEL)


def edit_in_ldtk(world: Path) -> None:
    """Move the player start, as an author would in the editor."""
    project = json.loads(world.read_text())
    level = next(lv for lv in project["levels"] if lv["identifier"] == LEVEL)
    for layer in level["layerInstances"]:
        for entity in layer.get("entityInstances") or []:
            if entity["__identifier"] == "PlayerStart":
                entity["px"] = [entity["px"][0] + 64, entity["px"][1]]
    world.write_text(json.dumps(project))


class GeneratedRecordTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.world = self.tmp / "world.ldtk"
        shutil.copy(default_sandbox_ldtk(), self.world)
        self.spec = self.tmp / "spec.json"
        self.spec.write_text(json.dumps(SPEC))
        made = area_create(self.world, self.spec)
        self.assertEqual(made.returncode, 0, made.stdout + made.stderr)
        generated_record.record(self.world, [LEVEL])

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def test_a_level_as_its_generator_left_it_is_replaced(self):
        self.assertIsNone(generated_record.check(self.world, level_of(self.world)))
        # Put the level first, so that a replace that moves it to the end shows.
        project = json.loads(self.world.read_text())
        project["levels"].sort(key=lambda lv: lv["identifier"] != LEVEL)
        self.world.write_text(json.dumps(project))
        order = [lv["identifier"] for lv in project["levels"]]
        again = area_create(self.world, self.spec, "--replace-existing")
        self.assertEqual(again.returncode, 0, again.stdout + again.stderr)
        # It keeps its place in the file: an unchanged world is written unchanged.
        self.assertEqual([lv["identifier"] for lv in json.loads(self.world.read_text())["levels"]], order)

    def test_a_level_edited_in_ldtk_is_not_replaced(self):
        edit_in_ldtk(self.world)
        self.assertIn((LEVEL, "HAND-EDITED"), generated_record.status(self.world))
        refused = area_create(self.world, self.spec, "--replace-existing")
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn("changed since its generator last wrote it", refused.stdout + refused.stderr)
        # The edit is still there.
        start = next(e for layer in level_of(self.world)["layerInstances"] for e in layer.get("entityInstances") or []
                     if e["__identifier"] == "PlayerStart")
        self.assertEqual(start["px"][0], 32 + 64)

    def test_overwrite_hand_edits_discards_the_edit(self):
        edit_in_ldtk(self.world)
        forced = area_create(self.world, self.spec, "--replace-existing", "--overwrite-hand-edits")
        self.assertEqual(forced.returncode, 0, forced.stdout + forced.stderr)

    def test_an_adopted_level_belongs_to_ldtk(self):
        self.assertEqual(generated_record.main(["adopt", LEVEL, "--ldtk", str(self.world)]), 0)
        refused = area_create(self.world, self.spec, "--replace-existing")
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn("LDtk-owned", refused.stdout + refused.stderr)
        self.assertEqual(generated_record.main(["release", LEVEL, "--ldtk", str(self.world)]), 0)
        self.assertIsNone(generated_record.check(self.world, level_of(self.world)))

    def test_moving_a_level_is_not_an_edit(self):
        # The LDtk editor owns placement (`area create --keep-position`).
        level = level_of(self.world)
        before = generated_record.fingerprint(level)
        level["worldX"] += 2048
        level["iid"] = "moved"
        self.assertEqual(generated_record.fingerprint(level), before)


if __name__ == "__main__":
    unittest.main()
