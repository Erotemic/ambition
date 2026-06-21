from __future__ import annotations

"""High-tech laser sword with side-mounted gun cluster.

The wielded weapon: a wide cyan leaf-blade mounted on a dense gunmetal
chassis with brass trim, three-barrel gatling cluster on the outboard
flank, and two thin "stinger" antennae sticking forward from the
crossguard. The guns fire shorter laser-sword projectiles (see the
`lasersword` target).

Rendered AXIS-ALIGNED with the blade pointing RIGHT (+X). The game
rotates the sprite at runtime around the ``grip`` anchor reported in
the YAML metadata, so rotation is a runtime transform — animations
in this sheet are FX states only.

Animations:

- `idle`: 8-frame energy hum. The blade visibly pulses width and
  brightness so it reads as alive.
- `fire`: 8-frame discharge. A bright white pulse travels from hilt
  to tip along the blade (suggesting projectile launch energy), the
  cluster muzzles flash, and the chassis recoils briefly.
- `dissipate`: 6-frame breakdown. The blade dims away while the
  metal chassis stays solid; pommel sparks scatter; the whole
  sprite fades on the tail end.
"""

import math
from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageDraw, ImageFilter

from ...authoring.tackon_sheet import build_sheet
from ...authoring import lasersword_common as lc

TARGET_NAME = "lasersword_with_guns"
SHEET_FILES = [f"{TARGET_NAME}_spritesheet.png", f"{TARGET_NAME}_spritesheet.yaml"]

# Working frame size in DESIGN units. Generous box; auto-crop hugs
# the actual silhouette. Scales with `lc.RENDER_SCALE`.
BASE_FRAME_SIZE = (320, 80)
FRAME_SIZE = (
    int(round(BASE_FRAME_SIZE[0] * lc.RENDER_SCALE)),
    int(round(BASE_FRAME_SIZE[1] * lc.RENDER_SCALE)),
)
AUTO_CROP = True

CANONICAL_ANGLE_DEG = 90.0  # blade-right axis-aligned

ROWS: List[Tuple[str, int, int]] = [
    ("idle", 8, 110),
    ("fire", 8, 65),
    ("dissipate", 6, 85),
]


def _idle_params(frame_idx: int, nframes: int):
    t = frame_idx / max(1, nframes)
    breath = 0.5 + 0.5 * math.sin(t * math.tau)
    return {
        "angle_deg": CANONICAL_ANGLE_DEG,
        "pulse": 0.82 + 0.28 * breath,
        "crystal_pulse": 0.80 + 0.30 * breath,
        "barrel_charge": 0.20 + 0.15 * breath,
        "barrel_flash": 0.0,
        "tip_flare": 0.0,
        "pulse_position": None,
        "pulse_intensity": 0.0,
        "offset": (0.0, 0.0),
        "fade": 0.0,
    }


def _fire_params(frame_idx: int, nframes: int):
    # White pulse travels from hilt (0.0) to past the tip (1.15) over
    # the animation. The dot fades in early and fades out as it
    # leaves the blade. Muzzle flash peaks on frame 1; recoil peaks
    # with the flash and settles by the last frames.
    t = frame_idx / max(1, nframes - 1)
    # Pulse position with a tiny lead before t=0 so the dot starts
    # at the hilt rather than off-screen on frame 0.
    pulse_position = lc.lerp(-0.05, 1.20, t)
    # Intensity bell-shaped: low at start, peak around t=0.45-0.6.
    intensity = math.sin(min(1.0, t) * math.pi) ** 0.75

    # Muzzle flash spikes at the start of fire — first two frames
    # carry the discharge, then it decays.
    if frame_idx == 0:
        flash = 0.85
    elif frame_idx == 1:
        flash = 1.00
    else:
        flash = max(
            0.0, 0.55 * math.cos(((frame_idx - 1) / max(1, nframes - 2)) * math.pi)
        )

    # Recoil along -blade-direction (so the chassis kicks back).
    # Strongest early, settles to zero by the end.
    recoil = -3.5 * flash

    # Blade-wide pulse: the body brightens during the discharge.
    blade_pulse = 1.10 + 0.45 * intensity
    crystal = 1.20 + 0.20 * flash

    return {
        "angle_deg": CANONICAL_ANGLE_DEG,
        "pulse": blade_pulse,
        "crystal_pulse": crystal,
        "barrel_charge": max(0.0, 0.85 - 0.55 * t),
        "barrel_flash": flash,
        "tip_flare": 0.55 * max(0.0, intensity) if pulse_position >= 0.95 else 0.0,
        "pulse_position": pulse_position,
        "pulse_intensity": intensity,
        "offset": (recoil, 0.0),
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
        "barrel_charge": max(0.0, 0.25 - 0.25 * t),
        "barrel_flash": 0.0,
        "tip_flare": 0.0,
        "pulse_position": None,
        "pulse_intensity": 0.0,
        "offset": (0.0, 0.0),
        "fade": t,
    }


def _params_for(animation: str, frame_idx: int, nframes: int):
    if animation == "idle":
        return _idle_params(frame_idx, nframes)
    if animation == "fire":
        return _fire_params(frame_idx, nframes)
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
        slash_streak=0.0,
        with_barrels=True,
        barrel_charge=p["barrel_charge"],
        barrel_flash=p["barrel_flash"],
        blade_fade=blade_fade,
        tip_flare=p["tip_flare"],
        pulse_position=p["pulse_position"],
        pulse_intensity=p["pulse_intensity"],
        extra_offset=p["offset"],
        frame_size=FRAME_SIZE,
    )

    if animation == "dissipate" and p["fade"] > 0.15:
        # Pommel sparks during disintegration.
        t = p["fade"]
        super_size = (FRAME_SIZE[0] * lc.SUPER, FRAME_SIZE[1] * lc.SUPER)
        spark_layer = Image.new("RGBA", super_size, (0, 0, 0, 0))
        sd = ImageDraw.Draw(spark_layer, "RGBA")
        anchors = lc.frame_anchors(
            angle_deg=p["angle_deg"],
            with_barrels=True,
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

    return base


def _anchor_dict(anchors: lc.WeaponAnchors) -> dict:
    return {
        "anchors": {
            "grip": {"x": round(anchors.grip[0], 2), "y": round(anchors.grip[1], 2)},
            "pommel": {
                "x": round(anchors.pommel[0], 2),
                "y": round(anchors.pommel[1], 2),
            },
            "guard": {"x": round(anchors.guard[0], 2), "y": round(anchors.guard[1], 2)},
            "muzzle": {
                "x": round(anchors.muzzle[0], 2),
                "y": round(anchors.muzzle[1], 2),
            },
            "tip": {"x": round(anchors.tip[0], 2), "y": round(anchors.tip[1], 2)},
        },
        "forward": {
            "x": round(anchors.forward[0], 4),
            "y": round(anchors.forward[1], 4),
        },
        "blade_angle_deg": round(anchors.angle_deg, 2),
    }


def frame_meta(animation: str, frame_idx: int, nframes: int) -> dict:
    p = _params_for(animation, frame_idx, nframes)
    anchors = lc.frame_anchors(
        angle_deg=p["angle_deg"],
        with_barrels=True,
        frame_size=FRAME_SIZE,
        offset_px=p["offset"],
    )
    return _anchor_dict(anchors)


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
        label_width=118,
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
