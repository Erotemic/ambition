#!/usr/bin/env python3
"""Visual asset manifest helpers for LDtk editor art.

This module deliberately stops at the LDtk/editor-integration boundary. It does
not know the final Ambition sprite-generator schema; instead it consumes a small,
stable manifest shape that can be generated from today's fixtures and later from
whatever RON/YAML metadata the sprite refactor emits.
"""

from __future__ import annotations

import argparse
import json
import struct
import zlib
from dataclasses import dataclass
from html import escape
from pathlib import Path
from typing import Any

from ambition_ldtk_tools.ldtk import (
    LdtkTransaction,
    alloc_uid,
    entity_defs,
    find_entity_def,
    find_tileset,
    load_project,
    path_from_ldtk,
    png_dimensions,
    rel_to_ldtk,
    repo_root_from_ldtk,
    tileset_defs,
)

DEFAULT_ENTITY_ICON_ORDER = [
    "CameraZone",
    "LoadingZone",
    "PlayerStart",
    "GravityZone",
    "MovingPlatform",
    "KinematicPath",
    "WaterVolume",
    "DamageVolume",
    "NpcSpawn",
    "EnemySpawn",
    "PickupSpawn",
    "Portal",
    "Switch",
    "LockWall",
    "PogoOrb",
    "ReboundPad",
]

DEFAULT_ICON_COLORS = {
    "CameraZone": (20, 184, 166),
    "LoadingZone": (249, 115, 22),
    "PlayerStart": (34, 197, 94),
    "GravityZone": (168, 85, 247),
    "MovingPlatform": (234, 179, 8),
    "KinematicPath": (250, 204, 21),
    "WaterVolume": (37, 99, 235),
    "DamageVolume": (239, 68, 68),
    "NpcSpawn": (96, 165, 250),
    "EnemySpawn": (220, 38, 38),
    "PickupSpawn": (52, 211, 153),
    "Portal": (236, 72, 153),
    "Switch": (244, 63, 94),
    "LockWall": (185, 28, 28),
    "PogoOrb": (250, 204, 21),
    "ReboundPad": (251, 146, 60),
}


@dataclass(frozen=True)
class ManifestIssue:
    severity: str
    code: str
    message: str



def load_manifest(path: Path) -> dict[str, Any]:
    text = path.read_text()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        pass
    try:
        import yaml  # type: ignore
    except Exception as ex:  # pragma: no cover - environment-dependent
        raise SystemExit(
            f"{path} is not JSON and PyYAML is not installed; use JSON or install yaml support"
        ) from ex
    data = yaml.safe_load(text)
    if not isinstance(data, dict):
        raise SystemExit(f"manifest {path} must contain a mapping/object")
    return data


def save_manifest(path: Path, data: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.suffix.lower() in {".yaml", ".yml"}:
        try:
            import yaml  # type: ignore
        except Exception:
            path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
            return
        path.write_text(yaml.safe_dump(data, sort_keys=False))
    else:
        path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")


def normalize_manifest(raw: dict[str, Any]) -> dict[str, Any]:
    """Accept a few friendly shapes and normalize to the canonical internal one."""
    out: dict[str, Any] = {
        "tilesets": list(raw.get("tilesets") or []),
        "entity_icons": dict(raw.get("entity_icons") or raw.get("entities") or {}),
    }
    if "editor_icons" in raw:
        editor = dict(raw["editor_icons"] or {})
        if not any(ts.get("identifier") == editor.get("identifier", "EditorIcons") for ts in out["tilesets"]):
            out["tilesets"].append(
                {
                    "identifier": editor.get("identifier", "EditorIcons"),
                    "path": editor.get("path"),
                    "tile_width": int(editor.get("tile_width", editor.get("grid", 32))),
                    "tile_height": int(editor.get("tile_height", editor.get("grid", 32))),
                    "padding": int(editor.get("padding", 0)),
                    "spacing": int(editor.get("spacing", 0)),
                }
            )
    return out


def tileset_defs(project: dict) -> list[dict]:
    return project.setdefault("defs", {}).setdefault("tilesets", [])


def entity_defs(project: dict) -> list[dict]:
    return project.setdefault("defs", {}).setdefault("entities", [])


def find_tileset(project: dict, identifier: str) -> dict | None:
    for ts in tileset_defs(project):
        if ts.get("identifier") == identifier:
            return ts
    return None


def find_entity_def(project: dict, identifier: str) -> dict | None:
    for ent in entity_defs(project):
        if ent.get("identifier") == identifier:
            return ent
    return None


def build_tileset(project: dict, ldtk: Path, spec: dict[str, Any]) -> dict:
    ident = str(spec.get("identifier") or spec.get("id") or Path(str(spec.get("path", "tileset"))).stem)
    path_raw = spec.get("path") or spec.get("image") or spec.get("png")
    if not path_raw:
        raise SystemExit(f"tileset {ident}: missing path/image")
    image_path = Path(path_raw)
    if not image_path.is_absolute():
        image_path = (Path.cwd() / image_path).resolve()
    dims = png_dimensions(image_path)
    if dims is None:
        raise SystemExit(f"tileset {ident}: {image_path} is not a readable PNG")
    px_wid, px_hei = dims
    tile_w = int(spec.get("tile_width") or spec.get("grid") or spec.get("tileGridSize") or 16)
    tile_h = int(spec.get("tile_height") or tile_w)
    grid = int(spec.get("tileGridSize") or tile_w)
    if tile_w != tile_h:
        # LDtk tilesets are grid-square for Tiles layers. Rectangular entity
        # tile refs still work through tileRect, but the source tileset def has
        # one grid value.
        grid = min(tile_w, tile_h)
    return {
        "__cHei": px_hei // max(grid, 1),
        "__cWid": px_wid // max(grid, 1),
        "cachedPixelData": None,
        "customData": [],
        "embedAtlas": None,
        "enumTags": [],
        "identifier": ident,
        "padding": int(spec.get("padding", 0)),
        "pxHei": px_hei,
        "pxWid": px_wid,
        "relPath": rel_to_ldtk(ldtk, image_path),
        "savedSelections": [],
        "spacing": int(spec.get("spacing", 0)),
        "tags": list(spec.get("tags") or []),
        "tagsSourceEnumUid": None,
        "tileGridSize": grid,
        "uid": alloc_uid(project),
    }


def upsert_tilesets(project: dict, ldtk: Path, manifest: dict[str, Any]) -> list[str]:
    messages: list[str] = []
    for spec in normalize_manifest(manifest)["tilesets"]:
        if not spec:
            continue
        built = build_tileset(project, ldtk, spec)
        existing = find_tileset(project, str(built["identifier"]))
        if existing:
            uid = existing.get("uid")
            existing.clear()
            existing.update(built)
            existing["uid"] = uid
            messages.append(f"updated tileset {built['identifier']} uid={uid}")
        else:
            tileset_defs(project).append(built)
            messages.append(f"added tileset {built['identifier']} uid={built['uid']}")
    return messages


def rect_from_value(value: Any) -> tuple[int, int, int, int]:
    if isinstance(value, str):
        parts = [int(p.strip()) for p in value.split(",")]
    elif isinstance(value, (list, tuple)):
        parts = [int(v) for v in value]
    elif isinstance(value, dict):
        parts = [int(value[k]) for k in ["x", "y", "w", "h"]]
    else:
        raise SystemExit(f"tile rect must be x,y,w,h, got {value!r}")
    if len(parts) != 4:
        raise SystemExit(f"tile rect must be x,y,w,h, got {value!r}")
    return parts[0], parts[1], parts[2], parts[3]


def apply_entity_icons(project: dict, manifest: dict[str, Any]) -> list[str]:
    norm = normalize_manifest(manifest)
    messages: list[str] = []
    for entity_id, spec_raw in sorted(norm["entity_icons"].items()):
        spec = dict(spec_raw or {})
        ent = find_entity_def(project, entity_id)
        if ent is None:
            messages.append(f"skipped missing entity def {entity_id}")
            continue
        tileset_id = str(spec.get("tileset") or spec.get("source") or spec.get("tileset_identifier") or "EditorIcons")
        ts = find_tileset(project, tileset_id)
        if ts is None:
            raise SystemExit(f"entity {entity_id}: tileset {tileset_id!r} is not registered")
        tile_value = spec.get("tile") or spec.get("rect") or spec.get("tileRect")
        if tile_value is None and "index" in spec:
            index = int(spec["index"])
            grid = int(ts.get("tileGridSize") or 16)
            c_wid = max(1, int(ts.get("__cWid") or (int(ts.get("pxWid") or grid) // grid)))
            tile_value = [(index % c_wid) * grid, (index // c_wid) * grid, grid, grid]
        if tile_value is None:
            raise SystemExit(f"entity {entity_id}: missing tile/rect/index")
        x, y, w, h = rect_from_value(tile_value)
        rect = {"tilesetUid": int(ts["uid"]), "x": x, "y": y, "w": w, "h": h}
        ent["tilesetId"] = int(ts["uid"])
        ent["renderMode"] = "Tile"
        ent["tileRenderMode"] = str(spec.get("tile_render_mode") or "Cover")
        ent["tileRect"] = dict(rect)
        ent["uiTileRect"] = dict(rect)
        messages.append(f"linked {entity_id} -> {tileset_id}[{x},{y},{w},{h}]")
    return messages


def validate_manifest(project: dict, ldtk: Path, manifest: dict[str, Any]) -> list[ManifestIssue]:
    issues: list[ManifestIssue] = []
    norm = normalize_manifest(manifest)
    for spec in norm["tilesets"]:
        ident = str(spec.get("identifier") or spec.get("id") or Path(str(spec.get("path", "tileset"))).stem)
        ts = find_tileset(project, ident)
        if ts is None:
            issues.append(ManifestIssue("error", "missing_tileset", f"tileset {ident!r} is not registered"))
            continue
        rel = ts.get("relPath")
        if rel:
            img = path_from_ldtk(ldtk, str(rel))
            dims = png_dimensions(img)
            if dims is None:
                issues.append(ManifestIssue("error", "missing_tileset_png", f"tileset {ident}: relPath {rel!r} is not a readable PNG"))
            elif [ts.get("pxWid"), ts.get("pxHei")] != [dims[0], dims[1]]:
                issues.append(ManifestIssue("error", "tileset_size_mismatch", f"tileset {ident}: LDtk size {[ts.get('pxWid'), ts.get('pxHei')]} != PNG size {list(dims)}"))
    for entity_id, spec_raw in sorted(norm["entity_icons"].items()):
        spec = dict(spec_raw or {})
        ent = find_entity_def(project, entity_id)
        if ent is None:
            issues.append(ManifestIssue("error", "missing_entity_def", f"entity def {entity_id!r} is missing"))
            continue
        tileset_id = str(spec.get("tileset") or spec.get("source") or spec.get("tileset_identifier") or "EditorIcons")
        ts = find_tileset(project, tileset_id)
        if ts is None:
            issues.append(ManifestIssue("error", "entity_missing_tileset", f"{entity_id}: tileset {tileset_id!r} is not registered"))
            continue
        rect = ent.get("tileRect") or {}
        if ent.get("tilesetId") != ts.get("uid") or rect.get("tilesetUid") != ts.get("uid"):
            issues.append(ManifestIssue("error", "entity_icon_tileset_mismatch", f"{entity_id}: LDtk icon does not reference {tileset_id}"))
            continue
        if rect:
            x, y, w, h = int(rect.get("x", 0)), int(rect.get("y", 0)), int(rect.get("w", 0)), int(rect.get("h", 0))
            if x < 0 or y < 0 or w <= 0 or h <= 0 or x + w > int(ts.get("pxWid") or 0) or y + h > int(ts.get("pxHei") or 0):
                issues.append(ManifestIssue("error", "entity_icon_rect_oob", f"{entity_id}: tileRect {rect} is outside tileset {tileset_id}"))
        else:
            issues.append(ManifestIssue("error", "entity_missing_tile_rect", f"{entity_id}: missing tileRect"))
    return issues


def collect_visual_ref_issues(project: dict) -> list[ManifestIssue]:
    issues: list[ManifestIssue] = []
    tiles_by_uid = {int(ts.get("uid")): ts for ts in tileset_defs(project) if ts.get("uid") is not None}
    for ent in entity_defs(project):
        rect = ent.get("tileRect")
        tileset_id = ent.get("tilesetId")
        if rect is None and tileset_id is None:
            continue
        ident = ent.get("identifier")
        uid = rect.get("tilesetUid") if isinstance(rect, dict) else tileset_id
        try:
            uid = int(uid)
        except Exception:
            issues.append(ManifestIssue("error", "bad_entity_tileset_uid", f"{ident}: invalid tileset uid {uid!r}"))
            continue
        ts = tiles_by_uid.get(uid)
        if ts is None:
            issues.append(ManifestIssue("error", "stale_entity_tileset_uid", f"{ident}: references missing tileset uid {uid}"))
            continue
        if not isinstance(rect, dict):
            issues.append(ManifestIssue("error", "missing_entity_tile_rect", f"{ident}: has tilesetId but no tileRect"))
            continue
        x, y, w, h = int(rect.get("x", 0)), int(rect.get("y", 0)), int(rect.get("w", 0)), int(rect.get("h", 0))
        if x < 0 or y < 0 or w <= 0 or h <= 0 or x + w > int(ts.get("pxWid") or 0) or y + h > int(ts.get("pxHei") or 0):
            issues.append(ManifestIssue("error", "entity_tile_rect_oob", f"{ident}: tileRect {rect} is outside {ts.get('identifier')}"))
    return issues


def apply_manifest(project: dict, ldtk: Path, manifest: dict[str, Any]) -> list[str]:
    messages = upsert_tilesets(project, ldtk, manifest)
    messages.extend(apply_entity_icons(project, manifest))
    return messages


def default_icon_manifest(ldtk: Path, icon_path: Path, tile_size: int, entities: list[str] | None = None) -> dict[str, Any]:
    ent_order = entities or DEFAULT_ENTITY_ICON_ORDER
    manifest: dict[str, Any] = {
        "editor_icons": {
            "identifier": "EditorIcons",
            "path": str(icon_path),
            "tile_width": tile_size,
            "tile_height": tile_size,
            "padding": 0,
            "spacing": 0,
        },
        "entity_icons": {},
    }
    for index, ent in enumerate(ent_order):
        manifest["entity_icons"][ent] = {"tileset": "EditorIcons", "index": index}
    return manifest


def _set_px(buf: bytearray, width: int, x: int, y: int, color: tuple[int, int, int, int]) -> None:
    if x < 0 or y < 0 or x >= width:
        return
    i = (y * width + x) * 4
    if i < 0 or i + 3 >= len(buf):
        return
    buf[i:i+4] = bytes(color)


def _fill_rect(buf: bytearray, width: int, height: int, x0: int, y0: int, x1: int, y1: int, color: tuple[int, int, int, int]) -> None:
    for y in range(max(0, y0), min(height, y1)):
        for x in range(max(0, x0), min(width, x1)):
            _set_px(buf, width, x, y, color)


def _draw_icon(buf: bytearray, width: int, height: int, tile: int, tile_size: int, ident: str) -> None:
    cols = max(1, width // tile_size)
    tx = (tile % cols) * tile_size
    ty = (tile // cols) * tile_size
    base = DEFAULT_ICON_COLORS.get(ident, (229, 231, 235))
    bg = (17, 24, 39, 255)
    fg = (*base, 255)
    soft = (*base, 88)
    white = (255, 255, 255, 255)
    _fill_rect(buf, width, height, tx, ty, tx + tile_size, ty + tile_size, bg)
    _fill_rect(buf, width, height, tx + 2, ty + 2, tx + tile_size - 2, ty + tile_size - 2, soft)
    # A few recognisable gizmo shapes; intentionally simple and stable.
    cx = tx + tile_size // 2
    cy = ty + tile_size // 2
    if ident == "CameraZone":
        _fill_rect(buf, width, height, tx + 7, ty + 9, tx + tile_size - 7, ty + tile_size - 8, fg)
        _fill_rect(buf, width, height, tx + tile_size - 9, ty + 13, tx + tile_size - 4, ty + tile_size - 12, fg)
        _fill_rect(buf, width, height, cx - 3, cy - 3, cx + 4, cy + 4, bg)
    elif ident == "LoadingZone":
        _fill_rect(buf, width, height, tx + 8, ty + 6, tx + tile_size - 8, ty + tile_size - 5, fg)
        _fill_rect(buf, width, height, tx + 12, ty + 10, tx + tile_size - 10, ty + tile_size - 9, bg)
        _fill_rect(buf, width, height, tx + tile_size - 11, cy - 2, tx + tile_size - 7, cy + 2, white)
    elif ident == "GravityZone":
        for i in range(8):
            _fill_rect(buf, width, height, cx - 2, ty + 6 + i, cx + 3, ty + 18 + i, fg)
        _fill_rect(buf, width, height, cx - 7, ty + 18, cx + 8, ty + 23, fg)
    elif ident == "PlayerStart":
        _fill_rect(buf, width, height, cx - 4, ty + 7, cx + 5, ty + 16, fg)
        _fill_rect(buf, width, height, cx - 6, ty + 16, cx + 7, ty + 27, fg)
    elif ident in {"EnemySpawn", "DamageVolume"}:
        _fill_rect(buf, width, height, cx - 10, cy + 6, cx + 11, cy + 10, fg)
        _fill_rect(buf, width, height, cx - 2, cy - 10, cx + 3, cy + 7, fg)
        _fill_rect(buf, width, height, cx - 8, cy - 4, cx + 9, cy + 1, fg)
    elif ident in {"NpcSpawn", "PickupSpawn"}:
        _fill_rect(buf, width, height, cx - 6, cy - 6, cx + 7, cy + 7, fg)
    else:
        _fill_rect(buf, width, height, tx + 8, ty + 8, tx + tile_size - 8, ty + tile_size - 8, fg)


def write_png(path: Path, width: int, height: int, rgba: bytes | bytearray) -> None:
    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    raw = bytearray()
    stride = width * 4
    for y in range(height):
        raw.append(0)
        raw.extend(rgba[y * stride : (y + 1) * stride])
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )


def generate_editor_icons(path: Path, *, tile_size: int = 32, entities: list[str] | None = None, columns: int = 8) -> dict[str, Any]:
    ent_order = entities or DEFAULT_ENTITY_ICON_ORDER
    cols = max(1, columns)
    rows = (len(ent_order) + cols - 1) // cols
    width = cols * tile_size
    height = rows * tile_size
    buf = bytearray([0, 0, 0, 0] * width * height)
    for idx, ident in enumerate(ent_order):
        _draw_icon(buf, width, height, idx, tile_size, ident)
    write_png(path, width, height, buf)
    return {"path": str(path), "tile_size": tile_size, "entities": ent_order, "size": [width, height]}


def preview_manifest_html(ldtk: Path, manifest: dict[str, Any]) -> str:
    norm = normalize_manifest(manifest)
    rows: list[str] = ["<html><body><h1>LDtk visual manifest preview</h1>"]
    rows.append("<h2>Tilesets</h2><ul>")
    for ts in norm["tilesets"]:
        ident = escape(str(ts.get("identifier") or ts.get("id") or "tileset"))
        path = escape(str(ts.get("path") or ts.get("image") or ""))
        rows.append(f"<li><b>{ident}</b>: {path}</li>")
    rows.append("</ul><h2>Entity icons</h2><table border='1' cellspacing='0' cellpadding='4'>")
    rows.append("<tr><th>Entity</th><th>Tileset</th><th>Tile</th></tr>")
    for ent, spec_raw in sorted(norm["entity_icons"].items()):
        spec = dict(spec_raw or {})
        rows.append(
            "<tr>"
            f"<td>{escape(str(ent))}</td>"
            f"<td>{escape(str(spec.get('tileset') or spec.get('source') or 'EditorIcons'))}</td>"
            f"<td>{escape(str(spec.get('tile') or spec.get('rect') or spec.get('index')))}</td>"
            "</tr>"
        )
    rows.append("</table></body></html>")
    return "\n".join(rows) + "\n"


def format_issues(issues: list[ManifestIssue]) -> str:
    if not issues:
        return "visual manifest validation passed.\n"
    lines = ["Visual manifest issues:"]
    for issue in issues:
        lines.append(f"  {issue.severity}: {issue.code}: {issue.message}")
    return "\n".join(lines) + "\n"


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="LDtk visual manifest scaffolding and application.")
    ap.add_argument("action", choices=["generate-editor-icons", "suggest-manifest", "apply-manifest", "validate-manifest", "preview-manifest"])
    ap.add_argument("ldtk", type=Path, nargs="?")
    ap.add_argument("manifest", type=Path, nargs="?")
    ap.add_argument("--out", type=Path)
    ap.add_argument("--icons", type=Path, help="Output/input editor icon PNG path")
    ap.add_argument("--tile-size", type=int, default=32)
    ap.add_argument("--entity", action="append", default=[], help="Entity identifier to include in generated icon manifest; repeatable")
    ap.add_argument("--format", choices=["text", "json"], default="text")
    ap.add_argument("--in-place", action="store_true")
    ap.add_argument("--output", type=Path)
    args = ap.parse_args(argv)

    if args.action == "generate-editor-icons":
        if not args.icons and not args.out:
            raise SystemExit("generate-editor-icons requires --icons or --out")
        path = args.icons or args.out
        info = generate_editor_icons(path, tile_size=args.tile_size, entities=args.entity or None)
        print(json.dumps(info, indent=2, sort_keys=True) if args.format == "json" else f"wrote editor icons {path} size={info['size']}")
        return 0

    if args.action == "suggest-manifest":
        if not args.ldtk:
            raise SystemExit("suggest-manifest requires <ldtk>")
        repo = repo_root_from_ldtk(args.ldtk)
        icon_path = args.icons or (repo / "crates" / "ambition_gameplay_core" / "assets" / "sprites" / "editor_icons.png")
        data = default_icon_manifest(args.ldtk, icon_path, args.tile_size, args.entity or None)
        if args.out:
            save_manifest(args.out, data)
            print(f"wrote {args.out}")
        else:
            print(json.dumps(data, indent=2, sort_keys=True))
        return 0

    if args.action in {"apply-manifest", "validate-manifest"}:
        if not args.ldtk or not args.manifest:
            raise SystemExit(f"{args.action} requires <ldtk> <manifest>")
        project = load_project(args.ldtk)
        manifest = load_manifest(args.manifest)
        if args.action == "validate-manifest":
            issues = validate_manifest(project, args.ldtk, manifest)
            print(json.dumps([i.__dict__ for i in issues], indent=2, sort_keys=True) if args.format == "json" else format_issues(issues), end="" if args.format == "text" else "\n")
            return 1 if any(i.severity == "error" for i in issues) else 0
        if not args.in_place and not args.output:
            raise SystemExit("apply-manifest requires --in-place or --output")
        tx = LdtkTransaction(args.ldtk, in_place=args.in_place, output=args.output)
        msgs = apply_manifest(tx.project, args.ldtk, manifest)
        if msgs:
            tx.note_changed(msgs)
        out = tx.write_if_changed()
        for msg in msgs:
            print(msg)
        print(f"wrote {out}")
        return 0

    if args.action == "preview-manifest":
        if not args.manifest:
            raise SystemExit("preview-manifest requires <manifest>")
        data = load_manifest(args.manifest)
        html = preview_manifest_html(args.ldtk or Path.cwd(), data)
        if args.out:
            args.out.parent.mkdir(parents=True, exist_ok=True)
            args.out.write_text(html)
            print(f"wrote {args.out}")
        else:
            print(html, end="")
        return 0
    return 64


if __name__ == "__main__":
    raise SystemExit(main())
