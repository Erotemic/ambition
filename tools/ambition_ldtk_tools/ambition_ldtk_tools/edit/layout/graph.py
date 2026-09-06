"""Graph construction and placement primitives for LDtk world layout."""

from __future__ import annotations

import math
from collections import defaultdict, deque
from typing import Iterable

from ambition_ldtk_tools.edit.layout.model import GroupInfo, LayoutEdge, LevelInfo, Point, Rect, ZoneRef

def field_map(obj: dict) -> dict[str, object]:
    return {f.get("__identifier"): f.get("__value") for f in obj.get("fieldInstances", [])}


def active_area_for_level(level: dict) -> str:
    fields = field_map(level)
    active = fields.get("activeArea")
    if isinstance(active, str) and active:
        return active
    return str(level.get("identifier"))


def _truthy_field_value(value: object) -> bool:
    if isinstance(value, bool):
        return value
    if isinstance(value, (int, float)):
        return value != 0
    if isinstance(value, str):
        return value.strip().lower() in {"1", "true", "yes", "y", "on", "locked", "lock"}
    return False


def locked_groups_from_level_fields(
    groups: dict[str, GroupInfo],
    *,
    lock_field: str = "layoutLocked",
) -> set[str]:
    """Return activeArea group ids locked by an optional level field.

    LDtk level fields are project-defined, so this helper is deliberately
    duck-typed. If a project has a boolean/string field named `layoutLocked`
    (or the CLI-selected --lock-field), any truthy value locks the whole
    activeArea group. If the field is absent, nothing happens.
    """

    locked: set[str] = set()
    for group_id, group in groups.items():
        for level in group.levels:
            if _truthy_field_value(field_map(level.level).get(lock_field)):
                locked.add(group_id)
                break
    return locked


def resolve_group_ids(
    names: Iterable[str],
    groups: dict[str, GroupInfo],
    levels_by_id: dict[str, LevelInfo],
    *,
    label: str,
) -> set[str]:
    resolved: set[str] = set()
    for name in names:
        if not name:
            continue
        if name in groups:
            resolved.add(name)
        elif name in levels_by_id:
            resolved.add(levels_by_id[name].active_area)
        else:
            raise SystemExit(f"{label} '{name}' is not a level identifier or activeArea")
    return resolved


def entity_rect(entity: dict) -> Rect:
    px = entity.get("px") or [0, 0]
    return Rect(int(px[0]), int(px[1]), int(entity.get("width") or 0), int(entity.get("height") or 0))


def iter_entities(level: dict) -> Iterable[dict]:
    for layer in level.get("layerInstances") or []:
        if layer.get("__type") != "Entities":
            continue
        for entity in layer.get("entityInstances") or []:
            yield entity


def build_groups(project: dict) -> tuple[dict[str, GroupInfo], dict[str, LevelInfo]]:
    levels_by_id: dict[str, LevelInfo] = {}
    groups: dict[str, GroupInfo] = {}
    for level in project.get("levels") or []:
        ident = str(level.get("identifier"))
        active = active_area_for_level(level)
        rect = Rect(
            int(level.get("worldX") or 0),
            int(level.get("worldY") or 0),
            int(level.get("pxWid") or 0),
            int(level.get("pxHei") or 0),
        )
        info = LevelInfo(identifier=ident, active_area=active, level=level, rect=rect)
        levels_by_id[ident] = info
        groups.setdefault(active, GroupInfo(id=active)).levels.append(info)

    for group in groups.values():
        min_x = min(level.rect.x for level in group.levels)
        min_y = min(level.rect.y for level in group.levels)
        max_x = max(level.rect.x2 for level in group.levels)
        max_y = max(level.rect.y2 for level in group.levels)
        group.anchor = Point(min_x, min_y)
        group.rect = Rect(0, 0, max_x - min_x, max_y - min_y)
    return groups, levels_by_id


def group_for_room(raw_room: str, groups: dict[str, GroupInfo], levels_by_id: dict[str, LevelInfo]) -> str | None:
    if raw_room in groups:
        return raw_room
    if raw_room in levels_by_id:
        return levels_by_id[raw_room].active_area
    return None


def find_zone(
    group: GroupInfo,
    zone_id: str,
    *,
    group_anchor: Point | None = None,
) -> ZoneRef | None:
    anchor = group.anchor if group_anchor is None else group_anchor
    for level in group.levels:
        level_rel = Point(level.rect.x - anchor.x, level.rect.y - anchor.y)
        level_rect_rel = Rect(level_rel.x, level_rel.y, level.rect.w, level.rect.h)
        for entity in iter_entities(level.level):
            if entity.get("__identifier") != "LoadingZone":
                continue
            fields = field_map(entity)
            if fields.get("id") != zone_id:
                continue
            rect = entity_rect(entity)
            center_rel_level = rect.center
            center_rel_group = Point(level_rel.x + center_rel_level.x, level_rel.y + center_rel_level.y)
            return ZoneRef(
                group_id=group.id,
                level_id=level.identifier,
                zone_id=zone_id,
                center_rel_group=center_rel_group,
                center_rel_level=center_rel_level,
                level_rect_rel_group=level_rect_rel,
                activation=str(fields.get("activation") or ""),
            )
    return None


def source_zone_ref(group: GroupInfo, level: LevelInfo, entity: dict) -> ZoneRef:
    rect = entity_rect(entity)
    fields = field_map(entity)
    level_rel = Point(level.rect.x - group.anchor.x, level.rect.y - group.anchor.y)
    level_rect_rel = Rect(level_rel.x, level_rel.y, level.rect.w, level.rect.h)
    return ZoneRef(
        group_id=group.id,
        level_id=level.identifier,
        zone_id=str(fields.get("id") or entity.get("iid") or ""),
        center_rel_group=Point(level_rel.x + rect.center.x, level_rel.y + rect.center.y),
        center_rel_level=rect.center,
        level_rect_rel_group=level_rect_rel,
        activation=str(fields.get("activation") or ""),
    )


def infer_direction(zone: ZoneRef) -> str:
    # Edge exits should follow the actual edge touched. Doors use the strongest
    # vector from level center to door center. This keeps basement doors below a
    # basement layer and upper doors above a hub without authored floor labels.
    margin = 48
    lvl = zone.level_rect_rel_group
    x = zone.center_rel_group.x
    y = zone.center_rel_group.y
    if x <= lvl.x + margin:
        return "left"
    if x >= lvl.x2 - margin:
        return "right"
    if y <= lvl.y + margin:
        return "up"
    if y >= lvl.y2 - margin:
        return "down"

    # Interior doors usually communicate their desired editor direction by
    # their vertical band: doors near the floor should put their target below,
    # doors near the ceiling should put their target above. This is especially
    # important for hub/basement rows, where many doors spread along X but are
    # all conceptually below the current layer.
    local_y = zone.center_rel_level.y / max(lvl.h, 1)
    if local_y >= 0.70:
        return "down"
    if local_y <= 0.30:
        return "up"

    dx = zone.center_rel_level.x - lvl.w // 2
    dy = zone.center_rel_level.y - lvl.h // 2
    if abs(dx) > abs(dy):
        return "right" if dx >= 0 else "left"
    return "down" if dy >= 0 else "up"


def build_edges(project: dict, groups: dict[str, GroupInfo], levels_by_id: dict[str, LevelInfo]) -> list[LayoutEdge]:
    edges: list[LayoutEdge] = []
    group_by_level = {level.identifier: groups[level.active_area] for level in levels_by_id.values()}
    for level in levels_by_id.values():
        source_group = group_by_level[level.identifier]
        for entity in iter_entities(level.level):
            if entity.get("__identifier") != "LoadingZone":
                continue
            fields = field_map(entity)
            raw_room = fields.get("target_room")
            raw_zone = fields.get("target_zone")
            if not isinstance(raw_room, str) or not raw_room:
                continue
            if not isinstance(raw_zone, str) or not raw_zone:
                continue
            target_group_id = group_for_room(raw_room, groups, levels_by_id)
            if target_group_id is None:
                # Keep a lightweight unresolved edge for reporting.
                source = source_zone_ref(source_group, level, entity)
                edges.append(
                    LayoutEdge(
                        source=source,
                        target=None,
                        target_group_id=raw_room,
                        target_room_raw=raw_room,
                        target_zone_raw=raw_zone,
                        direction=infer_direction(source),
                        weight=0.0,
                    )
                )
                continue
            source = source_zone_ref(source_group, level, entity)
            target = find_zone(groups[target_group_id], raw_zone)
            edges.append(
                LayoutEdge(
                    source=source,
                    target=target,
                    target_group_id=target_group_id,
                    target_room_raw=raw_room,
                    target_zone_raw=raw_zone,
                    direction=infer_direction(source),
                    weight=1.0 if target is not None else 0.65,
                )
            )
    return edges


def snap(value: int, grid: int) -> int:
    if grid <= 1:
        return value
    return int(round(value / grid) * grid)


def snap_point(p: Point, grid: int) -> Point:
    return Point(snap(p.x, grid), snap(p.y, grid))


def desired_anchor(
    edge: LayoutEdge,
    groups: dict[str, GroupInfo],
    placements: dict[str, Point],
    *,
    gap: int,
) -> Point:
    src_anchor = placements[edge.source.group_id]
    src_group = groups[edge.source.group_id]
    dst_group = groups[edge.target_group_id]
    src_zone_world = src_anchor + edge.source.center_rel_group
    target_rel = edge.target.center_rel_group if edge.target is not None else Point(dst_group.rect.w // 2, dst_group.rect.h // 2)

    if edge.direction == "right":
        return Point(src_anchor.x + src_group.rect.x2 + gap - dst_group.rect.x, src_zone_world.y - target_rel.y)
    if edge.direction == "left":
        return Point(src_anchor.x + src_group.rect.x - gap - dst_group.rect.w - dst_group.rect.x, src_zone_world.y - target_rel.y)
    if edge.direction == "up":
        return Point(src_zone_world.x - target_rel.x, src_anchor.y + src_group.rect.y - gap - dst_group.rect.h - dst_group.rect.y)
    return Point(src_zone_world.x - target_rel.x, src_anchor.y + src_group.rect.y2 + gap - dst_group.rect.y)


def overlaps_any(rect: Rect, placed_rects: list[tuple[str, Rect]], *, gap: int) -> bool:
    return any(rect.intersects(other, gap=gap) for _name, other in placed_rects)


def place_without_overlap(
    group: GroupInfo,
    desired: Point,
    placed_rects: list[tuple[str, Rect]],
    *,
    grid: int,
    gap: int,
    direction: str | None = None,
) -> Point:
    desired = snap_point(desired, grid)
    if not overlaps_any(group.rect.translated(desired), placed_rects, gap=gap):
        return desired

    # Deterministic expanding square search. Cost favors staying close to the
    # desired door alignment but will step aside to preserve editor readability.
    best: tuple[float, Point] | None = None
    step = max(grid, 64)
    max_radius = 32
    for radius in range(1, max_radius + 1):
        candidates: list[Point] = []
        for dx in range(-radius, radius + 1):
            for dy in (-radius, radius):
                candidates.append(Point(desired.x + dx * step, desired.y + dy * step))
        for dy in range(-radius + 1, radius):
            for dx in (-radius, radius):
                candidates.append(Point(desired.x + dx * step, desired.y + dy * step))
        for cand in candidates:
            cand = snap_point(cand, grid)
            if direction == "down" and cand.y < desired.y:
                continue
            if direction == "up" and cand.y > desired.y:
                continue
            if direction == "right" and cand.x < desired.x:
                continue
            if direction == "left" and cand.x > desired.x:
                continue
            rect = group.rect.translated(cand)
            if overlaps_any(rect, placed_rects, gap=gap):
                continue
            dist = math.hypot(cand.x - desired.x, cand.y - desired.y)
            # Prefer the requested row/column. Door rows should spread sideways
            # before they jump to another vertical layer; side doors should stack
            # vertically before they drift farther horizontally.
            if direction in {"down", "up"}:
                score = abs(cand.y - desired.y) * 4 + abs(cand.x - desired.x)
            elif direction in {"left", "right"}:
                score = abs(cand.x - desired.x) * 4 + abs(cand.y - desired.y)
            else:
                score = dist
            if best is None or score < best[0] or (score == best[0] and (cand.y, cand.x) < (best[1].y, best[1].x)):
                best = (score, cand)
        if best is not None:
            return best[1]
    return desired


def choose_start_group(start: str, groups: dict[str, GroupInfo], levels_by_id: dict[str, LevelInfo]) -> str:
    if start in groups:
        return start
    if start in levels_by_id:
        return levels_by_id[start].active_area
    raise SystemExit(f"start room/area '{start}' is not a level identifier or activeArea")


def level_top_left_relative_to_group(level: LevelInfo, group: GroupInfo) -> Point:
    return Point(level.rect.x - group.anchor.x, level.rect.y - group.anchor.y)


def build_adjacency(edges: list[LayoutEdge], groups: dict[str, GroupInfo]) -> dict[str, list[LayoutEdge]]:
    adjacency: dict[str, list[LayoutEdge]] = defaultdict(list)
    for edge in edges:
        if edge.target_group_id not in groups:
            continue
        if edge.source.group_id == edge.target_group_id:
            continue
        adjacency[edge.source.group_id].append(edge)
        # Reverse reachability is intentional. A pair of return LoadingZones will
        # usually provide a proper forward edge, but one-way authored links still
        # need graph reachability so the layout can keep the target nearby.
        adjacency[edge.target_group_id].append(edge)
    return adjacency


def seed_placements(
    groups: dict[str, GroupInfo],
    *,
    start_group_id: str,
    start_anchor: Point,
    locked_groups: set[str],
) -> tuple[dict[str, Point], list[tuple[str, Rect]], deque[str]]:
    placements: dict[str, Point] = {start_group_id: start_anchor}
    for group_id in sorted(locked_groups):
        placements.setdefault(group_id, groups[group_id].anchor)
    placed_rects: list[tuple[str, Rect]] = []
    for group_id in sorted(placements):
        placed_rects.append((group_id, groups[group_id].rect.translated(placements[group_id])))
    q: deque[str] = deque([start_group_id, *sorted(g for g in locked_groups if g != start_group_id)])
    return placements, placed_rects, q


def place_disconnected_components(
    groups: dict[str, GroupInfo],
    placements: dict[str, Point],
    placed_rects: list[tuple[str, Rect]],
    *,
    origin: Point,
    grid: int,
    gap: int,
    packing_padding: int,
) -> None:
    if len(placements) >= len(groups):
        return
    max_y = max((rect.y2 for _name, rect in placed_rects), default=origin.y)
    shelf_x = origin.x
    shelf_y = snap(max_y + gap * 2, grid)
    shelf_h = 0
    max_width = max(4096, int(math.sqrt(len(groups)) + 1) * 2048)
    for group_id, group in sorted(groups.items()):
        if group_id in placements:
            continue
        if shelf_x != origin.x and shelf_x + group.rect.w > origin.x + max_width:
            shelf_x = origin.x
            shelf_y = snap(shelf_y + shelf_h + gap, grid)
            shelf_h = 0
        desired = Point(shelf_x, shelf_y)
        placed = place_without_overlap(group, desired, placed_rects, grid=grid, gap=packing_padding)
        placements[group_id] = placed
        placed_rects.append((group_id, group.rect.translated(placed)))
        shelf_x = snap(placed.x + group.rect.w + gap, grid)
        shelf_h = max(shelf_h, group.rect.h)
