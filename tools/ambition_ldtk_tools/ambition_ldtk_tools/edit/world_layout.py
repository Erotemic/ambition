#!/usr/bin/env python3
"""Auto-layout LDtk levels from LoadingZone graph structure.

This is an editor-formatting tool for Free-layout LDtk worlds. It does not
change room contents. It only moves levels by updating `worldX` / `worldY` and
synchronizes cached entity `__worldX` / `__worldY` values.

Unlike `world repack`, this command treats levels with the same `activeArea`
field as a rigid group. A central group is anchored at the requested origin;
neighbor groups are placed near the LoadingZone door/edge that connects them,
with simple rectangle-overlap avoidance. The goal is not a mathematically exact
minimum-crossing graph drawing. The goal is stable, deterministic editor layout
that keeps rooms near the door that reaches them.
"""

from __future__ import annotations

import argparse
import json
import math
import shutil
import subprocess
import sys
from collections import defaultdict, deque
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable

from ambition_ldtk_tools.editor_format import dump_editor_style

REPO_ROOT = Path(__file__).resolve().parents[4]


@dataclass(frozen=True)
class Point:
    x: int
    y: int

    def __add__(self, other: "Point") -> "Point":
        return Point(self.x + other.x, self.y + other.y)

    def __sub__(self, other: "Point") -> "Point":
        return Point(self.x - other.x, self.y - other.y)


@dataclass(frozen=True)
class Rect:
    x: int
    y: int
    w: int
    h: int

    @property
    def x2(self) -> int:
        return self.x + self.w

    @property
    def y2(self) -> int:
        return self.y + self.h

    @property
    def center(self) -> Point:
        return Point(self.x + self.w // 2, self.y + self.h // 2)

    def translated(self, p: Point) -> "Rect":
        return Rect(self.x + p.x, self.y + p.y, self.w, self.h)

    def intersects(self, other: "Rect", *, gap: int = 0) -> bool:
        return (
            self.x < other.x2 + gap
            and self.x2 + gap > other.x
            and self.y < other.y2 + gap
            and self.y2 + gap > other.y
        )


@dataclass
class LevelInfo:
    identifier: str
    active_area: str
    level: dict
    rect: Rect


@dataclass
class GroupInfo:
    id: str
    levels: list[LevelInfo] = field(default_factory=list)
    anchor: Point = Point(0, 0)
    rect: Rect = Rect(0, 0, 0, 0)


@dataclass(frozen=True)
class ZoneRef:
    group_id: str
    level_id: str
    zone_id: str
    center_rel_group: Point
    center_rel_level: Point
    level_rect_rel_group: Rect
    activation: str


@dataclass(frozen=True)
class LayoutEdge:
    source: ZoneRef
    target: ZoneRef | None
    target_group_id: str
    target_room_raw: str
    target_zone_raw: str
    direction: str
    weight: float = 1.0


@dataclass
class LayoutResult:
    placements: dict[str, Point]
    groups: dict[str, GroupInfo]
    edges: list[LayoutEdge]
    unresolved_edges: list[LayoutEdge]
    moved_levels: int
    updated_entities: int
    report: str


def field_map(obj: dict) -> dict[str, object]:
    return {f.get("__identifier"): f.get("__value") for f in obj.get("fieldInstances", [])}


def active_area_for_level(level: dict) -> str:
    fields = field_map(level)
    active = fields.get("activeArea")
    if isinstance(active, str) and active:
        return active
    return str(level.get("identifier"))


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


def auto_layout(
    project: dict,
    *,
    start: str,
    origin: Point = Point(0, 0),
    grid: int | None = None,
    gap: int = 256,
) -> LayoutResult:
    groups, levels_by_id = build_groups(project)
    if not groups:
        raise SystemExit("project has no levels")
    if grid is None:
        grid = int(project.get("worldGridWidth") or 256)
    start_group_id = choose_start_group(start, groups, levels_by_id)
    start_group = groups[start_group_id]

    # Anchor so the requested start level/active area lands at origin. If start
    # names a level inside a multi-level group, preserve the intra-group offset
    # and put that level's top-left at the origin.
    if start in levels_by_id:
        start_level_rel = level_top_left_relative_to_group(levels_by_id[start], start_group)
        start_anchor = Point(origin.x - start_level_rel.x, origin.y - start_level_rel.y)
    else:
        start_anchor = origin
    start_anchor = snap_point(start_anchor, grid)

    edges = build_edges(project, groups, levels_by_id)
    unresolved_edges = [e for e in edges if e.target is None or e.target_group_id not in groups]
    adjacency: dict[str, list[LayoutEdge]] = defaultdict(list)
    for edge in edges:
        if edge.target_group_id not in groups:
            continue
        if edge.source.group_id == edge.target_group_id:
            continue
        adjacency[edge.source.group_id].append(edge)
        # Add a weaker reverse edge for reachability. The desired position still
        # uses the source zone of the reverse logical edge if one exists; if not,
        # BFS can still reach the component through the original edge direction.
        adjacency[edge.target_group_id].append(edge)

    placements: dict[str, Point] = {start_group_id: start_anchor}
    placed_rects: list[tuple[str, Rect]] = [(start_group_id, start_group.rect.translated(start_anchor))]
    q: deque[str] = deque([start_group_id])

    while q:
        current = q.popleft()
        outgoing = sorted(
            adjacency.get(current, []),
            key=lambda e: (
                e.direction,
                e.source.center_rel_group.y,
                e.source.center_rel_group.x,
                e.target_group_id,
            ),
        )
        for edge in outgoing:
            # If this edge is stored from a target back to a source, and the
            # current group is not the edge's source, use it only for reachability;
            # wait for/seek a forward edge if one exists. For simple one-way door
            # graphs there usually is a corresponding return LoadingZone, but this
            # fallback still keeps disconnected-looking authored links close.
            if edge.source.group_id != current:
                neighbor = edge.source.group_id
                if neighbor in placements:
                    continue
                reverse = next(
                    (
                        e
                        for e in adjacency.get(current, [])
                        if e.source.group_id == current and e.target_group_id == neighbor
                    ),
                    None,
                )
                if reverse is None:
                    src_anchor = placements[current]
                    neighbor_group = groups[neighbor]
                    desired = Point(src_anchor.x + groups[current].rect.x2 + gap, src_anchor.y)
                    placed = place_without_overlap(
                        neighbor_group,
                        desired,
                        placed_rects,
                        grid=grid,
                        gap=gap // 4,
                        direction="right",
                    )
                    placements[neighbor] = placed
                    placed_rects.append((neighbor, neighbor_group.rect.translated(placed)))
                    q.append(neighbor)
                continue

            neighbor = edge.target_group_id
            if neighbor in placements or neighbor not in groups:
                continue
            group = groups[neighbor]
            desired = desired_anchor(edge, groups, placements, gap=gap)
            placed = place_without_overlap(
                group,
                desired,
                placed_rects,
                grid=grid,
                gap=gap // 4,
                direction=edge.direction,
            )
            placements[neighbor] = placed
            placed_rects.append((neighbor, group.rect.translated(placed)))
            q.append(neighbor)

    # Disconnected components: pack in shelves below the main component. This is
    # rare for sandbox work but avoids leaving old sprawl behind.
    if len(placements) < len(groups):
        max_y = max(rect.y2 for _name, rect in placed_rects)
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
            placed = place_without_overlap(group, desired, placed_rects, grid=grid, gap=gap // 4)
            placements[group_id] = placed
            placed_rects.append((group_id, group.rect.translated(placed)))
            shelf_x = snap(placed.x + group.rect.w + gap, grid)
            shelf_h = max(shelf_h, group.rect.h)

    moved_levels = 0
    updated_entities = 0
    for group_id, group in groups.items():
        new_anchor = placements[group_id]
        delta = new_anchor - group.anchor
        for level in group.levels:
            old_x = int(level.level.get("worldX") or 0)
            old_y = int(level.level.get("worldY") or 0)
            new_x = old_x + delta.x
            new_y = old_y + delta.y
            if old_x != new_x or old_y != new_y:
                moved_levels += 1
            level.level["worldX"] = new_x
            level.level["worldY"] = new_y
            updated_entities += update_entity_world_coords(level.level)

    report = format_report(groups, placements, edges, unresolved_edges, start_group_id, grid=grid, gap=gap)
    return LayoutResult(
        placements=placements,
        groups=groups,
        edges=edges,
        unresolved_edges=unresolved_edges,
        moved_levels=moved_levels,
        updated_entities=updated_entities,
        report=report,
    )


def update_entity_world_coords(level: dict) -> int:
    world_x = int(level.get("worldX") or 0)
    world_y = int(level.get("worldY") or 0)
    count = 0
    for layer in level.get("layerInstances") or []:
        for ent in layer.get("entityInstances") or []:
            px = ent.get("px") or [0, 0]
            ent["__worldX"] = world_x + int(px[0])
            ent["__worldY"] = world_y + int(px[1])
            count += 1
    return count


def format_report(
    groups: dict[str, GroupInfo],
    placements: dict[str, Point],
    edges: list[LayoutEdge],
    unresolved_edges: list[LayoutEdge],
    start_group_id: str,
    *,
    grid: int,
    gap: int,
) -> str:
    lines: list[str] = []
    lines.append("LDtk world auto-layout report")
    lines.append(f"start group: {start_group_id}")
    lines.append(f"grid={grid} gap={gap}")
    lines.append("")
    lines.append(f"Groups ({len(groups)}):")
    for group_id in sorted(groups):
        group = groups[group_id]
        p = placements[group_id]
        level_ids = ", ".join(level.identifier for level in group.levels)
        lines.append(
            f"  {group_id:28s} at ({p.x:>6}, {p.y:>6}) "
            f"size={group.rect.w}x{group.rect.h} levels=[{level_ids}]"
        )
    lines.append("")
    resolved = [e for e in edges if e.target is not None and e.target_group_id in groups]
    lines.append(f"Resolved LoadingZone links: {len(resolved)}")
    for edge in sorted(
        resolved,
        key=lambda e: (e.source.group_id, e.source.zone_id, e.target_group_id, e.target_zone_raw),
    )[:80]:
        lines.append(
            f"  {edge.source.group_id}:{edge.source.zone_id} -> "
            f"{edge.target_group_id}:{edge.target_zone_raw} dir={edge.direction}"
        )
    if len(resolved) > 80:
        lines.append(f"  ... {len(resolved) - 80} more")
    lines.append("")
    lines.append(f"Unresolved or partial links: {len(unresolved_edges)}")
    for edge in unresolved_edges[:80]:
        target_state = "missing target zone" if edge.target_group_id in groups else "missing target room/area"
        lines.append(
            f"  {edge.source.group_id}:{edge.source.zone_id} -> "
            f"{edge.target_room_raw}:{edge.target_zone_raw} ({target_state})"
        )
    return "\n".join(lines) + "\n"


def write_report(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("action", choices=["auto-layout"], help="Subcommand action.")
    parser.add_argument("ldtk", type=Path, help="Target .ldtk file to layout.")
    parser.add_argument(
        "--start",
        default="central_hub_main",
        help="Start level identifier or activeArea to anchor at --origin (default central_hub_main).",
    )
    parser.add_argument(
        "--origin",
        default="0,0",
        help="World coordinate for the requested start level/area top-left (default 0,0).",
    )
    parser.add_argument("--gap", type=int, default=256, help="Preferred px gap between packed groups.")
    parser.add_argument(
        "--grid",
        type=int,
        default=None,
        help="Snap group anchors to this grid. Defaults to project worldGridWidth.",
    )
    parser.add_argument("--dry-run", action="store_true", help="Print the proposed layout without writing.")
    parser.add_argument("--report", type=Path, default=None, help="Optional report output path.")
    parser.add_argument("--in-place", action="store_true", help="Write back to the input .ldtk path.")
    parser.add_argument("--output", type=Path, default=None, help="Output path (alternative to --in-place).")
    parser.add_argument("--backup", action="store_true", help="When using --in-place, copy original to <ldtk>.bak first.")
    parser.add_argument("--no-repair", action="store_true", help="Skip repair + validate post-pass.")
    parser.add_argument(
        "--schema",
        type=Path,
        default=REPO_ROOT / "tools" / "ambition_ldtk_tools" / "schemas" / "ldtk" / "JSON_SCHEMA.json",
    )
    args = parser.parse_args(argv)

    if args.action != "auto-layout":
        return _fail(f"unknown world action '{args.action}'")
    if not args.ldtk.exists():
        return _fail(f"ldtk file not found: {args.ldtk}")
    if args.dry_run and (args.in_place or args.output is not None):
        return _fail("--dry-run cannot be combined with --in-place or --output")
    if not args.dry_run and not args.in_place and args.output is None:
        return _fail("choose --dry-run, --in-place, or --output <path>")

    try:
        ox_s, oy_s = args.origin.split(",", 1)
        origin = Point(int(ox_s), int(oy_s))
    except Exception:
        return _fail("--origin must be X,Y")

    project = json.loads(args.ldtk.read_text())
    result = auto_layout(project, start=args.start, origin=origin, grid=args.grid, gap=args.gap)
    print(result.report, end="")
    print(
        f"planned/moved {result.moved_levels} level(s); "
        f"updated cached coords for {result.updated_entities} entit(y/ies)."
    )
    if args.report:
        write_report(args.report, result.report)
        print(f"wrote report: {args.report}")
    if args.dry_run:
        return 0

    target = args.output or args.ldtk
    if args.in_place and args.backup:
        backup = args.ldtk.with_suffix(args.ldtk.suffix + ".bak")
        shutil.copy2(args.ldtk, backup)
        print(f"wrote backup: {backup}")
    target.write_text(dump_editor_style(project))
    print(f"wrote {target}")

    if args.no_repair:
        return 0
    cmd = [sys.executable, "-m", "ambition_ldtk_tools.repair", str(target), "--in-place"]
    print("$ " + " ".join(cmd))
    rc = subprocess.run(cmd).returncode
    if rc != 0:
        return rc
    cmd = [sys.executable, "-m", "ambition_ldtk_tools.validate", str(target)]
    if args.schema and args.schema.exists():
        cmd.extend(["--schema", str(args.schema), "--require-schema"])
    print("$ " + " ".join(cmd))
    return subprocess.run(cmd).returncode


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
