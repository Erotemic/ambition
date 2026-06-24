"""The single RON manifest emitter (stdlib only — no YAML in the write path).

Consolidates the two byte-identical-but-duplicated RON writers that lived in
``sheet.py`` (adapter spine) and ``tackon_sheet.py`` (tack-on spine). They
produced the same `Vec<SheetRecord>` RON shape the game reads
(`crates/ambition_sprite_sheet`); the only differences were data-conditional
(per-animation hit/hurt boxes present for adapters, `tuning` absent everywhere
today) and the row *input shape* (adapters carry an ``animations`` dict;
tack-ons carry normalized ``rows``). This emitter handles both, so its output is
byte-for-byte identical to both old writers (verified by the parity harness).

Entry points:
  * ``record_to_ron(record)``      — one ``SheetRecord`` block (rows already
                                     normalized). Used directly by the packed
                                     lab-props sheet, which writes its own header.
  * ``records_to_ron(target, recs)`` — header + ``[ … ]`` list (the common case).
  * ``render_adapter(manifest)``   — normalize an adapter manifest's
                                     ``animations`` dict into rows, then emit.
  * ``ron_tuning(manifest_or_tuning)`` — the optional ``tuning:`` field.
"""
from __future__ import annotations

from typing import Dict, List


def _ron_escape(s) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def _ron_some(inner: str) -> str:
    return f"Some({inner})"


def _ron_optional_rect(v) -> str:
    if not isinstance(v, dict):
        return "None"
    return _ron_some(
        f"(x: {int(v['x'])}, y: {int(v['y'])}, w: {int(v['w'])}, h: {int(v['h'])})"
    )


def _ron_optional_point(v) -> str:
    if not isinstance(v, dict):
        return "None"
    return _ron_some(f"(x: {float(v['x'])}, y: {float(v['y'])})")


def _ron_animation_box(box) -> str:
    """Serialize one animation's hit-or-hurt box (parts + bbox + poly)."""
    inner = []
    parts = box.get("parts")
    if isinstance(parts, list) and parts:
        formatted = ", ".join(
            f'(name: "{_ron_escape(str(p.get("name", "")))}", '
            f"x: {int(p['x'])}, y: {int(p['y'])}, w: {int(p['w'])}, h: {int(p['h'])})"
            for p in parts
            if isinstance(p, dict)
        )
        inner.append(f"parts: [{formatted}]")
    bbox = box.get("bbox")
    if isinstance(bbox, dict):
        inner.append(f"bbox: {_ron_optional_rect(bbox)}")
    # Optional convex polygon (sprite-frame pixel points) — an effect-conforming
    # hitbox shape (blade arc, cone) the runtime reads as a CombatVolume::Convex.
    poly = box.get("poly")
    if isinstance(poly, list) and poly:
        pts = ", ".join(f"({float(x)}, {float(y)})" for x, y in poly)
        inner.append(f"poly: [{pts}]")
    return "(" + ", ".join(inner) + ")"


def _ron_anim_metrics_map(metrics) -> str:
    """RON-serialize per-animation hit + hurt box metadata."""
    if not isinstance(metrics, dict) or not metrics:
        return "{}"
    items = []
    for anim_name, entry in sorted(metrics.items()):
        if not isinstance(entry, dict):
            continue
        inner = []
        for kind in ("hurtbox", "hitbox"):
            box = entry.get(kind)
            if not isinstance(box, dict):
                continue
            inner.append(f"{kind}: Some({_ron_animation_box(box)})")
        items.append(f'"{_ron_escape(anim_name)}": ({", ".join(inner)})')
    return "{" + ", ".join(items) + "}"


def _ron_body_metrics(bm) -> str:
    if not isinstance(bm, dict):
        return "None"
    parts = [
        f"body_pixel_bbox: {_ron_optional_rect(bm.get('body_pixel_bbox'))}",
        f"feet_pixel: {_ron_optional_point(bm.get('feet_pixel'))}",
        f"feet_anchor_norm: {_ron_optional_point(bm.get('feet_anchor_norm'))}",
    ]
    anim_metrics = bm.get("animations")
    if isinstance(anim_metrics, dict) and anim_metrics:
        parts.append(f"animations: {_ron_anim_metrics_map(anim_metrics)}")
    return _ron_some(f"({', '.join(parts)})")


# Full-manifest keys: a dict carrying any of these is a manifest, not a bare
# tuning dict, so absent tuning means "omit the field". Union of both spines'
# sets (tack-on lacked "crop"; harmless to include).
_FULL_MANIFEST_KEYS = {
    "target",
    "image",
    "animations",
    "rows",
    "frame_width",
    "frame_height",
    "label_width",
    "body_metrics",
    "crop",
}


def ron_tuning(manifest_or_tuning) -> str:
    """Serialize the optional ``tuning:`` field. Accepts a full manifest (reads
    its ``sheet_tuning`` / ``tuning``) or a bare tuning dict. Missing tuning →
    empty string so emitters omit the field; an explicit ``{}`` emits defaults."""
    if not isinstance(manifest_or_tuning, dict):
        return ""
    if "sheet_tuning" in manifest_or_tuning:
        tuning = manifest_or_tuning.get("sheet_tuning")
    elif "tuning" in manifest_or_tuning:
        tuning = manifest_or_tuning.get("tuning")
    elif any(key in manifest_or_tuning for key in _FULL_MANIFEST_KEYS):
        return ""
    else:
        tuning = manifest_or_tuning
    if tuning is None or not isinstance(tuning, dict):
        return ""
    collision_scale = float(tuning.get("collision_scale", 1.0))
    frame_sample_inset = int(tuning.get("frame_sample_inset", 0))
    feet_anchor_y = tuning.get("feet_anchor_y_override", tuning.get("feet_anchor_y"))
    fields = [f"collision_scale: {collision_scale}"]
    if feet_anchor_y is not None:
        fields.append(f"feet_anchor_y_override: Some({float(feet_anchor_y)})")
    fields.append(f"frame_sample_inset: {frame_sample_inset}")
    inner = "\n".join(f"        {field}," for field in fields)
    return f"    tuning: Some((\n{inner}\n    )),\n"


def _ron_anchors(anchors) -> str:
    if not isinstance(anchors, dict) or not anchors:
        return "{}"
    items = []
    for name, pos in sorted(anchors.items()):
        if not isinstance(pos, dict) or "x" not in pos or "y" not in pos:
            continue
        items.append(
            f'"{_ron_escape(str(name))}": (x: {float(pos["x"])}, y: {float(pos["y"])})'
        )
    return "{" + ", ".join(items) + "}" if items else "{}"


def _ron_rect(r) -> str:
    base = f"x: {int(r['x'])}, y: {int(r['y'])}, w: {int(r['w'])}, h: {int(r['h'])}"
    anchors = r.get("anchors")
    if anchors:
        return f"({base}, anchors: {_ron_anchors(anchors)})"
    return f"({base})"


def _ron_row(row) -> str:
    rects = ",\n            ".join(_ron_rect(r) for r in row.get("rects", []))
    return (
        f"(\n"
        f'        animation: "{_ron_escape(row["animation"])}",\n'
        f"        row_index: {int(row['row_index'])},\n"
        f"        frame_count: {int(row['frame_count'])},\n"
        f"        duration_ms: {int(row['duration_ms'])},\n"
        f"        duration_secs: {float(row['duration_secs'])},\n"
        f"        rects: [\n            {rects},\n        ],\n"
        f"    )"
    )


def record_to_ron(record: Dict) -> str:
    """Render one ``SheetRecord`` (rows already normalized). Caller wraps in
    ``[...]``. Multi-record callers (packed sheets) join several with ``,\\n``."""
    target = record["target"]
    row_entries = list(record.get("rows", []))
    if row_entries:
        rows_inner = "\n    ".join(_ron_row(r) + "," for r in row_entries)
        rows_field = f"    rows: [\n    {rows_inner}\n    ],\n"
    else:
        rows_field = "    rows: [],\n"
    y_offset = int(record.get("y_offset", 0))
    y_offset_field = f"    y_offset: {y_offset},\n" if y_offset else ""
    tuning_field = ron_tuning(record)
    return (
        f"(\n"
        f'    target: "{_ron_escape(target)}",\n'
        f'    image: "{_ron_escape(record.get("image") or f"{target}_spritesheet.png")}",\n'
        f"    label_width: {int(record.get('label_width', 0))},\n"
        f"    frame_width: {int(record['frame_width'])},\n"
        f"    frame_height: {int(record['frame_height'])},\n"
        f"{y_offset_field}"
        f"{tuning_field}"
        f"    body_metrics: {_ron_body_metrics(record.get('body_metrics'))},\n"
        f"{rows_field}"
        f")"
    )


def records_to_ron(target: str, records: List[Dict]) -> str:
    """Header + ``[ record, … ]`` — the universal `Vec<SheetRecord>` shape."""
    body = "".join(f"{record_to_ron(r)},\n" for r in records)
    return (
        f"// Auto-emitted from {target}_spritesheet.yaml — see\n"
        f"// `presentation::character_sprites::registry`.\n"
        f"[\n"
        f"{body}"
        f"]\n"
    )


def _normalize_adapter_rows(animations) -> List[Dict]:
    """Turn an adapter manifest's ``{name: {frames, duration_ms}}`` into the
    normalized row dicts ``record_to_ron`` expects."""
    rows = []
    items = animations.items() if isinstance(animations, dict) else []
    for row_index, (name, info) in enumerate(items):
        frames = info.get("frames", []) if isinstance(info, dict) else []
        duration_ms = int(info.get("duration_ms", 0)) if isinstance(info, dict) else 0
        rects = [
            fr
            for fr in frames
            if isinstance(fr, dict) and all(k in fr for k in ("x", "y", "w", "h"))
        ]
        rows.append(
            {
                "animation": name,
                "row_index": row_index,
                "frame_count": len(rects),
                "duration_ms": duration_ms,
                "duration_secs": round(duration_ms / 1000.0, 6),
                "rects": rects,
            }
        )
    return rows


def render_adapter(manifest: Dict) -> str:
    """Emit the RON for an adapter-pipeline manifest (animations dict input)."""
    record = dict(manifest)
    record["rows"] = _normalize_adapter_rows(manifest.get("animations") or {})
    return records_to_ron(manifest["target"], [record])
