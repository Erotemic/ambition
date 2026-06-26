#!/usr/bin/env python3
"""Build a bone rig for the Perfect Cell-ular Automaton from its hand-drawn SVG.

This is the *robust process* for turning ``pca-front-side.svg`` into an
animatable ``.rig.json``: it never reconstructs the art (the old, failed
approach) — it binds the artist's own vector parts to bones and lets the
existing FK/IK skeleton pose them.

How parts map to bones (the durable contract — survives id churn and reshaping
as long as these Inkscape *group labels* and naming keywords hold):

  * The top-level groups under ``VIEW-front-right`` are the body regions:
    ``Right-arm-front-right`` / ``left-arm`` (near/far arm),
    ``right-leg`` / ``left-leg`` (near/far leg), ``body`` (torso), and
    ``Head-front-right`` (head).
  * Within a limb, each leaf is sorted into a segment by keyword — anything
    ``hand``/``claw``/``finger`` → hand, ``lower``/``forearm``/``cuff``/
    ``shin``/``foot``/``toes``/``heel`` → lower/foot, else upper. Unlabelled
    detail paths inherit the label of their nearest labelled group (so
    ``finger-lines``, ``helmet-detail``, ``belly-grid`` resolve correctly).

Joints (shoulder/elbow/wrist, hip/knee/ankle, neck) are measured from the
aggregate bounding box of each segment's art, so the skeleton tracks the art.
Re-run this whenever the SVG changes; the ``.rig.json`` is a generated artifact.

    PY=.venv/bin/python
    $PY .../pca_rig_extract.py build       # writes the rig into targets/.../rigged/
    $PY .../pca_rig_extract.py validate     # rest pose vs full-view raster -> /tmp
"""
from __future__ import annotations

import argparse
import json
import math
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# Allow running as a loose script (the experimental dir isn't a package).
import sys
_AUTHORING = Path(__file__).resolve().parents[2]
if str(_AUTHORING) not in sys.path:
    sys.path.insert(0, str(_AUTHORING))

from authoring.svg_parts import (  # noqa: E402
    INK_NS, SVG_NS, MM_PER_INCH, rasterize_subset, _local, _label,
)

REPO = Path(__file__).resolve().parents[5]
SVG = REPO / "assets/manual-svgs/perfect-cellular-automaton/pca-front-side.svg"
RIGGED_DIR = _AUTHORING / "targets/characters/rigged"
RIG_OUT = RIGGED_DIR / "perfect_cellular_automaton.rig.json"

VIEW = "VIEW-front-right"
REF_DPI = 96.0

# top group label -> (region kind, side)
REGION = {
    "Right-arm-front-right": ("arm", "near"),
    "left-arm": ("arm", "far"),
    "right-leg": ("leg", "near"),
    "left-leg": ("leg", "far"),
    "body": ("torso", None),
    "Head-front-right": ("head", None),
}


def _segment(kind: str, label: str) -> str:
    """Which bone segment a leaf's (nearest) label belongs to within a region."""
    t = (label or "").lower()
    if kind == "arm":
        if any(k in t for k in ("hand", "claw", "finger")):
            return "hand"
        if any(k in t for k in ("lower", "forearm", "cuff", "sleeve")):
            return "lower"
        return "upper"
    if kind == "leg":
        if any(k in t for k in ("foot", "toes", "heel")):
            return "foot"
        if any(k in t for k in ("lower", "shin", "knee")):
            return "lower"
        return "upper"
    return kind  # torso / head: single bone


def _bone_name(kind: str, side: Optional[str], seg: str) -> str:
    if kind in ("torso", "head"):
        return kind
    return f"{side}_{kind[:3]}_{ {'upper':'u','lower':'l','hand':'hand','foot':'foot'}[seg] }".replace(" ", "")


def _classify_leaf(leaf: ET.Element, ancestry: List[str], top: str) -> Optional[str]:
    """Bone name for one drawable leaf, or None if its region is unmapped."""
    if top not in REGION:
        return None
    kind, side = REGION[top]
    # nearest non-empty label from leaf up to (not including) the top group
    near = next((l for l in ancestry if l and l != top), top)
    return _bone_name(kind, side, _segment(kind, near))


def _ancestry(elem: ET.Element, parent: Dict[ET.Element, ET.Element]) -> List[str]:
    out: List[str] = []
    cur: Optional[ET.Element] = elem
    while cur is not None:
        lbl = _label(cur)
        if lbl:
            out.append(lbl)
        cur = parent.get(cur)
    return out  # leaf-first


def _collect() -> Dict[str, List[Tuple[str, ET.Element]]]:
    """Group every drawable leaf under VIEW by bone name -> [(id, elem), ...]."""
    root = ET.fromstring(SVG.read_bytes())
    parent = {c: p for p in root.iter() for c in p}
    bones: Dict[str, List[Tuple[str, ET.Element]]] = {}
    for elem in root.iter():
        if _local(elem.tag) not in ("path", "polygon", "rect", "ellipse", "circle"):
            continue
        anc = _ancestry(elem, parent)
        if VIEW not in anc:
            continue
        top = anc[-1] if anc and anc[-1] != "Layer 1" else (anc[-2] if len(anc) > 1 else "")
        # top group = the child of VIEW; find it explicitly
        top = _top_group(elem, parent)
        bone = _classify_leaf(elem, anc, top)
        if bone is None:
            continue
        bones.setdefault(bone, []).append((elem.get("id", ""), elem))
    return bones


def _top_group(elem: ET.Element, parent: Dict[ET.Element, ET.Element]) -> str:
    """Label of the VIEW's direct child that contains ``elem``."""
    chain = []
    cur: Optional[ET.Element] = elem
    while cur is not None:
        chain.append(cur)
        cur = parent.get(cur)
    # chain is leaf..root; find VIEW then take the element just before it
    for i, node in enumerate(chain):
        if _label(node) == VIEW and i > 0:
            return _label(chain[i - 1]) or ""
    return ""


def _bbox_px(ids: List[str]) -> Optional[Tuple[float, float, float, float]]:
    """Aggregate full-canvas alpha bbox (ref-px @ REF_DPI) of the given ids."""
    img, (ox, oy), _ = rasterize_subset(SVG, VIEW, ids, REF_DPI)
    if img is None:
        return None
    w, h = img.size
    return (ox, oy, ox + w, oy + h)


def _cx(b):  # bbox center x
    return (b[0] + b[2]) / 2.0


def build_skeleton_data() -> dict:
    bones = _collect()
    ids = {name: [i for i, _ in items] for name, items in bones.items()}
    bb = {name: _bbox_px(idlist) for name, idlist in ids.items()}
    bb = {n: b for n, b in bb.items() if b is not None}

    def jc(name):  # joint helpers operate on segment bboxes
        return bb[name]

    # --- joints (ref-px), per chain, measured from segment bboxes ---
    J: Dict[str, Tuple[float, float]] = {}
    for side in ("near", "far"):
        u, l, h = f"{side}_arm_u", f"{side}_arm_l", f"{side}_arm_hand"
        if u in bb:
            J[f"{side}_shoulder"] = (_cx(bb[u]), bb[u][1])
        if u in bb and l in bb:
            J[f"{side}_elbow"] = ((_cx(bb[u]) + _cx(bb[l])) / 2, (bb[u][3] + bb[l][1]) / 2)
        if l in bb and h in bb:
            J[f"{side}_wrist"] = ((_cx(bb[l]) + _cx(bb[h])) / 2, (bb[l][3] + bb[h][1]) / 2)
        if h in bb:
            J[f"{side}_handtip"] = (_cx(bb[h]), bb[h][3])
        pu, pl, pf = f"{side}_leg_u", f"{side}_leg_l", f"{side}_leg_foot"
        if pu in bb:
            J[f"{side}_hip"] = (_cx(bb[pu]), bb[pu][1])
        if pu in bb and pl in bb:
            J[f"{side}_knee"] = ((_cx(bb[pu]) + _cx(bb[pl])) / 2, (bb[pu][3] + bb[pl][1]) / 2)
        if pl in bb and pf in bb:
            J[f"{side}_ankle"] = ((_cx(bb[pl]) + _cx(bb[pf])) / 2, (bb[pl][3] + bb[pf][1]) / 2)
        if pf in bb:
            # toe tip: foot far end horizontally from the ankle x
            fb = bb[pf]
            ax = J.get(f"{side}_ankle", (_cx(fb), fb[3]))[0]
            tipx = fb[0] if abs(fb[0] - ax) > abs(fb[2] - ax) else fb[2]
            J[f"{side}_toe"] = (tipx, (fb[1] + fb[3]) / 2)

    hip_c = (
        (J["near_hip"][0] + J["far_hip"][0]) / 2,
        (J["near_hip"][1] + J["far_hip"][1]) / 2,
    )
    neck = (_cx(bb["torso"]), bb["torso"][1]) if "torso" in bb else hip_c
    head_top = bb["head"][1] if "head" in bb else neck[1]

    # --- px -> frame mapping: feet to ground_y, body center to center_x ---
    feet_y = max(bb[k][3] for k in bb if k.endswith("foot")) if any(
        k.endswith("foot") for k in bb) else max(b[3] for b in bb.values())
    char_h = feet_y - head_top
    FRAME_H, FRAME_W = 192, 128
    GROUND_Y, CENTER_X, ANKLE_H = 176.0, 64.0, 2.0
    target_h = 150.0
    K = target_h / char_h
    center_x_px = hip_c[0]

    def m(p):  # ref-px -> frame units
        return ((p[0] - center_x_px) * K + CENTER_X, (p[1] - feet_y) * K + GROUND_Y)

    root = (CENTER_X, GROUND_Y)

    # --- bone tree: (name, parent, proximal joint, distal joint|None) ---
    specs: List[Tuple[str, Optional[str], Tuple[float, float], Optional[Tuple[float, float]]]] = [
        ("pelvis", None, hip_c, None),
        ("torso", "pelvis", hip_c, None),
        ("head", "torso", neck, None),
    ]
    for side in ("far", "near"):  # far first so near draws over it
        specs += [
            (f"{side}_arm_u", "torso", J[f"{side}_shoulder"], J[f"{side}_elbow"]),
            (f"{side}_arm_l", f"{side}_arm_u", J[f"{side}_elbow"], J[f"{side}_wrist"]),
            (f"{side}_arm_hand", f"{side}_arm_l", J[f"{side}_wrist"], J[f"{side}_handtip"]),
            (f"{side}_leg_u", "pelvis", J[f"{side}_hip"], J[f"{side}_knee"]),
            (f"{side}_leg_l", f"{side}_leg_u", J[f"{side}_knee"], J[f"{side}_ankle"]),
            (f"{side}_leg_foot", f"{side}_leg_l", J[f"{side}_ankle"], J[f"{side}_toe"]),
        ]

    # Build BoneWorld rest transforms, then derive parent-relative offset/rest.
    world: Dict[str, Tuple[Tuple[float, float], float]] = {}  # name -> (origin_frame, angle_deg)
    bone_json: List[dict] = []
    proximal_px: Dict[str, Tuple[float, float]] = {}
    for name, par, jp, jd in specs:
        o = m(jp)
        proximal_px[name] = jp
        if jd is not None:
            d = m(jd)
            ang = math.degrees(math.atan2(d[1] - o[1], d[0] - o[0]))
            length = math.hypot(d[0] - o[0], d[1] - o[1])
        else:
            ang, length = 0.0, 0.0
        if par is None:
            po, pa = root, 0.0
        else:
            po, pa = world[par]
        # offset in parent-local frame; rest_angle relative to parent
        dx, dy = o[0] - po[0], o[1] - po[1]
        ca, sa = math.cos(math.radians(-pa)), math.sin(math.radians(-pa))
        off = (dx * ca - dy * sa, dx * sa + dy * ca)
        bone_json.append({
            "name": name, "parent": par,
            "offset": [round(off[0], 3), round(off[1], 3)],
            "length": round(length, 3), "rest_angle": round(ang - pa, 3),
        })
        world[name] = (o, ang)

    return {
        "bones": bone_json,
        "ids": ids,
        "proximal_px": proximal_px,
        "rest_angle": {n: a for n, (_, a) in world.items()},
        "frame": dict(width=FRAME_W, height=FRAME_H, ground_y=GROUND_Y,
                      center_x=CENTER_X, ankle_h=ANKLE_H),
        "K": K,
    }


# z-order: far limbs behind torso/head, near limbs in front.
Z = {
    "far_leg_u": 5, "far_leg_l": 6, "far_leg_foot": 7,
    "far_arm_u": 8, "far_arm_l": 9, "far_arm_hand": 10,
    "pelvis": 20, "torso": 22, "head": 30,
    "near_leg_u": 40, "near_leg_l": 41, "near_leg_foot": 42,
    "near_arm_u": 60, "near_arm_l": 61, "near_arm_hand": 62,
}


def build_doc() -> dict:
    sk = build_skeleton_data()
    K = sk["K"]
    parts = []
    for b in sk["bones"]:
        name = b["name"]
        ids = sk["ids"].get(name)
        if not ids:
            continue
        jp = sk["proximal_px"][name]
        parts.append({
            "name": name, "bone": name, "z": Z.get(name, 50), "kind": "sprite",
            "include": ids,
            "pivot": [round(jp[0], 2), round(jp[1], 2)],
            "rest_angle": round(sk["rest_angle"][name], 3),
        })
    parts.sort(key=lambda p: p["z"])

    svg_rel = Path("../../../../../..") / SVG.relative_to(REPO)
    fr = sk["frame"]
    fr.update(supersample=4, render_scale=1)
    return {
        "name": "perfect_cellular_automaton",
        "frame": fr,
        "svg_source": {
            "path": str(svg_rel), "view": VIEW,
            "ref_dpi": REF_DPI, "scale": round(K, 6),
        },
        "palette": {},
        "bones": sk["bones"],
        "parts": parts,
        "ik_legs": [
            {"upper": "near_leg_u", "lower": "near_leg_l", "foot": "near_leg_foot",
             "channel_prefix": "near_foot", "rest_x": 4.0, "bend": 1.0},
            {"upper": "far_leg_u", "lower": "far_leg_l", "foot": "far_leg_foot",
             "channel_prefix": "far_foot", "rest_x": -4.0, "bend": 1.0},
        ],
        "clips": {
            "idle": {"loop": True, "frames": 8, "duration_ms": 130, "channels": {
                "torso": {"expr": "1.5*sin(tau*t)"},
                "near_arm_l": {"expr": "3*sin(tau*t)"},
                "far_arm_l": {"expr": "-3*sin(tau*t)"},
            }},
        },
        "sprite_tuning": {"collision_scale": 1.6},
    }


def cmd_build(_args):
    RIGGED_DIR.mkdir(parents=True, exist_ok=True)
    doc = build_doc()
    RIG_OUT.write_text(json.dumps(doc, indent=1) + "\n", encoding="utf8")
    print(f"wrote {RIG_OUT}  ({len(doc['bones'])} bones, {len(doc['parts'])} parts)")


def cmd_validate(_args):
    from authoring.rigdoc import RigDocument
    cmd_build(_args)
    doc = RigDocument.load(RIG_OUT)
    # Pure rest = idle@0 with IK disabled (IK tuning is separate from binding).
    doc.data["ik_legs"] = []
    frame = doc.render_at("idle", 0.0)
    out = Path("/tmp/pca_rig_rest.png")
    frame.save(out)
    print(f"rest frame -> {out}  size={frame.size}")


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("build").set_defaults(fn=cmd_build)
    sub.add_parser("validate").set_defaults(fn=cmd_validate)
    args = ap.parse_args()
    args.fn(args)
