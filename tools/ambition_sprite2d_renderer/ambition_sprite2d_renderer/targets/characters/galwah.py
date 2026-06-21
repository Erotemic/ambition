#!/usr/bin/env python3
"""Procedurally generate a GalWah / Galois-inspired spritesheet.

This script is intentionally pure code: no embedded PNG data, no base64 image
payloads, and no dependence on any repo-specific framework.  It uses Pillow to
construct a multi-row spritesheet from drawing primitives.

The design target is a distinct, young, intense romantic-era mathematician with
messy dark hair, a narrow face, a coat + cravat silhouette, manuscript poses,
and duel / death poses.
"""

from __future__ import annotations

import argparse
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence

from PIL import Image, ImageDraw, ImageFont

ACTOR_METADATA = {
    "actor": {"character_id": "npc_galwah", "display_name": "Galwah"},
    "body": {
        "body_plan": "HumanoidBiped",
        "body_kind": "Standard",
        "mass_class": "Medium",
        "traits": ["story", "humanoid", "story", "mystic"],
        "locomotion_hint": "Walk",
    },
    "capabilities": {
        "traversal": {
            "walk": True,
            "jump": None,
            "climb": None,
            "fly": None,
            "swim": None,
            "crawl": None,
            "use_lifts": True,
            "door_access": ["public"],
        },
        "interactions": {
            "talk": True,
            "trade": None,
            "carry": None,
            "open_doors": ["public"],
        },
    },
    "brain": {"default_preset": "patrol_peaceful"},
    "actions": {"default_preset": "peaceful"},
    "visual": {"default_pose": "idle"},
    "tags": ["story", "humanoid", "story", "mystic"],
    "sockets": {
        "head": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 64.0, "y": 24.0},
        },
        "chest": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 64.0, "y": 54.0},
        },
        "hand_l": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 48.0, "y": 64.0},
        },
        "hand_r": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 80.0, "y": 64.0},
        },
        "speech_bubble": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 64.0, "y": 8.0},
        },
    },
    "animation_bindings": {
        "default": {"animation": "turn", "events": []},
        "locomotion.walk": {"animation": "walk", "events": []},
        "interaction.talk": {"animation": "talk", "events": []},
        "interaction.use": {"animation": "interact", "events": []},
    },
}


# --- palette -----------------------------------------------------------------
BG = (0, 0, 0, 0)
OUTLINE = (28, 24, 24, 255)
HAIR = (55, 42, 36, 255)
HAIR_LIGHT = (92, 72, 62, 255)
SKIN = (226, 199, 176, 255)
SKIN_SHADE = (204, 176, 156, 255)
CHEEK = (214, 162, 156, 84)
COAT = (45, 52, 76, 255)
COAT_DARK = (31, 37, 58, 255)
VEST = (72, 72, 94, 255)
TROUSER = (56, 54, 60, 255)
CRAVAT = (238, 238, 238, 255)
SHIRT = (245, 243, 236, 255)
BOOT = (44, 34, 28, 255)
PAPER = (236, 228, 199, 255)
PAPER_LINE = (125, 100, 75, 255)
METAL = (110, 116, 122, 255)
WOOD = (110, 76, 46, 255)
RED = (156, 60, 58, 255)
SHADOW = (0, 0, 0, 36)
PREVIEW_BG = (243, 240, 233, 255)
LABEL = (55, 52, 58, 255)


# --- utility -----------------------------------------------------------------


def lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def clamp(v: float, lo: float, hi: float) -> float:
    return max(lo, min(hi, v))


def add(a: tuple[float, float], b: tuple[float, float]) -> tuple[float, float]:
    return (a[0] + b[0], a[1] + b[1])


def sub(a: tuple[float, float], b: tuple[float, float]) -> tuple[float, float]:
    return (a[0] - b[0], a[1] - b[1])


def mul(a: tuple[float, float], k: float) -> tuple[float, float]:
    return (a[0] * k, a[1] * k)


def midpoint(a: tuple[float, float], b: tuple[float, float]) -> tuple[float, float]:
    return ((a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5)


def length(v: tuple[float, float]) -> float:
    return math.hypot(v[0], v[1])


def normalize(v: tuple[float, float]) -> tuple[float, float]:
    n = length(v)
    if n < 1e-6:
        return (0.0, 0.0)
    return (v[0] / n, v[1] / n)


def perp(v: tuple[float, float]) -> tuple[float, float]:
    return (-v[1], v[0])


def rotate(v: tuple[float, float], ang: float) -> tuple[float, float]:
    c = math.cos(ang)
    s = math.sin(ang)
    return (v[0] * c - v[1] * s, v[0] * s + v[1] * c)


def qcurve(
    a: tuple[float, float], b: tuple[float, float], c: tuple[float, float], n: int = 12
) -> list[tuple[float, float]]:
    pts = []
    for i in range(n + 1):
        t = i / n
        mt = 1.0 - t
        x = mt * mt * a[0] + 2 * mt * t * b[0] + t * t * c[0]
        y = mt * mt * a[1] + 2 * mt * t * b[1] + t * t * c[1]
        pts.append((x, y))
    return pts


# --- primitives ---------------------------------------------------------------


def polygon(
    draw: ImageDraw.ImageDraw,
    pts: Sequence[tuple[float, float]],
    fill,
    outline=None,
    width: int = 1,
) -> None:
    draw.polygon([(round(x), round(y)) for x, y in pts], fill=fill, outline=outline)
    if outline is not None and width > 1:
        loop = list(pts) + [pts[0]]
        draw.line([(round(x), round(y)) for x, y in loop], fill=outline, width=width)


def line(
    draw: ImageDraw.ImageDraw, pts: Sequence[tuple[float, float]], fill, width: int = 1
) -> None:
    draw.line(
        [(round(x), round(y)) for x, y in pts], fill=fill, width=width, joint="curve"
    )


def ellipse(
    draw: ImageDraw.ImageDraw,
    center: tuple[float, float],
    rx: float,
    ry: float,
    fill,
    outline=None,
    width: int = 1,
) -> None:
    cx, cy = center
    draw.ellipse(
        (round(cx - rx), round(cy - ry), round(cx + rx), round(cy + ry)),
        fill=fill,
        outline=outline,
        width=width,
    )


def circle(
    draw: ImageDraw.ImageDraw,
    center: tuple[float, float],
    r: float,
    fill,
    outline=None,
    width: int = 1,
) -> None:
    ellipse(draw, center, r, r, fill, outline, width)


def stroke_polyline(
    draw: ImageDraw.ImageDraw, pts: Sequence[tuple[float, float]], color, width: float
) -> None:
    """Thick line with rounded joints via line + joint circles."""
    w = max(1, round(width))
    line(draw, pts, color, w)
    for p in pts:
        circle(draw, p, max(1.0, width * 0.48), color)


# --- pose model ---------------------------------------------------------------
@dataclass
class Pose:
    name: str
    facing: int = 1  # 1=right, -1=left
    yaw: float = 0.0  # 0 front, 1 side
    lean: float = 0.0  # torso lean
    head_tilt: float = 0.0
    blink: bool = False
    mouth: float = 0.0  # -1 frown, +1 open / talk
    step: float = 0.0  # walk cycle phase-ish
    body_y: float = 0.0
    crouch: float = 0.0
    left_hand: tuple[float, float] | None = None
    right_hand: tuple[float, float] | None = None
    left_foot: tuple[float, float] | None = None
    right_foot: tuple[float, float] | None = None
    left_item: str | None = None
    right_item: str | None = None
    coat_swing: float = 0.0
    paper_drop: float = 0.0
    pistol_smoke: bool = False
    hair_bounce: float = 0.0
    collapse: float = 0.0  # 0 standing, 1 collapsed


# --- figure drawing -----------------------------------------------------------
class GalwahRenderer:
    def __init__(self, frame_w: int = 96, frame_h: int = 96, aa: int = 4):
        self.frame_w = frame_w
        self.frame_h = frame_h
        self.aa = aa
        self.W = frame_w * aa
        self.H = frame_h * aa

    def S(self, x: float) -> float:
        return x * self.aa

    def pt(self, x: float, y: float) -> tuple[float, float]:
        return (self.S(x), self.S(y))

    def render_pose(self, pose: Pose) -> Image.Image:
        img = Image.new("RGBA", (self.W, self.H), BG)
        draw = ImageDraw.Draw(img, "RGBA")

        # Shadow / ground anchor.
        ellipse(
            draw,
            self.pt(48, 88 + pose.body_y * 0.18),
            self.S(18 - 8 * pose.collapse),
            self.S(4 + 2 * pose.collapse),
            SHADOW,
        )

        # Body scaffold.
        facing = 1 if pose.facing >= 0 else -1
        yaw = clamp(pose.yaw, 0.0, 1.0)
        front = 1.0 - yaw
        lean = pose.lean
        body_y = pose.body_y
        crouch = pose.crouch

        pelvis = self.pt(48 + lean * 1.8, 58 + body_y + crouch * 3 + pose.collapse * 8)
        chest = self.pt(48 + lean * 4.2, 41 + body_y + crouch * 1.8 + pose.collapse * 2)
        neck = self.pt(48 + lean * 4.8, 33 + body_y + crouch * 1.1)
        head = self.pt(
            48 + facing * yaw * 3.0 + lean * 4.5, 24 + body_y - pose.collapse * 2
        )

        shoulder_span = lerp(10.5, 5.5, yaw)
        hip_span = lerp(6.8, 4.2, yaw)
        left_shoulder = self.pt(
            48 - shoulder_span + lean * 3.6, 42 + body_y + crouch * 1.5
        )
        right_shoulder = self.pt(
            48 + shoulder_span + lean * 4.2, 42 + body_y + crouch * 1.5
        )
        left_hip = self.pt(
            48 - hip_span + lean * 1.8, 58 + body_y + crouch * 2.8 + pose.collapse * 8
        )
        right_hip = self.pt(
            48 + hip_span + lean * 1.8, 58 + body_y + crouch * 2.8 + pose.collapse * 8
        )

        # Near/far sides for draw order.
        near_side = "right" if facing > 0 else "left"
        far_side = "left" if near_side == "right" else "right"

        # Default locomotion endpoints.
        walk = pose.step
        if pose.left_foot is None:
            lf = (-7 - walk * 2.5, 29 - abs(walk) * 1.0 + pose.collapse * 7)
        else:
            lf = pose.left_foot
        if pose.right_foot is None:
            rf = (7 + walk * 2.5, 29 - abs(walk * 0.5) * 0.5 + pose.collapse * 6)
        else:
            rf = pose.right_foot

        if pose.left_hand is None:
            lh = (-15 - walk * 3.5, 7 + abs(walk) * 1.0)
        else:
            lh = pose.left_hand
        if pose.right_hand is None:
            rh = (15 + walk * 3.5, 8 + abs(walk) * 1.0)
        else:
            rh = pose.right_hand

        left_foot = add(pelvis, self.pt(lf[0], lf[1]))
        right_foot = add(pelvis, self.pt(rf[0], rf[1]))
        left_hand = add(chest, self.pt(lh[0], lh[1]))
        right_hand = add(chest, self.pt(rh[0], rh[1]))

        # Knees / elbows.
        def knee(
            a: tuple[float, float],
            b: tuple[float, float],
            side: float,
            bias: float = 1.0,
        ) -> tuple[float, float]:
            mid = midpoint(a, b)
            d = sub(b, a)
            n = normalize(perp(d))
            return add(mid, mul(n, self.S(5.0 * side * bias)))

        def elbow(
            a: tuple[float, float],
            b: tuple[float, float],
            side: float,
            bias: float = 1.0,
        ) -> tuple[float, float]:
            mid = midpoint(a, b)
            d = sub(b, a)
            n = normalize(perp(d))
            return add(mid, mul(n, self.S(5.0 * side * bias)))

        left_knee = knee(left_hip, left_foot, -0.8, 1.0 - 0.4 * pose.collapse)
        right_knee = knee(right_hip, right_foot, 0.8, 1.0 - 0.4 * pose.collapse)
        left_elbow = elbow(left_shoulder, left_hand, -0.8 * facing, 1.0)
        right_elbow = elbow(right_shoulder, right_hand, 0.8 * facing, 1.0)

        if pose.collapse > 0.25:
            left_elbow = add(left_elbow, self.pt(-2 * pose.collapse, 2 * pose.collapse))
            right_elbow = add(
                right_elbow, self.pt(2 * pose.collapse, 3 * pose.collapse)
            )

        # Draw order: far leg, near leg, torso, far arm, head, near arm, items.
        self._draw_leg(
            draw,
            left_hip if far_side == "left" else right_hip,
            left_knee if far_side == "left" else right_knee,
            left_foot if far_side == "left" else right_foot,
            far=True,
        )
        self._draw_leg(
            draw,
            right_hip if near_side == "right" else left_hip,
            right_knee if near_side == "right" else left_knee,
            right_foot if near_side == "right" else left_foot,
            far=False,
        )
        self._draw_torso(draw, pelvis, chest, neck, pose, facing, yaw)
        self._draw_arm(
            draw,
            left_shoulder if far_side == "left" else right_shoulder,
            left_elbow if far_side == "left" else right_elbow,
            left_hand if far_side == "left" else right_hand,
            far=True,
        )
        self._draw_head(draw, head, pose, facing, yaw)
        self._draw_arm(
            draw,
            right_shoulder if near_side == "right" else left_shoulder,
            right_elbow if near_side == "right" else left_elbow,
            right_hand if near_side == "right" else left_hand,
            far=False,
        )

        # Items.
        if pose.left_item:
            self._draw_item(draw, left_hand, pose.left_item, facing, pose)
        if pose.right_item:
            self._draw_item(draw, right_hand, pose.right_item, facing, pose)

        if pose.paper_drop > 0.0:
            p = self.pt(55 + 12 * pose.paper_drop, 56 + 22 * pose.paper_drop)
            self._draw_dropped_page(draw, p, 0.35 + pose.paper_drop * 1.2)

        if pose.pistol_smoke:
            base = add(right_hand, self.pt(8 * facing, -5))
            for i in range(4):
                off = rotate(self.pt(5 + i * 2.2, -4 - i * 1.5), -0.2 * facing)
                circle(
                    draw,
                    add(base, off),
                    self.S(2.2 - i * 0.3),
                    (230, 230, 230, 92 - i * 14),
                )

        # Downsample from supersampled render.
        return img.resize((self.frame_w, self.frame_h), Image.Resampling.LANCZOS)

    def _draw_leg(self, draw: ImageDraw.ImageDraw, hip, knee, foot, far: bool) -> None:
        trouser = COAT_DARK if far else TROUSER
        stroke_polyline(draw, [hip, knee, foot], trouser, self.S(4.6 if far else 5.6))
        # boot silhouette
        toe = add(foot, self.pt(4.8, 0.6))
        heel = add(foot, self.pt(-2.2, 0.8))
        upper = add(foot, self.pt(0.0, -2.6))
        polygon(
            draw,
            [
                upper,
                toe,
                add(toe, self.pt(0.4, 1.4)),
                add(heel, self.pt(0.0, 1.8)),
                heel,
            ],
            BOOT,
            OUTLINE,
            max(1, self.aa // 2),
        )

    def _draw_arm(
        self, draw: ImageDraw.ImageDraw, shoulder, elbow, hand, far: bool
    ) -> None:
        sleeve = (58, 64, 90, 235) if far else COAT
        stroke_polyline(
            draw, [shoulder, elbow, hand], sleeve, self.S(4.2 if far else 5.0)
        )
        circle(draw, hand, self.S(2.2), SKIN, OUTLINE, max(1, self.aa // 2))

    def _draw_torso(
        self,
        draw: ImageDraw.ImageDraw,
        pelvis,
        chest,
        neck,
        pose: Pose,
        facing: int,
        yaw: float,
    ) -> None:
        cx, cy = chest
        px, py = pelvis
        coat_swing = pose.coat_swing
        shoulder_half = self.S(12.5 - 4.0 * yaw)
        waist_half = self.S(9.5 - 2.5 * yaw)
        hem_half = self.S(13.5 + abs(coat_swing) * 2.0)
        hem_y = py + self.S(16.5 - pose.collapse * 3.0)
        tail_split = self.S(4.0 + abs(coat_swing) * 1.0)

        # Outer coat.
        coat_pts = [
            (cx - shoulder_half, cy - self.S(3.0)),
            (cx + shoulder_half, cy - self.S(3.0)),
            (cx + waist_half, py + self.S(3.0)),
            (cx + hem_half, hem_y),
            (px + tail_split, hem_y - self.S(1.5)),
            (px, py + self.S(9.0)),
            (px - tail_split, hem_y - self.S(1.5)),
            (cx - hem_half, hem_y),
            (cx - waist_half, py + self.S(3.0)),
        ]
        polygon(draw, coat_pts, COAT, OUTLINE, max(1, self.aa // 2))

        # Lapels and vest.
        left_lapel = [
            (cx - self.S(6.5), cy - self.S(1.5)),
            (cx - self.S(1.0), cy + self.S(7.0)),
            (cx - self.S(4.0), py + self.S(2.0)),
        ]
        right_lapel = [
            (cx + self.S(6.5), cy - self.S(1.5)),
            (cx + self.S(1.0), cy + self.S(7.0)),
            (cx + self.S(4.0), py + self.S(2.0)),
        ]
        polygon(draw, left_lapel, COAT_DARK, OUTLINE)
        polygon(draw, right_lapel, COAT_DARK, OUTLINE)
        vest_pts = [
            (cx - self.S(3.8), cy - self.S(2.0)),
            (cx + self.S(3.8), cy - self.S(2.0)),
            (cx + self.S(4.2), py + self.S(4.5)),
            (cx, py + self.S(7.5)),
            (cx - self.S(4.2), py + self.S(4.5)),
        ]
        polygon(draw, vest_pts, VEST, OUTLINE)
        for i in range(3):
            circle(draw, (cx, cy + self.S(2.0 + i * 4.0)), self.S(0.8), SHIRT)

        # Shirt + cravat.
        shirt_pts = [
            (cx - self.S(3.0), cy - self.S(4.2)),
            (cx + self.S(3.0), cy - self.S(4.2)),
            (cx + self.S(1.4), cy + self.S(1.2)),
            (cx - self.S(1.4), cy + self.S(1.2)),
        ]
        polygon(draw, shirt_pts, SHIRT, OUTLINE)
        cravat_knot = (cx, cy - self.S(1.2))
        polygon(
            draw,
            [
                add(cravat_knot, self.pt(-1.8, -1.2)),
                add(cravat_knot, self.pt(1.8, -1.2)),
                add(cravat_knot, self.pt(2.8, 1.4)),
                add(cravat_knot, self.pt(0.0, 3.6)),
                add(cravat_knot, self.pt(-2.8, 1.4)),
            ],
            CRAVAT,
            OUTLINE,
        )
        polygon(
            draw,
            [
                add(cravat_knot, self.pt(-0.8, 2.8)),
                add(cravat_knot, self.pt(0.3, 8.0)),
                add(cravat_knot, self.pt(-2.0, 8.3)),
            ],
            CRAVAT,
            OUTLINE,
        )
        polygon(
            draw,
            [
                add(cravat_knot, self.pt(0.8, 2.8)),
                add(cravat_knot, self.pt(2.0, 7.6)),
                add(cravat_knot, self.pt(0.0, 8.2)),
            ],
            CRAVAT,
            OUTLINE,
        )

    def _draw_head(
        self, draw: ImageDraw.ImageDraw, head, pose: Pose, facing: int, yaw: float
    ) -> None:
        cx, cy = head
        side = facing if yaw >= 0.15 else 0
        head_tilt = pose.head_tilt
        face_w = self.S(10.0 - 2.3 * yaw)
        face_h = self.S(12.5 - 0.8 * pose.collapse)

        # Hair volume behind face.
        hair_oval_center = (cx - self.S(1.0 * side), cy - self.S(4.8))
        ellipse(draw, hair_oval_center, self.S(11.0 - 1.3 * yaw), self.S(8.4), HAIR)

        # Neck.
        polygon(
            draw,
            [
                self.pt(cx / self.aa - 2.2, cy / self.aa + 10.0),
                self.pt(cx / self.aa + 2.2, cy / self.aa + 10.0),
                self.pt(cx / self.aa + 2.6, cy / self.aa + 15.0),
                self.pt(cx / self.aa - 2.6, cy / self.aa + 15.0),
            ],
            SKIN,
            OUTLINE,
        )

        if yaw < 0.18:
            # Front face.
            face_pts = [
                (cx - face_w * 0.95, cy - face_h * 0.75),
                (cx + face_w * 0.95, cy - face_h * 0.75),
                (cx + face_w * 1.08, cy + face_h * 0.10),
                (cx + face_w * 0.55, cy + face_h * 0.78),
                (cx, cy + face_h * 1.05),
                (cx - face_w * 0.55, cy + face_h * 0.78),
                (cx - face_w * 1.08, cy + face_h * 0.10),
            ]
            polygon(draw, face_pts, SKIN, OUTLINE)
            # Hairline and side tufts.
            fringe = [
                (cx - self.S(8.5), cy - self.S(8.8)),
                (cx - self.S(5.8), cy - self.S(12.6 + pose.hair_bounce)),
                (cx - self.S(1.5), cy - self.S(11.0)),
                (cx + self.S(1.2), cy - self.S(13.8 + pose.hair_bounce * 0.6)),
                (cx + self.S(5.8), cy - self.S(10.9)),
                (cx + self.S(8.8), cy - self.S(7.8)),
                (cx + self.S(7.0), cy - self.S(4.6)),
                (cx - self.S(7.2), cy - self.S(4.9)),
            ]
            polygon(draw, fringe, HAIR, OUTLINE)
            polygon(
                draw,
                [
                    (cx - self.S(10.2), cy - self.S(5.3)),
                    (cx - self.S(6.8), cy + self.S(2.0)),
                    (cx - self.S(8.4), cy + self.S(4.8)),
                    (cx - self.S(11.2), cy - self.S(1.0)),
                ],
                HAIR,
                OUTLINE,
            )
            polygon(
                draw,
                [
                    (cx + self.S(10.0), cy - self.S(4.6)),
                    (cx + self.S(6.4), cy + self.S(1.6)),
                    (cx + self.S(7.6), cy + self.S(4.6)),
                    (cx + self.S(10.8), cy - self.S(0.4)),
                ],
                HAIR,
                OUTLINE,
            )
            # Features.
            self._front_face_features(draw, (cx, cy), pose)
        elif yaw < 0.75:
            # Three-quarter.
            fx = cx + self.S(2.2 * facing)
            face_pts = [
                (cx - self.S(8.8 * facing), cy - self.S(8.6)),
                (cx + self.S(6.2 * facing), cy - self.S(10.0)),
                (cx + self.S(10.6 * facing), cy - self.S(4.4)),
                (cx + self.S(11.0 * facing), cy + self.S(1.4)),
                (cx + self.S(6.2 * facing), cy + self.S(8.8)),
                (cx - self.S(1.0 * facing), cy + self.S(12.3)),
                (cx - self.S(8.6 * facing), cy + self.S(6.8)),
                (cx - self.S(10.0 * facing), cy - self.S(0.8)),
            ]
            polygon(draw, face_pts, SKIN, OUTLINE)
            # Hair mass overlapping forehead for a younger, unkempt look.
            fringe = [
                (cx - self.S(8.2 * facing), cy - self.S(7.8)),
                (cx - self.S(2.2 * facing), cy - self.S(12.8 + pose.hair_bounce)),
                (cx + self.S(3.0 * facing), cy - self.S(11.4)),
                (cx + self.S(8.2 * facing), cy - self.S(7.4)),
                (cx + self.S(6.5 * facing), cy - self.S(3.8)),
                (cx - self.S(7.2 * facing), cy - self.S(4.5)),
            ]
            polygon(draw, fringe, HAIR, OUTLINE)
            polygon(
                draw,
                [
                    (cx - self.S(9.8 * facing), cy - self.S(2.5)),
                    (cx - self.S(6.0 * facing), cy + self.S(5.8)),
                    (cx - self.S(9.0 * facing), cy + self.S(3.8)),
                ],
                HAIR,
                OUTLINE,
            )
            self._three_quarter_features(draw, (fx, cy), pose, facing)
        else:
            # Profile / side.
            face_pts = [
                (cx - self.S(7.8 * facing), cy - self.S(8.8)),
                (cx + self.S(4.0 * facing), cy - self.S(10.8)),
                (cx + self.S(8.6 * facing), cy - self.S(7.2)),
                (cx + self.S(10.8 * facing), cy - self.S(1.8)),
                (cx + self.S(9.4 * facing), cy + self.S(0.8)),
                (cx + self.S(11.0 * facing), cy + self.S(3.0)),
                (cx + self.S(8.0 * facing), cy + self.S(7.2)),
                (cx + self.S(5.4 * facing), cy + self.S(10.8)),
                (cx - self.S(1.0 * facing), cy + self.S(12.2)),
                (cx - self.S(7.6 * facing), cy + self.S(6.2)),
                (cx - self.S(8.8 * facing), cy - self.S(0.8)),
            ]
            polygon(draw, face_pts, SKIN, OUTLINE)
            hair_pts = [
                (cx - self.S(7.8 * facing), cy - self.S(8.8)),
                (cx - self.S(2.6 * facing), cy - self.S(13.2 + pose.hair_bounce)),
                (cx + self.S(5.2 * facing), cy - self.S(12.2)),
                (cx + self.S(8.8 * facing), cy - self.S(8.0)),
                (cx + self.S(6.8 * facing), cy - self.S(3.0)),
                (cx - self.S(4.5 * facing), cy - self.S(3.8)),
                (cx - self.S(8.2 * facing), cy - self.S(1.6)),
            ]
            polygon(draw, hair_pts, HAIR, OUTLINE)
            self._side_features(draw, (cx, cy), pose, facing)

    def _front_face_features(
        self, draw: ImageDraw.ImageDraw, c: tuple[float, float], pose: Pose
    ) -> None:
        cx, cy = c
        ellipse(
            draw,
            (cx - self.S(11.2), cy - self.S(0.8)),
            self.S(1.3),
            self.S(2.2),
            SKIN,
            OUTLINE,
        )
        ellipse(
            draw,
            (cx + self.S(11.2), cy - self.S(0.8)),
            self.S(1.3),
            self.S(2.2),
            SKIN,
            OUTLINE,
        )
        line(
            draw,
            [
                self.pt(cx / self.aa - 5.4, cy / self.aa - 5.0),
                self.pt(cx / self.aa - 1.6, cy / self.aa - 5.6),
            ],
            OUTLINE,
            max(1, self.aa // 2),
        )
        line(
            draw,
            [
                self.pt(cx / self.aa + 1.6, cy / self.aa - 5.6),
                self.pt(cx / self.aa + 5.6, cy / self.aa - 5.0),
            ],
            OUTLINE,
            max(1, self.aa // 2),
        )
        if pose.blink:
            line(
                draw,
                [
                    self.pt(cx / self.aa - 4.8, cy / self.aa - 1.5),
                    self.pt(cx / self.aa - 1.8, cy / self.aa - 1.0),
                ],
                OUTLINE,
                max(1, self.aa // 2),
            )
            line(
                draw,
                [
                    self.pt(cx / self.aa + 1.8, cy / self.aa - 1.0),
                    self.pt(cx / self.aa + 4.8, cy / self.aa - 1.5),
                ],
                OUTLINE,
                max(1, self.aa // 2),
            )
        else:
            ellipse(
                draw,
                self.pt(cx / self.aa - 3.4, cy / self.aa - 1.0),
                self.S(1.15),
                self.S(1.45),
                SHIRT,
                OUTLINE,
            )
            ellipse(
                draw,
                self.pt(cx / self.aa + 3.4, cy / self.aa - 1.0),
                self.S(1.15),
                self.S(1.45),
                SHIRT,
                OUTLINE,
            )
            circle(
                draw,
                self.pt(cx / self.aa - 3.2, cy / self.aa - 0.9),
                self.S(0.48),
                OUTLINE,
            )
            circle(
                draw,
                self.pt(cx / self.aa + 3.2, cy / self.aa - 0.9),
                self.S(0.48),
                OUTLINE,
            )
        line(
            draw,
            [
                self.pt(cx / self.aa + 0.2, cy / self.aa - 1.4),
                self.pt(cx / self.aa - 0.5, cy / self.aa + 2.4),
                self.pt(cx / self.aa + 0.8, cy / self.aa + 3.6),
            ],
            SKIN_SHADE,
            max(1, self.aa // 2),
        )
        mouth_y = cy / self.aa + 7.0
        mouth_curve = pose.mouth * 1.6
        line(
            draw,
            qcurve(
                self.pt(cx / self.aa - 3.8, mouth_y),
                self.pt(cx / self.aa, mouth_y + mouth_curve),
                self.pt(cx / self.aa + 3.8, mouth_y),
            ),
            OUTLINE,
            max(1, self.aa // 2),
        )
        line(
            draw,
            [
                self.pt(cx / self.aa - 5.5, cy / self.aa + 2.2),
                self.pt(cx / self.aa - 6.4, cy / self.aa + 6.0),
            ],
            SKIN_SHADE,
            max(1, self.aa // 2),
        )
        line(
            draw,
            [
                self.pt(cx / self.aa + 5.5, cy / self.aa + 2.2),
                self.pt(cx / self.aa + 6.4, cy / self.aa + 6.0),
            ],
            SKIN_SHADE,
            max(1, self.aa // 2),
        )

    def _three_quarter_features(
        self, draw: ImageDraw.ImageDraw, c: tuple[float, float], pose: Pose, facing: int
    ) -> None:
        cx, cy = c
        # single visible ear
        ellipse(
            draw,
            self.pt(cx / self.aa - 8.8 * facing, cy / self.aa - 0.8),
            self.S(1.2),
            self.S(2.0),
            SKIN,
            OUTLINE,
        )
        line(
            draw,
            [
                self.pt(cx / self.aa - 4.8 * facing, cy / self.aa - 5.0),
                self.pt(cx / self.aa - 1.4 * facing, cy / self.aa - 5.2),
            ],
            OUTLINE,
            max(1, self.aa // 2),
        )
        line(
            draw,
            [
                self.pt(cx / self.aa + 1.0 * facing, cy / self.aa - 4.8),
                self.pt(cx / self.aa + 4.6 * facing, cy / self.aa - 5.3),
            ],
            OUTLINE,
            max(1, self.aa // 2),
        )
        if pose.blink:
            line(
                draw,
                [
                    self.pt(cx / self.aa - 4.2 * facing, cy / self.aa - 1.0),
                    self.pt(cx / self.aa - 1.8 * facing, cy / self.aa - 0.6),
                ],
                OUTLINE,
                max(1, self.aa // 2),
            )
            line(
                draw,
                [
                    self.pt(cx / self.aa + 1.0 * facing, cy / self.aa - 0.8),
                    self.pt(cx / self.aa + 3.4 * facing, cy / self.aa - 0.8),
                ],
                OUTLINE,
                max(1, self.aa // 2),
            )
        else:
            ellipse(
                draw,
                self.pt(cx / self.aa - 3.2 * facing, cy / self.aa - 0.9),
                self.S(1.05),
                self.S(1.35),
                SHIRT,
                OUTLINE,
            )
            circle(
                draw,
                self.pt(cx / self.aa - 3.0 * facing, cy / self.aa - 0.9),
                self.S(0.44),
                OUTLINE,
            )
            ellipse(
                draw,
                self.pt(cx / self.aa + 2.2 * facing, cy / self.aa - 0.9),
                self.S(0.85),
                self.S(1.1),
                SHIRT,
                OUTLINE,
            )
            circle(
                draw,
                self.pt(cx / self.aa + 2.1 * facing, cy / self.aa - 0.9),
                self.S(0.34),
                OUTLINE,
            )
        line(
            draw,
            [
                self.pt(cx / self.aa - 0.8 * facing, cy / self.aa - 1.2),
                self.pt(cx / self.aa + 1.8 * facing, cy / self.aa + 2.7),
                self.pt(cx / self.aa + 3.4 * facing, cy / self.aa + 3.8),
            ],
            SKIN_SHADE,
            max(1, self.aa // 2),
        )
        mouth_y = cy / self.aa + 6.8
        line(
            draw,
            qcurve(
                self.pt(cx / self.aa - 2.8 * facing, mouth_y),
                self.pt(cx / self.aa + 0.3 * facing, mouth_y + pose.mouth * 1.2),
                self.pt(cx / self.aa + 4.2 * facing, mouth_y - 0.5),
            ),
            OUTLINE,
            max(1, self.aa // 2),
        )

    def _side_features(
        self, draw: ImageDraw.ImageDraw, c: tuple[float, float], pose: Pose, facing: int
    ) -> None:
        cx, cy = c
        ellipse(
            draw,
            self.pt(cx / self.aa - 8.0 * facing, cy / self.aa - 1.2),
            self.S(1.1),
            self.S(1.9),
            SKIN,
            OUTLINE,
        )
        line(
            draw,
            [
                self.pt(cx / self.aa - 1.0 * facing, cy / self.aa - 5.0),
                self.pt(cx / self.aa + 3.8 * facing, cy / self.aa - 5.2),
            ],
            OUTLINE,
            max(1, self.aa // 2),
        )
        if pose.blink:
            line(
                draw,
                [
                    self.pt(cx / self.aa + 0.5 * facing, cy / self.aa - 0.8),
                    self.pt(cx / self.aa + 3.4 * facing, cy / self.aa - 0.8),
                ],
                OUTLINE,
                max(1, self.aa // 2),
            )
        else:
            ellipse(
                draw,
                self.pt(cx / self.aa + 2.0 * facing, cy / self.aa - 1.0),
                self.S(0.95),
                self.S(1.25),
                SHIRT,
                OUTLINE,
            )
            circle(
                draw,
                self.pt(cx / self.aa + 2.0 * facing, cy / self.aa - 0.95),
                self.S(0.38),
                OUTLINE,
            )
        # nose / lips / chin profile
        line(
            draw,
            [
                self.pt(cx / self.aa + 2.8 * facing, cy / self.aa - 1.0),
                self.pt(cx / self.aa + 5.3 * facing, cy / self.aa + 1.1),
                self.pt(cx / self.aa + 3.6 * facing, cy / self.aa + 2.2),
            ],
            SKIN_SHADE,
            max(1, self.aa // 2),
        )
        line(
            draw,
            qcurve(
                self.pt(cx / self.aa + 1.8 * facing, cy / self.aa + 6.5),
                self.pt(
                    cx / self.aa + 3.6 * facing, cy / self.aa + 7.1 + pose.mouth * 1.1
                ),
                self.pt(cx / self.aa + 5.3 * facing, cy / self.aa + 6.6),
            ),
            OUTLINE,
            max(1, self.aa // 2),
        )

    def _draw_item(
        self,
        draw: ImageDraw.ImageDraw,
        hand: tuple[float, float],
        item: str,
        facing: int,
        pose: Pose,
    ) -> None:
        if item == "page":
            self._draw_page(draw, hand, angle=0.12 * facing - pose.lean * 0.2)
        elif item == "pistol":
            self._draw_pistol(
                draw, hand, angle=-0.18 * facing + pose.lean * 0.12, facing=facing
            )
        elif item == "book":
            self._draw_book(draw, hand, angle=0.15 * facing)

    def _draw_page(
        self, draw: ImageDraw.ImageDraw, hand: tuple[float, float], angle: float = 0.0
    ) -> None:
        hw = self.S(4.7)
        hh = self.S(6.2)
        pts = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
        rot = [add(hand, rotate(p, angle)) for p in pts]
        polygon(draw, rot, PAPER, OUTLINE)
        # folded corner
        polygon(
            draw,
            [rot[1], midpoint(rot[1], rot[2]), midpoint(rot[1], rot[0])],
            (220, 211, 186, 255),
            OUTLINE,
        )
        for dy in (-2.6, -0.5, 1.7):
            a = add(hand, rotate((-hw * 0.65, self.S(dy)), angle))
            b = add(hand, rotate((hw * 0.45, self.S(dy + 0.2)), angle))
            line(draw, [a, b], PAPER_LINE, max(1, self.aa // 2))

    def _draw_book(
        self, draw: ImageDraw.ImageDraw, hand: tuple[float, float], angle: float = 0.0
    ) -> None:
        hw = self.S(5.0)
        hh = self.S(6.0)
        pts = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
        rot = [add(hand, rotate(p, angle)) for p in pts]
        polygon(draw, rot, RED, OUTLINE)
        line(
            draw,
            [midpoint(rot[0], rot[3]), midpoint(rot[1], rot[2])],
            (220, 210, 190, 255),
            max(1, self.aa // 2),
        )

    def _draw_pistol(
        self,
        draw: ImageDraw.ImageDraw,
        hand: tuple[float, float],
        angle: float,
        facing: int,
    ) -> None:
        body = [(-1.2, -1.1), (8.6, -1.1), (8.6, 0.8), (-1.2, 0.8)]
        grip = [(-0.2, 0.8), (1.5, 0.8), (2.1, 4.8), (0.2, 5.2), (-1.2, 1.8)]
        hammer = [(0.0, -1.1), (1.2, -2.8), (2.0, -1.1)]

        def t(poly):
            return [add(hand, rotate(self.pt(x * facing, y), angle)) for x, y in poly]

        polygon(draw, t(body), METAL, OUTLINE)
        polygon(draw, t(grip), WOOD, OUTLINE)
        polygon(draw, t(hammer), METAL, OUTLINE)

    def _draw_dropped_page(
        self, draw: ImageDraw.ImageDraw, p: tuple[float, float], angle: float
    ) -> None:
        self._draw_page(draw, p, angle)


# --- sheet layout -------------------------------------------------------------
def build_pose_rows() -> list[tuple[str, list[Pose]]]:
    rows: list[tuple[str, list[Pose]]] = []

    # Row 1: stationary front/quarter/side orientation poses.
    # Named `rest` (mapped to CharacterAnim::Idle at runtime) so the
    # catalog-driven sprite loader treats this row as the character's
    # idle animation. Previously named `turn`, which the runtime
    # didn't recognize as an idle equivalent and which left the Hall
    # of Characters pedestal as a colored-rectangle placeholder.
    rows.append(
        (
            "rest",
            [
                Pose(
                    "front",
                    facing=1,
                    yaw=0.0,
                    mouth=-0.15,
                    right_hand=(13, 9),
                    left_hand=(-12, 10),
                ),
                Pose(
                    "front_talk",
                    facing=1,
                    yaw=0.0,
                    mouth=0.45,
                    head_tilt=-0.05,
                    right_hand=(14, 8),
                    left_hand=(-11, 10),
                ),
                Pose(
                    "quarter_r",
                    facing=1,
                    yaw=0.35,
                    mouth=-0.05,
                    right_hand=(14, 8),
                    left_hand=(-12, 9),
                    hair_bounce=0.3,
                ),
                Pose(
                    "side_r",
                    facing=1,
                    yaw=1.0,
                    mouth=-0.05,
                    right_hand=(14, 8),
                    left_hand=(-12, 9),
                    hair_bounce=0.1,
                ),
                Pose(
                    "quarter_l",
                    facing=-1,
                    yaw=0.35,
                    mouth=-0.08,
                    right_hand=(12, 10),
                    left_hand=(-14, 8),
                    hair_bounce=0.1,
                ),
                Pose(
                    "side_l",
                    facing=-1,
                    yaw=1.0,
                    mouth=-0.08,
                    right_hand=(11, 10),
                    left_hand=(-14, 8),
                ),
                Pose(
                    "blink",
                    facing=1,
                    yaw=0.0,
                    blink=True,
                    head_tilt=0.03,
                    left_hand=(-13, 11),
                    right_hand=(13, 8),
                ),
                Pose(
                    "page_front",
                    facing=1,
                    yaw=0.0,
                    left_hand=(-8, 10),
                    right_hand=(11, 6),
                    right_item="page",
                    mouth=-0.08,
                ),
            ],
        )
    )

    # Row 2: walk cycle.
    steps = [-1.0, -0.55, 0.0, 0.65, 1.0, 0.55, 0.0, -0.65]
    walk_poses = []
    for i, step in enumerate(steps):
        walk_poses.append(
            Pose(
                f"walk_{i}",
                facing=1,
                yaw=1.0,
                lean=0.10 * math.sin(i / 8 * math.tau),
                step=step,
                right_hand=(13 + step * 3.5, 6.5 + abs(step)),
                left_hand=(-13 - step * 3.5, 9.0 + abs(step) * 0.5),
                coat_swing=step,
                body_y=abs(step) * -1.2,
                hair_bounce=0.25 * abs(step),
            )
        )
    rows.append(("walk", walk_poses))

    # Row 3: manuscript / theorem gestures.
    rows.append(
        (
            "theorem",
            [
                Pose(
                    "read",
                    facing=1,
                    yaw=0.25,
                    left_hand=(-5, 6),
                    right_hand=(7, 7),
                    left_item="page",
                    right_item="page",
                    mouth=-0.12,
                ),
                Pose(
                    "point_page",
                    facing=1,
                    yaw=0.25,
                    left_hand=(-6, 8),
                    right_hand=(15, 4),
                    left_item="page",
                    mouth=0.15,
                ),
                Pose(
                    "wave_page",
                    facing=1,
                    yaw=0.45,
                    left_hand=(-1, -1),
                    left_item="page",
                    right_hand=(15, 10),
                    coat_swing=0.3,
                    mouth=0.22,
                    hair_bounce=0.35,
                ),
                Pose(
                    "clutch_notes",
                    facing=1,
                    yaw=0.0,
                    left_hand=(-5, 12),
                    right_hand=(6, 9),
                    left_item="page",
                    right_item="page",
                    mouth=-0.2,
                ),
                Pose(
                    "lecture",
                    facing=-1,
                    yaw=0.32,
                    left_hand=(-15, 4),
                    right_hand=(7, 8),
                    right_item="page",
                    mouth=0.3,
                ),
                Pose(
                    "argument",
                    facing=-1,
                    yaw=0.55,
                    left_hand=(-16, 2),
                    right_hand=(3, 9),
                    right_item="book",
                    mouth=0.05,
                    head_tilt=-0.08,
                ),
                Pose(
                    "thinking",
                    facing=1,
                    yaw=0.18,
                    left_hand=(-6, 10),
                    right_hand=(5, 2),
                    left_item="page",
                    mouth=-0.18,
                ),
                Pose(
                    "brandish",
                    facing=1,
                    yaw=0.45,
                    left_hand=(-10, 13),
                    right_hand=(13, -1),
                    right_item="page",
                    coat_swing=0.5,
                    mouth=0.4,
                ),
            ],
        )
    )

    # Row 4: duel sequence.
    rows.append(
        (
            "duel",
            [
                Pose(
                    "adjust_cuff",
                    facing=1,
                    yaw=0.6,
                    right_hand=(8, 9),
                    left_hand=(-10, 8),
                    mouth=-0.18,
                ),
                Pose(
                    "reach_pistol",
                    facing=1,
                    yaw=0.8,
                    right_hand=(12, 10),
                    left_hand=(-8, 8),
                    lean=0.06,
                    mouth=-0.12,
                ),
                Pose(
                    "ready",
                    facing=1,
                    yaw=1.0,
                    right_hand=(16, 6),
                    right_item="pistol",
                    left_hand=(-13, 10),
                    lean=0.08,
                    coat_swing=0.35,
                ),
                Pose(
                    "present",
                    facing=1,
                    yaw=1.0,
                    right_hand=(21, 1),
                    right_item="pistol",
                    left_hand=(-11, 9),
                    lean=0.22,
                    mouth=-0.08,
                    coat_swing=0.5,
                ),
                Pose(
                    "aim",
                    facing=1,
                    yaw=1.0,
                    right_hand=(24, -1),
                    right_item="pistol",
                    left_hand=(-8, 7),
                    lean=0.26,
                    mouth=-0.2,
                ),
                Pose(
                    "fire",
                    facing=1,
                    yaw=1.0,
                    right_hand=(25, -1),
                    right_item="pistol",
                    left_hand=(-8, 8),
                    lean=0.2,
                    mouth=0.1,
                    pistol_smoke=True,
                    coat_swing=0.55,
                    hair_bounce=0.25,
                ),
                Pose(
                    "recoil",
                    facing=1,
                    yaw=0.95,
                    right_hand=(18, 3),
                    right_item="pistol",
                    left_hand=(-9, 10),
                    lean=-0.12,
                    body_y=1.0,
                    coat_swing=-0.3,
                ),
                Pose(
                    "wounded",
                    facing=1,
                    yaw=0.75,
                    right_hand=(10, 12),
                    right_item="pistol",
                    left_hand=(-4, 7),
                    lean=-0.22,
                    mouth=-0.35,
                    body_y=2.0,
                    coat_swing=-0.2,
                ),
            ],
        )
    )

    # Row 5: death / collapse.
    rows.append(
        (
            "death",
            [
                Pose(
                    "clutch",
                    facing=1,
                    yaw=0.6,
                    right_hand=(5, 9),
                    left_hand=(-2, 4),
                    left_item="page",
                    lean=-0.25,
                    mouth=-0.4,
                    body_y=2.0,
                    coat_swing=-0.2,
                ),
                Pose(
                    "stagger",
                    facing=1,
                    yaw=0.8,
                    right_hand=(9, 14),
                    left_hand=(-7, 5),
                    lean=-0.35,
                    mouth=-0.55,
                    body_y=4.0,
                    right_item="pistol",
                    paper_drop=0.1,
                ),
                Pose(
                    "kneel",
                    facing=1,
                    yaw=0.8,
                    right_hand=(8, 17),
                    left_hand=(-8, 8),
                    lean=-0.4,
                    mouth=-0.65,
                    body_y=8.0,
                    crouch=3.5,
                    right_item="pistol",
                    paper_drop=0.25,
                ),
                Pose(
                    "sink",
                    facing=1,
                    yaw=0.85,
                    right_hand=(4, 18),
                    left_hand=(-8, 11),
                    lean=-0.42,
                    mouth=-0.75,
                    body_y=12.0,
                    crouch=5.5,
                    right_item="pistol",
                    paper_drop=0.45,
                    collapse=0.2,
                ),
                Pose(
                    "fall_1",
                    facing=1,
                    yaw=1.0,
                    right_hand=(1, 20),
                    left_hand=(-10, 13),
                    lean=-0.46,
                    mouth=-0.8,
                    body_y=15.0,
                    crouch=6.5,
                    paper_drop=0.65,
                    collapse=0.38,
                ),
                Pose(
                    "fall_2",
                    facing=1,
                    yaw=1.0,
                    right_hand=(-2, 21),
                    left_hand=(-8, 15),
                    lean=-0.5,
                    mouth=-0.85,
                    body_y=17.0,
                    crouch=7.0,
                    paper_drop=0.82,
                    collapse=0.55,
                ),
                Pose(
                    "down",
                    facing=1,
                    yaw=1.0,
                    right_hand=(-4, 22),
                    left_hand=(-7, 17),
                    lean=-0.58,
                    mouth=-0.9,
                    body_y=19.0,
                    crouch=7.4,
                    paper_drop=0.96,
                    collapse=0.72,
                ),
                Pose(
                    "still",
                    facing=1,
                    yaw=1.0,
                    right_hand=(-5, 22),
                    left_hand=(-7, 18),
                    lean=-0.6,
                    mouth=-0.95,
                    body_y=20.0,
                    crouch=7.8,
                    paper_drop=1.0,
                    collapse=0.85,
                ),
            ],
        )
    )

    return rows


# --- tack-on registry hooks ---------------------------------------------------
#
# Hooks `render(out_dir, **opts)` into the standard
# `tackon_sheet.build_sheet` pipeline so the sheet gets the same
# treatment as every other procedural character: auto-cropped to the
# union alpha bbox across all frames (the same crop is applied to
# every frame, so poses within a row stay aligned), labeled with
# per-row metadata, and shipped with the YAML + RON manifests the
# sandbox's SheetRegistry expects at runtime.

TARGET_NAME = "galwah"

# Per-row frame counts come from `build_pose_rows()`; the durations
# pace the visual feel (faster duel/walk, slower theorem/death).
_ROW_DURATIONS_MS: dict[str, int] = {
    "rest": 130,
    "walk": 95,
    "theorem": 110,
    "duel": 85,
    "death": 115,
}


def _build_setup(**opts):
    """Resolve the (sheet_rows, render_fn, frame_size) tuple from kwargs.

    Shared by both `render` and `render_canonical` so the canonical
    pose uses the exact same rig + frame size as the full sheet.
    """
    frame_w = int(opts.get("frame_width", 96))
    frame_h = int(opts.get("frame_height", 96))
    aa = int(opts.get("aa", 4))
    renderer = GalwahRenderer(frame_w=frame_w, frame_h=frame_h, aa=aa)
    pose_rows = build_pose_rows()
    poses_by_anim: dict[str, list[Pose]] = {anim: poses for anim, poses in pose_rows}
    sheet_rows: list[tuple[str, int, int]] = [
        (anim, len(poses), _ROW_DURATIONS_MS.get(anim, 110))
        for anim, poses in pose_rows
    ]

    def render_fn(anim: str, frame_idx: int, nframes: int) -> Image.Image:
        return renderer.render_pose(poses_by_anim[anim][frame_idx])

    return sheet_rows, render_fn, (frame_w, frame_h)


def render(out_dir: str | Path, **opts) -> list[Path]:
    """Render the galwah spritesheet bundle via `tackon_sheet.build_sheet`.

    Optional kwargs:

    * ``frame_width`` / ``frame_height`` (default 96 / 96) — pre-crop
      frame size. Final sheet is auto-cropped to the union alpha bbox
      across every frame.
    * ``aa`` (default 4) — supersample factor for the renderer.
    """
    from ...authoring.tackon_sheet import build_sheet  # local import: heavy deps

    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    sheet_rows, render_fn, frame_size = _build_setup(**opts)

    outputs = build_sheet(
        target=TARGET_NAME,
        rows=sheet_rows,
        render_fn=render_fn,
        out_dir=out_dir,
        frame_size=frame_size,
        auto_crop=True,
    )
    return [
        outputs["spritesheet"],
        outputs["yaml"],
        outputs["ron"],
        outputs["preview"],
        outputs["canonical"],
        outputs["canonical_transparent"],
    ]


def render_canonical(out_dir: str | Path, **opts) -> Path:
    """Fast canonical-only path: render + save just the canonical frame.

    Doesn't pay for the full sheet build — uses
    [`tackon_sheet.write_canonical`] which renders one frame, crops to
    alpha bbox, and saves it. Shares `_build_setup` with `render()`
    so the canonical pose matches what the full sheet would produce.
    """
    from ...authoring.tackon_sheet import write_canonical

    sheet_rows, render_fn, frame_size = _build_setup(**opts)
    return write_canonical(
        TARGET_NAME,
        sheet_rows,
        render_fn,
        Path(out_dir),
        frame_size=frame_size,
    )


# --- sheet assembly -----------------------------------------------------------
def assemble_sheet(
    renderer: GalwahRenderer, rows: Sequence[tuple[str, list[Pose]]]
) -> Image.Image:
    cols = max(len(row) for _, row in rows)
    sheet = Image.new(
        "RGBA", (renderer.frame_w * cols, renderer.frame_h * len(rows)), BG
    )
    for row_idx, (_, poses) in enumerate(rows):
        for col_idx, pose in enumerate(poses):
            frame = renderer.render_pose(pose)
            sheet.alpha_composite(
                frame, (col_idx * renderer.frame_w, row_idx * renderer.frame_h)
            )
    return sheet


def assemble_preview(
    sheet: Image.Image,
    rows: Sequence[tuple[str, list[Pose]]],
    frame_w: int,
    frame_h: int,
) -> Image.Image:
    cols = max(len(row) for _, row in rows)
    label_w = 88
    pad = 10
    header_h = 28
    out = Image.new(
        "RGBA",
        (label_w + cols * frame_w + pad * 2, header_h + len(rows) * frame_h + pad * 2),
        PREVIEW_BG,
    )
    draw = ImageDraw.Draw(out, "RGBA")
    font = ImageFont.load_default()

    # Column labels
    for c in range(cols):
        txt = str(c)
        tx = label_w + pad + c * frame_w + frame_w // 2 - 3
        draw.text((tx, 8), txt, font=font, fill=LABEL)

    for r, (row_name, poses) in enumerate(rows):
        y = header_h + pad + r * frame_h
        draw.text((10, y + frame_h // 2 - 4), row_name, font=font, fill=LABEL)
        # faint row separator
        draw.line((0, y, out.width, y), fill=(210, 206, 198, 255), width=1)
        for c, pose in enumerate(poses):
            x = label_w + pad + c * frame_w
            out.alpha_composite(
                sheet.crop(
                    (c * frame_w, r * frame_h, (c + 1) * frame_w, (r + 1) * frame_h)
                ),
                (x, y),
            )
            draw.rectangle(
                (x, y, x + frame_w, y + frame_h), outline=(220, 214, 205, 255), width=1
            )
            draw.text(
                (x + 3, y + 3), pose.name[:10], font=font, fill=(102, 96, 88, 220)
            )

    draw.line(
        (
            0,
            header_h + pad + len(rows) * frame_h,
            out.width,
            header_h + pad + len(rows) * frame_h,
        ),
        fill=(210, 206, 198, 255),
        width=1,
    )
    return out


# --- cli ---------------------------------------------------------------------
def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate a procedural GalWah spritesheet."
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("galwah_sheet"),
        help="Directory to write outputs into.",
    )
    parser.add_argument(
        "--frame-width", type=int, default=96, help="Width of each frame."
    )
    parser.add_argument(
        "--frame-height", type=int, default=96, help="Height of each frame."
    )
    parser.add_argument("--aa", type=int, default=4, help="Supersampling factor.")
    parser.add_argument(
        "--sheet-name", default="galwah_spritesheet.png", help="Spritesheet filename."
    )
    parser.add_argument(
        "--no-preview", action="store_true", help="Do not emit labeled preview image."
    )
    parser.add_argument(
        "--no-canonical",
        action="store_true",
        help="Do not emit a canonical front-pose image.",
    )
    args = parser.parse_args()

    args.out_dir.mkdir(parents=True, exist_ok=True)

    renderer = GalwahRenderer(
        frame_w=args.frame_width, frame_h=args.frame_height, aa=args.aa
    )
    rows = build_pose_rows()
    sheet = assemble_sheet(renderer, rows)
    sheet_path = args.out_dir / args.sheet_name
    sheet.save(sheet_path)

    if not args.no_preview:
        preview = assemble_preview(sheet, rows, args.frame_width, args.frame_height)
        preview.save(args.out_dir / "galwah_preview_labeled.png")

    if not args.no_canonical:
        canonical = renderer.render_pose(rows[0][1][0])
        canonical.save(args.out_dir / "galwah_canonical.png")

    print(f"wrote {sheet_path}")
    if not args.no_preview:
        print(f"wrote {args.out_dir / 'galwah_preview_labeled.png'}")
    if not args.no_canonical:
        print(f"wrote {args.out_dir / 'galwah_canonical.png'}")


if __name__ == "__main__":
    main()
