"""Room inspection and textual summaries."""

from __future__ import annotations

from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Iterator

from ambition_ldtk_tools.edit.intgrid import LAYER_VALUE_NAMES
from ambition_ldtk_tools.edit.set_field import find_level
from ambition_ldtk_tools.ldtk import default_sandbox_ldtk
from ambition_ldtk_tools.ldtk.issues import Issue
from ambition_ldtk_tools.room_support.issues import room_issues
from ambition_ldtk_tools.area_authoring import load_project
from ambition_ldtk_tools.edit.intgrid import find_intgrid_layer

def _find_repo_root() -> Path:
    cwd = Path.cwd().resolve()
    if (cwd / "crates" / "ambition_actors").exists():
        return cwd
    here = Path(__file__).resolve()
    for parent in [here.parent, *here.parents]:
        if (parent / "crates" / "ambition_actors").exists():
            return parent
    # Fall back to the historical repo-relative layout used by installed tools;
    # callers can always pass --ldtk explicitly.
    return here.parents[4]


REPO_ROOT = _find_repo_root()
DEFAULT_LDTK = default_sandbox_ldtk(REPO_ROOT)

INTGRID_COLORS: dict[str, dict[int, str]] = {
    "Collision": {
        1: "#6b7280",  # Solid
        2: "#f59e0b",  # OneWayUp
        3: "#38bdf8",  # BlinkSoft
        4: "#0ea5e9",  # BlinkHard
        5: "#ef4444",  # Hazard
    },
    "Water": {1: "#2563eb", 2: "#1e40af"},
    "Climbable": {1: "#84cc16", 2: "#65a30d", 3: "#a3e635"},
}

ENTITY_COLORS: dict[str, str] = {
    "PlayerStart": "#22c55e",
    "LoadingZone": "#f97316",
    "GravityZone": "#a855f7",
    "MovingPlatform": "#eab308",
    "KinematicPath": "#fde047",
    "CameraZone": "#14b8a6",
    "Portal": "#ec4899",
    "Switch": "#f43f5e",
    "LockWall": "#b91c1c",
    "EnemySpawn": "#dc2626",
    "NpcSpawn": "#60a5fa",
    "PickupSpawn": "#34d399",
    "PogoOrb": "#facc15",
    "ReboundPad": "#fb923c",
    "DamageVolume": "#ef4444",
}


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

    def intersects(self, other: "Rect") -> bool:
        return self.x < other.x2 and self.x2 > other.x and self.y < other.y2 and self.y2 > other.y


def _field_map(entity: dict) -> dict[str, object]:
    return {f.get("__identifier"): f.get("__value") for f in entity.get("fieldInstances", [])}


def _entity_rect(entity: dict) -> Rect:
    px = entity.get("px") or [0, 0]
    return Rect(int(px[0]), int(px[1]), int(entity.get("width") or 0), int(entity.get("height") or 0))


def _iter_entity_layers(level: dict) -> Iterator[dict]:
    for layer in level.get("layerInstances") or []:
        if layer.get("__type") == "Entities":
            yield layer


def _iter_entities(level: dict) -> Iterator[tuple[dict, dict]]:
    for layer in _iter_entity_layers(level):
        for entity in layer.get("entityInstances") or []:
            yield layer, entity


def _intgrid_layers(level: dict) -> list[dict]:
    return [
        layer
        for layer in level.get("layerInstances") or []
        if layer.get("__type") == "IntGrid"
    ]


def _intgrid_stats(layer: dict) -> dict[int, dict[str, object]]:
    grid = int(layer.get("__gridSize") or 16)
    c_wid = int(layer.get("__cWid") or 0)
    c_hei = int(layer.get("__cHei") or 0)
    csv = layer.get("intGridCsv") or []
    stats: dict[int, dict[str, object]] = {}
    for cy in range(c_hei):
        for cx in range(c_wid):
            idx = cy * c_wid + cx
            if idx >= len(csv):
                continue
            value = int(csv[idx] or 0)
            if value == 0:
                continue
            cur = stats.setdefault(value, {"count": 0, "bbox_cells": [cx, cy, cx, cy]})
            cur["count"] = int(cur["count"]) + 1
            bx0, by0, bx1, by1 = cur["bbox_cells"]  # type: ignore[misc]
            cur["bbox_cells"] = [min(bx0, cx), min(by0, cy), max(bx1, cx), max(by1, cy)]
    for value, cur in stats.items():
        bx0, by0, bx1, by1 = cur["bbox_cells"]  # type: ignore[misc]
        cur["bbox_px"] = [bx0 * grid, by0 * grid, (bx1 + 1) * grid, (by1 + 1) * grid]
    return stats


def room_summary(project: dict, level_id: str) -> dict[str, object]:
    level = find_level(project, level_id)
    fields = {f.get("__identifier"): f.get("__value") for f in level.get("fieldInstances", [])}
    entity_counts: Counter[str] = Counter()
    layer_entity_counts: dict[str, Counter[str]] = defaultdict(Counter)
    entities: list[dict[str, object]] = []
    gravity_zones: list[dict[str, object]] = []
    loading_zones: list[dict[str, object]] = []
    moving_platforms: list[dict[str, object]] = []
    camera_zones: list[dict[str, object]] = []
    player_starts: list[dict[str, object]] = []

    for layer, entity in _iter_entities(level):
        ident = str(entity.get("__identifier"))
        rect = _entity_rect(entity)
        fields_map = _field_map(entity)
        row = {
            "layer": layer.get("__identifier"),
            "identifier": ident,
            "iid": entity.get("iid"),
            "px": [rect.x, rect.y],
            "size": [rect.w, rect.h],
            "fields": fields_map,
        }
        entities.append(row)
        entity_counts[ident] += 1
        layer_entity_counts[str(layer.get("__identifier"))][ident] += 1
        if ident == "GravityZone":
            gravity_zones.append(row)
        elif ident == "LoadingZone":
            loading_zones.append(row)
        elif ident == "MovingPlatform":
            moving_platforms.append(row)
        elif ident == "CameraZone":
            camera_zones.append(row)
        elif ident == "PlayerStart":
            player_starts.append(row)

    intgrid: dict[str, object] = {}
    for layer in _intgrid_layers(level):
        layer_id = str(layer.get("__identifier"))
        names = LAYER_VALUE_NAMES.get(layer_id, {})
        stats = _intgrid_stats(layer)
        intgrid[layer_id] = {
            "grid_size": int(layer.get("__gridSize") or 16),
            "cells": [int(layer.get("__cWid") or 0), int(layer.get("__cHei") or 0)],
            "values": {
                str(v): {
                    "name": names.get(v, f"value{v}"),
                    "count": data["count"],
                    "bbox_cells": data["bbox_cells"],
                    "bbox_px": data["bbox_px"],
                }
                for v, data in sorted(stats.items())
            },
        }

    return {
        "identifier": level.get("identifier"),
        "iid": level.get("iid"),
        "uid": level.get("uid"),
        "world": [level.get("worldX"), level.get("worldY")],
        "size": [level.get("pxWid"), level.get("pxHei")],
        "fields": fields,
        "intgrid": intgrid,
        "entity_counts": dict(sorted(entity_counts.items())),
        "layer_entity_counts": {
            layer: dict(sorted(counts.items()))
            for layer, counts in sorted(layer_entity_counts.items())
        },
        "player_starts": player_starts,
        "gravity_zones": gravity_zones,
        "loading_zones": loading_zones,
        "moving_platforms": moving_platforms,
        "camera_zones": camera_zones,
        "entities": entities,
        "issues": [issue.as_dict() for issue in room_issues(level, intgrid, entities)],
    }


def format_summary_text(summary: dict[str, object], *, include_entities: bool = False) -> str:
    lines: list[str] = []
    lines.append(f"Room: {summary['identifier']}")
    lines.append(f"Size: {summary['size'][0]}x{summary['size'][1]} px    World: {summary['world']}")
    fields = summary.get("fields") or {}
    if fields:
        pairs = ", ".join(f"{k}={v!r}" for k, v in sorted(fields.items()))  # type: ignore[union-attr]
        lines.append(f"Level fields: {pairs}")
    lines.append("")
    lines.append("IntGrid:")
    intgrid = summary.get("intgrid") or {}
    if not intgrid:
        lines.append("  (none)")
    for layer_id, layer in intgrid.items():  # type: ignore[union-attr]
        layer = layer  # type: ignore[assignment]
        lines.append(
            f"  {layer_id}: grid={layer['grid_size']} cells={layer['cells'][0]}x{layer['cells'][1]}"  # type: ignore[index]
        )
        values = layer.get("values") or {}  # type: ignore[union-attr]
        if not values:
            lines.append("    (empty)")
            continue
        for value, data in values.items():
            lines.append(
                f"    {value:>2} {data['name']:<12} count={data['count']:<5} bbox_px={data['bbox_px']}"  # type: ignore[index]
            )
    lines.append("")
    lines.append("Entities:")
    counts = summary.get("entity_counts") or {}
    if not counts:
        lines.append("  (none)")
    else:
        chunks = [f"{k}={v}" for k, v in sorted(counts.items())]  # type: ignore[union-attr]
        lines.append("  " + ", ".join(chunks))

    def list_named(title: str, key: str, field_names: tuple[str, ...]) -> None:
        rows = summary.get(key) or []
        lines.append("")
        lines.append(f"{title} ({len(rows)}):")
        if not rows:
            lines.append("  (none)")
            return
        for row in rows:  # type: ignore[assignment]
            fields = row.get("fields") or {}
            bits = []
            for name in field_names:
                if name in fields:
                    bits.append(f"{name}={fields[name]!r}")
            suffix = " " + ", ".join(bits) if bits else ""
            lines.append(
                f"  {row['identifier']} layer={row['layer']} px={row['px']} size={row['size']}{suffix}"
            )

    list_named("Player starts", "player_starts", ("name",))
    list_named("Gravity zones", "gravity_zones", ("id", "name", "dir"))
    list_named("Loading zones", "loading_zones", ("id", "activation", "target_room", "target_zone"))
    list_named("Moving platforms", "moving_platforms", ("name", "sweep_dx", "speed", "path_id"))
    list_named("Camera zones", "camera_zones", ("id", "mode", "priority"))

    issues = summary.get("issues") or []
    lines.append("")
    lines.append(f"Issues / review notes ({len(issues)}):")
    if not issues:
        lines.append("  (none detected by static room summary)")
    else:
        for issue in issues:  # type: ignore[assignment]
            if isinstance(issue, dict):
                code = issue.get("code", "room.issue")
                message = issue.get("message", issue)
                severity = issue.get("severity", "warning")
                lines.append(f"  - {severity}: {code}: {message}")
            else:
                lines.append(f"  - {issue}")

    if include_entities:
        lines.append("")
        lines.append("All entities:")
        for row in summary.get("entities") or []:  # type: ignore[assignment]
            lines.append(
                f"  {row['identifier']} layer={row['layer']} iid={row['iid']} px={row['px']} size={row['size']} fields={row['fields']}"
            )
    return "\n".join(lines) + "\n"
