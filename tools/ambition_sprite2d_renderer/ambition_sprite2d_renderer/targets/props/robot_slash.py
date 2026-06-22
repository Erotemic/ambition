from __future__ import annotations

"""Robot melee slash effect — a crisp energy arc for the robot's attacks.

A single overlay target with directional variants so the robot's swing
reads clearly no matter where it strikes. The effect is a white-cored,
cyan-haloed crescent that flashes in, peaks, and fades over a few
frames. The game spawns it at the ``origin`` anchor (the attacker's
hand / pivot) and plays the row that matches the attack direction.

Variants (one animation row each):

- ``side``: a forward crescent that sweeps in front of the robot
  (origin on the left, arc opening right). The default horizontal
  swing.
- ``up``: the same crescent rotated to sweep overhead (origin at the
  bottom). The up-tilt / anti-air attack.
- ``down``: a downward thrust **poke** — a narrow tapered lance instead
  of a wide arc (origin at the top, point stabbing down). The
  down-tile / pogo attack.

Anchors per frame: ``origin`` (pivot) and ``tip`` (leading point), plus
an ``effect`` block (kind + progress) mirroring ``generic_explosions``.
"""

import math
from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageDraw, ImageFilter

from ...authoring.tackon_sheet import build_sheet

TARGET_NAME = "robot_slash"
SHEET_FILES = (
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
)

FRAME_SIZE = (160, 160)
SUPER = 4
# 2 frames per row to match the player's 2-frame melee swing — the effect
# played 5 frames (~230ms) and lingered well past the snappy attack. The
# runtime plays the row once over `frames * frame_duration`, so 2 frames keeps
# the slash on screen for roughly the swing's length instead of trailing it.
ROWS: List[Tuple[str, int, int]] = [
    ("side", 2, 60),
    ("up", 2, 60),
    ("down", 2, 60),
]

# Energy palette — hot white core into a cool cyan halo. Reads as a
# clean robotic edge rather than fire or magic.
CORE = (246, 251, 255, 255)
HOT = (198, 236, 255, 255)
GLOW = (118, 198, 255, 255)
RIM = (44, 122, 210, 255)

# Per-variant geometry. ``origin`` is the pivot in design pixels;
# ``facing`` is the direction (degrees, 0 = +X right, +90 = down)
# the slash points.
GEOMETRY = {
    "side": {"origin": (30.0, 80.0), "facing": 0.0, "radius": 104.0},
    "up": {"origin": (80.0, 130.0), "facing": -90.0, "radius": 104.0},
    "down": {"origin": (80.0, 30.0), "facing": 90.0, "radius": 110.0},
}


def _px(v: float) -> float:
    return v * SUPER


def _envelope(t: float) -> float:
    """Flash-in / fade-out amplitude. Already strong on frame 0 so the
    slash lands instantly with the swing, peaks early, then fades out."""
    appear = min(1.0, (t + 0.18) / 0.30)
    fade = 1.0 - max(0.0, (t - 0.45) / 0.55)
    return max(0.0, appear * fade)


def _crescent_points(
    cx: float,
    cy: float,
    facing_deg: float,
    half_span_deg: float,
    r_outer: float,
    r_inner: float,
) -> List[Tuple[float, float]]:
    n = 16
    outer: List[Tuple[float, float]] = []
    inner: List[Tuple[float, float]] = []
    a0 = math.radians(facing_deg - half_span_deg)
    a1 = math.radians(facing_deg + half_span_deg)
    for i in range(n + 1):
        a = a0 + (a1 - a0) * (i / n)
        outer.append((cx + math.cos(a) * r_outer, cy + math.sin(a) * r_outer))
        inner.append((cx + math.cos(a) * r_inner, cy + math.sin(a) * r_inner))
    return outer + list(reversed(inner))


def _draw_arc_variant(spec: dict, t: float) -> Image.Image:
    """Side / up crescent slash."""
    cx, cy = spec["origin"]
    facing = spec["facing"]
    base_r = spec["radius"]
    amp = _envelope(t)

    w, h = FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER
    layer = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    if amp <= 0.01:
        return layer.resize(FRAME_SIZE, Image.Resampling.LANCZOS)

    # Span widens through the swing; radius expands slightly as it fades.
    half_span = 26.0 + 46.0 * min(1.0, t * 1.25)
    r_outer = base_r * (0.92 + 0.16 * t)
    r_inner = r_outer * (0.58 - 0.06 * t)

    def pts(ro, ri):
        return [
            (_px(x), _px(y))
            for (x, y) in _crescent_points(cx, cy, facing, half_span, ro, ri)
        ]

    # Cyan halo (blurred, slightly larger).
    glow = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow, "RGBA")
    gd.polygon(
        pts(r_outer * 1.06, r_inner * 0.92),
        fill=(GLOW[0], GLOW[1], GLOW[2], int(150 * amp)),
    )
    glow = glow.filter(ImageFilter.GaussianBlur(radius=max(3, int(SUPER * 2.2))))
    layer.alpha_composite(glow)

    # Rim + body.
    bd = ImageDraw.Draw(layer, "RGBA")
    bd.polygon(pts(r_outer, r_inner), fill=(RIM[0], RIM[1], RIM[2], int(210 * amp)))
    bd.polygon(
        pts(r_outer * 0.97, r_inner * 1.08),
        fill=(HOT[0], HOT[1], HOT[2], int(225 * amp)),
    )

    # Bright leading edge along the outer arc, brightest at the leading tip.
    outer_edge = _crescent_points(
        cx, cy, facing, half_span, r_outer * 0.99, r_outer * 0.99
    )[:17]
    edge_pts = [(_px(x), _px(y)) for (x, y) in outer_edge]
    bd.line(
        edge_pts,
        fill=(CORE[0], CORE[1], CORE[2], int(245 * amp)),
        width=max(2, int(SUPER * 1.6)),
        joint="curve",
    )

    # Leading-tip spark (the end of the swing arc).
    tip = outer_edge[-1]
    for r, a in ((5.5, 110), (3.0, 200), (1.5, 255)):
        bd.ellipse(
            (_px(tip[0] - r), _px(tip[1] - r), _px(tip[0] + r), _px(tip[1] + r)),
            fill=(CORE[0], CORE[1], CORE[2], int(a * amp)),
        )
    return layer.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


def _draw_poke_variant(spec: dict, t: float) -> Image.Image:
    """Down thrust — a narrow tapered lance stabbing along ``facing``."""
    cx, cy = spec["origin"]
    facing = math.radians(spec["facing"])
    base_r = spec["radius"]
    amp = _envelope(t)

    w, h = FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER
    layer = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    if amp <= 0.01:
        return layer.resize(FRAME_SIZE, Image.Resampling.LANCZOS)

    dx, dy = math.cos(facing), math.sin(facing)
    nx, ny = -dy, dx  # perpendicular
    reach = base_r * (0.80 + 0.30 * min(1.0, t * 1.3))
    width = 13.0 * amp

    base = (cx, cy)
    belly = (cx + dx * reach * 0.34, cy + dy * reach * 0.34)
    tip = (cx + dx * reach, cy + dy * reach)
    left = (belly[0] + nx * width, belly[1] + ny * width)
    right = (belly[0] - nx * width, belly[1] - ny * width)

    def P(pt):
        return (_px(pt[0]), _px(pt[1]))

    # Halo.
    glow = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow, "RGBA")
    gd.polygon(
        [P(base), P(left), P(tip), P(right)],
        fill=(GLOW[0], GLOW[1], GLOW[2], int(150 * amp)),
    )
    glow = glow.filter(ImageFilter.GaussianBlur(radius=max(3, int(SUPER * 2.0))))
    layer.alpha_composite(glow)

    bd = ImageDraw.Draw(layer, "RGBA")
    bd.polygon(
        [P(base), P(left), P(tip), P(right)],
        fill=(RIM[0], RIM[1], RIM[2], int(210 * amp)),
    )
    # Inner hot lance.
    left2 = (belly[0] + nx * width * 0.55, belly[1] + ny * width * 0.55)
    right2 = (belly[0] - nx * width * 0.55, belly[1] - ny * width * 0.55)
    bd.polygon(
        [P(base), P(left2), P(tip), P(right2)],
        fill=(HOT[0], HOT[1], HOT[2], int(230 * amp)),
    )
    # Bright core spine.
    bd.line(
        [P(base), P(tip)],
        fill=(CORE[0], CORE[1], CORE[2], int(245 * amp)),
        width=max(2, int(SUPER * 1.4)),
    )
    # Tip impact spark.
    for r, a in ((6.0, 110), (3.2, 205), (1.6, 255)):
        bd.ellipse(
            (_px(tip[0] - r), _px(tip[1] - r), _px(tip[0] + r), _px(tip[1] + r)),
            fill=(CORE[0], CORE[1], CORE[2], int(a * amp)),
        )
    return layer.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


def _draw_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    spec = GEOMETRY[anim]
    t = frame_idx / max(1, nframes - 1)
    if anim == "down":
        return _draw_poke_variant(spec, t)
    return _draw_arc_variant(spec, t)


def _tip_for(anim: str, t: float) -> Tuple[float, float]:
    spec = GEOMETRY[anim]
    cx, cy = spec["origin"]
    if anim == "down":
        reach = spec["radius"] * (0.80 + 0.30 * min(1.0, t * 1.3))
        a = math.radians(spec["facing"])
        return (cx + math.cos(a) * reach, cy + math.sin(a) * reach)
    half_span = 26.0 + 46.0 * min(1.0, t * 1.25)
    r_outer = spec["radius"] * (0.92 + 0.16 * t)
    a = math.radians(spec["facing"] + half_span)
    return (cx + math.cos(a) * r_outer, cy + math.sin(a) * r_outer)


def _frame_meta(anim: str, frame_idx: int, nframes: int) -> dict:
    t = frame_idx / max(1, nframes - 1)
    origin = GEOMETRY[anim]["origin"]
    tip = _tip_for(anim, t)
    return {
        "anchors": {
            "origin": {"x": round(origin[0], 2), "y": round(origin[1], 2)},
            "tip": {"x": round(tip[0], 2), "y": round(tip[1], 2)},
        },
        "effect": {"kind": anim, "progress": round(t, 4)},
    }


ACTOR_METADATA = {
    "actor": {"character_id": "fx_robot_slash", "display_name": "Robot Slash"},
    "body": {
        "body_plan": "Effect",
        "body_kind": "Slash",
        "locomotion_hint": "Stationary",
        "traits": ["fx", "slash", "melee", "overlay"],
    },
    "brain": {"default_preset": "stand_still"},
    "actions": {"default_preset": "peaceful"},
    "animation_bindings": {
        "default": {"animation": "side", "events": []},
        "action.melee.side": {"animation": "side", "events": []},
        "action.melee.up": {"animation": "up", "events": []},
        "action.melee.down": {"animation": "down", "events": []},
    },
    "sockets": {
        "origin": {
            "source": f"{TARGET_NAME}.geometry",
            "point": {
                "x": GEOMETRY["side"]["origin"][0],
                "y": GEOMETRY["side"]["origin"][1],
            },
        },
    },
    "tags": ["fx", "slash", "melee", "overlay"],
}


def render(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=_draw_frame,
        out_dir=out_dir,
        frame_size=opts.get("frame_size") or FRAME_SIZE,
        frame_meta_fn=_frame_meta,
        auto_crop=True,
        crop_margin=6,
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
