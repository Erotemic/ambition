#!/usr/bin/env python3
"""Build the yard and the bridge with the existing LDtk authoring tools."""

from pathlib import Path
import json
import os
import sys

PROJECT = Path(__file__).resolve().parent
REPO = PROJECT.parents[2]
TOOLS = str(REPO / "tools/ambition_ldtk_tools")
sys.path.insert(0, TOOLS)
os.environ["PYTHONPATH"] = os.pathsep.join(filter(None, [TOOLS, os.environ.get("PYTHONPATH")]))

from ambition_ldtk_tools.area.plan import ReplaceExistingLevelOp
from ambition_ldtk_tools.area_authoring import compile_area_create_plan
from ambition_ldtk_tools.edit.postprocess import run_repair_and_validate
from ambition_ldtk_tools.ldtk.transaction import LdtkTransaction


def main() -> int:
    target = PROJECT / "worlds/warden.ldtk"
    target.parent.mkdir(parents=True, exist_ok=True)
    template = REPO / "game/ambition_content/assets/worlds/sandbox.ldtk"
    tx = LdtkTransaction(target if target.exists() else template, output=target)
    if not target.exists():
        for index in reversed(range(len(tx.project["levels"]))):
            ReplaceExistingLevelOp(index).apply(tx.project)
    for room in ("yard", "bridge"):
        spec = json.loads((PROJECT / f"worlds/{room}.json").read_text())
        plan, _, _ = compile_area_create_plan(tx.project, spec, replace_existing=True)
        plan.apply(tx.project)
    tx.note_changed()
    tx.finish(write_message="wrote {path}")
    return run_repair_and_validate(target, REPO / "tools/ambition_ldtk_tools/schemas/ldtk/JSON_SCHEMA.json")


if __name__ == "__main__":
    raise SystemExit(main())
