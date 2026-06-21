from __future__ import annotations

"""Procedural burning flying shark mount sprite sheet.

A tack-on target for the pirate sky-mount: a broad, side-view shark with a
combat harness, ember fins, and persistent fire streaming from its dorsal ridge
and tail. The goal is a readable gameplay silhouette rather than a fully
realistic shark.
"""

import math
from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageColor, ImageDraw, ImageFilter

from ...authoring.tackon_sheet import build_sheet

RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_NAME = "burning_flying_shark"
SHEET_FILES = [
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.yaml",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
]

ACTOR_METADATA = {
    "actor": {
        "character_id": "npc_burning_flying_shark",
        "display_name": "Burning Flying Shark",
    },
    "body": {
        "body_plan": "Flyer",
        "body_kind": "Floating",
        "mass_class": "Heavy",
        "locomotion_hint": "Fly",
        "traits": ["enemy", "pirate", "aerial", "mount", "beast", "no_hands", "fire"],
    },
    "capabilities": {
        "traversal": {
            "walk": False,
            "jump": None,
            "climb": None,
            "fly": True,
            "swim": None,
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
    "actions": {"default_preset": "peaceful_float"},
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "locomotion.fly": {"animation": "fly", "events": []},
        "action.melee.primary": {
            "animation": "chomp",
            "events": [
                {
                    "t": 0.24,
                    "event": "telegraph_peak",
                    "source": "burning_flying_shark",
                },
                {
                    "t": 0.36,
                    "event": "hitbox_active_start",
                    "source": "burning_flying_shark",
                },
                {
                    "t": 0.64,
                    "event": "hitbox_active_end",
                    "source": "burning_flying_shark",
                },
            ],
        },
        "action.special.dive": {
            "animation": "dive",
            "events": [
                {"t": 0.40, "event": "dive_commit", "source": "burning_flying_shark"},
            ],
        },
    },
    "sockets": {
        "mouth": {
            "source": "burning_flying_shark.geometry",
            "point": {"x": 148.0, "y": 66.0},
        },
        "head": {
            "source": "burning_flying_shark.geometry",
            "point": {"x": 132.0, "y": 56.0},
        },
        "tail": {
            "source": "burning_flying_shark.geometry",
            "point": {"x": 34.0, "y": 64.0},
        },
        "saddle": {
            "source": "burning_flying_shark.geometry",
            "point": {"x": 88.0, "y": 44.0},
        },
        "ember_origin": {
            "source": "burning_flying_shark.geometry",
            "point": {"x": 58.0, "y": 42.0},
        },
    },
    "tags": ["pirate", "aerial", "enemy", "beast", "fire"],
}

ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 135),
    ("fly", 8, 90),
    ("chomp", 6, 82),
    ("dive", 8, 82),
]

FRAME_SIZE = (192, 128)
SUPER = 4
W, H = FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER


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


def _draw_glow(
    base: Image.Image, points: list[Point], color: RGBA, blur: float = 4.0
) -> None:
    layer = Image.new("RGBA", base.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer, "RGBA")
    draw.polygon([_pt(x, y) for x, y in points], fill=color)
    layer = layer.filter(ImageFilter.GaussianBlur(radius=blur * SUPER / 2.0))
    base.alpha_composite(layer)


def _draw_flame_plume(
    base: Image.Image, anchor: Point, direction: Point, scale: float, phase: float
) -> None:
    ax, ay = anchor
    dx, dy = direction
    length = 14.0 * scale * (0.9 + 0.18 * math.sin(phase))
    width = 6.4 * scale * (0.92 + 0.15 * math.cos(phase * 1.7))
    nx, ny = -dy, dx
    outer = [
        (ax - nx * width * 0.50, ay - ny * width * 0.50),
        (ax + nx * width * 0.58, ay + ny * width * 0.58),
        (ax + dx * length * 0.38, ay + dy * length * 0.38),
        (
            ax + nx * width * 0.42 + dx * length * 0.75,
            ay + ny * width * 0.42 + dy * length * 0.75,
        ),
        (ax + dx * length, ay + dy * length),
        (
            ax - nx * width * 0.35 + dx * length * 0.72,
            ay - ny * width * 0.35 + dy * length * 0.72,
        ),
        (ax - dx * length * 0.05, ay - dy * length * 0.05),
    ]
    mid = [
        (ax - nx * width * 0.22, ay - ny * width * 0.22),
        (ax + nx * width * 0.25, ay + ny * width * 0.25),
        (ax + dx * length * 0.35, ay + dy * length * 0.35),
        (
            ax + nx * width * 0.18 + dx * length * 0.65,
            ay + ny * width * 0.18 + dy * length * 0.65,
        ),
        (ax + dx * length * 0.84, ay + dy * length * 0.84),
        (
            ax - nx * width * 0.16 + dx * length * 0.60,
            ay - ny * width * 0.16 + dy * length * 0.60,
        ),
    ]
    inner = [
        (ax - nx * width * 0.10, ay - ny * width * 0.10),
        (ax + nx * width * 0.10, ay + ny * width * 0.10),
        (ax + dx * length * 0.32, ay + dy * length * 0.32),
        (ax + dx * length * 0.58, ay + dy * length * 0.58),
        (ax + dx * length * 0.72, ay + dy * length * 0.72),
        (
            ax + dx * length * 0.30 - nx * width * 0.08,
            ay + dy * length * 0.30 - ny * width * 0.08,
        ),
    ]
    _draw_glow(base, outer, _rgba("#ff621d", 120), blur=5.0 * scale)
    _draw_glow(base, mid, _rgba("#ff9d2f", 150), blur=3.2 * scale)
    draw = ImageDraw.Draw(base, "RGBA")
    draw.polygon([_pt(x, y) for x, y in outer], fill=_rgba("#ff7a22", 170))
    draw.polygon([_pt(x, y) for x, y in mid], fill=_rgba("#ffb142", 195))
    draw.polygon([_pt(x, y) for x, y in inner], fill=_rgba("#fff3b0", 215))


def _draw_shark(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img, "RGBA")

    t = frame_idx / max(1, nframes)
    cyc = math.tau * t
    bob = math.sin(cyc) * (1.6 if anim == "idle" else 2.4)
    tail_swing = math.sin(cyc * (1.0 if anim == "idle" else 1.35)) * (
        4.0 if anim == "fly" else 5.4
    )
    wing_flap = math.sin(cyc * (1.25 if anim == "idle" else 1.8)) * (
        5.5 if anim != "dive" else 2.8
    )
    mouth_open = 1.0 if anim == "chomp" and frame_idx in {1, 2, 3, 4} else 0.0
    if anim == "dive":
        nose_drop = 5.0 + 2.0 * math.sin(cyc * 1.2)
        body_pitch = -4.0
    else:
        nose_drop = 0.0
        body_pitch = 0.0
    if anim == "chomp":
        mouth_open = max(mouth_open, 0.18 + 0.82 * math.sin(t * math.pi))

    # Drop shadow intentionally omitted — the shark is airborne;
    # an under-body ground shadow reads as a grounded prop and the
    # arena lighting doesn't justify it. (Previous revisions added
    # an "ember cloud" ellipse here for visual weight; removed at
    # the user's request because it implied a floor that isn't
    # there.)

    # Body proportions.
    cx = 88.0
    cy = 62.0 + bob
    body_left = 50.0
    body_right = 126.0
    top = cy - 18.0 + body_pitch * 0.18
    bottom = cy + 18.0 + body_pitch * 0.12

    # Tail and fins behind the body.
    tail_base = (body_left + 6.0, cy + 1.0)
    tail_tip = (28.0, cy + tail_swing)
    tail_upper = [
        tail_base,
        (41.0, cy - 5.0),
        (tail_tip[0], tail_tip[1] - 16.0),
        (34.0, cy - 2.5),
    ]
    tail_lower = [
        tail_base,
        (40.0, cy + 7.0),
        (tail_tip[0], tail_tip[1] + 18.0),
        (33.0, cy + 5.0),
    ]
    rear_fin = [
        (76.0, cy + 3.0),
        (58.0, cy + 14.0 + wing_flap * 0.45),
        (85.0, cy + 15.0),
    ]
    draw.polygon(
        [_pt(*p) for p in tail_upper], fill=_rgba("#4a5968"), outline=_rgba("#182028")
    )
    draw.polygon(
        [_pt(*p) for p in tail_lower], fill=_rgba("#404d5c"), outline=_rgba("#182028")
    )
    draw.polygon(
        [_pt(*p) for p in rear_fin], fill=_rgba("#d5682f"), outline=_rgba("#3d1d16")
    )

    # Main body.
    draw.ellipse(
        _box(body_left, top, body_right, bottom),
        fill=_rgba("#596978"),
        outline=_rgba("#15202c"),
        width=_s(1.6),
    )
    draw.ellipse(
        _box(body_left + 4.0, top + 3.0, body_right - 6.0, bottom - 2.0),
        fill=_rgba("#667888"),
    )
    draw.pieslice(
        _box(78.0, top + 2.0, 132.0, bottom - 2.0), 80, 280, fill=_rgba("#495867", 120)
    )
    draw.pieslice(
        _box(58.0, cy - 11.0, 116.0, cy + 14.0), 108, 248, fill=_rgba("#8191a1", 115)
    )

    # Shark nose / head extension.
    head_pts = [
        (108.0, cy - 18.0 + body_pitch * 0.12),
        (141.5, cy - 10.0 + nose_drop * 0.12),
        (153.0, cy - 3.0 + nose_drop * 0.65),
        (157.0, cy + 1.0 + nose_drop),
        (153.0, cy + 6.0 + nose_drop * 1.15),
        (140.5, cy + 12.0 + nose_drop * 0.42),
        (105.0, cy + 18.0),
    ]
    draw.polygon(
        [_pt(*p) for p in head_pts], fill=_rgba("#627385"), outline=_rgba("#15202c")
    )

    # Belly.
    belly = [
        (75.0, cy + 7.0),
        (104.0, cy + 11.0),
        (136.0, cy + 11.0 + nose_drop * 0.45),
        (153.0, cy + 6.0 + nose_drop),
        (138.0, cy + 14.0),
        (113.0, cy + 16.0),
        (88.0, cy + 17.0),
    ]
    draw.polygon([_pt(*p) for p in belly], fill=_rgba("#c0c9cf", 210))

    # Mouth.
    upper_mouth = [(124.0, cy + 0.5), (154.0, cy + 1.0 + nose_drop * 0.72)]
    draw.line([_pt(*p) for p in upper_mouth], fill=_rgba("#172028"), width=_s(1.2))
    if mouth_open > 0.05:
        jaw = [
            (123.0, cy + 1.0),
            (150.5, cy + 7.5 + 8.5 * mouth_open + nose_drop * 0.55),
            (135.0, cy + 8.0 + 7.0 * mouth_open),
        ]
        draw.polygon(
            [_pt(*p) for p in jaw], fill=_rgba("#8a4140"), outline=_rgba("#1d1214")
        )
        for tooth_x in (131.0, 137.5, 144.0, 149.0):
            tooth = [
                (tooth_x, cy + 1.8),
                (tooth_x + 1.9, cy + 5.0),
                (tooth_x + 3.8, cy + 1.7),
            ]
            draw.polygon([_pt(*p) for p in tooth], fill=_rgba("#f5efe2"))
    else:
        for tooth_x in (129.0, 137.0, 145.0):
            tooth = [
                (tooth_x, cy + 1.5),
                (tooth_x + 1.6, cy + 4.2),
                (tooth_x + 3.2, cy + 1.4),
            ]
            draw.polygon([_pt(*p) for p in tooth], fill=_rgba("#ece7db"))

    # Eye and gill slits.
    draw.ellipse(
        _box(119.0, cy - 8.0 + nose_drop * 0.35, 126.5, cy - 1.5 + nose_drop * 0.35),
        fill=_rgba("#1c0b08"),
    )
    eye_glow = Image.new("RGBA", img.size, (0, 0, 0, 0))
    eg = ImageDraw.Draw(eye_glow, "RGBA")
    eg.ellipse(
        _box(120.4, cy - 6.6 + nose_drop * 0.35, 124.8, cy - 2.6 + nose_drop * 0.35),
        fill=_rgba("#ff8b29", 220),
    )
    eye_glow = eye_glow.filter(ImageFilter.GaussianBlur(radius=2.6))
    img.alpha_composite(eye_glow)
    draw.ellipse(
        _box(121.0, cy - 6.0 + nose_drop * 0.35, 124.1, cy - 3.2 + nose_drop * 0.35),
        fill=_rgba("#ffd76d"),
    )
    for gx in (112.0, 116.0, 120.0):
        draw.arc(
            _box(gx, cy - 4.0, gx + 7.0, cy + 8.0),
            260,
            78,
            fill=_rgba("#314150"),
            width=_s(0.8),
        )

    # Wings / pectoral fins and dorsal fin.
    wing_back = [
        (82.0, cy - 1.5),
        (60.0, cy - 21.0 - wing_flap),
        (95.0, cy - 9.0),
    ]
    wing_front = [
        (94.0, cy + 0.5),
        (62.0, cy + 12.0 + wing_flap),
        (103.0, cy + 10.0),
    ]
    dorsal = [
        (87.0, cy - 8.0),
        (94.0, cy - 31.0 - abs(wing_flap) * 0.35),
        (105.0, cy - 8.0),
    ]
    draw.polygon(
        [_pt(*p) for p in wing_back], fill=_rgba("#c85d2e"), outline=_rgba("#3d1d16")
    )
    draw.polygon(
        [_pt(*p) for p in wing_front], fill=_rgba("#da6e33"), outline=_rgba("#3d1d16")
    )
    draw.polygon(
        [_pt(*p) for p in dorsal], fill=_rgba("#de6b2b"), outline=_rgba("#4a1f13")
    )

    # Pirate harness / saddle.
    strap = _rgba("#5a3f28")
    brass = _rgba("#d0a85e")
    steel = _rgba("#b7c2cd")
    draw.rectangle(
        _box(78.0, cy - 10.5, 101.0, cy + 0.5),
        fill=_rgba("#4c3424"),
        outline=_rgba("#1b1210"),
    )
    draw.rectangle(
        _box(82.0, cy - 17.0, 95.0, cy - 10.0),
        fill=_rgba("#77533a"),
        outline=_rgba("#1b1210"),
    )
    draw.line(_box(77.0, cy - 3.0, 106.0, cy - 1.0), fill=strap, width=_s(1.5))
    draw.line(_box(88.0, cy - 13.0, 88.0, cy + 6.0), fill=strap, width=_s(1.2))
    draw.line(_box(99.0, cy - 11.0, 99.0, cy + 7.0), fill=strap, width=_s(1.2))
    draw.ellipse(_box(86.0, cy - 2.2, 89.8, cy + 1.6), fill=brass)
    draw.ellipse(_box(97.0, cy - 2.2, 100.8, cy + 1.6), fill=brass)
    draw.line(_box(101.0, cy - 13.0, 111.0, cy - 15.5), fill=steel, width=_s(0.9))
    draw.line(_box(111.0, cy - 15.5, 117.0, cy - 12.0), fill=steel, width=_s(0.9))

    # Flame plumes.
    flame_anchors = [
        ((95.0, cy - 26.0), (-0.25, -1.0), 1.0, cyc + 0.2),
        ((80.0, cy - 16.5), (-0.95, -0.28), 1.15, cyc + 1.1),
        ((74.0, cy + 13.0), (-0.92, 0.24), 0.95, cyc + 2.0),
        ((36.0, tail_tip[1] + 1.0), (-1.0, 0.10), 1.12, cyc + 1.6),
    ]
    for anchor, direction, scale, phase in flame_anchors:
        _draw_flame_plume(img, anchor, direction, scale, phase)

    # Ember specks.
    ember = Image.new("RGBA", img.size, (0, 0, 0, 0))
    ed = ImageDraw.Draw(ember, "RGBA")
    for i in range(12):
        ex = 58.0 - i * 5.8 + math.sin(cyc + i) * 1.7
        ey = cy - 22.0 + (i % 5) * 7.0 + math.cos(cyc * 1.8 + i * 0.7) * 1.6
        r = 0.85 + (i % 3) * 0.34
        ed.ellipse(_box(ex - r, ey - r, ex + r, ey + r), fill=_rgba("#ffb451", 160))
    ember = ember.filter(ImageFilter.GaussianBlur(radius=1.0))
    img.alpha_composite(ember)

    return _downsample(img)


def render_frame(animation: str, frame_idx: int, nframes: int) -> Image.Image:
    return _draw_shark(animation, frame_idx, nframes)


def render(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=render_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        label_width=118,
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
