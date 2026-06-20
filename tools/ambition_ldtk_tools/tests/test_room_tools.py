from __future__ import annotations

import json
import tarfile
from pathlib import Path

from ambition_ldtk_tools.room import (
    DEFAULT_LDTK,
    format_summary_text,
    render_room_png,
    render_room_svg,
    room_summary,
    write_bundle,
)


def test_room_summary_describes_symmetry_room() -> None:
    project = json.loads(DEFAULT_LDTK.read_text())
    summary = room_summary(project, "symmetry_room")
    assert summary["identifier"] == "symmetry_room"
    assert summary["size"] == [1280, 1280]
    assert summary["entity_counts"]["GravityZone"] == 4
    assert len(summary["player_starts"]) == 1
    text = format_summary_text(summary)
    assert "Gravity zones (4)" in text
    assert "Collision" in text


def test_room_render_svg_and_png(tmp_path: Path) -> None:
    project = json.loads(DEFAULT_LDTK.read_text())
    svg = render_room_svg(project, "symmetry_room")
    assert svg.startswith("<svg")
    assert "grav_down" in svg
    png_path = tmp_path / "symmetry_room.png"
    render_room_png(project, "symmetry_room", png_path, max_width=256)
    assert png_path.read_bytes().startswith(b"\x89PNG\r\n\x1a\n")


def test_room_bundle_debug_contains_chat_artifacts(tmp_path: Path) -> None:
    project = json.loads(DEFAULT_LDTK.read_text())
    out = tmp_path / "bundle.tar.gz"
    write_bundle(
        project=project,
        ldtk=DEFAULT_LDTK,
        level_id="symmetry_room",
        out=out,
        repo_root=DEFAULT_LDTK.parents[5],
        render_format="svg",
        include_debug=False,
        run_validate=False,
    )
    with tarfile.open(out, "r:gz") as tar:
        names = set(tar.getnames())
    assert "room_describe.txt" in names
    assert "room_describe.json" in names
    assert "room.svg" in names
    assert "README.txt" in names
