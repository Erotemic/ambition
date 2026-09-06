from __future__ import annotations

from ambition_ldtk_tools.edit.layer_split_entities import main as split_main


def mini_project_text() -> str:
    return """
{
  "nextUid": 100,
  "defs": {
    "layers": [
      {"__type": "Entities", "type": "Entities", "identifier": "Ambition", "uid": 1, "requiredTags": [], "excludedTags": []},
      {"__type": "Entities", "type": "Entities", "identifier": "AmbitionCameras", "uid": 2, "requiredTags": [], "excludedTags": []}
    ],
    "entities": [
      {"identifier": "CameraZone", "uid": 10, "tags": []},
      {"identifier": "LoadingZone", "uid": 11, "tags": []}
    ]
  },
  "levels": [
    {
      "identifier": "symmetry_room",
      "worldX": 0,
      "worldY": 0,
      "pxWid": 320,
      "pxHei": 240,
      "layerInstances": [
        {
          "__identifier": "Ambition",
          "__type": "Entities",
          "layerDefUid": 1,
          "iid": "Ambition-inst",
          "entityInstances": [
            {"__identifier": "CameraZone", "iid": "CameraZone-1", "px": [0, 0], "width": 64, "height": 64, "fieldInstances": []},
            {"__identifier": "LoadingZone", "iid": "LoadingZone-1", "px": [16, 16], "width": 64, "height": 64, "fieldInstances": []}
          ]
        },
        {
          "__identifier": "AmbitionCameras",
          "__type": "Entities",
          "layerDefUid": 2,
          "iid": "AmbitionCameras-inst",
          "entityInstances": []
        }
      ]
    }
  ]
}
"""


def test_split_entities_moves_and_noop_does_not_fail(tmp_path) -> None:
    path = tmp_path / "mini.ldtk"
    path.write_text(mini_project_text())

    assert split_main([
        "split-entities",
        str(path),
        "--type",
        "CameraZone",
        "--from-layer",
        "Ambition",
        "--to-layer",
        "AmbitionCameras",
        "--in-place",
    ]) == 0

    after_first = path.read_text()
    assert "CameraZone-1" in after_first

    # A second pass should be a no-op and, importantly, should not invoke full
    # repair/validation. Real sandbox files may contain cross-LDtk links that
    # validate only in a multi-file context.
    assert split_main([
        "split-entities",
        str(path),
        "--type",
        "CameraZone",
        "--from-layer",
        "Ambition",
        "--to-layer",
        "AmbitionCameras",
        "--in-place",
    ]) == 0
    assert path.read_text() == after_first
