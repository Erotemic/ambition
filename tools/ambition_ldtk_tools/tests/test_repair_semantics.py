#!/usr/bin/env python3
"""Semantic regression tests for LDtk repair/canonicalization.

These tests pin behavior learned from an actual LDtk 1.5.3 editor save:

* default-equal field values may keep either ``realEditorValues: []`` or an
  explicit wrapper, and both should be accepted without churn;
* entity ``__worldX`` / ``__worldY`` are cached world coordinates and must be
  synchronized from ``level.worldX/worldY + entity.px``.
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.validate import (
    normalize_editor_values,
    normalize_project_for_editor,
)  # noqa: E402


def minimal_project(real_editor_values):
    return {
        "jsonVersion": "1.5.3",
        "iid": "test-world",
        "worldLayout": "Free",
        "defaultGridSize": 16,
        "defs": {
            "layers": [
                {
                    "identifier": "Ambition",
                    "__type": "Entities",
                    "parallaxFactorX": 0,
                    "parallaxFactorY": 0,
                    "parallaxScaling": True,
                    "canSelectWhenInactive": True,
                    "guideGridHei": 16,
                    "guideGridWid": 16,
                    "uiFilterTags": [],
                    "useAsyncRender": False,
                }
            ],
            "entities": [
                {
                    "identifier": "LoadingZone",
                    "uid": 1,
                    "color": "#FFD166",
                    "fieldDefs": [
                        {
                            "identifier": "activation",
                            "uid": 2,
                            "__type": "String",
                            "type": "F_String",
                            "defaultOverride": {"id": "V_String", "params": ["Door"]},
                        }
                    ],
                }
            ],
            "levelFields": [
                {
                    "identifier": "activeArea",
                    "uid": 3,
                    "__type": "String",
                    "type": "F_String",
                    "defaultOverride": None,
                }
            ],
        },
        "levels": [
            {
                "identifier": "offset_level",
                "worldX": -10000,
                "worldY": 32,
                "pxWid": 1024,
                "pxHei": 1024,
                "fieldInstances": [
                    {
                        "__identifier": "activeArea",
                        "__type": "String",
                        "__value": "offset_area",
                        "__tile": None,
                        "defUid": 3,
                        "realEditorValues": [
                            {"id": "V_String", "params": ["offset_area"]}
                        ],
                    }
                ],
                "layerInstances": [
                    {
                        "__identifier": "Ambition",
                        "__type": "Entities",
                        "__gridSize": 16,
                        "entityInstances": [
                            {
                                "__identifier": "LoadingZone",
                                "iid": "LoadingZone-test",
                                "defUid": 1,
                                "px": [16, 64],
                                "width": 48,
                                "height": 96,
                                "__grid": [1, 4],
                                "__pivot": [0, 0],
                                "__worldX": -9984,
                                "__worldY": 96,
                                "fieldInstances": [
                                    {
                                        "__identifier": "activation",
                                        "__type": "String",
                                        "__value": "Door",
                                        "__tile": None,
                                        "defUid": 2,
                                        "realEditorValues": copy.deepcopy(
                                            real_editor_values
                                        ),
                                    }
                                ],
                            }
                        ],
                    }
                ],
            }
        ],
    }


def assert_no_repair_for_default_equal_editor_values() -> None:
    wrapper = [{"id": "V_String", "params": ["Door"]}]
    for real_values in ([], wrapper):
        project = minimal_project(real_values)
        before = json.dumps(project, sort_keys=True, separators=(",", ":"))
        changed = normalize_editor_values(project)
        after = json.dumps(project, sort_keys=True, separators=(",", ":"))
        if before != after or changed:
            raise AssertionError(
                "default-equal field values should not be rewritten; "
                f"realEditorValues={real_values!r}, changed={changed!r}"
            )
    print("ok: default-equal realEditorValues [] and wrapper are both accepted")


def assert_world_coordinates_are_repaired() -> None:
    project = minimal_project([])
    entity = project["levels"][0]["layerInstances"][0]["entityInstances"][0]
    entity["__worldX"] = 16
    entity["__worldY"] = 64
    changes = normalize_project_for_editor(project)
    if "synchronized 2 cached entity __worldX/__worldY values" not in changes:
        raise AssertionError(f"expected cached world coord repair, got {changes!r}")
    if entity["__worldX"] != -9984 or entity["__worldY"] != 96:
        raise AssertionError(
            f"world coords not repaired: {entity['__worldX']}, {entity['__worldY']}"
        )
    print("ok: stale cached __worldX/__worldY values are repaired")


def main() -> int:
    assert_no_repair_for_default_equal_editor_values()
    assert_world_coordinates_are_repaired()
    print("all repair semantics tests passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
