from __future__ import annotations

"""Standalone generator for a Flying Spaghetti Monster boss sprite sheet.

Large floating boss for the side scroller, rendered procedurally with PIL.
The silhouette leans into the iconic two meatballs, looping noodles, and
stalk-eyes, with bossy attack animations:
- hover / drift
- noodle whip lash
- meatball volley spit
- eye beam glare
- hurt / death feedback

Generator only. No registration or GUI wiring.
"""

import argparse
import math
from dataclasses import dataclass
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet

RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_NAME = "flying_spaghetti_monster_boss"
# Output (post-downsample) frame resolution. This is a BOSS that renders much
# larger on screen than a normal character, so its texture needs proportionally
# more native pixels to stay crisp — a normal character (the player) ships at a
# 256 native frame and reads sharp at its small on-screen size; a boss displayed
# 2-3x larger needs ~2-3x the native pixels for the same crispness. The body
# fills ~half the frame, so a (800,640) frame yields a ~400px body. Geometry is
# authored in WORK_FRAME_SIZE units and supersampled by SUPER, so raising the
# downsample target just preserves more of that detail (no redraw, no gameplay
# change — display size is collision-driven).
FRAME_SIZE = (800, 640)
WORK_FRAME_SIZE = (920, 840)
SUPER = 4
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 132),
    ("drift", 8, 98),
    ("noodle_whip", 7, 86),
    ("meatball_volley", 7, 88),
    ("eye_beam", 7, 90),
    ("hurt", 4, 96),
    ("death", 8, 112),
]

# Publish authoritative per-animation hurtboxes keyed by the GENERIC gameplay
# keys the boss combat looks animations up by (NOT these row names) — mirrors the
# Rust `FLYING_SPAGHETTI_MONSTER_SHEET` BossAnim mapping (idle→Rest,
# drift→DashEcho, noodle_whip→SideSweep, meatball_volley→FloorSlam,
# eye_beam→SpikeHalo, hurt→Hit, death→Death). Without this the boss falls back to
# the coarse idle alpha bbox (the whole noodle spread) for every pose; with it,
# the player's attacks register on the per-pose body.
ANIMATION_KEY_MAP = {
    "idle": "rest",
    "drift": "dash_echo",
    "noodle_whip": "side_sweep",
    "meatball_volley": "floor_slam",
    "eye_beam": "spike_halo",
    "hurt": "hit",
    "death": "death",
}

ACTOR_METADATA = {
    "actor": {
        "character_id": "npc_flying_spaghetti_monster_boss",
        "display_name": "Flying Spaghetti Monster Boss",
    },
    "body": {
        "body_plan": "BossMultipart",
        "body_kind": "Wide",
        "mass_class": "Boss",
        "locomotion_hint": "BossFloat",
        "traits": ["boss", "floating", "multipart", "no_hands", "noodle", "ranged"],
    },
    "capabilities": {
        "traversal": {
            "walk": False,
            "jump": None,
            "climb": None,
            "crawl": None,
            "fly": True,
            "swim": None,
            "use_lifts": None,
            "door_access": [],
        },
        "interactions": {
            "talk": None,
            "trade": None,
            "carry": None,
            "open_doors": [],
        },
    },
    "brain": {"default_preset": "boss_pattern"},
    "actions": {"default_preset": "fsm_boss_specials"},
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "locomotion.hover": {"animation": "drift", "events": []},
        "action.melee.primary": {
            "animation": "noodle_whip",
            "events": [
                {
                    "t": 0.34,
                    "event": "hitbox_active_start",
                    "source": "flying_spaghetti_monster_boss.noodle_whip",
                },
                {
                    "t": 0.62,
                    "event": "hitbox_active_end",
                    "source": "flying_spaghetti_monster_boss.noodle_whip",
                },
            ],
        },
        "action.ranged.primary": {
            "animation": "meatball_volley",
            "events": [
                {
                    "t": 0.48,
                    "event": "projectile_release",
                    "source": "flying_spaghetti_monster_boss.meatball_volley",
                }
            ],
        },
        "action.special.eye_beam": {
            "animation": "eye_beam",
            "events": [
                {
                    "t": 0.44,
                    "event": "beam_active_start",
                    "source": "flying_spaghetti_monster_boss.eye_beam",
                },
                {
                    "t": 0.72,
                    "event": "beam_active_end",
                    "source": "flying_spaghetti_monster_boss.eye_beam",
                },
            ],
        },
        "damage.hit": {"animation": "hurt", "events": []},
        "lifecycle.death": {"animation": "death", "events": []},
    },
    "sockets": {
        "core": {
            "source": "flying_spaghetti_monster_boss.geometry",
            "point": {"x": 160.0, "y": 136.0},
        },
        "meatball_l": {
            "source": "flying_spaghetti_monster_boss.geometry",
            "point": {"x": 126.0, "y": 140.0},
        },
        "meatball_r": {
            "source": "flying_spaghetti_monster_boss.geometry",
            "point": {"x": 192.0, "y": 138.0},
        },
        "eye_l": {
            "source": "flying_spaghetti_monster_boss.geometry",
            "point": {"x": 116.0, "y": 40.0},
        },
        "eye_r": {
            "source": "flying_spaghetti_monster_boss.geometry",
            "point": {"x": 220.0, "y": 42.0},
        },
        "beam_l": {
            "source": "flying_spaghetti_monster_boss.geometry",
            "point": {"x": 116.0, "y": 40.0},
        },
        "beam_r": {
            "source": "flying_spaghetti_monster_boss.geometry",
            "point": {"x": 220.0, "y": 42.0},
        },
        "noodle_tip": {
            "source": "flying_spaghetti_monster_boss.geometry",
            "point": {"x": 252.0, "y": 150.0},
        },
        "projectile_origin": {
            "source": "flying_spaghetti_monster_boss.geometry",
            "point": {"x": 190.0, "y": 132.0},
        },
    },
    "tags": ["boss", "floating", "multipart"],
}

OUTLINE = (52, 42, 34, 255)
NOODLE = (234, 220, 182, 255)
NOODLE_SHADE = (202, 184, 146, 255)
NOODLE_HI = (248, 238, 212, 255)
MEATBALL = (122, 84, 60, 255)
MEATBALL_SHADE = (86, 56, 40, 255)
MEATBALL_HI = (160, 118, 90, 255)
SAUCE = (166, 86, 58, 225)
SAUCE_DARK = (112, 52, 36, 220)
STALK = (154, 112, 86, 255)
STALK_SHADE = (120, 82, 62, 255)
EYE = (244, 236, 240, 255)
PUPIL = (50, 40, 42, 255)
BEAM = (255, 248, 214, 188)
BEAM2 = (255, 232, 164, 120)
IMPACT = (255, 240, 188, 150)
DUST = (180, 164, 142, 90)
HURT_RED = (198, 94, 110, 170)


def _s(v: float) -> int:
    return int(round(v * SUPER))


def _pt(p: Point) -> Tuple[int, int]:
    return (_s(p[0]), _s(p[1]))


def _box(cx: float, cy: float, rx: float, ry: float) -> Tuple[int, int, int, int]:
    return (_s(cx - rx), _s(cy - ry), _s(cx + rx), _s(cy + ry))


def _lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def _rot(x: float, y: float, deg: float) -> Point:
    r = math.radians(deg)
    c = math.cos(r)
    s = math.sin(r)
    return (x * c - y * s, x * s + y * c)


def _poly(
    draw: ImageDraw.ImageDraw,
    pts: Sequence[Point],
    fill: RGBA,
    outline: RGBA | None = OUTLINE,
    width: float = 1.0,
) -> None:
    draw.polygon([_pt(p) for p in pts], fill=fill)
    if outline is not None and len(pts) >= 2:
        draw.line(
            [_pt(p) for p in list(pts) + [pts[0]]],
            fill=outline,
            width=max(1, _s(width)),
            joint="curve",
        )


def _line(
    draw: ImageDraw.ImageDraw, pts: Sequence[Point], fill: RGBA, width: float
) -> None:
    if len(pts) >= 2:
        draw.line(
            [_pt(p) for p in pts], fill=fill, width=max(1, _s(width)), joint="curve"
        )


def _ellipse(
    draw: ImageDraw.ImageDraw,
    cx: float,
    cy: float,
    rx: float,
    ry: float,
    fill: RGBA,
    outline: RGBA | None = OUTLINE,
    width: float = 0.8,
) -> None:
    draw.ellipse(
        _box(cx, cy, rx, ry),
        fill=fill,
        outline=outline,
        width=max(1, _s(width)) if outline is not None else 0,
    )


def _circle(
    draw: ImageDraw.ImageDraw,
    center: Point,
    r: float,
    fill: RGBA,
    outline: RGBA | None = OUTLINE,
    width: float = 0.8,
) -> None:
    _ellipse(draw, center[0], center[1], r, r, fill, outline, width)


def _downsample(img: Image.Image) -> Image.Image:
    return img.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


def _quad(a: Point, b: Point, c: Point, steps: int = 18) -> List[Point]:
    pts: List[Point] = []
    for i in range(steps + 1):
        t = i / steps
        u = 1.0 - t
        pts.append(
            (
                u * u * a[0] + 2 * u * t * b[0] + t * t * c[0],
                u * u * a[1] + 2 * u * t * b[1] + t * t * c[1],
            )
        )
    return pts


def _cubic(a: Point, b: Point, c: Point, d: Point, steps: int = 22) -> List[Point]:
    pts: List[Point] = []
    for i in range(steps + 1):
        t = i / steps
        u = 1.0 - t
        pts.append(
            (
                u * u * u * a[0]
                + 3 * u * u * t * b[0]
                + 3 * u * t * t * c[0]
                + t * t * t * d[0],
                u * u * u * a[1]
                + 3 * u * u * t * b[1]
                + 3 * u * t * t * c[1]
                + t * t * t * d[1],
            )
        )
    return pts


def _cubic_from_triplet(
    a: Point, b: Point, c: Point, curve: float = 0.72, steps: int = 22
) -> List[Point]:
    c1 = (_lerp(a[0], b[0], curve), _lerp(a[1], b[1], curve))
    c2 = (_lerp(c[0], b[0], curve), _lerp(c[1], b[1], curve))
    return _cubic(a, c1, c2, c, steps)


@dataclass
class Pose:
    root_x: float = 0.0
    root_y: float = 0.0
    bob: float = 0.0
    tilt: float = 0.0
    noodle_wave: float = 0.0
    spread: float = 0.0
    whip: float = 0.0
    volley: float = 0.0
    beam: float = 0.0
    hurt: float = 0.0
    collapse: float = 0.0
    eye_aim: float = 0.0
    left_meatball_shift: float = 0.0
    right_meatball_shift: float = 0.0
    slither: float = 0.0


def _ease(t: float) -> float:
    t = max(0.0, min(1.0, t))
    return 0.5 - 0.5 * math.cos(math.pi * t)


def _pose(anim: str, frame_idx: int, nframes: int) -> Pose:
    t = 0.0 if nframes <= 1 else frame_idx / float(max(1, nframes - 1))
    cyc = math.tau * t
    wave = math.sin(cyc)
    pulse = math.sin(math.pi * t)
    p = Pose()
    if anim == "idle":
        p.bob = wave * 4.0
        p.tilt = wave * 2.6
        p.noodle_wave = wave * 1.4
        p.spread = 0.12 + abs(wave) * 0.42
        p.eye_aim = wave * 3.0
        p.slither = math.sin(cyc * 1.8) * 1.35
    elif anim == "drift":
        p.root_x = wave * 12.0
        p.bob = math.sin(cyc * 1.2) * 5.0
        p.tilt = wave * 5.0
        p.noodle_wave = math.sin(cyc * 1.6) * 1.6
        p.spread = 0.25 + abs(wave) * 0.45
        p.eye_aim = 6.0 * math.sin(cyc * 0.7)
        p.slither = math.sin(cyc * 2.1) * 1.1
    elif anim == "noodle_whip":
        wind = 1.0 - _ease(min(1.0, t / 0.3))
        lash = _ease(max(0.0, min(1.0, (t - 0.18) / 0.42)))
        p.root_x = -12.0 * wind + 8.0 * lash
        p.tilt = -10.0 * wind + 14.0 * lash
        p.bob = -3.0 * wind + 2.0 * lash
        p.whip = lash
        p.noodle_wave = -0.8 + lash * 2.2
        p.spread = 0.35 + lash * 0.4
        p.eye_aim = 10.0 * lash
        p.slither = -0.5 + lash * 1.2
    elif anim == "meatball_volley":
        charge = _ease(min(1.0, t / 0.45))
        fire = _ease(max(0.0, min(1.0, (t - 0.38) / 0.5)))
        p.bob = math.sin(cyc) * 2.5
        p.tilt = -8.0 * charge + 4.0 * fire
        p.noodle_wave = 0.5 + charge
        p.volley = fire
        p.left_meatball_shift = -5.0 * charge
        p.right_meatball_shift = 10.0 * charge
        p.eye_aim = 12.0 * fire
        p.slither = 0.3 + fire * 0.8
    elif anim == "eye_beam":
        charge = _ease(min(1.0, t / 0.40))
        fire = _ease(max(0.0, min(1.0, (t - 0.35) / 0.45)))
        p.bob = 1.5 + pulse * 2.0
        p.tilt = -6.0 * charge
        p.noodle_wave = 0.4 + charge * 1.2
        p.beam = fire
        p.spread = 0.35 + charge * 0.3
        p.eye_aim = 18.0 * fire
        p.slither = 0.4 + fire * 0.8
    elif anim == "hurt":
        bump = math.sin(t * math.pi)
        p.root_x = 5.0 * (1 if frame_idx % 2 == 0 else -1)
        p.bob = bump * -4.0
        p.tilt = (1 if frame_idx % 2 == 0 else -1) * 7.0
        p.noodle_wave = -1.4 * bump
        p.hurt = bump
        p.eye_aim = 0.0
        p.slither = -1.0 * bump
    elif anim == "death":
        c = _ease(t)
        p.root_y = 70.0 * c
        p.bob = 20.0 * c
        p.tilt = 78.0 * c
        p.noodle_wave = -0.6 - 1.8 * c
        p.spread = 0.2 + 1.2 * c
        p.collapse = c
        p.eye_aim = -10.0 * c
        p.slither = -1.2 * c
    return p


def _draw_noodle(
    draw: ImageDraw.ImageDraw, pts: Sequence[Point], width: float, front: bool
) -> None:
    _line(draw, pts, OUTLINE, width + 1.9)
    _line(draw, pts, NOODLE, width)
    _line(draw, pts, NOODLE_SHADE, max(1.2, width * 0.34))
    if front:
        hi_pts = [(x - width * 0.10, y - width * 0.08) for x, y in pts]
        _line(draw, hi_pts, NOODLE_HI, max(0.8, width * 0.18))


def _draw_meatball(
    draw: ImageDraw.ImageDraw,
    center: Point,
    rx: float,
    ry: float,
    sauce_tilt: float,
    sauce_drip: float = 0.0,
) -> None:
    cx, cy = center
    _ellipse(draw, cx, cy, rx, ry, MEATBALL, OUTLINE, 1.0)
    _ellipse(
        draw, cx - rx * 0.18, cy - ry * 0.15, rx * 0.62, ry * 0.58, MEATBALL_HI, None, 0
    )
    _ellipse(
        draw,
        cx + rx * 0.20,
        cy + ry * 0.18,
        rx * 0.46,
        ry * 0.44,
        MEATBALL_SHADE,
        None,
        0,
    )
    # Chunky texture.
    for ox, oy, rr in [
        (-12, -6, 4),
        (10, -2, 3.5),
        (-6, 12, 3),
        (14, 11, 2.8),
        (4, -13, 2.8),
    ]:
        _circle(
            draw,
            (cx + ox * rx / 26.0, cy + oy * ry / 24.0),
            rr * (rx / 24.0),
            MEATBALL_SHADE,
            None,
            0,
        )
    # Sauce cap.
    sauce = []
    for i in range(11):
        ang = math.radians(-160 + i * 32 + sauce_tilt)
        rad = rx * (0.72 + 0.14 * math.sin(i * 0.8 + sauce_tilt))
        sauce.append(
            (cx + math.cos(ang) * rad, cy - ry * 0.42 + math.sin(ang) * ry * 0.22)
        )
    sauce += [
        (cx + rx * 0.52, cy - ry * 0.04),
        (cx + rx * 0.18, cy + ry * 0.12),
        (cx - rx * 0.28, cy + ry * 0.06),
        (cx - rx * 0.52, cy - ry * 0.02),
    ]
    _poly(draw, sauce, SAUCE, None, 0)
    if sauce_drip > 0.02:
        drip = [
            (cx + rx * 0.18, cy + ry * 0.08),
            (cx + rx * 0.36, cy + ry * (0.26 + sauce_drip * 0.35)),
            (cx + rx * 0.12, cy + ry * 0.18),
        ]
        _poly(draw, drip, SAUCE_DARK, None, 0)


def _draw_eye_stalk(
    draw: ImageDraw.ImageDraw,
    root: Point,
    bend: Point,
    tip: Point,
    eye_angle: float,
    angry: float,
    blink: bool = False,
    dead: bool = False,
) -> None:
    pts = _cubic_from_triplet(root, bend, tip, curve=0.76, steps=20)
    _line(draw, pts, OUTLINE, 6.0)
    _line(draw, pts, STALK, 4.8)
    _line(draw, pts, STALK_SHADE, 1.6)
    ex, ey = tip
    _ellipse(draw, ex, ey, 15.4, 14.8, EYE, OUTLINE, 1.0)
    if dead:
        _line(draw, [(ex - 5, ey - 5), (ex + 5, ey + 5)], OUTLINE, 1.2)
        _line(draw, [(ex - 5, ey + 5), (ex + 5, ey - 5)], OUTLINE, 1.2)
        return
    lid_y = ey - 3.0 - angry * 1.2
    _line(draw, [(ex - 6, lid_y + 1), (ex + 6, lid_y - 1)], OUTLINE, 1.0)
    if blink:
        _line(draw, [(ex - 5, ey + 1), (ex + 5, ey + 1)], OUTLINE, 1.1)
    else:
        px = ex + math.cos(math.radians(eye_angle)) * 3.8
        py = ey + math.sin(math.radians(eye_angle)) * 3.8
        _circle(draw, (px, py), 4.0, PUPIL, PUPIL, 0.4)
        _circle(draw, (px - 1.6, py - 1.4), 1.1, (255, 255, 255, 200), None, 0)


def _render_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    p = _pose(anim, frame_idx, nframes)
    img = Image.new(
        "RGBA", (_s(WORK_FRAME_SIZE[0]), _s(WORK_FRAME_SIZE[1])), (0, 0, 0, 0)
    )
    draw = ImageDraw.Draw(img, "RGBA")

    root = (160.0 + p.root_x, 152.0 + p.root_y + p.bob)

    def P(x: float, y: float) -> Point:
        rx, ry = _rot(x, y, p.tilt)
        return (root[0] + rx, root[1] + ry)

    # Back noodles: big silhouette sweep.
    back_specs = [
        (-74, -18, -126, 12, -158, 40, 10.5, False),
        (-48, -30, -92, -58, -110, -96, 9.8, False),
        (-12, -36, -18, -88, -6, -132, 8.8, False),
        (26, -34, 48, -86, 74, -126, 8.6, False),
        (62, -16, 112, -48, 150, -58, 9.8, False),
        (76, 18, 132, 30, 164, 58, 10.4, False),
        (48, 40, 74, 92, 88, 146, 9.8, False),
        (-8, 44, -6, 100, -18, 154, 9.6, False),
        (-58, 28, -104, 66, -146, 92, 10.4, False),
    ]
    for idx, (ax, ay, bx, by, cx, cy, width, front) in enumerate(back_specs):
        wave = (
            math.sin(frame_idx * 0.9 + idx * 0.8 + p.slither * 0.7)
            * 10.0
            * (0.35 + p.spread)
        )
        pts = _cubic_from_triplet(
            P(ax, ay),
            P(bx + wave * 0.35, by + p.noodle_wave * 7.0 + wave * 0.12),
            P(cx + wave, cy + p.noodle_wave * 10.0),
            curve=0.78,
            steps=24,
        )
        _draw_noodle(draw, pts, width, front)

    # Extra tangled center mass behind the meatballs.
    tangle_specs = [
        (-54, -12, -12, -34, 28, -10, 9.6, False),
        (-42, 8, 4, -16, 58, -2, 9.4, False),
        (-28, 26, 8, 0, 46, 22, 8.8, False),
        (-10, -24, 18, -2, 42, 28, 8.6, False),
        (8, -28, -8, 6, -36, 30, 8.6, False),
        (22, 12, -12, 30, -46, 18, 8.8, False),
    ]
    for idx, (ax, ay, bx, by, cx, cy, width, front) in enumerate(tangle_specs):
        wave = math.sin(frame_idx * 1.05 + idx * 1.15 + p.slither) * 8.0
        pts = _cubic_from_triplet(
            P(ax, ay),
            P(bx + wave * 0.55, by + p.noodle_wave * 6.5 - wave * 0.14),
            P(cx - wave * 0.35, cy + p.noodle_wave * 5.2),
            curve=0.82,
            steps=24,
        )
        _draw_noodle(draw, pts, width, front)

    # Meatballs.
    left_ball = P(-44 + p.left_meatball_shift, -4)
    right_ball = P(34 + p.right_meatball_shift, 2)
    _draw_meatball(
        draw,
        left_ball,
        28.0,
        30.0,
        sauce_tilt=-12 + frame_idx * 3.0,
        sauce_drip=p.volley,
    )
    _draw_meatball(
        draw,
        right_ball,
        31.0,
        33.0,
        sauce_tilt=14 - frame_idx * 2.0,
        sauce_drip=max(0.0, p.volley - 0.1),
    )

    # Mid noodles wrapping across meatballs.
    mid_specs = [
        (-72, -8, -26, -42, 12, -14, 10.8, True),
        (-88, 18, -18, 2, 62, -18, 10.0, True),
        (-68, 44, -10, 34, 52, 20, 9.8, True),
        (-26, -54, 12, -18, 54, 18, 9.2, True),
        (20, -50, 64, -20, 98, 24, 9.2, True),
        (60, 2, 18, 36, -20, 64, 9.8, True),
        (94, -6, 56, 18, 22, 52, 9.4, True),
        (82, 42, 26, 58, -26, 76, 9.6, True),
    ]
    for idx, (ax, ay, bx, by, cx, cy, width, front) in enumerate(mid_specs):
        wave = math.sin(frame_idx * 0.75 + idx * 0.9 + 0.8 + p.slither) * 9.0
        pts = _cubic_from_triplet(
            P(ax, ay),
            P(bx + wave * 0.40, by + p.noodle_wave * 7.5),
            P(cx + wave * (0.7 + p.spread * 0.25), cy + p.noodle_wave * 8.5),
            curve=0.80,
            steps=24,
        )
        _draw_noodle(draw, pts, width, front)

    # Distinct front lash noodle for the whip.
    if anim == "noodle_whip":
        whip_end_x = 116.0 + 116.0 * p.whip
        whip_end_y = -20.0 - 10.0 * p.whip
        whip_mid_y = -68.0 - 40.0 * p.whip
        pts = _cubic_from_triplet(
            P(42, -6),
            P(82 + 46 * p.whip, whip_mid_y),
            P(whip_end_x, whip_end_y),
            curve=0.84,
            steps=28,
        )
        _draw_noodle(draw, pts, 10.2, True)
        if p.whip > 0.2:
            cx, cy = pts[-1]
            for expand in [0, 12]:
                box = (
                    _s(cx - 16 - expand),
                    _s(cy - 12 - expand * 0.5),
                    _s(cx + 18 + expand),
                    _s(cy + 12 + expand * 0.5),
                )
                draw.arc(box, 200, 340, fill=IMPACT, width=_s(2.0))

    # Lower dangling noodles in front.
    front_specs = [
        (-34, 30, -58, 76, -48, 146, 10.2, True),
        (4, 28, 8, 78, 18, 154, 10.0, True),
        (40, 22, 58, 72, 78, 142, 10.2, True),
        (82, 18, 122, 54, 150, 106, 9.6, True),
        (-82, 6, -120, 42, -162, 74, 9.8, True),
    ]
    for idx, (ax, ay, bx, by, cx, cy, width, front) in enumerate(front_specs):
        drip = math.sin(frame_idx * 0.7 + idx * 0.9 + p.slither * 0.9) * 8.0
        collapse_y = p.collapse * (20.0 + idx * 6)
        pts = _cubic_from_triplet(
            P(ax, ay),
            P(bx + drip * 0.35, by + p.noodle_wave * 8.0 + collapse_y * 0.2),
            P(cx + drip * 0.8, cy + collapse_y),
            curve=0.78,
            steps=24,
        )
        _draw_noodle(draw, pts, width, front)

    # Eye stalks: two larger matching eyes.
    eye_ang = p.eye_aim
    eye_dead = anim == "death" and p.collapse > 0.55
    blink = anim == "hurt"
    stalks = [
        (
            P(-28, -28),
            P(-44, -72 - p.beam * 8),
            P(-36, -106 - p.beam * 14),
            eye_ang - 2,
        ),
        (P(34, -26), P(52, -70 - p.beam * 8), P(60, -104 - p.beam * 14), eye_ang + 2),
    ]
    for root_pt, bend_pt, tip_pt, ang in stalks:
        if anim == "death":
            bend_pt = (bend_pt[0] - p.collapse * 22, bend_pt[1] + p.collapse * 22)
            tip_pt = (tip_pt[0] - p.collapse * 40, tip_pt[1] + p.collapse * 58)
        _draw_eye_stalk(
            draw,
            root_pt,
            bend_pt,
            tip_pt,
            ang,
            angry=0.8
            if anim in {"noodle_whip", "eye_beam", "meatball_volley"}
            else 0.35,
            blink=blink,
            dead=eye_dead,
        )

    # Attack extras.
    if anim == "meatball_volley" and p.volley > 0.02:
        # Telegraph only: no projectile sprite baked into the boss animation.
        arc = [
            P(54, -2),
            P(88 + 54 * p.volley, -20 - 12 * p.volley),
            P(126 + 74 * p.volley, -10 - 8 * p.volley),
        ]
        _line(draw, arc, SAUCE, 3.0)
        _line(draw, arc, SAUCE_DARK, 1.0)
        for cx, cy in arc[1:]:
            _ellipse(draw, cx, cy, 4.0 + p.volley * 2.0, 2.4 + p.volley, SAUCE, None, 0)
    if anim == "eye_beam" and p.beam > 0.05:
        for origin in [P(-36, -106 - p.beam * 14), P(60, -104 - p.beam * 14)]:
            tip = (origin[0] + 110 + 72 * p.beam, origin[1] - 8 + 2 * p.beam)
            beam_poly = [
                (origin[0] + 2, origin[1] - 5),
                (origin[0] + 6, origin[1] + 5),
                (tip[0], tip[1] + 16),
                (tip[0] + 12, tip[1]),
                (tip[0], tip[1] - 16),
            ]
            _poly(draw, beam_poly, BEAM2, None, 0)
            core = [
                (origin[0] + 1, origin[1] - 2),
                (origin[0] + 3, origin[1] + 2),
                (tip[0] - 8, tip[1] + 6),
                (tip[0], tip[1]),
                (tip[0] - 8, tip[1] - 6),
            ]
            _poly(draw, core, BEAM, None, 0)
    if anim == "hurt" and p.hurt > 0.1:
        overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
        overlay_draw = ImageDraw.Draw(overlay, "RGBA")

        _ellipse(
            overlay_draw,
            root[0] - 4,
            root[1] - 10,
            82,
            58,
            HURT_RED,
            None,
            0,
        )

        img = Image.alpha_composite(img, overlay)
        draw = ImageDraw.Draw(img, "RGBA")

    # Death smear / floor pile.
    if anim == "death" and p.collapse > 0.15:
        smear_center = P(-8, 118)
        _ellipse(
            draw,
            smear_center[0],
            smear_center[1],
            76 + 28 * p.collapse,
            14 + 12 * p.collapse,
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
        crop_margin=18,
        auto_crop=True,
        actor_metadata=ACTOR_METADATA,
        animation_key_map=ANIMATION_KEY_MAP,
    )
    return [
        outputs[k]
        for k in [
            "spritesheet",
            "yaml",
            "ron",
            "actor",
            "preview",
            "canonical",
            "canonical_transparent",
        ]
    ]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Render the standalone Flying Spaghetti Monster boss sprite sheet."
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
