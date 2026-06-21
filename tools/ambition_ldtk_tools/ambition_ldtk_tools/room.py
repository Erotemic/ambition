#!/usr/bin/env python3
"""Room-level inspection, rendering, and debug bundling helpers.

These commands are intentionally sandbox-friendly: they are read-only by
default, pure Python, and emit compact artifacts a chat agent can inspect
without launching LDtk or the game.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import math
import os
import struct
import subprocess
import sys
import tarfile
import tempfile
import zlib
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Iterator

from ambition_ldtk_tools.area_authoring import load_project
from ambition_ldtk_tools.edit.intgrid import LAYER_VALUE_NAMES, find_intgrid_layer
from ambition_ldtk_tools.edit.set_field import find_level
from ambition_ldtk_tools.ldtk.issues import Issue
from ambition_ldtk_tools.room_support.issues import room_issues

def _find_repo_root() -> Path:
    cwd = Path.cwd().resolve()
    if (cwd / "crates" / "ambition_gameplay_core").exists():
        return cwd
    here = Path(__file__).resolve()
    for parent in [here.parent, *here.parents]:
        if (parent / "crates" / "ambition_gameplay_core").exists():
            return parent
    # Fall back to the historical repo-relative layout used by installed tools;
    # callers can always pass --ldtk explicitly.
    return here.parents[4]


REPO_ROOT = _find_repo_root()
DEFAULT_LDTK = (
    REPO_ROOT
    / "crates"
    / "ambition_gameplay_core"
    / "assets"
    / "ambition"
    / "worlds"
    / "sandbox.ldtk"
)

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


def _svg_escape(s: object) -> str:
    return str(s).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace('"', "&quot;")


def render_room_svg(project: dict, level_id: str, *, max_width: int = 1400) -> str:
    level = find_level(project, level_id)
    width = int(level.get("pxWid") or 0)
    height = int(level.get("pxHei") or 0)
    scale = min(1.0, max_width / max(width, 1))
    stroke = max(1.0, 1.5 / max(scale, 0.01))
    font = max(12, int(13 / max(scale, 0.01)))
    parts: list[str] = []
    parts.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width * scale:.0f}" height="{height * scale:.0f}" viewBox="0 0 {width} {height}">'
    )
    parts.append('<rect x="0" y="0" width="100%" height="100%" fill="#111827"/>')
    parts.append(f'<rect x="0" y="0" width="{width}" height="{height}" fill="none" stroke="#f9fafb" stroke-width="{stroke}"/>')
    for layer in reversed(level.get("layerInstances") or []):
        if layer.get("__type") == "IntGrid":
            _append_svg_intgrid(parts, layer)
    for layer in _iter_entity_layers(level):
        for entity in layer.get("entityInstances") or []:
            _append_svg_entity(parts, layer, entity, stroke=stroke, font=font)
    parts.append("</svg>")
    return "\n".join(parts) + "\n"


def _append_svg_intgrid(parts: list[str], layer: dict) -> None:
    layer_id = str(layer.get("__identifier"))
    colors = INTGRID_COLORS.get(layer_id, {})
    grid = int(layer.get("__gridSize") or 16)
    c_wid = int(layer.get("__cWid") or 0)
    c_hei = int(layer.get("__cHei") or 0)
    csv = layer.get("intGridCsv") or []
    opacity = {"Collision": 0.82, "Water": 0.38, "Climbable": 0.5}.get(layer_id, 0.45)
    parts.append(f'<g id="intgrid-{_svg_escape(layer_id)}" opacity="{opacity}">')
    for cy in range(c_hei):
        run_value = 0
        run_start = 0
        for cx in range(c_wid + 1):
            value = int(csv[cy * c_wid + cx] or 0) if cx < c_wid and cy * c_wid + cx < len(csv) else 0
            if cx == 0:
                run_value = value
                run_start = 0
                continue
            if value != run_value:
                if run_value != 0:
                    color = colors.get(run_value, "#d1d5db")
                    parts.append(
                        f'<rect x="{run_start * grid}" y="{cy * grid}" width="{(cx - run_start) * grid}" height="{grid}" fill="{color}"/>'
                    )
                run_value = value
                run_start = cx
    parts.append("</g>")


def _append_svg_entity(parts: list[str], layer: dict, entity: dict, *, stroke: float, font: int) -> None:
    ident = str(entity.get("__identifier"))
    rect = _entity_rect(entity)
    color = ENTITY_COLORS.get(ident, "#e5e7eb")
    fields = _field_map(entity)
    label = fields.get("id") or fields.get("name") or ident
    fill_opacity = 0.16 if ident not in {"GravityZone", "CameraZone", "DamageVolume"} else 0.10
    parts.append(
        f'<rect x="{rect.x}" y="{rect.y}" width="{rect.w}" height="{rect.h}" fill="{color}" fill-opacity="{fill_opacity}" stroke="{color}" stroke-width="{stroke}"/>'
    )
    # Draw a simple sweep/path affordance for common moving-platform authoring.
    if ident == "MovingPlatform":
        sweep = fields.get("sweep_dx")
        if isinstance(sweep, (int, float)) and sweep:
            y = rect.y + rect.h / 2
            x0 = rect.x + rect.w / 2
            x1 = x0 + float(sweep)
            parts.append(
                f'<line x1="{x0}" y1="{y}" x2="{x1}" y2="{y}" stroke="{color}" stroke-width="{stroke * 2}" stroke-dasharray="8 6"/>'
            )
    if ident == "KinematicPath":
        points = fields.get("points")
        if isinstance(points, str):
            coords = _parse_points(points)
            if len(coords) >= 2:
                d = " ".join(f"L {x} {y}" for x, y in coords[1:])
                x0, y0 = coords[0]
                parts.append(f'<path d="M {x0} {y0} {d}" fill="none" stroke="{color}" stroke-width="{stroke * 2}" stroke-dasharray="6 5"/>')
    if rect.w >= 20 and rect.h >= 12:
        parts.append(
            f'<text x="{rect.x + 3}" y="{rect.y + max(12, font)}" fill="{color}" font-family="monospace" font-size="{font}" paint-order="stroke" stroke="#111827" stroke-width="3">{_svg_escape(label)}</text>'
        )


def _parse_points(points: str) -> list[tuple[int, int]]:
    out: list[tuple[int, int]] = []
    for part in points.split(";"):
        if not part.strip():
            continue
        xy = part.split(",")
        if len(xy) != 2:
            continue
        try:
            out.append((int(float(xy[0])), int(float(xy[1]))))
        except ValueError:
            continue
    return out


def render_room_png(project: dict, level_id: str, out: Path, *, max_width: int = 1400) -> None:
    level = find_level(project, level_id)
    width = int(level.get("pxWid") or 0)
    height = int(level.get("pxHei") or 0)
    scale = min(1.0, max_width / max(width, 1))
    out_w = max(1, int(math.ceil(width * scale)))
    out_h = max(1, int(math.ceil(height * scale)))
    pixels = bytearray([17, 24, 39, 255] * out_w * out_h)

    def draw_rect(rect: Rect, color: tuple[int, int, int, int]) -> None:
        x0 = max(0, int(rect.x * scale))
        y0 = max(0, int(rect.y * scale))
        x1 = min(out_w, max(x0 + 1, int(math.ceil(rect.x2 * scale))))
        y1 = min(out_h, max(y0 + 1, int(math.ceil(rect.y2 * scale))))
        r, g, b, a = color
        inv = 255 - a
        for y in range(y0, y1):
            row = y * out_w * 4
            for x in range(x0, x1):
                i = row + x * 4
                pixels[i] = (r * a + pixels[i] * inv) // 255
                pixels[i + 1] = (g * a + pixels[i + 1] * inv) // 255
                pixels[i + 2] = (b * a + pixels[i + 2] * inv) // 255
                pixels[i + 3] = 255

    for layer in reversed(level.get("layerInstances") or []):
        if layer.get("__type") != "IntGrid":
            continue
        layer_id = str(layer.get("__identifier"))
        colors = INTGRID_COLORS.get(layer_id, {})
        alpha = {"Collision": 210, "Water": 90, "Climbable": 120}.get(layer_id, 120)
        grid = int(layer.get("__gridSize") or 16)
        c_wid = int(layer.get("__cWid") or 0)
        c_hei = int(layer.get("__cHei") or 0)
        csv = layer.get("intGridCsv") or []
        for cy in range(c_hei):
            for cx in range(c_wid):
                idx = cy * c_wid + cx
                if idx >= len(csv):
                    continue
                value = int(csv[idx] or 0)
                if value == 0:
                    continue
                draw_rect(Rect(cx * grid, cy * grid, grid, grid), _hex_rgba(colors.get(value, "#d1d5db"), alpha))
    for layer in _iter_entity_layers(level):
        for entity in layer.get("entityInstances") or []:
            color = _hex_rgba(ENTITY_COLORS.get(str(entity.get("__identifier")), "#e5e7eb"), 115)
            draw_rect(_entity_rect(entity), color)
    _write_png(out, out_w, out_h, pixels)


def _hex_rgba(color: str, alpha: int) -> tuple[int, int, int, int]:
    color = color.lstrip("#")
    return int(color[0:2], 16), int(color[2:4], 16), int(color[4:6], 16), alpha


def _write_png(path: Path, width: int, height: int, rgba: bytes | bytearray) -> None:
    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    raw = bytearray()
    stride = width * 4
    for y in range(height):
        raw.append(0)
        raw.extend(rgba[y * stride : (y + 1) * stride])
    data = b"".join(
        [
            b"\x89PNG\r\n\x1a\n",
            chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)),
            chunk(b"IDAT", zlib.compress(bytes(raw), 9)),
            chunk(b"IEND", b""),
        ]
    )
    path.write_bytes(data)


def _matching_specs(level_id: str, repo_root: Path) -> list[Path]:
    specs_root = repo_root / "tools" / "ambition_ldtk_tools" / "specs"
    if not specs_root.exists():
        return []
    matches: list[Path] = []
    needles = [f'level_id: "{level_id}"', f'id: "{level_id}"', f'"level_id": "{level_id}"', f'"id": "{level_id}"']
    for path in specs_root.rglob("*"):
        if path.suffix.lower() not in {".ron", ".json", ".yaml", ".yml"}:
            continue
        try:
            text = path.read_text(errors="ignore")
        except OSError:
            continue
        if any(n in text for n in needles) or level_id in path.stem:
            matches.append(path)
    return sorted(matches)


def _collect_debug_files(repo_root: Path, level_id: str, patterns: list[str]) -> list[Path]:
    debug_root = repo_root / "debug_traces"
    if not debug_root.exists():
        return []
    out: list[Path] = []
    for path in debug_root.rglob("*.json"):
        rel = path.relative_to(debug_root).as_posix()
        name = path.name
        if any(fnmatch.fnmatch(name, pat) or fnmatch.fnmatch(rel, pat) for pat in patterns):
            out.append(path)
        elif level_id in name:
            out.append(path)
    return sorted(set(out))


def write_bundle(
    *,
    project: dict,
    ldtk: Path,
    level_id: str,
    out: Path,
    repo_root: Path,
    render_format: str = "svg",
    include_debug: bool = True,
    run_validate: bool = False,
) -> None:
    summary = room_summary(project, level_id)
    with tempfile.TemporaryDirectory(prefix="ambition-room-bundle-") as td:
        temp = Path(td)
        (temp / "room_describe.txt").write_text(format_summary_text(summary, include_entities=True))
        (temp / "room_describe.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
        if render_format == "png":
            render_room_png(project, level_id, temp / "room.png")
        else:
            (temp / "room.svg").write_text(render_room_svg(project, level_id))
        (temp / "README.txt").write_text(
            "Ambition LDtk room debug bundle\n"
            f"level={level_id}\n"
            f"ldtk={ldtk}\n"
            "Generated by: python -m ambition_ldtk_tools room bundle-debug\n"
        )
        specs = _matching_specs(level_id, repo_root)
        if specs:
            specs_dir = temp / "specs"
            specs_dir.mkdir()
            for spec in specs:
                target = specs_dir / spec.name
                target.write_bytes(spec.read_bytes())
        if include_debug:
            debug_files = _collect_debug_files(
                repo_root,
                level_id,
                [
                    "gravity_symmetry_room_failure_*.json",
                    "gravity_symmetry_room_failures/*.json",
                    "*failure*.json",
                ],
            )
            if debug_files:
                debug_dir = temp / "debug_traces"
                debug_dir.mkdir()
                for src in debug_files:
                    target = debug_dir / src.name
                    # Avoid collisions from recursive directories.
                    if target.exists():
                        target = debug_dir / (src.parent.name + "__" + src.name)
                    target.write_bytes(src.read_bytes())
        if run_validate:
            rc = subprocess.run(
                [sys.executable, "-m", "ambition_ldtk_tools.validate", str(ldtk)],
                cwd=repo_root,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
            )
            (temp / "validate.txt").write_text(rc.stdout + f"\nexit_code={rc.returncode}\n")
        out.parent.mkdir(parents=True, exist_ok=True)
        with tarfile.open(out, "w:gz") as tar:
            for path in sorted(temp.rglob("*")):
                if path.is_file():
                    tar.add(path, arcname=path.relative_to(temp))


def _cmd_describe(args: argparse.Namespace) -> int:
    project = load_project(args.ldtk)
    summary = room_summary(project, args.level)
    if args.format == "json":
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        print(format_summary_text(summary, include_entities=args.entities), end="")
    return 0


def _cmd_render(args: argparse.Namespace) -> int:
    project = load_project(args.ldtk)
    out = args.out
    out.parent.mkdir(parents=True, exist_ok=True)
    suffix = out.suffix.lower()
    if suffix == ".png":
        render_room_png(project, args.level, out, max_width=args.max_width)
    elif suffix == ".svg" or not suffix:
        if not suffix:
            out = out.with_suffix(".svg")
        out.write_text(render_room_svg(project, args.level, max_width=args.max_width))
    else:
        raise SystemExit("room render --out must end in .svg or .png")
    print(f"wrote {out}")
    return 0


def _cmd_bundle_debug(args: argparse.Namespace) -> int:
    project = load_project(args.ldtk)
    write_bundle(
        project=project,
        ldtk=args.ldtk,
        level_id=args.level,
        out=args.out,
        repo_root=args.repo_root,
        render_format=args.render_format,
        include_debug=not args.no_debug,
        run_validate=args.validate,
    )
    print(f"wrote {args.out}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--ldtk",
        type=Path,
        default=DEFAULT_LDTK,
        help="LDtk project path (default: Ambition sandbox.ldtk)",
    )
    sub = parser.add_subparsers(dest="action", required=True)

    describe = sub.add_parser("describe", help="Print a structured room summary")
    describe.add_argument("--level", required=True, help="level identifier")
    describe.add_argument("--format", choices=["text", "json"], default="text")
    describe.add_argument("--entities", action="store_true", help="include every entity row")
    describe.set_defaults(func=_cmd_describe)

    render = sub.add_parser("render", help="Render room geometry/entities to SVG or PNG")
    render.add_argument("--level", required=True, help="level identifier")
    render.add_argument("--out", required=True, type=Path, help="output .svg or .png")
    render.add_argument("--max-width", type=int, default=1400, help="maximum rendered pixel width")
    render.set_defaults(func=_cmd_render)

    bundle = sub.add_parser("bundle-debug", help="Create a chat-sandbox friendly room debug tarball")
    bundle.add_argument("--level", required=True, help="level identifier")
    bundle.add_argument("--out", required=True, type=Path, help="output .tar.gz")
    bundle.add_argument("--repo-root", type=Path, default=REPO_ROOT)
    bundle.add_argument("--render-format", choices=["svg", "png"], default="svg")
    bundle.add_argument("--no-debug", action="store_true", help="do not include debug_traces JSON files")
    bundle.add_argument("--validate", action="store_true", help="include validate command output")
    bundle.set_defaults(func=_cmd_bundle_debug)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())
