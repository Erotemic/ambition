from __future__ import annotations

import math
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Tuple

import yaml
from PIL import Image, ImageDraw, ImageFont

RGBA = Tuple[int, int, int, int]

SCALE = 4
BASE_FRAME = (128, 128)
LABEL_WIDTH = 100

ANIMATIONS = [
    ("idle", 6, 120),
    ("walk", 8, 90),
    ("slash", 6, 85),
    ("taunt", 6, 100),
    ("hurt", 4, 90),
    ("death", 8, 110),
]


def font(size: int = 14):
    try:
        return ImageFont.truetype("DejaVuSans.ttf", size)
    except Exception:
        return ImageFont.load_default()


def lerp(a, b, t):
    return a + (b - a) * t


def ease_in_out(t):
    return 0.5 - 0.5 * math.cos(math.pi * max(0.0, min(1.0, t)))


def oscillate(frame_idx: int, nframes: int, phase: float = 0.0) -> float:
    return math.sin((frame_idx / max(1, nframes)) * math.tau + phase)


def rot(pt, deg):
    rad = math.radians(deg)
    c = math.cos(rad)
    s = math.sin(rad)
    x, y = pt
    return (x * c - y * s, x * s + y * c)


def transform(pt, origin, deg=0.0, scale=1.0):
    x, y = pt
    x *= scale
    y *= scale
    x, y = rot((x, y), deg)
    return (origin[0] + x, origin[1] + y)


def poly(draw: ImageDraw.ImageDraw, points, fill, outline=None, width=1):
    draw.polygon(points, fill=fill)
    if outline is not None:
        draw.line(points + [points[0]], fill=outline, width=width, joint="curve")


def rotated_rect_points(center, w, h, deg):
    hw, hh = w / 2.0, h / 2.0
    pts = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
    return [transform(p, center, deg=deg) for p in pts]


def rotated_rect(draw, center, w, h, deg, fill, outline=None, width=1):
    pts = rotated_rect_points(center, w, h, deg)
    poly(draw, pts, fill, outline, width)
    return pts


def circle(draw, center, r, fill, outline=None, width=1):
    x, y = center
    draw.ellipse((x - r, y - r, x + r, y + r), fill=fill, outline=outline, width=width)


def ellipse(draw, bbox, fill, outline=None, width=1):
    draw.ellipse(bbox, fill=fill, outline=outline, width=width)


def line(draw, points, fill, width=1):
    draw.line(points, fill=fill, width=width, joint="curve")


def downsample(img: Image.Image, final_size=BASE_FRAME):
    alpha = img.getchannel("A")
    bbox = alpha.getbbox()
    if bbox is None:
        return img.resize(final_size, Image.Resampling.LANCZOS)
    x1, y1, x2, y2 = bbox
    crop = img.crop((x1, y1, x2, y2))
    fw, fh = final_size
    target_w = fw * 0.78
    target_h = fh * 0.88
    scale = min(target_w / max(1, crop.width), target_h / max(1, crop.height))
    new_size = (max(1, int(crop.width * scale)), max(1, int(crop.height * scale)))
    crop = crop.resize(new_size, Image.Resampling.LANCZOS)
    canvas = Image.new("RGBA", final_size, (0, 0, 0, 0))
    ox = int((fw - new_size[0]) / 2)
    oy = int(fh - new_size[1] - fh * 0.12)
    canvas.alpha_composite(crop, (ox, oy))
    return canvas


def alpha_bbox_metrics(frame: Image.Image):
    alpha = frame.getchannel("A")
    bbox = alpha.getbbox()
    if bbox is None:
        return {
            "body_pixel_bbox": {"x": 0, "y": 0, "w": 0, "h": 0},
            "feet_pixel": {"x": frame.width / 2.0, "y": frame.height},
            "feet_anchor_norm": {"x": 0.0, "y": -0.5},
        }
    x1, y1, x2, y2 = bbox
    feet_x = (x1 + x2) / 2.0
    feet_y = float(y2)
    return {
        "body_pixel_bbox": {"x": int(x1), "y": int(y1), "w": int(x2 - x1), "h": int(y2 - y1)},
        "feet_pixel": {"x": round(feet_x, 3), "y": round(feet_y, 3)},
        "feet_anchor_norm": {
            "x": round(feet_x / frame.width - 0.5, 6),
            "y": round(0.5 - feet_y / frame.height, 6),
        },
    }


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


PALETTES = {
    "pirate_admiral": Palette(
        outline=(26, 28, 35, 255),
        skin=(212, 188, 160, 255),
        skin_shadow=(166, 138, 116, 255),
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
    "pirate_corsair": Palette(
        outline=(18, 14, 16, 255),
        # Deep brown skin — noticeably darker than the existing
        # `pirate_admiral` (#D4BCA0) and `pirate_raider` (#EBC4A0).
        skin=(112, 76, 50, 255),
        skin_shadow=(72, 46, 28, 255),
        hat=(20, 24, 30, 255),
        coat=(28, 92, 92, 255),       # deep teal
        coat2=(206, 178, 92, 255),    # warm gold trim
        sash=(160, 38, 38, 255),      # bright crimson sash for contrast
        shirt=(238, 226, 198, 255),
        pants=(46, 42, 38, 255),
        boots=(54, 36, 22, 255),
        metal=(212, 218, 224, 255),
        gold=(228, 188, 76, 255),
        # Short cropped beard matching the warm-dark skin tone.
        beard=(38, 24, 16, 255),
        accent=(232, 220, 196, 255),
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
        pose["left_leg"] = -28.0 * s
        pose["right_leg"] = 28.0 * s
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
    neck_fill = pal.skin if kind in ("pirate_raider", "pirate_corsair") else pal.skin_shadow

    # Slightly tapered neck polygon.
    pts = [
        transform((-9, 0), base, deg=global_tilt),
        transform((8, 0), base, deg=global_tilt),
        (top[0] + 7, top[1]),
        (top[0] - 7, top[1]),
    ]
    poly(draw, pts, neck_fill, pal.outline, width=3)

    # Throat / shading line.
    line(draw, [((pts[0][0] + pts[1][0]) / 2 + 1, (pts[0][1] + pts[1][1]) / 2), (top[0] + 1, top[1] - 2)], pal.skin_shadow, width=2)

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
        knot = rotated_rect_points(transform((0, -2), chest, deg=global_tilt), 10, 8, global_tilt)
        tail_l = [transform(p, chest, deg=global_tilt) for p in [(-2, 2), (-10, 16), (-3, 18), (1, 8)]]
        tail_r = [transform(p, chest, deg=global_tilt) for p in [(2, 2), (10, 16), (4, 18), (-1, 8)]]
        poly(draw, knot, pal.sash, pal.outline, width=2)
        poly(draw, tail_l, pal.sash, pal.outline, width=2)
        poly(draw, tail_r, pal.sash, pal.outline, width=2)
    else:
        scarf = [transform(p, chest, deg=global_tilt) for p in [(-7, -4), (8, -4), (5, 10), (-9, 8)]]
        poly(draw, scarf, pal.accent or pal.coat2, pal.outline, width=2)

def draw_hat(draw, head_center, hat_scale, pal, skull=False, tilt=0.0):
    hx, hy = head_center
    brim_local = [(-44, -38), (-16, -48), (18, -46), (44, -36), (16, -30), (-20, -31)]
    crown_local = [(-18, -34), (-8, -62), (8, -63), (18, -34)]
    brim = [transform((x * hat_scale, y * hat_scale), head_center, deg=tilt) for x, y in brim_local]
    crown = [transform((x * hat_scale, y * hat_scale), head_center, deg=tilt) for x, y in crown_local]
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


def draw_face(draw, head_bbox, pal, eyepatch=False, beard=False, mean=False, x_eyes=False, blink=False, mouth_open=0.0):
    x1, y1, x2, y2 = head_bbox
    ellipse(draw, head_bbox, pal.skin, pal.outline, width=4)
    nose = [(lerp(x1, x2, 0.53), lerp(y1, y2, 0.42)), (lerp(x1, x2, 0.60), lerp(y1, y2, 0.56)), (lerp(x1, x2, 0.52), lerp(y1, y2, 0.60))]
    line(draw, nose, pal.skin_shadow, width=3)
    brow_y = lerp(y1, y2, 0.34)
    eye_y = lerp(y1, y2, 0.43)
    if x_eyes:
        for ex in [lerp(x1, x2, 0.38), lerp(x1, x2, 0.61)]:
            line(draw, [(ex - 6, eye_y - 6), (ex + 6, eye_y + 6)], pal.accent or (255, 255, 255, 255), width=3)
            line(draw, [(ex - 6, eye_y + 6), (ex + 6, eye_y - 6)], pal.accent or (255, 255, 255, 255), width=3)
    else:
        left_brow = [(lerp(x1, x2, 0.28), brow_y + (3 if mean else 0)), (lerp(x1, x2, 0.43), brow_y - (4 if mean else 1))]
        right_brow = [(lerp(x1, x2, 0.56), brow_y - (4 if mean else 1)), (lerp(x1, x2, 0.72), brow_y + (3 if mean else 0))]
        line(draw, left_brow, pal.outline, width=4)
        line(draw, right_brow, pal.outline, width=4)
        if eyepatch:
            patch_box = (lerp(x1, x2, 0.27), eye_y - 7, lerp(x1, x2, 0.47), eye_y + 7)
            ellipse(draw, patch_box, pal.hat, pal.outline, width=2)
            line(draw, [(x1 + 8, eye_y - 10), (x2 - 4, eye_y - 4)], pal.hat, width=3)
        else:
            if blink:
                line(draw, [(lerp(x1, x2, 0.31), eye_y), (lerp(x1, x2, 0.40), eye_y + 1)], pal.outline, width=3)
            else:
                ellipse(draw, (lerp(x1, x2, 0.31), eye_y - 4, lerp(x1, x2, 0.40), eye_y + 4), (255,255,255,255), pal.outline, width=2)
                circle(draw, (lerp(x1, x2, 0.36), eye_y), 2, pal.outline)
        if blink:
            line(draw, [(lerp(x1, x2, 0.58), eye_y), (lerp(x1, x2, 0.67), eye_y + 1)], pal.outline, width=3)
        else:
            ellipse(draw, (lerp(x1, x2, 0.58), eye_y - 4, lerp(x1, x2, 0.67), eye_y + 4), (255,255,255,255), pal.outline, width=2)
            circle(draw, (lerp(x1, x2, 0.62), eye_y), 2, pal.outline)
    mouth_mid = lerp(y1, y2, 0.80) + mouth_open * 10.0
    mouth = [(lerp(x1, x2, 0.38), lerp(y1, y2, 0.76)), (lerp(x1, x2, 0.51), mouth_mid), (lerp(x1, x2, 0.67), lerp(y1, y2, 0.73))]
    line(draw, mouth, pal.outline, width=3)
    if beard and pal.beard:
        beard_pts = [(lerp(x1,x2,0.23), lerp(y1,y2,0.63)), (lerp(x1,x2,0.50), lerp(y1,y2,0.94)), (lerp(x1,x2,0.80), lerp(y1,y2,0.62)), (lerp(x1,x2,0.69), lerp(y1,y2,0.86)), (lerp(x1,x2,0.38), lerp(y1,y2,0.86))]
        poly(draw, beard_pts, pal.beard, pal.outline, width=3)


def draw_character(kind: str, anim: str, frame_idx: int, nframes: int, frame_size=BASE_FRAME) -> Image.Image:
    pal = PALETTES[kind]
    w, h = frame_size[0] * SCALE, frame_size[1] * SCALE
    img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img, "RGBA")
    pose = animation_pose(anim, frame_idx, nframes)

    cx = w * (0.48 if kind == "pirate_admiral" else 0.50)
    ground = h * 0.83
    bob = pose["bob"] * SCALE
    root = (cx, ground + bob)
    global_tilt = pose["body_tilt"] + (5 if kind in ("pirate_raider", "pirate_corsair") and anim == "taunt" else 0)
    death_t = pose["death_t"]

    # Whole body offsets / lean for death.
    char_origin = (root[0] + pose["root_x"] * SCALE + death_t * 12 * SCALE, root[1] + death_t * 5 * SCALE)

    # Shadow
    if anim != "death":
        shadow_w = 42 * SCALE / 4 + abs(pose["root_x"]) * 1.5
        ellipse(draw, (cx - shadow_w, ground + 8, cx + shadow_w, ground + 26 - min(8, abs(bob) * 0.4)), (0, 0, 0, 70))

    # Local joints.
    hip = transform((0, -60), char_origin, deg=global_tilt)
    chest = transform((0, -124 + pose["shoulder_bounce"]), char_origin, deg=global_tilt)
    head_center = transform((8, -202 + pose["head_y"]), char_origin, deg=global_tilt + pose["head_tilt"])

    # Back arm / weapon arm first.
    if kind == "pirate_admiral":
        back_shoulder = transform((20, -136), char_origin, deg=global_tilt)
        front_shoulder = transform((-22, -136), char_origin, deg=global_tilt)
    else:
        back_shoulder = transform((24, -136), char_origin, deg=global_tilt)
        front_shoulder = transform((-26, -136), char_origin, deg=global_tilt)

    # Legs.
    left_hip = transform((-16, -56), char_origin, deg=global_tilt)
    right_hip = transform((18, -56), char_origin, deg=global_tilt)
    left_knee = transform((-18, 4), left_hip, deg=pose["left_leg"])
    right_knee = transform((12, 4), right_hip, deg=pose["right_leg"])
    left_foot = transform((-8, 52 - pose["left_foot_lift"]), left_knee, deg=pose["left_leg"] * 0.3)
    right_foot = transform((10, 52 - pose["right_foot_lift"]), right_knee, deg=pose["right_leg"] * 0.3)

    for hip_pt, knee_pt, foot_pt, ang in [
        (left_hip, left_knee, left_foot, pose["left_leg"]),
        (right_hip, right_knee, right_foot, pose["right_leg"]),
    ]:
        line(draw, [hip_pt, knee_pt, foot_pt], pal.pants, width=13)
        line(draw, [hip_pt, knee_pt, foot_pt], pal.outline, width=4)
        draw_boot(draw, foot_pt, 24, 18, ang * 0.2, pal)

    # Body / shirt / coat
    torso_pts = [transform(p, chest, deg=global_tilt) for p in [(-34, -8), (30, -8), (42, 58), (0, 76), (-44, 58)]]
    poly(draw, torso_pts, pal.coat, pal.outline, width=5)
    shirt_pts = [transform(p, chest, deg=global_tilt) for p in [(-10, -4), (18, -4), (14, 52), (-16, 52)]]
    poly(draw, shirt_pts, pal.shirt, pal.outline, width=4)
    lapel_left = [transform(p, chest, deg=global_tilt) for p in [(-16, -6), (-2, 16), (-10, 44), (-20, 18)]]
    lapel_right = [transform(p, chest, deg=global_tilt) for p in [(8, -6), (20, 16), (16, 42), (4, 18)]]
    poly(draw, lapel_left, pal.coat2, pal.outline, width=3)
    poly(draw, lapel_right, pal.coat2, pal.outline, width=3)
    sash_box = rotated_rect_points(transform((0, 24), chest, deg=global_tilt), 44, 12, global_tilt)
    poly(draw, sash_box, pal.sash, pal.outline, width=3)
    for bx in [-10, 0, 10]:
        circle(draw, transform((bx, 4), chest, deg=global_tilt), 3, pal.gold, pal.outline, width=1)

    coat_sway = pose["coat_sway"]
    tail_left = [transform(p, hip, deg=global_tilt + coat_sway) for p in [(-36, 0), (-8, -2), (-8, 48), (-30, 58)]]
    tail_right = [transform(p, hip, deg=global_tilt - coat_sway) for p in [(8, -2), (34, 0), (28, 58), (6, 48)]]
    poly(draw, tail_left, pal.coat, pal.outline, width=4)
    poly(draw, tail_right, pal.coat, pal.outline, width=4)

    # Back arm
    back_elbow = transform((4, 52), back_shoulder, deg=pose["left_arm"])
    back_hand = transform((0, 48), back_elbow, deg=pose["left_arm"] * 0.55)
    line(draw, [back_shoulder, back_elbow, back_hand], pal.coat, width=12)
    line(draw, [back_shoulder, back_elbow, back_hand], pal.outline, width=4)
    circle(draw, back_hand, 7, pal.skin, pal.outline, width=2)
    if anim == "taunt":
        line(draw, [transform((0,-10), back_hand), transform((10,-22), back_hand)], pal.outline, width=3)

    # Front arm / weapon
    front_elbow = transform((6, 50), front_shoulder, deg=pose["right_arm"])
    front_hand = transform((0, 46), front_elbow, deg=pose["weapon"] * 0.35)
    line(draw, [front_shoulder, front_elbow, front_hand], pal.coat, width=13)
    line(draw, [front_shoulder, front_elbow, front_hand], pal.outline, width=4)
    circle(draw, front_hand, 8, pal.skin, pal.outline, width=2)
    draw_sword(draw, front_hand, pose["weapon"], 92 if kind == "pirate_admiral" else 86, pal, curve=(16 if kind in ("pirate_raider", "pirate_corsair") else 5))
    if anim == "slash":
        arc_box = (front_hand[0] - 70, front_hand[1] - 96, front_hand[0] + 110, front_hand[1] + 76)
        draw.arc(arc_box, start=205, end=336, fill=(255, 245, 200, 180), width=8)
        draw.arc(arc_box, start=214, end=328, fill=(255, 255, 255, 120), width=4)
    elif anim in {"idle", "walk", "taunt"} and frame_idx % 2 == 0:
        blade_tip = transform((92 if kind == "pirate_admiral" else 86, 4 if kind in ("pirate_raider", "pirate_corsair") else 0), front_hand, pose["weapon"])
        line(draw, [blade_tip, (blade_tip[0] + 10, blade_tip[1] - 8)], (255, 255, 255, 100), width=2)

    # Neck / head / hat
    draw_human_neck(draw, chest, head_center, global_tilt, pal, kind=kind)

    head_bbox = (head_center[0] - 28, head_center[1] - 34, head_center[0] + 28, head_center[1] + 34)
    draw_face(draw, head_bbox, pal, eyepatch=(kind == "pirate_admiral"), beard=(kind in ("pirate_raider", "pirate_corsair")), mean=True, x_eyes=pose["x_eyes"], blink=pose["blink"], mouth_open=pose["mouth_open"])
    draw_hat(draw, head_center, 1.0, pal, skull=True, tilt=pose["hat_tilt"] + global_tilt * 0.15)

    if kind in ("pirate_raider", "pirate_corsair"):
        # chest skull motif
        chest_c = transform((0, 14), chest, deg=global_tilt)
        circle(draw, (chest_c[0], chest_c[1] - 4), 8, (242, 236, 230, 255), pal.outline, width=2)
        line(draw, [(chest_c[0] - 7, chest_c[1] + 5), (chest_c[0] + 7, chest_c[1] + 5)], (242,236,230,255), width=3)
        line(draw, [(chest_c[0], chest_c[1] + 1), (chest_c[0], chest_c[1] + 9)], pal.outline, width=2)

    # Death settle pose, ground line accent
    if anim == "death":
        draw.line((0, ground + 24, w, ground + 24), fill=(0, 0, 0, 0), width=1)

    return downsample(img, frame_size)


def build_sheet(target: str, rows: List[Tuple[str, int, int]], render_fn, out_dir: Path, frame_size=BASE_FRAME, label_width=LABEL_WIDTH):
    fw, fh = frame_size
    max_frames = max(n for _, n, _ in rows)
    sheet = Image.new("RGBA", (label_width + fw * max_frames, fh * len(rows)), (0, 0, 0, 0))
    preview = Image.new("RGBA", (label_width + fw * max_frames, fh * len(rows)), (34, 34, 40, 255))
    draw_sheet = ImageDraw.Draw(sheet, "RGBA")
    draw_prev = ImageDraw.Draw(preview, "RGBA")
    draw_prev.rectangle((0, 0, preview.width, preview.height), fill=(43, 33, 40, 255))

    rows_meta = []
    first = None
    for row_idx, (anim, nframes, duration_ms) in enumerate(rows):
        y = row_idx * fh
        for dr in [draw_sheet, draw_prev]:
            dr.rectangle((0, y, label_width - 1, y + fh - 1), fill=(18, 22, 30, 235))
            dr.text((8, y + 10), anim, fill=(236, 240, 244, 255), font=font(14))
            dr.text((8, y + 30), f"{nframes}f @ {duration_ms}ms", fill=(160, 170, 184, 255), font=font(11))
        rects = []
        for frame_idx in range(nframes):
            frame = render_fn(anim, frame_idx, nframes)
            if first is None:
                first = frame.copy()
            x = label_width + frame_idx * fw
            sheet.alpha_composite(frame, (x, y))
            preview.alpha_composite(frame, (x, y))
            rects.append({"x": x, "y": y, "w": fw, "h": fh})
        rows_meta.append({
            "animation": anim,
            "row_index": row_idx,
            "frame_count": nframes,
            "duration_ms": duration_ms,
            "duration_secs": round(duration_ms / 1000.0, 6),
            "rects": rects,
        })

    can = render_fn("idle", 1, 6)
    can_bg = Image.new("RGBA", frame_size, (43, 33, 40, 255))
    can_bg.alpha_composite(can, (0, 0))

    canonical_path = out_dir / f"{target}_canonical.png"
    canonical_transparent_path = out_dir / f"{target}_canonical_transparent.png"
    sheet_path = out_dir / f"{target}_spritesheet.png"
    yaml_path = out_dir / f"{target}_spritesheet.yaml"
    preview_path = out_dir / f"{target}_preview_labeled.png"

    can_bg.save(canonical_path)
    can.save(canonical_transparent_path)
    sheet.save(sheet_path)
    preview.save(preview_path)

    manifest = {
        "target": target,
        "image": sheet_path.name,
        "label_width": label_width,
        "frame_width": fw,
        "frame_height": fh,
        "rows": rows_meta,
        "body_metrics": alpha_bbox_metrics(first or can),
    }
    yaml_path.write_text(yaml.safe_dump(manifest, sort_keys=False, width=120))
    return {
        "canonical": canonical_path,
        "canonical_transparent": canonical_transparent_path,
        "spritesheet": sheet_path,
        "yaml": yaml_path,
        "preview": preview_path,
    }
