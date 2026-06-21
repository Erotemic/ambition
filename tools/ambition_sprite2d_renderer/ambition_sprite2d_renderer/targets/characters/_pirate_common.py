"""Drawing helpers shared by the pirate-family characters.

Pirate-specific paint code: palettes, body part draws (hat / face /
boot / sword / neck), the parametric `animation_pose` rig, and the
`draw_character(kind, anim, ...)` entry point that composes them.

Leading underscore is intentional — the target registry walks
``targets/characters/`` and treats files starting with ``_`` as
helpers, so this module won't try to register as a target.

The 5 core pirate character modules (admiral, raider, quartermaster,
lookout, navigator) re-use the same parametric rig keyed on their
palette + cohort tags (`SCARFED_KINDS`, `BEARDED_KINDS`,
`SKULL_MOTIF_KINDS`). Each `targets/characters/pirate_<role>.py`
module's `render()` calls `render_target(<role>, ...)` here, which
plumbs through to `tackon_sheet.build_sheet` with the right
`draw_character` partial.

A handful of non-pirate characters that happened to look pirate-ish
(colonial_statesman, viking variants, etc.) also re-use the
`draw_character` palette branches — see the per-kind switches inside
the body-draw helpers.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import (
    ANIMATIONS,
    BASE_FRAME,
    RGBA,
    SCALE,
    build_sheet,
    circle,
    downsample,
    ease_in_out,
    ellipse,
    font,
    lerp,
    line,
    oscillate,
    poly,
    rot,
    rotated_rect,
    rotated_rect_points,
    transform,
)


@dataclass
class Palette:
    outline: RGBA
    skin: RGBA
    skin_shadow: RGBA
    hat: RGBA
    coat: RGBA
    coat2: RGBA
    sash: RGBA
    shirt: RGBA
    pants: RGBA
    boots: RGBA
    metal: RGBA
    gold: RGBA
    beard: RGBA | None = None
    accent: RGBA | None = None


# Cohort tags so the parametric draw branches can ask "is this a
# scarf-wearing pirate" / "is this a bearded pirate" rather than
# spelling out every kind name in every branch. Lady pirates wear
# scarves and share the taunt-tilt + blade-tip-offset behavior with
# the male raiders, but they don't grow beards or wear the chest
# skull motif.
SCARFED_KINDS = (
    "pirate_raider",
    "pirate_quartermaster",
    "pirate_lookout",
    "pirate_navigator",
)
BEARDED_KINDS = ("pirate_raider", "pirate_quartermaster")
SKULL_MOTIF_KINDS = ("pirate_raider", "pirate_quartermaster")


PALETTES = {
    "pirate_admiral": Palette(
        outline=(26, 28, 35, 255),
        # Warm mid-brown skin. Reads as Caribbean-coast / mestizo
        # rather than European-pale; sits between the lighter raider
        # tone (#EBC4A0) and the deep brown quartermaster (#704C32) so
        # the cove lineup hits three distinct values at a glance.
        skin=(168, 124, 88, 255),
        skin_shadow=(112, 76, 48, 255),
        hat=(28, 31, 41, 255),
        coat=(88, 108, 138, 255),
        coat2=(146, 165, 191, 255),
        sash=(113, 40, 40, 255),
        shirt=(214, 205, 182, 255),
        pants=(212, 196, 160, 255),
        boots=(69, 50, 35, 255),
        metal=(210, 216, 228, 255),
        gold=(206, 171, 74, 255),
        beard=None,
        accent=(222, 72, 55, 255),
    ),
    "pirate_raider": Palette(
        outline=(28, 24, 26, 255),
        skin=(235, 196, 160, 255),
        skin_shadow=(175, 128, 95, 255),
        hat=(31, 23, 32, 255),
        coat=(196, 60, 52, 255),
        coat2=(229, 191, 105, 255),
        sash=(24, 24, 24, 255),
        shirt=(31, 31, 35, 255),
        pants=(66, 67, 73, 255),
        boots=(84, 53, 31, 255),
        metal=(201, 207, 214, 255),
        gold=(227, 184, 70, 255),
        beard=(77, 42, 23, 255),
        accent=(239, 239, 239, 255),
    ),
    # Third pirate variant — same silhouette family as `pirate_raider`
    # (broad cutlass-and-coat raider archetype) but a distinctly
    # darker skin tone so the lineup represents more of the actual
    # human phenotype range that historical Caribbean / Indian Ocean
    # / Mediterranean pirate crews drew from. The coat shifts from
    # raider's bright red to a deep teal so the silhouettes are
    # easy to tell apart at a glance even when palette-only
    # variants ship side-by-side.
    "pirate_quartermaster": Palette(
        outline=(18, 14, 16, 255),
        # Deep brown skin — noticeably darker than the existing
        # `pirate_admiral` (#A87C58) and `pirate_raider` (#EBC4A0).
        skin=(112, 76, 50, 255),
        skin_shadow=(72, 46, 28, 255),
        hat=(20, 24, 30, 255),
        coat=(28, 92, 92, 255),  # deep teal
        coat2=(206, 178, 92, 255),  # warm gold trim
        sash=(160, 38, 38, 255),  # bright crimson sash for contrast
        shirt=(238, 226, 198, 255),
        pants=(46, 42, 38, 255),
        boots=(54, 36, 22, 255),
        metal=(212, 218, 224, 255),
        gold=(228, 188, 76, 255),
        # Short cropped beard matching the warm-dark skin tone.
        beard=(38, 24, 16, 255),
        accent=(232, 220, 196, 255),
    ),
    # ─────────────────────────────────────────────────────────────────
    # Lady pirate variants. Same skeleton + hat / sash / sword
    # geometry as the male roles (the parametric character draws the
    # same silhouette), but no beard and a warmer scarf / coat
    # palette so they read as a distinct crew at a glance.
    #
    # Two skin tones — Lookout deep-brown (matches Quartermaster
    # range), Navigator pale-warm — so the lineup hits five distinct
    # phenotypes between the three men + two women.
    # ─────────────────────────────────────────────────────────────────
    "pirate_lookout": Palette(
        outline=(22, 18, 22, 255),
        # Deep brown skin in the Quartermaster range, slightly warmer.
        skin=(118, 80, 56, 255),
        skin_shadow=(76, 48, 32, 255),
        hat=(24, 28, 38, 255),  # dark navy cap
        coat=(176, 60, 88, 255),  # raspberry coat
        coat2=(232, 200, 132, 255),  # buttery trim
        sash=(48, 36, 28, 255),  # dark leather sash
        shirt=(244, 232, 208, 255),
        pants=(52, 38, 32, 255),
        boots=(60, 38, 22, 255),
        metal=(214, 220, 226, 255),
        gold=(228, 188, 76, 255),
        beard=None,  # no beard — lady pirate
        accent=(255, 230, 196, 255),
    ),
    "pirate_navigator": Palette(
        outline=(24, 20, 24, 255),
        # Pale-warm skin, between the existing Raider and a paler
        # northern-European reference. Distinct from Admiral's mid-
        # brown so the cove lineup reads as five different people.
        skin=(238, 206, 178, 255),
        skin_shadow=(196, 152, 116, 255),
        hat=(72, 24, 48, 255),  # plum hat
        coat=(46, 64, 96, 255),  # midnight navy coat
        coat2=(214, 198, 174, 255),  # bone trim
        sash=(212, 168, 64, 255),  # gold sash
        shirt=(244, 240, 232, 255),
        pants=(58, 50, 48, 255),
        boots=(64, 44, 28, 255),
        metal=(216, 222, 230, 255),
        gold=(232, 192, 80, 255),
        beard=None,  # no beard — lady pirate
        accent=(178, 116, 156, 255),
    ),
}


def animation_pose(anim, frame_idx, nframes):
    s = oscillate(frame_idx, nframes)
    c = math.cos((frame_idx / max(1, nframes)) * math.tau)
    t = frame_idx / max(1, nframes - 1)
    pose = {
        "root_x": 0.0,
        "bob": 0.0,
        "body_tilt": 0.0,
        "left_leg": -6.0,
        "right_leg": 6.0,
        "left_arm": 10.0,
        "right_arm": -20.0,
        "weapon": -18.0,
        "head_tilt": -4.0,
        "head_y": 0.0,
        "hat_tilt": 0.0,
        "left_foot_lift": 0.0,
        "right_foot_lift": 0.0,
        "coat_sway": 0.0,
        "shoulder_bounce": 0.0,
        "blink": False,
        "mouth_open": 0.0,
        "death_t": 0.0,
        "x_eyes": False,
    }
    if anim == "idle":
        pose["root_x"] = s * 1.8
        pose["bob"] = s * 4.0
        pose["body_tilt"] = s * 3.0
        pose["left_leg"] = -8.0 + c * 2.0
        pose["right_leg"] = 8.0 - c * 2.0
        pose["left_arm"] = 12.0 + s * 7.0
        pose["right_arm"] = -16.0 - s * 10.0
        pose["weapon"] = -16.0 - s * 8.0
        pose["head_tilt"] = -5.0 + s * 4.0
        pose["head_y"] = -abs(s) * 1.2
        pose["hat_tilt"] = s * 3.5
        pose["coat_sway"] = s * 10.0
        pose["shoulder_bounce"] = -abs(s) * 2.0
        pose["mouth_open"] = max(0.0, s) * 0.15
        pose["blink"] = frame_idx == max(0, nframes - 2)
    elif anim == "walk":
        pose["root_x"] = s * 2.5
        pose["bob"] = abs(s) * 6.0 - 1.5
        pose["body_tilt"] = s * 5.0
        # Dampened from ±28° to ±11.2°. With the anatomical knee
        # offsets above, the full ±28° amplitude swept the upper leg
        # past the body centerline at f2/8 and f6/8 — the pants would
        # visibly cross. ±11° keeps the leg swing visible while
        # leaving the feet on their own sides.
        pose["left_leg"] = -11.2 * s
        pose["right_leg"] = 11.2 * s
        pose["left_arm"] = 22.0 * s + 4.0
        pose["right_arm"] = -40.0 * s - 4.0
        pose["weapon"] = -24.0 - 24.0 * s
        pose["head_tilt"] = -4.0 + s * 3.0
        pose["head_y"] = -abs(c) * 1.0
        pose["hat_tilt"] = s * 4.0
        pose["left_foot_lift"] = max(0.0, -s) * 12.0
        pose["right_foot_lift"] = max(0.0, s) * 12.0
        pose["coat_sway"] = -s * 16.0
        pose["shoulder_bounce"] = abs(s) * 2.5
    elif anim == "slash":
        tt = ease_in_out(t)
        attack = math.sin(tt * math.pi)
        pose["root_x"] = -8.0 + 18.0 * tt
        pose["bob"] = -attack * 5.5
        pose["body_tilt"] = -18.0 + 38.0 * tt
        pose["left_leg"] = -10.0 - 6.0 * attack
        pose["right_leg"] = 14.0 + 10.0 * attack
        pose["left_arm"] = -6.0 - 34.0 * attack
        pose["right_arm"] = 72.0 - 155.0 * tt
        pose["weapon"] = 115.0 - 230.0 * tt
        pose["head_tilt"] = -14.0 + 16.0 * tt
        pose["hat_tilt"] = -4.0 + 10.0 * tt
        pose["coat_sway"] = 18.0 - 36.0 * tt
        pose["shoulder_bounce"] = attack * 3.5
        pose["mouth_open"] = attack * 0.35
    elif anim == "taunt":
        pose["root_x"] = s * 2.0
        pose["bob"] = s * 3.0
        pose["body_tilt"] = -8.0 + s * 4.0
        pose["left_leg"] = -10.0
        pose["right_leg"] = 12.0
        pose["left_arm"] = -62.0 + 14.0 * s
        pose["right_arm"] = 8.0 + 26.0 * s
        pose["weapon"] = -108.0 + 20.0 * s
        pose["head_tilt"] = -10.0 + s * 5.0
        pose["hat_tilt"] = -2.0 + s * 4.0
        pose["coat_sway"] = s * 8.0
        pose["shoulder_bounce"] = -s * 2.0
        pose["mouth_open"] = 0.30 + max(0.0, s) * 0.2
    elif anim == "hurt":
        phase = math.sin(t * math.pi)
        shake = math.sin(t * math.pi * 5.0) * (1.0 - t)
        pose["root_x"] = shake * 6.0
        pose["bob"] = -phase * 4.0
        pose["body_tilt"] = -18.0 * phase
        pose["left_leg"] = -6.0 + 10.0 * phase
        pose["right_leg"] = 6.0 - 8.0 * phase
        pose["left_arm"] = 28.0 * phase
        pose["right_arm"] = -6.0 + 28.0 * phase
        pose["weapon"] = -48.0 + 24.0 * phase
        pose["head_tilt"] = 14.0 * phase
        pose["hat_tilt"] = -10.0 * phase
        pose["coat_sway"] = -12.0 * phase
        pose["mouth_open"] = 0.4 * phase
    elif anim == "death":
        tt = ease_in_out(t)
        pose["death_t"] = tt
        pose["root_x"] = tt * 10.0
        pose["bob"] = -tt * 10.0
        pose["body_tilt"] = -65.0 * tt
        pose["left_leg"] = lerp(-6.0, 30.0, tt)
        pose["right_leg"] = lerp(6.0, -25.0, tt)
        pose["left_arm"] = lerp(10.0, 70.0, tt)
        pose["right_arm"] = lerp(-20.0, -80.0, tt)
        pose["weapon"] = lerp(-18.0, -120.0, tt)
        pose["head_tilt"] = lerp(-4.0, 25.0, tt)
        pose["hat_tilt"] = -12.0 * tt
        pose["coat_sway"] = 16.0 * tt
        pose["mouth_open"] = 0.45 * tt
        pose["x_eyes"] = tt > 0.55
    return pose


def draw_boot(draw, center, w, h, angle, pal, foot_forward=1):
    pts = rotated_rect_points(center, w, h * 0.58, angle)
    poly(draw, pts, pal.boots, pal.outline, width=3)
    toe = [
        transform((w * 0.28, -h * 0.10), center, angle),
        transform((w * 0.50, -h * 0.05), center, angle),
        transform((w * 0.50, h * 0.16), center, angle),
        transform((w * 0.15, h * 0.22), center, angle),
    ]
    poly(draw, toe, pal.boots, pal.outline, width=3)


def draw_sword(draw, hand, angle, length, pal, curve=0.0):
    guard = rotated_rect_points(transform((0, 2), hand, angle), 18, 6, angle)
    poly(draw, guard, pal.gold, pal.outline, width=3)
    grip = rotated_rect_points(transform((-6, 1), hand, angle), 10, 5, angle)
    poly(draw, grip, (68, 43, 27, 255), pal.outline, width=3)
    p0 = transform((6, 0), hand, angle)
    p1 = transform((length * 0.35, curve * 0.12), hand, angle)
    p2 = transform((length, curve), hand, angle)
    line(draw, [p0, p1, p2], pal.metal, width=5)
    line(draw, [p0, p1, p2], pal.outline, width=1)


def draw_human_neck(draw, chest, head_center, global_tilt, pal, kind="pirate_admiral"):
    """Draw a simple human neck with a collar, instead of the mockingbird-style spine."""
    # Base of neck emerges from the shirt / coat opening.
    base = transform((0, -22), chest, deg=global_tilt)
    top = (head_center[0] - 2, head_center[1] + 24)
    neck_fill = pal.skin if kind in SCARFED_KINDS else pal.skin_shadow

    # Slightly tapered neck polygon.
    pts = [
        transform((-9, 0), base, deg=global_tilt),
        transform((8, 0), base, deg=global_tilt),
        (top[0] + 7, top[1]),
        (top[0] - 7, top[1]),
    ]
    poly(draw, pts, neck_fill, pal.outline, width=3)

    # Throat / shading line.
    line(
        draw,
        [
            ((pts[0][0] + pts[1][0]) / 2 + 1, (pts[0][1] + pts[1][1]) / 2),
            (top[0] + 1, top[1] - 2),
        ],
        pal.skin_shadow,
        width=2,
    )

    # Shirt collar / cravat for a more human look.
    collar_left = [
        transform((-16, -18), chest, deg=global_tilt),
        transform((-2, -12), chest, deg=global_tilt),
        transform((-9, 2), chest, deg=global_tilt),
        transform((-18, -4), chest, deg=global_tilt),
    ]
    collar_right = [
        transform((2, -12), chest, deg=global_tilt),
        transform((16, -18), chest, deg=global_tilt),
        transform((18, -4), chest, deg=global_tilt),
        transform((9, 2), chest, deg=global_tilt),
    ]
    poly(draw, collar_left, pal.shirt, pal.outline, width=2)
    poly(draw, collar_right, pal.shirt, pal.outline, width=2)

    if kind == "pirate_admiral":
        knot = rotated_rect_points(
            transform((0, -2), chest, deg=global_tilt), 10, 8, global_tilt
        )
        tail_l = [
            transform(p, chest, deg=global_tilt)
            for p in [(-2, 2), (-10, 16), (-3, 18), (1, 8)]
        ]
        tail_r = [
            transform(p, chest, deg=global_tilt)
            for p in [(2, 2), (10, 16), (4, 18), (-1, 8)]
        ]
        poly(draw, knot, pal.sash, pal.outline, width=2)
        poly(draw, tail_l, pal.sash, pal.outline, width=2)
        poly(draw, tail_r, pal.sash, pal.outline, width=2)
    else:
        scarf = [
            transform(p, chest, deg=global_tilt)
            for p in [(-7, -4), (8, -4), (5, 10), (-9, 8)]
        ]
        poly(draw, scarf, pal.accent or pal.coat2, pal.outline, width=2)


def draw_hat(draw, head_center, hat_scale, pal, skull=False, tilt=0.0):
    hx, hy = head_center
    brim_local = [(-44, -38), (-16, -48), (18, -46), (44, -36), (16, -30), (-20, -31)]
    crown_local = [(-18, -34), (-8, -62), (8, -63), (18, -34)]
    brim = [
        transform((x * hat_scale, y * hat_scale), head_center, deg=tilt)
        for x, y in brim_local
    ]
    crown = [
        transform((x * hat_scale, y * hat_scale), head_center, deg=tilt)
        for x, y in crown_local
    ]
    poly(draw, brim, pal.hat, pal.outline, width=4)
    poly(draw, crown, pal.hat, pal.outline, width=4)
    if skull:
        skull_c = transform((6 * hat_scale, -46 * hat_scale), head_center, deg=tilt)
        circle(draw, skull_c, 6 * hat_scale, (232, 230, 226, 255), pal.outline, width=2)
        l1 = transform((2 * hat_scale, -42 * hat_scale), head_center, deg=tilt)
        l2 = transform((10 * hat_scale, -42 * hat_scale), head_center, deg=tilt)
        l3 = transform((6 * hat_scale, -40 * hat_scale), head_center, deg=tilt)
        l4 = transform((6 * hat_scale, -37 * hat_scale), head_center, deg=tilt)
        line(draw, [l1, l2], pal.outline, width=2)
        line(draw, [l3, l4], pal.outline, width=2)


def draw_face(
    draw,
    head_bbox,
    pal,
    eyepatch=False,
    beard=False,
    mean=False,
    x_eyes=False,
    blink=False,
    mouth_open=0.0,
):
    x1, y1, x2, y2 = head_bbox
    ellipse(draw, head_bbox, pal.skin, pal.outline, width=4)
    nose = [
        (lerp(x1, x2, 0.53), lerp(y1, y2, 0.42)),
        (lerp(x1, x2, 0.60), lerp(y1, y2, 0.56)),
        (lerp(x1, x2, 0.52), lerp(y1, y2, 0.60)),
    ]
    line(draw, nose, pal.skin_shadow, width=3)
    brow_y = lerp(y1, y2, 0.34)
    eye_y = lerp(y1, y2, 0.43)
    if x_eyes:
        for ex in [lerp(x1, x2, 0.38), lerp(x1, x2, 0.61)]:
            line(
                draw,
                [(ex - 6, eye_y - 6), (ex + 6, eye_y + 6)],
                pal.accent or (255, 255, 255, 255),
                width=3,
            )
            line(
                draw,
                [(ex - 6, eye_y + 6), (ex + 6, eye_y - 6)],
                pal.accent or (255, 255, 255, 255),
                width=3,
            )
    else:
        left_brow = [
            (lerp(x1, x2, 0.28), brow_y + (3 if mean else 0)),
            (lerp(x1, x2, 0.43), brow_y - (4 if mean else 1)),
        ]
        right_brow = [
            (lerp(x1, x2, 0.56), brow_y - (4 if mean else 1)),
            (lerp(x1, x2, 0.72), brow_y + (3 if mean else 0)),
        ]
        line(draw, left_brow, pal.outline, width=4)
        line(draw, right_brow, pal.outline, width=4)
        if eyepatch:
            patch_box = (lerp(x1, x2, 0.27), eye_y - 7, lerp(x1, x2, 0.47), eye_y + 7)
            ellipse(draw, patch_box, pal.hat, pal.outline, width=2)
            line(draw, [(x1 + 8, eye_y - 10), (x2 - 4, eye_y - 4)], pal.hat, width=3)
        else:
            if blink:
                line(
                    draw,
                    [(lerp(x1, x2, 0.31), eye_y), (lerp(x1, x2, 0.40), eye_y + 1)],
                    pal.outline,
                    width=3,
                )
            else:
                ellipse(
                    draw,
                    (lerp(x1, x2, 0.31), eye_y - 4, lerp(x1, x2, 0.40), eye_y + 4),
                    (255, 255, 255, 255),
                    pal.outline,
                    width=2,
                )
                circle(draw, (lerp(x1, x2, 0.36), eye_y), 2, pal.outline)
        if blink:
            line(
                draw,
                [(lerp(x1, x2, 0.58), eye_y), (lerp(x1, x2, 0.67), eye_y + 1)],
                pal.outline,
                width=3,
            )
        else:
            ellipse(
                draw,
                (lerp(x1, x2, 0.58), eye_y - 4, lerp(x1, x2, 0.67), eye_y + 4),
                (255, 255, 255, 255),
                pal.outline,
                width=2,
            )
            circle(draw, (lerp(x1, x2, 0.62), eye_y), 2, pal.outline)
    mouth_mid = lerp(y1, y2, 0.80) + mouth_open * 10.0
    mouth = [
        (lerp(x1, x2, 0.38), lerp(y1, y2, 0.76)),
        (lerp(x1, x2, 0.51), mouth_mid),
        (lerp(x1, x2, 0.67), lerp(y1, y2, 0.73)),
    ]
    line(draw, mouth, pal.outline, width=3)
    if beard and pal.beard:
        beard_pts = [
            (lerp(x1, x2, 0.23), lerp(y1, y2, 0.63)),
            (lerp(x1, x2, 0.50), lerp(y1, y2, 0.94)),
            (lerp(x1, x2, 0.80), lerp(y1, y2, 0.62)),
            (lerp(x1, x2, 0.69), lerp(y1, y2, 0.86)),
            (lerp(x1, x2, 0.38), lerp(y1, y2, 0.86)),
        ]
        poly(draw, beard_pts, pal.beard, pal.outline, width=3)


def draw_character(
    kind: str, anim: str, frame_idx: int, nframes: int, frame_size=BASE_FRAME
) -> Image.Image:
    pal = PALETTES[kind]
    w, h = frame_size[0] * SCALE, frame_size[1] * SCALE
    img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img, "RGBA")
    pose = animation_pose(anim, frame_idx, nframes)

    cx = w * (0.48 if kind == "pirate_admiral" else 0.50)
    ground = h * 0.83
    bob = pose["bob"] * SCALE
    root = (cx, ground + bob)
    global_tilt = pose["body_tilt"] + (
        5 if kind in SCARFED_KINDS and anim == "taunt" else 0
    )
    death_t = pose["death_t"]

    # Whole body offsets / lean for death.
    char_origin = (
        root[0] + pose["root_x"] * SCALE + death_t * 12 * SCALE,
        root[1] + death_t * 5 * SCALE,
    )

    # No baked drop shadow. A shadow ellipse below the feet extends the
    # auto-crop bbox downward, which moves the cropped frame's "bottom"
    # off the feet — every pirate ends up floating above their collision
    # AABB by the shadow's height. Cast shadows that need to track
    # gameplay state belong on the ECS visual layer, not the source PNG.
    # See agent memory: [[feedback-no-drop-shadows-on-sprites]].

    # Local joints.
    hip = transform((0, -60), char_origin, deg=global_tilt)
    chest = transform((0, -124 + pose["shoulder_bounce"]), char_origin, deg=global_tilt)
    head_center = transform(
        (8, -202 + pose["head_y"]), char_origin, deg=global_tilt + pose["head_tilt"]
    )

    # Back arm / weapon arm first.
    if kind == "pirate_admiral":
        back_shoulder = transform((20, -136), char_origin, deg=global_tilt)
        front_shoulder = transform((-22, -136), char_origin, deg=global_tilt)
    else:
        back_shoulder = transform((24, -136), char_origin, deg=global_tilt)
        front_shoulder = transform((-26, -136), char_origin, deg=global_tilt)

    # Legs. Upper-leg offsets put the knee mostly DOWN from the hip
    # with only a small outward bias, so the pants render as columns
    # instead of an inverted-V splay. Lower-leg offsets keep the same
    # ~30 px drop, giving a roughly even thigh/shin split for the
    # body's overall height. Previously the offsets were (-18, 4) and
    # (12, 4) — near-horizontal "thighs" that produced the bow-legged
    # silhouette and let leg-angle rotations sweep the upper leg
    # across the body centerline.
    left_hip = transform((-16, -56), char_origin, deg=global_tilt)
    right_hip = transform((18, -56), char_origin, deg=global_tilt)
    left_knee = transform((-4, 30), left_hip, deg=pose["left_leg"])
    right_knee = transform((4, 30), right_hip, deg=pose["right_leg"])
    left_foot = transform(
        (-8, 30 - pose["left_foot_lift"]), left_knee, deg=pose["left_leg"] * 0.3
    )
    right_foot = transform(
        (8, 30 - pose["right_foot_lift"]), right_knee, deg=pose["right_leg"] * 0.3
    )

    for hip_pt, knee_pt, foot_pt, ang in [
        (left_hip, left_knee, left_foot, pose["left_leg"]),
        (right_hip, right_knee, right_foot, pose["right_leg"]),
    ]:
        line(draw, [hip_pt, knee_pt, foot_pt], pal.pants, width=13)
        line(draw, [hip_pt, knee_pt, foot_pt], pal.outline, width=4)
        draw_boot(draw, foot_pt, 24, 18, ang * 0.2, pal)

    # Body / shirt / coat
    torso_pts = [
        transform(p, chest, deg=global_tilt)
        for p in [(-34, -8), (30, -8), (42, 58), (0, 76), (-44, 58)]
    ]
    poly(draw, torso_pts, pal.coat, pal.outline, width=5)
    shirt_pts = [
        transform(p, chest, deg=global_tilt)
        for p in [(-10, -4), (18, -4), (14, 52), (-16, 52)]
    ]
    poly(draw, shirt_pts, pal.shirt, pal.outline, width=4)
    lapel_left = [
        transform(p, chest, deg=global_tilt)
        for p in [(-16, -6), (-2, 16), (-10, 44), (-20, 18)]
    ]
    lapel_right = [
        transform(p, chest, deg=global_tilt)
        for p in [(8, -6), (20, 16), (16, 42), (4, 18)]
    ]
    poly(draw, lapel_left, pal.coat2, pal.outline, width=3)
    poly(draw, lapel_right, pal.coat2, pal.outline, width=3)
    sash_box = rotated_rect_points(
        transform((0, 24), chest, deg=global_tilt), 44, 12, global_tilt
    )
    poly(draw, sash_box, pal.sash, pal.outline, width=3)
    for bx in [-10, 0, 10]:
        circle(
            draw,
            transform((bx, 4), chest, deg=global_tilt),
            3,
            pal.gold,
            pal.outline,
            width=1,
        )

    coat_sway = pose["coat_sway"]
    tail_left = [
        transform(p, hip, deg=global_tilt + coat_sway)
        for p in [(-36, 0), (-8, -2), (-8, 48), (-30, 58)]
    ]
    tail_right = [
        transform(p, hip, deg=global_tilt - coat_sway)
        for p in [(8, -2), (34, 0), (28, 58), (6, 48)]
    ]
    poly(draw, tail_left, pal.coat, pal.outline, width=4)
    poly(draw, tail_right, pal.coat, pal.outline, width=4)

    # Back arm
    back_elbow = transform((4, 52), back_shoulder, deg=pose["left_arm"])
    back_hand = transform((0, 48), back_elbow, deg=pose["left_arm"] * 0.55)
    line(draw, [back_shoulder, back_elbow, back_hand], pal.coat, width=12)
    line(draw, [back_shoulder, back_elbow, back_hand], pal.outline, width=4)
    circle(draw, back_hand, 7, pal.skin, pal.outline, width=2)
    if anim == "taunt":
        line(
            draw,
            [transform((0, -10), back_hand), transform((10, -22), back_hand)],
            pal.outline,
            width=3,
        )

    # Front arm / weapon
    front_elbow = transform((6, 50), front_shoulder, deg=pose["right_arm"])
    front_hand = transform((0, 46), front_elbow, deg=pose["weapon"] * 0.35)
    line(draw, [front_shoulder, front_elbow, front_hand], pal.coat, width=13)
    line(draw, [front_shoulder, front_elbow, front_hand], pal.outline, width=4)
    circle(draw, front_hand, 8, pal.skin, pal.outline, width=2)
    draw_sword(
        draw,
        front_hand,
        pose["weapon"],
        92 if kind == "pirate_admiral" else 86,
        pal,
        curve=(16 if kind in SCARFED_KINDS else 5),
    )
    if anim == "slash":
        arc_box = (
            front_hand[0] - 70,
            front_hand[1] - 96,
            front_hand[0] + 110,
            front_hand[1] + 76,
        )
        draw.arc(arc_box, start=205, end=336, fill=(255, 245, 200, 180), width=8)
        draw.arc(arc_box, start=214, end=328, fill=(255, 255, 255, 120), width=4)
    elif anim in {"idle", "walk", "taunt"} and frame_idx % 2 == 0:
        blade_tip = transform(
            (92 if kind == "pirate_admiral" else 86, 4 if kind in SCARFED_KINDS else 0),
            front_hand,
            pose["weapon"],
        )
        line(
            draw,
            [blade_tip, (blade_tip[0] + 10, blade_tip[1] - 8)],
            (255, 255, 255, 100),
            width=2,
        )

    # Neck / head / hat
    draw_human_neck(draw, chest, head_center, global_tilt, pal, kind=kind)

    head_bbox = (
        head_center[0] - 28,
        head_center[1] - 34,
        head_center[0] + 28,
        head_center[1] + 34,
    )
    draw_face(
        draw,
        head_bbox,
        pal,
        eyepatch=(kind == "pirate_admiral"),
        beard=(kind in BEARDED_KINDS),
        mean=True,
        x_eyes=pose["x_eyes"],
        blink=pose["blink"],
        mouth_open=pose["mouth_open"],
    )
    draw_hat(
        draw,
        head_center,
        1.0,
        pal,
        skull=True,
        tilt=pose["hat_tilt"] + global_tilt * 0.15,
    )

    if kind in SKULL_MOTIF_KINDS:
        # chest skull motif
        chest_c = transform((0, 14), chest, deg=global_tilt)
        circle(
            draw,
            (chest_c[0], chest_c[1] - 4),
            8,
            (242, 236, 230, 255),
            pal.outline,
            width=2,
        )
        line(
            draw,
            [(chest_c[0] - 7, chest_c[1] + 5), (chest_c[0] + 7, chest_c[1] + 5)],
            (242, 236, 230, 255),
            width=3,
        )
        line(
            draw,
            [(chest_c[0], chest_c[1] + 1), (chest_c[0], chest_c[1] + 9)],
            pal.outline,
            width=2,
        )

    # Death settle pose, ground line accent
    if anim == "death":
        draw.line((0, ground + 24, w, ground + 24), fill=(0, 0, 0, 0), width=1)

    return downsample(img, frame_size)


def render_target(
    target: str, out_dir: Path, frame_size: Tuple[int, int] = BASE_FRAME
) -> Dict[str, Path]:
    """Build a pirate-family character sheet via the parametric rig.

    Thin shim used by the 5 pirate character modules so their per-target
    ``render()`` is one line. Delegates to `tackon_sheet.build_sheet`
    with `draw_character(target, ...)` as the per-frame renderer.

    Returns the dict that ``build_sheet`` produced — callers flatten it
    into the ``list[Path]`` shape that the tack-on discovery API
    expects.
    """
    out_dir.mkdir(parents=True, exist_ok=True)
    return build_sheet(
        target=target,
        rows=ANIMATIONS,
        render_fn=lambda anim, frame_idx, nframes: draw_character(
            target,
            anim,
            frame_idx,
            nframes,
            frame_size=frame_size,
        ),
        out_dir=out_dir,
        frame_size=frame_size,
    )
