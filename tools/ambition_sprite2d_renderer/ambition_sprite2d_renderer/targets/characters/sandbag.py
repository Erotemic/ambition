#!/usr/bin/env python3
"""Sandbag target for the Ambition 2D sprite renderer.

Sandbag now participates in the shared adapter/YAML pipeline while keeping the
old sparse tack-on renderer for compatibility.  The sparse target still emits
only idle/hit/death, but the adapter surface can render the standard character
rows and a handful of extended review rows so new dummy/variant characters can
share the same sheet generation code as robot, goblin, and boss.

Invoked through the parent CLI:

    python -m ambition_sprite2d_renderer render sandbag
    python -m ambition_sprite2d_renderer render-publish sandbag

The old ``render``/``install`` commands remain as a stable compatibility shim.
"""

from __future__ import annotations

import math
from dataclasses import asdict, dataclass

from ...authoring.actor_contract import write_actor_contract_for_tackon
from ...authoring.animation_vocab import DEFAULT_ADVANCED_TIMINGS
from pathlib import Path
from typing import Dict, List, Tuple

ACTOR_METADATA = {
    "actor": {"character_id": "sandbag", "display_name": "Sandbag"},
    "body": {
        "body_plan": "TrainingDummy",
        "body_kind": "Standard",
        "mass_class": "Static",
        "traits": ["training", "dummy"],
    },
    "capabilities": {
        "traversal": {
            "walk": False,
            "jump": None,
            "climb": None,
            "fly": None,
            "swim": None,
            "crawl": None,
            "use_lifts": None,
            "door_access": [],
        },
        "interactions": {"talk": None, "trade": None, "carry": None, "open_doors": []},
    },
    "brain": {"default_preset": "stand_still"},
    "actions": {"default_preset": "sandbag_punch"},
    "visual": {"default_pose": "idle"},
    "tags": ["training", "dummy"],
    "sockets": {
        "center": {"source": "explicit.profile.dummy", "point": {"x": 64.0, "y": 64.0}},
        "top": {"source": "explicit.profile.dummy", "point": {"x": 64.0, "y": 18.0}},
    },
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "damage.hit": {"animation": "hit", "events": []},
    },
}


try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError as ex:  # pragma: no cover
    raise SystemExit(
        "This generator needs Pillow. Install with: python -m pip install pillow"
    ) from ex

RGBA = Tuple[int, int, int, int]

FRAME_W = 128
FRAME_H = 128
LABEL_W = 100
SCALE = 4


@dataclass(frozen=True)
class SandbagSpec:
    target: str = "sandbag"
    seed: int = 0
    archetype: str = "training_dummy"
    variant: str = "classic"
    palette_name: str = "pale_cloth"

    def to_dict(self) -> Dict[str, object]:
        return asdict(self)


ADAPTER_ANIMATIONS: Dict[str, Dict[str, int]] = {
    "idle": {"frames": 6, "duration_ms": 120},
    "walk": {"frames": 8, "duration_ms": 105},
    "run": {"frames": 8, "duration_ms": 82},
    "jump": {"frames": 6, "duration_ms": 95},
    "fall": {"frames": 6, "duration_ms": 95},
    "slash": {"frames": 6, "duration_ms": 82},
    "hit": {"frames": 4, "duration_ms": 75},
    "death": {"frames": 7, "duration_ms": 112},
    "blink_out": {"frames": 6, "duration_ms": 62},
    "blink_in": {"frames": 6, "duration_ms": 62},
    "dash": {"frames": 6, "duration_ms": 70},
    "crouch": {"frames": 5, "duration_ms": 95},
    "wall_slide": {"frames": 6, "duration_ms": 96},
    "wall_jump": {"frames": 6, "duration_ms": 86},
    "ledge_grab": {"frames": 6, "duration_ms": 105},
    "climb": {"frames": 8, "duration_ms": 100},
    "swim": {"frames": 8, "duration_ms": 105},
    "interact": {"frames": 6, "duration_ms": 90},
    "talk": {"frames": 8, "duration_ms": 110},
    "block": {"frames": 6, "duration_ms": 85},
    **DEFAULT_ADVANCED_TIMINGS,
}

# Native rows for the sparse tack-on output. Do not pad with walk/run/etc. —
# CharacterSheetSpec maps missing rows to idle on demand at runtime.
SANDBAG_ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 120),
    ("hit", 4, 75),
    ("death", 7, 112),
]


def _s(v: float) -> int:
    return int(round(v * SCALE))


def _box(x1: float, y1: float, x2: float, y2: float) -> Tuple[int, int, int, int]:
    return (_s(x1), _s(y1), _s(x2), _s(y2))


def _rgba(hex_color: str, alpha: int = 255) -> RGBA:
    hex_color = hex_color.lstrip("#")
    return (
        int(hex_color[0:2], 16),
        int(hex_color[2:4], 16),
        int(hex_color[4:6], 16),
        alpha,
    )


def _font(size: int = 12):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size=size)
        except OSError:
            pass
    return ImageFont.load_default()


def _dashed_line(
    draw: ImageDraw.ImageDraw,
    xy: Tuple[float, float, float, float],
    fill: RGBA,
    width: float = 1.0,
    dash: float = 4.0,
) -> None:
    x1, y1, x2, y2 = xy
    total = math.hypot(x2 - x1, y2 - y1)
    if total <= 0:
        return
    steps = max(1, int(total / dash))
    for k in range(0, steps, 2):
        a = k / steps
        b = min(1.0, (k + 1) / steps)
        xa, ya = x1 + (x2 - x1) * a, y1 + (y2 - y1) * a
        xb, yb = x1 + (x2 - x1) * b, y1 + (y2 - y1) * b
        draw.line((_s(xa), _s(ya), _s(xb), _s(yb)), fill=fill, width=max(1, _s(width)))


def _draw_eye(
    draw: ImageDraw.ImageDraw,
    cx: float,
    cy: float,
    *,
    scale_y: float = 1.0,
    expression: str = "normal",
) -> None:
    dark = _rgba("14131d")
    hi = _rgba("f7f8ff", 235)
    if expression == "x":
        for dx in (-4, 4):
            draw.line(
                (_s(cx - dx), _s(cy - 4), _s(cx + dx), _s(cy + 4)),
                fill=dark,
                width=_s(2),
            )
        return
    if expression == "sleepy":
        draw.line((_s(cx - 5), _s(cy), _s(cx + 5), _s(cy + 1)), fill=dark, width=_s(2))
        return
    if expression == "squint":
        draw.arc(_box(cx - 5, cy - 4, cx + 5, cy + 5), 18, 162, fill=dark, width=_s(2))
        return
    draw.rounded_rectangle(
        _box(cx - 4.6, cy - 12.0 * scale_y, cx + 4.6, cy + 11.0 * scale_y),
        radius=_s(4.2),
        fill=dark,
    )
    draw.ellipse(
        _box(cx - 2.1, cy - 8.5 * scale_y, cx + 1.0, cy - 4.0 * scale_y), fill=hi
    )


def _draw_sandbag_body(
    layer: Image.Image,
    *,
    cx: float,
    cy: float,
    sx: float,
    sy: float,
    eyes: str = "normal",
    tint: float = 1.0,
    strap_swing: float = 0.0,
) -> None:
    """Draw the original pale-cloth sandbag character.

    The art intentionally rhymes with familiar platform-fighter sandbags:
    pale sack, oval eyes, stitched top/bottom bands, and a small top strap.
    Proportions, strap shape, seam layout, and shading are distinct from the
    reference so this remains an original procedural asset.
    """
    draw = ImageDraw.Draw(layer, "RGBA")

    def tone(rgb: Tuple[int, int, int], alpha: int = 255) -> RGBA:
        return tuple(max(0, min(255, int(v * tint))) for v in rgb) + (alpha,)

    cloth = tone((225, 227, 242))
    cloth_mid = tone((204, 207, 226))
    cloth_shadow = tone((163, 166, 187))
    cloth_dark = tone((99, 101, 120))
    stitch = tone((75, 77, 96))
    highlight = tone((247, 248, 255), 185)

    w = 44 * sx
    h = 76 * sy
    top = cy - h / 2
    bottom = cy + h / 2
    left = cx - w / 2
    right = cx + w / 2

    # Main sack silhouette: a soft, slightly asymmetric rounded body, not a
    # perfect cylinder. The lower half bulges a bit more than the top.
    draw.rounded_rectangle(
        _box(left + 1, top + 5, right - 1, bottom - 2),
        radius=_s(20),
        fill=cloth_mid,
        outline=stitch,
        width=_s(2.0),
    )
    draw.rounded_rectangle(
        _box(left + 4, top + 8, right - 7, bottom - 5),
        radius=_s(18),
        fill=cloth,
        width=0,
    )

    # Side shading and a belly highlight make the simple shape read as stuffed
    # cloth without copying any exact source highlights.
    draw.pieslice(
        _box(right - 20, top + 9, right + 10, bottom - 2), 84, 276, fill=cloth_shadow
    )
    draw.pieslice(
        _box(left + 4, top + 19, right - 12, bottom - 15), 105, 258, fill=highlight
    )
    draw.arc(
        _box(right - 17, top + 14, right + 1, bottom - 6),
        78,
        278,
        fill=cloth_dark,
        width=_s(1.3),
    )

    # Top and bottom stitched caps.
    draw.ellipse(
        _box(left + 2, top - 2, right - 2, top + 21),
        fill=tone((235, 237, 249)),
        outline=stitch,
        width=_s(1.6),
    )
    draw.arc(
        _box(left + 5, top + 3, right - 5, top + 19),
        10,
        172,
        fill=highlight,
        width=_s(1.0),
    )
    draw.arc(
        _box(left + 1, bottom - 20, right - 1, bottom + 1),
        10,
        170,
        fill=stitch,
        width=_s(1.5),
    )
    draw.arc(
        _box(left + 4, bottom - 17, right - 4, bottom - 2),
        13,
        169,
        fill=cloth_shadow,
        width=_s(3.0),
    )

    # Stitches on the cap bands. Small dashed arcs are enough at gameplay scale.
    for i in range(12):
        t = i / 11
        x = left + 7 + (w - 14) * t
        y_top = top + 17 + math.sin(t * math.pi) * 2.2
        y_bot = bottom - 9 + math.sin(t * math.pi) * 2.0
        draw.line(
            (_s(x - 1.5), _s(y_top), _s(x + 1.4), _s(y_top + 0.5)),
            fill=cloth_dark,
            width=_s(0.9),
        )
        draw.line(
            (_s(x - 1.6), _s(y_bot), _s(x + 1.5), _s(y_bot + 0.4)),
            fill=cloth_dark,
            width=_s(0.9),
        )

    # Hanging tab/strap: same visual vocabulary as the reference, but shorter,
    # wider, and attached at a different angle with an inset patch.
    strap_x = right - 9 + strap_swing
    strap_y = top + 3
    draw.rounded_rectangle(
        _box(strap_x - 5, strap_y - 1, strap_x + 8, strap_y + 28),
        radius=_s(4),
        fill=cloth_mid,
        outline=stitch,
        width=_s(1.4),
    )
    draw.rounded_rectangle(
        _box(strap_x - 2, strap_y + 4, strap_x + 5, strap_y + 20),
        radius=_s(2),
        fill=cloth,
        width=0,
    )
    draw.line(
        (_s(strap_x - 3), _s(strap_y + 23), _s(strap_x + 7), _s(strap_y + 22)),
        fill=cloth_dark,
        width=_s(1.1),
    )

    # A few cloth wrinkles, deliberately sparse.
    draw.arc(
        _box(left + 5, cy - 17, right - 11, cy + 1),
        192,
        340,
        fill=tone((150, 153, 175), 96),
        width=_s(1.0),
    )
    draw.arc(
        _box(left + 3, cy + 5, right - 8, cy + 24),
        195,
        342,
        fill=tone((150, 153, 175), 90),
        width=_s(1.0),
    )
    _dashed_line(
        draw,
        (left + 10, top + 28, left + 7, bottom - 19),
        tone((115, 117, 138), 110),
        width=0.8,
        dash=5,
    )

    # Face. The eyes are the immediately recognizable rhyme; placement,
    # spacing, and highlights differ from the reference.
    eye_y = cy - 10 * sy
    if eyes == "x":
        _draw_eye(draw, cx - 10 * sx, eye_y, expression="x")
        _draw_eye(draw, cx + 10 * sx, eye_y, expression="x")
    elif eyes == "sleepy":
        _draw_eye(draw, cx - 10 * sx, eye_y + 1, expression="sleepy")
        _draw_eye(draw, cx + 10 * sx, eye_y + 1, expression="sleepy")
    elif eyes == "squint":
        _draw_eye(draw, cx - 10 * sx, eye_y, expression="squint")
        _draw_eye(draw, cx + 10 * sx, eye_y, expression="squint")
    else:
        _draw_eye(draw, cx - 10 * sx, eye_y, scale_y=sy)
        _draw_eye(draw, cx + 10 * sx, eye_y, scale_y=sy)


def _impact_marks(canvas: Image.Image, frame_index: int) -> None:
    draw = ImageDraw.Draw(canvas, "RGBA")
    alpha = max(50, 215 - frame_index * 45)
    yellow = _rgba("ffe56f", alpha)
    orange = _rgba("ff8740", alpha)
    cx, cy = 29 + frame_index * 2, 58 - frame_index * 2
    rays = [(-13, 0), (13, 0), (0, -12), (0, 12), (-9, -8), (9, 8), (-9, 8), (9, -8)]
    for dx, dy in rays:
        draw.line(
            (_s(cx), _s(cy), _s(cx + dx), _s(cy + dy)),
            fill=yellow if abs(dx) + abs(dy) > 14 else orange,
            width=_s(2.0),
        )
    for k in range(3):
        draw.line(
            (_s(36 + k * 11), _s(47 + k * 12), _s(55 + k * 9), _s(47 + k * 12)),
            fill=_rgba("9ba0ba", max(0, alpha - 75)),
            width=_s(1.0),
        )


def _dust(
    canvas: Image.Image, frame_index: int, base_x: float = 68.0, base_y: float = 112.0
) -> None:
    draw = ImageDraw.Draw(canvas, "RGBA")
    for i in range(5):
        x = base_x - 20 + i * 10 + frame_index * (1.4 - i * 0.25)
        y = base_y + (i % 2) * 3 - frame_index * 0.45
        r = 1.8 + (i % 3)
        alpha = max(0, 92 - frame_index * 14)
        draw.ellipse(_box(x - r, y - r, x + r, y + r), fill=_rgba("9f937f", alpha))


def render_frame(animation: str, frame_index: int, frame_count: int) -> Image.Image:
    canvas = Image.new("RGBA", (FRAME_W * SCALE, FRAME_H * SCALE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(canvas, "RGBA")

    phase = math.sin((frame_index / max(1, frame_count)) * math.tau)
    cx = 65.0
    cy = 71.0
    sx = 1.0
    sy = 1.0
    angle = 0.0
    eyes = "normal"
    tint = 1.0
    strap_swing = phase * 0.8
    shadow_w = 45.0
    shadow_a = 70

    if animation == "idle":
        cy += phase * 1.5
        sx = 1.0 - phase * 0.018
        sy = 1.0 + phase * 0.028
        angle = phase * 1.4
        shadow_w = 45 + phase * 2.0
    elif animation == "walk":
        cy += abs(phase) * 1.0
        cx += phase * 2.5
        angle = phase * 3.0
        strap_swing = -phase * 2.2
        shadow_w = 45 + abs(phase) * 4.0
    elif animation == "run":
        cy += abs(phase) * 1.7
        cx += phase * 4.2
        sx = 1.0 + abs(phase) * 0.025
        sy = 1.0 - abs(phase) * 0.018
        angle = -7.0 + phase * 4.0
        strap_swing = -phase * 3.5
        shadow_w = 49 + abs(phase) * 5.0
    elif animation == "jump":
        arc = math.sin((frame_index / max(1, frame_count - 1)) * math.pi)
        cy -= 20.0 * arc
        cx += 4.0 * frame_index / max(1, frame_count - 1)
        sx = 0.96 + 0.05 * arc
        sy = 1.08 - 0.04 * arc
        angle = -8.0 + 10.0 * frame_index / max(1, frame_count - 1)
        eyes = "squint"
        shadow_w = 36 + 7 * (1.0 - arc)
        shadow_a = 48
    elif animation == "fall":
        t = frame_index / max(1, frame_count - 1)
        cy -= 12.0 - 17.0 * t
        sx = 1.04
        sy = 0.94
        angle = 8.0 + 5.0 * t
        eyes = "squint"
        strap_swing = 4.0
        shadow_w = 40 + 8.0 * t
    elif animation == "slash":
        t = frame_index / max(1, frame_count - 1)
        wind = 1.0 - min(1.0, t / 0.35)
        strike = max(0.0, min(1.0, (t - 0.20) / 0.42))
        cx += -5.0 * wind + 9.0 * strike
        angle = -12.0 * wind + 18.0 * strike
        sx = 1.0 + 0.06 * strike
        sy = 1.0 - 0.04 * strike
        eyes = "squint"
        strap_swing = 6.0 * strike
    elif animation == "blink_out":
        t = frame_index / max(1, frame_count - 1)
        sx = max(0.36, 1.0 - 0.58 * t)
        sy = 1.0 + 0.15 * math.sin(t * math.pi)
        cx -= 4.0 + 15.0 * t
        cy -= 3.0 + 14.0 * t
        angle = -12.0 - 26.0 * t
        tint = max(0.35, 1.0 - 0.45 * t)
        eyes = "squint"
        shadow_w = max(22.0, 44.0 - 20.0 * t)
        shadow_a = max(20, int(70 * (1.0 - 0.7 * t)))
    elif animation == "blink_in":
        t = frame_index / max(1, frame_count - 1)
        inv = 1.0 - t
        sx = max(0.42, 1.0 - 0.54 * inv)
        sy = 1.0 + 0.12 * math.sin(t * math.pi)
        cx += 12.0 * inv
        cy -= 11.0 * inv
        angle = 22.0 * inv
        tint = 0.55 + 0.45 * t
        eyes = "squint"
        shadow_w = 28.0 + 18.0 * t
        shadow_a = 45 + int(30 * t)
    elif animation == "dash":
        t = frame_index / max(1, frame_count - 1)
        cx += 7.0 + 9.0 * t
        sx = 1.16
        sy = 0.82
        angle = -15.0 + phase * 2.0
        eyes = "squint"
        strap_swing = 7.0
        shadow_w = 58
    elif animation == "crouch":
        t = math.sin((frame_index / max(1, frame_count - 1)) * math.pi)
        cy += 9.0 * t
        sx = 1.08
        sy = 0.78
        angle = -3.0 * t
        eyes = "squint"
        shadow_w = 52
    elif animation == "wall_slide":
        cy += frame_index * 2.0
        cx -= 13.0
        sx = 0.92
        sy = 1.04
        angle = -10.0
        strap_swing = 5.0
        eyes = "squint"
    elif animation == "wall_jump":
        t = frame_index / max(1, frame_count - 1)
        cx += 16.0 * t - 11.0
        cy -= math.sin(t * math.pi) * 22.0
        sx = 1.05
        sy = 0.92
        angle = -18.0 + 35.0 * t
        eyes = "squint"
        shadow_w = 37
    elif animation == "ledge_grab":
        t = frame_index / max(1, frame_count - 1)
        cx -= 8.0
        cy -= 10.0 - 7.0 * t
        sx = 0.94 + 0.06 * t
        sy = 1.06 - 0.04 * t
        angle = -6.0 + 5.0 * t
        strap_swing = -5.0
        eyes = "squint" if frame_index < frame_count - 2 else "normal"
    elif animation == "climb":
        cy += phase * 2.0
        cx += math.sin((frame_index / max(1, frame_count)) * math.tau * 2.0) * 2.5
        sx = 0.96
        sy = 1.04
        angle = phase * 6.0
        strap_swing = -phase * 4.0
    elif animation == "swim":
        t = frame_index / max(1, frame_count - 1)
        cx += phase * 2.0
        cy -= 8.0 + math.sin(t * math.tau * 2.0) * 2.0
        sx = 1.12
        sy = 0.78
        angle = 11.0 + phase * 5.0
        strap_swing = phase * 5.0
        eyes = "squint"
        shadow_a = 30
    elif animation == "interact":
        t = frame_index / max(1, frame_count - 1)
        cx += 2.0 * math.sin(t * math.pi)
        cy -= 3.0 * math.sin(t * math.pi)
        sx = 1.0 + 0.03 * math.sin(t * math.pi)
        sy = 1.0 - 0.02 * math.sin(t * math.pi)
        angle = -2.0 + 4.0 * math.sin(t * math.pi)
        eyes = "normal"
        strap_swing = -3.0 * math.sin(t * math.pi)
    elif animation == "talk":
        cy += math.sin(frame_index * math.tau / max(1, frame_count)) * 0.7
        sx = 1.0 + 0.015 * phase
        sy = 1.0 - 0.02 * phase
        angle = phase * 1.8
        eyes = "sleepy" if frame_index in {2, 6} else "normal"
        strap_swing = phase * 1.5
    elif animation == "block":
        t = min(1.0, frame_index / max(1, frame_count - 1) * 1.5)
        cx -= 4.0 * t
        sx = 0.94
        sy = 1.08
        angle = -9.0
        eyes = "squint"
        strap_swing = -4.0
        shadow_w = 48
    elif animation == "land":
        t = frame_index / max(1, frame_count - 1)
        squash = 1.0 - min(1.0, t * 1.7)
        cy += 10.0 * squash
        sx = 1.14 - 0.06 * t
        sy = 0.78 + 0.22 * t
        angle = -4.0 * squash
        eyes = "squint"
        shadow_w = 58
    elif animation == "roll":
        t = frame_index / max(1, frame_count - 1)
        cx += -12.0 + 24.0 * t
        cy += 7.0 + math.sin(t * math.tau) * 1.5
        sx = 1.05
        sy = 0.70
        angle = 360.0 * t
        eyes = "squint"
        strap_swing = 6.0 * math.sin(t * math.tau)
        shadow_w = 56
    elif animation == "slide":
        t = frame_index / max(1, frame_count - 1)
        cx += 11.0 * t
        cy += 9.0
        sx = 1.18
        sy = 0.66
        angle = -17.0
        eyes = "squint"
        strap_swing = 7.0
        shadow_w = 60
    elif animation == "crouch_walk":
        cy += 8.0 + abs(phase) * 0.8
        cx += phase * 1.8
        sx = 1.10
        sy = 0.74
        angle = phase * 2.0 - 3.0
        eyes = "squint"
        strap_swing = -phase * 2.0
        shadow_w = 54
    elif animation == "pickup":
        t = frame_index / max(1, frame_count - 1)
        bend = math.sin(t * math.pi)
        cy += 7.0 * bend
        sx = 1.0 + 0.05 * bend
        sy = 1.0 - 0.08 * bend
        angle = -12.0 * bend
        eyes = "squint"
        strap_swing = -5.0 * bend
    elif animation == "throw":
        t = frame_index / max(1, frame_count - 1)
        release = min(1.0, max(0.0, (t - 0.22) / 0.55))
        cx += -4.0 + 14.0 * release
        sx = 0.94 + 0.13 * release
        sy = 1.06 - 0.08 * release
        angle = -22.0 + 42.0 * release
        eyes = "squint"
        strap_swing = 7.0 * release
    elif animation == "aim":
        sx = 0.98
        sy = 1.03
        angle = -5.0 + phase * 0.8
        eyes = "squint"
        strap_swing = 1.0
    elif animation == "shoot":
        t = frame_index / max(1, frame_count - 1)
        recoil = 1.0 - min(1.0, t * 2.0)
        cx -= 4.0 * recoil
        sx = 0.94 + 0.04 * recoil
        sy = 1.04 - 0.02 * recoil
        angle = -9.0 - 9.0 * recoil
        eyes = "squint"
        strap_swing = -3.0
    elif animation == "charge":
        t = frame_index / max(1, frame_count - 1)
        pulse = 0.5 + 0.5 * math.sin(t * math.tau * 3.0)
        cy -= pulse * 2.5
        sx = 1.0 + 0.04 * pulse
        sy = 1.0 - 0.03 * pulse
        angle = phase * 3.0
        eyes = "squint"
        strap_swing = phase * 5.0
    elif animation == "cast":
        t = frame_index / max(1, frame_count - 1)
        cast = min(1.0, t * 1.4)
        cy -= math.sin(t * math.pi) * 3.0
        sx = 0.96 + 0.08 * cast
        sy = 1.05 - 0.04 * cast
        angle = -8.0 + 18.0 * cast
        eyes = "squint"
        strap_swing = -6.0 + 9.0 * cast
    elif animation == "celebrate":
        hop = abs(phase)
        cy -= 9.0 * hop
        sx = 1.0 - 0.04 * hop
        sy = 1.0 + 0.05 * hop
        angle = phase * 8.0
        eyes = "sleepy" if frame_index % 4 == 0 else "normal"
        strap_swing = phase * 8.0
    elif animation == "sit":
        t = min(1.0, frame_index / max(1, frame_count - 1) * 1.6)
        cy += 15.0 * t
        sx = 1.12
        sy = 0.67 + 0.10 * (1.0 - t)
        angle = -4.0 * t
        eyes = "sleepy" if t > 0.7 else "normal"
        shadow_w = 57
    elif animation == "sleep":
        cy += 15.0
        sx = 1.13
        sy = 0.64 + 0.03 * phase
        angle = -7.0
        eyes = "sleepy"
        strap_swing = 1.0 * phase
        shadow_w = 58
    elif animation == "hover":
        cy -= 10.0 + phase * 2.0
        sx = 0.98
        sy = 1.04
        angle = phase * 4.0
        eyes = "normal"
        shadow_w = 36
        shadow_a = 38
    elif animation == "stomp":
        t = frame_index / max(1, frame_count - 1)
        wind = 1.0 - min(1.0, t * 1.8)
        impact = max(0.0, min(1.0, (t - 0.43) / 0.18))
        cy -= 18.0 * wind
        cy += 11.0 * impact
        sx = 1.05 + 0.10 * impact
        sy = 1.04 - 0.22 * impact
        angle = -12.0 * wind + 10.0 * impact
        eyes = "squint"
        shadow_w = 48 + 16 * impact
    elif animation == "hit":
        poses = [
            (-5, -1, 1.06, 0.91, -10, "squint", 1.04, -3),
            (8, 3, 0.91, 1.10, 9, "squint", 0.97, 4),
            (1, 0, 1.04, 0.96, -5, "normal", 1.01, -1),
            (0, 0, 1.00, 1.00, 0, "normal", 1.00, 0),
        ][frame_index % 4]
        dx, dy, sx, sy, angle, eyes, tint, strap_swing = poses
        cx += dx
        cy += dy
        shadow_w = 49 + abs(dx) * 1.1
        shadow_a = 88
    elif animation == "death":
        poses = [
            (0, 0, 1.00, 1.00, 0, "sleepy", 1.00, 0),
            (7, 8, 1.03, 0.95, 15, "sleepy", 0.99, 2),
            (16, 17, 1.03, 0.92, 34, "sleepy", 0.97, 3),
            (23, 27, 1.03, 0.84, 57, "sleepy", 0.95, 3),
            (24, 35, 1.08, 0.72, 77, "x", 0.93, 4),
            (22, 38, 1.13, 0.61, 88, "x", 0.91, 5),
            (22, 39, 1.15, 0.56, 91, "x", 0.89, 5),
        ]
        dx, dy, sx, sy, angle, eyes, tint, strap_swing = poses[
            min(frame_index, len(poses) - 1)
        ]
        cx += dx
        cy += dy
        shadow_w = 53 + frame_index * 5.0
        shadow_a = 90
        _dust(canvas, frame_index, base_x=69 + dx * 0.3)
    else:
        raise KeyError(f"unknown sandbag animation: {animation!r}")

    # Ground shadow removed; in-game compositing handles ground contact.
    body_layer = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    _draw_sandbag_body(
        body_layer,
        cx=cx,
        cy=cy,
        sx=sx,
        sy=sy,
        eyes=eyes,
        tint=tint,
        strap_swing=strap_swing,
    )
    if angle:
        body_layer = body_layer.rotate(
            angle,
            center=(_s(cx), _s(cy + 18)),
            resample=Image.Resampling.BICUBIC,
            fillcolor=(0, 0, 0, 0),
        )
    canvas.alpha_composite(body_layer)

    if animation == "hit":
        _impact_marks(canvas, frame_index)
        _dust(canvas, frame_index, base_x=72, base_y=113)
    if animation == "death" and frame_index >= 3:
        _dust(canvas, frame_index, base_x=76, base_y=114)
    if animation == "slash":
        d = ImageDraw.Draw(canvas, "RGBA")
        t = frame_index / max(1, frame_count - 1)
        if 0.20 <= t <= 0.74:
            alpha = int(160 * math.sin((t - 0.20) / 0.54 * math.pi))
            d.arc(
                _box(66, 34, 126, 95),
                start=-70,
                end=35,
                fill=_rgba("ffdf70", alpha),
                width=_s(3.2),
            )
    if animation == "dash":
        d = ImageDraw.Draw(canvas, "RGBA")
        for i in range(4):
            y = 44 + i * 12 + math.sin(frame_index + i) * 2.0
            d.line(
                _box(18, y, 48 - i * 4, y - 2), fill=_rgba("dfe7ff", 78), width=_s(1.2)
            )
    if animation in {"blink_out", "blink_in"}:
        d = ImageDraw.Draw(canvas, "RGBA")
        t = frame_index / max(1, frame_count - 1)
        alpha = int(120 * (1.0 - abs(t - 0.5)))
        d.ellipse(_box(43, 32, 62, 94), outline=_rgba("c5b8ff", alpha), width=_s(1.5))
    if animation == "interact":
        d = ImageDraw.Draw(canvas, "RGBA")
        alpha = int(140 * math.sin((frame_index / max(1, frame_count - 1)) * math.pi))
        d.line(_box(92, 50, 105, 42), fill=_rgba("fff4a3", alpha), width=_s(1.8))
        d.line(_box(94, 61, 110, 61), fill=_rgba("fff4a3", alpha), width=_s(1.8))
    if animation == "talk":
        d = ImageDraw.Draw(canvas, "RGBA")
        if frame_index % 2 == 0:
            d.arc(
                _box(58, 70, 72, 78), 10, 170, fill=_rgba("14131d", 180), width=_s(1.5)
            )
    if animation == "block":
        d = ImageDraw.Draw(canvas, "RGBA")
        d.rounded_rectangle(
            _box(28, 38, 39, 91),
            radius=_s(5),
            fill=_rgba("c9ccd9", 178),
            outline=_rgba("4b4d60", 220),
            width=_s(1.3),
        )
    if animation in {"land", "stomp"}:
        d = ImageDraw.Draw(canvas, "RGBA")
        t = frame_index / max(1, frame_count - 1)
        impact = 1.0 - min(1.0, abs(t - 0.52) / 0.52)
        if impact > 0.02:
            d.arc(
                _box(24, 105 - 3 * impact, 105, 119 + 3 * impact),
                start=190,
                end=350,
                fill=_rgba("c5b8ff", int(120 * impact)),
                width=_s(1.5),
            )
    if animation in {"slide", "roll"}:
        d = ImageDraw.Draw(canvas, "RGBA")
        for i in range(3):
            d.line(
                _box(39 - i * 10, 101 + i * 4, 19 - i * 9, 105 + i * 4),
                fill=_rgba("a99b83", 82 - i * 15),
                width=_s(1.2),
            )
    if animation in {"aim", "shoot"}:
        d = ImageDraw.Draw(canvas, "RGBA")
        d.ellipse(_box(103, 53, 117, 67), outline=_rgba("ffe56f", 145), width=_s(1.2))
    if animation == "shoot":
        d = ImageDraw.Draw(canvas, "RGBA")
        t = frame_index / max(1, frame_count - 1)
        flash = 1.0 - min(1.0, t * 2.2)
        if flash > 0.03:
            d.polygon(
                [(_s(91), _s(61)), (_s(118), _s(51)), (_s(113), _s(69))],
                fill=_rgba("ffe56f", int(190 * flash)),
            )
    if animation in {"charge", "cast"}:
        d = ImageDraw.Draw(canvas, "RGBA")
        t = frame_index / max(1, frame_count - 1)
        r = 9 + 17 * min(1.0, t * 1.2)
        d.ellipse(
            _box(84 - r, 55 - r, 84 + r, 55 + r),
            outline=_rgba("c5b8ff", 115),
            width=_s(1.4),
        )
    if animation == "throw":
        d = ImageDraw.Draw(canvas, "RGBA")
        t = frame_index / max(1, frame_count - 1)
        if t > 0.2:
            u = min(1.0, (t - 0.2) / 0.8)
            x = 83 + 38 * u
            y = 47 - 16 * math.sin(u * math.pi) + 16 * u
            d.ellipse(
                _box(x - 4, y - 4, x + 4, y + 4),
                fill=_rgba("ffe56f", 210),
                outline=_rgba("4b4d60", 230),
                width=_s(1.0),
            )
    if animation == "sleep":
        d = ImageDraw.Draw(canvas, "RGBA")
        for i in range(3):
            u = ((frame_index + i * 2) % max(1, frame_count)) / max(1, frame_count - 1)
            d.text(
                (_s(78 + i * 8), _s(42 - u * 22)),
                "Z",
                fill=_rgba("4b4d60", int(155 * (1.0 - u * 0.4))),
            )
    if animation == "celebrate":
        d = ImageDraw.Draw(canvas, "RGBA")
        for i, (x, y) in enumerate([(38, 38), (54, 27), (78, 30), (94, 42), (48, 55)]):
            yy = y + ((frame_index + i) % max(1, frame_count)) * 2
            d.rectangle(
                _box(x - 2, yy - 2, x + 2, yy + 2),
                fill=_rgba("ffe56f" if i % 2 else "c5b8ff", 165),
            )
    if animation == "hover":
        d = ImageDraw.Draw(canvas, "RGBA")
        flame = 0.6 + 0.4 * math.sin(frame_index * 1.8)
        d.polygon(
            [
                (_s(56), _s(104)),
                (_s(48), _s(119 + 7 * flame)),
                (_s(65), _s(119 + 7 * flame)),
            ],
            fill=_rgba("c5b8ff", int(135 * flame)),
        )

    return canvas.resize((FRAME_W, FRAME_H), Image.Resampling.LANCZOS)


def _measure_body_extent(frame: Image.Image) -> Dict[str, object] | None:
    bbox = frame.getchannel("A").getbbox()
    if bbox is None:
        return None
    x1, y1, x2, y2 = bbox
    feet_y = y2 - 1
    feet_x = (x1 + x2 - 1) / 2.0
    return {
        "frame_width": frame.width,
        "frame_height": frame.height,
        "body_pixel_bbox": {
            "x": int(x1),
            "y": int(y1),
            "w": int(x2 - x1),
            "h": int(y2 - y1),
        },
        "feet_pixel": {"x": round(feet_x, 3), "y": round(float(feet_y), 3)},
        "feet_anchor_norm": {
            "x": round(feet_x / frame.width - 0.5, 6),
            "y": round(0.5 - feet_y / frame.height, 6),
        },
    }


def _yaml_scalar(value: object) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return str(value)
    text = str(value)
    if text.replace("_", "").replace("-", "").replace(".", "").isalnum():
        return text
    return repr(text)


def _write_manifest(path: Path, manifest: Dict[str, object]) -> None:
    # Small hand-rolled YAML writer to avoid requiring PyYAML for this one-off script.
    lines: List[str] = []

    def emit(key: str, value: object, indent: int = 0) -> None:
        pad = " " * indent
        if isinstance(value, dict):
            lines.append(f"{pad}{key}:")
            for k, v in value.items():
                emit(str(k), v, indent + 2)
        elif isinstance(value, list):
            lines.append(f"{pad}{key}:")
            for item in value:
                if isinstance(item, dict):
                    lines.append(f"{pad}-")
                    for k, v in item.items():
                        emit(str(k), v, indent + 2)
                else:
                    lines.append(f"{pad}- {_yaml_scalar(item)}")
        else:
            lines.append(f"{pad}{key}: {_yaml_scalar(value)}")

    for key, value in manifest.items():
        emit(key, value, 0)
    path.write_text("\n".join(lines) + "\n", encoding="utf8")


def _rows_for_sparse() -> List[Tuple[str, str, int, int]]:
    return [
        (name, name, frames, duration_ms) for name, frames, duration_ms in SANDBAG_ROWS
    ]


def build_sheet(
    rows: List[Tuple[str, str, int, int]], *, sheet_background: RGBA = (0, 0, 0, 0)
) -> Tuple[Image.Image, Dict[str, object]]:
    max_frames = max(frames for _, _, frames, _ in rows)
    sheet = Image.new(
        "RGBA", (LABEL_W + max_frames * FRAME_W, len(rows) * FRAME_H), sheet_background
    )
    draw = ImageDraw.Draw(sheet, "RGBA")
    font = _font(12)
    small = _font(10)
    manifest: Dict[str, object] = {
        "target": "sandbag",
        "frame_width": FRAME_W,
        "frame_height": FRAME_H,
        "label_width": LABEL_W,
        "border": 0,
        "animation_order": [row_name for row_name, _, _, _ in rows],
        "notes": "Sparse sheet: only rows listed in animation_order are emitted. Runtime should resolve missing animations to idle.",
        "style_notes": "Pale stitched cloth sandbag with strap and oval eyes; references the uploaded sandbag visually without copying its exact proportions or seams.",
        "animations": {},
    }
    first_frame: Image.Image | None = None
    for row_idx, (row_name, source_name, frame_count, duration_ms) in enumerate(rows):
        y = row_idx * FRAME_H
        draw.rectangle((0, y, LABEL_W, y + FRAME_H), fill=(24, 24, 38, 188))
        draw.text((8, y + 9), row_name, fill=(238, 239, 255, 255), font=font)
        label = f"{frame_count}f/{duration_ms}ms"
        if row_name != source_name:
            label += f" -> {source_name}"
        draw.text((8, y + 28), label, fill=(186, 189, 214, 255), font=small)
        frame_records = []
        for frame_index in range(frame_count):
            frame = render_frame(source_name, frame_index % frame_count, frame_count)
            x = LABEL_W + frame_index * FRAME_W
            sheet.alpha_composite(frame, (x, y))
            if first_frame is None and row_name == "idle" and frame_index == 0:
                first_frame = frame
            frame_records.append(
                {
                    "index": frame_index,
                    "x": x,
                    "y": y,
                    "w": FRAME_W,
                    "h": FRAME_H,
                    "duration_ms": duration_ms,
                }
            )
        manifest["animations"][row_name] = {
            "source_animation": source_name,
            "frames": frame_records,
            "duration_ms": duration_ms,
        }
    metrics = _measure_body_extent(first_frame) if first_frame is not None else None
    if metrics is not None:
        manifest["body_metrics"] = metrics
    return sheet, manifest


def write_outputs(out_dir: Path) -> Tuple[Path, Path, Path]:
    rows = _rows_for_sparse()
    out_dir.mkdir(parents=True, exist_ok=True)
    stem = "sandbag_spritesheet"
    png_path = out_dir / f"{stem}.png"
    yaml_path = out_dir / f"{stem}.yaml"
    ron_path = out_dir / f"{stem}.ron"
    sheet, manifest = build_sheet(rows)
    sheet.save(png_path)
    _write_manifest(yaml_path, manifest)
    # The runtime SheetRegistry parses RON, not YAML, at startup
    # (`presentation::character_sprites::registry`). The sandbag
    # manifest already has the `animations: {...}` shape that
    # `_adapter_manifest_to_ron` consumes — same path the adapter
    # `draw-all` pipeline uses for the row-ordered RON.
    from ...authoring.sheet import (
        _adapter_manifest_to_ron,
    )  # local import: tooling-only dependency

    manifest_for_ron = dict(manifest)
    manifest_for_ron["image"] = png_path.name
    ron_path.write_text(_adapter_manifest_to_ron(manifest_for_ron), encoding="utf8")
    actor_path = write_actor_contract_for_tackon(
        target=TARGET_NAME,
        image_out=png_path,
        sheet_ron_out=ron_path,
        manifest=manifest_for_ron,
        actor_metadata={
            "actor": {"character_id": "sandbag", "display_name": "Sandbag"},
            "brain": {"default_preset": "stand_still"},
            "actions": {"default_preset": "sandbag_punch"},
            "body": {
                "body_plan": "TrainingDummy",
                "body_kind": "Standard",
                "traits": ["training"],
            },
            "tags": ["training"],
        },
    )
    return png_path, yaml_path, ron_path, actor_path


TARGET_NAME = "sandbag"
SHEET_FILES = (
    "sandbag_spritesheet.png",
    "sandbag_spritesheet.yaml",
    "sandbag_spritesheet.ron",
    "sandbag_actor.ron",
)


def render(out_dir: Path) -> List[Path]:
    """Render the sparse sandbag spritesheet (idle/hit/death) into ``out_dir``."""
    out_dir.mkdir(parents=True, exist_ok=True)
    return list(write_outputs(out_dir))
