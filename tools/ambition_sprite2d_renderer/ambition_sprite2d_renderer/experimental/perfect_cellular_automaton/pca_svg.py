#!/usr/bin/env python3
"""Round-trip the paper-doll polygons <-> SVG so a human can fix parts in
Inkscape, then re-import and keep refining.

Each polygon becomes an SVG <polygon> that carries:
  * its FILL colour  (the palette colour it renders)
  * its PART name    (inkscape:label -- editable in Object Properties, Ctrl+Shift+O)
Polygons are organised into LAYERS by high-level region (head/torso/arm/leg/tail/
accent) for easy show/hide. The reference image is embedded as a locked, faint
backdrop layer to trace against. The palette is stored in <metadata> so import
maps any colour the human paints back to the nearest palette index.

Human workflow:
  1.  export  ->  agent-scratch/svg/<pose>.svg
  2.  open in Inkscape; move/reshape nodes (N), delete wrong polys, recolour, and
      RELABEL via Object Properties > Label = the part name. Save (Ctrl+S).
  3.  import  ->  writes <pose>_polys.json into a version dir; we render + eval it.

Import bakes any transform the editor added (translate/scale/matrix, incl. on the
parent layers), so moving whole objects with the selection tool is fine.

    PY=.venv/bin/python; EXP=.../perfect_cellular_automaton
    $PY $EXP/pca_svg.py export --version 13_airland            # all poses
    $PY $EXP/pca_svg.py import --pose top_side --version 14_handedit
"""
from __future__ import annotations

import argparse
import base64
import json
import re
import xml.etree.ElementTree as ET
from pathlib import Path

import numpy as np

import pca_paths as P

SVG_DIR = P.SCRATCH / "svg"
NS = {"svg": "http://www.w3.org/2000/svg",
      "inkscape": "http://www.inkscape.org/namespaces/inkscape",
      "sodipodi": "http://sodipodi.sourceforge.net/DTD/sodipodi-0.0.dtd",
      "xlink": "http://www.w3.org/1999/xlink"}

GROUP_OF = {
    "helmet": "head", "horn": "head", "face": "head", "eye": "head", "forehead_cell": "head",
    "neck": "torso", "core": "torso", "core_fill": "torso", "bodysuit": "torso",
    "chest_plate": "torso", "back_plate": "torso", "pec": "torso", "belly_panel": "torso",
    "belly_cell": "torso", "tail": "tail",
    "shoulder": "arm", "shoulder_spot": "arm", "upper_arm": "arm", "forearm": "arm", "hand": "arm",
    "thigh": "leg", "shin": "leg", "knee": "leg", "foot": "leg",
}
LAYERS = ["leg", "tail", "torso", "arm", "head", "accent"]   # draw order (back->front)
ACCENT = {"belly_cell", "forehead_cell", "shoulder_spot", "eye"}


def _hex(rgb):
    return "#%02x%02x%02x" % (int(rgb[0]), int(rgb[1]), int(rgb[2]))


def _layer_of(part):
    base = part[:-6] if part.endswith("_shade") else part
    if base in ACCENT:
        return "accent"
    return GROUP_OF.get(base, "torso")


def export(pose: str, version: str):
    d = json.loads((P.VERSIONS / version / f"{pose}_polys.json").read_text())
    w, h, pal, polys = d["w"], d["h"], np.array(d["palette"]), d["polys"]
    ref = P.REFS / f"{pose}.png"
    SVG_DIR.mkdir(parents=True, exist_ok=True)

    L = ['<?xml version="1.0" encoding="UTF-8"?>',
         f'<svg xmlns="{NS["svg"]}" xmlns:inkscape="{NS["inkscape"]}" '
         f'xmlns:sodipodi="{NS["sodipodi"]}" xmlns:xlink="{NS["xlink"]}" '
         f'width="{w}" height="{h}" viewBox="0 0 {w} {h}">',
         f'<metadata id="pca-palette">{json.dumps(pal.tolist())}</metadata>',
         f'<metadata id="pca-pose">{pose}</metadata>']

    # reference backdrop (embedded, faint, locked)
    if ref.exists():
        b64 = base64.b64encode(ref.read_bytes()).decode()
        L.append('<g inkscape:groupmode="layer" inkscape:label="reference" '
                 'sodipodi:insensitive="true" style="opacity:0.35">')
        L.append(f'<image x="0" y="0" width="{w}" height="{h}" '
                 f'xlink:href="data:image/png;base64,{b64}" '
                 'style="image-rendering:pixelated"/>')
        L.append('</g>')

    n = 0
    for layer in LAYERS:
        members = [p for p in polys if _layer_of(p["part"]) == layer]
        if not members:
            continue
        L.append(f'<g inkscape:groupmode="layer" inkscape:label="{layer}" id="layer-{layer}">')
        for p in members:
            pp = p["points"]
            dd = "M " + " L ".join(f"{int(x)},{int(y)}" for x, y in pp) + " Z"
            col = _hex(pal[p["color"]])
            # accents/shades: no stroke; main parts: thin black line-art
            stroke = "none" if (p["part"] in ACCENT or p["part"].endswith("_shade")) else "#000000"
            L.append(f'<path id="poly{n}" inkscape:label="{p["part"]}" '
                     f'd="{dd}" fill="{col}" stroke="{stroke}" stroke-width="1" '
                     f'fill-opacity="1"/>')
            n += 1
        L.append('</g>')
    L.append('</svg>')
    out = SVG_DIR / f"{pose}.svg"
    out.write_text("\n".join(L))
    print(f"exported {out}  ({n} polys, {w}x{h})")
    return out


# ---- import ----
def _compose(parent, child):
    return parent @ child


def _parse_transform(s):
    M = np.eye(3)
    if not s:
        return M
    for op, args in re.findall(r"(\w+)\s*\(([^)]*)\)", s):
        v = [float(x) for x in re.split(r"[,\s]+", args.strip()) if x]
        T = np.eye(3)
        if op == "translate":
            T[0, 2] = v[0]; T[1, 2] = v[1] if len(v) > 1 else 0
        elif op == "scale":
            T[0, 0] = v[0]; T[1, 1] = v[1] if len(v) > 1 else v[0]
        elif op == "matrix":
            T = np.array([[v[0], v[2], v[4]], [v[1], v[3], v[5]], [0, 0, 1]])
        elif op == "translateX":
            T[0, 2] = v[0]
        elif op == "translateY":
            T[1, 2] = v[0]
        M = M @ T
    return M


def _tag(e):
    return e.tag.split("}")[-1]


def _local(attrs, name):
    # fetch an attribute regardless of namespace prefix
    for k, val in attrs.items():
        if k.split("}")[-1] == name:
            return val
    return None


def import_svg(pose: str, version: str, src_version: str = None):
    svg = SVG_DIR / f"{pose}.svg"
    tree = ET.parse(svg)
    root = tree.getroot()
    pal = None
    for m in root.iter():
        if _tag(m) == "metadata" and _local(m.attrib, "id") == "pca-palette":
            pal = np.array(json.loads(m.text))
    if pal is None:
        raise SystemExit("no pca-palette metadata in SVG")
    w = int(float(root.get("width"))); h = int(float(root.get("height")))

    polys = []

    def walk(node, M):
        for ch in node:
            if _tag(ch) == "g":
                lbl = _local(ch.attrib, "label")
                if lbl == "reference":
                    continue                 # skip the backdrop
                walk(ch, M @ _parse_transform(ch.get("transform", "")))
            elif _tag(ch) in ("polygon", "path", "rect"):
                part = _local(ch.attrib, "label") or "other"
                Mc = M @ _parse_transform(ch.get("transform", ""))
                pts = _points(ch)
                if len(pts) < 3:
                    continue
                ph = np.c_[pts, np.ones(len(pts))] @ Mc.T
                pts = ph[:, :2]
                fill = ch.get("fill") or _style(ch.get("style", ""), "fill")
                ci = _nearest(fill, pal)
                polys.append({"part": part, "color": int(ci),
                              "area": float(_area(pts)),
                              "points": pts.astype(int).tolist()})

    walk(root, np.eye(3))
    # PRESERVE DOCUMENT ORDER as the z-order: the human authored the SVG layers/
    # paint order deliberately (back -> front), so that IS the stacking. Do NOT
    # re-sort by our part-name table -- it doesn't know custom labels like
    # 'chest-background' and would bury the pecs/eyes behind their backing.
    import pca_paperdoll as PD
    from PIL import Image
    out = {"w": w, "h": h, "palette": pal.tolist(), "polys": polys}
    vd = P.version_dir(version)
    (vd / f"{pose}_polys.json").write_text(json.dumps(out))
    rec = PD.render(polys, pal, w, h)
    rgba = np.dstack([rec, np.where((rec == 255).all(2), 0, 255).astype(np.uint8)])
    Image.fromarray(rgba, "RGBA").save(vd / "cand" / f"{pose}.png")
    print(f"imported {svg} -> {vd / f'{pose}_polys.json'}  ({len(polys)} polys, rendered cand)")
    return out


_NUM = r"-?\d*\.?\d+(?:[eE][+-]?\d+)?"


def _points(e):
    tg = _tag(e)
    if tg == "polygon":
        nums = [float(x) for x in re.split(r"[,\s]+", e.get("points", "").strip()) if x]
        return np.array(nums).reshape(-1, 2) if nums else np.zeros((0, 2))
    if tg == "rect":
        x = float(e.get("x", 0)); y = float(e.get("y", 0))
        ww = float(e.get("width", 0)); hh = float(e.get("height", 0))
        return np.array([[x, y], [x + ww, y], [x + ww, y + hh], [x, y + hh]], float)
    if tg == "path":
        return _flatten_path(e.get("d", ""))
    return np.zeros((0, 2))


def _flatten_path(d):
    """Parse an SVG path to a polygon -- full command set, relative/absolute,
    curves flattened to short segments. Enough for hand-edited Inkscape shapes."""
    toks = re.findall(r"[MmLlHhVvCcSsQqTtAaZz]|" + _NUM, d)
    pts = []; i = 0; cx = cy = sx = sy = 0.0; cmd = None
    n = len(toks)

    def num():
        nonlocal i
        v = float(toks[i]); i += 1; return v

    while i < n:
        if re.match(r"[A-Za-z]", toks[i]):
            cmd = toks[i]; i += 1
            if cmd in "Zz":
                cx, cy = sx, sy; continue
        rel = cmd.islower(); c = cmd.upper()
        if c == "M":
            x = num(); y = num()
            if rel: x += cx; y += cy
            cx, cy, sx, sy = x, y, x, y; pts.append((cx, cy)); cmd = "l" if rel else "L"
        elif c == "L":
            x = num(); y = num()
            if rel: x += cx; y += cy
            cx, cy = x, y; pts.append((cx, cy))
        elif c == "H":
            x = num(); cx = (cx + x) if rel else x; pts.append((cx, cy))
        elif c == "V":
            y = num(); cy = (cy + y) if rel else y; pts.append((cx, cy))
        elif c in ("C", "S", "Q", "T"):
            if c == "C":
                x1 = num(); y1 = num(); x2 = num(); y2 = num(); x = num(); y = num()
            elif c == "S":
                x2 = num(); y2 = num(); x = num(); y = num(); x1, y1 = cx, cy
            elif c == "Q":
                x1 = num(); y1 = num(); x = num(); y = num(); x2, y2 = x1, y1
            else:                               # T
                x = num(); y = num(); x1, y1 = cx, cy; x2, y2 = cx, cy
            if rel:
                x1 += cx; y1 += cy; x2 += cx; y2 += cy; x += cx; y += cy
            for t in (k / 6.0 for k in range(1, 7)):
                bx = (1 - t)**3 * cx + 3 * (1 - t)**2 * t * x1 + 3 * (1 - t) * t**2 * x2 + t**3 * x
                by = (1 - t)**3 * cy + 3 * (1 - t)**2 * t * y1 + 3 * (1 - t) * t**2 * y2 + t**3 * y
                pts.append((bx, by))
            cx, cy = x, y
        elif c == "A":
            num(); num(); num(); num(); num(); x = num(); y = num()
            if rel: x += cx; y += cy
            cx, cy = x, y; pts.append((cx, cy))
        else:
            i += 1                              # unknown -> skip a token defensively
    return np.array(pts) if pts else np.zeros((0, 2))


def _style(style, key):
    m = re.search(rf"{key}\s*:\s*([^;]+)", style or "")
    return m.group(1).strip() if m else None


def _nearest(fill, pal):
    if not fill or fill in ("none",):
        return 0
    fill = fill.strip()
    if fill.startswith("#"):
        fill = fill[1:]
        if len(fill) == 3:
            fill = "".join(c * 2 for c in fill)
        rgb = np.array([int(fill[i:i + 2], 16) for i in (0, 2, 4)])
    else:
        m = re.findall(r"\d+", fill)
        rgb = np.array([int(x) for x in m[:3]]) if len(m) >= 3 else np.array([0, 0, 0])
    return int(np.abs(pal.astype(int) - rgb).sum(1).argmin())


def _area(pts):
    x, y = pts[:, 0], pts[:, 1]
    return abs(np.dot(x, np.roll(y, 1)) - np.dot(y, np.roll(x, 1))) / 2.0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd", choices=["export", "import"])
    ap.add_argument("--pose", default=None)
    ap.add_argument("--version", required=True)
    a = ap.parse_args()
    poses = [a.pose] if a.pose else P.POSES
    for pose in poses:
        if a.cmd == "export":
            export(pose, a.version)
        else:
            import_svg(pose, a.version)


if __name__ == "__main__":
    main()
