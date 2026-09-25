"""Painted terrain: the `Terrain` and `Track` IntGrid layers.

A level paints ground cell by cell from a small slope palette; the engine's
LDtk loader (`crates/ambition_platformer2d_ldtk/src/terrain.rs`) traces the
painted outline into rideable surface chains. `Terrain` is solid all round;
`Track` lowers only its upward-facing runs, a road you can jump up through.

This module owns the LDtk side of that contract:

- ``PALETTE`` — the IntGrid values, mirrored from the Rust ``TERRAIN_PALETTE``
  (``tests/test_terrain.py`` fails if the two drift).
- ``render_palette_png`` — the tileset that draws each value as its shape, so
  the editor shows slopes as slopes.
- ``ensure_painted_layers`` — the tileset def, both layer defs (with one
  auto-rule per value, so painting in the editor shows the tile at once), and
  an empty instance of each layer in every level.
- ``paint`` — write a level's cells and the matching auto-layer tiles.
- ``ground`` / ``band`` / ``ceiling`` — rasterize a height profile into cells
  from the palette, for scripts that author a level from key points.

The palette is an LDtk-side limitation only: a level that needs a slope it
lacks can still author a ``SurfaceChain`` polyline directly.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Iterable

#: Quarter cells per cell edge: every slope height is a multiple of a quarter.
Q = 4

TERRAIN_LAYER = "Terrain"
TRACK_LAYER = "Track"
TILESET_IDENTIFIER = "terrain_palette"
PALETTE_PNG = "terrain_palette.png"


@dataclass(frozen=True)
class Cell:
    """One palette entry. ``kind`` is ``full``, ``floor`` or ``ceiling``.

    Floor ``left``/``right`` are the solid's height at the cell's left and right
    edges, in quarters, measured up from the cell's bottom; ceiling ones are
    its depth measured down from the top.
    """

    value: int
    name: str
    kind: str
    left: int = Q
    right: int = Q


PALETTE: tuple[Cell, ...] = (
    Cell(1, "Ground", "full"),
    Cell(2, "Up45", "floor", 0, 4),
    Cell(3, "Down45", "floor", 4, 0),
    Cell(4, "Up22a", "floor", 0, 2),
    Cell(5, "Up22b", "floor", 2, 4),
    Cell(6, "Down22a", "floor", 4, 2),
    Cell(7, "Down22b", "floor", 2, 0),
    Cell(8, "Up11a", "floor", 0, 1),
    Cell(9, "Up11b", "floor", 1, 2),
    Cell(10, "Up11c", "floor", 2, 3),
    Cell(11, "Up11d", "floor", 3, 4),
    Cell(12, "Down11a", "floor", 4, 3),
    Cell(13, "Down11b", "floor", 3, 2),
    Cell(14, "Down11c", "floor", 2, 1),
    Cell(15, "Down11d", "floor", 1, 0),
    Cell(16, "CeilDown45", "ceiling", 0, 4),
    Cell(17, "CeilUp45", "ceiling", 4, 0),
    Cell(18, "CeilDown22a", "ceiling", 0, 2),
    Cell(19, "CeilDown22b", "ceiling", 2, 4),
    Cell(20, "CeilUp22a", "ceiling", 4, 2),
    Cell(21, "CeilUp22b", "ceiling", 2, 0),
    Cell(22, "CeilDown11a", "ceiling", 0, 1),
    Cell(23, "CeilDown11b", "ceiling", 1, 2),
    Cell(24, "CeilDown11c", "ceiling", 2, 3),
    Cell(25, "CeilDown11d", "ceiling", 3, 4),
    Cell(26, "CeilUp11a", "ceiling", 4, 3),
    Cell(27, "CeilUp11b", "ceiling", 3, 2),
    Cell(28, "CeilUp11c", "ceiling", 2, 1),
    Cell(29, "CeilUp11d", "ceiling", 1, 0),
)

FLOOR_BY_HEIGHTS = {(c.left, c.right): c.value for c in PALETTE if c.kind == "floor"}
CEILING_BY_DEPTHS = {(c.left, c.right): c.value for c in PALETTE if c.kind == "ceiling"}
FULL = 1

#: Per-layer look: (earth fill, surface line) RGBA. Terrain is teal like the
#: in-game ground; a Track is amber so a road you can jump through reads as one.
STYLES = {
    TERRAIN_LAYER: ((31, 90, 102, 255), (111, 227, 240, 255)),
    TRACK_LAYER: ((138, 106, 42, 255), (240, 198, 111, 255)),
}


def cell_polygon(cell: Cell, grid: int) -> list[tuple[float, float]]:
    """The cell's solid in pixels, cell origin at (0, 0), y down."""
    s = grid / Q
    if cell.kind == "full":
        pts = [(0, 0), (Q, 0), (Q, Q), (0, Q)]
    elif cell.kind == "floor":
        pts = [(0, Q - cell.left), (Q, Q - cell.right), (Q, Q), (0, Q)]
    else:
        pts = [(0, 0), (Q, 0), (Q, cell.right), (0, cell.left)]
    return [(x * s, y * s) for x, y in pts]


def tile_id(layer: str, value: int) -> int:
    """Row 0 of the tileset is Terrain, row 1 Track; column ``value - 1``."""
    row = 0 if layer == TERRAIN_LAYER else 1
    return row * len(PALETTE) + (value - 1)


def render_palette_png(path: Path, grid: int = 16) -> None:
    """Draw every palette value, once per layer style, as a tileset PNG."""
    from PIL import Image, ImageDraw

    width, height = grid * len(PALETTE), grid * 2
    # Supersample so a 45° edge is clean, then downsample.
    k = 8
    image = Image.new("RGBA", (width * k, height * k), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    for row, layer in enumerate((TERRAIN_LAYER, TRACK_LAYER)):
        fill, line = STYLES[layer]
        for cell in PALETTE:
            ox, oy = (cell.value - 1) * grid, row * grid
            poly = [((ox + x) * k, (oy + y) * k) for x, y in cell_polygon(cell, grid)]
            draw.polygon(poly, fill=fill)
            s = grid / Q
            if cell.kind == "floor":
                a = (ox, oy + (Q - cell.left) * s)
                b = (ox + grid, oy + (Q - cell.right) * s)
            elif cell.kind == "ceiling":
                a = (ox, oy + cell.left * s)
                b = (ox + grid, oy + cell.right * s)
            else:
                continue
            draw.line([(a[0] * k, a[1] * k), (b[0] * k, b[1] * k)], fill=line, width=2 * k)
    image = image.resize((width, height), Image.LANCZOS)
    path.parent.mkdir(parents=True, exist_ok=True)
    image.save(path)


def _alloc_uid(project: dict) -> int:
    uid = int(project.get("nextUid", 1))
    project["nextUid"] = uid + 1
    return uid


def _layer_def(project: dict, identifier: str, tileset_uid: int, grid: int, doc: str) -> dict:
    fill, _ = STYLES[identifier]
    color = "#{:02X}{:02X}{:02X}".format(*fill[:3])
    rules = []
    for cell in PALETTE:
        rules.append({
            "uid": _alloc_uid(project), "active": True, "size": 1,
            "tileRectsIds": [[tile_id(identifier, cell.value)]], "alpha": 1,
            "chance": 1, "breakOnMatch": True, "pattern": [cell.value],
            "flipX": False, "flipY": False, "xModulo": 1, "yModulo": 1,
            "xOffset": 0, "yOffset": 0, "tileXOffset": 0, "tileYOffset": 0,
            "tileRandomXMin": 0, "tileRandomXMax": 0, "tileRandomYMin": 0,
            "tileRandomYMax": 0, "checker": "None", "tileMode": "Single",
            "pivotX": 0, "pivotY": 0, "outOfBoundsValue": None,
            "invalidated": False, "perlinActive": False, "perlinSeed": 0,
            "perlinScale": 0.2, "perlinOctaves": 2,
        })
    return {
        "__type": "IntGrid", "identifier": identifier, "type": "IntGrid",
        "uid": _alloc_uid(project), "doc": doc, "uiColor": None,
        "gridSize": grid, "guideGridWid": 0, "guideGridHei": 0,
        "displayOpacity": 1, "inactiveOpacity": 0.6, "hideInList": False,
        "hideFieldsWhenInactive": True, "canSelectWhenInactive": True,
        "renderInWorldView": True, "pxOffsetX": 0, "pxOffsetY": 0,
        "parallaxFactorX": 0, "parallaxFactorY": 0, "parallaxScaling": True,
        "requiredTags": [], "excludedTags": [],
        "autoTilesKilledByOtherLayerUid": None, "uiFilterTags": [],
        "useAsyncRender": False,
        "intGridValues": [
            {
                "value": cell.value, "identifier": cell.name, "color": color,
                "tile": {"tilesetUid": tileset_uid, "x": (cell.value - 1) * grid,
                         "y": 0 if identifier == TERRAIN_LAYER else grid,
                         "w": grid, "h": grid},
                "groupUid": 0,
            }
            for cell in PALETTE
        ],
        "intGridValuesGroups": [],
        "autoRuleGroups": [{
            "uid": _alloc_uid(project), "name": "palette", "color": None,
            "icon": None, "active": True, "isOptional": False, "rules": rules,
            "usesWizard": False, "collapsed": True,
            "biomeRequirementMode": 0, "requiredBiomeValues": [],
        }],
        "autoSourceLayerDefUid": None, "tilesetDefUid": tileset_uid,
        "tilePivotX": 0, "tilePivotY": 0, "biomeFieldUid": None,
    }


def ensure_painted_layers(project: dict, ldtk_path: Path) -> None:
    """Add the palette tileset, the ``Terrain`` and ``Track`` layer defs, and an
    empty instance of each in every level. Idempotent.

    The PNG is (re)written beside the ``.ldtk`` so the editor finds it.
    """
    grid = int(project.get("defaultGridSize", 16))
    render_palette_png(ldtk_path.parent / PALETTE_PNG, grid)
    tilesets = project["defs"]["tilesets"]
    tileset = next((t for t in tilesets if t["identifier"] == TILESET_IDENTIFIER), None)
    if tileset is None:
        tileset = {
            "__cWid": len(PALETTE), "__cHei": 2, "identifier": TILESET_IDENTIFIER,
            "uid": _alloc_uid(project), "relPath": PALETTE_PNG, "embedAtlas": None,
            "pxWid": grid * len(PALETTE), "pxHei": grid * 2, "tileGridSize": grid,
            "spacing": 0, "padding": 0, "tags": [], "tagsSourceEnumUid": None,
            "enumTags": [], "customData": [], "savedSelections": [],
            "cachedPixelData": None,
        }
        tilesets.append(tileset)
    layers = project["defs"]["layers"]
    docs = {
        TERRAIN_LAYER: "Painted ground: full earth and slope cells, traced into "
        "rideable surfaces. Solid all round.",
        TRACK_LAYER: "Painted road: the same palette, but only its top is "
        "ridden, so you can jump up through it.",
    }
    for identifier in (TERRAIN_LAYER, TRACK_LAYER):
        if any(layer["identifier"] == identifier for layer in layers):
            continue
        layer = _layer_def(project, identifier, tileset["uid"], grid, docs[identifier])
        # Draw under the entity layer (LDtk renders the first layer on top):
        # Terrain right after `Ambition`, Track right after Terrain.
        anchor = "Ambition" if identifier == TERRAIN_LAYER else TERRAIN_LAYER
        after = next(
            (i for i, l in enumerate(layers) if l["identifier"] == anchor), len(layers) - 1
        )
        layers.insert(after + 1, layer)
    for level in project.get("levels", []):
        instances = level.setdefault("layerInstances", [])
        for identifier in (TERRAIN_LAYER, TRACK_LAYER):
            if any(i["__identifier"] == identifier for i in instances):
                continue
            layer = next(l for l in layers if l["identifier"] == identifier)
            c_wid, c_hei = level["pxWid"] // grid, level["pxHei"] // grid
            instance = {
                "__identifier": identifier, "__type": "IntGrid",
                "__cWid": c_wid, "__cHei": c_hei, "__gridSize": grid,
                "__opacity": 1, "__pxTotalOffsetX": 0, "__pxTotalOffsetY": 0,
                "__tilesetDefUid": tileset["uid"], "__tilesetRelPath": PALETTE_PNG,
                "iid": _iid(project, level, identifier), "levelId": level["uid"],
                "layerDefUid": layer["uid"], "pxOffsetX": 0, "pxOffsetY": 0,
                "visible": True, "optionalRules": [],
                "intGridCsv": [0] * (c_wid * c_hei), "autoLayerTiles": [],
                "seed": level["uid"], "overrideTilesetUid": None,
                "gridTiles": [], "entityInstances": [],
            }
            # Keep instance order equal to def order (LDtk expects it).
            order = [l["identifier"] for l in layers]
            instances.append(instance)
            instances.sort(key=lambda i: order.index(i["__identifier"]) if i["__identifier"] in order else len(order))


def _iid(project: dict, level: dict, identifier: str) -> str:
    import uuid

    return str(uuid.uuid5(uuid.NAMESPACE_URL, f"ambition:{level['iid']}:{identifier}"))


def paint(project: dict, level_identifier: str, layer: str, cells: dict[tuple[int, int], int]) -> None:
    """Replace ``layer``'s cells in the named level, and its auto-layer tiles."""
    level = next(l for l in project["levels"] if l["identifier"] == level_identifier)
    instance = next(i for i in level["layerInstances"] if i["__identifier"] == layer)
    layer_def = next(l for l in project["defs"]["layers"] if l["identifier"] == layer)
    rule_uid = {r["pattern"][0]: r["uid"] for r in layer_def["autoRuleGroups"][0]["rules"]}
    c_wid, c_hei, grid = instance["__cWid"], instance["__cHei"], instance["__gridSize"]
    csv = [0] * (c_wid * c_hei)
    tiles = []
    for (cx, cy), value in sorted(cells.items(), key=lambda kv: (kv[0][1], kv[0][0])):
        if not (0 <= cx < c_wid and 0 <= cy < c_hei):
            continue
        if value not in rule_uid:
            raise ValueError(f"{layer} cell ({cx},{cy}): {value} is not a palette value")
        coord = cy * c_wid + cx
        csv[coord] = value
        t = tile_id(layer, value)
        tiles.append({
            "px": [cx * grid, cy * grid],
            "src": [(t % len(PALETTE)) * grid, (t // len(PALETTE)) * grid],
            "f": 0, "t": t, "d": [rule_uid[value], coord], "a": 1,
        })
    instance["intGridCsv"] = csv
    instance["autoLayerTiles"] = tiles


# ── Rasterizing a height profile ────────────────────────────────────────────


def _allowed_steps(height: int) -> list[int]:
    """The quarter rises a floor cell may make starting at ``height`` (quarters,
    y down): flats only on a cell boundary, 22° pieces at a half, 45° whole."""
    off = height % Q
    steps = [1, -1]
    if off == 0:
        steps += [0, 2, -2, 4, -4]
    elif off == 2:
        steps += [2, -2]
    return steps


#: Rasterizer weights (see ``quantize``), in quarters.
REVERSAL_COST = 40.0
SLOPE_CHANGE_COST = 0.5
WINDOW = 16


def quantize(profile: Callable[[float], float], x0: int, x1: int, grid: int) -> list[int]:
    """Surface heights, in quarters (y down), at every cell boundary from column
    ``x0`` to ``x1`` inclusive: the palette-drawable surface nearest ``profile``.

    A dynamic program over (height, last step, last direction): each column
    pays its squared distance from the profile, a small price for changing
    slope, and a large one for reversing direction. The palette has no
    half-height flat, so a flat whose target falls mid-cell must choose a cell
    line — a greedy rounder zig-zags a quarter up and down across it instead,
    and the reversal price is what forbids that sawtooth.

    ⚠ The direction is the last NONZERO step's, carried across flats: priced
    step to step, up-flat-down-flat reversed for free and paved a mid-cell flat
    with little trapezoids.
    """
    s = grid / Q
    targets = [profile(cx * grid) / s for cx in range(x0, x1 + 1)]
    # State (height, last step, last nonzero direction) -> cost.
    State = tuple[int, int, int]
    frontier: dict[State, float] = {}
    base = round(targets[0])
    for h in range(base - WINDOW, base + WINDOW + 1):
        if h % Q == 0:
            frontier[(h, 0, 0)] = (h - targets[0]) ** 2
    history: list[dict[State, State]] = []
    for target in targets[1:]:
        low, high = round(target) - WINDOW, round(target) + WINDOW
        nxt: dict[State, float] = {}
        back: dict[State, State] = {}
        for (h, last, direction), cost in frontier.items():
            for step in _allowed_steps(h):
                nh = h + step
                if not low <= nh <= high:
                    continue
                total = cost + (nh - target) ** 2 + SLOPE_CHANGE_COST * abs(step - last)
                if direction * step < 0:
                    total += REVERSAL_COST
                key = (nh, step, (step > 0) - (step < 0) if step else direction)
                if total < nxt.get(key, math.inf):
                    nxt[key] = total
                    back[key] = (h, last, direction)
        if not nxt:
            raise ValueError("the profile is steeper than the palette's 45° can follow")
        frontier = nxt
        history.append(back)
    # A run ends on a cell line, as it starts: otherwise its last column, with
    # no future to pay for, dips a free quarter toward the target.
    ends = {k: v for k, v in frontier.items() if k[0] % Q == 0} or frontier
    state = min(ends, key=ends.get)
    heights = [state[0]]
    for back in reversed(history):
        state = back[state]
        heights.append(state[0])
    heights.reverse()
    return heights


def surface(heights: list[int], x0: int, grid: int) -> Callable[[float], float]:
    """``y(x)`` in pixels along the painted surface ``quantize`` returned, so a
    script can stand things on the ground that was actually painted (up to
    half a cell from the profile it was drawn from)."""
    s = grid / Q

    def at(x: float) -> float:
        i = (x / grid) - x0
        k = min(max(int(math.floor(i)), 0), len(heights) - 2)
        t = min(max(i - k, 0.0), 1.0)
        return (heights[k] + (heights[k + 1] - heights[k]) * t) * s

    return at


def ground(profile: Callable[[float], float], x0: int, x1: int, grid: int, bottom: int) -> dict:
    """Cells for ground whose top follows ``profile`` over columns ``x0..x1``
    (exclusive end), solid down to cell row ``bottom`` (exclusive)."""
    heights = quantize(profile, x0, x1, grid)
    cells: dict[tuple[int, int], int] = {}
    for i, cx in enumerate(range(x0, x1)):
        a, b = heights[i], heights[i + 1]
        top_row = min(a, b) // Q
        for cy in range(top_row, bottom):
            value = _floor_value(a, b, cy)
            if value:
                cells[(cx, cy)] = value
    return cells


def band(profile: Callable[[float], float], x0: int, x1: int, grid: int, thickness: int) -> dict:
    """Cells for a road ``thickness`` cells thick whose top follows ``profile``:
    the same top as ``ground``, and an underside that is its copy shifted down.
    Paint it on the ``Track`` layer for a road you can jump up through."""
    if thickness < 1:
        raise ValueError("a road is at least one cell thick, so each cell meets one edge")
    heights = quantize(profile, x0, x1, grid)
    depth = thickness * Q
    cells: dict[tuple[int, int], int] = {}
    for i, cx in enumerate(range(x0, x1)):
        a, b = heights[i], heights[i + 1]
        for cy in range(min(a, b) // Q, (max(a, b) + depth) // Q + 1):
            # How much of the cell lies below the top, and below the underside.
            top = _coverage_floor(a, b, cy)
            under = _coverage_floor(a + depth, b + depth, cy)
            if under == (0, 0):
                value = _floor_value(a, b, cy)
            elif top == (Q, Q):
                # Earth from the cell's top down to the underside: a ceiling piece.
                dl, dr = Q - under[0], Q - under[1]
                if dl == dr == 0:
                    continue
                value = CEILING_BY_DEPTHS.get((dl, dr))
                if value is None:
                    raise ValueError(f"no ceiling tile for depths ({dl},{dr}) at ({cx},{cy})")
            else:
                raise ValueError(f"road cell ({cx},{cy}) meets both its edges")
            if value:
                cells[(cx, cy)] = value
    return cells


def ceiling(profile: Callable[[float], float], x0: int, x1: int, grid: int, top: int) -> dict:
    """Cells for a roof whose underside follows ``profile``, solid up to cell row
    ``top`` (inclusive)."""
    heights = quantize(profile, x0, x1, grid)
    cells: dict[tuple[int, int], int] = {}
    for i, cx in enumerate(range(x0, x1)):
        a, b = heights[i], heights[i + 1]
        for cy in range(top, max(a, b) // Q + 1):
            dl = min(max(a - cy * Q, 0), Q)
            dr = min(max(b - cy * Q, 0), Q)
            if dl == dr == 0:
                continue
            if dl == dr == Q:
                cells[(cx, cy)] = FULL
                continue
            value = CEILING_BY_DEPTHS.get((dl, dr))
            if value is None:
                raise ValueError(f"no ceiling tile for depths ({dl},{dr}) at ({cx},{cy})")
            cells[(cx, cy)] = value
    return cells


def _coverage_floor(a: int, b: int, cy: int) -> tuple[int, int]:
    """Solid height (quarters) at the left/right edge of cell row ``cy`` below a
    surface at heights ``a``/``b``."""
    bottom = (cy + 1) * Q
    return min(max(bottom - a, 0), Q), min(max(bottom - b, 0), Q)


def _floor_value(a: int, b: int, cy: int) -> int | None:
    left, right = _coverage_floor(a, b, cy)
    if left == right == 0:
        return None
    if left == right == Q:
        return FULL
    value = FLOOR_BY_HEIGHTS.get((left, right))
    if value is None:
        raise ValueError(f"no floor tile for heights ({left},{right}) in row {cy}")
    return value


def cosine_profile(keys: Iterable[tuple[float, float]]) -> Callable[[float], float]:
    """``y(x)`` through key points joined by cosine-eased curves (flat runs stay
    flat), the same ground shape the Sanic authoring scripts drew before."""
    keys = list(keys)

    def at(x: float) -> float:
        if x <= keys[0][0]:
            return keys[0][1]
        for (x0, y0), (x1, y1) in zip(keys, keys[1:]):
            if x0 <= x <= x1:
                if x1 == x0:
                    return y1
                t = (x - x0) / (x1 - x0)
                return y0 + (y1 - y0) * (1 - math.cos(math.pi * t)) / 2
        return keys[-1][1]

    return at


def main(argv: list[str] | None = None) -> int:
    """``terrain paint <spec.json> --ldtk <file>``.

    The spec is ``{"level_id": str, "layers": {"Terrain": [[cx, cy, value],
    ...], "Track": [...]}}``; each named layer's cells are replaced.
    """
    import argparse
    import json

    from ambition_ldtk_tools.ldtk.io import load_project, write_project

    ap = argparse.ArgumentParser(prog="terrain paint")
    ap.add_argument("spec", type=Path)
    ap.add_argument("--ldtk", type=Path, required=True)
    args = ap.parse_args(argv)
    spec = json.loads(args.spec.read_text())
    project = load_project(args.ldtk)
    ensure_painted_layers(project, args.ldtk)
    for layer, cells in spec["layers"].items():
        if layer not in (TERRAIN_LAYER, TRACK_LAYER):
            raise SystemExit(f"terrain paint: {layer} is not a painted layer")
        paint(project, spec["level_id"], layer, {(cx, cy): v for cx, cy, v in cells})
    write_project(args.ldtk, project)
    counts = ", ".join(f"{k} {len(v)}" for k, v in spec["layers"].items())
    print(f"painted {args.ldtk.name} {spec['level_id']}: {counts} cells")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
