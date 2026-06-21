#!/usr/bin/env python3
"""
YAML scene-graph sprite renderer for the mockingbird boss.

The schema is intentionally SVG-like:
- every node is either a group or a shape
- groups and shapes both have labels, transforms, visibility, and z_order
- shapes store one primitive in `primitive:`
- groups recursively contain `children:`
- all transforms are applied to vector geometry before rasterization

Dependencies:
    python -m pip install pillow pyyaml
"""

from __future__ import annotations

import argparse
import copy
import json
import logging
import math
import shutil
import time
from pathlib import Path
from typing import Dict, Iterable, List, Tuple

from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

from ambition_sprite2d_renderer.cli.console import print_paths
from ambition_sprite2d_renderer.core.manifest_ron import records_to_ron

try:
    import yaml
except Exception as ex:
    raise SystemExit("Missing dependency: python -m pip install pyyaml") from ex

RGBA = Tuple[int, int, int, int]
DATA_DIR = Path(__file__).resolve().parent
TOOL_ROOT = DATA_DIR.parents[1]
TARGET_NAME = "mockingbird_boss"
DEFAULT_SCENE = DATA_DIR / "mockingbird_boss_scene.yaml"
OUTPUT_FILES = [
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_spritesheet_manifest.json",
    f"{TARGET_NAME}_canonical.png",
    f"{TARGET_NAME}_canonical_transparent.png",
    f"{TARGET_NAME}_preview_labeled.png",
    f"{TARGET_NAME}_parts_debug.png",
    "sources_and_inspirations.md",
]

LOG = logging.getLogger("mockingbird_scene")


def now():
    return time.perf_counter()


class Timer:
    def __init__(self, label):
        self.label = label

    def __enter__(self):
        self.t0 = now()
        LOG.info("start: %s", self.label)
        return self

    def __exit__(self, exc_type, exc, tb):
        LOG.info("done: %s in %.3fs", self.label, now() - self.t0)


def mat_mul(A, B):
    return [
        [
            A[0][0] * B[0][0] + A[0][1] * B[1][0],
            A[0][0] * B[0][1] + A[0][1] * B[1][1],
            A[0][0] * B[0][2] + A[0][1] * B[1][2] + A[0][2],
        ],
        [
            A[1][0] * B[0][0] + A[1][1] * B[1][0],
            A[1][0] * B[0][1] + A[1][1] * B[1][1],
            A[1][0] * B[0][2] + A[1][1] * B[1][2] + A[1][2],
        ],
        [0, 0, 1],
    ]


def mat_apply(M, p):
    x, y = p
    return (M[0][0] * x + M[0][1] * y + M[0][2], M[1][0] * x + M[1][1] * y + M[1][2])


def transform_matrix(t=None):
    t = t or {}
    x = float(t.get("x", 0))
    y = float(t.get("y", 0))
    r = math.radians(float(t.get("rotation", 0)))
    sx = float(t.get("scale_x", t.get("scale", 1)))
    sy = float(t.get("scale_y", t.get("scale", 1)))
    c = math.cos(r)
    s = math.sin(r)
    return [[c * sx, -s * sy, x], [s * sx, c * sy, y], [0, 0, 1]]


def transform_merge(base, delta):
    """Merge additive animation transform deltas into a YAML scene transform."""
    out = dict(base or {})
    for key in ("x", "y", "rotation"):
        if key in delta:
            out[key] = float(out.get(key, 0)) + float(delta[key])
    for key in ("scale", "scale_x", "scale_y"):
        if key in delta:
            out[key] = float(out.get(key, 1)) * float(delta[key])
    return out


def color_scale_alpha(color, scale):
    r, g, b, a = color
    return (r, g, b, max(0, min(255, int(round(a * scale)))))


def wave(phase, cycles=1.0, offset=0.0):
    return math.sin(math.tau * (phase * cycles + offset))


def blink01(phase, cycles=1.0, offset=0.0):
    return 0.5 + 0.5 * wave(phase, cycles, offset)


def lerp(a, b, t):
    return a + (b - a) * t


def rect_points(box):
    x1, y1, x2, y2 = box
    return [(x1, y1), (x2, y1), (x2, y2), (x1, y2)]


def ellipse_points(box, n=32):
    x1, y1, x2, y2 = box
    cx, cy = (x1 + x2) / 2, (y1 + y2) / 2
    rx, ry = (x2 - x1) / 2, (y2 - y1) / 2
    return [
        (cx + math.cos(i * math.tau / n) * rx, cy + math.sin(i * math.tau / n) * ry)
        for i in range(n)
    ]


def rotated_rect(cx, cy, w, h, deg):
    r = math.radians(deg)
    c = math.cos(r)
    s = math.sin(r)
    pts = []
    for dx, dy in [(-w / 2, -h / 2), (w / 2, -h / 2), (w / 2, h / 2), (-w / 2, h / 2)]:
        pts.append((cx + dx * c - dy * s, cy + dx * s + dy * c))
    return pts


def crop_alpha(img: Image.Image, pad=8) -> Image.Image:
    bbox = img.getbbox()
    if bbox is None:
        return img
    x1, y1, x2, y2 = bbox
    return img.crop(
        (
            max(0, x1 - pad),
            max(0, y1 - pad),
            min(img.width, x2 + pad),
            min(img.height, y2 + pad),
        )
    )


def add_outline(img: Image.Image, color=(14, 17, 24, 255)) -> Image.Image:
    alpha = img.getchannel("A")
    grown = alpha.filter(ImageFilter.MaxFilter(3))
    ring = ImageChops.subtract(grown, alpha)
    rim = Image.new("RGBA", img.size, color)
    rim.putalpha(ring)
    return Image.alpha_composite(rim, img)


def font(size=14):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            pass
    return ImageFont.load_default()


class Scene:
    def __init__(self, data):
        self.data = data
        self.colors = data.get("colors", {})
        self.root = data["root"]
        self.meta = data.get("meta", {})
        self.render_cfg = data.get("render", {})
        self.animations = data.get(
            "animations", {"rest": {"frames": 1, "duration_ms": 100}}
        )

    @classmethod
    def load(cls, fpath):
        with open(fpath, "r") as f:
            return cls(yaml.safe_load(f))

    def save(self, fpath):
        with open(fpath, "w") as f:
            yaml.safe_dump(self.data, f, sort_keys=False, width=120)

    def color(self, value, default=(255, 255, 255, 255)):
        if value is None:
            return default
        if isinstance(value, str):
            value = self.colors.get(value, value)
        if isinstance(value, str):
            raise KeyError(f"Unknown color name: {value}")
        if len(value) == 3:
            return tuple(value) + (255,)
        return tuple(value)

    def background_rgba(self):
        value = self.render_cfg.get("background_rgba", [231, 235, 240, 255])
        return self.color(value)


class Renderer:
    def __init__(self, scene: Scene, frame_size=None, aa_scale=None, origin=None):
        self.scene = scene
        if frame_size is None:
            frame_size = tuple(
                scene.render_cfg.get(
                    "work_size", scene.meta.get("work_size", [900, 640])
                )
            )
        self.frame_size = tuple(frame_size)
        self.aa = int(
            aa_scale if aa_scale is not None else scene.render_cfg.get("supersample", 2)
        )
        self.origin = tuple(
            origin
            if origin is not None
            else scene.meta.get(
                "origin", [self.frame_size[0] / 2, self.frame_size[1] / 2]
            )
        )
        self.bounds_by_id: Dict[str, Tuple[float, float, float, float]] = {}
        self.nodes_by_id: Dict[str, dict] = {}

    def S(self, v):
        return int(round(v * self.aa))

    def P(self, p):
        return (self.S(p[0] + self.origin[0]), self.S(p[1] + self.origin[1]))

    def update_bounds(self, node_id, pts):
        if not pts:
            return
        pts = [(x + self.origin[0], y + self.origin[1]) for x, y in pts]
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        box = (min(xs), min(ys), max(xs), max(ys))
        old = self.bounds_by_id.get(node_id)
        if old is None:
            self.bounds_by_id[node_id] = box
        else:
            self.bounds_by_id[node_id] = (
                min(old[0], box[0]),
                min(old[1], box[1]),
                max(old[2], box[2]),
                max(old[3], box[3]),
            )

    def draw_polygon(self, draw, pts, fill, outline=None, width=1):
        if not pts:
            return
        draw.polygon([self.P(p) for p in pts], fill=fill)
        if outline is not None:
            draw.line(
                [self.P(p) for p in pts + [pts[0]]],
                fill=outline,
                width=max(1, self.S(width)),
                joint="curve",
            )

    def draw_line(self, draw, pts, fill, width=1):
        if not pts:
            return
        draw.line(
            [self.P(p) for p in pts],
            fill=fill,
            width=max(1, self.S(width)),
            joint="curve",
        )

    def primitive_points(self, prim):
        typ = prim.get("type")
        if typ == "polygon":
            return [tuple(p) for p in prim.get("points", [])]
        if typ in {"rect", "rounded_rect"}:
            return rect_points(prim.get("box", [0, 0, 10, 10]))
        if typ == "ellipse":
            return ellipse_points(prim.get("box", [0, 0, 10, 10]))
        if typ == "line":
            return [tuple(p) for p in prim.get("points", [])]
        return []

    def animation_transform_delta(self, node_id, anim_name, phase):
        """Return per-node transform deltas for sprite-sheet animation frames.

        These are deliberately vector-space transforms.  The renderer never warps
        an already-rasterized frame; instead, each moving part is transformed
        before primitive rasterization so the sheet stays crisp.
        """
        s1 = wave(phase, 1.0)
        s2 = wave(phase, 2.0)
        c1 = math.cos(math.tau * phase)
        b2 = blink01(phase, 2.0)

        # Death is mostly a single collapse pose with small settling motion.
        if anim_name == "death":
            settle = min(1.0, phase * 1.35)
            wobble = wave(phase, 1.5) * (1.0 - settle)
            death_pose = {
                "root": {
                    "x": -10 * settle,
                    "y": 34 * settle,
                    "rotation": 17 * settle + wobble * 3,
                },
                "body": {"rotation": 8 * settle, "scale_y": 0.96},
                "head": {"x": -6 * settle, "y": 8 * settle, "rotation": 13 * settle},
                "head_lower_jaw": {"y": 5 * settle, "rotation": 20 * settle},
                "front_wing": {"y": 10 * settle, "rotation": 24 * settle},
                "rear_wing": {"y": 8 * settle, "rotation": -18 * settle},
                "left_foreclaw": {"y": 12 * settle, "rotation": -30 * settle},
                "right_foreclaw": {"y": 10 * settle, "rotation": 26 * settle},
                "left_leg": {"y": 8 * settle, "rotation": -13 * settle},
                "right_leg": {"y": 8 * settle, "rotation": 12 * settle},
                "tail": {"y": 6 * settle, "rotation": -11 * settle},
            }
            return death_pose.get(node_id, {})

        # Baseline engine hover motion is present in every living animation.
        delta = {}
        if node_id == "root":
            delta.update({"y": -3.4 * s1, "rotation": 0.9 * s1})
        elif node_id == "body":
            delta.update({"y": 1.7 * c1, "rotation": 0.55 * s1})
        elif node_id == "tail":
            delta.update({"rotation": -1.2 * s1, "y": 1.0 * c1})
        elif node_id == "head":
            delta.update({"x": 1.1 * c1, "rotation": 0.8 * s1})
        elif node_id == "front_wing":
            delta.update({"rotation": 2.4 * s1, "y": -1.1 * c1})
        elif node_id == "rear_wing":
            delta.update({"rotation": -1.9 * s1, "y": 0.9 * c1})
        elif node_id == "left_leg":
            delta.update({"rotation": 1.4 * s1, "y": 1.2 * b2})
        elif node_id == "right_leg":
            delta.update({"rotation": -1.2 * s1, "y": 1.1 * blink01(phase, 2.0, 0.5)})
        elif node_id == "left_foreclaw":
            delta.update({"rotation": -2.0 * s1, "x": -1.4 * b2})
        elif node_id == "right_foreclaw":
            delta.update({"rotation": 1.7 * s1, "x": 1.2 * blink01(phase, 2.0, 0.5)})

        # Animation-specific accents.
        if anim_name == "thrust":
            if node_id == "root":
                delta.update(
                    {
                        "x": delta.get("x", 0) - 3.0 - 2.4 * b2,
                        "y": delta.get("y", 0) - 2.0 * b2,
                    }
                )
            elif node_id == "body":
                delta.update(
                    {"rotation": delta.get("rotation", 0) - 2.6, "scale_x": 1.018}
                )
            elif node_id in {"front_wing", "rear_wing"}:
                delta.update(
                    {
                        "rotation": delta.get("rotation", 0)
                        + (7 if node_id == "front_wing" else -6)
                    }
                )
            elif node_id == "tail":
                delta.update(
                    {
                        "x": delta.get("x", 0) + 3.0,
                        "rotation": delta.get("rotation", 0) - 3.5,
                    }
                )
        elif anim_name == "bite":
            snap = blink01(phase, 1.0)
            if node_id == "head":
                delta.update(
                    {
                        "x": delta.get("x", 0) - 6.0 * snap,
                        "rotation": delta.get("rotation", 0) - 4.0 * snap,
                    }
                )
            elif node_id == "head_lower_jaw":
                delta.update({"y": 5.5 * snap, "rotation": 23.0 * snap})
            elif node_id == "head_teeth":
                delta.update({"x": -2.2 * snap})
            elif node_id == "body":
                delta.update(
                    {
                        "x": -2.2 * snap,
                        "rotation": delta.get("rotation", 0) - 1.0 * snap,
                    }
                )
        elif anim_name == "slash":
            cut = blink01(phase, 1.0)
            lead = wave(phase, 1.0, -0.18)
            if node_id == "left_foreclaw":
                delta.update(
                    {
                        "x": -16.0 * cut,
                        "y": -6.0 * cut,
                        "rotation": -48.0 * cut + 5 * lead,
                    }
                )
            elif node_id == "right_foreclaw":
                delta.update({"x": -10.0 * cut, "y": 4.0 * cut, "rotation": 31.0 * cut})
            elif node_id == "front_wing":
                delta.update({"rotation": delta.get("rotation", 0) + 10.0 * cut})
            elif node_id == "body":
                delta.update(
                    {"rotation": delta.get("rotation", 0) - 3.0 * cut, "x": -3.0 * cut}
                )
        elif anim_name == "hit":
            jolt = wave(phase, 2.0)
            if node_id == "root":
                delta.update(
                    {
                        "x": 7.0 * jolt,
                        "y": delta.get("y", 0) - 3.0 * abs(jolt),
                        "rotation": delta.get("rotation", 0) + 4.0 * jolt,
                    }
                )
            elif node_id == "head":
                delta.update(
                    {
                        "x": delta.get("x", 0) + 5.0 * jolt,
                        "rotation": delta.get("rotation", 0) + 6.0 * jolt,
                    }
                )
            elif node_id in {
                "front_wing",
                "rear_wing",
                "left_foreclaw",
                "right_foreclaw",
            }:
                delta.update(
                    {
                        "rotation": delta.get("rotation", 0)
                        + (
                            10.0
                            if node_id in {"front_wing", "right_foreclaw"}
                            else -10.0
                        )
                        * jolt
                    }
                )
        return delta

    def animated_transform(self, node, anim_name, phase):
        base = node.get("transform", {})
        delta = self.animation_transform_delta(node.get("id", ""), anim_name, phase)
        return transform_merge(base, delta) if delta else base

    def draw_death_eye_x(self, draw, node, prim, M):
        box = prim.get("box", [-32, -4, -29, -1])
        x1, y1, x2, y2 = box
        # Deliberately larger than the tiny live-eye dot so the death read is
        # unmistakable at 256px sprite scale. Draw a dark stroke first, then a
        # bright stroke, to keep the X visible on both the skull and background.
        pad = max(7.5, max(x2 - x1, y2 - y1) * 2.4)
        cx = (x1 + x2) / 2
        cy = (y1 + y2) / 2
        a = [mat_apply(M, (cx - pad, cy - pad)), mat_apply(M, (cx + pad, cy + pad))]
        b = [mat_apply(M, (cx - pad, cy + pad)), mat_apply(M, (cx + pad, cy - pad))]
        self.draw_line(draw, a, self.scene.color("outline"), 4.0)
        self.draw_line(draw, b, self.scene.color("outline"), 4.0)
        self.draw_line(draw, a, self.scene.color("tooth"), 2.2)
        self.draw_line(draw, b, self.scene.color("tooth"), 2.2)
        return a + b

    def draw_special(self, draw, node, prim, M, anim_name, phase):
        typ = prim.get("type")
        outline = self.scene.color(prim.get("outline", "outline"))
        pts_for_bounds = []
        if typ == "flame":
            if anim_name == "death":
                return []
            length = float(prim.get("length", 56))
            height = float(prim.get("height", 24))
            # Every living animation gets an alternating engine pattern.  The
            # outer plume, inner plume, and core drift at different rates so
            # consecutive frames are visually distinct even in idle/hover.
            flicker = 0.78 + 0.30 * blink01(phase, 2.0, 0.10)
            stretch = flicker
            height *= 0.86 + 0.24 * blink01(phase, 3.0, 0.29)
            alpha_scale = 0.72 + 0.42 * blink01(phase, 2.0, 0.34)
            if anim_name == "thrust":
                stretch += 0.62 + 0.28 * blink01(phase, 3.0, 0.17)
                alpha_scale += 0.20
            elif anim_name in {"bite", "slash", "hit"}:
                stretch += 0.14 * blink01(phase, 1.0, 0.45)
            outer = [
                (0, 0),
                (length * 0.2 * stretch, -height * 0.35),
                (length * 0.55 * stretch, -height * 0.15),
                (length * stretch, height * 0.15),
                (length * 0.55 * stretch, height * 0.45),
                (length * 0.15 * stretch, height * 0.35),
            ]
            inner = [
                (0, 0),
                (length * 0.25 * stretch, -height * 0.15),
                (length * 0.72 * stretch, height * 0.1),
                (length * 0.28 * stretch, height * 0.3),
            ]
            core = [
                (0, 0),
                (length * 0.16 * stretch, -height * 0.06),
                (length * 0.42 * stretch, height * 0.06),
                (length * 0.2 * stretch, height * 0.22),
            ]
            for idx, (pts, col) in enumerate(
                [(outer, "flame1"), (inner, "flame2"), (core, "flame3")]
            ):
                wpts = [mat_apply(M, p) for p in pts]
                color = color_scale_alpha(
                    self.scene.color(col), alpha_scale * (1.0 + 0.10 * idx)
                )
                self.draw_polygon(draw, wpts, color, None)
                pts_for_bounds += wpts
        elif typ == "spine":
            start = tuple(prim.get("start", [0, 0]))
            end = tuple(prim.get("end", [-60, 0]))
            bend = float(prim.get("bend", -8))
            segs = int(prim.get("segments", 7))
            centers = []
            for i in range(segs):
                t = i / max(1, segs - 1)
                centers.append(
                    (
                        lerp(start[0], end[0], t),
                        lerp(start[1], end[1], t) + math.sin(t * math.pi) * bend,
                    )
                )
            for a, b in zip(centers, centers[1:]):
                w = [mat_apply(M, a), mat_apply(M, b)]
                self.draw_line(draw, w, self.scene.color("steel2"), 2)
                pts_for_bounds += w
            for i, p in enumerate(centers):
                t = i / max(1, segs - 1)
                rr = rotated_rect(p[0], p[1], 10, 8, -10 - 12 * t)
                w = [mat_apply(M, q) for q in rr]
                self.draw_polygon(draw, w, self.scene.color("dark2"), outline, 1)
                pts_for_bounds += w
        elif typ == "teeth_row":
            start = prim.get("start", [0, 0])
            end = prim.get("end", [20, 0])
            count = int(prim.get("count", 6))
            direction = prim.get("direction", "down")
            length = float(prim.get("length", 7))
            width = float(prim.get("tooth_width", 3.2))
            lean = float(prim.get("lean", -1.2))
            sign = 1 if direction == "down" else -1
            for i in range(count):
                t = i / max(1, count - 1)
                x = lerp(start[0], end[0], t)
                y = lerp(start[1], end[1], t)
                tri = [
                    (x, y),
                    (x + lean, y + sign * length),
                    (x + width, y + sign * length * 0.78),
                ]
                w = [mat_apply(M, p) for p in tri]
                self.draw_polygon(
                    draw, w, self.scene.color(prim.get("fill", "tooth")), None
                )
                pts_for_bounds += w
        elif typ == "teeth":
            # compatibility: draw old combined teeth as two rows
            for row in [
                {
                    "start": prim.get("upper_start", [0, 0]),
                    "end": prim.get("upper_end", [20, 0]),
                    "count": prim.get("count", 6),
                    "direction": "down",
                    "length": 7,
                    "tooth_width": 3.2,
                    "lean": -2.4,
                    "fill": "tooth",
                },
                {
                    "start": prim.get("lower_start", [0, 10]),
                    "end": prim.get("lower_end", [20, 10]),
                    "count": max(1, int(prim.get("count", 6)) - 2),
                    "direction": "up",
                    "length": 6.4,
                    "tooth_width": 3.0,
                    "lean": -2.0,
                    "fill": "tooth",
                },
            ]:
                pts_for_bounds += self.draw_special(
                    draw, node, {"type": "teeth_row", **row}, M, anim_name, phase
                )
        elif typ == "wing":
            darker = prim.get("darker", False)
            base_fill = self.scene.color("dark2" if darker else "dark")
            shade_fill = self.scene.color("dark2" if darker else "steel2")
            panel_fill = self.scene.color("steel")
            # A chunkier swept wing with a clearer root, trailing edge, and underslung cannon.
            main_wing = [
                (2, 0),
                (-16, -10),
                (-46, -15),
                (-78, -9),
                (-70, -1),
                (-38, 5),
                (-10, 9),
            ]
            wpts = [mat_apply(M, p) for p in main_wing]
            self.draw_polygon(draw, wpts, base_fill, outline, 1.2)
            pts_for_bounds += wpts
            # darker underside / trailing panel
            panel = [(-6, 1), (-24, 4), (-54, 7), (-70, 5), (-56, 12), (-22, 10)]
            w = [mat_apply(M, p) for p in panel]
            self.draw_polygon(draw, w, shade_fill, None)
            pts_for_bounds += w
            # leading edge shine / armor strip
            shine = [(-14, -8), (-42, -12), (-64, -9), (-43, -6)]
            w = [mat_apply(M, p) for p in shine]
            self.draw_polygon(draw, w, panel_fill, None)
            pts_for_bounds += w
            # root fairing
            fairing = [(-4, -3), (8, -1), (8, 4), (-5, 5), (-10, 1)]
            w = [mat_apply(M, p) for p in fairing]
            self.draw_polygon(draw, w, self.scene.color("steel2"), outline, 1)
            pts_for_bounds += w
            if prim.get("cannon", True):
                # pylon
                pylon = [(-24, 4), (-38, 9)]
                self.draw_line(
                    draw,
                    [mat_apply(M, pylon[0]), mat_apply(M, pylon[1])],
                    self.scene.color("steel2"),
                    2,
                )
                pts_for_bounds += [mat_apply(M, p) for p in pylon]
                # cannon body and muzzle pod
                for box, fill in [
                    ((-84, 9, -42, 15), "dark2"),
                    ((-44, 8, -30, 16), "steel2"),
                    ((-94, 8, -82, 16), "steel"),
                ]:
                    pts = (
                        ellipse_points(box, 20)
                        if (box[2] - box[0]) <= 16
                        else rect_points(box)
                    )
                    w = [mat_apply(M, p) for p in pts]
                    self.draw_polygon(
                        draw,
                        w,
                        self.scene.color(fill),
                        outline if fill != "steel" else None,
                        1,
                    )
                    pts_for_bounds += w
                # muzzle barrel
                barrel = [(-102, 11), (-84, 11)]
                self.draw_line(
                    draw,
                    [mat_apply(M, barrel[0]), mat_apply(M, barrel[1])],
                    self.scene.color("outline"),
                    2,
                )
                pts_for_bounds += [mat_apply(M, p) for p in barrel]
        elif typ == "leg":
            hip = tuple(prim.get("hip", [0, 0]))
            knee = tuple(prim.get("knee", [0, 20]))
            foot = tuple(prim.get("foot", [0, 40]))
            w = [mat_apply(M, p) for p in [hip, knee, foot]]
            self.draw_line(draw, w[:2], self.scene.color("steel2"), 3.5)
            self.draw_line(draw, w[1:], self.scene.color("steel"), 3)
            for tip in [
                (foot[0] - 7, foot[1] + 2),
                (foot[0] - 8, foot[1] - 2),
                (foot[0] + 2, foot[1] + 4),
            ]:
                self.draw_line(
                    draw,
                    [mat_apply(M, foot), mat_apply(M, tip)],
                    self.scene.color("outline"),
                    2,
                )
            pts_for_bounds += w
        elif typ == "claw_arm":
            anchor = tuple(prim.get("anchor", [0, 0]))
            elbow = tuple(prim.get("elbow", [-10, 10]))
            wrist = tuple(prim.get("wrist", [-20, 20]))
            spread = float(prim.get("spread", 7))
            w = [mat_apply(M, p) for p in [anchor, elbow, wrist]]
            self.draw_line(draw, w[:2], self.scene.color("steel2"), 3)
            self.draw_line(draw, w[1:], self.scene.color("steel"), 2.8)
            for tip in [
                (wrist[0] - 9, wrist[1] - spread),
                (wrist[0] - 12, wrist[1]),
                (wrist[0] - 9, wrist[1] + spread),
            ]:
                self.draw_line(
                    draw,
                    [mat_apply(M, wrist), mat_apply(M, tip)],
                    self.scene.color("outline"),
                    2,
                )
            pts_for_bounds += w
        elif typ == "rotor":
            center = tuple(prim.get("center", [0, 0]))
            radius = float(prim.get("radius", 26))
            tilt = float(prim.get("tilt", 0.24))
            base_phase = float(prim.get("phase", 0))
            spin = (
                0.0
                if anim_name == "death"
                else {
                    # "rest" is the boss's hovering idle pose (its gentle
                    # hover-bob); attack rows spin faster.
                    "rest": 1.7,
                    "thrust": 2.8,
                    "bite": 1.9,
                    "slash": 2.2,
                    "hit": 2.5,
                }.get(anim_name, 1.7)
            )
            phase_deg = base_phase + phase * 360 * spin
            strobe = 0.5 + 0.5 * math.sin(
                # The idle "rest" pose bobs slower (2.0) than the active rows (3.0).
                math.tau * phase * (3.0 if anim_name != "rest" else 2.0)
                + math.radians(base_phase)
            )
            blur_alpha = int(prim.get("blur_alpha", 48))
            shine_alpha = int(prim.get("shine_alpha", 145))
            if anim_name == "death":
                blur_alpha = int(blur_alpha * 0.28)
                shine_alpha = int(shine_alpha * 0.18)
            else:
                blur_alpha = int(blur_alpha * (0.78 + 0.30 * strobe))
                shine_alpha = int(shine_alpha * (0.72 + 0.46 * strobe))
            c = mat_apply(M, center)

            # Rotor as a spinning disc: layered translucent ellipses plus alternating
            # specular streaks. This reads better at sprite scale than outlined blades
            # and makes every living animation visibly cycle frame-to-frame.
            disc_layers = [
                (radius * 1.08, radius * tilt, blur_alpha),
                (radius * 0.88, radius * tilt * 0.62, int(blur_alpha * 0.72)),
                (radius * 0.58, radius * tilt * 0.33, int(blur_alpha * 0.45)),
            ]
            for rx, ry, alpha in disc_layers:
                pts = ellipse_points([c[0] - rx, c[1] - ry, c[0] + rx, c[1] + ry], 44)
                # keep the spinning rotor readable without bright white halos
                self.draw_polygon(draw, pts, (116, 132, 150, max(8, alpha)), None)
                pts_for_bounds += pts

            r = math.radians(phase_deg)
            co = math.cos(r)
            si = math.sin(r)

            def proj(u, v):
                return (c[0] + u * co - v * si, c[1] + (u * si + v * co) * tilt)

            shine_len = radius * 1.02
            shine_w = radius * 0.12
            shine = [
                proj(-shine_len * 0.92, -shine_w),
                proj(shine_len * 0.78, -shine_w * 0.34),
                proj(shine_len * 0.96, shine_w * 0.22),
                proj(-shine_len * 0.72, shine_w * 0.55),
            ]
            self.draw_polygon(
                draw, shine, (196, 210, 226, int(shine_alpha * 0.92)), None
            )
            pts_for_bounds += shine
            if anim_name != "death":
                r2 = r + math.pi / 2
                co2 = math.cos(r2)
                si2 = math.sin(r2)

                def proj2(u, v):
                    return (c[0] + u * co2 - v * si2, c[1] + (u * si2 + v * co2) * tilt)

                alt = [
                    proj2(-shine_len * 0.64, -shine_w * 0.55),
                    proj2(shine_len * 0.60, -shine_w * 0.18),
                    proj2(shine_len * 0.70, shine_w * 0.18),
                    proj2(-shine_len * 0.58, shine_w * 0.45),
                ]
                alt_alpha = int(shine_alpha * (0.22 + 0.34 * (1.0 - strobe)))
                self.draw_polygon(draw, alt, (116, 132, 150, alt_alpha), None)
                pts_for_bounds += alt

            # Do not draw a separate rotor hub ball here; the mast assembly already
            # provides the visible hub / cap, and leaving only the blurred blades
            # reads better for this sprite.
        else:
            LOG.warning(
                "unsupported special primitive type=%s node=%s", typ, node.get("id")
            )
        return pts_for_bounds

    def draw_shape(self, draw, node, M, anim_name, phase):
        prim = node.get("primitive", {})
        typ = prim.get("type")
        pts_for_bounds = []
        fill = (
            self.scene.color(prim.get("fill")) if prim.get("fill") is not None else None
        )
        outline = (
            self.scene.color(prim.get("outline"))
            if prim.get("outline") is not None
            else None
        )
        width = float(prim.get("width", 1))
        if typ in {"polygon", "rect", "rounded_rect", "ellipse", "line"}:
            pts = [mat_apply(M, p) for p in self.primitive_points(prim)]
            is_dead_eye = anim_name == "death" and node.get("id") == "head_06_ellipse"
            if is_dead_eye:
                fill = self.scene.color("black")
            if typ == "line":
                self.draw_line(draw, pts, fill or self.scene.color("outline"), width)
            else:
                self.draw_polygon(draw, pts, fill, outline, width)
                if is_dead_eye:
                    pts_for_bounds += self.draw_death_eye_x(draw, node, prim, M)
            pts_for_bounds += pts
        else:
            pts_for_bounds += (
                self.draw_special(draw, node, prim, M, anim_name, phase) or []
            )
        return pts_for_bounds

    def render_node(self, draw, node, parent_M, anim_name, phase):
        if not node.get("visible", True):
            return []
        self.nodes_by_id[node["id"]] = node
        M = mat_mul(
            parent_M, transform_matrix(self.animated_transform(node, anim_name, phase))
        )
        pts_all = []
        if node.get("kind") == "shape":
            pts_all += self.draw_shape(draw, node, M, anim_name, phase)
        else:
            children = node.get("children", [])
            for child in sorted(
                children, key=lambda n: (int(n.get("z_order", 0)), str(n.get("id", "")))
            ):
                pts_all += self.render_node(draw, child, M, anim_name, phase)
        self.update_bounds(node["id"], pts_all)
        return pts_all

    def render(self, anim_name="rest", frame_index=0, nframes=1, debug=False):
        W, H = self.frame_size
        img = Image.new("RGBA", (W * self.aa, H * self.aa), (0, 0, 0, 0))
        draw = ImageDraw.Draw(img, "RGBA")
        self.bounds_by_id = {}
        self.nodes_by_id = {}
        phase = 0 if nframes <= 1 else frame_index / nframes
        LOG.debug(
            "render frame anim=%s frame=%d/%d size=%s aa=%s",
            anim_name,
            frame_index,
            nframes,
            self.frame_size,
            self.aa,
        )
        self.render_node(
            draw, self.scene.root, [[1, 0, 0], [0, 1, 0], [0, 0, 1]], anim_name, phase
        )
        img = img.resize((W, H), Image.Resampling.LANCZOS)
        if bool(self.scene.render_cfg.get("global_outline", False)):
            img = add_outline(img, self.scene.color("outline"))
        if debug:
            d = ImageDraw.Draw(img, "RGBA")
            for node_id, box in self.bounds_by_id.items():
                x1, y1, x2, y2 = box
                d.rectangle((x1, y1, x2, y2), outline=(255, 0, 255, 120), width=1)
                d.text((x1, y1 - 9), node_id, fill=(255, 0, 255, 180))
        return img


def fit_on_canvas(
    img,
    size=(512, 512),
    background=True,
    fill_frac=(0.92, 0.82),
    pad=12,
    bg_color=(231, 235, 240, 255),
    center=(0.5, 0.53),
):
    """Crop transparent slack and fit the sprite inside a target frame.

    The mockingbird is a wide flying boss.  Square 256 px frames made the
    alpha bbox only about one third of the canvas height, which forced the
    Rust renderer to scale the whole texture quad up.  Keep this function
    aspect-preserving, but let the scene choose a wide/high-resolution target
    frame and fill fraction so the native pixels are spent on the character.
    """
    img = crop_alpha(img, pad)
    W, H = size
    bg = Image.new("RGBA", size, tuple(bg_color) if background else (0, 0, 0, 0))
    if img.width == 0 or img.height == 0:
        return bg
    scale = min(W * fill_frac[0] / img.width, H * fill_frac[1] / img.height)
    obj = img.resize(
        (max(1, int(img.width * scale)), max(1, int(img.height * scale))),
        Image.Resampling.LANCZOS,
    )
    bg.alpha_composite(
        obj, (int(W * center[0] - obj.width / 2), int(H * center[1] - obj.height / 2))
    )
    return bg


def alpha_metrics(img):
    """Return compact alpha-bbox metrics for generated manifests."""
    bbox = img.getbbox()
    if bbox is None:
        return {"bbox": None, "fill_x": 0.0, "fill_y": 0.0}
    x1, y1, x2, y2 = bbox
    return {
        "bbox": {"x": x1, "y": y1, "w": x2 - x1, "h": y2 - y1},
        "fill_x": round((x2 - x1) / max(1, img.width), 4),
        "fill_y": round((y2 - y1) / max(1, img.height), 4),
    }


def render_work(scene, anim="rest", frame=0, nframes=1, debug=False):
    work = tuple(
        scene.render_cfg.get("work_size", scene.meta.get("work_size", [900, 640]))
    )
    origin = tuple(scene.meta.get("origin", [work[0] / 2, work[1] / 2]))
    return Renderer(
        scene, work, aa_scale=scene.render_cfg.get("supersample", 2), origin=origin
    ).render(anim, frame, nframes, debug=debug)


def build_sheet(scene, frame_size=(256, 256), labeled=False):
    rows = list(scene.animations.items())
    max_frames = max(int(info.get("frames", 1)) for _, info in rows)
    fw, fh = frame_size
    label_w = 120 if labeled else 0
    sheet = Image.new("RGBA", (label_w + fw * max_frames, fh * len(rows)), (0, 0, 0, 0))
    d = ImageDraw.Draw(sheet, "RGBA")
    meta_rows = []
    for r, (name, info) in enumerate(rows):
        nframes = int(info.get("frames", 1))
        dur = int(info.get("duration_ms", 100))
        LOG.info("render row %s (%d frames)", name, nframes)
        if labeled:
            d.rounded_rectangle(
                (8, r * fh + 10, label_w - 10, r * fh + fh - 10),
                radius=8,
                fill=(32, 38, 48, 220),
            )
            d.text((16, r * fh + 16), name, fill=(244, 246, 250, 255), font=font(16))
            d.text(
                (16, r * fh + 38),
                f"{nframes}f @ {dur}ms",
                fill=(176, 184, 196, 255),
                font=font(12),
            )
        rects = []
        for c in range(nframes):
            raw = render_work(scene, name, c, nframes)
            frame = fit_on_canvas(
                raw,
                size=frame_size,
                background=False,
                fill_frac=tuple(scene.render_cfg.get("frame_fill_frac", [0.92, 0.9])),
                pad=16,
                center=tuple(scene.render_cfg.get("frame_center", [0.5, 0.53])),
            )
            x = label_w + c * fw
            y = r * fh
            sheet.alpha_composite(frame, (x, y))
            rects.append({"x": x, "y": y, "w": fw, "h": fh})
            if labeled:
                d.rectangle(
                    (x, y, x + fw - 1, y + fh - 1), outline=(72, 82, 94, 90), width=1
                )
        meta_rows.append(
            {
                "row_index": r,
                "animation": name,
                "frames": nframes,
                "duration_ms": dur,
                "rects": rects,
            }
        )
    return sheet, meta_rows


def make_parts_debug(scene, bg_color=None):
    entries = [("final", crop_alpha(render_work(scene, "rest", 1, 6, debug=True), 10))]

    def walk(n):
        yield n
        for c in n.get("children", []):
            yield from walk(c)

    for node in walk(scene.root):
        if node["id"] == scene.root["id"]:
            continue
        sub = copy.deepcopy(scene.data)
        sub["root"] = copy.deepcopy(node)
        sub["root"]["transform"] = {
            "x": 0,
            "y": 0,
            "rotation": 0,
            "scale_x": 1,
            "scale_y": 1,
        }
        simg = render_work(Scene(sub), "rest", 1, 6)
        entries.append((node["id"], crop_alpha(simg, 8)))
    cols = 3
    cell_w = 260
    cell_h = 190
    rows = math.ceil(len(entries) / cols)
    bg = Image.new(
        "RGBA",
        (cols * cell_w, rows * cell_h),
        tuple(bg_color or scene.background_rgba()),
    )
    d = ImageDraw.Draw(bg, "RGBA")
    for i, (name, img) in enumerate(entries):
        col, row = i % cols, i // cols
        x0 = col * cell_w
        y0 = row * cell_h
        d.rounded_rectangle(
            (x0 + 8, y0 + 8, x0 + cell_w - 8, y0 + cell_h - 8),
            radius=10,
            fill=(247, 249, 252, 255),
            outline=(190, 198, 208, 255),
            width=1,
        )
        d.text((x0 + 16, y0 + 14), name, fill=(40, 46, 55, 255), font=font(14))
        max_w, max_h = cell_w - 28, cell_h - 58
        scale = min(max_w / max(1, img.width), max_h / max(1, img.height))
        obj = img.resize(
            (max(1, int(img.width * scale)), max(1, int(img.height * scale))),
            Image.Resampling.LANCZOS,
        )
        bg.alpha_composite(
            obj, (x0 + (cell_w - obj.width) // 2, y0 + 52 + (max_h - obj.height) // 2)
        )
    return bg


def _union_alpha_bbox(frames):
    """Union of every frame's alpha bbox (frame-local px) → one body box.

    ``frames`` is a list of ``alpha_metrics`` dicts (``{"bbox": {x,y,w,h}}``).
    Returns ``None`` if no frame has opaque pixels."""
    x0 = y0 = x1 = y1 = None
    for fr in frames:
        bb = fr.get("bbox") if isinstance(fr, dict) else None
        if not bb:
            continue
        fx0, fy0 = bb["x"], bb["y"]
        fx1, fy1 = bb["x"] + bb["w"], bb["y"] + bb["h"]
        x0 = fx0 if x0 is None else min(x0, fx0)
        y0 = fy0 if y0 is None else min(y0, fy0)
        x1 = fx1 if x1 is None else max(x1, fx1)
        y1 = fy1 if y1 is None else max(y1, fy1)
    if x0 is None:
        return None
    return {"x": int(x0), "y": int(y0), "w": int(x1 - x0), "h": int(y1 - y0)}


def _runtime_sheet_record(manifest):
    """Build the runtime ``SheetRecord`` dict the game's `SheetRegistry`
    deserializes, from this generator's render ``manifest``.

    The key field is ``body_metrics.body_pixel_bbox``: the union alpha
    bbox across every animation frame, so the boss's *damageable* volume
    (`BossSpriteMetrics::body_pixel_bbox`, the priority-3 hurtbox source
    in `damageable_volumes`) tracks the visible bird instead of falling
    back to the bare, frame-unaligned `combat_size` box — which is what
    let attacks on the visible body whiff. The mockingbird flies, so the
    feet anchor is informational only (the body bbox center drives the
    boss `combat_offset`)."""
    fw = int(manifest["frame_size"]["w"])
    fh = int(manifest["frame_size"]["h"])
    rows = manifest.get("rows", [])
    alpha = manifest.get("alpha_summary", {})
    bbox = _union_alpha_bbox(fr for frames in alpha.values() for fr in frames)
    if bbox is None:
        bbox = {"x": 0, "y": 0, "w": fw, "h": fh}
    feet_x = bbox["x"] + bbox["w"] / 2.0
    feet_y = bbox["y"] + bbox["h"]
    body_metrics = {
        "body_pixel_bbox": bbox,
        "feet_pixel": {"x": float(feet_x), "y": float(feet_y)},
        "feet_anchor_norm": {
            "x": float(feet_x / fw - 0.5),
            "y": float(0.5 - feet_y / fh),
        },
    }
    norm_rows = []
    for r in rows:
        rects = [
            {"x": int(rc["x"]), "y": int(rc["y"]), "w": int(rc["w"]), "h": int(rc["h"])}
            for rc in r.get("rects", [])
        ]
        duration_ms = int(r.get("duration_ms", 100))
        norm_rows.append(
            {
                "animation": r["animation"],
                "row_index": int(r.get("row_index", 0)),
                "frame_count": int(r.get("frame_count", r.get("frames", len(rects)))),
                "duration_ms": duration_ms,
                "duration_secs": round(duration_ms / 1000.0, 6),
                "rects": rects,
            }
        )
    return {
        "target": TARGET_NAME,
        "image": f"{TARGET_NAME}_spritesheet.png",
        "label_width": 0,
        "frame_width": fw,
        "frame_height": fh,
        "rows": norm_rows,
        "body_metrics": body_metrics,
    }


def render_outputs(
    scene_path=DEFAULT_SCENE, outdir=None, frame_size=None, quick=False, force=False
):
    with Timer(f"load scene {scene_path}"):
        scene = Scene.load(scene_path)
    outdir = Path(outdir) if outdir else TOOL_ROOT / "generated" / TARGET_NAME
    outdir.mkdir(parents=True, exist_ok=True)
    if frame_size is None:
        frame_size = tuple(scene.render_cfg.get("default_frame_size", [256, 256]))
    canonical_size = tuple(scene.render_cfg.get("default_canonical_size", [512, 512]))
    outputs = []
    bg_color = scene.background_rgba()
    with Timer("canonical renders"):
        raw = render_work(scene, "rest", 1, 6)
        can = fit_on_canvas(
            raw,
            canonical_size,
            True,
            tuple(scene.render_cfg.get("canonical_fill_frac", [0.94, 0.88])),
            20,
            bg_color=bg_color,
            center=tuple(
                scene.render_cfg.get(
                    "canonical_center",
                    scene.render_cfg.get("frame_center", [0.5, 0.53]),
                )
            ),
        )
        cant = fit_on_canvas(
            raw,
            canonical_size,
            False,
            tuple(scene.render_cfg.get("canonical_fill_frac", [0.94, 0.88])),
            20,
            bg_color=bg_color,
            center=tuple(
                scene.render_cfg.get(
                    "canonical_center",
                    scene.render_cfg.get("frame_center", [0.5, 0.53]),
                )
            ),
        )
        can.save(outdir / f"{TARGET_NAME}_canonical.png")
        outputs.append(outdir / f"{TARGET_NAME}_canonical.png")
        cant.save(outdir / f"{TARGET_NAME}_canonical_transparent.png")
        outputs.append(outdir / f"{TARGET_NAME}_canonical_transparent.png")
    with Timer("parts debug"):
        dbg = make_parts_debug(scene, bg_color=bg_color)
        dbg.save(outdir / f"{TARGET_NAME}_parts_debug.png")
        outputs.append(outdir / f"{TARGET_NAME}_parts_debug.png")
    if not quick:
        with Timer("spritesheet"):
            sheet, rows = build_sheet(scene, frame_size, labeled=False)
            sheet.save(outdir / f"{TARGET_NAME}_spritesheet.png")
            outputs.append(outdir / f"{TARGET_NAME}_spritesheet.png")
        with Timer("labeled preview"):
            preview, _ = build_sheet(scene, frame_size, labeled=True)
            preview.save(outdir / f"{TARGET_NAME}_preview_labeled.png")
            outputs.append(outdir / f"{TARGET_NAME}_preview_labeled.png")
    else:
        rows = []
    alpha_summary = {}
    if not quick:
        sheet_path = outdir / f"{TARGET_NAME}_spritesheet.png"
        sheet_img = Image.open(sheet_path)
        for row in rows:
            frames = []
            for rect in row["rects"]:
                crop = sheet_img.crop(
                    (rect["x"], rect["y"], rect["x"] + rect["w"], rect["y"] + rect["h"])
                )
                frames.append(alpha_metrics(crop))
            alpha_summary[row["animation"]] = frames
    manifest = {
        "target": TARGET_NAME,
        "schema_version": scene.data.get("schema_version"),
        "scene": str(Path(scene_path).name),
        "frame_size": {"w": frame_size[0], "h": frame_size[1]},
        "canonical_size": {"w": canonical_size[0], "h": canonical_size[1]},
        "frame_fill_frac": list(scene.render_cfg.get("frame_fill_frac", [0.92, 0.9])),
        "canonical_fill_frac": list(
            scene.render_cfg.get("canonical_fill_frac", [0.94, 0.88])
        ),
        "quick": quick,
        "source_urls": scene.meta.get("source_urls", []),
        "rows": rows,
        "alpha_summary": alpha_summary,
    }
    (outdir / f"{TARGET_NAME}_spritesheet_manifest.json").write_text(
        json.dumps(manifest, indent=2)
    )
    outputs.append(outdir / f"{TARGET_NAME}_spritesheet_manifest.json")
    if not quick:
        # Runtime sidecar the sandbox's SheetRegistry deserializes. Carries
        # body_metrics (alpha-bbox hurtbox) so the boss is damageable on its
        # visible body — without it, derive_boss_sprite_metrics finds no
        # metrics and the boss falls back to a frame-unaligned combat_size box.
        ron_path = outdir / f"{TARGET_NAME}_spritesheet.ron"
        ron_path.write_text(
            records_to_ron(TARGET_NAME, [_runtime_sheet_record(manifest)]),
            encoding="utf8",
        )
        outputs.append(ron_path)
    lines = ["# Mockingbird boss sprite inspirations", "", "Archival source links:", ""]
    lines += [f"- {u}" for u in scene.meta.get("source_urls", [])]
    lines += [
        "",
        "Notes:",
        "- This generator is nested scene-graph / vector-geometry driven.",
        "- Shape and group transforms are applied before rasterization.",
        "- The global post-composite outline is disabled by default; shapes own their outlines locally.",
        "",
    ]
    (outdir / "sources_and_inspirations.md").write_text("\n".join(lines))
    outputs.append(outdir / "sources_and_inspirations.md")
    return outputs


def install_outputs(render_dir=None, install_dir=None):
    src_dir = Path(render_dir) if render_dir else TOOL_ROOT / "generated" / TARGET_NAME
    repo_root = TOOL_ROOT.parents[1]
    dst_dir = (
        Path(install_dir)
        if install_dir
        else repo_root
        / "crates"
        / "ambition_gameplay_core"
        / "assets"
        / "sprites"
        / TARGET_NAME
    )
    dst_dir.mkdir(parents=True, exist_ok=True)
    copied = []
    for p in src_dir.iterdir():
        if p.is_file():
            dst = dst_dir / p.name
            shutil.copy2(p, dst)
            copied.append(dst)
    return copied


def cli(argv=None):
    p = argparse.ArgumentParser(
        description="Render/install nested scene-graph mockingbird boss sprites"
    )
    p.add_argument(
        "--log-level", default="INFO", choices=["DEBUG", "INFO", "WARNING", "ERROR"]
    )
    sub = p.add_subparsers(dest="cmd", required=True)
    for name in ["render", "preview"]:
        q = sub.add_parser(name)
        q.add_argument("--scene", "--rig", default=str(DEFAULT_SCENE))
        q.add_argument("--outdir", default=None)
        q.add_argument("--frame-width", type=int, default=None)
        q.add_argument("--frame-height", type=int, default=None)
        q.add_argument("--quick", action="store_true")
        q.add_argument("--force", action="store_true")
    q = sub.add_parser("install")
    q.add_argument("--render-dir", default=None)
    q.add_argument("--install-dir", default=None)
    q = sub.add_parser("render-publish")
    q.add_argument("--scene", "--rig", default=str(DEFAULT_SCENE))
    q.add_argument("--outdir", default=None)
    q.add_argument("--install-dir", default=None)
    q.add_argument("--frame-width", type=int, default=None)
    q.add_argument("--frame-height", type=int, default=None)
    q.add_argument("--quick", action="store_true")
    q.add_argument("--force", action="store_true")
    args = p.parse_args(argv)
    logging.basicConfig(
        level=getattr(logging, args.log_level), format="[%(levelname)s] %(message)s"
    )
    if args.cmd in {"render", "preview"}:
        fs = (
            None
            if args.frame_width is None
            else (args.frame_width, args.frame_height or args.frame_width)
        )
        outs = render_outputs(args.scene, args.outdir, fs, args.quick, args.force)
        print("Rendered:")
        print_paths(outs, prefix="  ")
    elif args.cmd == "install":
        outs = install_outputs(args.render_dir, args.install_dir)
        print("Installed:")
        print_paths(outs, prefix="  ")
    elif args.cmd == "render-publish":
        fs = (
            None
            if args.frame_width is None
            else (args.frame_width, args.frame_height or args.frame_width)
        )
        render_outputs(args.scene, args.outdir, fs, args.quick, args.force)
        outs = install_outputs(args.outdir, args.install_dir)
        print("Installed:")
        print_paths(outs, prefix="  ")
    return 0


if __name__ == "__main__":
    raise SystemExit(cli())
