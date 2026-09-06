"""Selectable layout backends for LDtk world auto-layout."""

from __future__ import annotations

from collections import defaultdict, deque
from typing import Iterable

from ambition_ldtk_tools.edit.layout.model import GroupInfo, LayoutEdge, LayoutResult, Point, Rect, ZoneRef
from ambition_ldtk_tools.edit.layout.graph import (
    build_adjacency,
    build_edges,
    build_groups,
    choose_start_group,
    desired_anchor,
    level_top_left_relative_to_group,
    locked_groups_from_level_fields,
    place_disconnected_components,
    place_without_overlap,
    resolve_group_ids,
    seed_placements,
    snap,
    snap_point,
)
from ambition_ldtk_tools.edit.layout.writeback import format_report, update_entity_world_coords

def layout_greedy(
    groups: dict[str, GroupInfo],
    adjacency: dict[str, list[LayoutEdge]],
    *,
    start_group_id: str,
    start_anchor: Point,
    origin: Point,
    grid: int,
    gap: int,
    packing_padding: int,
    locked_groups: set[str],
) -> dict[str, Point]:
    placements, placed_rects, q = seed_placements(
        groups,
        start_group_id=start_group_id,
        start_anchor=start_anchor,
        locked_groups=locked_groups,
    )
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
                        gap=packing_padding,
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
                gap=packing_padding,
                direction=edge.direction,
            )
            placements[neighbor] = placed
            placed_rects.append((neighbor, group.rect.translated(placed)))
            q.append(neighbor)

    place_disconnected_components(
        groups,
        placements,
        placed_rects,
        origin=origin,
        grid=grid,
        gap=gap,
        packing_padding=packing_padding,
    )
    return placements


def _direction_vec(direction: str) -> tuple[int, int]:
    if direction == "left":
        return (-1, 0)
    if direction == "right":
        return (1, 0)
    if direction == "up":
        return (0, -1)
    return (0, 1)


def layout_layered(
    groups: dict[str, GroupInfo],
    adjacency: dict[str, list[LayoutEdge]],
    *,
    start_group_id: str,
    start_anchor: Point,
    origin: Point,
    grid: int,
    gap: int,
    packing_padding: int,
    locked_groups: set[str],
) -> dict[str, Point]:
    """Sugiyama-inspired rank layout for hub/layer maps.

    This is intentionally simple: infer integer rank coordinates by walking the
    LoadingZone graph in door directions, then pack each horizontal rank from
    left to right. It emphasizes readable strata over exact door-coordinate
    alignment. Locks are treated as fixed obstacles.
    """

    rank: dict[str, tuple[int, int]] = {start_group_id: (0, 0)}
    q: deque[str] = deque([start_group_id])
    while q:
        current = q.popleft()
        cx, cy = rank[current]
        for edge in sorted(adjacency.get(current, []), key=lambda e: (e.direction, e.target_group_id, e.source.group_id)):
            if edge.source.group_id == current:
                neighbor = edge.target_group_id
                dx, dy = _direction_vec(edge.direction)
            else:
                neighbor = edge.source.group_id
                dx, dy = _direction_vec(edge.direction)
                dx, dy = -dx, -dy
            if neighbor in rank or neighbor not in groups:
                continue
            rank[neighbor] = (cx + dx, cy + dy)
            q.append(neighbor)

    # Unreached nodes get shelves below the reached ranks.
    next_unreached_y = max((y for _x, y in rank.values()), default=0) + 2
    for group_id in sorted(groups):
        if group_id not in rank:
            rank[group_id] = (0, next_unreached_y)
            next_unreached_y += 1

    placements, placed_rects, _q = seed_placements(
        groups,
        start_group_id=start_group_id,
        start_anchor=start_anchor,
        locked_groups=locked_groups,
    )

    rows: dict[int, list[str]] = defaultdict(list)
    for group_id, (_rx, ry) in rank.items():
        rows[ry].append(group_id)

    # Determine row baselines from top to bottom. Preserve the requested start
    # row at start_anchor.y and build rows above/below with max row heights.
    sorted_rows = sorted(rows)
    row_heights = {ry: max(groups[group_id].rect.h for group_id in rows[ry]) for ry in rows}
    row_y: dict[int, int] = {rank[start_group_id][1]: start_anchor.y}
    cursor = start_anchor.y
    for ry in sorted(r for r in sorted_rows if r > rank[start_group_id][1]):
        prev_rows = [r for r in row_y if r < ry]
        prev = max(prev_rows, default=rank[start_group_id][1])
        cursor = row_y[prev] + row_heights[prev] + gap
        row_y[ry] = snap(cursor, grid)
    for ry in reversed([r for r in sorted_rows if r < rank[start_group_id][1]]):
        next_rows = [r for r in row_y if r > ry]
        nxt = min(next_rows, default=rank[start_group_id][1])
        cursor = row_y[nxt] - row_heights[ry] - gap
        row_y[ry] = snap(cursor, grid)

    for ry in sorted_rows:
        ordered = sorted(rows[ry], key=lambda gid: (rank[gid][0], gid))
        total_w = sum(groups[gid].rect.w for gid in ordered) + gap * max(0, len(ordered) - 1)
        # Center each row around the start anchor. The start row is shifted so
        # the requested start group remains exactly anchored unless it is locked.
        x = snap(origin.x - total_w // 2, grid)
        if start_group_id in ordered:
            before_w = 0
            for gid in ordered:
                if gid == start_group_id:
                    break
                before_w += groups[gid].rect.w + gap
            x = start_anchor.x - before_w
        for gid in ordered:
            if gid in placements:
                continue
            desired = Point(x, row_y[ry])
            placed = place_without_overlap(groups[gid], desired, placed_rects, grid=grid, gap=packing_padding)
            placements[gid] = placed
            placed_rects.append((gid, groups[gid].rect.translated(placed)))
            x = snap(placed.x + groups[gid].rect.w + gap, grid)

    return placements


class _UnionFind:
    def __init__(self, items: Iterable[str]):
        self.parent = {item: item for item in items}

    def find(self, item: str) -> str:
        parent = self.parent[item]
        if parent != item:
            self.parent[item] = self.find(parent)
        return self.parent[item]

    def union(self, a: str, b: str) -> None:
        ra = self.find(a)
        rb = self.find(b)
        if ra == rb:
            return
        if rb < ra:
            ra, rb = rb, ra
        self.parent[rb] = ra


def linkage_clusters(
    groups: dict[str, GroupInfo],
    edges: list[LayoutEdge],
    *,
    locked_groups: set[str],
    min_links: int = 2,
    degree_limit: int = 4,
) -> list[list[str]]:
    """Find low-degree, tightly linked room islands for clustered layout.

    Bidirectional door pairs have weight 2, so the default clusters sequential
    chains/corridors while avoiding high-degree hub absorption. Locked groups are
    left as singleton islands so author landmarks remain stable obstacles.
    """

    degree: dict[str, set[str]] = defaultdict(set)
    weights: dict[tuple[str, str], int] = defaultdict(int)
    for edge in edges:
        a = edge.source.group_id
        b = edge.target_group_id
        if a == b or a not in groups or b not in groups:
            continue
        degree[a].add(b)
        degree[b].add(a)
        key = tuple(sorted((a, b)))
        weights[key] += 1

    uf = _UnionFind(groups)
    for (a, b), weight in sorted(weights.items(), key=lambda item: (-item[1], item[0])):
        if a in locked_groups or b in locked_groups:
            continue
        if weight < min_links:
            continue
        if len(degree[a]) > degree_limit or len(degree[b]) > degree_limit:
            continue
        uf.union(a, b)

    buckets: dict[str, list[str]] = defaultdict(list)
    for group_id in groups:
        buckets[uf.find(group_id)].append(group_id)
    return [sorted(v) for _k, v in sorted(buckets.items(), key=lambda kv: (kv[0], kv[1]))]


def _union_rect(rects: Iterable[Rect]) -> Rect:
    rects = list(rects)
    if not rects:
        return Rect(0, 0, 0, 0)
    x0 = min(r.x for r in rects)
    y0 = min(r.y for r in rects)
    x1 = max(r.x2 for r in rects)
    y1 = max(r.y2 for r in rects)
    return Rect(0, 0, x1 - x0, y1 - y0)


def _normalize_local_placements(
    placements: dict[str, Point],
    groups: dict[str, GroupInfo],
) -> tuple[dict[str, Point], Rect]:
    rects = [groups[gid].rect.translated(p) for gid, p in placements.items()]
    if not rects:
        return placements, Rect(0, 0, 0, 0)
    min_x = min(r.x for r in rects)
    min_y = min(r.y for r in rects)
    out = {gid: Point(p.x - min_x, p.y - min_y) for gid, p in placements.items()}
    norm_rects = [groups[gid].rect.translated(p) for gid, p in out.items()]
    return out, _union_rect(norm_rects)


def _best_cluster_root(cluster: list[str], start_group_id: str, adjacency: dict[str, list[LayoutEdge]]) -> str:
    if start_group_id in cluster:
        return start_group_id
    return max(cluster, key=lambda gid: (len(adjacency.get(gid, [])), -len(gid), gid))


def layout_clustered(
    groups: dict[str, GroupInfo],
    edges: list[LayoutEdge],
    adjacency: dict[str, list[LayoutEdge]],
    *,
    start_group_id: str,
    start_anchor: Point,
    origin: Point,
    grid: int,
    gap: int,
    packing_padding: int,
    locked_groups: set[str],
    min_links: int,
    degree_limit: int,
) -> tuple[dict[str, Point], list[list[str]]]:
    clusters = linkage_clusters(
        groups,
        edges,
        locked_groups=locked_groups,
        min_links=min_links,
        degree_limit=degree_limit,
    )
    cluster_for_group = {gid: f"cluster_{idx:03d}" for idx, cluster in enumerate(clusters) for gid in cluster}
    local: dict[str, dict[str, Point]] = {}
    virtual_groups: dict[str, GroupInfo] = {}

    for idx, cluster in enumerate(clusters):
        cluster_id = f"cluster_{idx:03d}"
        if len(cluster) == 1:
            gid = cluster[0]
            local[cluster_id] = {gid: Point(0, 0)}
            virtual_groups[cluster_id] = GroupInfo(
                id=cluster_id,
                anchor=groups[gid].anchor,
                rect=groups[gid].rect,
            )
            continue
        subset = {gid: groups[gid] for gid in cluster}
        subset_adj = {
            gid: [e for e in adjacency.get(gid, []) if e.source.group_id in subset and e.target_group_id in subset]
            for gid in subset
        }
        root = _best_cluster_root(cluster, start_group_id, adjacency)
        sub = layout_greedy(
            subset,
            subset_adj,
            start_group_id=root,
            start_anchor=Point(0, 0),
            origin=Point(0, 0),
            grid=grid,
            gap=gap,
            packing_padding=packing_padding,
            locked_groups=set(),
        )
        sub, rect = _normalize_local_placements(sub, groups)
        local[cluster_id] = sub
        virtual_groups[cluster_id] = GroupInfo(id=cluster_id, rect=rect)

    virtual_edges: list[LayoutEdge] = []
    for edge in edges:
        if edge.target_group_id not in groups:
            continue
        a = cluster_for_group[edge.source.group_id]
        b = cluster_for_group[edge.target_group_id]
        if a == b:
            continue
        src_local = local[a][edge.source.group_id]
        dst_local = local[b][edge.target_group_id]
        source = ZoneRef(
            group_id=a,
            level_id=edge.source.level_id,
            zone_id=edge.source.zone_id,
            center_rel_group=src_local + edge.source.center_rel_group,
            center_rel_level=edge.source.center_rel_level,
            level_rect_rel_group=edge.source.level_rect_rel_group.translated(src_local),
            activation=edge.source.activation,
        )
        target = None
        if edge.target is not None:
            target = ZoneRef(
                group_id=b,
                level_id=edge.target.level_id,
                zone_id=edge.target.zone_id,
                center_rel_group=dst_local + edge.target.center_rel_group,
                center_rel_level=edge.target.center_rel_level,
                level_rect_rel_group=edge.target.level_rect_rel_group.translated(dst_local),
                activation=edge.target.activation,
            )
        virtual_edges.append(
            LayoutEdge(
                source=source,
                target=target,
                target_group_id=b,
                target_room_raw=edge.target_room_raw,
                target_zone_raw=edge.target_zone_raw,
                direction=edge.direction,
                weight=edge.weight,
            )
        )

    virtual_adjacency = build_adjacency(virtual_edges, virtual_groups)
    start_cluster = cluster_for_group[start_group_id]
    virtual_locks = {cluster_for_group[g] for g in locked_groups if g in cluster_for_group}
    start_local = local[start_cluster][start_group_id]
    virtual_start_anchor = Point(start_anchor.x - start_local.x, start_anchor.y - start_local.y)
    virtual_placements = layout_greedy(
        virtual_groups,
        virtual_adjacency,
        start_group_id=start_cluster,
        start_anchor=virtual_start_anchor,
        origin=origin,
        grid=grid,
        gap=gap,
        packing_padding=packing_padding,
        locked_groups=virtual_locks,
    )

    placements: dict[str, Point] = {}
    for cluster_id, members in local.items():
        base = virtual_placements[cluster_id]
        for gid, rel in members.items():
            placements[gid] = base + rel
    # Preserve exact locked group positions even when a locked singleton became a
    # virtual obstacle. This should be redundant but keeps the contract explicit.
    for gid in locked_groups:
        placements[gid] = groups[gid].anchor
    return placements, clusters


def auto_layout(
    project: dict,
    *,
    start: str,
    origin: Point = Point(0, 0),
    grid: int | None = None,
    gap: int = 256,
    padding: int | None = None,
    lock: Iterable[str] = (),
    lock_field: str = "layoutLocked",
    respect_field_locks: bool = True,
    strategy: str = "greedy",
    cluster_min_links: int = 2,
    cluster_degree_limit: int = 4,
) -> LayoutResult:
    groups, levels_by_id = build_groups(project)
    if not groups:
        raise SystemExit("project has no levels")
    if grid is None:
        grid = int(project.get("worldGridWidth") or 256)
    start_group_id = choose_start_group(start, groups, levels_by_id)
    start_group = groups[start_group_id]
    packing_padding = gap // 4 if padding is None else max(0, int(padding))
    locked_groups = resolve_group_ids(lock, groups, levels_by_id, label="--lock")
    if respect_field_locks:
        locked_groups |= locked_groups_from_level_fields(groups, lock_field=lock_field)

    # Anchor so the requested start level/active area lands at origin. If start
    # names a level inside a multi-level group, preserve the intra-group offset
    # and put that level's top-left at the origin.
    if start in levels_by_id:
        start_level_rel = level_top_left_relative_to_group(levels_by_id[start], start_group)
        start_anchor = Point(origin.x - start_level_rel.x, origin.y - start_level_rel.y)
    else:
        start_anchor = origin
    start_anchor = snap_point(start_anchor, grid)

    if start_group_id in locked_groups:
        # Explicit locks should be stable. If the caller wants the start group at
        # origin, unlock it for this pass or move it once before locking it.
        start_anchor = start_group.anchor

    edges = build_edges(project, groups, levels_by_id)
    unresolved_edges = [e for e in edges if e.target is None or e.target_group_id not in groups]
    adjacency = build_adjacency(edges, groups)
    clusters: list[list[str]] = []
    if strategy == "greedy":
        placements = layout_greedy(
            groups,
            adjacency,
            start_group_id=start_group_id,
            start_anchor=start_anchor,
            origin=origin,
            grid=grid,
            gap=gap,
            packing_padding=packing_padding,
            locked_groups=locked_groups,
        )
    elif strategy == "layered":
        placements = layout_layered(
            groups,
            adjacency,
            start_group_id=start_group_id,
            start_anchor=start_anchor,
            origin=origin,
            grid=grid,
            gap=gap,
            packing_padding=packing_padding,
            locked_groups=locked_groups,
        )
    elif strategy == "clustered":
        placements, clusters = layout_clustered(
            groups,
            edges,
            adjacency,
            start_group_id=start_group_id,
            start_anchor=start_anchor,
            origin=origin,
            grid=grid,
            gap=gap,
            packing_padding=packing_padding,
            locked_groups=locked_groups,
            min_links=cluster_min_links,
            degree_limit=cluster_degree_limit,
        )
    else:
        raise SystemExit(f"unknown auto-layout strategy '{strategy}'")

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

    report = format_report(
        groups,
        placements,
        edges,
        unresolved_edges,
        start_group_id,
        grid=grid,
        gap=gap,
        packing_padding=packing_padding,
        locked_groups=locked_groups,
        strategy=strategy,
        clusters=clusters,
    )
    return LayoutResult(
        placements=placements,
        groups=groups,
        edges=edges,
        unresolved_edges=unresolved_edges,
        moved_levels=moved_levels,
        updated_entities=updated_entities,
        report=report,
        locked_groups=locked_groups,
        packing_padding=packing_padding,
        strategy=strategy,
        clusters=clusters,
    )
