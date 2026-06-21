from __future__ import annotations

"""Procedural sprite-sheet generator for generic explosion FX.

Designed to stay mostly standalone for now so it can be iterated on without
clobbering shared renderer helpers. The output is a sheet of several explosion
variants that can be spawned as overlay entities in-game.

Rows:
- ``classic_burst`` — default classic cartoony blast
- ``burst_round``   — fuller round detonation
- ``shockwave``     — punchy blast ring with strong expanding rim
- ``smoke_burst``   — heavier smoke / ember breakup
- ``starburst``     — sharper pointy explosion silhouette
"""

import math
from pathlib import Path
from typing import Dict, List, Tuple

from PIL import Image, ImageDraw

from ...core.draw import overlay_draw as _overlay_draw
from ...authoring.tackon_sheet import build_sheet, write_canonical

RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_NAME = "generic_explosions"
SHEET_FILES = [
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
]

ACTOR_METADATA = {
    "actor": {
        "character_id": "fx_generic_explosions",
        "display_name": "Generic Explosions",
    },
    "body": {
        "body_plan": "Effect",
        "body_kind": "Burst",
        "mass_class": "Light",
        "locomotion_hint": "Stationary",
        "traits": ["fx", "explosion", "overlay"],
    },
    "brain": {"default_preset": "stand_still"},
    "actions": {"default_preset": "peaceful"},
    "animation_bindings": {
        "default": {"animation": "classic_burst", "events": []},
        "action.special.round": {"animation": "burst_round", "events": []},
        "action.special.shockwave": {"animation": "shockwave", "events": []},
        "action.special.smoke": {"animation": "smoke_burst", "events": []},
        "action.special.starburst": {"animation": "starburst", "events": []},
    },
    "sockets": {
        "origin": {
            "source": f"{TARGET_NAME}.geometry",
            "point": {"x": 72.0, "y": 72.0},
        },
    },
    "tags": ["fx", "explosion", "overlay"],
}

ROWS: List[Tuple[str, int, int]] = [
    ("classic_burst", 7, 74),
    ("burst_round", 7, 84),
    ("shockwave", 6, 78),
    ("smoke_burst", 7, 96),
    ("starburst", 6, 82),
]

FRAME_SIZE = (144, 144)
SUPER = 4
W, H = FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER

OUTLINE = (40, 20, 28, 255)
CORE = (255, 250, 222, 255)
CORE_HI = (255, 255, 245, 255)
FLAME_A = (255, 208, 96, 255)
FLAME_B = (239, 129, 68, 255)
SMOKE = (118, 72, 92, 220)
SMOKE_DARK = (74, 40, 60, 228)
EMBER = (255, 236, 164, 255)
RING = (255, 232, 170, 200)


def _s(v: float) -> int:
    return int(round(v * SUPER))


def _pt(x: float, y: float) -> Tuple[int, int]:
    return (_s(x), _s(y))


def _box(x1: float, y1: float, x2: float, y2: float) -> Tuple[int, int, int, int]:
    return (_s(x1), _s(y1), _s(x2), _s(y2))


def _lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def _mul_alpha(color: RGBA, factor: float) -> RGBA:
    factor = max(0.0, factor)
    return (
        color[0],
        color[1],
        color[2],
        max(0, min(255, int(round(color[3] * factor)))),
    )


def _ease(t: float) -> float:
    t = max(0.0, min(1.0, t))
    return t * t * (3.0 - 2.0 * t)


# `_overlay_draw` now comes from core.draw (imported above) — the one canonical
# alpha-clobber-safe scratch-layer helper. Kept as a local alias so the existing
# call sites are untouched.


def _composite_ellipse(
    img: Image.Image,
    bbox: Tuple[float, float, float, float],
    *,
    fill: RGBA,
    outline: RGBA | None = None,
    width: int = 1,
) -> None:
    layer, draw = _overlay_draw(img)
    draw.ellipse(bbox, fill=fill, outline=outline, width=width)
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
    draw.polygon(points, fill=fill, outline=outline)
    if outline is not None and len(points) > 1:
        draw.line(points + [points[0]], fill=outline, width=width, joint="curve")
    img.alpha_composite(layer)


def _draw_ring(
    img: Image.Image,
    cx: float,
    cy: float,
    rx: float,
    ry: float,
    *,
    color: RGBA,
    width_px: int,
) -> None:
    layer, draw = _overlay_draw(img)
    draw.ellipse((cx - rx, cy - ry, cx + rx, cy + ry), outline=color, width=width_px)
    img.alpha_composite(layer)


def _star_points(
    cx: float,
    cy: float,
    inner_r: float,
    outer_r: float,
    spokes: int,
    seed: float = 0.0,
    aspect_y: float = 0.92,
) -> List[Point]:
    pts: List[Point] = []
    for i in range(spokes * 2):
        ang = (math.tau * i / (spokes * 2)) + seed
        rr = outer_r if i % 2 == 0 else inner_r
        pts.append((cx + math.cos(ang) * rr, cy + math.sin(ang) * rr * aspect_y))
    return pts


def _draw_smoke_puffs(
    img: Image.Image,
    cx: float,
    cy: float,
    radius: float,
    t: float,
    *,
    density: int = 7,
    seed: float = 0.0,
    color: RGBA = SMOKE,
) -> None:
    layer, draw = _overlay_draw(img)
    spread = radius * (0.48 + t * 0.78)
    for i in range(density):
        ang = (math.tau * i / max(1, density)) + seed * 0.67
        orbit = spread * (0.62 + 0.09 * math.sin(seed + i * 1.6))
        rx = radius * (0.22 + 0.05 * math.sin(seed * 1.1 + i * 1.9))
        ry = radius * (0.18 + 0.04 * math.cos(seed * 1.5 + i * 1.3))
        ox = cx + math.cos(ang) * orbit
        oy = cy + math.sin(ang) * orbit * 0.82
        draw.ellipse((ox - rx, oy - ry, ox + rx, oy + ry), fill=color)
    draw.ellipse(
        (
            cx - radius * 0.44,
            cy - radius * 0.38,
            cx + radius * 0.44,
            cy + radius * 0.38,
        ),
        fill=color,
    )
    img.alpha_composite(layer)


def _draw_embers(
    img: Image.Image,
    cx: float,
    cy: float,
    radius: float,
    t: float,
    *,
    count: int = 8,
    seed: float = 0.0,
    line_color: RGBA = EMBER,
    tip_color: RGBA = FLAME_A,
    length_scale: float = 1.0,
) -> None:
    layer, draw = _overlay_draw(img)
    for i in range(count):
        ang = (math.tau * i / max(1, count)) + seed * 0.53
        start_r = radius * (0.62 + 0.08 * math.sin(seed + i))
        end_r = (
            radius * (0.94 + 0.34 * t + 0.08 * math.cos(seed * 1.2 + i)) * length_scale
        )
        x1 = cx + math.cos(ang) * start_r
        y1 = cy + math.sin(ang) * start_r
        x2 = cx + math.cos(ang) * end_r
        y2 = cy + math.sin(ang) * end_r
        draw.line(
            (x1, y1, x2, y2), fill=line_color, width=max(1, int(round(radius * 0.08)))
        )
        rr = max(1.5, radius * 0.06)
        draw.ellipse((x2 - rr, y2 - rr, x2 + rr, y2 + rr), fill=tip_color)
    img.alpha_composite(layer)


def _draw_explosion_sprite(img: Image.Image, spec: Dict[str, float]) -> None:
    cx = spec["cx"]
    cy = spec["cy"]
    base_r = spec["base_r"]
    t = spec["t"]
    expansion = spec["expansion"]
    fade = spec["fade"]
    energy = spec["energy"]
    style = spec["style"]
    seed = spec.get("seed", 0.0)

    smoke_r = base_r * _lerp(0.34, 1.18, expansion)
    flame_outer = base_r * _lerp(0.18, 0.96, energy)
    flame_inner = flame_outer * spec.get("inner_ratio", 0.46)
    core_r = base_r * _lerp(0.10, 0.47, energy)

    smoke_color = _mul_alpha(spec.get("smoke_color", SMOKE), _lerp(0.62, 0.22, fade))
    smoke_dark = _mul_alpha(SMOKE_DARK, _lerp(0.56, 0.18, fade))
    ember_line = _mul_alpha(EMBER, _lerp(0.95, 0.16, fade))
    ember_tip = _mul_alpha(FLAME_A, _lerp(0.92, 0.10, fade))
    flame_a = _mul_alpha(FLAME_A, _lerp(1.00, 0.18, fade))
    flame_b = _mul_alpha(FLAME_B, _lerp(0.96, 0.12, fade))
    core = _mul_alpha(CORE, _lerp(1.00, 0.10, fade))
    core_hi = _mul_alpha(CORE_HI, _lerp(1.00, 0.05, fade))
    outline = _mul_alpha(OUTLINE, _lerp(0.95, 0.22, fade))

    density = int(spec.get("smoke_count", 7))
    ember_count = max(
        2, int(round(spec.get("ember_count", 8) * _lerp(1.0, 0.55, fade)))
    )
    spokes = int(spec.get("spokes", 6))

    if style in {"round", "shockwave", "smoke", "classic"}:
        _draw_smoke_puffs(
            img,
            cx,
            cy,
            smoke_r,
            expansion,
            density=density,
            seed=seed,
            color=smoke_color,
        )
    elif style == "star":
        _draw_smoke_puffs(
            img,
            cx,
            cy,
            smoke_r * 0.9,
            expansion,
            density=max(5, density - 1),
            seed=seed,
            color=smoke_color,
        )

    if style == "shockwave":
        ring_r = base_r * _lerp(0.42, 1.40, expansion)
        ring_alpha = _mul_alpha(RING, _lerp(0.85, 0.14, fade))
        _draw_ring(
            img,
            cx,
            cy,
            ring_r,
            ring_r * 0.90,
            color=ring_alpha,
            width_px=max(1, int(round(_s(_lerp(1.6, 0.8, fade))))),
        )

    if flame_outer > base_r * 0.12:
        if style == "smoke":
            flame_pts = _star_points(
                cx,
                cy,
                flame_inner * 0.82,
                flame_outer * 0.82,
                spokes=max(5, spokes - 1),
                seed=seed * 0.7,
                aspect_y=0.96,
            )
        else:
            flame_pts = _star_points(
                cx,
                cy,
                flame_inner,
                flame_outer,
                spokes=spokes,
                seed=seed * 0.7,
                aspect_y=0.92,
            )
        _composite_polygon(
            img, flame_pts, fill=flame_b, outline=outline, width=max(1, _s(0.7))
        )

        hot_outer = _star_points(
            cx,
            cy,
            flame_inner * 0.62,
            flame_outer * 0.64,
            spokes=spokes,
            seed=seed * 1.1 + 0.35,
            aspect_y=0.94,
        )
        _composite_polygon(img, hot_outer, fill=flame_a, outline=None)

    if core_r > base_r * 0.06:
        _composite_ellipse(
            img,
            (cx - core_r, cy - core_r, cx + core_r, cy + core_r),
            fill=core,
            outline=outline,
            width=max(1, _s(0.7)),
        )
        _composite_ellipse(
            img,
            (
                cx - core_r * 0.50,
                cy - core_r * 0.50,
                cx + core_r * 0.50,
                cy + core_r * 0.50,
            ),
            fill=core_hi,
        )

    if style == "smoke":
        _draw_smoke_puffs(
            img,
            cx,
            cy,
            smoke_r * 0.78,
            expansion,
            density=max(5, density - 1),
            seed=seed + 0.9,
            color=smoke_dark,
        )

    _draw_embers(
        img,
        cx,
        cy,
        smoke_r,
        expansion,
        count=ember_count,
        seed=seed,
        line_color=ember_line,
        tip_color=ember_tip,
        length_scale=_lerp(0.85, 1.08, expansion),
    )


def _frame_spec(anim: str, frame_idx: int, nframes: int) -> Dict[str, float]:
    p = frame_idx / max(1, nframes - 1)
    expansion = _ease(min(1.0, p / 0.58))
    fade = _ease(max(0.0, (p - 0.50) / 0.50))
    energy = math.sin(math.pi * p) ** 0.9
    cx = FRAME_SIZE[0] * 0.5
    cy = FRAME_SIZE[1] * 0.5
    spec: Dict[str, float] = {
        "cx": cx,
        "cy": cy,
        "t": p,
        "expansion": expansion,
        "fade": fade,
        "energy": energy,
        "style": "classic",
        "base_r": 30.0,
        "smoke_count": 7,
        "ember_count": 8,
        "spokes": 6,
        "seed": 0.0,
        "inner_ratio": 0.44,
    }
    if anim == "classic_burst":
        spec.update(
            {
                "style": "classic",
                "base_r": 27.0,
                "spokes": 6,
                "ember_count": 7,
                "seed": 0.15,
            }
        )
    elif anim == "burst_round":
        spec.update(
            {
                "style": "round",
                "base_r": 31.0,
                "smoke_count": 8,
                "spokes": 7,
                "ember_count": 9,
                "seed": 0.85,
            }
        )
    elif anim == "shockwave":
        spec.update(
            {
                "style": "shockwave",
                "base_r": 29.0,
                "spokes": 6,
                "ember_count": 6,
                "seed": 1.5,
                "inner_ratio": 0.36,
            }
        )
    elif anim == "smoke_burst":
        spec.update(
            {
                "style": "smoke",
                "base_r": 28.0,
                "smoke_count": 10,
                "spokes": 5,
                "ember_count": 6,
                "seed": 2.1,
                "inner_ratio": 0.50,
            }
        )
    elif anim == "starburst":
        spec.update(
            {
                "style": "star",
                "base_r": 30.0,
                "smoke_count": 6,
                "spokes": 8,
                "ember_count": 10,
                "seed": 2.8,
                "inner_ratio": 0.32,
            }
        )
    else:
        raise ValueError(f"unknown animation: {anim}")
    return spec


def _draw_frame(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    spec = _frame_spec(anim, frame_idx, nframes)
    spec["cx"] *= SUPER
    spec["cy"] *= SUPER
    spec["base_r"] *= SUPER
    _draw_explosion_sprite(img, spec)
    return img.resize(FRAME_SIZE, Image.Resampling.NEAREST)


def _frame_meta(anim: str, frame_idx: int, nframes: int) -> dict:
    spec = _frame_spec(anim, frame_idx, nframes)
    return {
        "anchors": {
            "origin": {"x": round(spec["cx"], 2), "y": round(spec["cy"], 2)},
            "core": {"x": round(spec["cx"], 2), "y": round(spec["cy"], 2)},
        },
        "effect": {
            "kind": anim,
            "progress": round(spec["t"], 4),
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
        crop_margin=6,
        actor_metadata=ACTOR_METADATA,
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
        crop_margin=6,
    )
