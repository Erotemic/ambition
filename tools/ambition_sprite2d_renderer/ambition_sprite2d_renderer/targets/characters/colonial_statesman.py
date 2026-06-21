from __future__ import annotations

"""Standalone generator for a portrait-inspired colonial statesman character.

Visual goal:
- inspired by formal 18th-century oil portrait aesthetics
- powdered white wig with side rolls
- stern face, pale skin, dark formal coat, white cravat
- restrained, dignified motion rather than wild cartoon posing
- still readable as a side-scroller unit with a few active combat / command moves

This character is not a pirate; it is a formal statesman / aristocratic duelist
that leans into classic portrait styling.
"""

import argparse
import math
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet

ACTOR_METADATA = {
    "actor": {
        "character_id": "npc_colonial_statesman",
        "display_name": "Colonial Statesman",
    },
    "body": {
        "body_plan": "HumanoidBiped",
        "body_kind": "Standard",
        "mass_class": "Medium",
        "traits": ["story", "humanoid", "story", "statesman"],
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
    "tags": ["story", "humanoid", "story", "statesman"],
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

TARGET_NAME = "colonial_statesman"
FRAME_SIZE = (320, 352)
WORK_FRAME_SIZE = (640, 704)
SUPER = 4
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 130),
    ("walk", 8, 96),
    ("address", 6, 112),
    ("thrust", 7, 82),
    ("pistol", 6, 88),
    ("hurt", 4, 92),
    ("death", 8, 112),
]

OUTLINE = (28, 22, 20, 255)
SKIN = (232, 205, 177, 255)
SKIN_SHADE = (198, 166, 142, 255)
BLUSH = (208, 151, 132, 255)
WIG = (236, 236, 228, 255)
WIG_SHADE = (206, 205, 197, 255)
COAT = (36, 36, 40, 255)
COAT_HI = (70, 72, 82, 255)
VEST = (244, 242, 236, 255)
CRAVAT = (252, 251, 247, 255)
BREECH = (228, 226, 219, 255)
BOOT = (30, 28, 30, 255)
GOLD = (209, 176, 87, 255)
RAPIER = (190, 199, 212, 255)
GUNMETAL = (98, 103, 112, 255)
WOOD = (114, 74, 46, 255)
MUZZLE = (244, 218, 142, 176)
FX = (240, 220, 150, 160)
SHADOW = (90, 70, 54, 80)


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
        self.head = 0.0
        self.left_arm = 0.0
        self.right_arm = 0.0
        self.left_leg = 0.0
        self.right_leg = 0.0
        self.left_lift = 0.0
        self.right_lift = 0.0
        self.coat_sway = 0.0
        self.cravat = 0.0
        self.rapier = 0.0
        self.pistol = 0.0
        self.flash = 0.0
        self.open_mouth = 0.0
        self.dead_t = 0.0
        self.blink = False
        self.x_eye = False

        if anim == "idle":
            self.bob = s * 1.5
            self.tilt = s * 1.3
            self.head = -2.0 + s * 1.0
            self.left_arm = -4.0 + s * 2.0
            self.right_arm = 2.0 - s * 1.5
            self.left_leg = -2.0 + c * 1.2
            self.right_leg = 2.0 - c * 1.0
            self.coat_sway = s * 2.0
            self.cravat = max(0.0, s) * 2.0
            self.blink = frame_idx == nframes - 2
        elif anim == "walk":
            self.root_x = s * 2.0
            self.bob = abs(s) * 2.6 - 0.4
            self.tilt = s * 2.2
            self.head = -2.0 - s * 0.8
            self.left_leg = -22.0 * s
            self.right_leg = 20.0 * s
            self.left_lift = max(0.0, -s) * 8.0
            self.right_lift = max(0.0, s) * 7.0
            self.left_arm = 12.0 * s - 4.0
            self.right_arm = -10.0 * s + 4.0
            self.coat_sway = -s * 6.0
        elif anim == "address":
            self.bob = s * 1.0
            self.tilt = -1.5 + s * 0.8
            self.head = -1.5 + s * 1.2
            self.left_arm = _lerp(-6.0, 30.0, math.sin(t * math.pi))
            self.right_arm = -4.0
            self.left_leg = -1.0
            self.right_leg = 2.0
            self.open_mouth = 0.08 + max(0.0, s) * 0.06
            self.coat_sway = s * 1.4
            self.cravat = 1.0 + max(0.0, s) * 2.0
        elif anim == "thrust":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-6.0, 18.0, tt)
            self.bob = -hit * 2.0
            self.tilt = _lerp(-6.0, 10.0, tt)
            self.head = _lerp(-4.0, 8.0, tt)
            self.left_arm = _lerp(-18.0, 56.0, tt)
            self.right_arm = _lerp(6.0, -24.0, tt)
            self.left_leg = _lerp(-10.0, 16.0, tt)
            self.right_leg = _lerp(8.0, -6.0, tt)
            self.left_lift = _lerp(0.0, 6.0, tt)
            self.coat_sway = _lerp(6.0, -14.0, tt)
            self.rapier = hit
        elif anim == "pistol":
            tt = _ease(t)
            self.root_x = _lerp(-4.0, 8.0, tt)
            self.bob = -math.sin(tt * math.pi) * 1.6
            self.tilt = _lerp(-2.0, 4.0, tt)
            self.head = _lerp(-2.0, 3.0, tt)
            self.left_arm = -8.0
            self.right_arm = _lerp(-12.0, 40.0, tt)
            self.left_leg = -4.0
            self.right_leg = 4.0
            self.coat_sway = _lerp(3.0, -6.0, tt)
            self.pistol = tt
            self.flash = 1.0 if frame_idx == nframes - 2 else 0.0
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 5.0) * (1.0 - t)
            self.root_x = shake * 3.0 - hit * 3.5
            self.bob = -hit * 2.0
            self.tilt = -9.0 * hit
            self.head = 6.0 * hit
            self.left_arm = 12.0 * hit
            self.right_arm = 16.0 * hit
            self.left_leg = -8.0 * hit
            self.right_leg = 7.0 * hit
            self.coat_sway = -8.0 * hit
            self.open_mouth = 0.10 * hit
        elif anim == "death":
            tt = _ease(t)
            self.dead_t = tt
            self.root_x = tt * 14.0
            self.root_y = tt * 8.0
            self.bob = -tt * 4.0
            self.tilt = -78.0 * tt
            self.head = -16.0 * tt
            self.left_arm = _lerp(-4.0, 48.0, tt)
            self.right_arm = _lerp(4.0, -52.0, tt)
            self.left_leg = _lerp(-2.0, 18.0, tt)
            self.right_leg = _lerp(2.0, -18.0, tt)
            self.coat_sway = -20.0 * tt
            self.x_eye = tt > 0.58


def _draw_leg(
    draw: ImageDraw.ImageDraw, hip: Point, thigh_ang: float, lift: float, front: bool
) -> Point:
    thigh_len = 46
    shin_len = 44
    knee = (
        hip[0] + thigh_len * math.cos(math.radians(thigh_ang)),
        hip[1] + thigh_len * math.sin(math.radians(thigh_ang)),
    )
    ankle = (
        knee[0] + shin_len * math.cos(math.radians(thigh_ang + 10)),
        knee[1] + shin_len * math.sin(math.radians(thigh_ang + 10)) - lift,
    )
    col = BREECH if front else (212, 210, 205, 255)
    _line(draw, [hip, knee, ankle], col, 8.0 if front else 7.0)
    _line(draw, [hip, knee, ankle], OUTLINE, 1.1)
    _ellipse(draw, knee[0], knee[1], 5.2, 5.6, col, OUTLINE, 0.5)
    boot = [
        (ankle[0] - 7, ankle[1] - 6),
        (ankle[0] + 10, ankle[1] - 6),
        (ankle[0] + 16, ankle[1] + 4),
        (ankle[0] + 6, ankle[1] + 10),
        (ankle[0] - 8, ankle[1] + 8),
    ]
    _poly(draw, boot, BOOT, OUTLINE, 0.8)
    return ankle


def _draw_hand(draw: ImageDraw.ImageDraw, p: Point, r: float = 4.4) -> None:
    _ellipse(draw, p[0], p[1], r, r * 0.88, SKIN, OUTLINE, 0.5)


def _render_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    img = Image.new(
        "RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0)
    )
    draw = ImageDraw.Draw(img, "RGBA")
    pose = Pose(anim, frame_idx, nframes)

    root = (
        WORK_FRAME_SIZE[0] * 0.47 + pose.root_x,
        WORK_FRAME_SIZE[1] * 0.77 + pose.root_y + pose.bob,
    )
    tilt = pose.tilt

    def P(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, tilt)
        return (root[0] + rx, root[1] + ry)

    # subtle drop shadow
    _ellipse(draw, root[0] + 6, root[1] + 14, 54, 12, SHADOW, None, 0)

    # far leg first
    far_hip = P(10, -104)
    _draw_leg(draw, far_hip, 92 + pose.right_leg, pose.right_lift, False)

    # coat tails behind body
    tail_l = [
        P(-18, -102),
        P(-6, -38),
        P(-20 + pose.coat_sway * 0.4, 22),
        P(-2, 18),
        P(10, -24),
        P(2, -102),
    ]
    tail_r = [
        P(8, -102),
        P(12, -34),
        P(28 + pose.coat_sway * 0.55, 18),
        P(44, 12),
        P(32, -40),
        P(26, -102),
    ]
    _poly(draw, tail_l, COAT, OUTLINE, 1.0)
    _poly(draw, tail_r, COAT, OUTLINE, 1.0)

    # torso/coat
    torso = [
        P(-34, -200),
        P(6, -214),
        P(36, -198),
        P(48, -148),
        P(44, -100),
        P(22, -76),
        P(-10, -72),
        P(-36, -94),
        P(-42, -148),
    ]
    _poly(draw, torso, COAT, OUTLINE, 1.2)
    lapel_l = [P(-12, -188), P(0, -194), P(-4, -116), P(-18, -100), P(-26, -124)]
    lapel_r = [P(10, -190), P(22, -188), P(34, -124), P(18, -98), P(6, -118)]
    _poly(draw, lapel_l, COAT_HI, OUTLINE, 0.6)
    _poly(draw, lapel_r, COAT_HI, OUTLINE, 0.6)
    vest = [
        P(-8, -196),
        P(14, -194),
        P(18, -104),
        P(-2, -92),
        P(-18, -108),
        P(-18, -176),
    ]
    _poly(draw, vest, VEST, OUTLINE, 0.8)
    cravat = [
        P(-2, -202),
        P(10, -202),
        P(14, -174 + pose.cravat * 0.2),
        P(6, -144),
        P(-4, -170),
        P(-10, -184),
    ]
    _poly(draw, cravat, CRAVAT, OUTLINE, 0.7)
    folds = [P(0, -182), P(8, -166), P(2, -152), P(12, -138)]
    _line(draw, folds, (214, 214, 210, 255), 0.8)

    # far arm
    far_shoulder = P(28, -182)
    far_elbow = P(40 + pose.right_arm * 0.18, -138 + pose.right_arm * 0.10)
    far_hand = P(46 + pose.right_arm * 0.28, -92 + pose.right_arm * 0.12)
    _line(draw, [far_shoulder, far_elbow, far_hand], COAT, 7.2)
    _line(draw, [far_shoulder, far_elbow, far_hand], OUTLINE, 1.0)
    _draw_hand(draw, far_hand, 4.2)

    # head + wig
    head_root = P(-2, -246)
    head_ang = tilt + pose.head

    def H(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, head_ang)
        return (head_root[0] + rx, head_root[1] + ry)

    wig_back = [
        H(-28, -10),
        H(-22, -40),
        H(0, -54),
        H(20, -46),
        H(32, -20),
        H(30, 12),
        H(22, 26),
        H(12, 24),
        H(8, 0),
        H(-20, 8),
    ]
    _poly(draw, wig_back, WIG_SHADE, OUTLINE, 1.0)
    side_left = [H(-34, -4), H(-48, 10), H(-50, 28), H(-38, 42), H(-18, 34), H(-20, 12)]
    side_right = [H(30, -4), H(46, 6), H(50, 24), H(42, 42), H(24, 36), H(22, 8)]
    _poly(draw, side_left, WIG, OUTLINE, 0.8)
    _poly(draw, side_right, WIG, OUTLINE, 0.8)
    head = [
        H(-20, -18),
        H(-10, -36),
        H(12, -40),
        H(28, -26),
        H(30, 0),
        H(16, 20),
        H(-6, 24),
        H(-24, 10),
    ]
    _poly(draw, head, SKIN, OUTLINE, 1.0)
    _ellipse(draw, H(8, -2)[0], H(8, -2)[1], 9.0, 7.2, BLUSH, None, 0)
    _ellipse(draw, H(-8, 0)[0], H(-8, 0)[1], 8.5, 6.8, BLUSH, None, 0)

    # facial features
    if pose.x_eye:
        _line(draw, [H(-6, -6), H(0, 0)], OUTLINE, 0.8)
        _line(draw, [H(-6, 0), H(0, -6)], OUTLINE, 0.8)
        _line(draw, [H(12, -6), H(18, 0)], OUTLINE, 0.8)
        _line(draw, [H(12, 0), H(18, -6)], OUTLINE, 0.8)
    elif pose.blink:
        _line(draw, [H(-8, -3), H(0, -3)], OUTLINE, 0.8)
        _line(draw, [H(10, -4), H(18, -4)], OUTLINE, 0.8)
    else:
        _ellipse(
            draw,
            H(-4, -3)[0],
            H(-4, -3)[1],
            3.5,
            2.8,
            (239, 241, 240, 255),
            OUTLINE,
            0.4,
        )
        _ellipse(
            draw,
            H(14, -4)[0],
            H(14, -4)[1],
            3.5,
            2.8,
            (239, 241, 240, 255),
            OUTLINE,
            0.4,
        )
        _circle(draw, H(-3, -3), 0.9, (36, 44, 54, 255), (36, 44, 54, 255), 0.1)
        _circle(draw, H(15, -4), 0.9, (36, 44, 54, 255), (36, 44, 54, 255), 0.1)
        _line(draw, [H(-9, -8), H(-1, -10)], OUTLINE, 0.5)
        _line(draw, [H(10, -9), H(18, -10)], OUTLINE, 0.5)
    nose = [H(6, -2), H(10, 6), H(4, 10), H(2, 4)]
    _poly(draw, nose, SKIN_SHADE, OUTLINE, 0.3)
    if pose.open_mouth > 0.02:
        _ellipse(
            draw,
            H(7, 14)[0],
            H(7, 14)[1],
            4.6,
            2.4 + pose.open_mouth * 10.0,
            (102, 62, 66, 255),
            OUTLINE,
            0.4,
        )
    else:
        _line(draw, [H(2, 14), H(8, 15), H(14, 14)], (114, 76, 72, 255), 0.7)

    # near leg
    near_hip = P(-12, -104)
    _draw_leg(draw, near_hip, 92 + pose.left_leg, pose.left_lift, True)

    # near arm with weapon / gesturing
    near_shoulder = P(-28, -182)
    near_elbow = P(-40 + pose.left_arm * 0.20, -140 + pose.left_arm * 0.10)
    near_hand = P(-46 + pose.left_arm * 0.36, -96 + pose.left_arm * 0.14)
    _line(draw, [near_shoulder, near_elbow, near_hand], COAT, 7.6)
    _line(draw, [near_shoulder, near_elbow, near_hand], OUTLINE, 1.0)
    _draw_hand(draw, near_hand, 4.4)

    if anim == "thrust":
        guard = (near_hand[0] + 6, near_hand[1] + 2)
        blade_tip = (guard[0] + 94 + pose.rapier * 40, guard[1] - 8)
        _line(draw, [guard, blade_tip], RAPIER, 1.6)
        _line(draw, [guard, (guard[0] + 12, guard[1] - 1)], OUTLINE, 0.5)
        _poly(
            draw,
            [
                (guard[0] - 2, guard[1] - 4),
                (guard[0] + 8, guard[1] - 2),
                (guard[0] + 8, guard[1] + 4),
                (guard[0] - 2, guard[1] + 2),
            ],
            GOLD,
            OUTLINE,
            0.4,
        )
        if pose.rapier > 0.2:
            cx, cy = blade_tip
            box = (_s(cx - 40), _s(cy - 20), _s(cx + 24), _s(cy + 26))
            draw.arc(box, 200, 350, fill=FX, width=_s(3.0))
    elif anim == "pistol":
        gun_base = (far_hand[0] + 6, far_hand[1] - 1)
        barrel = (gun_base[0] + 44 + pose.pistol * 8, gun_base[1] - 2)
        _poly(
            draw,
            [
                (gun_base[0] - 3, gun_base[1] - 3),
                (gun_base[0] + 10, gun_base[1] - 4),
                (gun_base[0] + 14, gun_base[1] + 2),
                (gun_base[0] + 2, gun_base[1] + 4),
            ],
            WOOD,
            OUTLINE,
            0.4,
        )
        _line(draw, [(gun_base[0] + 8, gun_base[1] - 1), barrel], GUNMETAL, 1.8)
        if pose.flash > 0.5:
            cx, cy = barrel
            _poly(
                draw,
                [(cx, cy), (cx + 16, cy - 6), (cx + 26, cy), (cx + 16, cy + 6)],
                MUZZLE,
                None,
                0,
            )

    # buttons and cuff details
    for y in [-176, -154, -132]:
        _ellipse(draw, P(4, y)[0], P(4, y)[1], 2.0, 2.0, GOLD, OUTLINE, 0.3)
    _line(draw, [P(-36, -126), P(-28, -126)], CRAVAT, 0.8)
    _line(draw, [P(42, -124), P(50, -124)], CRAVAT, 0.8)

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
        description="Render the portrait-inspired Colonial Statesman sprite sheet."
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
