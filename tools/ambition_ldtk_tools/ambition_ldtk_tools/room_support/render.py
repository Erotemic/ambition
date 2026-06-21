"""Room SVG/PNG rendering helpers."""

from __future__ import annotations

import math
import struct
import zlib
from pathlib import Path

from ambition_ldtk_tools.edit.intgrid import LAYER_VALUE_NAMES
from ambition_ldtk_tools.edit.set_field import find_level
from ambition_ldtk_tools.room_support.inspect import ENTITY_COLORS, INTGRID_COLORS, Rect, _entity_rect, _field_map, _intgrid_layers, _iter_entities, _iter_entity_layers, room_summary

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
