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


# : Fields whose value is an `EntityRef` naming another entity this one is
# : authored ON TOP OF. A pair joined by one of these is a RELATIONSHIP, not a
# : duplicate, and the author put them at the same pixel on purpose.
RIDING_REF_FIELDS = ("mounted_on",)


def referenced_iids(entity: dict[str, Any]) -> set[str]:
    """Every entity iid this one names through a riding reference."""

    iids: set[str] = set()
    for name in RIDING_REF_FIELDS:
        value = field_value(entity.get("fieldInstances"), name)
        if isinstance(value, dict):
            ref = value.get("entityIid")
            if ref:
                iids.add(str(ref))
    return iids


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
    issues.extend(placement_id_collision_issues(project))
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


# : `LoadingZone` ids are ROOM-SCOPED by design and reused on purpose —
# : `return_door` names the way back in seven different rooms, and a zone's
# : `target_zone` is resolved WITHIN its `target_room`, so the room disambiguates.
# : Every other kind's id can become a `SimId::placement(..)`, which is GLOBAL.
ROOM_SCOPED_ID_KINDS = {"LoadingZone"}


def placement_id_collision_issues(project: dict[str, Any]) -> list[Issue]:
    """Warn when one authored `id` names things in two different rooms.

    ⛔⛔ **`SimId::placement(id)` IS A GLOBAL NAMESPACE AND NOTHING CHECKS IT
    ACROSS ROOMS** — recorded as a carried risk on ledger D125: *"two rooms
    authoring one id would suppress both"*. An authored rule reaches that
    namespace through `placement:<id>` (`authored_logic/prepared.rs`), so the
    day somebody writes `placement:return_door` it names seven zones at once.

    ⭐ **green today, which is the point of adding it now.** Measured across the
    shipped worlds: twelve entity kinds carry an `id`, and the only cross-room
    reuse is `LoadingZone` — `return_door` (7 rooms), `east_exit` (3),
    `west_exit` (2) — every one of them deliberate. Nothing else collides, so
    this costs nothing until it earns its keep.

    ⚠ **per FILE, not per project.** The ledger is not world-scoped, so a
    cross-WORLD collision is possible in principle; measured 0 today, and
    checking it here would need every world loaded at once, which this validator
    does not do. Recorded rather than silently half-covered.
    """

    issues: list[Issue] = []
    rooms_by_id: dict[tuple[str, str], list[str]] = defaultdict(list)
    for level in project.get("levels") or []:
        level_id = level.get("identifier", "<unknown>")
        for layer in level.get("layerInstances") or []:
            for entity in layer.get("entityInstances") or []:
                kind = entity.get("__identifier")
                if kind in ROOM_SCOPED_ID_KINDS:
                    continue
                value = field_value(entity.get("fieldInstances"), "id")
                if not isinstance(value, str) or not value:
                    continue
                rooms = rooms_by_id[(str(kind), value)]
                if level_id not in rooms:
                    rooms.append(level_id)

    for (kind, value), rooms in sorted(rooms_by_id.items()):
        if len(rooms) < 2:
            continue
        issues.append(
            Issue(
                severity="warning",
                code="validate.placement_id_collision",
                message=(
                    f"{kind} id {value!r} is authored in {len(rooms)} rooms "
                    f"({', '.join(sorted(rooms))}); `SimId::placement` is a GLOBAL "
                    "namespace, so an authored rule naming `placement:"
                    f"{value}` would mean all of them"
                ),
                level=rooms[0],
                layer=None,
                fix_hint="Give each one a distinct id, or scope it by room the way a LoadingZone is.",
                data={"kind": kind, "id": value, "rooms": sorted(rooms)},
            )
        )
    return issues


def spawn_overlap_issues(project: dict[str, Any]) -> list[Issue]:
    """Warn when spawn markers overlap or nearly overlap."""

    spawn_gap_px = 4.0
    spawn_kinds = {"NpcSpawn", "EnemySpawn", "BossSpawn"}
    issues: list[Issue] = []
    spawns_by_level: dict[str, list[tuple[str, str, str | None, float, float, float, float, str, set[str]]]] = defaultdict(list)
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
                    (
                        str(ident),
                        entity_name(entity),
                        layer_id,
                        ex,
                        ey,
                        ew,
                        eh,
                        str(entity.get("iid") or ""),
                        referenced_iids(entity),
                    )
                )
    for level_id, items in spawns_by_level.items():
        for i in range(len(items)):
            for j in range(i + 1, len(items)):
                ka, la, a_layer, ax, ay, aw, ah, a_iid, a_refs = items[i]
                kb, lb, _b_layer, bx, by, bw, bh, b_iid, b_refs = items[j]
                # position-identical is what a mount IS, so position alone can
                # never tell the two apart. The FIELDS can: one names the other.
                if (b_iid and b_iid in a_refs) or (a_iid and a_iid in b_refs):
                    continue
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
                if any(csv[(c_hei - 1) * c_wid + x] == 1 for x in range(c_wid)):
                    return True
                # A FLOOR STOPS A FALL WHEREVER IT IS, not only on the outermost row.
                # `portal_lab` is walled on three sides and reported open at the bottom —
                # because its full-width floor sits FIVE rows above the level boundary, with
                # empty margin below it that nothing can reach.
                #
                # only the BOTTOM gets this. The same idea on left/right
                # (a full-height column of solid) is the wrong test and much
                # NOISIER — measured: it opens 46 sides instead of 6, because a
                # corridor's side wall legitimately has a doorway gap in it.
                #  a floor is continuous by nature; a wall is not.
                return any(
                    all(csv[row * c_wid + x] == 1 for x in range(c_wid))
                    for row in range(c_hei)
                )
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
