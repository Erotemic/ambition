"""The published sidecars of a sprite sheet, read for the inspector: frame
layout (`_spritesheet.yaml`), portrait clips (`_portraits.ron`), part tracks
(`_parts.ron`), body joints (`_body_rig.ron`) and identity (`_actor.ron`).

`_spritesheet.yaml` has three shapes in the published tree: the current one
names `image`; an older generator sidecar names `images` (and the generator's
`spec`); the oldest names only `pages`. All three give the same rows."""

from __future__ import annotations

import json
import os
import re
from copy import deepcopy
from pathlib import Path
from typing import Any

import yaml

from .paths import real

try:
    _Loader = yaml.CSafeLoader
except AttributeError:  # pragma: no cover - PyYAML without libyaml
    _Loader = yaml.SafeLoader

#: The suffix of each published file of a sheet, and its role.
SHEET_SUFFIXES = (
    ("_spritesheet.yaml", "sheet_layout"),
    ("_spritesheet.ron", "sheet_record"),
    ("_actor.ron", "actor"),
    ("_portraits.png", "portraits"),
    ("_portraits.ron", "portrait_clips"),
    ("_parts.png", "parts_atlas"),
    ("_parts.ron", "parts_table"),
    ("_body_rig.ron", "body_rig"),
    ("_authoring.yaml", "authoring"),
)
_PAGE = re.compile(r"^(?P<stem>.+)_spritesheet(?:\.(?P<page>\d+))?\.png$")


def sheet_role(name: str) -> tuple[str, str] | None:
    """The sheet key and role of a published file name, or None."""
    m = _PAGE.match(name)
    if m:
        return m.group("stem"), f"sheet_page:{int(m.group('page') or 0)}"
    for suffix, role in SHEET_SUFFIXES:
        if name.endswith(suffix):
            return name[: -len(suffix)], role
    return None


def page_files(layout: dict[str, Any], key: str) -> list[str]:
    """The page image names of a sheet, in page order."""
    if layout.get("images"):
        return list(layout["images"])
    first = layout.get("image") or f"{key}_spritesheet.png"
    pages = int(layout.get("pages") or 1)
    base = first[: -len(".png")]
    return [first] + [f"{base}.{n}.png" for n in range(1, pages)]


class SidecarCache:
    """Readings of sidecar files, kept by (path, size, mtime) between builds:
    parsing the large sheet YAML is most of a rebuild otherwise. A reader's
    result must be plain JSON data. With no `path`, nothing is kept."""

    VERSION = 4  # 4: score summaries carry their form (`kinds._score_summary`)

    def __init__(self, path: Path | None) -> None:
        self.path = path
        self.data: dict[str, list] = {}
        self.used: dict[str, list] = {}
        try:
            saved = json.loads(path.read_text()) if path else {}
            if saved.get("version") == self.VERSION:
                self.data = saved["entries"]
        except (OSError, ValueError, AttributeError, KeyError):
            pass

    def read(self, reader, path: Path):
        st = path.stat()
        key = f"{reader.__name__}:{path}"
        stamp = [st.st_size, st.st_mtime_ns]
        hit = self.data.get(key)
        if hit is None or hit[0] != stamp:
            hit = [stamp, json.dumps(reader(path))]
        self.used[key] = hit
        # A fresh copy each time: the build annotates what it is given.
        return json.loads(hit[1])

    def save(self) -> None:
        if not self.path:
            return
        self.path.parent.mkdir(parents=True, exist_ok=True)
        partial = self.path.with_suffix(f".{os.getpid()}.partial")
        partial.write_text(json.dumps({"version": self.VERSION, "entries": self.used}))
        partial.replace(self.path)


def read_layout(path: Path) -> dict[str, Any]:
    """The frame layout of a sheet: frame size, pages and each animation
    with its frames (rect, page, trim offset)."""
    with open(path) as f:
        data = yaml.load(f, Loader=_Loader) or {}
    key = path.name[: -len("_spritesheet.yaml")]
    rows = []
    for row in data.get("rows") or []:
        rects = []
        for r in row.get("rects") or []:
            rects.append([r["x"], r["y"], r["w"], r["h"], r.get("fpage", row.get("page", 0)), *(r.get("off") or (0, 0))])
        rows.append({
            "name": row["animation"],
            "frames": len(rects) or int(row.get("frame_count", 0)),
            "ms": row.get("duration_ms"),
            "mirror_of": row.get("mirror_of"),
            "rects": rects,
        })
    body = data.get("body_metrics") or {}
    return {
        "key": key,
        "frame": [data.get("frame_width"), data.get("frame_height")],
        "pages": page_files(data, key),
        "rows": rows,
        "feet": (body.get("feet_pixel") or None),
        "body_bbox": body.get("body_pixel_bbox"),
        "description": data.get("authoring_description"),
        "schema": "image" if data.get("image") else "images" if data.get("images") else "pages",
        "generator_spec": {k: data.get(k) for k in ("seed", "archetype", "variant", "held_item") if data.get(k) is not None},
    }


def use_runtime_layouts(assets, products, inspection: dict) -> None:
    """Use the runtime owner's frame geometry for each published manifest."""
    layouts = {}
    for unit in inspection["units"].values():
        for sheet in unit.get("published", []):
            layout = sheet.pop("layout", None)
            if sheet["tier"] == "full" and layout is not None:
                layouts.setdefault(real(sheet["path"]), {})[sheet["key"]] = layout
    for asset in assets.values():
        records = [products[key] for key in asset.products
                   if products[key].role == "sheet_record" and products[key].root.of is None]
        matches = [layout for product in records for layout in layouts.get(real(product.file), {}).values()]
        if len(matches) == 1:
            authored = asset.data.get("layout") or {}
            asset.data["layout"] = {key: authored[key] for key in ("description", "generator_spec") if key in authored} | deepcopy(matches[0])


def use_catalog_poses(assets, inspection: dict) -> None:
    """Include each identity's declared rest pose in the sheet camera."""
    poses = {}
    for identity in inspection["identities"].values():
        catalog = identity["catalog"]
        if catalog.get("rest_row"):
            poses.setdefault(catalog["manifest"], set()).add(catalog["rest_row"])
    for asset in assets.values():
        if asset.data.get("layout"):
            asset.data["layout"]["cover_rows"] = sorted({pose for product in asset.products for pose in poses.get(product, [])})


_STR = r'"((?:[^"\\]|\\.)*)"'


def _ron_field(text: str, name: str) -> str | None:
    m = re.search(rf"\b{name}: (?:Some\()?{_STR}", text)
    return m.group(1) if m else None


def read_actor(path: Path) -> dict[str, Any]:
    """The identity fields of an `_actor.ron`."""
    text = path.read_text()
    out: dict[str, Any] = {
        "character_id": _ron_field(text, "character_id"),
        "display_name": _ron_field(text, "display_name"),
        "renderer_target": _ron_field(text, "renderer_target"),
        "surface": _ron_field(text, "surface"),
        "description": _ron_field(text, "authoring_description"),
    }
    m = re.search(r"lineage: Some\(\((.*?)\)\)", text, re.S)
    if m:
        out["lineage"] = {k: _ron_field(m.group(1), k) for k in ("family", "variant", "creator") if _ron_field(m.group(1), k)}
    m = re.search(r"\btags: \[(.*?)\]", text, re.S)
    if m:
        out["tags"] = re.findall(_STR, m.group(1))
    return {k: v for k, v in out.items() if v}


def read_portraits(path: Path) -> dict[str, Any]:
    """Each portrait clip (an expression, a talking loop) with its frame rects."""
    text = path.read_text()
    clips = {}
    for name, body in re.findall(r'"([A-Za-z0-9_]+)": \((.*?)\]\s*,\s*\)', text, re.S):
        rects = [list(map(int, r)) for r in re.findall(r"x: (\d+), y: (\d+), w: (\d+), h: (\d+)", body)]
        if rects:
            ms = re.search(r"duration_ms: (\d+)", body)
            clips[name] = {"rects": rects, "ms": int(ms.group(1)) if ms else 0}
    return {"clips": clips, "default": _ron_field(text, "default_clip") or "default"}


def read_parts(path: Path) -> dict[str, Any]:
    """The part atlas: how many parts, and the named tracks the clips draw."""
    text = path.read_text()
    m = re.search(r"\btracks: \[(.*?)\]", text, re.S)
    return {
        "parts": len(re.findall(r'\(name: "', text)),
        "tracks": re.findall(_STR, m.group(1)) if m else [],
        "clips": len(re.findall(r'^\s*"[^"]+": \(\s*frame_duration_s', text, re.M)),
    }


_NUM = r"-?[0-9.]+(?:e-?[0-9]+)?"
_PART = re.compile(rf"\(name: {_STR}, page: (\d+), rect: \((\d+), (\d+), (\d+), (\d+)\), pivot: \(({_NUM}), ({_NUM})\)\)")
_CLIP_OR_FRAME_OR_DRAW = re.compile(
    rf'"(?P<clip>[^"]+)": \((?P<head>.*?)frames: \['
    rf"|(?P<open>\[)"
    rf"|\(part: (?P<part>\d+), at: \((?P<ax>{_NUM}), (?P<ay>{_NUM})\), rotation: (?P<rot>{_NUM}), "
    rf"scale: \((?P<sx>{_NUM}), (?P<sy>{_NUM})\)(?:, track: \d+)?(?:, opacity: (?P<op>{_NUM}))?"
    rf"(?:, tint: \((?P<tr>{_NUM}), (?P<tg>{_NUM}), (?P<tb>{_NUM})\))?\)",
    re.S,
)


def read_part_draws(path: Path) -> dict[str, Any]:
    """A part flipbook (`_parts.ron`) as the inspector page draws it: each
    part's atlas rect and pivot, and each clip's frames of ordered draws.

    A part is ``[page, x, y, w, h, pivot_x, pivot_y]`` in atlas texels; a draw
    is ``[part, at_x, at_y, rotation, scale_x, scale_y, opacity, tint]`` with
    the trailing defaults left out: ``at`` in published pixels from the feet,
    the rotation in radians clockwise (+y down). A tier copy stores its parts
    at ``texel_scale`` texels per published pixel."""
    text = path.read_text()

    def pair(name: str) -> list[float]:
        m = re.search(rf"\b{name}: \(({_NUM}), ({_NUM})\)", text)
        return [float(m.group(1)), float(m.group(2))] if m else [0.0, 0.0]

    m = re.search(rf"\btexel_scale: ({_NUM})", text)
    pages = re.search(r"\bpages: \[(.*?)\]", text, re.S)
    parts = [[int(pg), int(x), int(y), int(w), int(h), float(px), float(py)]
             for _name, pg, x, y, w, h, px, py in _PART.findall(text)]
    start = text.find("clips: {")
    end = text.find("baked_clips:", start)
    clips: dict[str, Any] = {}
    frames: list | None = None
    for hit in _CLIP_OR_FRAME_OR_DRAW.finditer(text, start, end if end >= 0 else len(text)):
        if hit.group("clip"):
            head = hit.group("head")
            duration = re.search(rf"frame_duration_s: ({_NUM})", head)
            fade = re.search(r"frame_opacity: \[(.*?)\]", head, re.S)
            clip: dict[str, Any] = {"s": float(duration.group(1)) if duration else 0.1, "frames": []}
            if "tween: Linear" in head:
                clip["tween"] = "linear"
            if fade:
                clip["opacity"] = [float(v) for v in re.findall(_NUM, fade.group(1))]
            clips[hit.group("clip")] = clip
            frames = clip["frames"]
        elif hit.group("open") is not None:
            if frames is not None:
                frames.append([])
        elif frames:
            draw = [int(hit.group("part")), *(float(hit.group(k)) for k in ("ax", "ay", "rot", "sx", "sy"))]
            opacity = float(hit.group("op")) if hit.group("op") else 1.0
            tint = [float(hit.group(k)) for k in ("tr", "tg", "tb")] if hit.group("tr") else None
            draw += [opacity, tint]
            # Trailing defaults are left out: rotation 0, scale 1, opacity 1, no tint.
            defaults = [0.0, 1.0, 1.0, 1.0, None]
            while len(draw) > 3 and draw[-1] == defaults[len(draw) - 4]:
                draw.pop()
            frames[-1].append(draw)
    return {
        "frame": [int(v) for v in pair("frame_size")],
        "feet": pair("feet_pixel"),
        "texel_scale": float(m.group(1)) if m else 1.0,
        "pages": re.findall(_STR, pages.group(1)) if pages else [],
        "parts": parts,
        "clips": clips,
    }


def read_body_rig(path: Path) -> dict[str, Any]:
    """The joints of a published body rig, each with its parent."""
    text = path.read_text()
    joints = [[name, parent or None] for name, parent in re.findall(r'\(name: "([^"]+)", parent: (?:Some\("([^"]+)"\)|None)\)', text)]
    return {"joints": joints}
