from __future__ import annotations

"""Standalone generator for a Smart House character sprite sheet.

Concept:
- a literal house that is also "smart"
- the face is integrated into the front of the house
- round spectacles, expressive brows, thoughtful mouth
- chimney, roof, windows, and a light-bulb / academic vibe
- stompy little foundation-feet for side-scroller readability

Generator only. No registration or GUI wiring.
"""

import argparse
import math
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet

RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_NAME = "smart_house"
# Files the tack-on installer copies into the sandbox sprites dir.
# Names match what `build_sheet` writes (target_spritesheet.{png,yaml,ron}).
SHEET_FILES = [
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
]
FRAME_SIZE = (320, 320)
WORK_FRAME_SIZE = (640, 640)
SUPER = 4
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 130),
    ("walk", 8, 95),
    ("ponder", 6, 108),
    ("lecture", 6, 94),
    ("idea", 6, 102),
    ("ram", 7, 80),
    ("hurt", 4, 90),
    ("death", 8, 112),
]

ACTOR_METADATA = {
    "actor": {
        "character_id": "npc_smart_house",
        "display_name": "Smart House",
    },
    "body": {
        "body_plan": "PropActor",
        "body_kind": "PropLike",
        "mass_class": "Heavy",
        "locomotion_hint": "StompyWalk",
        "traits": ["story", "prop_actor", "building", "speaker", "mobile_house"],
    },
    "capabilities": {
        "traversal": {
            "walk": True,
            "jump": None,
            "climb": None,
            "crawl": None,
            "fly": None,
            "swim": None,
            "use_lifts": None,
            "door_access": ["public"],
        },
        "interactions": {
            "talk": True,
            "trade": None,
            "carry": None,
            "open_doors": [],
        },
    },
    "brain": {"default_preset": "stand_still"},
    "actions": {"default_preset": "peaceful"},
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "locomotion.stompy_walk": {"animation": "walk", "events": []},
        "interaction.ponder": {"animation": "ponder", "events": []},
        "interaction.lecture": {
            "animation": "lecture",
            "events": [
                {"t": 0.42, "event": "speech_cue", "source": "smart_house.lecture"}
            ],
        },
        "interaction.idea": {
            "animation": "idea",
            "events": [{"t": 0.48, "event": "vfx_cue", "source": "smart_house.idea"}],
        },
        "action.special.ram": {
            "animation": "ram",
            "events": [
                {
                    "t": 0.44,
                    "event": "hitbox_active_start",
                    "source": "smart_house.ram",
                },
                {"t": 0.62, "event": "hitbox_active_end", "source": "smart_house.ram"},
            ],
        },
        "damage.hit": {"animation": "hurt", "events": []},
        "lifecycle.death": {"animation": "death", "events": []},
    },
    "sockets": {
        "door_center": {
            "source": "smart_house.geometry",
            "point": {"x": 160.0, "y": 210.0},
        },
        "face_center": {
            "source": "smart_house.geometry",
            "point": {"x": 160.0, "y": 132.0},
        },
        "speech_bubble": {
            "source": "smart_house.geometry",
            "point": {"x": 160.0, "y": 48.0},
        },
        "chimney": {"source": "smart_house.geometry", "point": {"x": 112.0, "y": 48.0}},
        "lightbulb": {
            "source": "smart_house.geometry",
            "point": {"x": 160.0, "y": 38.0},
        },
        "book_origin": {
            "source": "smart_house.geometry",
            "point": {"x": 114.0, "y": 172.0},
        },
        "paper_origin": {
            "source": "smart_house.geometry",
            "point": {"x": 210.0, "y": 172.0},
        },
        "ram_front": {
            "source": "smart_house.geometry",
            "point": {"x": 248.0, "y": 146.0},
        },
    },
    "tags": ["story", "prop_actor", "speaker", "mobile_house"],
}

OUTLINE = (30, 24, 20, 255)
WOOD = (198, 162, 104, 255)
WOOD_SHADE = (160, 127, 79, 255)
WOOD_DARK = (118, 91, 58, 255)
ROOF = (96, 56, 48, 255)
ROOF_HI = (136, 84, 72, 255)
CHIMNEY = (154, 96, 84, 255)
WINDOW = (160, 214, 238, 255)
WINDOW_SHADE = (114, 174, 202, 255)
GLASS_HI = (232, 246, 250, 255)
STONE = (146, 150, 164, 255)
STONE_SHADE = (105, 110, 124, 255)
DOOR = (110, 76, 48, 255)
DOOR_SHADE = (86, 56, 36, 255)
BRASS = (214, 178, 86, 255)
EYE = (244, 246, 250, 255)
PUPIL = (44, 40, 48, 255)
BROW = (78, 58, 44, 255)
TONGUE = (202, 112, 118, 255)
MOUTH = (88, 54, 58, 255)
BOOK = (84, 112, 184, 255)
BOOK_PAPER = (236, 232, 212, 255)
BULB = (252, 232, 140, 255)
BULB_GLOW = (255, 238, 164, 140)
PAPER = (238, 228, 196, 255)
SMOKE = (188, 192, 204, 130)
DUST = (132, 112, 84, 130)
FX = (255, 232, 156, 150)


def _s(v: float) -> int:
    return int(round(v * SUPER))


def _pt(p: Point) -> Tuple[int, int]:
    return (_s(p[0]), _s(p[1]))


def _box(cx: float, cy: float, rx: float, ry: float) -> Tuple[int, int, int, int]:
    return (_s(cx - rx), _s(cy - ry), _s(cx + rx), _s(cy + ry))


def _rot(x: float, y: float, deg: float) -> Point:
    rad = math.radians(deg)
    c = math.cos(rad)
    s = math.sin(rad)
    return (x * c - y * s, x * s + y * c)


def _lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def _ease(t: float) -> float:
    t = max(0.0, min(1.0, t))
    return 0.5 - 0.5 * math.cos(math.pi * t)


def _poly(
    draw: ImageDraw.ImageDraw,
    pts: Sequence[Point],
    fill: RGBA,
    outline: RGBA = OUTLINE,
    width: float = 1.0,
) -> None:
    ipts = [_pt(p) for p in pts]
    draw.polygon(ipts, fill=fill)
    if outline and width > 0:
        draw.line(
            ipts + [ipts[0]], fill=outline, width=max(1, _s(width)), joint="curve"
        )


def _line(
    draw: ImageDraw.ImageDraw, pts: Sequence[Point], fill: RGBA, width: float = 1.0
) -> None:
    draw.line([_pt(p) for p in pts], fill=fill, width=max(1, _s(width)), joint="curve")


def _ellipse(
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
        _box(cx, cy, rx, ry), fill=fill, outline=outline, width=max(1, _s(width))
    )


def _circle(
    draw: ImageDraw.ImageDraw,
    p: Point,
    r: float,
    fill: RGBA,
    outline: RGBA = OUTLINE,
    width: float = 1.0,
) -> None:
    _ellipse(draw, p[0], p[1], r, r, fill, outline, width)


def _downsample(img: Image.Image) -> Image.Image:
    return img.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


class Pose:
    def __init__(self, anim: str, frame_idx: int, nframes: int) -> None:
        t = frame_idx / max(1, nframes - 1)
        cyc = math.tau * frame_idx / max(1, nframes)
        s = math.sin(cyc)
        c = math.cos(cyc)

        self.root_x = 0.0
        self.root_y = 0.0
        self.bob = 0.0
        self.tilt = 0.0
        self.roof_tilt = 0.0
        self.chimney = 0.0
        self.brow = 0.0
        self.mouth_open = 0.0
        self.left_leg = 0.0
        self.right_leg = 0.0
        self.left_lift = 0.0
        self.right_lift = 0.0
        self.left_arm = 0.0
        self.right_arm = 0.0
        self.book = 0.0
        self.paper = 0.0
        self.idea = 0.0
        self.smoke = 0.0
        self.impact = 0.0
        self.dead_t = 0.0
        self.blink = False
        self.x_eye = False

        if anim == "idle":
            self.bob = s * 1.6
            self.tilt = s * 1.1
            self.roof_tilt = s * 1.6
            self.chimney = -s * 2.0
            self.brow = s * 1.0
            self.left_arm = -2.0 + s * 2.0
            self.right_arm = 2.0 - s * 1.8
            self.smoke = 0.35 + max(0.0, s) * 0.3
            self.blink = frame_idx == nframes - 2
        elif anim == "walk":
            self.root_x = s * 2.0
            self.bob = abs(s) * 2.8 - 0.5
            self.tilt = s * 2.0
            self.roof_tilt = -s * 2.6
            self.left_leg = -20.0 * s
            self.right_leg = 20.0 * s
            self.left_lift = max(0.0, -s) * 7.0
            self.right_lift = max(0.0, s) * 7.0
            self.left_arm = 14.0 * s
            self.right_arm = -12.0 * s
            self.smoke = 0.25 + abs(s) * 0.25
        elif anim == "ponder":
            self.bob = s * 1.1
            self.tilt = -1.0 + s * 0.8
            self.roof_tilt = s * 0.8
            self.brow = -4.0 + max(0.0, s) * 6.0
            self.left_arm = _lerp(0.0, 26.0, math.sin(t * math.pi))
            self.book = math.sin(t * math.pi)
            self.mouth_open = 0.04
            self.smoke = 0.25
        elif anim == "lecture":
            self.bob = s * 1.2
            self.tilt = s * 1.5
            self.roof_tilt = s * 1.2
            self.left_arm = -14.0 + s * 8.0
            self.right_arm = 28.0 - s * 6.0
            self.paper = 0.6 + max(0.0, s) * 0.35
            self.brow = -2.0
            self.mouth_open = 0.12 + max(0.0, s) * 0.06
            self.smoke = 0.22
        elif anim == "idea":
            tt = _ease(t)
            self.bob = -math.sin(tt * math.pi) * 1.8
            self.tilt = _lerp(-2.0, 2.0, tt)
            self.roof_tilt = _lerp(-2.0, 3.0, tt)
            self.left_arm = _lerp(-4.0, 16.0, tt)
            self.right_arm = _lerp(-6.0, 18.0, tt)
            self.idea = math.sin(tt * math.pi)
            self.brow = _lerp(4.0, -6.0, tt)
            self.mouth_open = 0.10 + self.idea * 0.06
            self.smoke = 0.18
        elif anim == "ram":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-8.0, 24.0, tt)
            self.bob = -hit * 2.0
            self.tilt = _lerp(-6.0, 12.0, tt)
            self.roof_tilt = _lerp(-8.0, 16.0, tt)
            self.left_leg = _lerp(-8.0, 10.0, tt)
            self.right_leg = _lerp(10.0, -4.0, tt)
            self.left_lift = _lerp(0.0, 4.0, tt)
            self.left_arm = _lerp(-10.0, 18.0, tt)
            self.right_arm = _lerp(10.0, -18.0, tt)
            self.brow = -8.0
            self.mouth_open = 0.12
            self.impact = hit
            self.smoke = 0.12
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 5.0) * (1.0 - t)
            self.root_x = shake * 3.0 - hit * 3.0
            self.bob = -hit * 2.0
            self.tilt = -8.0 * hit
            self.roof_tilt = -10.0 * hit
            self.left_arm = 12.0 * hit
            self.right_arm = 14.0 * hit
            self.mouth_open = 0.16 * hit
            self.smoke = 0.10
        elif anim == "death":
            tt = _ease(t)
            self.dead_t = tt
            self.root_x = tt * 18.0
            self.root_y = tt * 12.0
            self.bob = -tt * 4.0
            self.tilt = -72.0 * tt
            self.roof_tilt = -20.0 * tt
            self.left_leg = _lerp(-2.0, 20.0, tt)
            self.right_leg = _lerp(2.0, -18.0, tt)
            self.left_arm = _lerp(0.0, 26.0, tt)
            self.right_arm = _lerp(0.0, -30.0, tt)
            self.smoke = 0.4
            self.x_eye = tt > 0.55


def _draw_leg(
    draw: ImageDraw.ImageDraw, hip: Point, ang: float, lift: float, *, front: bool
) -> Point:
    seg1 = 22
    seg2 = 26
    knee = (
        hip[0] + seg1 * math.cos(math.radians(ang)),
        hip[1] + seg1 * math.sin(math.radians(ang)),
    )
    foot = (
        knee[0] + seg2 * math.cos(math.radians(ang + 8)),
        knee[1] + seg2 * math.sin(math.radians(ang + 8)) - lift,
    )
    col = STONE if front else STONE_SHADE
    _line(draw, [hip, knee, foot], col, 8.0 if front else 7.0)
    _line(draw, [hip, knee, foot], OUTLINE, 1.1)
    _ellipse(
        draw,
        foot[0],
        foot[1] + 4,
        10.0,
        4.5,
        STONE_SHADE if front else (90, 94, 104, 255),
        OUTLINE,
        0.7,
    )
    return foot


def _draw_arm(
    draw: ImageDraw.ImageDraw,
    shoulder: Point,
    ang: float,
    length: float,
    *,
    front: bool,
) -> Point:
    elbow = (
        shoulder[0] + (length * 0.45) * math.cos(math.radians(ang)),
        shoulder[1] + (length * 0.45) * math.sin(math.radians(ang)),
    )
    hand = (
        shoulder[0] + length * math.cos(math.radians(ang)),
        shoulder[1] + length * math.sin(math.radians(ang)),
    )
    col = WOOD_SHADE if front else WOOD_DARK
    _line(draw, [shoulder, elbow, hand], col, 6.8 if front else 5.8)
    _line(draw, [shoulder, elbow, hand], OUTLINE, 0.9)
    _circle(draw, hand, 4.4 if front else 3.8, BRASS, OUTLINE, 0.5)
    return hand


def _render_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    img = Image.new(
        "RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0)
    )
    draw = ImageDraw.Draw(img, "RGBA")
    pose = Pose(anim, frame_idx, nframes)

    root = (
        WORK_FRAME_SIZE[0] * 0.48 + pose.root_x,
        WORK_FRAME_SIZE[1] * 0.75 + pose.root_y + pose.bob,
    )
    body_ang = pose.tilt

    def P(x: float, y: float, extra: float = 0.0) -> Point:
        rx, ry = _rot(x, y, body_ang + extra)
        return (root[0] + rx, root[1] + ry)

    # far leg first
    far_hip = P(18, -12)
    _draw_leg(draw, far_hip, 94 + pose.right_leg, pose.right_lift, front=False)

    # far arm
    far_shoulder = P(60, -126)
    far_hand = _draw_arm(draw, far_shoulder, 42 + pose.right_arm, 34, front=False)

    # house body
    body = [P(-76, -168), P(84, -168), P(84, -14), P(-76, -14)]
    _poly(draw, body, WOOD, OUTLINE, 1.4)
    # siding lines
    for y in [-142, -114, -86, -58, -30]:
        _line(draw, [P(-72, y), P(80, y)], WOOD_SHADE, 1.0)
    for x in [-46, -6, 34, 66]:
        _line(draw, [P(x, -164), P(x, -18)], WOOD_SHADE, 0.7)

    # roof
    roof = [P(-96, -170), P(4, -244, pose.roof_tilt), P(112, -170)]
    _poly(draw, roof, ROOF, OUTLINE, 1.4)
    roof_edge = [P(-86, -170), P(6, -224, pose.roof_tilt), P(100, -170)]
    _line(draw, roof_edge, ROOF_HI, 2.0)
    for frac in [0.12, 0.28, 0.44, 0.60, 0.76]:
        ax = _lerp(-86, 92, frac)
        _line(
            draw,
            [P(ax, -170), P(ax - 22, -186 - frac * 16, pose.roof_tilt * 0.8)],
            ROOF_HI,
            0.8,
        )

    # chimney and smoke
    chimney = [
        P(44, -216, pose.chimney),
        P(68, -216, pose.chimney),
        P(68, -152),
        P(44, -152),
    ]
    _poly(draw, chimney, CHIMNEY, OUTLINE, 0.8)
    if pose.smoke > 0.05:
        sx, sy = P(58, -224, pose.chimney)
        for i, (dx, dy, rr) in enumerate([(0, 0, 8), (10, -12, 9), (2, -22, 10)]):
            _ellipse(
                draw,
                sx + dx,
                sy + dy - pose.smoke * 6 * i,
                rr,
                rr * 0.75,
                SMOKE,
                None,
                0,
            )

    # windows / eyes with glasses
    win_l = [P(-54, -136), P(-8, -136), P(-8, -94), P(-54, -94)]
    win_r = [P(8, -136), P(54, -136), P(54, -94), P(8, -94)]
    _poly(draw, win_l, WINDOW, OUTLINE, 1.0)
    _poly(draw, win_r, WINDOW, OUTLINE, 1.0)
    for pts in [win_l, win_r]:
        x0, y0 = pts[0]
        x1, _ = pts[1]
        _, y1 = pts[2]
        _line(draw, [((x0 + x1) / 2, y0), ((x0 + x1) / 2, y1)], WINDOW_SHADE, 0.8)
        _line(draw, [(x0, (y0 + y1) / 2), (x1, (y0 + y1) / 2)], WINDOW_SHADE, 0.8)
    _line(draw, [P(-48, -132), P(-18, -102)], GLASS_HI, 0.8)
    _line(draw, [P(14, -132), P(44, -102)], GLASS_HI, 0.8)

    eye_l = P(-31, -114)
    eye_r = P(31, -114)
    # spectacles
    _ellipse(draw, eye_l[0], eye_l[1], 16.0, 12.0, (0, 0, 0, 0), OUTLINE, 0.9)
    _ellipse(draw, eye_r[0], eye_r[1], 16.0, 12.0, (0, 0, 0, 0), OUTLINE, 0.9)
    _line(draw, [P(-15, -114), P(15, -114)], OUTLINE, 0.8)
    if pose.x_eye:
        _line(draw, [P(-38, -122), P(-24, -106)], OUTLINE, 0.8)
        _line(draw, [P(-38, -106), P(-24, -122)], OUTLINE, 0.8)
        _line(draw, [P(24, -122), P(38, -106)], OUTLINE, 0.8)
        _line(draw, [P(24, -106), P(38, -122)], OUTLINE, 0.8)
    elif pose.blink:
        _line(draw, [P(-38, -114), P(-24, -114)], BROW, 0.9)
        _line(draw, [P(24, -114), P(38, -114)], BROW, 0.9)
    else:
        _ellipse(draw, eye_l[0], eye_l[1], 7.0, 5.6, EYE, OUTLINE, 0.5)
        _ellipse(draw, eye_r[0], eye_r[1], 7.0, 5.6, EYE, OUTLINE, 0.5)
        _circle(draw, (eye_l[0] + 1, eye_l[1]), 1.4, PUPIL, PUPIL, 0.1)
        _circle(draw, (eye_r[0] + 1, eye_r[1]), 1.4, PUPIL, PUPIL, 0.1)
    _line(draw, [P(-44, -134 + pose.brow), P(-20, -138 + pose.brow)], BROW, 1.0)
    _line(draw, [P(20, -138 + pose.brow), P(44, -134 + pose.brow)], BROW, 1.0)

    # central nose / knocker
    _ellipse(draw, P(0, -86)[0], P(0, -86)[1], 5.0, 5.0, BRASS, OUTLINE, 0.5)

    # mouth / door
    door = [P(-24, -72), P(24, -72), P(24, -12), P(-24, -12)]
    _poly(draw, door, DOOR, OUTLINE, 1.0)
    _line(draw, [P(0, -68), P(0, -14)], DOOR_SHADE, 0.8)
    if pose.mouth_open > 0.03:
        _ellipse(
            draw,
            P(0, -40)[0],
            P(0, -40)[1],
            11.0,
            6.0 + pose.mouth_open * 14.0,
            MOUTH,
            OUTLINE,
            0.5,
        )
        _poly(draw, [P(-4, -34), P(0, -28), P(4, -34)], TONGUE, OUTLINE, 0.3)
    else:
        _line(draw, [P(-12, -40), P(0, -36), P(12, -40)], MOUTH, 0.9)
    _circle(draw, P(14, -38), 2.4, BRASS, OUTLINE, 0.3)

    # front leg and front arm
    near_hip = P(-18, -12)
    near_foot = _draw_leg(
        draw, near_hip, 94 + pose.left_leg, pose.left_lift, front=True
    )
    near_shoulder = P(-60, -126)
    near_hand = _draw_arm(draw, near_shoulder, 154 - pose.left_arm, 38, front=True)

    # foundation / trim over top of legs for clean stacking
    foundation = [P(-86, -18), P(94, -18), P(94, 8), P(-86, 8)]
    _poly(draw, foundation, STONE, OUTLINE, 1.1)
    _line(draw, [P(-82, -4), P(90, -4)], STONE_SHADE, 0.8)

    # props / fx
    if anim == "ponder" and pose.book > 0.05:
        bx, by = near_hand[0] - 10, near_hand[1] - 2
        _poly(
            draw,
            [
                (bx - 10, by - 8),
                (bx + 6, by - 10),
                (bx + 10, by + 8),
                (bx - 6, by + 10),
            ],
            BOOK,
            OUTLINE,
            0.5,
        )
        _line(draw, [(bx - 2, by - 6), (bx + 4, by + 6)], BOOK_PAPER, 0.8)
        _line(draw, [(bx - 7, by - 4), (bx + 0, by + 8)], BOOK_PAPER, 0.6)
    if anim == "lecture" and pose.paper > 0.05:
        px, py = far_hand[0] + 12, far_hand[1] - 4
        _poly(
            draw,
            [
                (px - 9, py - 12),
                (px + 7, py - 10),
                (px + 10, py + 10),
                (px - 8, py + 12),
            ],
            PAPER,
            OUTLINE,
            0.45,
        )
        _line(draw, [(px - 5, py - 6), (px + 3, py - 4)], WOOD_DARK, 0.5)
        _line(draw, [(px - 4, py), (px + 4, py + 2)], WOOD_DARK, 0.5)
        _line(draw, [(px - 3, py + 6), (px + 5, py + 8)], WOOD_DARK, 0.5)
    if anim == "idea" and pose.idea > 0.08:
        bx, by = P(0, -268, pose.roof_tilt)
        glow_r = 14 + pose.idea * 8
        _ellipse(draw, bx, by, glow_r + 10, glow_r + 10, BULB_GLOW, None, 0)
        _ellipse(draw, bx, by, 12.0, 16.0, BULB, OUTLINE, 0.6)
        _ellipse(draw, bx, by + 17, 7.0, 5.0, BRASS, OUTLINE, 0.5)
        _line(draw, [(bx, by + 12), (bx, by + 20)], OUTLINE, 0.5)
        for ang in [-60, -30, 0, 30, 60]:
            r0 = 20
            r1 = 30 + pose.idea * 6
            _line(
                draw,
                [
                    (
                        bx + math.cos(math.radians(ang)) * r0,
                        by + math.sin(math.radians(ang)) * r0,
                    ),
                    (
                        bx + math.cos(math.radians(ang)) * r1,
                        by + math.sin(math.radians(ang)) * r1,
                    ),
                ],
                FX,
                1.0,
            )
    if anim == "ram" and pose.impact > 0.15:
        cx, cy = P(96, -108)
        box = (_s(cx - 36), _s(cy - 26), _s(cx + 42), _s(cy + 36))
        draw.arc(box, 220, 350, fill=FX, width=_s(3.6))
    if anim in {"walk", "ram"} and (pose.left_lift > 0.5 or pose.right_lift > 0.5):
        for dx in [-30, -8, 14, 34]:
            c = (near_foot[0] + dx, near_foot[1] + 8)
            _poly(
                draw,
                [
                    (c[0] - 3, c[1]),
                    (c[0], c[1] - 4),
                    (c[0] + 4, c[1] - 1),
                    (c[0] + 1, c[1] + 3),
                ],
                DUST,
                None,
                0,
            )

    return _downsample(img)


def render(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=lambda anim, frame_idx, nframes: _render_frame(
            anim, frame_idx, nframes
        ),
        out_dir=out_dir,
        frame_size=opts.get("frame_size", FRAME_SIZE),
        crop_margin=10,
        auto_crop=True,
        actor_metadata=ACTOR_METADATA,
    )
    return [
        outputs["spritesheet"],
        outputs["yaml"],
        outputs["ron"],
        outputs["actor"],
        outputs["preview"],
        outputs["canonical"],
        outputs["canonical_transparent"],
    ]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Render the standalone Smart House sprite sheet."
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path(__file__).resolve().parents[2] / "generated" / TARGET_NAME,
    )
    args = parser.parse_args(argv)
    for path in render(args.out_dir):
        print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
