"""Bone-toolkit sprite target for a Stochastic Parrot enemy.

A noisy mimic-bird built on ``ambition_sprite2d_renderer.authoring.skeleton``.
The goal is a first-pass enemy sheet that demonstrates the new bone tools on
something smaller and more expressive than the player robot.

Visual pitch:
- bright, overconfident parrot silhouette
- exaggerated beak and eye
- layered wings / tail attached to a simple FK rig
- hop/peck/squawk personality with a pronounced macaw beak

Rows: idle / walk / fly / slash / taunt / hurt / death.

    PYTHONPATH=tools/ambition_sprite2d_renderer python -m ambition_sprite2d_renderer sheet stochastic_parrot
    PYTHONPATH=tools/ambition_sprite2d_renderer python -m ambition_sprite2d_renderer publish stochastic_parrot
"""

from __future__ import annotations

import math
from pathlib import Path
from typing import Dict, List, Tuple

from PIL import Image, ImageColor, ImageDraw, ImageOps

from ...authoring.common_draw import draw_capsule
from ...authoring.rig import add, clamp, ease_in_out_sine, vec
from ...authoring.skeleton import (
    Channel,
    Clip,
    PartCtx,
    Rig,
    Skeleton,
    composite_polygon,
    draw_polygon,
    rounded_polygon,
    two_bone_ik,
)
from ...authoring.tackon_sheet import build_sheet, write_canonical

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_NAME = "stochastic_parrot"
FRAME_W, FRAME_H = 128, 128
SS = 4
GROUND_Y = 103.0
CENTER_X = 64.0
ANKLE_H = 1.3
LEG_U, LEG_L = 12.0, 9.0
WING_U, WING_L = 14.0, 11.0
OUTLINE_W = 1.2

ROWS: List[Tuple[str, int, int]] = [
    ("idle", 8, 120),
    ("walk", 8, 95),
    ("fly", 10, 82),
    ("turnaround", 9, 82),
    ("turnaround_flight", 9, 74),
    ("dive_bomb", 9, 72),
    ("hover_peck", 10, 68),
    ("banked_strafe", 10, 74),
    ("slash", 8, 76),
    ("taunt", 10, 90),
    ("hurt", 4, 92),
    ("death", 8, 108),
]
LOOPS = {"idle", "walk", "fly", "taunt"}

ACTOR_METADATA = {
    "actor": {"character_id": TARGET_NAME, "display_name": "Stochastic Parrot"},
    "body": {
        "body_plan": "AvianBiped",
        "body_kind": "Standard",
        "traits": ["enemy", "bird", "mimic", "noisy", "chaotic"],
    },
    "visual": {"default_pose": "idle"},
    "tags": ["enemy", "bird", "stochastic_parrot"],
}


def _rgba(hex_color: str, alpha: int = 255) -> Color:
    r, g, b = ImageColor.getrgb(hex_color)
    return (r, g, b, alpha)


PAL: Dict[str, Color] = {
    "outline": _rgba("#221B21"),
    "body": _rgba("#D93B31"),
    "body_dark": _rgba("#A41F1E"),
    "body_light": _rgba("#F06148"),
    "wing_yellow": _rgba("#F6C73A"),
    "wing_gold": _rgba("#DCA11A"),
    "wing_blue": _rgba("#2E66D9"),
    "wing_blue_dark": _rgba("#1748AF"),
    "head": _rgba("#D93B31"),
    "head_light": _rgba("#F06148"),
    "face_patch": _rgba("#F6F0E3", 210),
    "face_line": _rgba("#6E7078", 165),
    "beak_upper": _rgba("#F1E7D5"),
    "beak_upper_shadow": _rgba("#D7C9B2"),
    "beak_lower": _rgba("#373336"),
    "leg": _rgba("#8A8C95"),
    "talon": _rgba("#E0C590"),
    "eye": _rgba("#F8EC98"),
    "pupil": _rgba("#151417"),
    "shadow": _rgba("#1D1A20", 90),
}


# ---- Skeleton -----------------------------------------------------------------


def _build_skeleton() -> Skeleton:
    sk = Skeleton()
    sk.bone("body", offset=(0.0, -23.0))
    sk.bone("head", parent="body", offset=(13.0, -10.5))
    sk.bone("beak", parent="head", offset=(11.5, 0.5), length=10.0)
    sk.bone("tail", parent="body", offset=(-16.0, 2.0), length=14.0, rest_angle=160.0)
    sk.bone("far_wing_u", parent="body", offset=(-2.0, -3.0), length=WING_U, rest_angle=162.0)
    sk.bone("far_wing_l", parent="far_wing_u", offset=(WING_U, 0.0), length=WING_L, rest_angle=6.0)
    sk.bone("near_wing_u", parent="body", offset=(3.0, -3.5), length=WING_U, rest_angle=150.0)
    sk.bone("near_wing_l", parent="near_wing_u", offset=(WING_U, 0.0), length=WING_L, rest_angle=8.0)
    sk.bone("far_leg_u", parent="body", offset=(-4.5, 8.0), length=LEG_U, rest_angle=90.0)
    sk.bone("far_leg_l", parent="far_leg_u", offset=(LEG_U, 0.0), length=LEG_L)
    sk.bone("far_foot", parent="far_leg_l", offset=(LEG_L, 0.0), length=7.0, rest_angle=-90.0)
    sk.bone("near_leg_u", parent="body", offset=(2.0, 8.5), length=LEG_U, rest_angle=90.0)
    sk.bone("near_leg_l", parent="near_leg_u", offset=(LEG_U, 0.0), length=LEG_L)
    sk.bone("near_foot", parent="near_leg_l", offset=(LEG_L, 0.0), length=7.0, rest_angle=-90.0)
    return sk


_SKEL = _build_skeleton()


# ---- Parts --------------------------------------------------------------------


def _leg_painter(upper: str, lower: str, tint: Color, toe_tint: Color, r_u: float, r_l: float):
    def fn(ctx: PartCtx) -> None:
        u, low = ctx.world[upper], ctx.world[lower]
        ow = ctx.L(0.45)
        draw_capsule(ctx.draw, ctx.cw(u.origin), ctx.cw(u.tip), ctx.L(r_u), tint, PAL["outline"], ow)
        draw_capsule(ctx.draw, ctx.cw(low.origin), ctx.cw(low.tip), ctx.L(r_l), tint, PAL["outline"], ow)
        jx, jy = ctx.cw(low.origin)
        jr = ctx.L(r_u * 0.55)
        ctx.draw.ellipse((jx - jr, jy - jr, jx + jr, jy + jr), fill=PAL["talon"])
        hx, hy = ctx.cw(low.tip)
        for spread, length in ((-28.0, 4.5), (0.0, 5.4), (26.0, 4.2)):
            tx, ty = ctx.cw(add(low.tip, vec(length, low.angle + spread)))
            ctx.draw.line((hx, hy, tx, ty), fill=toe_tint, width=max(1, int(ctx.L(0.8))))
    return fn



def _tail_painter(ctx: PartCtx) -> None:
    angle_sway = clamp(ctx.params.get("tail_fan", 0.0), -1.0, 1.0)
    base_shapes = [
        [(-1.0, -3.4), (12.0, -7.4), (14.5, -2.4), (2.2, 1.4)],
        [(-0.4, -0.8), (14.5, -1.0), (15.2, 4.0), (1.2, 2.8)],
        [(-0.2, 2.0), (12.8, 5.3), (13.6, 9.2), (0.8, 4.9)],
    ]
    shades = [PAL["body_dark"], PAL["body"], PAL["wing_blue"]]
    for idx, pts in enumerate(base_shapes):
        offset = (idx - 1) * 6.2 * angle_sway
        poly = rounded_polygon([ctx.pt((x, y + offset)) for x, y in pts], radius=ctx.L(1.7))
        draw_polygon(ctx.draw, poly, shades[idx], PAL["outline"], ctx.L(0.65))



def _body_painter(ctx: PartCtx) -> None:
    pts = [(-18.5, -12.5), (4.0, -18.5), (18.5, -11.0), (20.5, -1.0), (14.5, 11.0), (-1.5, 16.0), (-15.5, 12.0), (-21.0, 2.5)]
    poly = rounded_polygon(ctx.pts(pts), radius=ctx.L(4.2))
    draw_polygon(ctx.draw, poly, PAL["body"], PAL["outline"], ctx.L(OUTLINE_W))
    belly = [(-6.5, -6.0), (10.0, -6.5), (12.5, 5.0), (5.5, 12.0), (-4.5, 9.5), (-7.0, 1.8)]
    composite_polygon(ctx.img, rounded_polygon(ctx.pts(belly), radius=ctx.L(3.0)), (*PAL["body_light"][:3], 82))
    back = [(-17.5, -10.0), (-6.0, -12.5), (-6.5, 11.0), (-15.0, 8.0)]
    composite_polygon(ctx.img, rounded_polygon(ctx.pts(back), radius=ctx.L(2.5)), (*PAL["body_dark"][:3], 108))
    shoulder = [(-1.2, -12.4), (8.7, -14.5), (12.8, -9.6), (4.4, -5.9), (-1.4, -7.2)]
    composite_polygon(ctx.img, rounded_polygon(ctx.pts(shoulder), radius=ctx.L(2.1)), (*PAL["wing_yellow"][:3], 150))



def _wing_painter(
    upper: str,
    lower: str,
    covert_tint: Color,
    primary_tint: Color,
    r_u: float,
    r_l: float,
    shoulder_tint: Color,
    tip_tint: Color,
):
    def fn(ctx: PartCtx) -> None:
        u, low = ctx.world[upper], ctx.world[lower]
        is_far = upper.startswith("far_")
        scale = 0.9 if is_far else 1.0
        ow = ctx.L(0.42)

        def rot_pt(center: Point, angle_deg: float, along: float, across: float) -> Point:
            rad = math.radians(angle_deg)
            ca, sa = math.cos(rad), math.sin(rad)
            return (center[0] + ca * along - sa * across, center[1] + sa * along + ca * across)

        def feather(center: Point, ang: float, length: float, base_w: float, tip_w: float, fill: Color, shaft: Color, highlight_alpha: int = 56) -> None:
            pts = [
                rot_pt(center, ang, -0.72 * scale, -base_w),
                rot_pt(center, ang, length * 0.12, -base_w * 0.96),
                rot_pt(center, ang, length * 0.42, -base_w * 0.72),
                rot_pt(center, ang, length * 0.74, -tip_w * 1.08),
                rot_pt(center, ang, length + 0.8 * scale, 0.0),
                rot_pt(center, ang, length * 0.74, tip_w * 1.08),
                rot_pt(center, ang, length * 0.42, base_w * 0.72),
                rot_pt(center, ang, length * 0.12, base_w * 0.96),
                rot_pt(center, ang, -0.55 * scale, base_w),
            ]
            poly = rounded_polygon([ctx.cw(p) for p in pts], radius=ctx.L(1.0 * scale))
            draw_polygon(ctx.draw, poly, fill, PAL["outline"], ctx.L(0.22))
            hi = [
                rot_pt(center, ang, 0.25 * scale, -base_w * 0.36),
                rot_pt(center, ang, length * 0.52, -base_w * 0.24),
                rot_pt(center, ang, length * 0.76, -tip_w * 0.18),
                rot_pt(center, ang, length * 0.58, 0.05),
                rot_pt(center, ang, 0.45 * scale, 0.04),
            ]
            composite_polygon(ctx.img, rounded_polygon([ctx.cw(p) for p in hi], radius=ctx.L(0.55 * scale)), (*PAL["body_light"][:3], highlight_alpha if not is_far else int(highlight_alpha * 0.7)))
            shaft_pts = [
                ctx.cw(rot_pt(center, ang, 0.22 * scale, -0.08 * scale)),
                ctx.cw(rot_pt(center, ang, length * 0.82, 0.0)),
            ]
            ctx.draw.line((shaft_pts[0][0], shaft_pts[0][1], shaft_pts[1][0], shaft_pts[1][1]), fill=shaft, width=max(1, int(ctx.L(0.18))))

        shoulder = u.origin
        elbow = u.tip

        draw_capsule(ctx.draw, ctx.cw(shoulder), ctx.cw(elbow), ctx.L(r_u * (0.74 if is_far else 0.8)), shoulder_tint, PAL["outline"], ow)

        shoulder_panel = [
            rot_pt(shoulder, u.angle, 0.1, -r_u * 0.82),
            rot_pt(shoulder, u.angle, 4.8 * scale, -r_u * 1.08),
            rot_pt(elbow, low.angle, 0.7 * scale, -r_l * 0.94),
            rot_pt(elbow, low.angle, 3.2 * scale, -r_l * 0.32),
            rot_pt(elbow, low.angle, 2.7 * scale, r_l * 0.4),
            rot_pt(elbow, low.angle, 0.1 * scale, r_l * 0.95),
            rot_pt(shoulder, u.angle, 0.8, r_u * 0.76),
        ]
        draw_polygon(ctx.draw, rounded_polygon([ctx.cw(p) for p in shoulder_panel], radius=ctx.L(1.25)), shoulder_tint, PAL["outline"], ctx.L(0.24))
        arm_hi = [
            rot_pt(shoulder, u.angle, 1.0, -r_u * 0.2),
            rot_pt(shoulder, u.angle, 3.9 * scale, -r_u * 0.52),
            rot_pt(elbow, low.angle, 0.3 * scale, -r_l * 0.28),
            rot_pt(elbow, low.angle, 0.8 * scale, 0.08),
            rot_pt(shoulder, u.angle, 1.1, r_u * 0.1),
        ]
        composite_polygon(ctx.img, rounded_polygon([ctx.cw(p) for p in arm_hi], radius=ctx.L(0.82)), (*PAL["body_light"][:3], 138 if not is_far else 96))

        covert_panel = [
            rot_pt(elbow, low.angle, -0.25 * scale, -r_l * 1.02),
            rot_pt(elbow, low.angle, 3.0 * scale, -r_l * 1.05),
            rot_pt(elbow, low.angle, 7.0 * scale, -r_l * 0.92),
            rot_pt(elbow, low.angle, 10.4 * scale, -r_l * 0.46),
            rot_pt(elbow, low.angle, 11.1 * scale, 0.12),
            rot_pt(elbow, low.angle, 8.7 * scale, r_l * 0.72),
            rot_pt(elbow, low.angle, 3.8 * scale, r_l * 1.0),
            rot_pt(elbow, low.angle, 0.25 * scale, r_l * 0.78),
        ]
        draw_polygon(ctx.draw, rounded_polygon([ctx.cw(p) for p in covert_panel], radius=ctx.L(1.28)), covert_tint, PAL["outline"], ctx.L(0.23))
        covert_hi = [
            rot_pt(elbow, low.angle, 1.0 * scale, -r_l * 0.42),
            rot_pt(elbow, low.angle, 4.9 * scale, -r_l * 0.54),
            rot_pt(elbow, low.angle, 7.4 * scale, -r_l * 0.22),
            rot_pt(elbow, low.angle, 6.2 * scale, 0.28),
            rot_pt(elbow, low.angle, 2.3 * scale, 0.18),
        ]
        composite_polygon(ctx.img, rounded_polygon([ctx.cw(p) for p in covert_hi], radius=ctx.L(0.9)), (*PAL["wing_yellow"][:3], 172 if not is_far else 122))

        feather_bed = [
            rot_pt(elbow, low.angle, 2.0 * scale, -r_l * 0.32),
            rot_pt(elbow, low.angle, 6.5 * scale, -r_l * 0.24),
            rot_pt(elbow, low.angle, 10.8 * scale, -r_l * 0.02),
            rot_pt(elbow, low.angle, 11.0 * scale, r_l * 0.34),
            rot_pt(elbow, low.angle, 8.4 * scale, r_l * 0.54),
            rot_pt(elbow, low.angle, 3.6 * scale, r_l * 0.48),
        ]
        composite_polygon(ctx.img, rounded_polygon([ctx.cw(p) for p in feather_bed], radius=ctx.L(0.95)), (*covert_tint[:3], 214 if not is_far else 160))

        alula = [
            rot_pt(elbow, low.angle - 10.0, -0.5 * scale, -0.72),
            rot_pt(elbow, low.angle - 16.0, 2.4 * scale, -1.08),
            rot_pt(elbow, low.angle - 6.0, 3.4 * scale, -0.22),
            rot_pt(elbow, low.angle + 2.0, 1.0 * scale, 0.62),
        ]
        draw_polygon(ctx.draw, rounded_polygon([ctx.cw(p) for p in alula], radius=ctx.L(0.76)), shoulder_tint, PAL["outline"], ctx.L(0.16))

        secondary_specs = [
            (add(elbow, vec(1.4 * scale, low.angle - 4.2)), low.angle + 34.0, 6.7 * scale, 1.38 * scale, 0.72 * scale, covert_tint),
            (add(elbow, vec(2.8 * scale, low.angle - 2.2)), low.angle + 24.0, 7.5 * scale, 1.46 * scale, 0.76 * scale, covert_tint),
            (add(elbow, vec(4.2 * scale, low.angle - 0.2)), low.angle + 15.0, 8.2 * scale, 1.5 * scale, 0.78 * scale, covert_tint),
            (add(elbow, vec(5.5 * scale, low.angle + 1.6)), low.angle + 6.0, 8.8 * scale, 1.48 * scale, 0.76 * scale, covert_tint),
            (add(elbow, vec(6.7 * scale, low.angle + 3.3)), low.angle - 3.0, 9.0 * scale, 1.42 * scale, 0.72 * scale, covert_tint),
        ]
        primary_specs = [
            (add(elbow, vec(7.3 * scale, low.angle - 5.2)), low.angle + 18.0, 9.4 * scale, 1.34 * scale, 0.64 * scale, primary_tint),
            (add(elbow, vec(8.2 * scale, low.angle - 1.8)), low.angle + 6.0, 10.6 * scale, 1.38 * scale, 0.62 * scale, primary_tint),
            (add(elbow, vec(8.8 * scale, low.angle + 1.6)), low.angle - 6.0, 11.3 * scale, 1.34 * scale, 0.6 * scale, primary_tint),
            (add(elbow, vec(8.8 * scale, low.angle + 5.1)), low.angle - 17.0, 11.2 * scale, 1.26 * scale, 0.56 * scale, tip_tint),
            (add(elbow, vec(8.3 * scale, low.angle + 8.0)), low.angle - 29.0, 10.4 * scale, 1.12 * scale, 0.5 * scale, tip_tint),
        ]
        shaft_color = (*PAL["outline"][:3], 124)
        for spec in reversed(primary_specs):
            feather(*spec, shaft_color, 42)
        for spec in reversed(secondary_specs):
            feather(*spec, shaft_color, 54)

        root_coverts = [
            rot_pt(elbow, low.angle, 0.8 * scale, -r_l * 0.34),
            rot_pt(elbow, low.angle, 3.8 * scale, -r_l * 0.48),
            rot_pt(elbow, low.angle, 6.2 * scale, -r_l * 0.22),
            rot_pt(elbow, low.angle, 6.5 * scale, 0.18),
            rot_pt(elbow, low.angle, 4.2 * scale, 0.38),
            rot_pt(elbow, low.angle, 1.2 * scale, 0.28),
        ]
        composite_polygon(ctx.img, rounded_polygon([ctx.cw(p) for p in root_coverts], radius=ctx.L(0.82)), (*covert_tint[:3], 188 if not is_far else 132))

    return fn


def _head_painter(ctx: PartCtx) -> None:
    p = ctx.params
    pts = [(-11.5, -10.5), (2.0, -11.8), (11.0, -6.0), (12.5, 3.5), (6.5, 10.5), (-4.5, 9.8), (-12.5, 2.8), (-13.0, -5.5)]
    poly = rounded_polygon(ctx.pts(pts), radius=ctx.L(4.0))
    draw_polygon(ctx.draw, poly, PAL["head"], PAL["outline"], ctx.L(OUTLINE_W))
    hi = [(-4.0, -8.8), (5.8, -9.2), (9.0, -3.2), (5.8, 1.5), (-2.8, -0.2)]
    composite_polygon(ctx.img, rounded_polygon(ctx.pts(hi), radius=ctx.L(2.6)), (*PAL["head_light"][:3], 112))
    face_patch = [(0.3, -6.0), (8.6, -5.2), (10.0, 2.2), (8.3, 8.2), (3.2, 9.2), (-0.2, 4.8), (-0.1, -0.5)]
    composite_polygon(ctx.img, rounded_polygon(ctx.pts(face_patch), radius=ctx.L(2.5)), PAL["face_patch"])
    line_x = [2.8, 4.5, 6.2, 7.8]
    for idx, x in enumerate(line_x):
        p0 = ctx.pt((x, -2.7 + 0.35 * idx))
        p1 = ctx.pt((x - 0.55, 5.1 + 0.25 * idx))
        ctx.draw.line((p0[0], p0[1], p1[0], p1[1]), fill=PAL["face_line"], width=max(1, int(ctx.L(0.28))))

    blink = p.get("blink", 0.0) > 0.5
    squint = clamp(p.get("eye_squint", 0.0), 0.0, 1.0)
    eye_h = 6.1 * (1.0 - 0.55 * squint)
    if blink:
        eye_h = 1.0
    ec = ctx.pt((5.4, -2.2))
    ew, eh = ctx.L(4.8), ctx.L(eye_h)
    ctx.draw.ellipse((ec[0] - ew / 2, ec[1] - eh / 2, ec[0] + ew / 2, ec[1] + eh / 2), fill=PAL["eye"], outline=PAL["outline"], width=max(1, int(ctx.L(0.35))))
    if not blink:
        pw, ph = ctx.L(1.55), ctx.L(max(1.75, eye_h * 0.48))
        pupil_x = ec[0] + ctx.L(0.6 + 0.72 * clamp(p.get("look_x", 0.0), -1.0, 1.0))
        pupil_y = ec[1] + ctx.L(-0.1 + 0.45 * clamp(p.get("look_y", 0.0), -1.0, 1.0))
        ctx.draw.ellipse((pupil_x - pw / 2, pupil_y - ph / 2, pupil_x + pw / 2, pupil_y + ph / 2), fill=PAL["pupil"])
    brow = [(-1.5, -7.8), (7.2, -9.5), (8.2, -7.6), (0.8, -6.3)]
    composite_polygon(ctx.img, rounded_polygon(ctx.pts(brow), radius=ctx.L(1.1)), (*PAL["body_dark"][:3], 120))



def _beak_painter(ctx: PartCtx) -> None:
    p = ctx.params
    open_amt = clamp(p.get("beak_open", 0.0), 0.0, 1.0)

    # User-space reference points from the macaw jaw sketch.  We interpret the
    # original coordinates as a small design grid with +y upward, then map them
    # into the renderer's local space where +y points downward.
    sx, sy = 4.15, 4.0
    ox, oy = -1.8, 8.9

    def map_u(pt: Point) -> Point:
        x, y = pt
        return (ox + x * sx, oy - y * sy)

    def rot_about(pt: Point, pivot: Point, deg: float) -> Point:
        # Local space has +y downward, so positive degrees rotate clockwise.
        a = math.radians(deg)
        c, s = math.cos(a), math.sin(a)
        dx, dy = pt[0] - pivot[0], pt[1] - pivot[1]
        return (pivot[0] + dx * c - dy * s, pivot[1] + dx * s + dy * c)

    upper_src = [
        (0.0, 3.0),
        (1.0, 4.2),
        (2.3, 4.4),
        (3.2, 3.5),
        (3.4, 2.0),
        (3.1, 0.2),
        (2.4, 0.8),
        (1.0, 2.0),
        (0.0, 1.2),
    ]
    lower_src = [
        (0.1, 0.4),
        (0.1, 1.1),
        (1.7, 1.1),
        (0.9, 0.3),
    ]
    pup = map_u((0.0, 3.0))
    plow = map_u((0.1, 0.4))
    upper_pts = [map_u(p) for p in upper_src]
    lower_pts = [map_u(p) for p in lower_src]

    upper_deg = -14.0 * open_amt
    lower_deg = 20.0 * open_amt
    upper_pts = [rot_about(pt, pup, upper_deg) for pt in upper_pts]
    lower_pts = [rot_about(pt, plow, lower_deg) for pt in lower_pts]

    # Small vertical backplate / cere so the dual pivots feel mechanically
    # distinct rather than like a single duck bill.
    backplate = [
        (pup[0] - 2.2, pup[1] - 2.4),
        (pup[0] - 0.3, pup[1] - 2.1),
        (plow[0] + 0.2, plow[1] + 1.2),
        (plow[0] - 2.4, plow[1] + 1.3),
    ]
    cere = [
        (pup[0] - 0.4, pup[1] - 0.9),
        (pup[0] + 2.4, pup[1] - 1.4),
        (pup[0] + 3.3, pup[1] + 0.2),
        (pup[0] + 0.9, pup[1] + 0.8),
    ]
    upper_shadow = [
        (1.0, 3.45),
        (2.15, 3.65),
        (2.95, 3.05),
        (3.05, 2.15),
        (2.7, 1.0),
        (2.2, 1.25),
        (1.0, 2.15),
    ]
    upper_shadow = [rot_about(map_u(p), pup, upper_deg) for p in upper_shadow]
    lower_highlight = [
        (0.25, 0.62),
        (0.45, 0.96),
        (1.35, 0.95),
        (0.92, 0.55),
    ]
    lower_highlight = [rot_about(map_u(p), plow, lower_deg) for p in lower_highlight]

    draw_polygon(ctx.draw, rounded_polygon(ctx.pts(backplate), radius=ctx.L(1.3)), PAL["face_line"], PAL["outline"], ctx.L(0.55))
    composite_polygon(ctx.img, rounded_polygon(ctx.pts(cere), radius=ctx.L(1.25)), (*PAL["face_patch"][:3], 205))

    # Draw lower first so the upper hook can overbite and hide it at rest.
    draw_polygon(ctx.draw, rounded_polygon(ctx.pts(lower_pts), radius=ctx.L(1.15)), PAL["beak_lower"], PAL["outline"], ctx.L(0.72))
    composite_polygon(ctx.img, rounded_polygon(ctx.pts(lower_highlight), radius=ctx.L(0.65)), (88, 86, 92, 180))

    draw_polygon(ctx.draw, rounded_polygon(ctx.pts(upper_pts), radius=ctx.L(1.55)), PAL["beak_upper"], PAL["outline"], ctx.L(0.8))
    composite_polygon(ctx.img, rounded_polygon(ctx.pts(upper_shadow), radius=ctx.L(1.0)), (*PAL["beak_upper_shadow"][:3], 190))

    # Nostril and hinge pins.
    nostril = rot_about(map_u((1.15, 3.1)), pup, upper_deg)
    nx, ny = ctx.pt(nostril)
    nrx, nry = ctx.L(0.42), ctx.L(0.6)
    ctx.draw.ellipse((nx - nrx, ny - nry, nx + nrx, ny + nry), fill=PAL["outline"])
    for pin, fill in ((pup, PAL["beak_upper_shadow"]), (plow, PAL["outline"])):
        px, py = ctx.pt(pin)
        rr = ctx.L(0.42)
        ctx.draw.ellipse((px - rr, py - rr, px + rr, py + rr), fill=fill, outline=PAL["outline"])


def _build_rig() -> Rig:
    rig = Rig(_SKEL)
    rig.part("far_tail", "tail", 8, _tail_painter)
    rig.part("far_leg", "far_leg_u", 12, _leg_painter("far_leg_u", "far_leg_l", PAL["leg"], PAL["talon"], 1.15, 0.95))
    rig.part("far_wing", "far_wing_u", 18, _wing_painter("far_wing_u", "far_wing_l", PAL["wing_gold"], PAL["wing_blue"], 3.2, 2.7, PAL["body_dark"], PAL["wing_blue_dark"]))
    rig.part("body", "body", 30, _body_painter)
    rig.part("near_leg", "near_leg_u", 42, _leg_painter("near_leg_u", "near_leg_l", PAL["leg"], PAL["talon"], 1.2, 1.0))
    rig.part("near_wing", "near_wing_u", 48, _wing_painter("near_wing_u", "near_wing_l", PAL["wing_yellow"], PAL["wing_blue"], 3.5, 2.9, PAL["body"], PAL["wing_blue_dark"]))
    rig.part("head", "head", 60, _head_painter)
    rig.part("beak", "beak", 64, _beak_painter)
    return rig


_RIG = _build_rig()


# ---- Clips --------------------------------------------------------------------


def _step_wave(t: float, phase: float = 0.0) -> float:
    return math.sin((t + phase) * math.tau)


DEFAULT_FOOT_X = {"near": 4.5, "far": -2.5}
DEFAULT_AIR_FOOT = {"near": (5.0, -8.5), "far": (-0.5, -9.5)}


CLIP_IDLE = Clip(
    loop=True,
    channels={
        "root_y": lambda t: -0.8 + 1.6 * ease_in_out_sine(0.5 - 0.5 * math.cos(t * math.tau)),
        "body": lambda t: 2.8 * math.sin(t * math.tau),
        "head": lambda t: -4.6 * math.sin(t * math.tau + 0.3),
        "tail": lambda t: 10.0 * math.sin(t * math.tau + 0.75),
        "tail_fan": lambda t: 0.35 * math.sin(t * math.tau + 1.1),
        "near_wing_u": lambda t: -8.0 + 5.0 * math.sin(t * math.tau + 0.4),
        "near_wing_l": lambda t: -4.0 + 3.0 * math.sin(t * math.tau + 0.7),
        "far_wing_u": lambda t: -5.0 + 3.8 * math.sin(t * math.tau + 1.2),
        "far_wing_l": lambda t: -2.0 + 2.6 * math.sin(t * math.tau + 1.4),
        "blink": Channel((0.00, 0.0), (0.11, 0.0), (0.13, 1.0), (0.15, 0.0), (0.67, 0.0), (0.695, 1.0), (0.72, 0.0)),
        "eye_squint": lambda t: 0.18 + 0.08 * math.sin(t * math.tau + 0.8),
        "look_x": lambda t: 0.15 + 0.25 * math.sin(t * math.tau + 0.2),
        "look_y": lambda t: -0.05,
        "beak_open": lambda t: 0.04 + 0.02 * math.sin(t * math.tau + 1.2),
    },
)



def _walk_root_y(t: float) -> float:
    return -1.0 + 3.5 * abs(math.sin(t * math.tau))



def _walk_foot_x(side: str, t: float) -> float:
    phase = t if side == "near" else (t + 0.5) % 1.0
    # Stance at ends, faster swing through the middle.
    swing = math.sin(phase * math.tau)
    return DEFAULT_FOOT_X[side] + 5.5 * swing



def _walk_foot_lift(side: str, t: float) -> float:
    phase = t if side == "near" else (t + 0.5) % 1.0
    lift = max(0.0, math.sin(phase * math.tau))
    return 5.0 * (lift ** 1.45)



def _walk_foot_pitch(side: str, t: float) -> float:
    phase = t if side == "near" else (t + 0.5) % 1.0
    s = math.sin(phase * math.tau)
    return -10.0 * max(0.0, s) + 6.0 * max(0.0, -s)


CLIP_WALK = Clip(
    loop=True,
    channels={
        "root_y": _walk_root_y,
        "body": lambda t: 6.0 * math.sin(t * math.tau),
        "head": lambda t: -7.5 * math.sin(t * math.tau + 0.2),
        "tail": lambda t: 16.0 * math.sin(t * math.tau + 0.85),
        "tail_fan": lambda t: 0.55 * math.sin(t * math.tau + 0.85),
        "near_wing_u": lambda t: -12.0 + 8.5 * math.sin(t * math.tau + 0.1),
        "near_wing_l": lambda t: -7.0 + 5.0 * math.sin(t * math.tau + 0.4),
        "far_wing_u": lambda t: -9.0 + 6.5 * math.sin(t * math.tau + 0.8),
        "far_wing_l": lambda t: -5.0 + 4.2 * math.sin(t * math.tau + 1.0),
        "near_foot_x": lambda t: _walk_foot_x("near", t),
        "far_foot_x": lambda t: _walk_foot_x("far", t),
        "near_foot_lift": lambda t: _walk_foot_lift("near", t),
        "far_foot_lift": lambda t: _walk_foot_lift("far", t),
        "near_foot_pitch": lambda t: _walk_foot_pitch("near", t),
        "far_foot_pitch": lambda t: _walk_foot_pitch("far", t),
        "blink": Channel((0.0, 0.0), (0.45, 0.0), (0.48, 1.0), (0.52, 0.0)),
        "eye_squint": 0.22,
        "look_x": 0.4,
        "beak_open": lambda t: 0.08 + 0.05 * max(0.0, math.sin(t * math.tau)),
    },
)


CLIP_FLY = Clip(
    loop=True,
    channels={
        "airborne": 1.0,
        "root_x": lambda t: 0.9 * math.sin(t * math.tau),
        "root_y": lambda t: -22.0 + 2.8 * math.sin(t * math.tau * 2.0 + 0.15),
        "body": lambda t: -6.0 + 7.5 * math.sin(t * math.tau * 2.0 + 0.1),
        "head": lambda t: -4.0 - 6.5 * math.sin(t * math.tau * 2.0 + 0.35),
        "tail": lambda t: 22.0 * math.sin(t * math.tau * 2.0 + math.pi),
        "tail_fan": lambda t: 0.65 + 0.35 * math.sin(t * math.tau * 2.0 + 0.85),
        "near_wing_u": lambda t: -28.0 + 40.0 * math.sin(t * math.tau * 2.0 + 0.05),
        "near_wing_l": lambda t: -6.0 + 30.0 * math.sin(t * math.tau * 2.0 - 0.05),
        "far_wing_u": lambda t: -22.0 + 32.0 * math.sin(t * math.tau * 2.0 + 0.18),
        "far_wing_l": lambda t: -4.0 + 22.0 * math.sin(t * math.tau * 2.0 + 0.05),
        "near_foot_x": lambda t: 5.2 + 1.2 * math.sin(t * math.tau + 0.3),
        "near_foot_y": lambda t: -8.2 + 1.0 * math.sin(t * math.tau * 2.0 + 1.1),
        "far_foot_x": lambda t: -0.6 + 1.0 * math.sin(t * math.tau + 0.8),
        "far_foot_y": lambda t: -9.4 + 0.8 * math.sin(t * math.tau * 2.0 + 0.5),
        "near_foot_pitch": lambda t: -6.0 + 5.0 * math.sin(t * math.tau + 0.6),
        "far_foot_pitch": lambda t: -9.0 + 4.0 * math.sin(t * math.tau + 1.0),
        "blink": Channel((0.0, 0.0), (0.30, 0.0), (0.33, 1.0), (0.37, 0.0), (0.78, 0.0), (0.81, 1.0), (0.84, 0.0)),
        "eye_squint": lambda t: 0.18 + 0.12 * max(0.0, math.sin(t * math.tau * 2.0)),
        "look_x": 0.55,
        "look_y": -0.08,
        "beak_open": lambda t: 0.05 + 0.06 * max(0.0, math.sin(t * math.tau * 2.0 + 0.3)),
    },
)


CLIP_TURNAROUND = Clip(
    loop=False,
    channels={
        # Grounded pivot: a planted, stepping turn with partially opened wings.
        "airborne": 0.0,
        "turn_front": Channel((0.0, 0.0), (0.50, 1.0, "sine"), (1.0, 0.0)),
        "turn_flip": Channel((0.0, 0.0), (0.50, 0.0), (0.501, 1.0), (1.0, 1.0)),
        "root_x": Channel((0.0, -1.8), (0.50, 0.0), (1.0, 1.8)),
        "root_y": Channel((0.0, -0.8), (0.25, -2.0), (0.50, -0.2), (0.75, -2.0), (1.0, -0.8)),
        "body": Channel((0.0, -3.0), (0.25, -12.0), (0.50, 0.0), (0.75, 12.0), (1.0, 3.0)),
        "head": Channel((0.0, -1.5), (0.25, -7.0), (0.50, 0.0), (0.75, 7.0), (1.0, 1.5)),
        "tail": Channel((0.0, 10.0), (0.25, 22.0), (0.50, 0.0), (0.75, -22.0), (1.0, -10.0)),
        "tail_fan": Channel((0.0, 0.36), (0.50, 0.82), (1.0, 0.36)),
        "near_wing_u": Channel((0.0, -10.0), (0.25, 0.0), (0.50, 10.0, "out"), (0.75, 0.0), (1.0, -10.0)),
        "near_wing_l": Channel((0.0, -7.0), (0.25, 1.5), (0.50, 8.0, "out"), (0.75, 1.5), (1.0, -7.0)),
        "far_wing_u": Channel((0.0, -8.0), (0.25, -1.0), (0.50, 7.0, "out"), (0.75, -1.0), (1.0, -8.0)),
        "far_wing_l": Channel((0.0, -5.0), (0.25, 0.5), (0.50, 6.0, "out"), (0.75, 0.5), (1.0, -5.0)),
        "near_foot_x": Channel((0.0, 5.0), (0.50, 0.8), (1.0, 5.0)),
        "near_foot_lift": Channel((0.0, 0.0), (0.25, 1.8), (0.50, 0.2), (0.75, 1.8), (1.0, 0.0)),
        "far_foot_x": Channel((0.0, -0.5), (0.50, -2.0), (1.0, -0.5)),
        "far_foot_lift": Channel((0.0, 1.0), (0.25, 0.1), (0.50, 1.8), (0.75, 0.1), (1.0, 1.0)),
        "near_foot_pitch": Channel((0.0, -4.0), (0.50, 2.0), (1.0, -4.0)),
        "far_foot_pitch": Channel((0.0, -6.0), (0.50, 0.0), (1.0, -6.0)),
        "beak_open": Channel((0.0, 0.04), (0.50, 0.0), (1.0, 0.04)),
        "eye_squint": Channel((0.0, 0.14), (0.50, 0.08), (1.0, 0.14)),
        "look_x": Channel((0.0, 0.50), (0.50, 0.0), (1.0, 0.50)),
        "look_y": -0.03,
        "blink": 0.0,
    },
)

CLIP_TURNAROUND_FLIGHT = Clip(
    loop=False,
    channels={
        # Airborne turn: big wing spread and tucked feet.
        "airborne": 1.0,
        "turn_front": Channel((0.0, 0.0), (0.50, 1.0, "sine"), (1.0, 0.0)),
        "turn_flip": Channel((0.0, 0.0), (0.50, 0.0), (0.501, 1.0), (1.0, 1.0)),
        "root_x": Channel((0.0, -2.5), (0.50, 0.0), (1.0, 2.5)),
        "root_y": Channel((0.0, -20.0), (0.25, -25.0), (0.50, -19.0), (0.75, -25.0), (1.0, -20.0)),
        "body": Channel((0.0, -8.0), (0.25, -20.0), (0.50, -1.0), (0.75, 20.0), (1.0, 8.0)),
        "head": Channel((0.0, -3.0), (0.25, -10.0), (0.50, 0.0), (0.75, 10.0), (1.0, 3.0)),
        "tail": Channel((0.0, 12.0), (0.25, 30.0), (0.50, 0.0), (0.75, -30.0), (1.0, -12.0)),
        "tail_fan": Channel((0.0, 0.45), (0.50, 1.0), (1.0, 0.45)),
        "near_wing_u": Channel((0.0, -24.0), (0.25, 34.0), (0.50, 58.0, "out"), (0.75, 34.0), (1.0, -24.0)),
        "near_wing_l": Channel((0.0, -10.0), (0.25, 24.0), (0.50, 42.0, "out"), (0.75, 24.0), (1.0, -10.0)),
        "far_wing_u": Channel((0.0, -18.0), (0.25, 24.0), (0.50, 44.0, "out"), (0.75, 24.0), (1.0, -18.0)),
        "far_wing_l": Channel((0.0, -7.0), (0.25, 18.0), (0.50, 32.0, "out"), (0.75, 18.0), (1.0, -7.0)),
        "near_foot_x": Channel((0.0, 5.2), (0.50, 1.8), (1.0, 5.2)),
        "near_foot_y": Channel((0.0, -8.6), (0.50, -11.6), (1.0, -8.6)),
        "far_foot_x": Channel((0.0, -0.6), (0.50, -1.8), (1.0, -0.6)),
        "far_foot_y": Channel((0.0, -9.6), (0.50, -11.0), (1.0, -9.6)),
        "near_foot_pitch": Channel((0.0, -6.0), (0.50, -1.0), (1.0, -6.0)),
        "far_foot_pitch": Channel((0.0, -9.0), (0.50, -2.0), (1.0, -9.0)),
        "beak_open": Channel((0.0, 0.05), (0.50, 0.0), (1.0, 0.05)),
        "eye_squint": Channel((0.0, 0.18), (0.50, 0.10), (1.0, 0.18)),
        "look_x": Channel((0.0, 0.55), (0.50, 0.0), (1.0, 0.55)),
        "look_y": -0.05,
        "blink": 0.0,
    },
)

CLIP_DIVE_BOMB = Clip(
    loop=False,
    channels={
        "airborne": 1.0,
        "root_x": Channel((0.0, -10.0), (0.22, -7.0), (0.55, 18.0, "out"), (0.78, 9.0), (1.0, 2.0)),
        "root_y": Channel((0.0, -30.0), (0.22, -34.0), (0.55, -6.0, "out"), (0.78, -16.0), (1.0, -22.0)),
        "body": Channel((0.0, -18.0), (0.22, -28.0), (0.55, 22.0, "out"), (0.78, -4.0), (1.0, -10.0)),
        "head": Channel((0.0, -8.0), (0.26, -16.0), (0.55, 20.0, "out"), (0.80, 4.0), (1.0, -4.0)),
        "tail": Channel((0.0, 18.0), (0.30, 32.0), (0.55, -18.0), (0.82, 10.0), (1.0, 14.0)),
        "tail_fan": Channel((0.0, 0.30), (0.45, 0.05), (0.72, 0.95), (1.0, 0.45)),
        "near_wing_u": Channel((0.0, -22.0), (0.35, -40.0), (0.56, -32.0), (0.72, 24.0, "out"), (1.0, -10.0)),
        "near_wing_l": Channel((0.0, -12.0), (0.35, -26.0), (0.56, -20.0), (0.72, 28.0, "out"), (1.0, -6.0)),
        "far_wing_u": Channel((0.0, -18.0), (0.35, -30.0), (0.56, -24.0), (0.72, 18.0, "out"), (1.0, -8.0)),
        "far_wing_l": Channel((0.0, -9.0), (0.35, -19.0), (0.56, -15.0), (0.72, 20.0, "out"), (1.0, -5.0)),
        "near_foot_x": Channel((0.0, 5.0), (0.55, 8.0), (1.0, 4.8)),
        "near_foot_y": Channel((0.0, -9.0), (0.55, -5.5), (1.0, -8.8)),
        "far_foot_x": Channel((0.0, -0.6), (0.55, 2.0), (1.0, -0.5)),
        "far_foot_y": Channel((0.0, -10.0), (0.55, -7.0), (1.0, -9.5)),
        "near_foot_pitch": Channel((0.0, -8.0), (0.55, 3.0), (1.0, -8.0)),
        "far_foot_pitch": Channel((0.0, -10.0), (0.55, 1.0), (1.0, -9.0)),
        "beak_open": Channel((0.0, 0.05), (0.40, 0.25), (0.56, 0.95, "out"), (0.80, 0.20), (1.0, 0.05)),
        "eye_squint": Channel((0.0, 0.25), (0.55, 0.75), (1.0, 0.22)),
        "look_x": 0.8,
        "blink": 0.0,
    },
)


CLIP_HOVER_PECK = Clip(
    loop=False,
    channels={
        "airborne": 1.0,
        "root_x": lambda t: 1.5 * math.sin(t * math.tau),
        "root_y": lambda t: -21.0 + 2.4 * math.sin(t * math.tau * 3.0),
        "body": lambda t: -4.0 + 4.0 * math.sin(t * math.tau * 3.0 + 0.3),
        "head": lambda t: -5.0 + 15.0 * max(0.0, math.sin(t * math.tau * 2.0 - 0.35)),
        "tail": lambda t: 14.0 * math.sin(t * math.tau * 3.0 + math.pi),
        "tail_fan": lambda t: 0.5 + 0.25 * math.sin(t * math.tau * 3.0),
        "near_wing_u": lambda t: -24.0 + 44.0 * math.sin(t * math.tau * 3.0),
        "near_wing_l": lambda t: -8.0 + 34.0 * math.sin(t * math.tau * 3.0 - 0.1),
        "far_wing_u": lambda t: -20.0 + 34.0 * math.sin(t * math.tau * 3.0 + 0.2),
        "far_wing_l": lambda t: -6.0 + 24.0 * math.sin(t * math.tau * 3.0),
        "near_foot_x": lambda t: 5.8 + 0.7 * math.sin(t * math.tau * 2.0 + 0.5),
        "near_foot_y": lambda t: -8.0 + 0.9 * math.sin(t * math.tau * 3.0 + 1.0),
        "far_foot_x": lambda t: -0.2 + 0.5 * math.sin(t * math.tau * 2.0 + 1.0),
        "far_foot_y": lambda t: -9.2 + 0.8 * math.sin(t * math.tau * 3.0 + 0.45),
        "near_foot_pitch": -5.0,
        "far_foot_pitch": -8.0,
        "beak_open": lambda t: 0.10 + 0.80 * max(0.0, math.sin(t * math.tau * 2.0 - 0.2)),
        "eye_squint": lambda t: 0.26 + 0.25 * max(0.0, math.sin(t * math.tau * 2.0 - 0.2)),
        "look_x": 0.8,
        "blink": 0.0,
    },
)


CLIP_BANKED_STRAFE = Clip(
    loop=False,
    channels={
        "airborne": 1.0,
        "root_x": Channel((0.0, -14.0), (0.35, -4.0), (0.70, 16.0, "out"), (1.0, 22.0)),
        "root_y": Channel((0.0, -22.0), (0.30, -28.0), (0.65, -18.0), (1.0, -24.0)),
        "body": Channel((0.0, -18.0), (0.35, -34.0), (0.70, 22.0, "out"), (1.0, 8.0)),
        "head": Channel((0.0, 4.0), (0.35, 12.0), (0.70, -8.0), (1.0, -2.0)),
        "tail": Channel((0.0, 28.0), (0.35, 38.0), (0.70, -12.0), (1.0, 4.0)),
        "tail_fan": Channel((0.0, 0.85), (0.50, 1.0), (1.0, 0.6)),
        "near_wing_u": Channel((0.0, -36.0), (0.35, -48.0), (0.70, 24.0), (1.0, 10.0)),
        "near_wing_l": Channel((0.0, -18.0), (0.35, -28.0), (0.70, 30.0), (1.0, 18.0)),
        "far_wing_u": Channel((0.0, 18.0), (0.35, 34.0), (0.70, -24.0), (1.0, -10.0)),
        "far_wing_l": Channel((0.0, 16.0), (0.35, 26.0), (0.70, -14.0), (1.0, -5.0)),
        "near_foot_x": Channel((0.0, 4.5), (0.5, 7.5), (1.0, 5.0)),
        "near_foot_y": Channel((0.0, -8.5), (0.5, -6.5), (1.0, -8.0)),
        "far_foot_x": Channel((0.0, -1.0), (0.5, 1.0), (1.0, -0.3)),
        "far_foot_y": Channel((0.0, -9.8), (0.5, -7.6), (1.0, -9.0)),
        "near_foot_pitch": Channel((0.0, -18.0), (0.5, 2.0), (1.0, -8.0)),
        "far_foot_pitch": Channel((0.0, 8.0), (0.5, -12.0), (1.0, -8.0)),
        "beak_open": 0.12,
        "eye_squint": 0.35,
        "look_x": 0.75,
        "blink": 0.0,
    },
)

CLIP_SLASH = Clip(
    loop=False,
    channels={
        "airborne": 1.0,
        "root_x": Channel((0.0, -3.0), (0.18, -8.0), (0.48, 16.0, "out"), (0.78, 7.0), (1.0, 2.0)),
        "root_y": Channel((0.0, -18.0), (0.22, -24.0), (0.44, -12.0, "out"), (0.72, -19.0), (1.0, -20.0)),
        "body": Channel((0.0, -5.0), (0.22, -15.0), (0.45, 12.0, "out"), (0.7, -3.0), (1.0, -6.0)),
        "head": Channel((0.0, -2.0), (0.22, -12.0), (0.44, 17.0, "out"), (0.72, 5.0), (1.0, -2.0)),
        "tail": Channel((0.0, 10.0), (0.24, 24.0), (0.46, -10.0), (0.76, 8.0), (1.0, 12.0)),
        "tail_fan": Channel((0.0, 0.35), (0.32, 0.95), (0.55, 0.1), (1.0, 0.4)),
        "near_wing_u": Channel((0.0, -22.0), (0.20, 8.0), (0.42, 38.0, "out"), (0.68, -6.0), (1.0, -18.0)),
        "near_wing_l": Channel((0.0, -8.0), (0.18, 16.0), (0.42, 26.0, "out"), (0.68, -4.0), (1.0, -6.0)),
        "far_wing_u": Channel((0.0, -18.0), (0.20, 4.0), (0.42, 28.0, "out"), (0.68, -3.0), (1.0, -14.0)),
        "far_wing_l": Channel((0.0, -6.0), (0.18, 12.0), (0.42, 21.0, "out"), (0.68, -2.0), (1.0, -5.0)),
        "near_foot_x": Channel((0.0, 4.8), (0.45, 7.6), (1.0, 4.7)),
        "near_foot_y": Channel((0.0, -7.8), (0.45, -5.8), (1.0, -8.5)),
        "far_foot_x": Channel((0.0, -0.8), (0.45, 1.8), (1.0, -0.5)),
        "far_foot_y": Channel((0.0, -9.5), (0.45, -7.2), (1.0, -9.3)),
        "near_foot_pitch": -6.0,
        "far_foot_pitch": -9.0,
        "beak_open": Channel((0.0, 0.10), (0.22, 0.55), (0.44, 1.0, "out"), (0.68, 0.18), (1.0, 0.08)),
        "eye_squint": Channel((0.0, 0.2), (0.24, 0.35), (0.44, 0.7), (0.70, 0.22), (1.0, 0.18)),
        "look_x": 0.75,
        "blink": 0.0,
    },
)

CLIP_TAUNT = Clip(
    loop=True,
    channels={
        "airborne": 1.0,
        "root_x": lambda t: 1.1 * math.sin(t * math.tau),
        "root_y": lambda t: -18.0 + 3.2 * math.sin(t * math.tau * 2.0) ** 2,
        "body": lambda t: -4.0 + 6.0 * math.sin(t * math.tau * 2.0 + 0.1),
        "head": lambda t: 6.0 * math.sin(t * math.tau + 0.35),
        "tail": lambda t: 24.0 * math.sin(t * math.tau * 2.0 + 0.7),
        "tail_fan": lambda t: 0.8 + 0.4 * math.sin(t * math.tau * 2.0 + 0.7),
        "near_wing_u": lambda t: -18.0 + 34.0 * max(-0.2, math.sin(t * math.tau * 2.0 + 0.1)),
        "near_wing_l": lambda t: -4.0 + 24.0 * max(-0.2, math.sin(t * math.tau * 2.0 + 0.1)),
        "far_wing_u": lambda t: -14.0 + 28.0 * max(-0.25, math.sin(t * math.tau * 2.0 + 0.28)),
        "far_wing_l": lambda t: -2.0 + 18.0 * max(-0.25, math.sin(t * math.tau * 2.0 + 0.28)),
        "near_foot_x": lambda t: 5.5 + 1.0 * math.sin(t * math.tau + 0.2),
        "near_foot_y": lambda t: -8.0 + 1.2 * math.sin(t * math.tau * 2.0 + 1.0),
        "far_foot_x": lambda t: -0.2 + 0.8 * math.sin(t * math.tau + 0.8),
        "far_foot_y": lambda t: -9.2 + 1.0 * math.sin(t * math.tau * 2.0 + 0.45),
        "near_foot_pitch": -5.0,
        "far_foot_pitch": -8.0,
        "beak_open": lambda t: 0.22 + 0.7 * max(0.0, math.sin(t * math.tau + 0.12)),
        "blink": Channel((0.0, 0.0), (0.22, 0.0), (0.24, 1.0), (0.27, 0.0), (0.8, 0.0), (0.82, 1.0), (0.86, 0.0)),
        "eye_squint": lambda t: 0.24 + 0.18 * max(0.0, math.sin(t * math.tau + 0.2)),
        "look_x": lambda t: 0.5 + 0.2 * math.sin(t * math.tau + 0.4),
        "look_y": lambda t: -0.15,
    },
)

CLIP_HURT = Clip(
    loop=False,
    channels={
        "airborne": 1.0,
        "root_x": Channel((0.0, 0.0), (0.18, -6.0), (0.44, 3.0), (1.0, -1.0)),
        "root_y": Channel((0.0, -16.0), (0.22, -10.0), (0.44, -18.0), (1.0, -20.0)),
        "body": Channel((0.0, -4.0), (0.20, -16.0), (0.44, 5.0), (1.0, -5.0)),
        "head": Channel((0.0, 0.0), (0.20, 11.0), (0.44, -8.0), (1.0, -2.0)),
        "tail": Channel((0.0, 10.0), (0.20, -12.0), (0.44, 16.0), (1.0, 8.0)),
        "near_wing_u": Channel((0.0, 2.0), (0.18, 26.0), (0.5, -10.0), (1.0, -20.0)),
        "near_wing_l": Channel((0.0, 0.0), (0.18, 22.0), (0.5, -6.0), (1.0, -8.0)),
        "far_wing_u": Channel((0.0, 0.0), (0.18, 18.0), (0.5, -8.0), (1.0, -16.0)),
        "far_wing_l": Channel((0.0, 0.0), (0.18, 13.0), (0.5, -4.0), (1.0, -6.0)),
        "near_foot_x": 6.0,
        "near_foot_y": -6.2,
        "far_foot_x": 0.8,
        "far_foot_y": -7.4,
        "near_foot_pitch": -1.0,
        "far_foot_pitch": -3.0,
        "beak_open": Channel((0.0, 0.0), (0.2, 0.6), (0.5, 0.12), (1.0, 0.0)),
        "eye_squint": 0.7,
        "blink": 0.0,
        "look_x": -0.3,
    },
)

CLIP_DEATH = Clip(
    loop=False,
    channels={
        "root_x": Channel((0.0, 0.0), (0.36, 6.0), (0.72, 14.0), (1.0, 16.0)),
        "root_y": Channel((0.0, 0.0), (0.30, -3.0), (0.60, 7.0), (1.0, 17.0)),
        "body": Channel((0.0, 0.0), (0.32, 18.0), (0.55, 58.0, "out"), (1.0, 86.0)),
        "head": Channel((0.0, 0.0), (0.32, -14.0), (0.60, -26.0), (1.0, -18.0)),
        "tail": Channel((0.0, 0.0), (0.35, -24.0), (0.70, -34.0), (1.0, -30.0)),
        "tail_fan": Channel((0.0, 0.0), (0.50, 0.8), (1.0, 0.1)),
        "near_wing_u": Channel((0.0, 0.0), (0.35, 28.0), (0.65, 44.0), (1.0, 40.0)),
        "near_wing_l": Channel((0.0, 0.0), (0.35, 22.0), (0.65, 35.0), (1.0, 30.0)),
        "far_wing_u": Channel((0.0, 0.0), (0.35, 22.0), (0.65, 34.0), (1.0, 30.0)),
        "far_wing_l": Channel((0.0, 0.0), (0.35, 16.0), (0.65, 24.0), (1.0, 20.0)),
        "beak_open": Channel((0.0, 0.0), (0.45, 0.7), (1.0, 0.18)),
        "eye_squint": Channel((0.0, 0.1), (0.45, 0.8), (1.0, 1.0)),
        "blink": Channel((0.0, 0.0), (0.7, 0.0), (0.76, 1.0), (1.0, 1.0)),
        "look_x": -0.6,
    },
)

CLIPS: Dict[str, Clip] = {
    "idle": CLIP_IDLE,
    "walk": CLIP_WALK,
    "fly": CLIP_FLY,
    "turnaround": CLIP_TURNAROUND,
    "turnaround_flight": CLIP_TURNAROUND_FLIGHT,
    "dive_bomb": CLIP_DIVE_BOMB,
    "hover_peck": CLIP_HOVER_PECK,
    "banked_strafe": CLIP_BANKED_STRAFE,
    "slash": CLIP_SLASH,
    "taunt": CLIP_TAUNT,
    "hurt": CLIP_HURT,
    "death": CLIP_DEATH,
}


# ---- Solving / rendering ------------------------------------------------------


def _foot_target(sampled: Dict[str, float], side: str, root: Point) -> Point:
    airborne = sampled.get("airborne", 0.0) > 0.5
    if airborne:
        dx, dy = DEFAULT_AIR_FOOT[side]
        ax = root[0] + sampled.get(f"{side}_foot_x", dx)
        ay = root[1] + sampled.get(f"{side}_foot_y", dy)
        return (ax, ay)
    ax = CENTER_X + sampled.get(f"{side}_foot_x", DEFAULT_FOOT_X[side])
    ay = GROUND_Y - ANKLE_H - sampled.get(f"{side}_foot_lift", 0.0)
    return (ax, ay)



def _solve(animation: str, t: float):
    sampled = CLIPS[animation].sample(t)
    root = (CENTER_X + sampled.get("root_x", 0.0), GROUND_Y + sampled.get("root_y", 0.0))
    angles = {name: val for name, val in sampled.items() if name in _SKEL.bones}
    w0 = _SKEL.world(angles, root=root)
    for side in ("far", "near"):
        hip = w0[f"{side}_leg_u"].origin
        ankle = _foot_target(sampled, side, root)
        a1, a2 = two_bone_ik(hip, ankle, LEG_U, LEG_L, bend=1.0)
        body_angle = w0["body"].angle
        angles[f"{side}_leg_u"] = a1 - body_angle - 90.0
        angles[f"{side}_leg_l"] = a2 - a1
        pitch = sampled.get(f"{side}_foot_pitch", 0.0)
        angles[f"{side}_foot"] = pitch - a2 + 90.0
    world = _SKEL.world(angles, root=root)
    return world, sampled



def _render_side_actor(world, params: Dict[str, float], mirrored: bool = False) -> Image.Image:
    actor = Image.new("RGBA", (FRAME_W * SS, FRAME_H * SS), (0, 0, 0, 0))
    _RIG.draw(actor, ImageDraw.Draw(actor), world, SS, params)
    if mirrored:
        actor = ImageOps.mirror(actor)
    return actor



def _turn_pt(pt: Point, cx: float, cy: float, mirror: bool = False) -> Point:
    x, y = pt
    if mirror:
        x = -x
    return (cx + x * SS, cy + y * SS)



def _turn_pts(pts: List[Point], cx: float, cy: float, mirror: bool = False) -> List[Point]:
    return [_turn_pt(p, cx, cy, mirror) for p in pts]



def _turn_poly(draw: ImageDraw.ImageDraw, pts: List[Point], cx: float, cy: float, fill: Color, outline: Color = PAL["outline"], width: float = 0.7, radius: float = 1.8, mirror: bool = False) -> None:
    poly = rounded_polygon(_turn_pts(pts, cx, cy, mirror), radius=SS * radius)
    draw_polygon(draw, poly, fill, outline, SS * width)



def _turn_fill(img: Image.Image, pts: List[Point], cx: float, cy: float, fill: Color, radius: float = 1.6, mirror: bool = False) -> None:
    poly = rounded_polygon(_turn_pts(pts, cx, cy, mirror), radius=SS * radius)
    composite_polygon(img, poly, fill)



def _turn_rot(center: Point, angle_deg: float, along: float, across: float) -> Point:
    rad = math.radians(angle_deg)
    ca, sa = math.cos(rad), math.sin(rad)
    return (center[0] + ca * along - sa * across, center[1] + sa * along + ca * across)



def _turn_feather_pts(origin: Point, angle_deg: float, length: float, base_w: float, tip_w: float) -> List[Point]:
    return [
        _turn_rot(origin, angle_deg, -0.5, -base_w),
        _turn_rot(origin, angle_deg, length * 0.18, -base_w * 0.94),
        _turn_rot(origin, angle_deg, length * 0.55, -base_w * 0.64),
        _turn_rot(origin, angle_deg, length * 0.84, -tip_w * 1.08),
        _turn_rot(origin, angle_deg, length + 0.45, 0.0),
        _turn_rot(origin, angle_deg, length * 0.84, tip_w * 1.08),
        _turn_rot(origin, angle_deg, length * 0.55, base_w * 0.64),
        _turn_rot(origin, angle_deg, length * 0.18, base_w * 0.94),
        _turn_rot(origin, angle_deg, -0.35, base_w),
    ]



def _turn_draw_feather(draw: ImageDraw.ImageDraw, cx: float, cy: float, origin: Point, angle_deg: float, length: float, base_w: float, tip_w: float, fill: Color, mirror: bool = False, width: float = 0.3, radius: float = 0.9) -> None:
    pts = _turn_feather_pts(origin, angle_deg, length, base_w, tip_w)
    poly = rounded_polygon(_turn_pts(pts, cx, cy, mirror), radius=SS * radius)
    draw_polygon(draw, poly, fill, PAL["outline"], SS * width)
    s0 = _turn_pt(_turn_rot(origin, angle_deg, 0.4, -0.08), cx, cy, mirror)
    s1 = _turn_pt(_turn_rot(origin, angle_deg, length * 0.82, 0.0), cx, cy, mirror)
    draw.line((s0[0], s0[1], s1[0], s1[1]), fill=(*PAL["outline"][:3], 138), width=max(1, int(SS * 0.18)))



def _draw_turn_three_quarter_wing(img: Image.Image, draw: ImageDraw.ImageDraw, cx: float, cy: float, wing_lift: float, near: bool, airborne: bool, mirror: bool = False) -> None:
    sign = 1.0 if near else -1.0
    spread = (0.52 + 0.56 * wing_lift) if airborne else (0.18 + 0.42 * wing_lift)
    lift_y = 4.0 + 10.0 * spread
    shoulder_fill = PAL["body"] if near else PAL["body_dark"]
    covert_fill = PAL["wing_yellow"] if near else PAL["wing_gold"]
    primary_fill = PAL["wing_blue"] if near else PAL["wing_blue_dark"]
    tip_fill = PAL["wing_blue_dark"] if near else PAL["wing_blue"]
    hi_alpha = 178 if near else 112

    shoulder = [
        (sign * 4.0, -8.2),
        (sign * 7.2, -14.4 - 0.42 * lift_y),
        (sign * 10.9, -14.0 - 0.58 * lift_y),
        (sign * 12.2, -8.0 + 0.06 * lift_y),
        (sign * 9.2, -0.6 + 0.34 * lift_y),
        (sign * 4.6, -2.4),
    ]
    _turn_poly(draw, shoulder, cx, cy, shoulder_fill, width=0.5 if near else 0.46, radius=1.75, mirror=mirror)
    _turn_fill(img, [
        (sign * 5.1, -7.6),
        (sign * 8.1, -11.9 - 0.22 * lift_y),
        (sign * 10.4, -11.2 - 0.34 * lift_y),
        (sign * 8.8, -4.6),
        (sign * 5.5, -5.0),
    ], cx, cy, (*PAL["body_light"][:3], 120 if near else 80), radius=1.0, mirror=mirror)

    coverts = [
        (sign * 6.8, -11.0 - 0.24 * lift_y),
        (sign * 11.8, -16.4 - 0.56 * lift_y),
        (sign * 17.8, -15.1 - 0.58 * lift_y),
        (sign * 20.0, -8.8 - 0.02 * lift_y),
        (sign * 18.2, -1.4 + 0.42 * lift_y),
        (sign * 12.0, 1.0 + 0.42 * lift_y),
        (sign * 8.2, -0.4 + 0.22 * lift_y),
    ]
    _turn_poly(draw, coverts, cx, cy, covert_fill, width=0.48, radius=1.95, mirror=mirror)
    _turn_fill(img, [
        (sign * 9.1, -10.0 - 0.18 * lift_y),
        (sign * 14.8, -12.6 - 0.40 * lift_y),
        (sign * 17.0, -7.4 + 0.02 * lift_y),
        (sign * 13.0, -2.0 + 0.20 * lift_y),
        (sign * 9.8, -3.1 + 0.06 * lift_y),
    ], cx, cy, (*PAL["wing_yellow"][:3], hi_alpha), radius=1.15, mirror=mirror)
    feather_bed = [
        (sign * 10.6, -10.6 - 0.20 * lift_y),
        (sign * 14.8, -10.2 - 0.24 * lift_y),
        (sign * 19.2, -8.5 - 0.16 * lift_y),
        (sign * 19.4, -4.8 + 0.10 * lift_y),
        (sign * 15.4, -2.4 + 0.24 * lift_y),
        (sign * 11.5, -3.4 + 0.14 * lift_y),
    ]
    _turn_fill(img, feather_bed, cx, cy, (*covert_fill[:3], 214 if near else 168), radius=1.05, mirror=mirror)

    feather_data = [
        ((sign * 9.8, -12.0 - 0.34 * lift_y), -48.0 * sign, 7.8 + 1.6 * spread, 1.48, 0.78, covert_fill),
        ((sign * 11.8, -11.2 - 0.24 * lift_y), -31.0 * sign, 8.5 + 2.0 * spread, 1.52, 0.8, covert_fill),
        ((sign * 13.8, -9.8 - 0.12 * lift_y), -16.0 * sign, 9.0 + 2.2 * spread, 1.52, 0.78, covert_fill),
        ((sign * 15.2, -8.0 - 0.04 * lift_y), -2.0 * sign, 9.4 + 2.4 * spread, 1.46, 0.74, covert_fill),
        ((sign * 15.8, -12.0 - 0.42 * lift_y), -30.0 * sign, 9.8 + 2.5 * spread, 1.36, 0.64, primary_fill),
        ((sign * 17.6, -9.3 - 0.26 * lift_y), -12.0 * sign, 10.8 + 2.8 * spread, 1.36, 0.6, primary_fill),
        ((sign * 18.6, -6.0 - 0.02 * lift_y), 6.0 * sign, 11.6 + 3.0 * spread, 1.3, 0.58, tip_fill),
        ((sign * 18.6, -2.2 + 0.18 * lift_y), 22.0 * sign, 10.8 + 2.8 * spread, 1.18, 0.52, tip_fill),
    ]
    for origin, ang, length, base_w, tip_w, fill in reversed(feather_data):
        _turn_draw_feather(draw, cx, cy, origin, ang, length, base_w, tip_w, fill, mirror=mirror, width=0.28 if near else 0.25, radius=0.82)

    join_coverts = [
        (sign * 8.6, -11.3 - 0.22 * lift_y),
        (sign * 11.5, -12.1 - 0.26 * lift_y),
        (sign * 14.6, -11.0 - 0.18 * lift_y),
        (sign * 14.9, -8.0 - 0.04 * lift_y),
        (sign * 11.9, -7.2 + 0.04 * lift_y),
        (sign * 9.2, -8.1 + 0.02 * lift_y),
    ]
    _turn_fill(img, join_coverts, cx, cy, (*covert_fill[:3], 186 if near else 138), radius=0.82, mirror=mirror)

def _draw_turn_front_wing(img: Image.Image, draw: ImageDraw.ImageDraw, cx: float, cy: float, wing_lift: float, airborne: bool, mirror: bool = False) -> None:
    spread = (0.58 + 0.62 * wing_lift) if airborne else (0.14 + 0.36 * wing_lift)
    lift_y = 5.0 + 11.0 * spread
    shoulder = [
        (3.0, -7.1),
        (5.8, -13.8 - 0.34 * lift_y),
        (9.0, -13.5 - 0.44 * lift_y),
        (10.0, -7.0 + 0.12 * lift_y),
        (7.2, -0.8 + 0.34 * lift_y),
        (3.6, -2.3),
    ]
    _turn_poly(draw, shoulder, cx, cy, PAL["body"], width=0.46, radius=1.55, mirror=mirror)
    _turn_fill(img, [(4.3, -7.0), (6.6, -11.1 - 0.18 * lift_y), (8.2, -10.7 - 0.22 * lift_y), (7.0, -5.1), (4.8, -4.8)], cx, cy, (*PAL["body_light"][:3], 118), radius=0.9, mirror=mirror)

    coverts = [
        (5.1, -10.8 - 0.22 * lift_y),
        (9.8, -16.3 - 0.50 * lift_y),
        (15.6, -15.5 - 0.52 * lift_y),
        (19.0, -9.0 - 0.06 * lift_y),
        (18.2, -1.0 + 0.52 * lift_y),
        (12.2, 2.1 + 0.34 * lift_y),
        (8.0, 0.8 + 0.26 * lift_y),
    ]
    _turn_poly(draw, coverts, cx, cy, PAL["wing_yellow"], width=0.46, radius=1.9, mirror=mirror)
    _turn_fill(img, [(7.0, -10.0 - 0.14 * lift_y), (12.8, -13.0 - 0.38 * lift_y), (15.8, -7.0 + 0.02 * lift_y), (12.0, -1.4 + 0.20 * lift_y), (8.8, -2.4)], cx, cy, (*PAL["wing_gold"][:3], 162), radius=1.02, mirror=mirror)
    _turn_fill(img, [(8.2, -10.4 - 0.12 * lift_y), (12.2, -10.0 - 0.16 * lift_y), (17.0, -8.2 - 0.10 * lift_y), (17.2, -4.5 + 0.10 * lift_y), (13.2, -2.1 + 0.20 * lift_y), (9.4, -3.0)], cx, cy, (*PAL["wing_yellow"][:3], 214), radius=1.0, mirror=mirror)

    feathers = [
        ((7.2, -12.0 - 0.28 * lift_y), -44.0, 7.9 + 1.8 * spread, 1.45, 0.78, PAL["wing_yellow"]),
        ((9.3, -11.4 - 0.22 * lift_y), -28.0, 8.7 + 2.1 * spread, 1.5, 0.8, PAL["wing_yellow"]),
        ((11.5, -10.4 - 0.14 * lift_y), -12.0, 9.5 + 2.4 * spread, 1.52, 0.78, PAL["wing_yellow"]),
        ((13.6, -9.0 - 0.06 * lift_y), 2.0, 10.2 + 2.7 * spread, 1.42, 0.68, PAL["wing_blue"]),
        ((15.2, -6.8 + 0.04 * lift_y), 18.0, 10.9 + 2.9 * spread, 1.3, 0.58, PAL["wing_blue"]),
        ((15.8, -3.2 + 0.14 * lift_y), 33.0, 9.9 + 2.7 * spread, 1.14, 0.48, PAL["wing_blue_dark"]),
    ]
    for origin, ang, length, base_w, tip_w, fill in reversed(feathers):
        _turn_draw_feather(draw, cx, cy, origin, ang, length, base_w, tip_w, fill, mirror=mirror, width=0.25, radius=0.8)
    _turn_fill(img, [(6.7, -10.7 - 0.16 * lift_y), (9.6, -11.2 - 0.18 * lift_y), (12.4, -10.2 - 0.10 * lift_y), (12.5, -7.6), (10.0, -6.6), (7.4, -7.3)], cx, cy, (*PAL["wing_yellow"][:3], 184), radius=0.78, mirror=mirror)

def _render_turn_three_quarter(params: Dict[str, float], mirrored: bool = False) -> Image.Image:
    img = Image.new("RGBA", (FRAME_W * SS, FRAME_H * SS), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    cx = (CENTER_X + params.get("root_x", 0.0)) * SS
    airborne = params.get("airborne", 0.0) > 0.5
    default_root_y = -21.0 if airborne else -1.0
    cy = (GROUND_Y + params.get("root_y", default_root_y) - 22.0) * SS
    wing_src = params.get("near_wing_u", -24.0 if airborne else -10.0)
    wing_base = -24.0 if airborne else -10.0
    wing_span = 82.0 if airborne else 28.0
    wing_lift = clamp((wing_src - wing_base) / wing_span, 0.0, 1.0)
    beak_open = clamp(params.get("beak_open", 0.0), 0.0, 1.0)
    tail_fan = clamp(params.get("tail_fan", 0.5), 0.0, 1.0)

    shadow_w = SS * ((15.0 if airborne else 20.0) - (2.0 if airborne else 3.0) * wing_lift)
    shadow_h = SS * (3.0 if airborne else 4.0)
    sx, sy = (CENTER_X + params.get("root_x", 0.0)) * SS, (GROUND_Y + 0.5) * SS
    shadow = (*PAL["shadow"][:3], 46 if airborne else PAL["shadow"][3])
    draw.ellipse((sx - shadow_w, sy - shadow_h, sx + shadow_w, sy + shadow_h), fill=shadow)

    for dx, top, mid, bot, color in [
        (-9.0, 4.0, 17.0 + 3.0 * tail_fan, 28.0, PAL["body_dark"]),
        (-2.5, 3.0, 18.0 + 4.0 * tail_fan, 29.5, PAL["body"]),
        (4.0, 4.0, 16.0 + 3.0 * tail_fan, 27.5, PAL["wing_blue"]),
    ]:
        tail = [(dx - 1.3, 0.0), (dx + 2.2, top), (dx + 5.2, mid), (dx + 0.8, bot), (dx - 3.2, mid - 2.5)]
        _turn_poly(draw, tail, cx - 10.0 * SS, cy + 5.0 * SS, color, width=0.45, radius=1.25, mirror=mirrored)

    _draw_turn_three_quarter_wing(img, draw, cx, cy, wing_lift, near=False, airborne=airborne, mirror=mirrored)

    body = [(-17.0, -13.0), (-2.0, -18.5), (13.0, -14.0), (19.0, -2.0), (16.0, 12.0), (4.0, 20.0), (-11.0, 15.0), (-19.0, 3.5)]
    _turn_poly(draw, body, cx, cy, PAL["body"], width=0.8, radius=4.0, mirror=mirrored)
    _turn_fill(img, [(-10.0, -7.0), (6.0, -8.5), (10.5, 5.0), (4.5, 15.0), (-6.5, 12.0), (-9.5, 1.0)], cx, cy, (*PAL["body_light"][:3], 88), radius=3.0, mirror=mirrored)
    _turn_fill(img, [(-16.0, -11.0), (-6.0, -14.0), (-4.0, 11.0), (-14.0, 8.5)], cx, cy, (*PAL["body_dark"][:3], 110), radius=2.8, mirror=mirrored)
    _turn_fill(img, [(-1.0, -13.2), (7.5, -14.8), (11.5, -9.8), (4.0, -5.0), (-1.5, -6.5)], cx, cy, (*PAL["wing_yellow"][:3], 150), radius=2.0, mirror=mirrored)

    _draw_turn_three_quarter_wing(img, draw, cx, cy, wing_lift, near=True, airborne=airborne, mirror=mirrored)

    if airborne:
        for dx, dy, pitch in [(-1.6, 10.0, -0.6), (3.0, 10.8, -0.2)]:
            shin = [(dx - 0.6, dy - 0.5), (dx + 0.4, dy - 0.2), (dx + 0.8, dy + 2.4), (dx - 0.2, dy + 2.7)]
            _turn_poly(draw, shin, cx, cy, PAL["leg"], width=0.35, radius=0.7, mirror=mirrored)
            base = _turn_pt((dx + 0.2, dy + 2.4), cx, cy, mirror=mirrored)
            for spread, length in ((-0.5, 2.5), (0.0, 3.0), (0.5, 2.3)):
                sign = -1.0 if mirrored else 1.0
                tx = base[0] + sign * SS * length
                ty = base[1] + SS * (spread + pitch * 1.6)
                draw.line((base[0], base[1], tx, ty), fill=PAL["talon"], width=max(1, int(SS * 0.45)))
    else:
        for dx, dy, pitch in [(-2.0, 13.5, -0.2), (4.5, 14.8, 0.15)]:
            shin = [(dx - 0.8, dy - 0.8), (dx + 0.4, dy - 0.5), (dx + 0.9, dy + 3.5), (dx - 0.4, dy + 3.7)]
            _turn_poly(draw, shin, cx, cy, PAL["leg"], width=0.4, radius=0.8, mirror=mirrored)
            base = _turn_pt((dx + 0.2, dy + 3.3), cx, cy, mirror=mirrored)
            for spread, length in ((-0.8, 3.3), (0.0, 3.9), (0.8, 3.1)):
                sign = -1.0 if mirrored else 1.0
                tx = base[0] + sign * SS * length
                ty = base[1] + SS * (spread + pitch * 2.2)
                draw.line((base[0], base[1], tx, ty), fill=PAL["talon"], width=max(1, int(SS * 0.5)))

    head = [(-4.0, -23.0), (8.5, -23.8), (17.0, -16.0), (17.0, -4.0), (10.0, 6.5), (-1.5, 6.0), (-10.0, -1.5), (-10.0, -13.5)]
    _turn_poly(draw, head, cx + 4.0 * SS, cy - 1.0 * SS, PAL["head"], width=0.8, radius=3.7, mirror=mirrored)
    _turn_fill(img, [(-0.5, -20.0), (8.5, -20.5), (13.0, -13.0), (11.5, -6.0), (2.0, -5.8), (-1.8, -10.5)], cx + 4.0 * SS, cy - 1.0 * SS, (*PAL["head_light"][:3], 110), radius=2.5, mirror=mirrored)
    _turn_fill(img, [(2.5, -18.0), (10.0, -17.2), (12.2, -3.0), (10.0, 4.5), (4.5, 5.5), (0.8, 1.0), (0.8, -10.0)], cx + 4.0 * SS, cy - 1.0 * SS, PAL["face_patch"], radius=2.3, mirror=mirrored)
    for x0, y0, x1, y1 in ((4.0, -14.0, 3.4, -2.0), (6.2, -12.5, 5.8, -0.5), (8.2, -10.5, 8.0, 1.5)):
        p0 = _turn_pt((x0, y0), cx + 4.0 * SS, cy - 1.0 * SS, mirror=mirrored)
        p1 = _turn_pt((x1, y1), cx + 4.0 * SS, cy - 1.0 * SS, mirror=mirrored)
        draw.line((p0[0], p0[1], p1[0], p1[1]), fill=PAL["face_line"], width=max(1, int(SS * 0.28)))

    eye_c = _turn_pt((7.0, -11.0), cx + 4.0 * SS, cy - 1.0 * SS, mirror=mirrored)
    ew, eh = SS * 4.7, SS * 5.3
    draw.ellipse((eye_c[0] - ew / 2, eye_c[1] - eh / 2, eye_c[0] + ew / 2, eye_c[1] + eh / 2), fill=PAL["eye"], outline=PAL["outline"], width=max(1, int(SS * 0.35)))
    pw, ph = SS * 1.55, SS * 2.2
    pupil_dx = (-0.3 if mirrored else 0.3) * SS
    draw.ellipse((eye_c[0] - pw / 2 + pupil_dx, eye_c[1] - ph / 2, eye_c[0] + pw / 2 + pupil_dx, eye_c[1] + ph / 2), fill=PAL["pupil"])
    tiny_eye = _turn_pt((1.6, -11.5), cx + 4.0 * SS, cy - 1.0 * SS, mirror=mirrored)
    draw.ellipse((tiny_eye[0] - SS * 1.2, tiny_eye[1] - SS * 1.5, tiny_eye[0] + SS * 1.2, tiny_eye[1] + SS * 1.5), fill=(*PAL["eye"][:3], 170), outline=(*PAL["outline"][:3], 180), width=max(1, int(SS * 0.22)))
    draw.ellipse((tiny_eye[0] - SS * 0.35, tiny_eye[1] - SS * 0.55, tiny_eye[0] + SS * 0.35, tiny_eye[1] + SS * 0.55), fill=(*PAL["pupil"][:3], 180))

    beak_cx = cx + 14.0 * SS * (-1.0 if mirrored else 1.0)
    beak_cy = cy - 7.0 * SS
    upper = [(-1.0, -4.0), (6.0, -6.2), (13.0, -4.5), (17.0, -0.2), (16.0, 4.8), (12.0, 10.0), (4.5, 8.2), (0.5, 3.0)]
    lower = [(-0.5, 3.0 + 2.0 * beak_open), (5.0, 5.0 + 5.0 * beak_open), (10.5, 5.8 + 6.5 * beak_open), (8.0, 10.8 + 7.8 * beak_open), (2.0, 9.0 + 5.2 * beak_open)]
    _turn_poly(draw, upper, beak_cx, beak_cy, PAL["beak_upper"], width=0.7, radius=1.5, mirror=mirrored)
    _turn_fill(img, [(1.0, -1.5), (8.0, -2.7), (13.2, 0.0), (10.2, 5.4), (5.0, 5.0)], beak_cx, beak_cy, (*PAL["beak_upper_shadow"][:3], 185), radius=1.1, mirror=mirrored)
    _turn_poly(draw, lower, beak_cx, beak_cy, PAL["beak_lower"], width=0.65, radius=1.2, mirror=mirrored)
    nostril = _turn_pt((5.0, -2.3), beak_cx, beak_cy, mirror=mirrored)
    draw.ellipse((nostril[0] - SS * 0.45, nostril[1] - SS * 0.6, nostril[0] + SS * 0.45, nostril[1] + SS * 0.6), fill=PAL["outline"])
    return img


def _render_turn_front(params: Dict[str, float]) -> Image.Image:
    img = Image.new("RGBA", (FRAME_W * SS, FRAME_H * SS), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    cx = (CENTER_X + params.get("root_x", 0.0)) * SS
    airborne = params.get("airborne", 0.0) > 0.5
    default_root_y = -19.0 if airborne else -0.5
    cy = (GROUND_Y + params.get("root_y", default_root_y) - 21.0) * SS
    wing_src = params.get("near_wing_u", -24.0 if airborne else -10.0)
    wing_base = -24.0 if airborne else -10.0
    wing_span = 82.0 if airborne else 28.0
    wing_lift = clamp((wing_src - wing_base) / wing_span, 0.0, 1.0)
    beak_open = clamp(params.get("beak_open", 0.0), 0.0, 1.0)
    tail_fan = clamp(params.get("tail_fan", 0.8), 0.0, 1.0)

    shadow_w = SS * ((14.0 if airborne else 18.0) - 2.0 * wing_lift)
    shadow_h = SS * (3.1 if airborne else 4.2)
    sx, sy = (CENTER_X + params.get("root_x", 0.0)) * SS, (GROUND_Y + 0.5) * SS
    shadow = (*PAL["shadow"][:3], 42 if airborne else PAL["shadow"][3])
    draw.ellipse((sx - shadow_w, sy - shadow_h, sx + shadow_w, sy + shadow_h), fill=shadow)

    _draw_turn_front_wing(img, draw, cx, cy, wing_lift, airborne, mirror=True)
    _draw_turn_front_wing(img, draw, cx, cy, wing_lift, airborne, mirror=False)

    for dx, color in ((-5.0, PAL["body_dark"]), (0.0, PAL["body"]), (5.0, PAL["wing_blue"])):
        tail = [(dx - 2.0, 15.5), (dx, 24.0 + 4.0 * tail_fan), (dx + 2.0, 33.0), (dx - 4.0, 30.5), (dx - 5.0, 20.5)]
        _turn_poly(draw, tail, cx, cy, color, width=0.45, radius=1.0)

    body = [(-14.0, -14.5), (-4.0, -19.0), (4.0, -19.0), (14.0, -14.5), (16.5, -2.0), (14.0, 15.5), (6.5, 22.0), (-6.5, 22.0), (-14.0, 15.5), (-16.5, -2.0)]
    _turn_poly(draw, body, cx, cy, PAL["body"], width=0.8, radius=4.2)
    _turn_fill(img, [(-8.0, -8.0), (0.0, -10.0), (8.0, -8.0), (10.0, 8.0), (5.0, 18.0), (-5.0, 18.0), (-10.0, 8.0)], cx, cy, (*PAL["body_light"][:3], 95), radius=3.0)
    _turn_fill(img, [(-3.0, -14.0), (0.0, -15.0), (3.0, -14.0), (7.0, -10.0), (0.0, -5.0), (-7.0, -10.0)], cx, cy, (*PAL["wing_yellow"][:3], 140), radius=1.8)

    if airborne:
        for x in (-3.2, 3.2):
            shin = [(x - 0.6, 11.6), (x + 0.4, 11.6), (x + 0.8, 14.4), (x - 0.2, 14.5)]
            _turn_poly(draw, shin, cx, cy, PAL["leg"], width=0.32, radius=0.6)
            base = _turn_pt((x + 0.2, 14.2), cx, cy)
            for spread, length in ((-0.55, 2.2), (0.0, 2.8), (0.55, 2.2)):
                tx = base[0] + SS * (length if x > 0 else -length)
                ty = base[1] + SS * spread
                draw.line((base[0], base[1], tx, ty), fill=PAL["talon"], width=max(1, int(SS * 0.4)))
    else:
        for x in (-4.5, 4.5):
            shin = [(x - 0.7, 15.0), (x + 0.4, 15.0), (x + 0.9, 19.2), (x - 0.2, 19.3)]
            _turn_poly(draw, shin, cx, cy, PAL["leg"], width=0.35, radius=0.7)
            base = _turn_pt((x + 0.2, 19.0), cx, cy)
            for spread, length in ((-0.7, 2.6), (0.0, 3.2), (0.7, 2.6)):
                tx = base[0] + SS * (length if x > 0 else -length)
                ty = base[1] + SS * spread
                draw.line((base[0], base[1], tx, ty), fill=PAL["talon"], width=max(1, int(SS * 0.45)))

    head = [(-11.0, -26.0), (-3.0, -29.0), (3.0, -29.0), (11.0, -26.0), (14.0, -16.0), (12.0, -3.0), (6.0, 7.0), (-6.0, 7.0), (-12.0, -3.0), (-14.0, -16.0)]
    _turn_poly(draw, head, cx, cy - 2.0 * SS, PAL["head"], width=0.8, radius=4.0)
    _turn_fill(img, [(-7.5, -22.0), (-1.5, -24.0), (1.5, -24.0), (7.5, -22.0), (10.0, -9.0), (8.0, 0.5), (4.0, 4.0), (-4.0, 4.0), (-8.0, 0.5), (-10.0, -9.0)], cx, cy - 2.0 * SS, PAL["face_patch"], radius=3.0)
    _turn_fill(img, [(-5.5, -24.0), (0.0, -25.0), (5.5, -24.0), (7.5, -17.0), (0.0, -13.0), (-7.5, -17.0)], cx, cy - 2.0 * SS, (*PAL["head_light"][:3], 100), radius=2.0)
    for sign in (-1.0, 1.0):
        for y0, y1 in ((-14.5, -2.0), (-11.0, 1.0), (-7.0, 3.5)):
            p0 = _turn_pt((sign * 5.2, y0), cx, cy - 2.0 * SS)
            p1 = _turn_pt((sign * 4.2, y1), cx, cy - 2.0 * SS)
            draw.line((p0[0], p0[1], p1[0], p1[1]), fill=PAL["face_line"], width=max(1, int(SS * 0.26)))
    for sign in (-1.0, 1.0):
        ec = _turn_pt((sign * 6.0, -12.5), cx, cy - 2.0 * SS)
        ew, eh = SS * 4.4, SS * 5.0
        draw.ellipse((ec[0] - ew / 2, ec[1] - eh / 2, ec[0] + ew / 2, ec[1] + eh / 2), fill=PAL["eye"], outline=PAL["outline"], width=max(1, int(SS * 0.35)))
        pw, ph = SS * 1.4, SS * 2.0
        draw.ellipse((ec[0] - pw / 2 + sign * SS * 0.25, ec[1] - ph / 2, ec[0] + pw / 2 + sign * SS * 0.25, ec[1] + ph / 2), fill=PAL["pupil"])

    upper = [(-4.5, -3.0), (-1.5, -6.5), (1.5, -6.5), (4.5, -3.0), (3.4, 3.8), (0.0, 9.0), (-3.4, 3.8)]
    lower = [(-2.3, 4.2 + 2.0 * beak_open), (0.0, 7.0 + 5.5 * beak_open), (2.3, 4.2 + 2.0 * beak_open), (1.3, 11.0 + 6.5 * beak_open), (-1.3, 11.0 + 6.5 * beak_open)]
    _turn_poly(draw, upper, cx, cy - 6.5 * SS, PAL["beak_upper"], width=0.7, radius=1.2)
    _turn_fill(img, [(-2.4, -1.2), (0.0, -3.0), (2.4, -1.2), (1.6, 4.2), (0.0, 6.6), (-1.6, 4.2)], cx, cy - 6.5 * SS, (*PAL["beak_upper_shadow"][:3], 190), radius=1.0)
    _turn_poly(draw, lower, cx, cy - 6.5 * SS, PAL["beak_lower"], width=0.65, radius=1.0)
    for sign in (-1.0, 1.0):
        n = _turn_pt((sign * 1.7, -1.5), cx, cy - 6.5 * SS)
        draw.ellipse((n[0] - SS * 0.35, n[1] - SS * 0.45, n[0] + SS * 0.35, n[1] + SS * 0.45), fill=PAL["outline"])
    return img


def _render_turnaround_actor(frame_idx: int, nframes: int, world, params: Dict[str, float]) -> Image.Image:
    side_r = _render_side_actor(world, params, mirrored=False)
    side_l = _render_side_actor(world, params, mirrored=True)
    half_r = _render_turn_three_quarter(params, mirrored=False)
    half_l = _render_turn_three_quarter(params, mirrored=True)
    front = _render_turn_front(params)

    step = int(round(frame_idx * 8 / max(1, nframes - 1)))
    if step <= 0:
        return side_r
    if step <= 2:
        return half_r
    if step <= 5:
        return front
    if step <= 7:
        return half_l
    return side_l



def render_frame(animation: str, frame_idx: int, nframes: int) -> Image.Image:
    if animation in LOOPS:
        t = frame_idx / max(1, nframes)
    else:
        t = frame_idx / max(1, nframes - 1)
    img = Image.new("RGBA", (FRAME_W * SS, FRAME_H * SS), (0, 0, 0, 0))
    world, params = _solve(animation, t)
    if animation in {"turnaround", "turnaround_flight"}:
        actor = _render_turnaround_actor(frame_idx, nframes, world, params)
    else:
        actor = _render_side_actor(world, params, mirrored=params.get("turn_flip", 0.0) > 0.5)
    img.alpha_composite(actor)
    return img.resize((FRAME_W, FRAME_H), Image.Resampling.LANCZOS)


# ---- Target registration hooks ------------------------------------------------


def render(out_dir: Path, **opts) -> List[Path]:
    del opts
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=render_frame,
        out_dir=Path(out_dir),
        frame_size=(FRAME_W, FRAME_H),
        actor_metadata=ACTOR_METADATA,
    )
    keys = ("spritesheet", "yaml", "ron", "actor", "canonical", "canonical_transparent", "preview")
    return [Path(outputs[k]) for k in keys if outputs.get(k)]



def render_canonical(out_dir: Path, **opts) -> Path:
    del opts
    return write_canonical(TARGET_NAME, ROWS, render_frame, Path(out_dir))
