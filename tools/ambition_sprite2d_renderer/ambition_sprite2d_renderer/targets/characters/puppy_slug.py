from __future__ import annotations

"""Procedural "puppy slug" enemy sprite sheet.

A late-2010s deep-dream homage: an elongated slug-like body with a
chain of half-formed dog faces budding out of its dorsal ridge.
The hitbox / hurtbox is the whole body. The creature crawls along
floors and (in later animations) clings to walls and ceilings.

The sprite is intentionally a base "shape and weirdness" pass —
fur fractals and full dream-feedback texture are expected to come
from a shader fed this sheet as input. We commit to:

- a readable side silhouette (slug body + segmented dog-head bumps),
- enough internal palette variation that a fractal/noise shader has
  a useful base to perturb,
- jaundiced fur tones + glassy puppy eyes so the creature reads as
  "wrong dog" even before any post-processing.

Animations:
- `idle`:        breathing wobble while stationary.
- `crawl`:       full undulation cycle, body translates left→right
                 within the frame so the loop feels propulsive.
- `wall_crawl`:  same locomotion but the creature has rotated 90°
                 so its belly is against a vertical surface on its
                 LEFT. Used for climbing up a wall on the right
                 side of a room. (Game flips the sprite for the
                 other wall / ceiling — we don't bake those.)
- `ceiling_crawl`: belly up against a ceiling — full 180° flip
                 from `crawl`, but we still render it so the
                 dog-head bumps droop down toward the floor under
                 gravity, which a simple sprite flip wouldn't do.
- `hurt`:        recoil ripple. Body squashes, fur darkens.
- `death`:       deflating melt. The creature loses structure and
                 the dog faces dissolve back into the slug ridge.
"""

import math
from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageColor, ImageDraw, ImageFilter

from ...authoring.tackon_sheet import build_sheet

RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_NAME = "puppy_slug"
SHEET_FILES = [
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
]

ACTOR_METADATA = {
    "actor": {
        "character_id": "npc_puppy_slug",
        "display_name": "Puppy Slug",
    },
    "body": {
        "body_plan": "Crawler",
        "body_kind": "Crawler",
        "mass_class": "Light",
        "locomotion_hint": "Slither",
        "traits": ["enemy", "ai_era", "crawler", "no_hands", "wall_crawler"],
    },
    "capabilities": {
        "traversal": {
            "walk": True,
            "jump": None,
            "climb": True,
            "fly": None,
            "swim": None,
            "crawl": True,
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
    "brain": {"default_preset": "wanderer_puppy_slug"},
    "actions": {"default_preset": "peaceful_slither"},
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "locomotion.walk": {"animation": "walk", "events": []},
        "locomotion.wall_crawl": {"animation": "wall_walk", "events": []},
        "locomotion.ceiling_crawl": {"animation": "ceiling_walk", "events": []},
        "damage.hit": {"animation": "hurt", "events": []},
        "lifecycle.death": {"animation": "death", "events": []},
    },
    "sockets": {
        "mouth": {"source": "puppy_slug.geometry", "point": {"x": 96.0, "y": 48.0}},
        "head": {"source": "puppy_slug.geometry", "point": {"x": 96.0, "y": 40.0}},
        "belly": {"source": "puppy_slug.geometry", "point": {"x": 64.0, "y": 66.0}},
        "tail": {"source": "puppy_slug.geometry", "point": {"x": 28.0, "y": 58.0}},
        "wall_contact": {
            "source": "puppy_slug.geometry",
            "point": {"x": 64.0, "y": 72.0},
        },
    },
    "tags": ["enemy", "ai_era", "crawler"],
}

# Frame size: roomy enough that the dog-face bumps are legible
# (Crawlid-from-Hollow-Knight role: a small ground grunt with
# obvious silhouette readability). The creature still occupies
# only the central ~60% of the frame so wall/ceiling rotations
# fit comfortably. Auto-crop trims to silhouette so callers don't
# pay for the empty margin.
FRAME_SIZE = (128, 96)
SUPER = 4
W, H = FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER

# Row names match what the engine's `CharacterAnim::from_name` table
# accepts: `walk` → `CharacterAnim::Walk` is the locomotion slot the
# enemy animator picks whenever vel.x is non-zero. The `wall_walk`
# and `ceiling_walk` rows are kept in the sheet for a future surface-
# wrapping brain (they'd map to new `CharacterAnim::WallWalk` /
# `CeilingWalk` variants); the runtime currently drops them silently.
ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 140),
    ("walk", 10, 90),
    ("wall_walk", 10, 95),
    ("ceiling_walk", 10, 95),
    ("hurt", 4, 70),
    ("death", 8, 110),
]

# ---- Palette -----------------------------------------------------------------
# Jaundiced fur with a hint of green-blue in the shadows — picked so a
# deep-dream feedback shader has hue room in both directions without
# washing out. Pup faces use a warmer, pinker tone so the bumps read as
# "faces" against the body even at small sizes.
PAL_BODY_DARK = "#3a2a1d"
PAL_BODY_MID = "#7a5a32"
PAL_BODY_LIGHT = "#c79a55"
PAL_BODY_HIGHLIGHT = "#f0d088"
PAL_BELLY = "#d8b58a"
PAL_PUP_FACE = "#b07845"
PAL_PUP_FACE_LIGHT = "#dca87a"
PAL_PUP_SNOUT = "#3e2218"
PAL_PUP_NOSE = "#1a0e08"
PAL_EYE_WHITE = "#f6efd9"
PAL_EYE_IRIS = "#6a3a14"
PAL_EYE_PUPIL = "#0c0604"
PAL_SLIME = "#a8c46a"
PAL_OUTLINE = "#1a0f08"


def _rgba(color: str, alpha: int = 255) -> RGBA:
    r, g, b = ImageColor.getrgb(color)
    return (r, g, b, alpha)


def _s(v: float) -> int:
    return int(round(v * SUPER))


def _pt(x: float, y: float) -> Tuple[int, int]:
    return (_s(x), _s(y))


def _box(x1: float, y1: float, x2: float, y2: float) -> Tuple[int, int, int, int]:
    return (_s(x1), _s(y1), _s(x2), _s(y2))


def _downsample(img: Image.Image) -> Image.Image:
    return img.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


# ---- Geometry helpers --------------------------------------------------------


def _body_centerline(
    cx: float,
    cy: float,
    length: float,
    n: int,
    phase: float,
    wave_amp: float,
    wave_freq: float,
    pitch: float = 0.0,
) -> List[Point]:
    """Sample (x, y) along the slug centerline.

    The body is laid out left→right around `cx, cy`. `phase` shifts
    the wave so successive frames feel like the same body has moved
    along itself (propulsive undulation rather than a wobble in
    place). `pitch` tilts the whole line — used to make wall-crawl
    angle into the wall.
    """
    points: List[Point] = []
    half = length / 2.0
    cos_p = math.cos(pitch)
    sin_p = math.sin(pitch)
    for i in range(n):
        t = i / (n - 1)
        # Local (un-pitched) coords: x runs head→tail, y waves.
        lx = (t - 0.5) * length
        wave = math.sin(t * wave_freq * math.tau + phase) * wave_amp
        # Taper the wave at head + tail so the ends still feel anchored.
        edge_taper = math.sin(t * math.pi)
        ly = wave * (0.35 + 0.65 * edge_taper)
        # Apply pitch rotation around the body center.
        rx = lx * cos_p - ly * sin_p
        ry = lx * sin_p + ly * cos_p
        points.append((cx + rx, cy + ry))
    _ = half  # `length/2` is implicit in the parameterisation; keep symbol for readability.
    return points


def _segment_radius(t: float, base: float) -> float:
    """Body radius profile along the slug (t in [0, 1] head→tail)."""
    # Bulbous head, thicker mid, tapered tail. Three-lobed envelope
    # so a feedback shader has visible thickness gradients to chew on.
    head_lobe = math.exp(-((t - 0.06) ** 2) / 0.012)
    mid_lobe = math.exp(-((t - 0.55) ** 2) / 0.08)
    tail_taper = max(0.05, 1.0 - max(0.0, (t - 0.85) / 0.15) ** 2)
    return base * (0.65 + 0.55 * head_lobe + 0.40 * mid_lobe) * tail_taper


def _ring_points(
    center: Point, radius: float, normal: Point, n: int = 14
) -> List[Point]:
    """Approximate an ellipse perpendicular to `normal` for body shading."""
    nx, ny = normal
    # Tangent perpendicular to normal in 2D.
    tx, ty = -ny, nx
    pts = []
    for k in range(n):
        a = k / n * math.tau
        # Squashed perpendicular to the body — long axis along tangent,
        # short axis pinched along normal so it reads as a rim, not a circle.
        rx = math.cos(a) * radius * 1.0
        ry = math.sin(a) * radius * 0.55
        pts.append((center[0] + rx * tx + ry * nx, center[1] + rx * ty + ry * ny))
    return pts


# ---- Body drawing ------------------------------------------------------------


def _draw_slime_trail(
    img: Image.Image,
    centerline: List[Point],
    pitch: float,
    trail_strength: float,
) -> None:
    """Wet sheen under the belly. Strength is dialed by anim state."""
    if trail_strength <= 0.01:
        return
    layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer, "RGBA")
    # The "down" relative to body pitch.
    dx = math.sin(pitch)
    dy = math.cos(pitch)
    points = []
    for i, (x, y) in enumerate(centerline):
        t = i / (len(centerline) - 1)
        r = _segment_radius(t, base=9.0)
        bx = x + dx * (r - 1.0)
        by = y + dy * (r - 1.0)
        points.append(_pt(bx, by))
    if len(points) >= 2:
        d.line(points, fill=_rgba(PAL_SLIME, int(150 * trail_strength)), width=_s(2.4))
    layer = layer.filter(ImageFilter.GaussianBlur(radius=_s(0.9)))
    img.alpha_composite(layer)


def _draw_body(
    img: Image.Image,
    centerline: List[Point],
    pitch: float,
    sag: float,
    fur_phase: float,
    death_progress: float = 0.0,
) -> None:
    """Render the slug body as overlapping ellipses with rim shading.

    Drawing as a chain of stacked ellipses (rather than one polygon)
    gives natural mid-body bulges and lets a downstream shader bite
    into the silhouette per-segment.
    """
    n = len(centerline)
    base_r = 10.0
    # Slight droop perpendicular to pitch direction (gravity).
    grav_x = math.sin(pitch + math.pi / 2.0)
    grav_y = math.cos(pitch + math.pi / 2.0)

    body_layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
    bd = ImageDraw.Draw(body_layer, "RGBA")

    # Pass 1: dark base silhouette (a chubby outline).
    for i, (x, y) in enumerate(centerline):
        t = i / (n - 1)
        r = _segment_radius(t, base_r) * (1.0 - 0.35 * death_progress)
        ox = x + grav_x * sag * 0.45 * math.sin(t * math.pi)
        oy = y + grav_y * sag * 0.45 * math.sin(t * math.pi)
        bd.ellipse(
            _box(ox - r - 0.8, oy - r - 0.8, ox + r + 0.8, oy + r + 0.8),
            fill=_rgba(PAL_OUTLINE),
        )

    # Pass 2: mid-tone fill.
    for i, (x, y) in enumerate(centerline):
        t = i / (n - 1)
        r = _segment_radius(t, base_r) * (1.0 - 0.35 * death_progress)
        ox = x + grav_x * sag * 0.45 * math.sin(t * math.pi)
        oy = y + grav_y * sag * 0.45 * math.sin(t * math.pi)
        bd.ellipse(_box(ox - r, oy - r, ox + r, oy + r), fill=_rgba(PAL_BODY_MID))

    # Pass 3: dorsal highlight ridge. Offset opposite gravity.
    for i, (x, y) in enumerate(centerline):
        t = i / (n - 1)
        r = _segment_radius(t, base_r) * (1.0 - 0.35 * death_progress)
        hx = x - grav_x * r * 0.45
        hy = y - grav_y * r * 0.45
        rr = r * 0.55
        bd.ellipse(
            _box(hx - rr, hy - rr * 0.7, hx + rr, hy + rr * 0.7),
            fill=_rgba(PAL_BODY_LIGHT),
        )

    # Pass 4: belly band — lighter tone hugging the gravity-down side.
    for i, (x, y) in enumerate(centerline):
        t = i / (n - 1)
        r = _segment_radius(t, base_r) * (1.0 - 0.35 * death_progress)
        bx = x + grav_x * r * 0.55
        by = y + grav_y * r * 0.55
        rr = r * 0.40
        bd.ellipse(
            _box(bx - rr, by - rr * 0.55, bx + rr, by + rr * 0.55),
            fill=_rgba(PAL_BELLY, 230),
        )

    # Pass 5: fur tufts along the dorsal ridge. Short angled strokes.
    # These look intentional (not random) so a follow-up shader can
    # find them as base structure to amplify.
    for i in range(2, n - 2):
        t = i / (n - 1)
        x, y = centerline[i]
        # Tangent for tuft orientation.
        px, py = centerline[i - 1]
        nx2, ny2 = centerline[i + 1]
        tx = nx2 - px
        ty = ny2 - py
        tl = math.hypot(tx, ty) or 1.0
        tx /= tl
        ty /= tl
        # Tuft sticks up opposite gravity.
        ux = -grav_x
        uy = -grav_y
        r = _segment_radius(t, base_r)
        # Vary tuft length by phase + index so the row feels organic.
        wobble = 0.6 + 0.4 * math.sin(fur_phase + i * 0.9)
        L = r * 0.7 * wobble * (1.0 - death_progress * 0.8)
        ax = x + ux * (r * 0.55) + tx * 1.2
        ay = y + uy * (r * 0.55) + ty * 1.2
        bx = ax + ux * L + tx * 0.6
        by = ay + uy * L + ty * 0.6
        bd.line([_pt(ax, ay), _pt(bx, by)], fill=_rgba(PAL_BODY_DARK), width=_s(1.1))

    img.alpha_composite(body_layer)


# ---- Puppy heads -------------------------------------------------------------


def _draw_pup_head(
    img: Image.Image,
    center: Point,
    radius: float,
    facing: float,
    eye_open: float,
    pitch: float,
    melt: float = 0.0,
) -> None:
    """Draw one of the dog faces budding out of the dorsal ridge.

    `facing` is the angle (radians) the snout points; the eyes look
    along the same direction. `melt` collapses the head back into
    the body for the death animation.
    """
    if melt >= 0.98:
        return
    cx, cy = center
    layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer, "RGBA")

    head_w = radius * 2.4 * (1.0 - 0.6 * melt)
    head_h = radius * 2.1 * (1.0 - 0.6 * melt)

    # Snout direction.
    sx = math.cos(facing)
    sy = math.sin(facing)
    # Perpendicular for ears.
    px = -sy
    py = sx

    # Skull base.
    d.ellipse(
        _box(
            cx - head_w / 2 - 0.5,
            cy - head_h / 2 - 0.5,
            cx + head_w / 2 + 0.5,
            cy + head_h / 2 + 0.5,
        ),
        fill=_rgba(PAL_OUTLINE),
    )
    d.ellipse(
        _box(cx - head_w / 2, cy - head_h / 2, cx + head_w / 2, cy + head_h / 2),
        fill=_rgba(PAL_PUP_FACE),
    )

    # Cheek highlight.
    d.ellipse(
        _box(
            cx - head_w * 0.3,
            cy - head_h * 0.10,
            cx + head_w * 0.18,
            cy + head_h * 0.32,
        ),
        fill=_rgba(PAL_PUP_FACE_LIGHT, 200),
    )

    if melt < 0.5:
        # Ears: two angled drops above the head. They use the
        # "up" direction (opposite gravity, modulated by snout).
        up_x = -math.sin(pitch)
        up_y = -math.cos(pitch)
        ear_base_left = (
            cx + (px * 0.55 - up_x * 0.05) * head_w * 0.30,
            cy + (py * 0.55 - up_y * 0.05) * head_h * 0.30,
        )
        ear_tip_left = (
            ear_base_left[0] + up_x * head_h * 0.55 + px * head_w * 0.15,
            ear_base_left[1] + up_y * head_h * 0.55 + py * head_h * 0.15,
        )
        ear_base_right = (
            cx + (-px * 0.55 - up_x * 0.05) * head_w * 0.30,
            cy + (-py * 0.55 - up_y * 0.05) * head_h * 0.30,
        )
        ear_tip_right = (
            ear_base_right[0] + up_x * head_h * 0.55 - px * head_w * 0.15,
            ear_base_right[1] + up_y * head_h * 0.55 - py * head_h * 0.15,
        )
        for base, tip in (
            (ear_base_left, ear_tip_left),
            (ear_base_right, ear_tip_right),
        ):
            tri = [
                (base[0] - px * head_w * 0.08, base[1] - py * head_w * 0.08),
                (base[0] + px * head_w * 0.08, base[1] + py * head_w * 0.08),
                tip,
            ]
            d.polygon(
                [_pt(*p) for p in tri],
                fill=_rgba(PAL_BODY_DARK),
                outline=_rgba(PAL_OUTLINE),
            )

    # Snout — fat oval extending from the face center along `facing`.
    if melt < 0.7:
        snout_cx = cx + sx * head_w * 0.40
        snout_cy = cy + sy * head_w * 0.40
        snout_r = head_w * 0.32 * (1.0 - melt)
        d.ellipse(
            _box(
                snout_cx - snout_r,
                snout_cy - snout_r * 0.75,
                snout_cx + snout_r,
                snout_cy + snout_r * 0.75,
            ),
            fill=_rgba(PAL_PUP_SNOUT),
            outline=_rgba(PAL_OUTLINE),
        )
        # Wet nose tip.
        nose_cx = snout_cx + sx * snout_r * 0.55
        nose_cy = snout_cy + sy * snout_r * 0.55
        nr = snout_r * 0.30
        d.ellipse(
            _box(nose_cx - nr, nose_cy - nr * 0.85, nose_cx + nr, nose_cy + nr * 0.85),
            fill=_rgba(PAL_PUP_NOSE),
        )
        # Mouth seam.
        m_a = (snout_cx - sx * snout_r * 0.05, snout_cy - sy * snout_r * 0.05)
        m_b = (
            snout_cx + sx * snout_r * 0.55 + px * snout_r * 0.10,
            snout_cy + sy * snout_r * 0.55 + py * snout_r * 0.10,
        )
        d.line([_pt(*m_a), _pt(*m_b)], fill=_rgba(PAL_OUTLINE), width=_s(0.8))

    # Eyes — two glassy beads. Eye-open lerps the pupil to a slit.
    if melt < 0.4:
        # Eye centers offset perpendicular to facing (sides of the muzzle).
        for sign in (-1.0, 1.0):
            ex = cx + sx * head_w * 0.05 + px * sign * head_w * 0.22
            ey = cy + sy * head_w * 0.05 + py * sign * head_w * 0.22
            er = head_w * 0.13
            d.ellipse(
                _box(ex - er, ey - er, ex + er, ey + er), fill=_rgba(PAL_EYE_WHITE)
            )
            ir = er * 0.65 * eye_open
            if ir > 0.01:
                # Iris drifts slightly toward snout direction.
                ix = ex + sx * er * 0.20
                iy = ey + sy * er * 0.20
                d.ellipse(
                    _box(ix - ir, iy - ir, ix + ir, iy + ir), fill=_rgba(PAL_EYE_IRIS)
                )
                pr = ir * 0.55
                d.ellipse(
                    _box(ix - pr, iy - pr, ix + pr, iy + pr), fill=_rgba(PAL_EYE_PUPIL)
                )
            else:
                # Closed-eye slit.
                d.line(
                    [_pt(ex - er * 0.8, ey), _pt(ex + er * 0.8, ey)],
                    fill=_rgba(PAL_EYE_PUPIL),
                    width=_s(0.8),
                )

    img.alpha_composite(layer)


def _pup_heads_along(
    centerline: List[Point],
    pitch: float,
    count: int,
    phase: float,
    head_scale: float = 1.0,
) -> List[Tuple[Point, float, float]]:
    """Pick mounting points on the dorsal ridge for pup heads.

    Returns a list of (center, radius, facing_angle) tuples.
    Heads are spaced evenly along the body but skip the very tail.
    """
    n = len(centerline)
    grav_x = math.sin(pitch + math.pi / 2.0)
    grav_y = math.cos(pitch + math.pi / 2.0)
    heads: List[Tuple[Point, float, float]] = []
    # Mount fractions along the body — keep heads out of the tail
    # and spread them across the front 75% so they overlap and feel
    # crowded (deep-dream "more faces than makes sense").
    fracs = [0.08 + i * (0.72 / max(1, count - 1)) for i in range(count)]
    for j, frac in enumerate(fracs):
        idx = int(frac * (n - 1))
        cx, cy = centerline[idx]
        t = idx / (n - 1)
        r = _segment_radius(t, base=10.0)
        # Each head bumps OPPOSITE gravity so they ride the dorsal ridge.
        # Plus a small wobble so they don't sit in a dead straight line.
        wobble = math.sin(phase * 2.0 + j * 1.2) * 1.2
        # Heads only lift ~half the body radius so they sit IN the
        # ridge, not on stalks. With the bigger head radius below,
        # this lets the heads visually merge with the body — the
        # whole dorsal line reads as a chain of fused faces.
        bump = r * 0.45 + wobble
        hx = cx - grav_x * bump
        hy = cy - grav_y * bump
        # Facing: lean slightly toward the head end of the slug (i.e. left),
        # blended with phase so heads turn as the body undulates. Wall-crawl
        # rotates this naturally through `pitch`.
        forward = math.atan2(-grav_y, -grav_x) - math.pi / 2.0  # along body, head-ward
        facing = forward + math.sin(phase + j * 0.8) * 0.35
        # Heads are MEANT to dominate — the silhouette of a puppy
        # slug is "row of dog faces on a tube," not "tube with
        # decorations." So head_r is comparable to body radius.
        head_r = (6.5 + 1.0 * math.sin(j * 1.3)) * head_scale
        heads.append(((hx, hy), head_r, facing))
    return heads


# ---- Per-animation params ----------------------------------------------------


def _params_for(anim: str, frame_idx: int, nframes: int):
    """Drive every per-frame variable from anim + frame."""
    t = frame_idx / max(1, nframes)
    tau = math.tau

    # Defaults: body sits horizontally, head end at left.
    cx = FRAME_SIZE[0] / 2.0
    cy = FRAME_SIZE[1] * 0.62
    # Shorter aspect ratio than a long slug — fat caterpillar feel.
    length = FRAME_SIZE[0] * 0.80
    pitch = 0.0
    wave_amp = 0.7
    wave_freq = 1.6
    phase = t * tau
    fur_phase = t * tau * 0.6
    sag = 0.0
    eye_open = 1.0
    trail_strength = 0.35
    head_count = 2
    head_phase = phase
    head_scale = 1.0
    death_progress = 0.0
    body_translate = 0.0  # extra X translation applied to whole creature

    if anim == "idle":
        wave_amp = 0.6
        wave_freq = 1.2
        phase = math.sin(t * tau) * 0.7
        head_phase = math.sin(t * tau) * 0.5
        trail_strength = 0.25
        eye_open = 1.0
        sag = 0.6 + 0.4 * math.sin(t * tau)
    elif anim == "walk":
        wave_amp = 2.2
        wave_freq = 2.0
        # Body translates left→right within the frame across the loop —
        # makes the sheet feel like the slug is actually crawling
        # when previewed end-to-end. The game can ignore the
        # translation by anchoring on the body center.
        body_translate = -7.0 + 14.0 * t
        trail_strength = 0.55
        head_phase = phase
    elif anim == "wall_walk":
        # Belly on a vertical wall to the LEFT of the creature.
        # Rotate the entire body so head is up (-Y) and tail down (+Y).
        pitch = -math.pi / 2.0
        cx = FRAME_SIZE[0] * 0.62
        cy = FRAME_SIZE[1] / 2.0
        length = FRAME_SIZE[1] * 0.85
        wave_amp = 1.8
        wave_freq = 2.0
        # Translate the creature up the wall over the loop.
        body_translate = 0.0
        trail_strength = 0.45
    elif anim == "ceiling_walk":
        # Upside-down. Gravity now sags the heads downward (toward floor).
        pitch = math.pi
        wave_amp = 2.0
        wave_freq = 2.0
        body_translate = 7.0 - 14.0 * t
        trail_strength = 0.40
        sag = 1.3  # heads droop visibly
    elif anim == "hurt":
        # Sharp squash + fur darkens.
        squash = math.sin(t * math.pi)
        wave_amp = 0.4
        wave_freq = 1.0
        length *= 1.0 - 0.18 * squash
        sag = 1.8 * squash
        trail_strength = 0.15
        eye_open = max(0.1, 1.0 - 1.5 * squash)
        head_count = 2
    elif anim == "death":
        # Deflating melt — body flattens, heads dissolve.
        death_progress = min(1.0, t * 1.15)
        wave_amp = 0.6 * (1.0 - death_progress)
        wave_freq = 1.4
        length *= 1.0 - 0.10 * death_progress
        sag = 2.6 + 3.5 * death_progress
        trail_strength = 0.6 * (1.0 - death_progress)
        eye_open = max(0.0, 1.0 - 2.2 * death_progress)
        head_count = max(1, int(round(2 * (1.0 - death_progress))))
        head_scale = max(0.2, 1.0 - 0.6 * death_progress)
    else:
        raise ValueError(f"unknown animation: {anim}")

    return {
        "cx": cx + body_translate * math.cos(pitch),
        "cy": cy + body_translate * math.sin(pitch),
        "length": length,
        "pitch": pitch,
        "wave_amp": wave_amp,
        "wave_freq": wave_freq,
        "phase": phase,
        "fur_phase": fur_phase,
        "sag": sag,
        "eye_open": eye_open,
        "trail_strength": trail_strength,
        "head_count": head_count,
        "head_phase": head_phase,
        "head_scale": head_scale,
        "death_progress": death_progress,
    }


# ---- Renderer ----------------------------------------------------------------


def _render_internal(p: dict) -> Image.Image:
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))

    centerline = _body_centerline(
        cx=p["cx"],
        cy=p["cy"],
        length=p["length"],
        n=28,
        phase=p["phase"],
        wave_amp=p["wave_amp"],
        wave_freq=p["wave_freq"],
        pitch=p["pitch"],
    )

    _draw_slime_trail(img, centerline, p["pitch"], p["trail_strength"])
    _draw_body(
        img,
        centerline,
        pitch=p["pitch"],
        sag=p["sag"],
        fur_phase=p["fur_phase"],
        death_progress=p["death_progress"],
    )

    heads = _pup_heads_along(
        centerline,
        pitch=p["pitch"],
        count=p["head_count"],
        phase=p["head_phase"],
        head_scale=p["head_scale"],
    )
    for center, radius, facing in heads:
        _draw_pup_head(
            img,
            center=center,
            radius=radius,
            facing=facing,
            eye_open=p["eye_open"],
            pitch=p["pitch"],
            melt=p["death_progress"],
        )

    return _downsample(img)


def render_frame(animation: str, frame_idx: int, nframes: int) -> Image.Image:
    p = _params_for(animation, frame_idx, nframes)
    return _render_internal(p)


def render(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=render_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        label_width=120,
        actor_metadata=ACTOR_METADATA,
    )
    return [
        outputs["canonical"],
        outputs["canonical_transparent"],
        outputs["spritesheet"],
        outputs["yaml"],
        outputs["ron"],
        outputs["actor"],
        outputs["preview"],
    ]
