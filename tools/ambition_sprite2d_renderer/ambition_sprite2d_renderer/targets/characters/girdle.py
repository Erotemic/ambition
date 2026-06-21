from __future__ import annotations

"""Standalone generator for the Girdle sprite sheet.

Girdle is an important NPC: a severe, funny, unsettling logician inspired by
Kurt Godel's facial structure and styling.  The sheet prioritizes readable
front-view, side-view, and turning rows so cutscenes can show him directly
addressing the camera before rotating back into side-scroller space.
"""

import argparse
import math
from dataclasses import dataclass
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageDraw

ACTOR_METADATA = {
    "actor": {"character_id": "npc_girdle", "display_name": "Girdle"},
    "body": {
        "body_plan": "HumanoidBiped",
        "body_kind": "Standard",
        "mass_class": "Medium",
        "traits": ["story", "humanoid", "story"],
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
    "tags": ["story", "humanoid", "story"],
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
        "default": {"animation": "idle", "events": []},
        "locomotion.walk": {"animation": "walk", "events": []},
        "interaction.talk": {"animation": "talk", "events": []},
        "interaction.use": {"animation": "interact", "events": []},
    },
}


RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_BASENAME = "girdle"
FRAME_SIZE = (240, 224)
WORK_FRAME_SIZE = (480, 448)
SUPER = 4
ROWS: List[Tuple[str, int, int]] = [
    ("front_idle", 6, 130),
    ("front_talk", 6, 110),
    ("turn_front_to_side", 7, 95),
    ("side_idle", 6, 130),
    ("side_walk", 8, 95),
    ("loophole", 8, 90),
    ("turn_side_to_front", 7, 95),
]

OUTLINE = (20, 18, 18, 255)
SKIN = (214, 186, 154, 255)
SKIN_SHADE = (176, 144, 118, 255)
SKIN_LIGHT = (235, 214, 188, 255)
HAIR = (62, 50, 46, 255)
HAIR_LIGHT = (94, 82, 76, 255)
GLASS = (28, 26, 30, 255)
GLASS_TINT = (180, 200, 220, 72)
SUIT = (124, 98, 72, 255)
SUIT_DARK = (84, 65, 50, 255)
SUIT_LIGHT = (157, 131, 103, 255)
SHIRT = (226, 218, 204, 255)
TIE = (76, 72, 78, 255)
PAPER = (239, 232, 205, 255)
PAPER_SHADE = (198, 186, 152, 255)
INK = (42, 40, 50, 255)
SHADOW = (0, 0, 0, 38)
RED = (138, 56, 50, 255)


@dataclass
class Pose:
    yaw: float = 0.0  # 0 = front, 1 = right side profile
    bob: float = 0.0
    lean: float = 0.0
    head_tilt: float = 0.0
    shoulder_shift: float = 0.0
    blink: bool = False
    mouth: float = 0.0
    front_arm_angle: float = 0.0
    back_arm_angle: float = 0.0
    front_arm_reach: float = 0.0
    back_arm_reach: float = 0.0
    front_leg: float = 0.0
    back_leg: float = 0.0
    front_foot_lift: float = 0.0
    back_foot_lift: float = 0.0
    paper_raise: float = 0.0
    point: float = 0.0
    manuscript: float = 0.0
    eye_widen: float = 0.0

    def __init__(self, anim: str, frame_idx: int, nframes: int) -> None:
        t = frame_idx / max(1, nframes - 1)
        cyc = math.tau * frame_idx / max(1, nframes)
        s = math.sin(cyc)
        c = math.cos(cyc)
        self.yaw = 0.0
        self.bob = self.lean = self.head_tilt = self.shoulder_shift = 0.0
        self.blink = False
        self.mouth = 0.0
        self.front_arm_angle = self.back_arm_angle = 0.0
        self.front_arm_reach = self.back_arm_reach = 0.0
        self.front_leg = self.back_leg = 0.0
        self.front_foot_lift = self.back_foot_lift = 0.0
        self.paper_raise = self.point = self.manuscript = 0.0
        self.eye_widen = 0.0
        if anim == "front_idle":
            self.yaw = 0.0
            self.bob = s * 1.2
            self.head_tilt = s * 1.0
            self.lean = s * 0.5
            self.paper_raise = -1.0 + max(0.0, s) * 2.0
            self.blink = frame_idx == nframes - 2
        elif anim == "front_talk":
            self.yaw = 0.0
            self.bob = s * 1.0
            self.head_tilt = -1.0 + s * 1.4
            self.lean = s * 0.8
            self.front_arm_angle = -16.0 + s * 6.0
            self.back_arm_angle = 8.0 - s * 2.0
            self.front_arm_reach = max(0.0, s) * 8.0
            self.mouth = 0.18 + 0.18 * max(0.0, math.sin(cyc * 1.5))
            self.eye_widen = max(0.0, math.sin(cyc * 1.3)) * 0.15
            self.manuscript = 0.5
        elif anim == "turn_front_to_side":
            self.yaw = ease(t)
            self.bob = math.sin(t * math.pi) * 0.8
            self.head_tilt = -2.0 + t * 1.0
            self.lean = -4.0 * t
            self.front_arm_angle = -6.0 * t
            self.back_arm_angle = 6.0 * t
            self.paper_raise = -1.0 + t * 2.0
            self.manuscript = 0.25 + t * 0.5
        elif anim == "side_idle":
            self.yaw = 1.0
            self.bob = s * 1.1
            self.head_tilt = -2.0 + s * 1.1
            self.lean = -2.0 + s * 0.8
            self.front_arm_angle = 2.0 + s * 3.0
            self.back_arm_angle = -8.0 - s * 2.0
            self.paper_raise = -0.4 + max(0.0, s) * 1.8
            self.blink = frame_idx == nframes - 2
        elif anim == "side_walk":
            self.yaw = 1.0
            self.bob = abs(s) * 2.0 - 0.6
            self.lean = -4.0 + s * 1.8
            self.head_tilt = -3.0 - s * 1.0
            self.front_leg = 18.0 * s
            self.back_leg = -18.0 * s
            self.front_foot_lift = max(0.0, s) * 6.0
            self.back_foot_lift = max(0.0, -s) * 6.0
            self.front_arm_angle = -14.0 * s + 4.0
            self.back_arm_angle = 12.0 * s - 8.0
            self.front_arm_reach = max(0.0, -s) * 6.0
            self.back_arm_reach = max(0.0, s) * 4.0
        elif anim == "loophole":
            hit = math.sin(t * math.pi)
            self.yaw = 0.72 + 0.10 * math.sin(t * math.pi)
            self.bob = -hit * 1.2
            self.lean = -8.0 + hit * 4.0
            self.head_tilt = -4.0 + hit * 2.0
            self.front_arm_angle = -54.0 + hit * 10.0
            self.front_arm_reach = 30.0 + hit * 8.0
            self.back_arm_angle = -10.0
            self.back_arm_reach = 6.0
            self.manuscript = 1.0
            self.paper_raise = 10.0 * hit
            self.point = hit
            self.mouth = 0.10 + hit * 0.18
            self.eye_widen = 0.2 + hit * 0.25
        elif anim == "turn_side_to_front":
            self.yaw = 1.0 - ease(t)
            self.bob = math.sin(t * math.pi) * 0.8
            self.head_tilt = -1.0 + (1.0 - t) * -1.0
            self.lean = -4.0 * (1.0 - t)
            self.front_arm_angle = -6.0 * (1.0 - t)
            self.back_arm_angle = 6.0 * (1.0 - t)
            self.paper_raise = 1.0 - t * 2.0
            self.manuscript = 0.75 - t * 0.5


def mix(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def ease(t: float) -> float:
    t = max(0.0, min(1.0, t))
    return 0.5 - 0.5 * math.cos(math.pi * t)


def s(v: float) -> int:
    return int(round(v * SUPER))


def pt(p: Point) -> Tuple[int, int]:
    return (s(p[0]), s(p[1]))


def box(cx: float, cy: float, rx: float, ry: float) -> Tuple[int, int, int, int]:
    return (s(cx - rx), s(cy - ry), s(cx + rx), s(cy + ry))


def rot(x: float, y: float, deg: float) -> Point:
    rad = math.radians(deg)
    c = math.cos(rad)
    ss = math.sin(rad)
    return (x * c - y * ss, x * ss + y * c)


def poly(
    draw: ImageDraw.ImageDraw,
    pts: Sequence[Point],
    fill: RGBA,
    outline: RGBA = OUTLINE,
    width: float = 1.0,
) -> None:
    ipts = [pt(p) for p in pts]
    draw.polygon(ipts, fill=fill)
    if outline and width > 0:
        draw.line(ipts + [ipts[0]], fill=outline, width=max(1, s(width)), joint="curve")


def line(
    draw: ImageDraw.ImageDraw, pts: Sequence[Point], fill: RGBA, width: float = 1.0
) -> None:
    draw.line([pt(p) for p in pts], fill=fill, width=max(1, s(width)), joint="curve")


def composite_transparent_ellipse(
    img: Image.Image, cx: float, cy: float, rx: float, ry: float, fill: RGBA
) -> Image.Image:
    """Draw a translucent ellipse via alpha compositing.

    This keeps the underlying eye / skin pixels visible under the lens tint.
    """
    overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
    overlay_draw = ImageDraw.Draw(overlay, "RGBA")
    ellipse(overlay_draw, cx, cy, rx, ry, fill, None, 0)
    composed = Image.alpha_composite(img, overlay)
    img.paste(composed)
    return img


def ellipse(
    draw: ImageDraw.ImageDraw,
    cx: float,
    cy: float,
    rx: float,
    ry: float,
    fill: RGBA,
    outline: RGBA = OUTLINE,
    width: float = 1.0,
) -> None:
    draw.ellipse(
        box(cx, cy, rx, ry), fill=fill, outline=outline, width=max(1, s(width))
    )


def circle(
    draw: ImageDraw.ImageDraw,
    c: Point,
    r: float,
    fill: RGBA,
    outline: RGBA = OUTLINE,
    width: float = 1.0,
) -> None:
    ellipse(draw, c[0], c[1], r, r, fill, outline, width)


def downsample(img: Image.Image) -> Image.Image:
    return img.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


def draw_paper(
    draw: ImageDraw.ImageDraw,
    center: Point,
    angle: float,
    w: float = 22.0,
    h: float = 28.0,
    with_title: bool = False,
) -> None:
    hw = w / 2
    hh = h / 2
    pts = []
    for x, y in [(-hw, -hh), (hw, -hh + 2), (hw - 2, hh), (-hw + 3, hh - 1)]:
        rx, ry = rot(x, y, angle)
        pts.append((center[0] + rx, center[1] + ry))
    poly(draw, pts, PAPER, OUTLINE, 0.7)
    line(
        draw, [(center[0] - 7, center[1] - 6), (center[0] + 5, center[1] - 6)], INK, 0.5
    )
    line(draw, [(center[0] - 7, center[1] - 1), (center[0] + 7, center[1])], INK, 0.45)
    line(
        draw,
        [(center[0] - 7, center[1] + 4), (center[0] + 6, center[1] + 5)],
        INK,
        0.45,
    )
    if with_title:
        line(
            draw,
            [(center[0] - 7, center[1] - 10), (center[0] + 2, center[1] - 10)],
            RED,
            0.55,
        )


class GirdleRenderer:
    def render_frame(self, anim: str, frame_idx: int, nframes: int) -> Image.Image:
        img = Image.new(
            "RGBA",
            (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER),
            (0, 0, 0, 0),
        )
        draw = ImageDraw.Draw(img, "RGBA")
        pose = Pose(anim, frame_idx, nframes)
        root = (WORK_FRAME_SIZE[0] * 0.50, WORK_FRAME_SIZE[1] * 0.82 + pose.bob)
        tilt = pose.lean

        def P(x: float, y: float) -> Point:
            rx, ry = rot(x, y, tilt)
            return (root[0] + rx, root[1] + ry)

        ellipse(
            draw, root[0], root[1] + 4, 42, 8, SHADOW, outline=(0, 0, 0, 0), width=0
        )

        yaw = pose.yaw
        body_yaw = yaw * 0.85

        # Legs behind torso.
        self._draw_legs(draw, P, pose, body_yaw)
        # Torso and arms.
        chest = self._draw_torso(draw, P, pose, body_yaw)
        self._draw_arms(draw, P, pose, body_yaw, chest)
        # Head last for readability.
        head_anchor = P(mix(0.0, 14.0, body_yaw), -120)
        self._draw_head(img, draw, head_anchor, pose)

        if anim == "loophole":
            origin = P(72, -102)
            for i, ang in enumerate((-18, 6, 25)):
                cx = origin[0] + 18 + i * 18
                cy = origin[1] - 18 + abs(i - 1) * 6
                draw_paper(draw, (cx, cy), ang, 18, 24, with_title=i == 0)

        return downsample(img)

    def _draw_legs(self, draw: ImageDraw.ImageDraw, P, pose: Pose, yaw: float) -> None:
        front_x = mix(14.0, 18.0, yaw)
        back_x = mix(-14.0, -6.0, yaw)
        hip_y = -54.0
        knee_y = -20.0
        foot_y = 0.0
        # Back leg
        back_hip = P(back_x, hip_y)
        back_knee = P(back_x - 4 + pose.back_leg * 0.16, knee_y)
        back_foot = P(back_x - 4 + pose.back_leg * 0.12, foot_y - pose.back_foot_lift)
        line(draw, [back_hip, back_knee, back_foot], SUIT_DARK, 6.0)
        line(draw, [back_hip, back_knee, back_foot], OUTLINE, 1.2)
        poly(
            draw,
            [
                (back_foot[0] - 7, back_foot[1] - 3),
                (back_foot[0] + 14, back_foot[1] - 3),
                (back_foot[0] + 18, back_foot[1] + 4),
                (back_foot[0] - 5, back_foot[1] + 5),
            ],
            SUIT_DARK,
            OUTLINE,
            0.8,
        )
        # Front leg
        front_hip = P(front_x, hip_y)
        front_knee = P(front_x + 4 + pose.front_leg * 0.16, knee_y)
        front_foot = P(
            front_x + 8 + pose.front_leg * 0.12, foot_y - pose.front_foot_lift
        )
        line(draw, [front_hip, front_knee, front_foot], SUIT, 6.8)
        line(draw, [front_hip, front_knee, front_foot], OUTLINE, 1.4)
        poly(
            draw,
            [
                (front_foot[0] - 7, front_foot[1] - 3),
                (front_foot[0] + 15, front_foot[1] - 3),
                (front_foot[0] + 20, front_foot[1] + 4),
                (front_foot[0] - 5, front_foot[1] + 5),
            ],
            SUIT,
            OUTLINE,
            0.9,
        )

    def _draw_torso(
        self, draw: ImageDraw.ImageDraw, P, pose: Pose, yaw: float
    ) -> Point:
        if yaw < 0.35:
            torso = [
                P(-28, -116),
                P(28, -116),
                P(34, -60),
                P(20, -42),
                P(-20, -42),
                P(-34, -60),
            ]
            poly(draw, torso, SUIT, OUTLINE, 1.2)
            # lapels
            poly(
                draw,
                [P(-7, -112), P(0, -88), P(-14, -62), P(-24, -94)],
                SUIT_LIGHT,
                OUTLINE,
                0.8,
            )
            poly(
                draw,
                [P(7, -112), P(0, -88), P(14, -62), P(24, -94)],
                SUIT_DARK,
                OUTLINE,
                0.8,
            )
            poly(
                draw,
                [P(-10, -116), P(10, -116), P(8, -72), P(-8, -72)],
                SHIRT,
                OUTLINE,
                0.8,
            )
            poly(draw, [P(-8, -116), P(0, -100), P(8, -116)], SHIRT, OUTLINE, 0.6)
            poly(draw, [P(-4, -100), P(0, -70), P(4, -100)], TIE, OUTLINE, 0.6)
            line(draw, [P(-12, -46), P(-14, -2)], SUIT_LIGHT, 1.0)
            line(draw, [P(12, -46), P(14, -2)], SUIT_DARK, 1.0)
            return P(0, -90)
        else:
            torso = [
                P(-20, -116),
                P(18, -118),
                P(28, -100),
                P(30, -58),
                P(18, -42),
                P(-10, -42),
                P(-24, -62),
                P(-24, -98),
            ]
            poly(draw, torso, SUIT, OUTLINE, 1.2)
            poly(
                draw,
                [P(-4, -114), P(10, -108), P(18, -86), P(8, -60), P(-2, -78)],
                SUIT_DARK,
                OUTLINE,
                0.8,
            )
            poly(
                draw,
                [P(-8, -114), P(6, -112), P(4, -74), P(-8, -72)],
                SHIRT,
                OUTLINE,
                0.7,
            )
            poly(draw, [P(-2, -110), P(5, -92), P(1, -72)], TIE, OUTLINE, 0.6)
            line(draw, [P(-8, -46), P(-5, -2)], SUIT_LIGHT, 0.9)
            line(draw, [P(12, -46), P(16, -2)], SUIT_DARK, 0.9)
            return P(8, -90)

    def _draw_arms(
        self, draw: ImageDraw.ImageDraw, P, pose: Pose, yaw: float, chest: Point
    ) -> None:
        if yaw < 0.35:
            left_sh = P(-26, -106)
            right_sh = P(26, -106)
            # Back/left arm holding manuscript loosely.
            left_el = P(
                -32 + pose.back_arm_angle * 0.08, -72 + pose.back_arm_angle * 0.10
            )
            left_hand = P(
                -30 + pose.back_arm_angle * 0.10, -36 + pose.paper_raise * 0.4
            )
            line(draw, [left_sh, left_el, left_hand], SUIT_DARK, 5.0)
            line(draw, [left_el, left_hand], SKIN_SHADE, 4.0)
            line(draw, [left_sh, left_el, left_hand], OUTLINE, 1.0)
            circle(draw, left_hand, 4.0, SKIN_SHADE, OUTLINE, 0.6)
            if pose.manuscript > 0.1 or pose.paper_raise > -0.5:
                draw_paper(
                    draw,
                    (left_hand[0] - 4, left_hand[1] + 2),
                    -10 + pose.paper_raise * 0.2,
                    18,
                    24,
                    with_title=True,
                )

            right_el = P(
                28 + pose.front_arm_angle * 0.08 + pose.front_arm_reach * 0.10,
                -72 + pose.front_arm_angle * 0.12,
            )
            right_hand = P(
                28 + pose.front_arm_angle * 0.10 + pose.front_arm_reach,
                -38 + pose.front_arm_reach * 0.04,
            )
            line(draw, [right_sh, right_el, right_hand], SUIT, 5.4)
            line(draw, [right_el, right_hand], SKIN, 4.3)
            line(draw, [right_sh, right_el, right_hand], OUTLINE, 1.1)
            circle(draw, right_hand, 4.2, SKIN, OUTLINE, 0.7)
            if pose.front_arm_reach > 4.0:
                line(
                    draw,
                    [
                        (right_hand[0] + 2, right_hand[1]),
                        (right_hand[0] + 18, right_hand[1] - 2),
                    ],
                    SKIN,
                    1.2,
                )
                line(
                    draw,
                    [
                        (right_hand[0] + 2, right_hand[1]),
                        (right_hand[0] + 18, right_hand[1] - 2),
                    ],
                    OUTLINE,
                    0.4,
                )
        else:
            back_sh = P(-12, -106)
            front_sh = P(18, -106)
            back_el = P(
                -22 + pose.back_arm_angle * 0.06, -74 + pose.back_arm_angle * 0.10
            )
            back_hand = P(
                -18 + pose.back_arm_angle * 0.08 + pose.back_arm_reach,
                -40 + pose.paper_raise * 0.3,
            )
            line(draw, [back_sh, back_el, back_hand], SUIT_DARK, 4.6)
            line(draw, [back_el, back_hand], SKIN_SHADE, 3.6)
            line(draw, [back_sh, back_el, back_hand], OUTLINE, 0.9)
            circle(draw, back_hand, 3.6, SKIN_SHADE, OUTLINE, 0.6)
            if pose.manuscript > 0.15:
                draw_paper(
                    draw,
                    (back_hand[0] - 1, back_hand[1] + 1),
                    -14 + pose.paper_raise * 0.2,
                    18,
                    24,
                    with_title=True,
                )

            front_el = P(
                30 + pose.front_arm_angle * 0.08 + pose.front_arm_reach * 0.22,
                -78 + pose.front_arm_angle * 0.12,
            )
            front_hand = P(
                44 + pose.front_arm_angle * 0.12 + pose.front_arm_reach,
                -52 + pose.front_arm_reach * 0.16,
            )
            line(draw, [front_sh, front_el, front_hand], SUIT, 5.2)
            line(draw, [front_el, front_hand], SKIN, 4.2)
            line(draw, [front_sh, front_el, front_hand], OUTLINE, 1.0)
            circle(draw, front_hand, 3.9, SKIN, OUTLINE, 0.6)
            if pose.point > 0.05:
                tip = (front_hand[0] + 22, front_hand[1] - 8)
                line(draw, [front_hand, tip], SKIN, 1.3)
                line(draw, [front_hand, tip], OUTLINE, 0.45)

    def _draw_head(
        self, img: Image.Image, draw: ImageDraw.ImageDraw, anchor: Point, pose: Pose
    ) -> None:
        yaw = pose.yaw
        cx, cy = anchor
        if yaw < 0.22:
            self._draw_head_front(img, draw, (cx, cy), pose)
        elif yaw > 0.78:
            self._draw_head_side(img, draw, (cx, cy), pose)
        else:
            self._draw_head_three_quarter(img, draw, (cx, cy), pose)

    def _draw_head_front(
        self, img: Image.Image, draw: ImageDraw.ImageDraw, c: Point, pose: Pose
    ) -> None:
        cx, cy = c
        # Ears
        ellipse(draw, cx - 22, cy + 2, 5.5, 8.5, SKIN_SHADE, OUTLINE, 0.7)
        ellipse(draw, cx + 22, cy + 2, 5.5, 8.5, SKIN_SHADE, OUTLINE, 0.7)
        # Hair cap and sides
        poly(
            draw,
            [
                (cx - 20, cy - 30),
                (cx + 20, cy - 30),
                (cx + 22, cy - 4),
                (cx + 14, cy - 24),
                (cx - 14, cy - 24),
                (cx - 22, cy - 4),
            ],
            HAIR,
            OUTLINE,
            0.8,
        )
        # Face
        face = [
            (cx - 17, cy - 20),
            (cx + 17, cy - 20),
            (cx + 19, cy + 8),
            (cx + 10, cy + 24),
            (cx, cy + 30),
            (cx - 10, cy + 24),
            (cx - 19, cy + 8),
        ]
        poly(draw, face, SKIN, OUTLINE, 1.0)
        # Forehead highlight
        line(draw, [(cx - 8, cy - 16), (cx + 8, cy - 16)], SKIN_LIGHT, 0.8)
        # Hairline: high and receding
        line(
            draw,
            [
                (cx - 12, cy - 18),
                (cx - 8, cy - 26),
                (cx + 8, cy - 26),
                (cx + 12, cy - 18),
            ],
            OUTLINE,
            1.0,
        )
        line(draw, [(cx - 16, cy - 10), (cx - 16, cy - 24)], HAIR, 1.2)
        line(draw, [(cx + 16, cy - 10), (cx + 16, cy - 24)], HAIR, 1.2)
        # Glasses
        img = composite_transparent_ellipse(img, cx - 10, cy + 0, 8.5, 8.5, GLASS_TINT)
        img = composite_transparent_ellipse(img, cx + 10, cy + 0, 8.5, 8.5, GLASS_TINT)
        ellipse(draw, cx - 10, cy + 0, 8.5, 8.5, None, GLASS, 1.1)
        ellipse(draw, cx + 10, cy + 0, 8.5, 8.5, None, GLASS, 1.1)
        line(draw, [(cx - 1.5, cy), (cx + 1.5, cy)], GLASS, 0.8)
        line(draw, [(cx - 18, cy - 1), (cx - 25, cy - 4)], GLASS, 0.7)
        line(draw, [(cx + 18, cy - 1), (cx + 25, cy - 4)], GLASS, 0.7)
        # Eyes and brows
        if pose.blink:
            line(draw, [(cx - 13, cy + 0), (cx - 7, cy + 0)], OUTLINE, 0.7)
            line(draw, [(cx + 7, cy + 0), (cx + 13, cy + 0)], OUTLINE, 0.7)
        else:
            ellipse(
                draw, cx - 10, cy + 0, 2.3, 2.0 + pose.eye_widen, SHIRT, OUTLINE, 0.5
            )
            ellipse(
                draw, cx + 10, cy + 0, 2.3, 2.0 + pose.eye_widen, SHIRT, OUTLINE, 0.5
            )
            circle(draw, (cx - 10, cy + 0), 0.8, OUTLINE, OUTLINE, 0.2)
            circle(draw, (cx + 10, cy + 0), 0.8, OUTLINE, OUTLINE, 0.2)
        line(draw, [(cx - 15, cy - 7), (cx - 6, cy - 5)], OUTLINE, 0.8)
        line(draw, [(cx + 6, cy - 5), (cx + 15, cy - 7)], OUTLINE, 0.8)
        # Nose and mouth
        line(draw, [(cx, cy - 3), (cx - 1, cy + 8), (cx + 2, cy + 10)], SKIN_SHADE, 0.9)
        line(
            draw,
            [(cx - 7, cy + 17), (cx, cy + 19 + pose.mouth * 6), (cx + 7, cy + 17)],
            OUTLINE,
            0.8,
        )
        line(draw, [(cx - 6, cy + 21), (cx + 6, cy + 21)], SKIN_SHADE, 0.45)
        # Wrinkles
        line(draw, [(cx - 12, cy - 16), (cx - 4, cy - 14)], SKIN_SHADE, 0.4)
        line(draw, [(cx + 4, cy - 14), (cx + 12, cy - 16)], SKIN_SHADE, 0.4)
        line(draw, [(cx - 8, cy + 6), (cx - 10, cy + 16)], SKIN_SHADE, 0.45)
        line(draw, [(cx + 8, cy + 6), (cx + 10, cy + 16)], SKIN_SHADE, 0.45)

    def _draw_head_three_quarter(
        self, img: Image.Image, draw: ImageDraw.ImageDraw, c: Point, pose: Pose
    ) -> None:
        cx, cy = c
        yaw = pose.yaw
        # Visible ear
        ellipse(draw, cx - 20, cy + 2, 5.0, 8.0, SKIN_SHADE, OUTLINE, 0.6)
        # Hair back
        poly(
            draw,
            [
                (cx - 18, cy - 28),
                (cx + 14, cy - 30),
                (cx + 20, cy - 20),
                (cx + 18, cy - 3),
                (cx - 4, cy - 8),
                (cx - 18, cy - 2),
            ],
            HAIR,
            OUTLINE,
            0.8,
        )
        face = [
            (cx - 15, cy - 18),
            (cx + 11, cy - 21),
            (cx + 20, cy - 10),
            (cx + 25, cy + 3),
            (cx + 21, cy + 11),
            (cx + 9, cy + 24),
            (cx - 8, cy + 26),
            (cx - 17, cy + 14),
        ]
        poly(draw, face, SKIN, OUTLINE, 0.9)
        # Receding hairline and forehead plane
        line(
            draw,
            [(cx - 4, cy - 18), (cx + 2, cy - 26), (cx + 10, cy - 25)],
            OUTLINE,
            0.9,
        )
        line(draw, [(cx - 10, cy - 12), (cx - 10, cy - 24)], HAIR, 1.0)
        # Glasses, near lens prominent.
        img = composite_transparent_ellipse(img, cx + 5, cy - 1, 8.8, 8.8, GLASS_TINT)
        img = composite_transparent_ellipse(img, cx - 8, cy + 0.5, 6.0, 6.0, GLASS_TINT)
        ellipse(draw, cx + 5, cy - 1, 8.8, 8.8, None, GLASS, 1.0)
        ellipse(draw, cx - 8, cy + 0.5, 6.0, 6.0, None, GLASS, 0.9)
        line(draw, [(cx - 2, cy), (cx + 0.5, cy - 1)], GLASS, 0.8)
        line(draw, [(cx + 13, cy - 1), (cx + 21, cy - 4)], GLASS, 0.7)
        line(draw, [(cx - 14, cy), (cx - 22, cy - 3)], GLASS, 0.6)
        if pose.blink:
            line(draw, [(cx + 2, cy + 0), (cx + 8, cy + 0)], OUTLINE, 0.7)
            line(draw, [(cx - 10, cy + 1), (cx - 6, cy + 1)], OUTLINE, 0.6)
        else:
            ellipse(draw, cx + 5, cy, 2.4, 2.1 + pose.eye_widen, SHIRT, OUTLINE, 0.45)
            circle(draw, (cx + 5, cy), 0.8, OUTLINE, OUTLINE, 0.2)
            ellipse(
                draw,
                cx - 8,
                cy + 1,
                1.6,
                1.4 + pose.eye_widen * 0.6,
                SHIRT,
                OUTLINE,
                0.4,
            )
            circle(draw, (cx - 8, cy + 1), 0.6, OUTLINE, OUTLINE, 0.2)
        line(draw, [(cx - 11, cy - 7), (cx - 4, cy - 6)], OUTLINE, 0.7)
        line(draw, [(cx + 1, cy - 6), (cx + 10, cy - 8)], OUTLINE, 0.8)
        # Nose and mouth
        poly(
            draw,
            [(cx + 8, cy + 2), (cx + 25, cy + 4), (cx + 11, cy + 10)],
            SKIN,
            OUTLINE,
            0.6,
        )
        line(
            draw,
            [
                (cx + 1, cy + 17),
                (cx + 10, cy + 18 + pose.mouth * 6),
                (cx + 17, cy + 16),
            ],
            OUTLINE,
            0.8,
        )
        line(draw, [(cx + 2, cy + 20), (cx + 12, cy + 20)], SKIN_SHADE, 0.45)
        line(draw, [(cx + 0, cy + 6), (cx - 2, cy + 15)], SKIN_SHADE, 0.4)

    def _draw_head_side(
        self, img: Image.Image, draw: ImageDraw.ImageDraw, c: Point, pose: Pose
    ) -> None:
        cx, cy = c
        # Ear
        ellipse(draw, cx - 18, cy + 2, 5.5, 8.0, SKIN_SHADE, OUTLINE, 0.7)
        # Hair mass
        poly(
            draw,
            [
                (cx - 16, cy - 30),
                (cx + 7, cy - 30),
                (cx + 18, cy - 22),
                (cx + 18, cy - 8),
                (cx + 6, cy - 10),
                (cx - 12, cy - 6),
                (cx - 18, cy - 12),
            ],
            HAIR,
            OUTLINE,
            0.8,
        )
        # Face outline profile
        face = [
            (cx - 12, cy - 20),
            (cx + 6, cy - 22),
            (cx + 20, cy - 14),
            (cx + 27, cy - 2),
            (cx + 24, cy + 6),
            (cx + 14, cy + 14),
            (cx + 9, cy + 24),
            (cx - 7, cy + 26),
            (cx - 16, cy + 14),
        ]
        poly(draw, face, SKIN, OUTLINE, 0.9)
        line(
            draw,
            [(cx + 0, cy - 18), (cx + 5, cy - 28), (cx + 11, cy - 28)],
            OUTLINE,
            0.9,
        )
        img = composite_transparent_ellipse(img, cx + 5, cy - 1, 8.6, 8.6, GLASS_TINT)
        ellipse(draw, cx + 5, cy - 1, 8.6, 8.6, None, GLASS, 1.0)
        line(draw, [(cx + 13, cy - 1), (cx + 21, cy - 4)], GLASS, 0.7)
        if pose.blink:
            line(draw, [(cx + 2, cy + 0), (cx + 8, cy + 0)], OUTLINE, 0.7)
        else:
            ellipse(draw, cx + 5, cy, 2.4, 2.2 + pose.eye_widen, SHIRT, OUTLINE, 0.4)
            circle(draw, (cx + 5, cy), 0.8, OUTLINE, OUTLINE, 0.2)
        line(draw, [(cx + 1, cy - 7), (cx + 10, cy - 8)], OUTLINE, 0.8)
        poly(
            draw,
            [(cx + 8, cy + 2), (cx + 28, cy + 4), (cx + 12, cy + 10)],
            SKIN,
            OUTLINE,
            0.6,
        )
        line(
            draw,
            [
                (cx + 1, cy + 17),
                (cx + 12, cy + 18 + pose.mouth * 5),
                (cx + 18, cy + 15),
            ],
            OUTLINE,
            0.8,
        )
        line(draw, [(cx + 2, cy + 20), (cx + 11, cy + 20)], SKIN_SHADE, 0.45)
        line(draw, [(cx + 1, cy + 6), (cx - 1, cy + 14)], SKIN_SHADE, 0.4)


def _write_yaml(path: Path) -> None:
    lines = [
        f"target: {TARGET_BASENAME}",
        f"frame_width: {FRAME_SIZE[0]}",
        f"frame_height: {FRAME_SIZE[1]}",
        "rows:",
    ]
    for name, frames, ms in ROWS:
        lines += [f"  - name: {name}", f"    frames: {frames}", f"    frame_ms: {ms}"]
    path.write_text("\n".join(lines) + "\n")


def _write_ron(path: Path) -> None:
    rows = [
        f'        (name: "{name}", frames: {frames}, frame_ms: {ms}),'
        for name, frames, ms in ROWS
    ]
    path.write_text(
        "\n".join(
            [
                "(",
                f'    target: "{TARGET_BASENAME}",',
                f"    frame_width: {FRAME_SIZE[0]},",
                f"    frame_height: {FRAME_SIZE[1]},",
                "    rows: [",
                *rows,
                "    ],",
                ")",
            ]
        )
        + "\n"
    )


def _render_sheet(renderer: GirdleRenderer, out_dir: Path) -> List[Path]:
    fw, fh = FRAME_SIZE
    sheet_w = max(frames for _, frames, _ in ROWS) * fw
    sheet_h = len(ROWS) * fh
    sheet = Image.new("RGBA", (sheet_w, sheet_h), (0, 0, 0, 0))
    preview = Image.new("RGBA", (sheet_w + 160, sheet_h), (246, 243, 236, 255))
    pdraw = ImageDraw.Draw(preview)
    canonical = None
    for row_idx, (name, nframes, _ms) in enumerate(ROWS):
        pdraw.text((8, row_idx * fh + 8), name, fill=(36, 36, 36, 255))
        for frame_idx in range(nframes):
            frame = renderer.render_frame(name, frame_idx, nframes)
            x = frame_idx * fw
            y = row_idx * fh
            sheet.alpha_composite(frame, (x, y))
            preview.alpha_composite(frame, (x + 160, y))
            if canonical is None and name == "front_idle" and frame_idx == 0:
                canonical = frame
    if canonical is None:
        canonical = renderer.render_frame(ROWS[0][0], 0, ROWS[0][1])
    paths = [
        out_dir / f"{TARGET_BASENAME}.png",
        out_dir / f"{TARGET_BASENAME}.yaml",
        out_dir / f"{TARGET_BASENAME}.ron",
        out_dir / f"{TARGET_BASENAME}_preview_labeled.png",
        out_dir / f"{TARGET_BASENAME}_canonical.png",
    ]
    sheet.save(paths[0])
    _write_yaml(paths[1])
    _write_ron(paths[2])
    preview.save(paths[3])
    canonical.save(paths[4])
    return paths


def render(out_dir: str | Path, **opts) -> List[Path]:
    """Render the girdle spritesheet bundle via the shared
    `tackon_sheet.build_sheet` pipeline (auto-cropped, with the
    runtime-compatible YAML+RON shape). See `bear_mauler.render` for
    the full rationale — same conversion."""
    from ...authoring.tackon_sheet import build_sheet

    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    renderer = GirdleRenderer()
    outputs = build_sheet(
        target=TARGET_BASENAME,
        rows=ROWS,
        render_fn=renderer.render_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
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


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Render the Girdle character spritesheet."
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path(__file__).resolve().parents[2] / "generated" / TARGET_BASENAME,
    )
    args = parser.parse_args(argv)
    for path in render(args.out_dir):
        print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
