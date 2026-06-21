from __future__ import annotations

"""Laser-sword projectile fired by the `lasersword_with_guns` weapon.

Mechanically a fast-traveling projectile; visually a smaller copy of
the wielded weapon with the gun chassis stripped off — it's a SWORD
flying through the air, not the gun assembly that fired it.

The projectile is rendered AXIS-ALIGNED with the blade pointing
RIGHT (+X). The game rotates the sprite at runtime around the
``pommel`` anchor reported in the YAML metadata, so we don't need to
bake spin frames into the spritesheet.

Animations:

- `idle`: 8-frame energy hum. Blade pulses width / brightness so it
  reads as a live energy edge rather than a static painted blade.
- `dissipate`: 6-frame impact fade-out. The blade thins (only the
  blade — the metal hilt stays solid), pommel sparks scatter, and
  the whole sprite fades to nothing on the last frame.
"""

import math
from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageDraw, ImageFilter

from ...authoring.tackon_sheet import build_sheet
from ...authoring import lasersword_common as lc

TARGET_NAME = "lasersword"
SHEET_FILES = [f"{TARGET_NAME}_spritesheet.png", f"{TARGET_NAME}_spritesheet.yaml"]

# Working frame size in DESIGN units. Generous box; auto-crop hugs
# the actual silhouette. Scales with `lc.RENDER_SCALE`.
BASE_FRAME_SIZE = (320, 64)
FRAME_SIZE = (
    int(round(BASE_FRAME_SIZE[0] * lc.RENDER_SCALE)),
    int(round(BASE_FRAME_SIZE[1] * lc.RENDER_SCALE)),
)
AUTO_CROP = True

# PIL Image.rotate is CCW; blade-local +Y is "down" in image coords,
# so a 90° CCW rotation maps blade-local +Y to image +X (blade
# points right).
CANONICAL_ANGLE_DEG = 90.0

ROWS: List[Tuple[str, int, int]] = [
    ("idle", 8, 110),
    ("dissipate", 6, 80),
]


def _idle_params(frame_idx: int, nframes: int):
    # Visible energy hum: pulse the blade width / brightness on a
    # full sine cycle across the animation. Amplitude is high enough
    # that the blade is obviously "alive".
    t = frame_idx / max(1, nframes)
    breath = 0.5 + 0.5 * math.sin(t * math.tau)
    return {
        "angle_deg": CANONICAL_ANGLE_DEG,
        "pulse": 0.82 + 0.28 * breath,
        "crystal_pulse": 0.80 + 0.30 * breath,
        "slash_streak": 0.0,
        "fade": 0.0,
    }


def _dissipate_params(frame_idx: int, nframes: int):
    t = frame_idx / max(1, nframes - 1)
    pulse = max(0.05, 1.10 - 1.05 * t)
    crystal = max(0.05, 1.40 * (1.0 - lc.ease_in_out(t)) + 0.05)
    return {
        "angle_deg": CANONICAL_ANGLE_DEG,
        "pulse": pulse,
        "crystal_pulse": crystal,
        "slash_streak": 0.0,
        "fade": t,
    }


def _params_for(animation: str, frame_idx: int, nframes: int):
    if animation == "idle":
        return _idle_params(frame_idx, nframes)
    if animation == "dissipate":
        return _dissipate_params(frame_idx, nframes)
    raise ValueError(f"unknown animation: {animation}")


def render_frame(animation: str, frame_idx: int, nframes: int) -> Image.Image:
    p = _params_for(animation, frame_idx, nframes)
    blade_fade = p["fade"] if animation == "dissipate" else 0.0
    base = lc.draw_weapon(
        angle_deg=p["angle_deg"],
        pulse=p["pulse"],
        crystal_pulse=p["crystal_pulse"],
        slash_streak=p["slash_streak"],
        with_barrels=False,
        blade_fade=blade_fade,
        frame_size=FRAME_SIZE,
    )

    if animation == "dissipate" and p["fade"] > 0.15:
        # Pommel sparks scatter as the projectile dissolves.
        t = p["fade"]
        super_size = (FRAME_SIZE[0] * lc.SUPER, FRAME_SIZE[1] * lc.SUPER)
        spark_layer = Image.new("RGBA", super_size, (0, 0, 0, 0))
        sd = ImageDraw.Draw(spark_layer, "RGBA")
        anchors = lc.frame_anchors(
            angle_deg=p["angle_deg"],
            with_barrels=False,
            frame_size=FRAME_SIZE,
        )
        cx = anchors.pommel[0] * lc.SUPER
        cy = anchors.pommel[1] * lc.SUPER
        for k in range(12):
            ang = k / 12.0 * math.tau + t * math.pi
            r = lc.s(6.0 + 16.0 * t)
            sx = cx + math.cos(ang) * r
            sy = cy + math.sin(ang) * r
            rad = lc.s(1.4 * (1.0 - t))
            sd.ellipse(
                (sx - rad, sy - rad, sx + rad, sy + rad),
                fill=lc.with_alpha(lc.BLADE_HALO, int(220 * (1.0 - t))),
            )
        spark_layer = spark_layer.filter(
            ImageFilter.GaussianBlur(radius=max(2, int(lc.SUPER * 0.55)))
        )
        spark_layer = spark_layer.resize(FRAME_SIZE, Image.Resampling.LANCZOS)
        base.alpha_composite(spark_layer)
        # Only the blade fades during dissipate (handled by
        # `blade_fade` in `draw_weapon`); the metal hilt stays solid.
        # The game removes the sprite from rendering after the
        # animation ends rather than us cross-fading the chassis.

    return base


def frame_meta(animation: str, frame_idx: int, nframes: int) -> dict:
    p = _params_for(animation, frame_idx, nframes)
    anchors = lc.frame_anchors(
        angle_deg=p["angle_deg"],
        with_barrels=False,
        frame_size=FRAME_SIZE,
    )
    return {
        "anchors": {
            "grip": {"x": round(anchors.grip[0], 2), "y": round(anchors.grip[1], 2)},
            "pommel": {
                "x": round(anchors.pommel[0], 2),
                "y": round(anchors.pommel[1], 2),
            },
            "guard": {"x": round(anchors.guard[0], 2), "y": round(anchors.guard[1], 2)},
            "tip": {"x": round(anchors.tip[0], 2), "y": round(anchors.tip[1], 2)},
        },
        "forward": {
            "x": round(anchors.forward[0], 4),
            "y": round(anchors.forward[1], 4),
        },
        "blade_angle_deg": round(anchors.angle_deg, 2),
    }


def render(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    frame_size = opts.get("frame_size") or FRAME_SIZE
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=render_frame,
        out_dir=out_dir,
        frame_size=frame_size,
        label_width=110,
        frame_meta_fn=frame_meta,
        auto_crop=opts.get("auto_crop", AUTO_CROP),
    )
    return [
        outputs["canonical"],
        outputs["canonical_transparent"],
        outputs["spritesheet"],
        outputs["yaml"],
        outputs["preview"],
    ]
