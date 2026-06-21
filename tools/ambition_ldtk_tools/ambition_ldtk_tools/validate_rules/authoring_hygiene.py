"""Author-facing Ambition LDtk hygiene rules.

These checks are intentionally soft warnings. They describe level-authoring
smells that are hard to notice in raw LDtk JSON but show up clearly in game or
in debug overlays.
"""

from __future__ import annotations

from collections import defaultdict
from typing import Any

from ambition_ldtk_tools.ldtk.issues import Issue


def field_value(fields: list[dict[str, Any]] | None, name: str, default: Any = None) -> Any:
    for field in fields or []:
        if field.get("__identifier") == name:
            return field.get("__value")
    return default


def entity_name(entity: dict[str, Any]) -> str:
    return f"{entity.get('__identifier')} {entity.get('iid', '<no-iid>')}"


def rect(entity: dict[str, Any]) -> tuple[float, float, float, float]:
    px = entity.get("px") or [0, 0]
    return (
        float(px[0]),
        float(px[1]),
        float(entity.get("width", 0) or 0),
        float(entity.get("height", 0) or 0),
    )


def strict_rects_intersect(
    a: tuple[float, float, float, float],
    b: tuple[float, float, float, float],
) -> bool:
    ax, ay, aw, ah = a
    bx, by, bw, bh = b
    return ax < bx + bw and ax + aw > bx and ay < by + bh and ay + ah > by


def authoring_hygiene_issues(project: dict[str, Any]) -> list[Issue]:
    """Return warning issues for room-authoring hygiene smells."""

    issues: list[Issue] = []
    issues.extend(debug_label_overlap_issues(project))
    issues.extend(spawn_overlap_issues(project))
    issues.extend(loading_zone_support_issues(project))
    return issues


def debug_label_overlap_issues(project: dict[str, Any]) -> list[Issue]:
    """Warn when debug-label rectangles overlap within a level."""

    issues: list[Issue] = []
    label_rects_by_level: dict[str, list[tuple[str, str | None, float, float, float, float]]] = defaultdict(list)
    for level in project.get("levels") or []:
        level_id = level.get("identifier", "<unknown>")
        for layer in level.get("layerInstances") or []:
            layer_id = layer.get("__identifier")
            for entity in layer.get("entityInstances") or []:
                if entity.get("__identifier") != "DebugLabel":
                    continue
                ex, ey, ew, eh = rect(entity)
                label_rects_by_level[level_id].append(
                    (entity_name(entity), layer_id, ex, ey, ew, eh)
                )
    for level_id, rects in label_rects_by_level.items():
        for i in range(len(rects)):
            for j in range(i + 1, len(rects)):
                ai, a_layer, ax, ay, aw, ah = rects[i]
                bi, _b_layer, bx, by, bw, bh = rects[j]
                if strict_rects_intersect((ax, ay, aw, ah), (bx, by, bw, bh)):
                    issues.append(
                        Issue(
                            severity="warning",
                            code="validate.debug_label_overlap",
                            message=(
                                f"DebugLabels {ai!r} and {bi!r} overlap; space them apart "
                                "or stack vertically so debug-overlay text remains readable"
                            ),
                            level=level_id,
                            layer=a_layer,
                            fix_hint="Move one label or resize its entity rectangle.",
                            data={"a": ai, "b": bi},
                        )
                    )
    return issues


def spawn_overlap_issues(project: dict[str, Any]) -> list[Issue]:
    """Warn when spawn markers overlap or nearly overlap."""

    spawn_gap_px = 4.0
    spawn_kinds = {"NpcSpawn", "EnemySpawn", "BossSpawn"}
    issues: list[Issue] = []
    spawns_by_level: dict[str, list[tuple[str, str, str | None, float, float, float, float]]] = defaultdict(list)
    for level in project.get("levels") or []:
        level_id = level.get("identifier", "<unknown>")
        for layer in level.get("layerInstances") or []:
            layer_id = layer.get("__identifier")
            for entity in layer.get("entityInstances") or []:
                ident = entity.get("__identifier")
                if ident not in spawn_kinds:
                    continue
                ex, ey, ew, eh = rect(entity)
                spawns_by_level[level_id].append(
                    (str(ident), entity_name(entity), layer_id, ex, ey, ew, eh)
                )
    for level_id, items in spawns_by_level.items():
        for i in range(len(items)):
            for j in range(i + 1, len(items)):
                ka, la, a_layer, ax, ay, aw, ah = items[i]
                kb, lb, _b_layer, bx, by, bw, bh = items[j]
                inflate = spawn_gap_px / 2.0
                if strict_rects_intersect(
                    (ax - inflate, ay - inflate, aw + 2 * inflate, ah + 2 * inflate),
                    (bx - inflate, by - inflate, bw + 2 * inflate, bh + 2 * inflate),
                ):
                    issues.append(
                        Issue(
                            severity="warning",
                            code="validate.spawn_overlap",
                            message=(
                                f"{ka} {la!r} and {kb} {lb!r} overlap or sit within "
                                f"{spawn_gap_px:g}px; wide sprites may bleed across slot boundaries"
                            ),
                            level=level_id,
                            layer=a_layer,
                            fix_hint="Move the spawn markers farther apart or narrow their rectangles.",
                            data={
                                "a": {"kind": ka, "name": la, "rect": [ax, ay, aw, ah]},
                                "b": {"kind": kb, "name": lb, "rect": [bx, by, bw, bh]},
                                "gap_px": spawn_gap_px,
                            },
                        )
                    )
    return issues


def loading_zone_support_issues(project: dict[str, Any]) -> list[Issue]:
    """Warn about unsupported Door LoadingZones and unblocked room edges."""

    stand_gap = 16.0
    issues: list[Issue] = []
    for level in project.get("levels") or []:
        level_id = level.get("identifier", "<unknown>")
        width = int(level.get("pxWid", 0) or 0)
        height = int(level.get("pxHei", 0) or 0)
        solids: list[tuple[float, float, float, float]] = []
        one_ways: list[tuple[float, float, float, float]] = []
        doors: list[tuple[str, str | None, str, tuple[float, float, float, float]]] = []
        edge_exits: set[str] = set()
        intgrid_layer: dict[str, Any] | None = None

        for layer in level.get("layerInstances") or []:
            layer_id = layer.get("__identifier")
            if layer_id == "Collision":
                intgrid_layer = layer
            for entity in layer.get("entityInstances") or []:
                ident = entity.get("__identifier")
                if ident == "Solid":
                    solids.append(rect(entity))
                elif ident == "OneWayPlatform":
                    one_ways.append(rect(entity))
                elif ident == "LoadingZone":
                    fields = entity.get("fieldInstances") or []
                    activation = str(field_value(fields, "activation", "Door"))
                    er = rect(entity)
                    doors.append((entity_name(entity), layer_id, activation, er))
                    if activation == "EdgeExit":
                        ex, ey, ew, eh = er
                        if ex <= 1:
                            edge_exits.add("left")
                        if ex + ew >= width - 1:
                            edge_exits.add("right")
                        if ey <= 1:
                            edge_exits.add("top")
                        if ey + eh >= height - 1:
                            edge_exits.add("bottom")

        ig_grid = int(intgrid_layer.get("__gridSize", 16)) if intgrid_layer else 16
        ig_c_wid = int(intgrid_layer.get("__cWid", 0)) if intgrid_layer else 0
        ig_c_hei = int(intgrid_layer.get("__cHei", 0)) if intgrid_layer else 0
        ig_csv = intgrid_layer.get("intGridCsv", []) if intgrid_layer else []

        def intgrid_rect_intersects_walkable(
            rect_xywh: tuple[float, float, float, float],
        ) -> bool:
            if not (intgrid_layer and ig_c_wid and ig_c_hei and ig_csv):
                return False
            rx, ry, rw, rh = rect_xywh
            cx0 = max(0, int(rx) // ig_grid)
            cy0 = max(0, int(ry) // ig_grid)
            cx1 = min(ig_c_wid - 1, int(rx + rw - 1) // ig_grid)
            cy1 = min(ig_c_hei - 1, int(ry + rh - 1) // ig_grid)
            for cy in range(cy0, cy1 + 1):
                for cx in range(cx0, cx1 + 1):
                    value = ig_csv[cy * ig_c_wid + cx]
                    if value in (1, 2):  # Solid or OneWayPlatform
                        return True
            return False

        for name, layer_id, activation, (dx, dy, dw, dh) in doors:
            if activation != "Door":
                continue
            probe = (dx, dy + dh, dw, stand_gap)
            supports = (
                any(strict_rects_intersect(probe, solid) for solid in solids)
                or any(strict_rects_intersect(probe, one_way) for one_way in one_ways)
                or intgrid_rect_intersects_walkable(probe)
            )
            if not supports:
                issues.append(
                    Issue(
                        severity="warning",
                        code="validate.loading_zone_midair",
                        message=(
                            f"LoadingZone {name!r} is a Door with no walkable surface within "
                            f"{int(stand_gap)}px below; it looks like a teleport hanging in mid-air"
                        ),
                        level=level_id,
                        layer=layer_id,
                        fix_hint="Add Solid/OneWayPlatform under it or switch activation to EdgeExit.",
                        data={"door": name, "probe": list(probe)},
                    )
                )

        if width <= 0 or height <= 0:
            continue

        grid_size = int(intgrid_layer.get("__gridSize", 16)) if intgrid_layer else 16
        c_wid = int(intgrid_layer.get("__cWid", 0)) if intgrid_layer else 0
        c_hei = int(intgrid_layer.get("__cHei", 0)) if intgrid_layer else 0
        csv = intgrid_layer.get("intGridCsv", []) if intgrid_layer else []
        _ = grid_size  # kept for parity/readability with the historical check

        def intgrid_blocks_side(side: str) -> bool:
            if not (intgrid_layer and c_wid and c_hei and csv):
                return False
            if side == "left":
                return any(csv[y * c_wid] == 1 for y in range(c_hei))
            if side == "right":
                return any(csv[y * c_wid + (c_wid - 1)] == 1 for y in range(c_hei))
            if side == "top":
                return any(csv[x] == 1 for x in range(c_wid))
            if side == "bottom":
                return any(csv[(c_hei - 1) * c_wid + x] == 1 for x in range(c_wid))
            return False

        sides = (
            ("left", (0.0, 0.0, 1.0, float(height))),
            ("right", (float(max(0, width - 1)), 0.0, 1.0, float(height))),
            ("top", (0.0, 0.0, float(width), 1.0)),
            ("bottom", (0.0, float(max(0, height - 1)), float(width), 1.0)),
        )
        for side_name, probe in sides:
            if side_name in edge_exits:
                continue
            blocks_side = any(strict_rects_intersect(probe, solid) for solid in solids) or intgrid_blocks_side(side_name)
            if not blocks_side:
                issues.append(
                    Issue(
                        severity="warning",
                        code="validate.missing_level_wall",
                        message=(
                            f"level {level_id!r} has no Solid blocking the {side_name} edge "
                            "and no EdgeExit on that side; the controlled body can leave the world"
                        ),
                        level=level_id,
                        layer="Collision",
                        fix_hint="Add a Solid/Collision wall on that side or author an EdgeExit LoadingZone.",
                        data={"side": side_name, "probe": list(probe)},
                    )
                )
    return issues
