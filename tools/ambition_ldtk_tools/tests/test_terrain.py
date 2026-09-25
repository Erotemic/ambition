"""The painted-terrain palette, its rasterizer and its LDtk layers."""

from __future__ import annotations

import re
from pathlib import Path

import pytest

from ambition_ldtk_tools import terrain

REPO = Path(__file__).resolve().parents[3]
RUST_PALETTE = REPO / "crates/ambition_platformer2d_ldtk/src/terrain.rs"


def test_the_palette_is_the_rust_loaders_palette():
    """Two copies of one table: the loader reads these values, the tools paint
    them. A value painted here that the loader reads differently is a slope
    that silently changes direction."""
    source = RUST_PALETTE.read_text()
    rows = re.findall(
        r'\((\d+), "(\w+)", (Full|F \{ left: (\d), right: (\d) \}|C \{ left: (\d), right: (\d) \})\)',
        source,
    )
    assert rows, "the Rust palette's shape changed; update this parse"
    rust = []
    for value, name, shape, fl, fr, cl, cr in rows:
        if shape == "Full":
            rust.append((int(value), name, "full", 4, 4))
        elif fl:
            rust.append((int(value), name, "floor", int(fl), int(fr)))
        else:
            rust.append((int(value), name, "ceiling", int(cl), int(cr)))
    python = [(c.value, c.name, c.kind, c.left, c.right) for c in terrain.PALETTE]
    assert rust == python


def hill(x: float) -> float:
    return terrain.cosine_profile([(0, 400), (160, 400), (480, 240), (640, 240), (960, 400)])(x)


def test_ground_follows_a_hill_with_palette_cells_only():
    cells = terrain.ground(hill, 0, 60, 16, bottom=40)
    values = {c.value for c in terrain.PALETTE}
    assert set(cells.values()) <= values
    # The surface stays within a cell of the profile it was drawn from.
    heights = terrain.quantize(hill, 0, 60, 16)
    for cx, h in enumerate(heights):
        assert abs(h * 4 - hill(cx * 16)) <= 16, (cx, h * 4, hill(cx * 16))
    # Flats sit on cell boundaries: the palette has no half-height flat.
    for a, b in zip(heights, heights[1:]):
        if a == b:
            assert a % terrain.Q == 0


def test_a_road_band_has_an_underside_that_mirrors_its_top():
    cells = terrain.band(hill, 0, 60, 16, thickness=1)
    kinds = {next(c.kind for c in terrain.PALETTE if c.value == v) for v in cells.values()}
    assert "floor" in kinds and "ceiling" in kinds


def test_a_ceiling_hangs_down_to_its_profile():
    cells = terrain.ceiling(hill, 0, 60, 16, top=0)
    assert (0, 0) in cells and cells[(0, 0)] == terrain.FULL
    assert any(
        next(c.kind for c in terrain.PALETTE if c.value == v) == "ceiling" for v in cells.values()
    )


def minimal_project() -> dict:
    return {
        "defaultGridSize": 16,
        "nextUid": 10,
        "defs": {
            "tilesets": [],
            "layers": [{"identifier": "Collision"}, {"identifier": "Ambition"}],
        },
        "levels": [{
            "identifier": "room", "iid": "abc", "uid": 1, "pxWid": 160, "pxHei": 64,
            "layerInstances": [{"__identifier": "Collision"}, {"__identifier": "Ambition"}],
        }],
    }


def test_the_layers_paint_their_cells_and_draw_them(tmp_path):
    project = minimal_project()
    terrain.ensure_painted_layers(project, tmp_path / "room.ldtk")
    assert (tmp_path / terrain.PALETTE_PNG).exists()
    order = [layer["identifier"] for layer in project["defs"]["layers"]]
    assert order == ["Collision", "Ambition", "Terrain", "Track"], "under the entities"
    instances = [i["__identifier"] for i in project["levels"][0]["layerInstances"]]
    assert instances == order, "instances follow the defs' order"
    terrain.ensure_painted_layers(project, tmp_path / "room.ldtk")
    assert len(project["defs"]["layers"]) == 4, "idempotent"

    terrain.paint(project, "room", "Terrain", {(0, 3): 1, (1, 3): 1, (2, 2): 2})
    instance = next(
        i for i in project["levels"][0]["layerInstances"] if i["__identifier"] == "Terrain"
    )
    assert instance["intGridCsv"][3 * 10 + 1] == 1
    assert instance["intGridCsv"][2 * 10 + 2] == 2
    assert len(instance["autoLayerTiles"]) == 3
    slope = next(t for t in instance["autoLayerTiles"] if t["px"] == [32, 32])
    assert slope["t"] == terrain.tile_id("Terrain", 2)

    with pytest.raises(ValueError, match="palette"):
        terrain.paint(project, "room", "Track", {(0, 0): 99})


def test_a_flat_that_falls_mid_cell_stays_flat_on_a_cell_line():
    """1740 px is 108.75 cells: no palette flat lies there. A greedy rounder
    zig-zagged a quarter up and down across it — a sawtooth floor no loop can
    attach to. The floor must pick a cell line and stay on it."""
    heights = terrain.quantize(lambda x: 1740.0, 0, 50, 16)
    assert len(set(heights)) == 1, heights
    assert heights[0] % terrain.Q == 0


def test_a_hill_never_reverses_direction_mid_slope():
    heights = terrain.quantize(hill, 0, 60, 16)
    steps = [b - a for a, b in zip(heights, heights[1:])]
    signs = [s for s in steps if s]
    reversals = sum(1 for a, b in zip(signs, signs[1:]) if a * b < 0)
    # Up, then down: one reversal at the crest, none on the way.
    assert reversals <= 2, steps


def test_a_mid_cell_flat_is_not_paved_with_trapezoids():
    """Up, flat, down, flat reverses too: a road at 1160 px (a half cell off a
    cell line) came out as a row of bumps with a vertex on every cell line."""
    heights = terrain.quantize(lambda x: 1160.0, 0, 50, 16)
    signs = [b - a for a, b in zip(heights, heights[1:]) if b != a]
    reversals = sum(1 for a, b in zip(signs, signs[1:]) if a * b < 0)
    assert reversals == 0, heights
    # Equidistant from two cell lines, it may change line once — not bump.
    assert len(set(h for h in heights if h % terrain.Q == 0)) <= 2, heights
