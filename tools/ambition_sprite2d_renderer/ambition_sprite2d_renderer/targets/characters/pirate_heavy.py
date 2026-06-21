from __future__ import annotations

"""Standalone generator for several pirate-heavy sprite variants.

This target deliberately does NOT route through the pirate-family
``draw_character`` rig (see ``_pirate_common.draw_character``). It only
reuses ``build_sheet`` (from ``tackon_sheet``) for the Ambition-compatible PNG/YAML/RON sheet
layout. The art template is a bespoke heavy bruiser silhouette: huge shoulders,
big arms, bandana hair, skirt, heavy boots, and a massive cleaver/anchor weapon.

Unlike the earlier single-character versions, this module can render a small
family of related pirate-heavy variants with different proportions, skin tones,
accessories, and small facial / costume features so they feel like related crew
members rather than the exact same sprite recolored.
"""

import argparse
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Sequence, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet

ACTOR_METADATA = {
    "actor": {"character_id": "npc_pirate_heavy", "display_name": "Pirate Heavy"},
    "body": {
        "body_plan": "HumanoidBiped",
        "body_kind": "Wide",
        "mass_class": "Heavy",
        "traits": ["enemy", "combatant", "pirate", "heavy"],
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
    "tags": ["story", "humanoid", "enemy", "combatant", "pirate", "heavy"],
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

BASE_TARGET_NAME = "pirate_heavy"
# Output (post-downsample) frame resolution. Heavy pirate bruisers render larger
# on screen than a normal character, so the texture needs more native pixels to
# stay crisp (a normal character ships at a 256 native frame). The body fills
# ~half the frame, so (640,576) yields a ~340px body. Geometry is authored in
# WORK_FRAME_SIZE units and supersampled by SUPER (=6), leaving ample detail to
# preserve at the higher downsample target — no redraw, no gameplay change
# (display size is collision-driven).
FRAME_SIZE = (640, 576)
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

OUTLINE = (20, 16, 18, 255)
METAL = (194, 202, 206, 255)
METAL_DARK = (104, 111, 118, 255)
GOLD = (232, 183, 68, 255)
TEETH = (250, 244, 220, 255)
SLASH = (255, 244, 190, 150)


@dataclass(frozen=True)
class Palette:
    skin: RGBA
    skin_shadow: RGBA
    hair: RGBA
    hair_hi: RGBA
    bandana: RGBA
    bandana_dark: RGBA
    shirt: RGBA
    bodice: RGBA
    bodice_hi: RGBA
    skirt: RGBA
    skirt_hi: RGBA
    sash: RGBA
    boot: RGBA
    boot_hi: RGBA
    slash: RGBA = SLASH


@dataclass(frozen=True)
class VariantSpec:
    slug: str
    display_name: str
    palette: Palette
    shoulder_scale: float = 1.0
    bust_scale: float = 1.0
    waist_scale: float = 1.0
    hip_scale: float = 1.0
    arm_scale: float = 1.0
    head_scale: float = 1.0
    cleaver_scale: float = 1.0
    cheek_scar: bool = False
    nose_ring: bool = False
    earrings: int = 0
    hair_beads: bool = False
    necklace: bool = False
    beauty_mark: bool = False
    brow_notch: bool = False
    freckles: bool = False


VARIANTS: Dict[str, VariantSpec] = {
    "broadside_bess": VariantSpec(
        slug="broadside_bess",
        display_name="Broadside Bess",
        palette=Palette(
            skin=(165, 105, 72, 255),
            skin_shadow=(106, 61, 43, 255),
            hair=(50, 31, 26, 255),
            hair_hi=(86, 54, 38, 255),
            bandana=(190, 36, 58, 255),
            bandana_dark=(120, 20, 38, 255),
            shirt=(235, 220, 190, 255),
            bodice=(60, 30, 44, 255),
            bodice_hi=(138, 82, 88, 255),
            skirt=(42, 68, 100, 255),
            skirt_hi=(72, 112, 154, 255),
            sash=(225, 170, 60, 255),
            boot=(57, 38, 29, 255),
            boot_hi=(98, 61, 38, 255),
        ),
        shoulder_scale=1.04,
        bust_scale=1.08,
        hip_scale=1.03,
        arm_scale=1.05,
        cheek_scar=True,
        earrings=2,
        necklace=True,
    ),
    "iron_mary": VariantSpec(
        slug="iron_mary",
        display_name="Iron Mary",
        palette=Palette(
            skin=(114, 78, 54, 255),
            skin_shadow=(73, 47, 31, 255),
            hair=(38, 24, 18, 255),
            hair_hi=(78, 51, 34, 255),
            bandana=(160, 26, 46, 255),
            bandana_dark=(98, 16, 30, 255),
            shirt=(236, 226, 196, 255),
            bodice=(46, 34, 53, 255),
            bodice_hi=(107, 82, 118, 255),
            skirt=(52, 88, 73, 255),
            skirt_hi=(84, 129, 109, 255),
            sash=(214, 164, 58, 255),
            boot=(54, 34, 26, 255),
            boot_hi=(88, 58, 40, 255),
            slash=(255, 238, 185, 150),
        ),
        shoulder_scale=1.10,
        bust_scale=1.15,
        waist_scale=0.96,
        hip_scale=1.02,
        arm_scale=1.12,
        cleaver_scale=1.07,
        cheek_scar=True,
        nose_ring=True,
        earrings=1,
        necklace=True,
        brow_notch=True,
    ),
    "salt_annet": VariantSpec(
        slug="salt_annet",
        display_name="Salt Annet",
        palette=Palette(
            skin=(205, 146, 110, 255),
            skin_shadow=(140, 93, 66, 255),
            hair=(71, 45, 30, 255),
            hair_hi=(120, 84, 56, 255),
            bandana=(168, 53, 84, 255),
            bandana_dark=(111, 32, 56, 255),
            shirt=(244, 230, 200, 255),
            bodice=(73, 42, 54, 255),
            bodice_hi=(158, 103, 118, 255),
            skirt=(45, 83, 112, 255),
            skirt_hi=(92, 139, 176, 255),
            sash=(239, 190, 74, 255),
            boot=(69, 43, 29, 255),
            boot_hi=(111, 72, 44, 255),
        ),
        shoulder_scale=0.98,
        bust_scale=1.12,
        waist_scale=0.93,
        hip_scale=1.08,
        arm_scale=0.95,
        head_scale=1.02,
        cleaver_scale=0.97,
        earrings=2,
        hair_beads=True,
        beauty_mark=True,
        freckles=True,
    ),
}


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
    c: Point,
    r: float,
    fill: RGBA,
    outline: RGBA = OUTLINE,
    width: float = 1.0,
) -> None:
    _ellipse(draw, c[0], c[1], r, r, fill, outline, width)


def _rect_poly(center: Point, w: float, h: float, deg: float) -> List[Point]:
    hw = w * 0.5
    hh = h * 0.5
    return [
        (center[0] + dx, center[1] + dy)
        for dx, dy in (
            _rot_local(-hw, -hh, deg),
            _rot_local(hw, -hh, deg),
            _rot_local(hw, hh, deg),
            _rot_local(-hw, hh, deg),
        )
    ]


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
        self.head_tilt = 0.0
        self.left_arm = 0.0
        self.right_arm = 0.0
        self.left_leg = 0.0
        self.right_leg = 0.0
        self.weapon = -34.0
        self.weapon_lift = 0.0
        self.left_foot_lift = 0.0
        self.right_foot_lift = 0.0
        self.skirt_sway = 0.0
        self.mouth = 0.05
        self.blink = False
        self.x_eyes = False
        self.death_t = 0.0
        self.impact = 0.0

        if anim == "idle":
            self.root_x = s * 1.2
            self.bob = s * 1.8
            self.tilt = s * 1.8
            self.head_tilt = -2.0 + s * 2.4
            self.left_arm = -7.0 + s * 5.0
            self.right_arm = 5.0 - s * 4.0
            self.left_leg = -2.0 + c * 1.5
            self.right_leg = 2.0 - c * 1.5
            self.weapon = -38.0 + s * 4.0
            self.skirt_sway = s * 4.0
            self.mouth = max(0.0, s) * 0.06
            self.blink = frame_idx == nframes - 2
        elif anim == "walk":
            self.root_x = s * 2.2
            self.bob = abs(s) * 3.4 - 0.8
            self.tilt = s * 4.0
            self.head_tilt = -2.0 - s * 2.0
            self.left_leg = -22.0 * s
            self.right_leg = 22.0 * s
            self.left_arm = 16.0 * s - 5.0
            self.right_arm = -13.0 * s + 3.0
            self.weapon = -42.0 - s * 8.0
            self.left_foot_lift = max(0.0, -s) * 8.0
            self.right_foot_lift = max(0.0, s) * 8.0
            self.skirt_sway = -s * 9.0
        elif anim == "slash":
            tt = _ease(t)
            hit = math.sin(tt * math.pi)
            self.root_x = _lerp(-7.0, 7.0, tt)
            self.bob = -hit * 4.0
            self.tilt = _lerp(-13.0, 20.0, tt)
            self.head_tilt = _lerp(-11.0, 5.0, tt)
            self.left_arm = _lerp(-28.0, 18.0, tt) + hit * 8.0
            self.right_arm = _lerp(-112.0, 12.0, tt)
            self.weapon = _lerp(-142.0, -8.0, tt)
            self.weapon_lift = -hit * 9.0 + tt * 22.0
            self.left_leg = -10.0 - hit * 4.0
            self.right_leg = 13.0 + hit * 5.0
            self.skirt_sway = _lerp(13.0, -12.0, tt)
            self.mouth = 0.40 * hit + 0.12
            self.impact = hit
        elif anim == "taunt":
            self.root_x = s * 1.0
            self.bob = s * 1.4
            self.tilt = -7.0 + s * 2.0
            self.head_tilt = -9.0 + s * 4.0
            self.left_arm = -98.0 + s * 8.0
            self.right_arm = -14.0 + s * 6.0
            self.weapon = -82.0 + s * 5.0
            self.skirt_sway = s * 4.0
            self.mouth = 0.40 + max(0.0, s) * 0.16
        elif anim == "hurt":
            hit = math.sin(t * math.pi)
            shake = math.sin(t * math.pi * 5.0) * (1.0 - t)
            self.root_x = shake * 5.0
            self.bob = -hit * 3.0
            self.tilt = -16.0 * hit
            self.head_tilt = 14.0 * hit
            self.left_arm = 30.0 * hit
            self.right_arm = 22.0 * hit
            self.weapon = -58.0 + 18.0 * hit
            self.left_leg = 10.0 * hit
            self.right_leg = -8.0 * hit
            self.skirt_sway = -10.0 * hit
            self.mouth = 0.35 * hit
        elif anim == "death":
            tt = _ease(t)
            self.death_t = tt
            self.root_x = tt * 15.0
            self.root_y = tt * 5.0
            self.bob = -tt * 5.0
            self.tilt = -78.0 * tt
            self.head_tilt = 26.0 * tt
            self.left_arm = _lerp(-8.0, 60.0, tt)
            self.right_arm = _lerp(4.0, -70.0, tt)
            self.weapon = _lerp(-38.0, -128.0, tt)
            self.left_leg = _lerp(-2.0, 30.0, tt)
            self.right_leg = _lerp(2.0, -28.0, tt)
            self.skirt_sway = 14.0 * tt
            self.mouth = 0.30 * tt
            self.x_eyes = tt > 0.55


def _draw_cleaver(
    draw: ImageDraw.ImageDraw,
    hand: Point,
    angle: float,
    pal: Palette,
    scale: float = 1.0,
    front: bool = True,
) -> None:
    def tr(x: float, y: float) -> Point:
        rx, ry = _rot_local(x * scale, y * scale, angle)
        return (hand[0] + rx, hand[1] + ry)

    _line(draw, [tr(-6, 7), tr(55, -1), tr(85, -2)], OUTLINE, 6.4 * scale)
    _line(draw, [tr(-6, 7), tr(55, -1), tr(85, -2)], (88, 54, 34, 255), 4.4 * scale)
    _poly(
        draw,
        _rect_poly(tr(1, 6), 14 * scale, 8 * scale, angle - 8),
        GOLD,
        OUTLINE,
        1.6 * scale,
    )
    _circle(draw, tr(-9, 7), 4.0 * scale, GOLD, OUTLINE, 1.3 * scale)

    blade = [
        tr(54, -25),
        tr(72, -34),
        tr(91, -27),
        tr(96, -12),
        tr(84, -3),
        tr(98, 13),
        tr(92, 29),
        tr(72, 35),
        tr(55, 24),
        tr(65, 6),
        tr(50, 0),
    ]
    _poly(draw, blade, METAL, OUTLINE, 2.0 * scale)
    notch = [tr(78, -18), tr(87, -14), tr(82, -7)]
    _poly(draw, notch, (145, 154, 160, 255), OUTLINE, 1.0 * scale)
    _line(draw, [tr(58, -18), tr(87, 22)], (238, 244, 244, 185), 1.4 * scale)
    _line(draw, [tr(49, 0), tr(72, 0)], METAL_DARK, 1.2 * scale)
    if front:
        for x, y in [(68, -20), (82, -20), (86, 18), (69, 24)]:
            _circle(draw, tr(x, y), 2.0 * scale, METAL_DARK, OUTLINE, 0.6 * scale)


def _draw_boot(
    draw: ImageDraw.ImageDraw, p: Point, toe_dir: float, pal: Palette
) -> None:
    _poly(
        draw,
        _rect_poly((p[0], p[1] - 6), 15, 20, toe_dir * 0.25),
        pal.boot,
        OUTLINE,
        1.6,
    )
    toe = [
        (p[0] - 7, p[1] - 2),
        (p[0] + 10 + toe_dir * 2, p[1] - 2),
        (p[0] + 15 + toe_dir * 3, p[1] + 5),
        (p[0] - 6, p[1] + 6),
    ]
    _poly(draw, toe, pal.boot_hi, OUTLINE, 1.4)
    _line(draw, [(p[0] - 5, p[1] - 12), (p[0] + 6, p[1] - 10)], GOLD, 1.0)


def _draw_face(
    draw: ImageDraw.ImageDraw, head: Point, pose: Pose, spec: VariantSpec
) -> None:
    pal = spec.palette
    hx, hy = head
    hrx = 21 * spec.head_scale
    hry = 25 * spec.head_scale
    hair_back = [
        (hx - 23, hy - 14),
        (hx - 9, hy - 28),
        (hx + 16, hy - 25),
        (hx + 28, hy - 10),
        (hx + 22, hy + 18),
        (hx + 10, hy + 30),
        (hx - 14, hy + 28),
        (hx - 27, hy + 8),
    ]
    _poly(draw, hair_back, pal.hair, OUTLINE, 1.5)
    _poly(
        draw,
        [(hx - 26, hy + 0), (hx - 37, hy + 21), (hx - 25, hy + 34), (hx - 16, hy + 14)],
        pal.hair,
        OUTLINE,
        1.4,
    )
    _poly(
        draw,
        [(hx + 20, hy + 3), (hx + 35, hy + 18), (hx + 24, hy + 31), (hx + 14, hy + 14)],
        pal.hair,
        OUTLINE,
        1.4,
    )

    if spec.hair_beads:
        for bead_pt in [(hx - 31, hy + 24), (hx - 25, hy + 31), (hx + 27, hy + 29)]:
            _circle(draw, bead_pt, 2.8, GOLD, OUTLINE, 0.6)

    _ellipse(draw, hx, hy, hrx, hry, pal.skin, OUTLINE, 1.8)
    jaw = [
        (hx - 18, hy + 9),
        (hx - 9, hy + 28),
        (hx + 10, hy + 28),
        (hx + 19, hy + 9),
        (hx + 12, hy + 22),
        (hx - 10, hy + 22),
    ]
    _poly(draw, jaw, pal.skin, OUTLINE, 1.2)

    band = [
        (hx - 23, hy - 13),
        (hx - 13, hy - 22),
        (hx + 12, hy - 22),
        (hx + 24, hy - 13),
        (hx + 19, hy - 6),
        (hx - 20, hy - 6),
    ]
    _poly(draw, band, pal.bandana, OUTLINE, 1.4)
    _circle(draw, (hx + 22, hy - 12), 4.5, pal.bandana_dark, OUTLINE, 1.0)
    _poly(
        draw,
        [(hx + 25, hy - 11), (hx + 42, hy - 18), (hx + 36, hy - 4)],
        pal.bandana,
        OUTLINE,
        1.0,
    )
    _poly(
        draw,
        [(hx + 24, hy - 8), (hx + 39, hy + 6), (hx + 29, hy + 8)],
        pal.bandana_dark,
        OUTLINE,
        1.0,
    )
    _line(draw, [(hx - 17, hy - 16), (hx + 14, hy - 17)], (238, 92, 100, 210), 1.0)

    _line(
        draw,
        [(hx - 13, hy - 2), (hx - 18, hy + 16), (hx - 12, hy + 27)],
        pal.hair_hi,
        1.5,
    )
    _line(
        draw,
        [(hx + 10, hy - 1), (hx + 16, hy + 16), (hx + 9, hy + 27)],
        pal.hair_hi,
        1.2,
    )

    eye_y = hy + 2
    if pose.x_eyes:
        for ex in [hx - 8, hx + 8]:
            _line(draw, [(ex - 4, eye_y - 4), (ex + 4, eye_y + 4)], OUTLINE, 1.2)
            _line(draw, [(ex - 4, eye_y + 4), (ex + 4, eye_y - 4)], OUTLINE, 1.2)
    else:
        left_brow_y = eye_y - 5
        right_brow_y = eye_y - 6
        _line(draw, [(hx - 15, left_brow_y), (hx - 6, eye_y - 2)], OUTLINE, 1.4)
        _line(draw, [(hx + 5, eye_y - 2), (hx + 16, right_brow_y)], OUTLINE, 1.4)
        if spec.brow_notch:
            _line(
                draw,
                [(hx - 10, left_brow_y - 2), (hx - 8, left_brow_y + 2)],
                pal.skin_shadow,
                0.9,
            )
        if pose.blink:
            _line(draw, [(hx - 13, eye_y + 1), (hx - 6, eye_y + 1)], OUTLINE, 1.1)
            _line(draw, [(hx + 6, eye_y + 1), (hx + 13, eye_y + 1)], OUTLINE, 1.1)
        else:
            _ellipse(draw, hx - 10, eye_y + 1, 4, 3, (252, 248, 236, 255), OUTLINE, 0.8)
            _ellipse(draw, hx + 10, eye_y + 1, 4, 3, (252, 248, 236, 255), OUTLINE, 0.8)
            _circle(draw, (hx - 9, eye_y + 1), 1.4, OUTLINE, OUTLINE, 0.2)
            _circle(draw, (hx + 9, eye_y + 1), 1.4, OUTLINE, OUTLINE, 0.2)
            _line(draw, [(hx - 14, eye_y - 1), (hx - 17, eye_y - 5)], OUTLINE, 0.9)
            _line(draw, [(hx - 10, eye_y - 3), (hx - 11, eye_y - 7)], OUTLINE, 0.9)
            _line(draw, [(hx - 6, eye_y - 1), (hx - 3, eye_y - 5)], OUTLINE, 0.9)
            _line(draw, [(hx + 6, eye_y - 1), (hx + 3, eye_y - 5)], OUTLINE, 0.9)
            _line(draw, [(hx + 10, eye_y - 3), (hx + 11, eye_y - 7)], OUTLINE, 0.9)
            _line(draw, [(hx + 14, eye_y - 1), (hx + 17, eye_y - 5)], OUTLINE, 0.9)

    if spec.cheek_scar:
        _line(
            draw,
            [(hx + 8, hy + 8), (hx + 13, hy + 15), (hx + 10, hy + 20)],
            pal.skin_shadow,
            1.0,
        )
    if spec.freckles:
        for dx, dy in [(-8, 10), (-4, 12), (5, 12), (9, 10)]:
            _circle(
                draw, (hx + dx, hy + dy), 0.8, pal.skin_shadow, pal.skin_shadow, 0.1
            )
    if spec.beauty_mark:
        _circle(draw, (hx + 12, hy + 18), 1.1, OUTLINE, OUTLINE, 0.1)
    if spec.nose_ring:
        _line(
            draw, [(hx + 1, hy + 10), (hx + 4, hy + 11), (hx + 2, hy + 14)], GOLD, 0.8
        )

    _line(
        draw, [(hx, hy + 3), (hx + 3, hy + 11), (hx - 2, hy + 13)], pal.skin_shadow, 1.0
    )
    mouth_y = hy + 19 + pose.mouth * 3.5
    if pose.mouth > 0.2:
        _ellipse(
            draw,
            hx,
            mouth_y - 1,
            7,
            3 + pose.mouth * 3.0,
            (70, 24, 32, 255),
            OUTLINE,
            1.0,
        )
        _line(draw, [(hx - 5, mouth_y - 3), (hx + 5, mouth_y - 3)], TEETH, 1.0)
    else:
        _line(
            draw,
            [(hx - 7, mouth_y), (hx, mouth_y + 2), (hx + 8, mouth_y - 1)],
            OUTLINE,
            1.1,
        )

    if spec.earrings >= 1:
        _circle(draw, (hx + 25, hy + 11), 2.2, GOLD, OUTLINE, 0.6)
    if spec.earrings >= 2:
        _circle(draw, (hx - 24, hy + 11), 2.2, GOLD, OUTLINE, 0.6)


def _draw_torso(draw: ImageDraw.ImageDraw, p, pose: Pose, spec: VariantSpec) -> None:
    pal = spec.palette
    P = p
    shoulder_x = 40 * spec.shoulder_scale
    chest_x = 36 * spec.bust_scale
    waist_x = 18 * spec.waist_scale
    skirt_x = 50 * spec.hip_scale

    shoulders = [
        P(-shoulder_x, -98),
        P(-24, -114),
        P(28, -114),
        P(shoulder_x + 5, -97),
        P(29, -71),
        P(-29, -71),
    ]
    _poly(draw, shoulders, pal.shirt, OUTLINE, 2.0)
    _ellipse(
        draw,
        *P(-42 * spec.shoulder_scale, -96),
        13 * spec.arm_scale,
        12 * spec.arm_scale,
        pal.shirt,
        OUTLINE,
        1.6,
    )
    _ellipse(
        draw,
        *P(43 * spec.shoulder_scale, -96),
        13 * spec.arm_scale,
        12 * spec.arm_scale,
        pal.shirt,
        OUTLINE,
        1.6,
    )

    upper = [
        P(-36 * spec.bust_scale, -103),
        P(-21 * spec.bust_scale, -114),
        P(17 * spec.bust_scale, -114),
        P(37 * spec.bust_scale, -102),
        P(35 * spec.bust_scale, -74),
        P(22 * spec.bust_scale, -63),
        P(0, -57),
        P(-23 * spec.bust_scale, -63),
        P(-36 * spec.bust_scale, -74),
    ]
    _poly(draw, upper, pal.bodice, OUTLINE, 1.8)
    left_plate = [
        P(-33 * spec.bust_scale, -100),
        P(-12 * spec.bust_scale, -109),
        P(-1, -99),
        P(-2, -84),
        P(-18 * spec.bust_scale, -68),
        P(-33 * spec.bust_scale, -78),
    ]
    right_plate = [
        P(11 * spec.bust_scale, -109),
        P(33 * spec.bust_scale, -100),
        P(33 * spec.bust_scale, -78),
        P(18 * spec.bust_scale, -68),
        P(2, -84),
        P(1, -99),
    ]
    _poly(draw, left_plate, pal.bodice_hi, OUTLINE, 1.0)
    _poly(draw, right_plate, pal.bodice_hi, OUTLINE, 1.0)
    _line(
        draw,
        [
            P(-24 * spec.bust_scale, -81),
            P(-11 * spec.bust_scale, -71),
            P(0, -67),
            P(12 * spec.bust_scale, -71),
            P(25 * spec.bust_scale, -81),
        ],
        (94, 52, 64, 255),
        1.4,
    )
    _line(draw, [P(0, -109), P(0, -58)], OUTLINE, 1.0)
    _poly(
        draw,
        [
            P(-9 * spec.waist_scale, -93),
            P(9 * spec.waist_scale, -93),
            P(7 * spec.waist_scale, -74),
            P(-8 * spec.waist_scale, -74),
        ],
        (88, 45, 58, 255),
        OUTLINE,
        0.8,
    )
    _line(
        draw,
        [P(-24 * spec.bust_scale, -99), P(-8 * spec.bust_scale, -105)],
        pal.bodice_hi,
        1.0,
    )
    _line(
        draw,
        [P(8 * spec.bust_scale, -105), P(24 * spec.bust_scale, -99)],
        pal.bodice_hi,
        1.0,
    )
    _line(
        draw,
        [P(-18 * spec.bust_scale, -71), P(-7 * spec.bust_scale, -63)],
        pal.bodice_hi,
        0.8,
    )
    _line(
        draw,
        [P(7 * spec.bust_scale, -63), P(18 * spec.bust_scale, -71)],
        pal.bodice_hi,
        0.8,
    )

    sash = [
        P(-35 * spec.hip_scale, -66),
        P(34 * spec.hip_scale, -65),
        P(30 * spec.hip_scale, -54),
        P(-34 * spec.hip_scale, -55),
    ]
    _poly(draw, sash, pal.sash, OUTLINE, 1.4)
    _poly(draw, [P(-6, -68), P(10, -67), P(8, -53), P(-8, -54)], GOLD, OUTLINE, 1.0)

    sway = pose.skirt_sway
    skirt = [
        P(-34 * spec.hip_scale, -54),
        P(31 * spec.hip_scale, -54),
        P(skirt_x + sway * 0.18, -7),
        P(24 * spec.hip_scale, 7),
        P(0, 0),
        P(-25 * spec.hip_scale, 8),
        P(-skirt_x + sway * 0.18, -7),
    ]
    _poly(draw, skirt, pal.skirt, OUTLINE, 2.0)
    pleats = [
        (-26, -50, -36, 0),
        (-10, -51, -15, 2),
        (10, -51, 14, 2),
        (27, -50, 36, -1),
    ]
    for x1, y1, x2, y2 in pleats:
        _line(
            draw,
            [
                P(x1 * spec.hip_scale + sway * 0.04, y1),
                P(x2 * spec.hip_scale + sway * 0.10, y2),
            ],
            pal.skirt_hi,
            1.2,
        )
    _line(
        draw,
        [
            P(-48 * spec.hip_scale + sway * 0.18, -6),
            P(-25 * spec.hip_scale, 8),
            P(0, 0),
            P(24 * spec.hip_scale, 7),
            P(skirt_x + sway * 0.18, -7),
        ],
        (26, 40, 62, 255),
        1.0,
    )

    if spec.necklace:
        for c in [P(-10, -61), P(0, -58), P(10, -61)]:
            _circle(draw, c, 1.8, GOLD, OUTLINE, 0.4)


def _draw_limbs(
    draw: ImageDraw.ImageDraw, p, pose: Pose, spec: VariantSpec
) -> Tuple[Point, Point]:
    pal = spec.palette
    P = p
    left_hip = P(-22 * spec.hip_scale, -47)
    right_hip = P(21 * spec.hip_scale, -47)
    left_knee = P(-25 * spec.hip_scale + pose.left_leg * 0.18, -18)
    right_knee = P(24 * spec.hip_scale + pose.right_leg * 0.18, -18)
    left_foot = P(-29 * spec.hip_scale + pose.left_leg * 0.16, 5 - pose.left_foot_lift)
    right_foot = P(
        31 * spec.hip_scale + pose.right_leg * 0.16, 5 - pose.right_foot_lift
    )
    for hip, knee, foot in [
        (left_hip, left_knee, left_foot),
        (right_hip, right_knee, right_foot),
    ]:
        _line(draw, [hip, knee, foot], pal.skin_shadow, 6.4)
        _line(draw, [hip, knee, foot], OUTLINE, 1.8)
        _draw_boot(draw, foot, 1 if foot[0] > hip[0] else -1, pal)

    left_shoulder = P(-44 * spec.shoulder_scale, -95)
    left_elbow = P(
        -55 * spec.shoulder_scale + pose.left_arm * 0.06, -63 + pose.left_arm * 0.15
    )
    left_hand = P(
        -42 * spec.shoulder_scale + pose.left_arm * 0.22, -41 + pose.left_arm * 0.24
    )
    _line(draw, [left_shoulder, left_elbow], pal.skin_shadow, 9.5 * spec.arm_scale)
    _line(draw, [left_elbow, left_hand], pal.skin, 8.4 * spec.arm_scale)
    _line(draw, [left_shoulder, left_elbow, left_hand], OUTLINE, 2.2)
    _ellipse(
        draw,
        left_elbow[0],
        left_elbow[1],
        8 * spec.arm_scale,
        10 * spec.arm_scale,
        pal.skin,
        OUTLINE,
        1.4,
    )
    _circle(draw, left_hand, 7.5 * spec.arm_scale, pal.skin, OUTLINE, 1.5)

    right_shoulder = P(44 * spec.shoulder_scale, -96)
    right_elbow = P(
        55 * spec.shoulder_scale + pose.right_arm * 0.05,
        -64 + pose.right_arm * 0.16 + pose.weapon_lift * 0.2,
    )
    right_hand = P(
        43 * spec.shoulder_scale + pose.right_arm * 0.24,
        -41 + pose.right_arm * 0.27 + pose.weapon_lift,
    )
    _line(draw, [right_shoulder, right_elbow], pal.skin_shadow, 10.0 * spec.arm_scale)
    _line(draw, [right_elbow, right_hand], pal.skin, 9.0 * spec.arm_scale)
    _line(draw, [right_shoulder, right_elbow, right_hand], OUTLINE, 2.2)
    _ellipse(
        draw,
        right_elbow[0],
        right_elbow[1],
        9 * spec.arm_scale,
        10 * spec.arm_scale,
        pal.skin,
        OUTLINE,
        1.4,
    )
    _circle(draw, right_hand, 8.2 * spec.arm_scale, pal.skin, OUTLINE, 1.5)

    _line(
        draw,
        [(left_hand[0] - 5, left_hand[1] - 6), (left_hand[0] + 6, left_hand[1] - 5)],
        GOLD,
        1.2,
    )
    _line(
        draw,
        [
            (right_hand[0] - 6, right_hand[1] - 6),
            (right_hand[0] + 7, right_hand[1] - 4),
        ],
        GOLD,
        1.2,
    )
    return left_hand, right_hand


def _draw_variant(
    anim: str, frame_idx: int, nframes: int, spec: VariantSpec
) -> Image.Image:
    img = Image.new(
        "RGBA", (WORK_FRAME_SIZE[0] * SUPER, WORK_FRAME_SIZE[1] * SUPER), (0, 0, 0, 0)
    )
    draw = ImageDraw.Draw(img, "RGBA")
    pose = Pose(anim, frame_idx, nframes)
    pal = spec.palette

    root = (
        WORK_FRAME_SIZE[0] * 0.46 + pose.root_x + pose.death_t * 8.0,
        WORK_FRAME_SIZE[1] * 0.67 + pose.root_y + pose.bob,
    )
    tilt = pose.tilt

    def P(x: float, y: float) -> Point:
        rx, ry = _rot_local(x, y, tilt)
        return (root[0] + rx, root[1] + ry)

    weapon_in_front = anim == "slash"
    if not weapon_in_front:
        back_hand = P(
            38 * spec.shoulder_scale + pose.right_arm * 0.12, -44 + pose.weapon_lift
        )
        _draw_cleaver(
            draw,
            back_hand,
            pose.weapon + tilt,
            pal,
            scale=spec.cleaver_scale,
            front=False,
        )

    if anim == "slash" and pose.impact > 0.10:
        cx, cy = P(38, -62)
        box = (_s(cx - 82), _s(cy - 82), _s(cx + 90), _s(cy + 52))
        draw.arc(box, 204, 334, fill=pal.slash, width=_s(6.0 + pose.impact * 2.0))
        draw.arc(box, 214, 326, fill=(255, 255, 255, 115), width=_s(2.5))

    head = P(0, -125 + pose.head_tilt * 0.10)
    hair_shadow = [
        P(-22, -137),
        P(-8, -153),
        P(18, -149),
        P(35, -127),
        P(26, -93),
        P(8, -83),
        P(-18, -87),
        P(-34, -109),
    ]
    _poly(draw, hair_shadow, pal.hair, OUTLINE, 1.4)

    _draw_limbs(draw, P, pose, spec)
    _draw_torso(draw, P, pose, spec)
    _poly(
        draw,
        [P(-11, -112), P(12, -112), P(9, -95), P(-10, -95)],
        pal.skin_shadow,
        OUTLINE,
        1.4,
    )
    _draw_face(draw, head, pose, spec)

    if weapon_in_front:
        hand = P(
            43 * spec.shoulder_scale + pose.right_arm * 0.24,
            -41 + pose.right_arm * 0.27 + pose.weapon_lift,
        )
        _draw_cleaver(
            draw,
            hand,
            pose.weapon + tilt,
            pal,
            scale=1.05 * spec.cleaver_scale,
            front=True,
        )

    if anim == "slash" and pose.impact > 0.45:
        for i, (dx, dy) in enumerate([(-44, 3), (-30, 9), (44, 2), (57, 10)]):
            jitter = math.sin(frame_idx + i) * 1.5
            c = P(dx + jitter, dy)
            _poly(
                draw,
                [(c[0] - 2.5, c[1] - 1.5), (c[0] + 3.0, c[1]), (c[0], c[1] + 3.0)],
                (118, 92, 62, 170),
                (72, 52, 36, 150),
                0.5,
            )

    return _downsample(img)


def render_variant(spec: VariantSpec, out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    frame_size = opts.get("frame_size", FRAME_SIZE)
    target_name = f"{BASE_TARGET_NAME}_{spec.slug}"
    outputs = build_sheet(
        target=target_name,
        rows=ROWS,
        render_fn=lambda anim, frame_idx, nframes: _draw_variant(
            anim, frame_idx, nframes, spec
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


def render(out_dir: str | Path, variant: str = "all", **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs: List[Path] = []
    if variant == "all":
        for slug, spec in VARIANTS.items():
            outputs.extend(render_variant(spec, out_dir / slug, **opts))
    else:
        spec = VARIANTS[variant]
        outputs.extend(render_variant(spec, out_dir, **opts))
    return outputs


# CLI hook for `render-publish pirate_heavy`. Each variant's sheet
# files live under `out_dir/{slug}/` from `render` above; this
# function walks every variant subdir and copies its PNG + YAML +
# RON to the destination sprites folder (flat, per-variant
# filenames). Mirrors the pattern in `sandbag.install`.
def install(out_dir: "Path | str", dest_root: "Path | str") -> List["Path"]:
    import shutil

    out_dir = Path(out_dir)
    dest_root = Path(dest_root)
    dest_root.mkdir(parents=True, exist_ok=True)
    copied: List[Path] = []
    for slug in VARIANTS.keys():
        target = f"{BASE_TARGET_NAME}_{slug}"
        for ext in ("png", "yaml", "ron"):
            src = out_dir / slug / f"{target}_spritesheet.{ext}"
            if not src.exists():
                continue
            dst = dest_root / src.name
            shutil.copy2(src, dst)
            copied.append(dst)
    return copied


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Render one or more pirate-heavy sprite variants."
    )
    parser.add_argument("--variant", choices=["all", *VARIANTS.keys()], default="all")
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path(__file__).resolve().parents[2]
        / "generated"
        / "pirate_heavy_variants",
    )
    args = parser.parse_args(argv)
    for path in render(args.out_dir, variant=args.variant):
        print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
