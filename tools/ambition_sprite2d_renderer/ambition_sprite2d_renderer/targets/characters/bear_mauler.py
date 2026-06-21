from __future__ import annotations

"""Standalone side-profile bear enemy generator.

A corrected bear-specific design for side-scroller readability:
- Four-limbed anatomy only: one near foreleg, one far foreleg, one near hindleg, one far hindleg.
- Far limbs are darker and tucked so the silhouette reads as a bear, not an insect rig.
- Rows are gameplay-oriented: swipe, slam, charge, hurt, death.
"""

import argparse
import math
from dataclasses import dataclass
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageDraw

ACTOR_METADATA = {'actor': {'character_id': 'npc_bear_mauler', 'display_name': 'Bear Mauler'},
 'body': {'body_plan': 'Quadruped',
          'body_kind': 'Wide',
          'mass_class': 'Heavy',
          'traits': ['enemy', 'beast', 'no_hands', 'bear', 'mauler'],
          'locomotion_hint': 'Walk'},
 'capabilities': {'traversal': {'walk': True,
                                'jump': None,
                                'climb': None,
                                'fly': None,
                                'swim': None,
                                'crawl': None,
                                'use_lifts': None,
                                'door_access': []},
                  'interactions': {'talk': None, 'trade': None, 'carry': None, 'open_doors': []}},
 'brain': {'default_preset': 'melee_brute_brute'},
 'actions': {'default_preset': 'beast_bite'},
 'visual': {'default_pose': 'idle'},
 'tags': ['enemy', 'beast', 'no_hands', 'bear', 'mauler'],
 'sockets': {'mouth': {'source': 'explicit.profile.beast', 'point': {'x': 96.0, 'y': 54.0}},
             'tail_tip': {'source': 'explicit.profile.beast', 'point': {'x': 28.0, 'y': 66.0}},
             'center': {'source': 'explicit.profile.beast', 'point': {'x': 64.0, 'y': 64.0}}},
 'animation_bindings': {'default': {'animation': 'idle', 'events': []},
                        'locomotion.walk': {'animation': 'walk', 'events': []},
                        'action.melee.primary': {'animation': 'bite',
                                                 'events': [{'t': 0.35,
                                                             'event': 'hitbox_active_start',
                                                             'source': 'explicit.profile.beast'},
                                                            {'t': 0.58,
                                                             'event': 'hitbox_active_end',
                                                             'source': 'explicit.profile.beast'}]}}}


RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_BASENAME = "bear_mauler"
FRAME_SIZE = (240, 224)
WORK_FRAME_SIZE = (480, 448)
SUPER = 4
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 130),
    ("walk", 8, 95),
    ("swipe", 7, 80),
    ("slam", 8, 90),
    ("charge", 7, 75),
    ("hurt", 4, 90),
    ("death", 8, 110),
]

OUTLINE = (22, 18, 16, 255)
FUR_DARK = (58, 40, 30, 255)
FUR = (105, 74, 50, 255)
FUR_LIGHT = (154, 112, 72, 255)
MUZZLE = (205, 174, 126, 255)
MUZZLE_DARK = (136, 96, 66, 255)
NOSE = (24, 22, 22, 255)
CLAW = (238, 230, 204, 255)
EYE = (255, 210, 84, 255)
EYE_HOT = (255, 246, 196, 255)
SHADOW = (0, 0, 0, 42)
DUST = (200, 150, 100, 120)
ARC = (245, 210, 160, 128)


@dataclass
class Pose:
    root_x: float = 0.0
    root_y: float = 0.0
    bob: float = 0.0
    lean: float = 0.0
    head_tilt: float = 0.0
    neck_extend: float = 0.0
    jaw_open: float = 0.0
    hump: float = 0.0
    near_fore: float = 0.0
    far_fore: float = 0.0
    near_hind: float = 0.0
    far_hind: float = 0.0
    near_fore_lift: float = 0.0
    far_fore_lift: float = 0.0
    near_hind_lift: float = 0.0
    far_hind_lift: float = 0.0
    swipe_arc: float = 0.0
    slam_arc: float = 0.0
    dust: float = 0.0
    blink: bool = False
    x_eyes: bool = False

    def __init__(self, anim: str, frame_idx: int, nframes: int):
        t = frame_idx / max(1, nframes - 1)
        cyc = math.tau * frame_idx / max(1, nframes)
        s = math.sin(cyc)
        c = math.cos(cyc)

        self.root_x = 0.0
        self.root_y = 0.0
        self.bob = 0.0
        self.lean = 0.0
        self.head_tilt = 0.0
        self.neck_extend = 0.0
        self.jaw_open = 0.0
        self.hump = 0.0
        self.near_fore = 0.0
        self.far_fore = 0.0
        self.near_hind = 0.0
        self.far_hind = 0.0
        self.near_fore_lift = 0.0
        self.far_fore_lift = 0.0
        self.near_hind_lift = 0.0
        self.far_hind_lift = 0.0
        self.swipe_arc = 0.0
        self.slam_arc = 0.0
        self.dust = 0.0
        self.blink = False
        self.x_eyes = False

        if anim == "idle":
            self.bob = s * 1.4
            self.lean = s * 0.8
            self.head_tilt = -s * 1.0
            self.hump = abs(s) * 2.0
            self.near_fore = 4.0 + s * 2.0
            self.far_fore = -5.0 - s * 1.4
            self.near_hind = c * 0.8
            self.far_hind = -c * 0.8
            self.jaw_open = max(0.0, s) * 0.04
            self.blink = frame_idx == nframes - 2
        elif anim == "walk":
            self.root_x = s * 2.5
            self.bob = abs(s) * 2.8 - 0.8
            self.lean = s * 1.6
            self.head_tilt = -s * 1.3
            self.near_fore = 18.0 * s
            self.far_fore = -16.0 * s
            self.near_hind = -18.0 * s
            self.far_hind = 16.0 * s
            self.near_fore_lift = max(0.0, s) * 8.0
            self.far_fore_lift = max(0.0, -s) * 5.0
            self.near_hind_lift = max(0.0, -s) * 7.0
            self.far_hind_lift = max(0.0, s) * 4.5
        elif anim == "swipe":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-6.0, 8.0, tt)
            self.bob = -hit * 4.0
            self.lean = _lerp(-8.0, 14.0, tt)
            self.head_tilt = _lerp(-6.0, 8.0, tt)
            self.neck_extend = hit * 6.0
            self.near_fore = _lerp(-72.0, 34.0, tt)
            self.far_fore = _lerp(-18.0, 8.0, tt)
            self.near_fore_lift = _lerp(34.0, 0.0, tt) + hit * 8.0
            self.near_hind = -8.0 - hit * 3.0
            self.far_hind = 8.0 + hit * 2.0
            self.jaw_open = 0.18 * hit
            self.swipe_arc = hit
        elif anim == "slam":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            rear = math.sin(tt * math.pi * 0.8)
            self.root_x = _lerp(-4.0, 5.0, tt)
            self.root_y = -rear * 12.0
            self.bob = -hit * 2.5
            self.lean = _lerp(-22.0, 26.0, tt)
            self.head_tilt = _lerp(-12.0, 12.0, tt)
            self.near_fore = _lerp(-112.0, 52.0, tt)
            self.far_fore = _lerp(-96.0, 38.0, tt)
            self.near_fore_lift = _lerp(60.0, 0.0, tt) + hit * 8.0
            self.far_fore_lift = _lerp(44.0, 0.0, tt)
            self.near_hind = 18.0 - hit * 4.0
            self.far_hind = -12.0 + hit * 2.0
            self.jaw_open = 0.24 * hit
            self.slam_arc = hit
            self.dust = max(0.0, tt - 0.55) * 2.0
        elif anim == "charge":
            tt = _ease(t)
            pulse = math.sin(t * math.pi * 2.0)
            self.root_x = _lerp(-10.0, 16.0, tt)
            self.bob = abs(pulse) * 2.0 - 1.0
            self.lean = 14.0 + pulse * 2.0
            self.head_tilt = 8.0
            self.neck_extend = 12.0 + tt * 8.0
            self.near_fore = -18.0 + pulse * 18.0
            self.far_fore = 20.0 - pulse * 16.0
            self.near_hind = 18.0 - pulse * 18.0
            self.far_hind = -18.0 + pulse * 16.0
            self.near_fore_lift = max(0.0, pulse) * 7.0
            self.near_hind_lift = max(0.0, -pulse) * 7.0
            self.jaw_open = 0.08
            self.dust = 0.5 + abs(pulse) * 0.4
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 4.0) * (1.0 - t)
            self.root_x = shake * 4.0
            self.bob = -hit * 2.4
            self.lean = -14.0 * hit
            self.head_tilt = 18.0 * hit
            self.jaw_open = 0.22 * hit
            self.near_fore = 22.0 * hit
            self.far_fore = 16.0 * hit
            self.near_hind = 8.0 * hit
            self.far_hind = -6.0 * hit
        elif anim == "death":
            tt = _ease(t)
            self.root_x = tt * 18.0
            self.root_y = tt * 8.0
            self.bob = -tt * 3.0
            self.lean = -82.0 * tt
            self.head_tilt = 28.0 * tt
            self.near_fore = _lerp(4.0, 64.0, tt)
            self.far_fore = _lerp(-5.0, -48.0, tt)
            self.near_hind = _lerp(0.0, 30.0, tt)
            self.far_hind = _lerp(0.0, -24.0, tt)
            self.near_fore_lift = tt * 8.0
            self.far_fore_lift = tt * 5.0
            self.near_hind_lift = tt * 6.0
            self.jaw_open = 0.25 * tt
            self.x_eyes = tt > 0.55
            self.dust = max(0.0, tt - 0.5)


def _s(v: float) -> int:
    return int(round(v * SUPER))


def _pt(p: Point) -> Tuple[int, int]:
    return (_s(p[0]), _s(p[1]))


def _box(cx: float, cy: float, rx: float, ry: float) -> Tuple[int, int, int, int]:
    return (_s(cx - rx), _s(cy - ry), _s(cx + rx), _s(cy + ry))


def _lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def _ease(t: float) -> float:
    t = max(0.0, min(1.0, t))
    return 0.5 - 0.5 * math.cos(math.pi * t)


def _rot_local(x: float, y: float, deg: float) -> Point:
    rad = math.radians(deg)
    c = math.cos(rad)
    s = math.sin(rad)
    return (x * c - y * s, x * s + y * c)


def _poly(draw: ImageDraw.ImageDraw, pts: Sequence[Point], fill: RGBA, outline: RGBA = OUTLINE, width: float = 1.0) -> None:
    ipts = [_pt(p) for p in pts]
    draw.polygon(ipts, fill=fill)
    if outline and width > 0:
        draw.line(ipts + [ipts[0]], fill=outline, width=max(1, _s(width)), joint="curve")


def _line(draw: ImageDraw.ImageDraw, pts: Sequence[Point], fill: RGBA, width: float = 1.0) -> None:
    draw.line([_pt(p) for p in pts], fill=fill, width=max(1, _s(width)), joint="curve")


def _ellipse(draw: ImageDraw.ImageDraw, cx: float, cy: float, rx: float, ry: float, fill: RGBA, outline: RGBA = OUTLINE, width: float = 1.0) -> None:
    draw.ellipse(_box(cx, cy, rx, ry), fill=fill, outline=outline, width=max(1, _s(width)))


def _circle(draw: ImageDraw.ImageDraw, c: Point, r: float, fill: RGBA, outline: RGBA = OUTLINE, width: float = 1.0) -> None:
    _ellipse(draw, c[0], c[1], r, r, fill, outline, width)


def _downsample(img: Image.Image) -> Image.Image:
    return img.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


class BearMaulerRenderer:
    def render_frame(self, anim: str, frame_idx: int, nframes: int) -> Image.Image:
        img = Image.new("RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0))
        draw = ImageDraw.Draw(img, "RGBA")
        pose = Pose(anim, frame_idx, nframes)

        root = (WORK_FRAME_SIZE[0] * 0.42 + pose.root_x, WORK_FRAME_SIZE[1] * 0.78 + pose.root_y + pose.bob)
        tilt = pose.lean

        def P(x: float, y: float) -> Point:
            rx, ry = _rot_local(x, y, tilt)
            return (root[0] + rx, root[1] + ry)

        # No baked ground drop shadow; the scene renderer owns contact shadows.
        self._draw_far_limbs(draw, P, pose)
        self._draw_body(draw, P, pose)
        self._draw_head(draw, P, pose)
        self._draw_near_limbs(draw, P, pose)
        if anim == "swipe" and pose.swipe_arc > 0.18:
            self._draw_swipe_fx(draw, P, pose)
        if anim == "slam" and pose.slam_arc > 0.2:
            self._draw_slam_fx(draw, P, pose)
        if pose.dust > 0.1:
            self._draw_dust(draw, P, pose)
        return _downsample(img)

    def _draw_shadow(self, draw, P):
        c = P(-8, 16)
        _ellipse(draw, c[0], c[1], 62, 12, SHADOW, outline=(0, 0, 0, 0), width=0)

    def _draw_body(self, draw, P, pose):
        back = [P(-76, -70), P(-42, -111 - pose.hump), P(22, -119 - pose.hump), P(72, -92), P(90, -58), P(66, -28), P(8, -20), P(-58, -28), P(-92, -48)]
        _poly(draw, back, FUR_DARK, OUTLINE, 1.8)
        barrel = [P(-64, -68), P(-28, -99 - pose.hump * 0.7), P(38, -97), P(78, -68), P(68, -36), P(16, -26), P(-48, -34), P(-78, -52)]
        _poly(draw, barrel, FUR, OUTLINE, 1.4)
        flank = [P(-36, -66), P(-6, -82), P(40, -74), P(54, -52), P(26, -38), P(-26, -44)]
        _poly(draw, flank, FUR_LIGHT, OUTLINE, 0.9)
        _line(draw, [P(-34, -82), P(-20, -44)], FUR_DARK, 0.9)
        _line(draw, [P(22, -78), P(34, -42)], FUR_DARK, 0.9)
        _poly(draw, [P(-88, -58), P(-102, -62), P(-96, -47)], FUR_DARK, OUTLINE, 0.8)

    def _draw_head(self, draw, P, pose):
        hx, hy = P(82 + pose.neck_extend, -92 + pose.head_tilt * 0.12)
        neck = [P(44, -90), P(72 + pose.neck_extend * 0.3, -104), P(84 + pose.neck_extend * 0.2, -72), P(46, -62)]
        _poly(draw, neck, FUR_DARK, OUTLINE, 1.0)
        head = [(hx - 28, hy - 20), (hx + 12, hy - 26), (hx + 42, hy - 12), (hx + 48, hy + 8), (hx + 20, hy + 22), (hx - 22, hy + 18), (hx - 34, hy - 2)]
        _poly(draw, head, FUR, OUTLINE, 1.3)
        _circle(draw, (hx - 18, hy - 20), 9, FUR_DARK, OUTLINE, 1.0)
        _circle(draw, (hx + 8, hy - 24), 8, FUR_DARK, OUTLINE, 1.0)
        snout = [(hx + 14, hy - 4), (hx + 54, hy - 1), (hx + 68, hy + 8), (hx + 52, hy + 18), (hx + 16, hy + 16)]
        _poly(draw, snout, MUZZLE, OUTLINE, 1.0)
        lower_drop = pose.jaw_open * 20.0
        lower = [(hx + 18, hy + 14), (hx + 48, hy + 18 + lower_drop), (hx + 62, hy + 14 + lower_drop), (hx + 48, hy + 25 + lower_drop), (hx + 18, hy + 23)]
        _poly(draw, lower, MUZZLE_DARK, OUTLINE, 0.8)
        _circle(draw, (hx + 58, hy + 6), 5, NOSE, OUTLINE, 0.6)
        if pose.x_eyes:
            _line(draw, [(hx + 2, hy - 7), (hx + 12, hy + 3)], OUTLINE, 1.0)
            _line(draw, [(hx + 2, hy + 3), (hx + 12, hy - 7)], OUTLINE, 1.0)
        elif pose.blink:
            _line(draw, [(hx + 1, hy - 4), (hx + 13, hy - 4)], EYE_HOT, 1.0)
        else:
            _ellipse(draw, hx + 7, hy - 4, 5, 3.5, EYE, EYE_HOT, 0.7)
            _circle(draw, (hx + 8, hy - 4), 1.3, OUTLINE, OUTLINE, 0.4)
        for x in (34, 44, 54):
            _line(draw, [(hx + x, hy + 13), (hx + x - 3, hy + 19 + lower_drop * 0.2)], CLAW, 0.7)

    def _limb_points(self, P, kind, phase, lift):
        if kind == "near_fore":
            upper = P(46, -50)
            joint = P(54 + phase * 0.22, -22 + phase * 0.10 - lift * 0.45)
            paw = P(60 + phase * 0.18, 14 - lift)
        elif kind == "far_fore":
            upper = P(24, -54)
            joint = P(29 + phase * 0.18, -20 + phase * 0.10 - lift * 0.35)
            paw = P(34 + phase * 0.16, 12 - lift)
        elif kind == "near_hind":
            upper = P(-44, -36)
            joint = P(-36 + phase * 0.18, -4 - lift * 0.25)
            paw = P(-22 + phase * 0.15, 15 - lift)
        else:
            upper = P(-64, -38)
            joint = P(-66 + phase * 0.16, -6 - lift * 0.18)
            paw = P(-56 + phase * 0.14, 13 - lift)
        return upper, joint, paw

    def _draw_limb(self, draw, upper, joint, paw, front: bool, is_fore: bool):
        base = FUR if front else FUR_DARK
        hi = FUR_LIGHT if front else FUR
        width = 9.0 if front else 7.0
        _line(draw, [upper, joint], base, width)
        _line(draw, [joint, paw], base, width - 1.0)
        _line(draw, [upper, joint, paw], OUTLINE, 2.0 if front else 1.4)
        _ellipse(draw, joint[0], joint[1], 7.0 if front else 5.8, 8.0 if front else 6.4, hi, OUTLINE, 0.9)
        _ellipse(draw, paw[0], paw[1], 13.0 if front else 10.0, 6.0 if front else 5.0, base, OUTLINE, 1.0)
        for dy in (-3, 0, 3):
            _line(draw, [(paw[0] + 8, paw[1] + dy), (paw[0] + 18, paw[1] + dy - 1)], CLAW, 1.2 if front else 0.9)
        if is_fore and front:
            _line(draw, [(paw[0] - 6, paw[1] - 5), (paw[0] + 6, paw[1] - 5)], FUR_LIGHT, 0.9)

    def _draw_far_limbs(self, draw, P, pose):
        self._draw_limb(draw, *self._limb_points(P, "far_hind", pose.far_hind, pose.far_hind_lift), front=False, is_fore=False)
        self._draw_limb(draw, *self._limb_points(P, "far_fore", pose.far_fore, pose.far_fore_lift), front=False, is_fore=True)

    def _draw_near_limbs(self, draw, P, pose):
        self._draw_limb(draw, *self._limb_points(P, "near_hind", pose.near_hind, pose.near_hind_lift), front=True, is_fore=False)
        self._draw_limb(draw, *self._limb_points(P, "near_fore", pose.near_fore, pose.near_fore_lift), front=True, is_fore=True)

    def _draw_swipe_fx(self, draw, P, pose):
        cx, cy = P(88, -34)
        box = (_s(cx - 72), _s(cy - 70), _s(cx + 70), _s(cy + 70))
        draw.arc(box, 212, 334, fill=ARC, width=_s(5.0 + pose.swipe_arc * 2.0))
        draw.arc(box, 224, 322, fill=(255, 246, 218, 100), width=_s(2.0))

    def _draw_slam_fx(self, draw, P, pose):
        c = P(62, 12)
        _ellipse(draw, c[0], c[1], 34 + pose.slam_arc * 10, 6 + pose.slam_arc * 2, DUST, outline=(0, 0, 0, 0), width=0)
        for dx in (-22, -8, 8, 22):
            _poly(draw, [P(62 + dx, 9), P(70 + dx, -2), P(78 + dx, 9)], (220, 180, 130, 155), (160, 110, 80, 130), 0.5)

    def _draw_dust(self, draw, P, pose):
        for dx, dy, rx in [(-48, 14, 8), (-20, 16, 10), (28, 15, 9)]:
            c = P(dx, dy)
            _ellipse(draw, c[0], c[1], rx + pose.dust * 7, 4 + pose.dust * 3, DUST, outline=(0, 0, 0, 0), width=0)


def _write_yaml(path: Path) -> None:
    lines = [f"target: {TARGET_BASENAME}", f"frame_width: {FRAME_SIZE[0]}", f"frame_height: {FRAME_SIZE[1]}", "rows:"]
    for name, frames, ms in ROWS:
        lines.extend([f"  - name: {name}", f"    frames: {frames}", f"    frame_ms: {ms}"])
    path.write_text("\n".join(lines) + "\n")


def _write_ron(path: Path) -> None:
    row_lines = [f'        (name: "{name}", frames: {frames}, frame_ms: {ms}),' for name, frames, ms in ROWS]
    ron = ["(", f'    target: "{TARGET_BASENAME}",', f'    frame_width: {FRAME_SIZE[0]},', f'    frame_height: {FRAME_SIZE[1]},', "    rows: [", *row_lines, "    ],", ")"]
    path.write_text("\n".join(ron) + "\n")


def _render_sheet(renderer: BearMaulerRenderer, out_dir: Path):
    fw, fh = FRAME_SIZE
    sheet_w = max(frames for _, frames, _ in ROWS) * fw
    sheet_h = len(ROWS) * fh
    sheet = Image.new("RGBA", (sheet_w, sheet_h), (0, 0, 0, 0))
    preview = Image.new("RGBA", (sheet_w + 128, sheet_h), (248, 246, 242, 255))
    pdraw = ImageDraw.Draw(preview)
    canonical = None
    for row_idx, (name, nframes, _ms) in enumerate(ROWS):
        pdraw.text((8, row_idx * fh + 8), name, fill=(36, 36, 36, 255))
        for frame_idx in range(nframes):
            frame = renderer.render_frame(name, frame_idx, nframes)
            x = frame_idx * fw
            y = row_idx * fh
            sheet.alpha_composite(frame, (x, y))
            preview.alpha_composite(frame, (x + 128, y))
            if canonical is None and name == "idle" and frame_idx == 0:
                canonical = frame
    if canonical is None:
        canonical = renderer.render_frame(ROWS[0][0], 0, ROWS[0][1])
    spritesheet_path = out_dir / f"{TARGET_BASENAME}.png"
    yaml_path = out_dir / f"{TARGET_BASENAME}.yaml"
    ron_path = out_dir / f"{TARGET_BASENAME}.ron"
    preview_path = out_dir / f"{TARGET_BASENAME}_preview_labeled.png"
    canonical_path = out_dir / f"{TARGET_BASENAME}_canonical.png"
    sheet.save(spritesheet_path)
    preview.save(preview_path)
    canonical.save(canonical_path)
    _write_yaml(yaml_path)
    _write_ron(ron_path)
    return [spritesheet_path, yaml_path, ron_path, preview_path, canonical_path]


def render(out_dir: str | Path, **opts):
    """Render the bear_mauler spritesheet bundle via the shared
    `tackon_sheet.build_sheet` pipeline.

    Routes through the standard auto-cropped + manifested pipeline so
    the sheet (a) gets the union-bbox crop that every other tack-on
    character gets (frames keep uniform dimensions so poses stay
    aligned within a row, and the sheet sheds the ~50% transparent
    margin the bespoke 240×224 layout used to bake in), and (b) emits
    the `body_metrics` + per-row `rects` shape the sandbox's
    SheetRegistry parses at runtime. The bespoke `_render_sheet` +
    `_write_yaml` + `_write_ron` helpers stay below for standalone-CLI
    use, but discovery routes through here.
    """
    from ...authoring.tackon_sheet import build_sheet
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    renderer = BearMaulerRenderer()
    outputs = build_sheet(
        target=TARGET_BASENAME,
        rows=ROWS,
        render_fn=renderer.render_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        auto_crop=True,
    )
    return [
        outputs["spritesheet"], outputs["yaml"], outputs["ron"],
        outputs["preview"], outputs["canonical"], outputs["canonical_transparent"],
    ]


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Render a corrected four-limbed side-profile bear mauler enemy spritesheet.")
    parser.add_argument("--out-dir", type=Path, default=Path(__file__).resolve().parents[2] / "generated" / TARGET_BASENAME)
    args = parser.parse_args(argv)
    for path in render(args.out_dir):
        print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
