from __future__ import annotations

"""Standalone generator for a heavy ninja sprite.

This target is intentionally bespoke and does not reuse the pirate-heavy
construction.  The silhouette is a masked brute ninja: broad shoulders,
layered cowl, plated forearms, rope belt, split hakama, and a giant iron
kanabo.  The pose language is crouched and coiled rather than swaggering.

It only reuses ``build_sheet`` for the Ambition-compatible spritesheet / YAML /
RON output pipeline.
"""

import argparse
import math
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet

ACTOR_METADATA = {
    "actor": {"character_id": "npc_ninja_heavy", "display_name": "Ninja Heavy"},
    "body": {
        "body_plan": "HumanoidBiped",
        "body_kind": "Wide",
        "mass_class": "Heavy",
        "traits": ["story", "humanoid", "enemy", "combatant", "ninja", "heavy"],
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
    "brain": {"default_preset": "melee_brute_brute"},
    "actions": {"default_preset": "brute_lunge"},
    "visual": {"default_pose": "idle"},
    "tags": ["story", "humanoid", "enemy", "combatant", "ninja", "heavy"],
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
        "weapon_grip": {
            "source": "explicit.profile.combat_humanoid",
            "point": {"x": 80.0, "y": 64.0},
        },
        "weapon_tip": {
            "source": "explicit.profile.combat_humanoid",
            "point": {"x": 104.0, "y": 60.0},
        },
    },
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "locomotion.walk": {"animation": "walk", "events": []},
        "interaction.talk": {"animation": "talk", "events": []},
        "interaction.use": {"animation": "interact", "events": []},
        "action.melee.primary": {
            "animation": "slash",
            "events": [
                {
                    "t": 0.34,
                    "event": "hitbox_active_start",
                    "source": "explicit.profile.combat_humanoid",
                },
                {
                    "t": 0.58,
                    "event": "hitbox_active_end",
                    "source": "explicit.profile.combat_humanoid",
                },
            ],
        },
    },
}


RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_NAME = "ninja_heavy"
FRAME_SIZE = (320, 288)
WORK_FRAME_SIZE = (640, 576)
SUPER = 6
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 130),
    ("walk", 8, 95),
    ("slash", 7, 80),
    ("taunt", 6, 110),
    ("hurt", 4, 90),
    ("death", 8, 110),
]

OUTLINE = (14, 14, 18, 255)
SMOKE = (30, 34, 42, 170)
CLOTH_DARK = (25, 28, 34, 255)
CLOTH = (48, 55, 66, 255)
CLOTH_MID = (76, 84, 96, 255)
CLOTH_HI = (111, 122, 138, 255)
INDIGO = (42, 48, 86, 255)
INDIGO_HI = (72, 84, 136, 255)
WRAP = (90, 58, 52, 255)
WRAP_HI = (144, 101, 90, 255)
ROPE = (175, 142, 86, 255)
BRASS = (194, 154, 62, 255)
STEEL = (188, 193, 201, 255)
STEEL_DARK = (95, 100, 110, 255)
EYE = (210, 46, 58, 255)
EYE_HI = (255, 116, 88, 255)
DUST = (119, 101, 78, 160)
BAND = (112, 32, 35, 255)
SPIKE = (228, 234, 240, 255)


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


def _rot(x: float, y: float, deg: float) -> Point:
    rad = math.radians(deg)
    c = math.cos(rad)
    s = math.sin(rad)
    return (x * c - y * s, x * s + y * c)


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


def _rect(center: Point, w: float, h: float, deg: float) -> List[Point]:
    hw = w * 0.5
    hh = h * 0.5
    corners = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
    out = []
    for x, y in corners:
        rx, ry = _rot(x, y, deg)
        out.append((center[0] + rx, center[1] + ry))
    return out


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
        self.lean = 0.0
        self.head_tilt = 0.0
        self.left_arm = 0.0
        self.right_arm = 0.0
        self.left_leg = 0.0
        self.right_leg = 0.0
        self.club = -56.0
        self.club_lift = 0.0
        self.left_foot_lift = 0.0
        self.right_foot_lift = 0.0
        self.skirt_sway = 0.0
        self.eye_narrow = 0.0
        self.dead_t = 0.0
        self.impact = 0.0
        self.hurt = 0.0
        self.x_eyes = False

        if anim == "idle":
            self.root_x = s * 0.8
            self.bob = s * 1.6
            self.lean = -2.5 + s * 1.5
            self.head_tilt = -2.0 + s * 1.3
            self.left_arm = -8.0 + s * 4.0
            self.right_arm = 6.0 - s * 4.0
            self.left_leg = -3.0 + c * 1.3
            self.right_leg = 3.0 - c * 1.3
            self.club = -54.0 + s * 2.0
            self.skirt_sway = s * 2.0
            self.eye_narrow = 0.15 + max(0.0, s) * 0.08
        elif anim == "walk":
            self.root_x = s * 2.0
            self.bob = abs(s) * 2.4 - 0.5
            self.lean = -5.0 + s * 3.0
            self.head_tilt = -2.0 - s * 1.6
            self.left_leg = -18.0 * s
            self.right_leg = 18.0 * s
            self.left_arm = 12.0 * s - 8.0
            self.right_arm = -10.0 * s + 4.0
            self.club = -60.0 - s * 6.0
            self.left_foot_lift = max(0.0, -s) * 7.0
            self.right_foot_lift = max(0.0, s) * 7.0
            self.skirt_sway = -s * 6.0
            self.eye_narrow = 0.24
        elif anim == "slash":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-6.0, 7.0, tt)
            self.bob = -hit * 3.5
            self.lean = _lerp(-16.0, 18.0, tt)
            self.head_tilt = _lerp(-10.0, 5.0, tt)
            self.left_arm = _lerp(-20.0, 34.0, tt)
            self.right_arm = _lerp(-118.0, 6.0, tt)
            self.club = _lerp(-145.0, -24.0, tt)
            self.club_lift = -hit * 11.0 + tt * 26.0
            self.left_leg = -9.0 - hit * 4.0
            self.right_leg = 10.0 + hit * 4.0
            self.skirt_sway = _lerp(8.0, -10.0, tt)
            self.eye_narrow = 0.45 + hit * 0.2
            self.impact = hit
        elif anim == "taunt":
            self.root_x = s * 0.7
            self.bob = s * 1.2
            self.lean = -9.0 + s * 2.5
            self.head_tilt = -6.0 + s * 2.0
            self.left_arm = -82.0 + s * 7.0
            self.right_arm = -9.0 + s * 5.0
            self.club = -85.0 + s * 4.0
            self.skirt_sway = s * 3.0
            self.eye_narrow = 0.5
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 5.0) * (1.0 - t)
            self.root_x = shake * 4.0
            self.bob = -hit * 3.0
            self.lean = -18.0 * hit
            self.head_tilt = 12.0 * hit
            self.left_arm = 22.0 * hit
            self.right_arm = 20.0 * hit
            self.club = -64.0 + 15.0 * hit
            self.left_leg = 6.0 * hit
            self.right_leg = -6.0 * hit
            self.skirt_sway = -8.0 * hit
            self.eye_narrow = 0.4
            self.hurt = hit
        elif anim == "death":
            tt = _ease(t)
            self.dead_t = tt
            self.root_x = tt * 12.0
            self.root_y = tt * 5.0
            self.bob = -tt * 5.0
            self.lean = -80.0 * tt
            self.head_tilt = 20.0 * tt
            self.left_arm = _lerp(-8.0, 58.0, tt)
            self.right_arm = _lerp(6.0, -72.0, tt)
            self.club = _lerp(-54.0, -138.0, tt)
            self.left_leg = _lerp(-3.0, 28.0, tt)
            self.right_leg = _lerp(3.0, -24.0, tt)
            self.skirt_sway = 12.0 * tt
            self.eye_narrow = 0.2
            self.x_eyes = tt > 0.55


def _draw_kanabo(
    draw: ImageDraw.ImageDraw, hand: Point, angle: float, front: bool = True
) -> None:
    def tr(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, angle)
        return (hand[0] + rx, hand[1] + ry)

    shaft = [tr(-4, 6), tr(56, 0), tr(96, -2)]
    _line(draw, shaft, OUTLINE, 7.0)
    _line(draw, shaft, WRAP, 4.8)
    for x in [4, 20, 36, 52, 68]:
        _line(draw, [tr(x, -6), tr(x + 5, 9)], WRAP_HI, 1.0)
    _poly(draw, _rect(tr(0, 5), 12, 8, angle - 10), BRASS, OUTLINE, 1.2)
    _circle(draw, tr(-10, 6), 4.0, BRASS, OUTLINE, 1.0)

    club = [
        tr(74, -22),
        tr(102, -28),
        tr(117, -17),
        tr(121, 2),
        tr(116, 21),
        tr(97, 33),
        tr(74, 30),
        tr(84, 8),
        tr(70, -4),
    ]
    _poly(draw, club, STEEL_DARK, OUTLINE, 1.8)
    _poly(
        draw,
        [
            tr(78, -18),
            tr(98, -22),
            tr(111, -14),
            tr(115, 2),
            tr(110, 17),
            tr(96, 25),
            tr(79, 23),
            tr(87, 6),
            tr(74, -4),
        ],
        STEEL,
        OUTLINE,
        1.0,
    )
    spike_pts = [
        (86, -23),
        (102, -22),
        (111, -10),
        (114, 5),
        (109, 18),
        (96, 27),
        (84, 28),
    ]
    for sx, sy in spike_pts:
        _poly(
            draw, [tr(sx - 2, sy), tr(sx + 2, sy), tr(sx, sy - 8)], SPIKE, OUTLINE, 0.7
        )
    if front:
        _line(draw, [tr(80, -15), tr(108, 18)], (255, 255, 255, 110), 1.0)


def _draw_feet(draw: ImageDraw.ImageDraw, p: Point, toe: float) -> None:
    foot = [
        (p[0] - 13, p[1] - 3),
        (p[0] + 10 + toe * 1.2, p[1] - 4),
        (p[0] + 17 + toe * 1.8, p[1] + 4),
        (p[0] - 10, p[1] + 6),
    ]
    _poly(draw, foot, CLOTH_DARK, OUTLINE, 1.2)
    _line(draw, [(p[0] - 8, p[1] - 5), (p[0] + 7, p[1] - 5)], CLOTH_HI, 0.8)


def _draw_head(draw: ImageDraw.ImageDraw, p, pose: Pose) -> None:
    P = p
    # hood / cowl back mass
    hood = [
        P(-27, -148),
        P(-18, -164),
        P(18, -164),
        P(31, -148),
        P(28, -121),
        P(15, -101),
        P(-15, -102),
        P(-29, -120),
    ]
    _poly(draw, hood, CLOTH_DARK, OUTLINE, 1.8)
    hood_inner = [
        P(-19, -148),
        P(-11, -158),
        P(10, -158),
        P(20, -148),
        P(18, -125),
        P(8, -111),
        P(-7, -111),
        P(-18, -125),
    ]
    _poly(draw, hood_inner, CLOTH, OUTLINE, 1.0)

    # mask / face wrap
    face = [
        P(-18, -142),
        P(-5, -150),
        P(11, -149),
        P(20, -139),
        P(17, -119),
        P(9, -108),
        P(-6, -108),
        P(-18, -118),
    ]
    _poly(draw, face, CLOTH_MID, OUTLINE, 1.4)
    mask = [P(-16, -133), P(16, -133), P(12, -116), P(-13, -116)]
    _poly(draw, mask, BAND, OUTLINE, 1.2)

    # red eye slit
    if pose.x_eyes:
        for ex in [-7, 7]:
            _line(draw, [P(ex - 4, -126), P(ex + 4, -118)], OUTLINE, 1.2)
            _line(draw, [P(ex - 4, -118), P(ex + 4, -126)], OUTLINE, 1.2)
    else:
        slit_y = -124
        _poly(
            draw,
            [
                P(-13, slit_y),
                P(-2, slit_y - 3),
                P(9, slit_y - 2),
                P(14, slit_y + 1),
                P(8, slit_y + 4),
                P(-4, slit_y + 4),
                P(-14, slit_y + 1),
            ],
            EYE,
            OUTLINE,
            0.8,
        )
        _line(draw, [P(-10, slit_y + 2), P(10, slit_y + 2)], EYE_HI, 0.8)
        # angry eyelid angle
        _line(
            draw,
            [P(-15, slit_y - 2 - pose.eye_narrow * 3.0), P(-3, slit_y - 5)],
            OUTLINE,
            1.0,
        )
        _line(
            draw,
            [P(3, slit_y - 5), P(15, slit_y - 2 - pose.eye_narrow * 3.0)],
            OUTLINE,
            1.0,
        )

    # head ties
    _poly(draw, [P(18, -139), P(37, -145), P(29, -131)], BAND, OUTLINE, 0.9)
    _poly(draw, [P(19, -136), P(35, -124), P(26, -121)], BAND, OUTLINE, 0.9)


def _draw_torso(draw: ImageDraw.ImageDraw, p, pose: Pose) -> None:
    P = p
    # smoke skirt shadow behind body for ninja vibe
    shadow = [
        P(-40, -80),
        P(41, -82),
        P(52 + pose.skirt_sway * 0.15, -14),
        P(0, 9),
        P(-51 + pose.skirt_sway * 0.15, -14),
    ]
    _poly(draw, shadow, SMOKE, None, 0)

    shoulders = [
        P(-47, -103),
        P(-28, -122),
        P(30, -122),
        P(50, -103),
        P(40, -76),
        P(-41, -76),
    ]
    _poly(draw, shoulders, CLOTH_DARK, OUTLINE, 2.0)
    pauld_l = [P(-49, -104), P(-35, -117), P(-17, -112), P(-21, -92), P(-45, -87)]
    pauld_r = [P(21, -112), P(41, -117), P(52, -103), P(47, -87), P(23, -92)]
    _poly(draw, pauld_l, CLOTH, OUTLINE, 1.3)
    _poly(draw, pauld_r, CLOTH, OUTLINE, 1.3)

    chest = [
        P(-31, -104),
        P(-19, -116),
        P(18, -116),
        P(31, -104),
        P(28, -74),
        P(0, -62),
        P(-28, -74),
    ]
    _poly(draw, chest, CLOTH, OUTLINE, 1.6)
    chest_plate = [P(-22, -99), P(22, -99), P(18, -77), P(0, -67), P(-18, -77)]
    _poly(draw, chest_plate, INDIGO, OUTLINE, 1.2)
    _line(draw, [P(0, -110), P(0, -68)], CLOTH_HI, 1.0)
    _line(draw, [P(-17, -90), P(17, -90)], INDIGO_HI, 1.0)

    waist = [P(-34, -66), P(34, -65), P(30, -53), P(-35, -54)]
    _poly(draw, waist, ROPE, OUTLINE, 1.3)
    for x in [-24, -12, 0, 12, 24]:
        _line(draw, [P(x, -68), P(x + 6, -52)], (214, 186, 122, 255), 0.8)

    # split hakama panels instead of pirate skirt
    sway = pose.skirt_sway
    left_panel = [P(-35, -54), P(-4, -54), P(-8, 2), P(-46 + sway * 0.15, -5)]
    mid_panel = [P(-6, -54), P(7, -54), P(5, 0), P(-4, 1)]
    right_panel = [P(5, -54), P(36, -54), P(46 + sway * 0.15, -5), P(9, 2)]
    _poly(draw, left_panel, INDIGO, OUTLINE, 1.6)
    _poly(draw, mid_panel, CLOTH_DARK, OUTLINE, 1.2)
    _poly(draw, right_panel, INDIGO, OUTLINE, 1.6)
    for x in [-25, -14, 15, 27]:
        _line(draw, [P(x + sway * 0.05, -50), P(x + sway * 0.12, -4)], INDIGO_HI, 1.0)


def _draw_limbs(draw: ImageDraw.ImageDraw, p, pose: Pose) -> Tuple[Point, Point]:
    P = p
    left_hip = P(-18, -46)
    right_hip = P(18, -46)
    left_knee = P(-24 + pose.left_leg * 0.16, -15)
    right_knee = P(24 + pose.right_leg * 0.16, -15)
    left_foot = P(-30 + pose.left_leg * 0.16, 6 - pose.left_foot_lift)
    right_foot = P(30 + pose.right_leg * 0.16, 6 - pose.right_foot_lift)
    for hip, knee, foot in [
        (left_hip, left_knee, left_foot),
        (right_hip, right_knee, right_foot),
    ]:
        _line(draw, [hip, knee, foot], CLOTH_DARK, 7.4)
        _line(draw, [hip, knee, foot], OUTLINE, 1.8)
        _draw_feet(draw, foot, 1 if foot[0] > hip[0] else -1)

    left_shoulder = P(-42, -95)
    left_elbow = P(-53 + pose.left_arm * 0.05, -65 + pose.left_arm * 0.14)
    left_hand = P(-37 + pose.left_arm * 0.22, -42 + pose.left_arm * 0.18)
    _line(draw, [left_shoulder, left_elbow, left_hand], CLOTH_DARK, 10.0)
    _line(draw, [left_shoulder, left_elbow, left_hand], OUTLINE, 2.1)
    _ellipse(draw, left_elbow[0], left_elbow[1], 7.5, 9.0, CLOTH, OUTLINE, 1.0)
    _poly(draw, _rect(left_hand, 14, 12, pose.lean * 0.2), CLOTH, OUTLINE, 1.0)
    _line(
        draw,
        [(left_hand[0] - 6, left_hand[1] - 4), (left_hand[0] + 5, left_hand[1] - 4)],
        BRASS,
        0.9,
    )

    right_shoulder = P(43, -95)
    right_elbow = P(
        56 + pose.right_arm * 0.05, -64 + pose.right_arm * 0.15 + pose.club_lift * 0.2
    )
    right_hand = P(
        40 + pose.right_arm * 0.22, -41 + pose.right_arm * 0.22 + pose.club_lift
    )
    _line(draw, [right_shoulder, right_elbow, right_hand], CLOTH_DARK, 10.6)
    _line(draw, [right_shoulder, right_elbow, right_hand], OUTLINE, 2.1)
    _ellipse(draw, right_elbow[0], right_elbow[1], 8.0, 9.0, CLOTH, OUTLINE, 1.0)
    _poly(draw, _rect(right_hand, 14, 12, pose.lean * 0.2), CLOTH, OUTLINE, 1.0)
    _line(
        draw,
        [
            (right_hand[0] - 6, right_hand[1] - 4),
            (right_hand[0] + 5, right_hand[1] - 4),
        ],
        BRASS,
        0.9,
    )
    return left_hand, right_hand


def _render_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    img = Image.new(
        "RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0)
    )
    draw = ImageDraw.Draw(img, "RGBA")
    pose = Pose(anim, frame_idx, nframes)

    root = (
        WORK_FRAME_SIZE[0] * 0.48 + pose.root_x + pose.dead_t * 7.0,
        WORK_FRAME_SIZE[1] * 0.68 + pose.root_y + pose.bob,
    )
    tilt = pose.lean

    def P(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, tilt)
        return (root[0] + rx, root[1] + ry)

    # club behind body except during smash
    if anim != "slash":
        back_hand = P(39 + pose.right_arm * 0.14, -42 + pose.club_lift)
        _draw_kanabo(draw, back_hand, pose.club + tilt, front=False)

    if anim == "slash" and pose.impact > 0.1:
        cx, cy = P(28, -76)
        box = (_s(cx - 88), _s(cy - 94), _s(cx + 95), _s(cy + 48))
        draw.arc(box, 210, 332, fill=(214, 200, 176, 155), width=_s(7.0))
        draw.arc(box, 220, 323, fill=(255, 255, 255, 110), width=_s(2.6))

    # smoke wisps near ground during taunt/slash
    if anim in {"slash", "taunt"}:
        for dx, dy, r in [(-42, -4, 11), (46, -2, 12)]:
            c = P(dx, dy)
            _ellipse(draw, c[0], c[1], r + pose.impact * 4, r * 0.6, SMOKE, None, 0)

    _draw_limbs(draw, P, pose)
    _draw_torso(draw, P, pose)
    _draw_head(draw, P, pose)

    # front club on smash frames
    if anim == "slash":
        hand = P(
            40 + pose.right_arm * 0.22, -41 + pose.right_arm * 0.22 + pose.club_lift
        )
        _draw_kanabo(draw, hand, pose.club + tilt, front=True)

    if anim == "slash" and pose.impact > 0.45:
        for i, (dx, dy) in enumerate([(-50, 8), (-34, 13), (35, 9), (53, 12)]):
            jitter = math.sin(frame_idx + i) * 1.5
            c = P(dx + jitter, dy)
            _poly(
                draw,
                [(c[0] - 2.5, c[1] - 1.5), (c[0] + 3.0, c[1]), (c[0], c[1] + 3.0)],
                DUST,
                (80, 66, 50, 130),
                0.5,
            )

    return _downsample(img)


def render(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    frame_size = opts.get("frame_size", FRAME_SIZE)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=lambda anim, frame_idx, nframes: _render_frame(
            anim, frame_idx, nframes
        ),
        out_dir=out_dir,
        frame_size=frame_size,
        crop_margin=10,
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


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Render the standalone ninja-heavy spritesheet."
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
