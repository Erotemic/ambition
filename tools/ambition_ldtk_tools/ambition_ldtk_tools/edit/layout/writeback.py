"""LDtk writeback and textual reports for world auto-layout."""

from __future__ import annotations

from pathlib import Path

from ambition_ldtk_tools.edit.layout.model import GroupInfo, LayoutEdge, Point

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
    packing_padding: int,
    locked_groups: set[str],
    strategy: str,
    clusters: list[list[str]],
) -> str:
    lines: list[str] = []
    lines.append("LDtk world auto-layout report")
    lines.append(f"start group: {start_group_id}")
    lines.append(f"strategy={strategy}")
    lines.append(f"grid={grid} gap={gap} padding={packing_padding}")
    multi_clusters = [c for c in clusters if len(c) > 1]
    if multi_clusters:
        lines.append(f"linkage clusters: {len(multi_clusters)}")
        for cluster in multi_clusters[:24]:
            lines.append("  cluster: " + ", ".join(cluster))
        if len(multi_clusters) > 24:
            lines.append(f"  ... {len(multi_clusters) - 24} more")
    if locked_groups:
        lines.append("locked groups: " + ", ".join(sorted(locked_groups)))
    lines.append("")
    lines.append(f"Groups ({len(groups)}):")
    for group_id in sorted(groups):
        group = groups[group_id]
        p = placements[group_id]
        level_ids = ", ".join(level.identifier for level in group.levels)
        lock_mark = " locked" if group_id in locked_groups else ""
        lines.append(
            f"  {group_id:28s} at ({p.x:>6}, {p.y:>6}) "
            f"size={group.rect.w}x{group.rect.h}{lock_mark} levels=[{level_ids}]"
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
