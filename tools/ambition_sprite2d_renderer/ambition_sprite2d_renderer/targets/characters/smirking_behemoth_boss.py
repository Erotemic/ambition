from __future__ import annotations

"""Procedural sprite generator for the Smirking Behemoth boss.

Design goals derived from the repo discussion and the pasted visual refs:

- Giant dark monolith / slab silhouette with rounded top corners.
- Large expressive eyes and a small, smirking side-mouth.
- Four authored rows only:
    * ``rest``       — mouth closed / idle smirk (acts as mouth-closed row)
    * ``mouth_open`` — mouth open bite / taunt pose
    * ``eye_beam``   — eyes flash only; NO projectile art in the boss sheet
    * ``death``      — cracked / collapsing death animation
- No drop shadows. The sheet should be transparent-only outside the sprite.
- Emit the standard ``*_spritesheet.png`` / ``yaml`` / ``ron`` / ``actor``
  bundle through the generic tack-on renderer so the sandbox and catalog can
  load it without bespoke conversion steps.
"""

import math
from pathlib import Path
from typing import Dict, List, Tuple

from PIL import Image, ImageColor, ImageDraw

from ...authoring.tackon_sheet import build_sheet, write_canonical

RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_NAME = "smirking_behemoth_boss"
SHEET_FILES = [
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
]

ACTOR_METADATA = {
    "actor": {
        "character_id": "npc_smirking_behemoth_boss",
        "display_name": "Smirking Behemoth",
    },
    "body": {
        "body_plan": "BossMultipart",
        "body_kind": "Wide",
        "mass_class": "Heavy",
        "locomotion_hint": "Stationary",
        "traits": ["boss", "behemoth", "eye_beam", "mouth_attack"],
    },
    "brain": {"default_preset": "stand_still"},
    "actions": {"default_preset": "peaceful"},
    "animation_bindings": {
        "default": {"animation": "rest", "events": []},
        "action.melee.primary": {
            "animation": "mouth_open",
            "events": [
                {"t": 0.28, "event": "telegraph_peak", "source": TARGET_NAME},
                {"t": 0.42, "event": "hitbox_active_start", "source": TARGET_NAME},
                {"t": 0.70, "event": "hitbox_active_end", "source": TARGET_NAME},
            ],
        },
        "action.special.eye_beam": {
            "animation": "eye_beam",
            "events": [
                {"t": 0.30, "event": "eye_charge_start", "source": TARGET_NAME},
                {"t": 0.56, "event": "eye_charge_peak", "source": TARGET_NAME},
                {"t": 0.82, "event": "eye_charge_release", "source": TARGET_NAME},
            ],
        },
        "death": {"animation": "death", "events": []},
    },
    "sockets": {
        "eye": {"source": f"{TARGET_NAME}.geometry", "point": {"x": 164.0, "y": 96.0}},
        "eye_right": {
            "source": f"{TARGET_NAME}.geometry",
            "point": {"x": 149.0, "y": 101.0},
        },
        "mouth": {
            "source": f"{TARGET_NAME}.geometry",
            "point": {"x": 176.0, "y": 164.0},
        },
    },
    "tags": ["boss", "behemoth", "eye_beam"],
}

# Row 0 intentionally uses the Idle alias `rest`; this is the mouth-closed pose.
ROWS: List[Tuple[str, int, int]] = [
    ("rest", 6, 125),
    ("mouth_open", 6, 92),
    ("eye_beam", 6, 82),
    ("death", 8, 108),
]

FRAME_SIZE = (208, 288)
# The visible monolith slab intentionally starts below the hat and ends on the
# frame floor.  Keep the gameplay body metrics tied to these constants so the
# generated contact/hurt box touches the floor but never extends under it.
SMIRKING_BODY_X1 = 0.0
SMIRKING_BODY_Y1 = 22.0
SMIRKING_BODY_X2 = float(FRAME_SIZE[0])
SMIRKING_BODY_Y2 = float(FRAME_SIZE[1])
SMIRKING_BODY_FEET_X = (SMIRKING_BODY_X1 + SMIRKING_BODY_X2) * 0.5
SMIRKING_BODY_FEET_Y = SMIRKING_BODY_Y2
SUPER = 4
W, H = FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER

OUTLINE = (16, 8, 20, 255)
BODY = (24, 2, 24, 255)
BODY_HI = (48, 24, 52, 255)
BODY_MID = (34, 10, 38, 255)
EYE_WHITE = (246, 242, 248, 255)
EYE_GLOW = (255, 247, 205, 255)
EYE_PUPIL = (228, 200, 28, 255)
VEIN = (214, 120, 130, 255)
MOUTH_LIP = (250, 250, 252, 255)
MOUTH_INNER = (96, 44, 58, 255)
MOUTH_TONGUE = (182, 132, 176, 255)
DUST = (112, 112, 118, 255)
CRACK = (72, 54, 72, 255)
HAT = (42, 36, 46, 255)
HAT_BAND = (188, 132, 208, 255)
HAT_HI = (92, 86, 98, 255)
EXPLOSION_CORE = (255, 247, 210, 255)
EXPLOSION_FLAME = (236, 165, 82, 255)
EXPLOSION_SMOKE = (112, 50, 80, 224)


def _rgba(color: str, alpha: int = 255) -> RGBA:
    r, g, b = ImageColor.getrgb(color)
    return (r, g, b, alpha)


def _s(v: float) -> int:
    return int(round(v * SUPER))


def _pt(x: float, y: float) -> Tuple[int, int]:
    return (_s(x), _s(y))


def _box(x1: float, y1: float, x2: float, y2: float) -> Tuple[int, int, int, int]:
    return (_s(x1), _s(y1), _s(x2), _s(y2))


def _overlay_draw(img: Image.Image) -> tuple[Image.Image, ImageDraw.ImageDraw]:
    """Return a transparent overlay layer + draw object for alpha compositing.

    PIL's ImageDraw writes straight RGBA pixels into the destination, which can
    overwrite existing color/alpha when given semi-transparent fills. For glows
    and other translucent shapes, draw onto a scratch layer and compose it back
    with `alpha_composite` instead.
    """
    layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
    return layer, ImageDraw.Draw(layer, "RGBA")


def _composite_ellipse(
    img: Image.Image,
    bbox: Tuple[float, float, float, float],
    *,
    fill: RGBA,
    outline: RGBA | None = None,
    width: int = 1,
) -> None:
    layer, draw = _overlay_draw(img)
    draw.ellipse(_box(*bbox), fill=fill, outline=outline, width=width)
    img.alpha_composite(layer)


def _composite_polygon(
    img: Image.Image,
    points: List[Point],
    *,
    fill: RGBA,
    outline: RGBA | None = None,
    width: int = 1,
) -> None:
    layer, draw = _overlay_draw(img)
    draw.polygon([_pt(x, y) for x, y in points], fill=fill, outline=outline)
    if outline is not None and len(points) > 1:
        draw.line(
            [_pt(x, y) for x, y in points + [points[0]]],
            fill=outline,
            width=width,
            joint="curve",
        )
    img.alpha_composite(layer)


def _composite_rounded_rect(
    img: Image.Image,
    bbox: Tuple[float, float, float, float],
    *,
    radius: float,
    fill: RGBA,
    outline: RGBA | None = None,
    width: int = 1,
) -> None:
    layer, draw = _overlay_draw(img)
    draw.rounded_rectangle(
        _box(*bbox), radius=_s(radius), fill=fill, outline=outline, width=width
    )
    img.alpha_composite(layer)


def _draw_explosion(
    img: Image.Image,
    center: Point,
    radius: float,
    progress: float,
    *,
    core_fill: RGBA = EXPLOSION_CORE,
    flame_fill: RGBA = EXPLOSION_FLAME,
    smoke_fill: RGBA = EXPLOSION_SMOKE,
    outline: RGBA | None = OUTLINE,
    seed: float = 0.0,
    spark_count: int = 6,
) -> None:
    """Self-contained explosion helper for bursty death / impact FX.

    This stays local to the Smirking Behemoth target for now, but is written to
    be generally reusable later: deterministic, alpha-composited, and suitable
    for explosions, muzzle pops, impact bursts, or magical detonations.
    """
    progress = max(0.0, min(1.0, progress))
    cx, cy = center
    layer, draw = _overlay_draw(img)

    smoke_r = radius * (0.74 + progress * 0.95)
    flame_r = radius * (0.60 + progress * 0.34)
    core_r = radius * (0.30 + (1.0 - progress) * 0.24)

    # Smoke puff ring
    smoke_count = 7
    for i in range(smoke_count):
        ang = (math.tau * i / smoke_count) + seed * 0.67
        orbit = smoke_r * (0.52 + 0.10 * math.sin(seed + i * 1.3))
        rx = smoke_r * (0.34 + 0.07 * math.sin(seed * 1.2 + i * 1.7))
        ry = smoke_r * (0.27 + 0.05 * math.cos(seed * 1.5 + i * 1.1))
        ox = cx + math.cos(ang) * orbit
        oy = cy + math.sin(ang) * orbit * 0.82
        draw.ellipse((ox - rx, oy - ry, ox + rx, oy + ry), fill=smoke_fill)
    draw.ellipse(
        (
            cx - smoke_r * 0.45,
            cy - smoke_r * 0.38,
            cx + smoke_r * 0.45,
            cy + smoke_r * 0.38,
        ),
        fill=smoke_fill,
    )

    # Flame starburst
    pts: List[Point] = []
    spokes = 12
    for i in range(spokes):
        ang = (math.tau * i / spokes) + seed * 0.41
        if i % 2 == 0:
            rr = flame_r * (0.94 + 0.11 * math.sin(seed + i))
        else:
            rr = flame_r * (0.43 + 0.09 * math.cos(seed * 0.8 + i * 1.2))
        pts.append((cx + math.cos(ang) * rr, cy + math.sin(ang) * rr))
    draw.polygon(pts, fill=flame_fill, outline=outline)

    # Core / flash
    draw.ellipse(
        (cx - core_r, cy - core_r, cx + core_r, cy + core_r),
        fill=core_fill,
        outline=outline,
    )
    inner_r = core_r * 0.52
    draw.ellipse(
        (cx - inner_r, cy - inner_r, cx + inner_r, cy + inner_r),
        fill=(255, 252, 236, 255),
    )

    # Radial sparks
    for i in range(max(0, spark_count)):
        ang = (math.tau * i / max(1, spark_count)) + seed * 0.51
        start_r = smoke_r * (0.70 + 0.06 * math.sin(seed + i))
        end_r = smoke_r * (1.04 + 0.24 * progress + 0.07 * math.cos(seed * 1.1 + i))
        x1 = cx + math.cos(ang) * start_r
        y1 = cy + math.sin(ang) * start_r
        x2 = cx + math.cos(ang) * end_r
        y2 = cy + math.sin(ang) * end_r
        draw.line(
            (x1, y1, x2, y2), fill=core_fill, width=max(1, int(round(radius * 0.08)))
        )

    img.alpha_composite(layer)


def _erase_rect(img: Image.Image, x1: float, y1: float, x2: float, y2: float) -> None:
    """Punch a transparent chip out of the supersampled frame."""
    img.paste((0, 0, 0, 0), _box(x1, y1, x2, y2))


def _erase_ellipse(
    img: Image.Image, x1: float, y1: float, x2: float, y2: float
) -> None:
    """Punch a transparent elliptical chip out of the supersampled frame."""
    mask = Image.new("L", img.size, 0)
    mask_draw = ImageDraw.Draw(mask)
    mask_draw.ellipse(_box(x1, y1, x2, y2), fill=255)
    img.paste((0, 0, 0, 0), (0, 0), mask)


def _erase_polygon(img: Image.Image, points: List[Point]) -> None:
    """Punch a transparent polygonal chip out of the supersampled frame."""
    mask = Image.new("L", img.size, 0)
    mask_draw = ImageDraw.Draw(mask)
    mask_draw.polygon([_pt(x, y) for x, y in points], fill=255)
    img.paste((0, 0, 0, 0), (0, 0), mask)


def _lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def _ease(t: float) -> float:
    t = max(0.0, min(1.0, t))
    return t * t * (3.0 - 2.0 * t)


def _arc_points(
    cx: float,
    cy: float,
    rx: float,
    ry: float,
    start_deg: float,
    end_deg: float,
    steps: int = 16,
) -> List[Point]:
    pts: List[Point] = []
    for i in range(steps + 1):
        f = i / max(1, steps)
        deg = _lerp(start_deg, end_deg, f)
        rad = math.radians(deg)
        pts.append((cx + math.cos(rad) * rx, cy + math.sin(rad) * ry))
    return pts


def _rounded_monolith(
    draw: ImageDraw.ImageDraw,
    box: Tuple[float, float, float, float],
    radius: float,
    fill: RGBA,
    outline: RGBA,
) -> None:
    """Draw the behemoth body with only the TOP corners rounded.

    Avoids floor-contact softness so the boss still feels heavy and planted.
    """
    x1, y1, x2, y2 = box
    r = radius
    pts: List[Point] = [(x1, y2), (x1, y1 + r)]
    pts += _arc_points(x1 + r, y1 + r, r, r, 180, 270, 8)[1:]
    pts += _arc_points(x2 - r, y1 + r, r, r, 270, 360, 8)[1:]
    pts += [(x2, y2), (x1, y2)]
    draw.polygon([_pt(x, y) for x, y in pts], fill=fill, outline=outline)


def _draw_cracks(draw: ImageDraw.ImageDraw, settle: float) -> None:
    crack_sets = [
        [(92, 76), (86, 88), (90, 98), (84, 110)],
        [(118, 72), (124, 86), (122, 100)],
        [(102, 44), (96, 54), (102, 60), (97, 69)],
        [(78, 128), (86, 137), (82, 149)],
    ]
    for idx, seg in enumerate(crack_sets):
        visible = min(1.0, max(0.0, settle * 1.25 - idx * 0.11))
        if visible <= 0:
            continue
        count = max(2, int(round(len(seg) * visible)))
        pts = [_pt(*p) for p in seg[:count]]
        draw.line(pts, fill=CRACK, width=max(1, _s(0.85)), joint="curve")


def _body_geometry(anim: str, frame_idx: int, nframes: int) -> Dict[str, float]:
    t = frame_idx / max(1, nframes - 1)
    # Smirking Behemoth is intentionally a near-AABB monolith. Keep the
    # rest / attack body planted and pixel-tight so the generated
    # body_metrics bbox, LDtk character box, and runtime hurtbox can all
    # be the same rectangle. Attack motion happens in facial features and
    # external eye-beam projectiles, not by bobbing the slab.
    bob = 0.0
    if anim == "mouth_open":
        mouth_open = 0.12 + 0.88 * math.sin(t * math.pi)
        beam = 0.0
        settle = 0.0
    elif anim == "eye_beam":
        mouth_open = 0.0
        beam = 0.35 + 0.65 * math.sin(t * math.pi)
        settle = 0.0
    elif anim == "death":
        mouth_open = 0.45 + 0.25 * t
        beam = 0.0
        settle = _ease(t)
        # Death may crack/settle internally, but do not move the slab
        # footprint off the floor: the anvil/explosion owns the motion.
        bob = 0.0
    else:
        mouth_open = 0.0
        beam = 0.0
        settle = 0.0

    body_x1 = SMIRKING_BODY_X1
    # Leave just enough top space for the hat to sit ON the monolith's
    # head instead of being clipped into it. The hat reaches y=0, the
    # slab reaches the bottom pixel, and the generated body metrics use
    # the same constants so runtime contact/hurt boxes touch the floor
    # without creating an invisible gutter below the boss.
    body_y1 = SMIRKING_BODY_Y1
    body_x2 = SMIRKING_BODY_X2
    body_y2 = SMIRKING_BODY_Y2
    eye_x = body_x2 - 44.0 + math.sin(t * math.tau) * (1.0 if anim == "rest" else 0.35)
    eye_y = body_y1 + 82.0 + (settle * 1.5)
    eye_r = 14.0 + beam * 4.5
    mouth_x = body_x2 - 24.0
    mouth_y = body_y1 + 142.0 + settle * 1.5
    mouth_w = 50.0 + mouth_open * 10.0
    mouth_h = 5.0 + mouth_open * 30.0
    death_eye_left_x = body_x1 + 68.0
    death_eye_right_x = body_x1 + 132.0
    death_eye_y = body_y1 + 92.0 + settle * 2.0
    hat_cx = (body_x1 + body_x2) * 0.5 - 4.0 + (settle * 8.0)
    # The brim underside sits exactly on the slab top. Keep the crown
    # touching the top pixel and inside the frame and the brim visually tangent to the head.
    hat_y = body_y1 - 14.0 + settle * 1.6
    hat_tilt = -3.0 + math.sin(t * math.tau) * 1.5 + settle * 16.0
    return {
        "t": t,
        "bob": bob,
        "beam": beam,
        "settle": settle,
        "mouth_open": mouth_open,
        "body_x1": body_x1,
        "body_y1": body_y1,
        "body_x2": body_x2,
        "body_y2": body_y2,
        "eye_x": eye_x,
        "eye_y": eye_y,
        "eye_r": eye_r,
        "mouth_x": mouth_x,
        "mouth_y": mouth_y,
        "mouth_w": mouth_w,
        "mouth_h": mouth_h,
        "death_eye_left_x": death_eye_left_x,
        "death_eye_right_x": death_eye_right_x,
        "death_eye_y": death_eye_y,
        "hat_cx": hat_cx,
        "hat_y": hat_y,
        "hat_tilt": hat_tilt,
    }


def _draw_body(draw: ImageDraw.ImageDraw, g: Dict[str, float]) -> None:
    _rounded_monolith(
        draw,
        (g["body_x1"], g["body_y1"], g["body_x2"], g["body_y2"]),
        radius=15.0,
        fill=BODY,
        outline=OUTLINE,
    )
    # Keep the slab nearly featureless. A subtle inset panel gives volume,
    # but it deliberately avoids the earlier accidental "P" silhouette.
    draw.rounded_rectangle(
        _box(
            g["body_x1"] + 11.0,
            g["body_y1"] + 18.0,
            g["body_x2"] - 12.0,
            g["body_y2"] - 28.0,
        ),
        radius=_s(10.0),
        fill=BODY_MID,
        outline=None,
    )
    draw.rounded_rectangle(
        _box(
            g["body_x1"] + 22.0,
            g["body_y1"] + 30.0,
            g["body_x2"] - 26.0,
            g["body_y2"] - 52.0,
        ),
        radius=_s(8.0),
        fill=BODY_HI,
        outline=None,
    )
    # Darken the rim so the body reads as a single monolith, not armor plates.
    draw.rectangle(
        _box(g["body_x1"], g["body_y1"] + 55.0, g["body_x1"] + 14.0, g["body_y2"]),
        fill=BODY,
    )
    draw.rectangle(
        _box(g["body_x2"] - 11.0, g["body_y1"] + 65.0, g["body_x2"], g["body_y2"]),
        fill=BODY,
    )


def _rot_points(points: List[Point], cx: float, cy: float, deg: float) -> List[Point]:
    rad = math.radians(deg)
    c = math.cos(rad)
    s = math.sin(rad)
    out: List[Point] = []
    for x, y in points:
        dx = x - cx
        dy = y - cy
        out.append((cx + dx * c - dy * s, cy + dx * s + dy * c))
    return out


def _draw_hat(img: Image.Image, g: Dict[str, float]) -> None:
    """Distinctive little bowler hat so the boss isn't just Grinning Colossus."""
    cx = g["hat_cx"]
    y = g["hat_y"]
    tilt = g["hat_tilt"]
    brim = _rot_points(
        [
            (cx - 24.0, y + 8.0),
            (cx + 19.0, y + 8.0),
            (cx + 16.0, y + 14.0),
            (cx - 27.0, y + 14.0),
        ],
        cx,
        y + 10.0,
        tilt,
    )
    crown = _rot_points(
        [
            (cx - 12.0, y - 8.0),
            (cx + 8.0, y - 8.0),
            (cx + 11.0, y + 8.0),
            (cx - 15.0, y + 8.0),
        ],
        cx,
        y + 2.0,
        tilt,
    )
    band = _rot_points(
        [
            (cx - 12.5, y + 2.5),
            (cx + 9.5, y + 2.5),
            (cx + 10.5, y + 7.0),
            (cx - 13.5, y + 7.0),
        ],
        cx,
        y + 5.0,
        tilt,
    )
    brim_hi = _rot_points(
        [
            (cx - 20.0, y + 9.0),
            (cx - 4.0, y + 9.0),
            (cx - 6.0, y + 11.0),
            (cx - 22.0, y + 11.0),
        ],
        cx,
        y + 10.0,
        tilt,
    )
    _composite_polygon(img, brim, fill=HAT, outline=OUTLINE, width=max(1, _s(0.9)))
    _composite_polygon(img, crown, fill=HAT, outline=OUTLINE, width=max(1, _s(0.9)))
    _composite_polygon(img, band, fill=HAT_BAND, outline=None)
    _composite_polygon(img, brim_hi, fill=HAT_HI, outline=None)


def _draw_eye(
    draw: ImageDraw.ImageDraw,
    g: Dict[str, float],
    *,
    cx: float | None = None,
    cy: float | None = None,
    bloodshot: bool = False,
    img: Image.Image | None = None,
) -> None:
    beam = g["beam"]
    settle = g["settle"]
    cx = g["eye_x"] if cx is None else cx
    cy = g["eye_y"] if cy is None else cy
    rx = g["eye_r"]
    ry = rx - 1.3

    # Eye-beam telegraph: flash the eye itself only. Projectiles are emitted
    # by a separate asset, so this row has no bubbles, rays, or beam sprites.
    if beam > 0.0:
        glow = max(0.0, beam - 0.18)
        if glow > 0.0 and img is not None:
            alpha = int(58 + glow * 90)
            _composite_ellipse(
                img,
                (
                    cx - rx - 5.0 * glow,
                    cy - ry - 5.0 * glow,
                    cx + rx + 5.0 * glow,
                    cy + ry + 5.0 * glow,
                ),
                fill=(255, 246, 196, alpha),
            )

    eye_fill = EYE_GLOW if beam > 0.72 else EYE_WHITE
    draw.ellipse(
        _box(cx - rx, cy - ry, cx + rx, cy + ry),
        fill=eye_fill,
        outline=OUTLINE,
        width=max(1, _s(1.0)),
    )

    if bloodshot:
        veins = [
            [
                (cx - rx + 3.0, cy - 1.0),
                (cx - rx + 8.0, cy - 5.0),
                (cx - rx + 11.0, cy - 9.0),
            ],
            [
                (cx + rx - 3.0, cy + 1.0),
                (cx + rx - 8.0, cy + 5.0),
                (cx + rx - 11.0, cy + 9.0),
            ],
            [(cx - 1.0, cy - ry + 3.0), (cx - 4.0, cy - ry + 7.0)],
            [(cx + 1.0, cy + ry - 3.0), (cx + 5.0, cy + ry - 8.0)],
        ]
        for seg in veins:
            draw.line([_pt(x, y) for x, y in seg], fill=VEIN, width=max(1, _s(0.75)))

    pupil_color = EYE_GLOW if beam > 0.55 else EYE_PUPIL
    pupil_r = 3.6 + beam * 1.5
    draw.ellipse(
        _box(cx - pupil_r + 1.0, cy - pupil_r, cx + pupil_r + 1.0, cy + pupil_r),
        fill=pupil_color,
        outline=OUTLINE if beam > 0.75 else None,
    )

    if settle < 0.95:
        draw.ellipse(
            _box(cx - 4.4, cy - 5.2, cx - 1.0, cy - 1.8), fill=(255, 255, 255, 190)
        )


def _draw_mouth(
    draw: ImageDraw.ImageDraw, g: Dict[str, float], *, centered: bool = False
) -> None:
    open_amt = g["mouth_open"]
    if centered:
        mx = (g["body_x1"] + g["body_x2"]) * 0.5 + 1.0
        my = g["body_y1"] + 124.0
        mw = 36.0
        mh = 27.0
    else:
        mx = g["mouth_x"]
        my = g["mouth_y"]
        mw = g["mouth_w"]
        mh = g["mouth_h"]

    if open_amt < 0.08 and not centered:
        # Right-edge smirk / mouth slit, matching the source boss language.
        x0 = g["body_x2"] - 48.0
        x1 = g["body_x2"] - 6.0
        y = my
        pts = [(x0, y + 1.0), (x0 + 14.0, y + 1.6), (x1 - 10.0, y - 0.4), (x1, y - 1.1)]
        draw.line(
            [_pt(x, yy) for x, yy in pts],
            fill=MOUTH_LIP,
            width=max(1, _s(1.7)),
            joint="curve",
        )
        return

    # Side-mouth cavity: a white upper lip with a maroon blocky interior.
    if not centered:
        left = g["body_x2"] - mw
        right = g["body_x2"] - 6.0
        top = my - 5.0
        bottom = my + mh
    else:
        left = mx - mw * 0.5
        right = mx + mw * 0.5
        top = my - mh * 0.45
        bottom = my + mh * 0.55

    draw.rounded_rectangle(
        _box(left, top, right, bottom),
        radius=_s(3.0),
        fill=MOUTH_INNER,
        outline=OUTLINE,
        width=max(1, _s(1.0)),
    )
    draw.rounded_rectangle(
        _box(left, top, right, top + 5.0),
        radius=_s(2.0),
        fill=MOUTH_LIP,
        outline=None,
    )
    if centered:
        # Keep the death mouth as a clean gaping maw. Earlier revisions had a
        # gray plug/tongue shape here that read like an unrelated circle, so
        # we drop it and keep only the upper teeth bars. Any blast energy near
        # the mouth is now communicated by separate explosion FX.
        for offs in (-13.0, -7.0, -1.0, 5.0, 11.0):
            draw.line(
                [_pt(mx + offs, top + 0.5), _pt(mx + offs, top + 7.0)],
                fill=OUTLINE,
                width=max(1, _s(0.65)),
            )


def _erode_death_body(img: Image.Image, g: Dict[str, float]) -> None:
    """Legacy name retained, but now only emits breakup debris.

    Earlier revisions punched transparent holes into the body silhouette, which
    made the death FX read like the explosion pass was erasing the sprite. We
    now keep the behemoth body intact and spawn opaque debris shards around it
    instead.
    """
    settle = g["settle"]
    if settle <= 0.16:
        return
    shards = [
        (
            [
                (g["body_x2"] + 2.0, g["body_y2"] - 41.0),
                (g["body_x2"] + 18.0, g["body_y2"] - 39.0),
                (g["body_x2"] + 13.0, g["body_y2"] - 23.0),
                (g["body_x2"] + 1.0, g["body_y2"] - 28.0),
            ],
            0.18,
        ),
        (
            [
                (g["body_x2"] + 6.0, g["body_y2"] - 11.0),
                (g["body_x2"] + 22.0, g["body_y2"] - 6.0),
                (g["body_x2"] + 18.0, g["body_y2"] + 10.0),
                (g["body_x2"] + 4.0, g["body_y2"] + 5.0),
            ],
            0.28,
        ),
        (
            [
                (g["body_x1"] - 19.0, g["body_y2"] - 18.0),
                (g["body_x1"] - 2.0, g["body_y2"] - 14.0),
                (g["body_x1"] - 6.0, g["body_y2"] + 5.0),
                (g["body_x1"] - 23.0, g["body_y2"] + 1.0),
            ],
            0.38,
        ),
        (
            [
                (g["body_x2"] + 4.0, g["body_y1"] + 16.0),
                (g["body_x2"] + 24.0, g["body_y1"] + 20.0),
                (g["body_x2"] + 22.0, g["body_y1"] + 37.0),
                (g["body_x2"] + 8.0, g["body_y1"] + 31.0),
            ],
            0.52,
        ),
        (
            [
                (g["body_x1"] - 22.0, g["body_y1"] + 8.0),
                (g["body_x1"] - 4.0, g["body_y1"] + 9.0),
                (g["body_x1"] - 5.0, g["body_y1"] + 24.0),
                (g["body_x1"] - 18.0, g["body_y1"] + 22.0),
            ],
            0.66,
        ),
    ]
    for points, threshold in shards:
        if settle >= threshold:
            _composite_polygon(
                img, points, fill=BODY, outline=OUTLINE, width=max(1, _s(0.8))
            )
    if settle >= 0.74:
        _composite_ellipse(
            img,
            (
                g["body_x2"] + 3.0,
                g["body_y2"] - 30.0,
                g["body_x2"] + 31.0,
                g["body_y2"] - 1.0,
            ),
            fill=BODY,
            outline=OUTLINE,
            width=max(1, _s(0.8)),
        )


def _draw_death_explosions(img: Image.Image, g: Dict[str, float]) -> None:
    settle = g["settle"]
    if settle <= 0.12:
        return
    bursts = [
        ((g["body_x1"] + 10.0, g["body_y1"] + 18.0), 7.5, 0.12, 0.0),
        ((g["body_x2"] - 10.0, g["body_y1"] + 20.0), 8.0, 0.20, 0.8),
        ((g["body_x1"] - 7.0, g["body_y1"] + 64.0), 9.0, 0.30, 1.6),
        ((g["body_x2"] + 8.0, g["body_y1"] + 56.0), 9.0, 0.38, 2.3),
        (
            ((g["body_x1"] + g["body_x2"]) * 0.5 + 2.0, g["body_y1"] + 126.0),
            8.5,
            0.48,
            3.1,
        ),
        ((g["body_x2"] - 16.0, g["body_y2"] - 18.0), 10.0, 0.58, 3.9),
        ((g["body_x1"] - 10.0, g["body_y2"] - 10.0), 10.5, 0.68, 4.6),
        ((g["body_x2"] + 18.0, g["body_y2"] + 8.0), 11.5, 0.78, 5.4),
    ]
    for (cx, cy), radius, threshold, seed in bursts:
        if settle < threshold:
            continue
        burst_progress = min(1.0, (settle - threshold) / 0.26)
        _draw_explosion(
            img,
            (_s(cx), _s(cy)),
            _s(radius),
            burst_progress,
            core_fill=EXPLOSION_CORE,
            flame_fill=EXPLOSION_FLAME,
            smoke_fill=EXPLOSION_SMOKE,
            outline=OUTLINE,
            seed=seed + settle * 0.6,
            spark_count=5,
        )


def _draw_dust(draw: ImageDraw.ImageDraw, g: Dict[str, float]) -> None:
    settle = g["settle"]
    if settle <= 0.08:
        return
    particles = [
        (56.0, 44.0, -8.0, -9.0),
        (84.0, 36.0, -3.0, -13.0),
        (155.0, 46.0, 12.0, -12.0),
        (184.0, 88.0, 16.0, -3.0),
        (188.0, 139.0, 18.0, 3.0),
        (154.0, 206.0, 7.0, 15.0),
        (72.0, 216.0, -10.0, 11.0),
        (120.0, 30.0, 2.0, -16.0),
    ]
    for i, (ax, ay, vx, vy) in enumerate(particles):
        if settle < i * 0.075:
            continue
        px = ax + vx * settle * 2.8
        py = ay + vy * settle * 2.8
        r = 1.6 + (i % 3) * 0.5
        draw.ellipse(_box(px - r, py - r, px + r, py + r), fill=DUST)


def _draw_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img, "RGBA")
    g = _body_geometry(anim, frame_idx, nframes)

    _draw_body(draw, g)
    _draw_hat(img, g)

    if anim == "death":
        _erode_death_body(img, g)
        _draw_death_explosions(img, g)
        draw = ImageDraw.Draw(img, "RGBA")
        # Death turns front-facing: two separated cracked eyes plus central maw.
        _draw_eye(
            draw,
            g,
            cx=g["death_eye_left_x"],
            cy=g["death_eye_y"],
            bloodshot=True,
            img=img,
        )
        _draw_eye(
            draw,
            g,
            cx=g["death_eye_right_x"],
            cy=g["death_eye_y"] + 1.5,
            bloodshot=True,
            img=img,
        )
        _draw_mouth(draw, g, centered=True)
        _draw_cracks(draw, g["settle"])
        _draw_dust(draw, g)
    else:
        _draw_eye(draw, g, img=img)
        _draw_mouth(draw, g)

    return img.resize(FRAME_SIZE, Image.Resampling.NEAREST)


def _body_metrics_for_sheet(frame_width: int, frame_height: int) -> dict:
    """Gameplay body metrics for the monolith body only, excluding the hat.

    The alpha bbox intentionally spans the whole frame because the tiny hat
    reaches the top edge, but gameplay hurtboxes/contact damage should be tight
    around the black behemoth slab. The bbox bottom is exactly the frame floor:
    it blocks morph-ball tunneling under the boss while avoiding the old bug
    where the contact box extended below the visible body into the floor.
    """
    del frame_width, frame_height
    body_x = int(round(SMIRKING_BODY_X1))
    body_y = int(round(SMIRKING_BODY_Y1))
    body_w = int(round(SMIRKING_BODY_X2 - SMIRKING_BODY_X1))
    body_h = int(round(SMIRKING_BODY_Y2 - SMIRKING_BODY_Y1))
    feet_x = float(SMIRKING_BODY_FEET_X)
    feet_y = float(SMIRKING_BODY_FEET_Y)
    return {
        "body_pixel_bbox": {"x": body_x, "y": body_y, "w": body_w, "h": body_h},
        "feet_pixel": {"x": feet_x, "y": feet_y},
        "feet_anchor_norm": {"x": 0.0, "y": -0.5},
    }


def _frame_meta(anim: str, frame_idx: int, nframes: int) -> dict:
    g = _body_geometry(anim, frame_idx, nframes)
    anchors = {
        "mouth": {
            "x": round(g["mouth_x"], 2),
            "y": round(g["mouth_y"] + max(2.0, g["mouth_h"] * 0.4), 2),
        },
        "eye": {"x": round(g["eye_x"], 2), "y": round(g["eye_y"], 2)},
        "eye_right": {
            "x": round(g.get("death_eye_right_x", g["eye_x"] + 16.0), 2),
            "y": round(g.get("death_eye_y", g["eye_y"] + 2.0), 2),
        },
        "core": {
            "x": round((g["body_x1"] + g["body_x2"]) * 0.5, 2),
            "y": round((g["body_y1"] + g["body_y2"]) * 0.5, 2),
        },
    }
    return {
        "anchors": anchors,
        "attack": {
            "kind": anim,
        },
    }


def render(out_dir: str | Path, **opts) -> List[Path]:
    del opts
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=_draw_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        frame_meta_fn=_frame_meta,
        auto_crop=True,
        crop_margin=4,
        actor_metadata=ACTOR_METADATA,
        body_metrics_fn=_body_metrics_for_sheet,
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


def render_canonical(out_dir: str | Path, **opts) -> Path:
    del opts
    return write_canonical(
        TARGET_NAME,
        ROWS,
        _draw_frame,
        Path(out_dir),
        frame_size=FRAME_SIZE,
        crop_margin=4,
    )
