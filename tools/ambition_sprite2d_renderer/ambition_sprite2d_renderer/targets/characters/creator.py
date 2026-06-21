from __future__ import annotations

"""Bespoke procedural sprite for the Creator character.

The Creator is intentionally more distinctive than the simple toon-rig cast: a
robed / tailored figure with an asymmetric mantle, high collar, luminous chest
sigil, and a small geometric halo frame. The result is meant to read as an
important authored NPC for the intro rather than a generic townsperson.
"""

import math
from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageColor, ImageDraw, ImageFilter

from ...authoring.tackon_sheet import build_sheet

ACTOR_METADATA = {
    "actor": {
        "character_id": "npc_creator",
        "display_name": "Creator",
        "actor_id": "creator",
    },
    "body": {
        "body_plan": "HumanoidBiped",
        "body_kind": "Standard",
        "mass_class": "Medium",
        "traits": ["story", "humanoid", "intro", "story", "creator"],
        "locomotion_hint": "Walk",
    },
    "capabilities": {
        "traversal": {
            "walk": True,
            "jump": None,
            "climb": None,
            "fly": None,
            "swim": None,
            "crawl": None,
            "use_lifts": True,
            "door_access": ["public"],
        },
        "interactions": {
            "talk": True,
            "trade": None,
            "carry": None,
            "open_doors": ["public"],
        },
    },
    "brain": {"default_preset": "patrol_peaceful"},
    "actions": {"default_preset": "peaceful"},
    "visual": {"default_pose": "idle"},
    "tags": ["story", "humanoid", "intro", "story", "creator"],
    "sockets": {
        "head": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 64.0, "y": 24.0},
        },
        "chest": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 64.0, "y": 54.0},
        },
        "hand_l": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 48.0, "y": 64.0},
        },
        "hand_r": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 80.0, "y": 64.0},
        },
        "speech_bubble": {
            "source": "explicit.profile.humanoid",
            "point": {"x": 64.0, "y": 8.0},
        },
    },
    "animation_bindings": {
        "default": {"animation": "idle", "events": []},
        "locomotion.walk": {"animation": "walk", "events": []},
        "interaction.talk": {"animation": "talk", "events": []},
        "interaction.use": {"animation": "interact", "events": []},
    },
    "missing_information": [
        "Creator story-presence state is intentionally authored outside the "
        "renderer sidecar."
    ],
}


RGBA = Tuple[int, int, int, int]

TARGET_NAME = "creator"
SHEET_FILES = [f"{TARGET_NAME}_spritesheet.png", f"{TARGET_NAME}_spritesheet.yaml"]

ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 145),
    ("speak", 6, 110),
    ("gesture", 6, 100),
    ("walk", 8, 95),
]

FRAME_SIZE = (160, 192)
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


def _glow_ellipse(base: Image.Image, bbox, fill: RGBA, blur: float = 4.0) -> None:
    layer = Image.new("RGBA", base.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer, "RGBA")
    draw.ellipse(bbox, fill=fill)
    layer = layer.filter(ImageFilter.GaussianBlur(radius=blur * SUPER / 2.0))
    base.alpha_composite(layer)


def _glow_polygon(base: Image.Image, pts, fill: RGBA, blur: float = 4.0) -> None:
    layer = Image.new("RGBA", base.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer, "RGBA")
    draw.polygon([_pt(x, y) for x, y in pts], fill=fill)
    layer = layer.filter(ImageFilter.GaussianBlur(radius=blur * SUPER / 2.0))
    base.alpha_composite(layer)


def _draw_creator(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img, "RGBA")

    t = frame_idx / max(1, nframes)
    cyc = math.tau * t
    bob = math.sin(cyc) * (1.0 if anim != "walk" else 1.5)
    sway = math.sin(cyc * 0.6) * (0.9 if anim == "idle" else 1.35)
    halo_phase = cyc * (0.65 if anim == "idle" else 1.0)
    speak_open = 0.0
    if anim == "speak":
        speak_open = 0.15 + 0.85 * (0.5 + 0.5 * math.sin(cyc * 1.8))
    gesture = 0.0
    if anim == "gesture":
        gesture = 0.25 + 0.75 * (0.5 + 0.5 * math.sin(cyc * 1.4 - 0.5))
    walk_phase = cyc
    step = math.sin(walk_phase)

    # Palette.
    coat_dark = _rgba("#22253d")
    coat_mid = _rgba("#343b63")
    coat_light = _rgba("#55639c")
    lining = _rgba("#8a5d9d")
    mantle = _rgba("#643f7d")
    brass = _rgba("#d6b370")
    cloth_light = _rgba("#9ea9d1")
    skin = _rgba("#d7b3a0")
    hair = _rgba("#f1eee6")
    shadow = _rgba("#10131f")
    glow_cyan = _rgba("#8befff")
    glow_violet = _rgba("#b88cff")

    # Ground shadow removed; the in-game renderer composites the
    # Creator over scene geometry that provides ground contact.

    base_x = 79.0
    base_y = 164.0 + bob
    torso_y = 92.0 + bob
    head_y = 52.0 + bob - sway * 0.5

    # Legs and boots.
    left_leg_dx = -5.5 + (step * 3.0 if anim == "walk" else -0.6)
    right_leg_dx = 5.0 - (step * 3.0 if anim == "walk" else -0.2)
    left_foot_y = base_y + (2.5 if anim == "walk" and step > 0 else 0.0)
    right_foot_y = base_y + (2.5 if anim == "walk" and step < 0 else 0.0)
    for leg_dx, foot_y, coat_slit in [
        (left_leg_dx, left_foot_y, -4.0),
        (right_leg_dx, right_foot_y, 4.0),
    ]:
        draw.rectangle(
            _box(base_x + leg_dx - 4.2, 120 + bob, base_x + leg_dx + 4.0, foot_y - 8.0),
            fill=_rgba("#2d314f"),
            outline=shadow,
        )
        draw.rectangle(
            _box(base_x + leg_dx - 4.7, foot_y - 11.0, base_x + leg_dx + 5.2, foot_y),
            fill=_rgba("#4f5565"),
            outline=shadow,
        )
        draw.line(
            _box(
                base_x + coat_slit,
                105 + bob,
                base_x + coat_slit + leg_dx * 0.18,
                128 + bob,
            ),
            fill=shadow,
            width=_s(0.6),
        )

    # Main coat silhouette.
    coat = [
        (60.0, 85.0 + bob),
        (72.0, 77.0 + bob),
        (89.0, 75.0 + bob),
        (99.0, 80.0 + bob),
        (103.0, 96.0 + bob),
        (108.0, 126.0 + bob),
        (112.0, 156.0 + bob),
        (96.0, 164.0 + bob),
        (88.0, 118.0 + bob),
        (79.0, 165.0 + bob),
        (70.0, 118.0 + bob),
        (59.0, 160.0 + bob),
        (45.0, 153.0 + bob),
        (50.0, 126.0 + bob),
        (54.0, 98.0 + bob),
    ]
    draw.polygon([_pt(*p) for p in coat], fill=coat_dark, outline=shadow)
    inner_coat = [
        (64.0, 88.0 + bob),
        (77.0, 80.0 + bob),
        (89.0, 79.0 + bob),
        (97.0, 84.0 + bob),
        (101.0, 98.0 + bob),
        (104.0, 126.0 + bob),
        (107.0, 151.0 + bob),
        (95.0, 157.0 + bob),
        (87.0, 116.0 + bob),
        (79.0, 160.0 + bob),
        (71.0, 115.0 + bob),
        (62.0, 154.0 + bob),
        (50.0, 148.0 + bob),
        (56.0, 100.0 + bob),
    ]
    draw.polygon([_pt(*p) for p in inner_coat], fill=coat_mid)
    draw.polygon(
        [_pt(72.0, 112.0 + bob), _pt(79.0, 160.0 + bob), _pt(86.0, 112.0 + bob)],
        fill=lining,
    )

    # High collar and lapels.
    collar_left = [
        (67.0, 77.0 + bob),
        (77.0, 63.0 + bob),
        (84.0, 80.0 + bob),
        (77.0, 93.0 + bob),
    ]
    collar_right = [
        (92.0, 77.0 + bob),
        (82.0, 63.0 + bob),
        (76.0, 80.0 + bob),
        (82.0, 94.0 + bob),
    ]
    draw.polygon([_pt(*p) for p in collar_left], fill=coat_light, outline=shadow)
    draw.polygon([_pt(*p) for p in collar_right], fill=coat_light, outline=shadow)

    # Asymmetric mantle / shoulder piece.
    mantle_poly = [
        (58.0, 77.0 + bob),
        (71.0, 68.0 + bob - sway * 0.2),
        (93.0, 68.0 + bob),
        (104.0, 77.0 + bob),
        (100.0, 88.0 + bob),
        (75.0, 90.0 + bob),
        (62.0, 86.0 + bob),
    ]
    draw.polygon([_pt(*p) for p in mantle_poly], fill=mantle, outline=shadow)
    epaulet = [
        (56.0, 78.0 + bob),
        (45.0, 88.0 + bob),
        (52.0, 100.0 + bob),
        (70.0, 90.0 + bob),
    ]
    draw.polygon([_pt(*p) for p in epaulet], fill=_rgba("#7b4d96"), outline=shadow)
    draw.line(_box(61.0, 83.0 + bob, 98.0, 83.0 + bob), fill=brass, width=_s(0.8))

    # Arms.
    left_arm_angle = -0.35 - gesture * 0.85
    right_arm_angle = 0.2 + (0.15 if anim == "speak" else 0.0)
    left_shoulder = (61.0, 90.0 + bob)
    right_shoulder = (97.0, 90.0 + bob)

    def limb_pts(origin, ang, upper_len, fore_len):
        ox, oy = origin
        ex = ox + math.cos(ang) * upper_len
        ey = oy + math.sin(ang) * upper_len
        hx = ex + math.cos(ang + 0.2) * fore_len
        hy = ey + math.sin(ang + 0.2) * fore_len
        return (ox, oy), (ex, ey), (hx, hy)

    l0, l1, l2 = limb_pts(left_shoulder, left_arm_angle, 16.0, 13.0)
    r0, r1, r2 = limb_pts(right_shoulder, right_arm_angle, 18.0, 12.0)
    draw.line(
        (_s(l0[0]), _s(l0[1]), _s(l1[0]), _s(l1[1])), fill=coat_mid, width=_s(3.3)
    )
    draw.line(
        (_s(l1[0]), _s(l1[1]), _s(l2[0]), _s(l2[1])), fill=coat_light, width=_s(2.9)
    )
    draw.line(
        (_s(r0[0]), _s(r0[1]), _s(r1[0]), _s(r1[1])), fill=coat_mid, width=_s(3.1)
    )
    draw.line(
        (_s(r1[0]), _s(r1[1]), _s(r2[0]), _s(r2[1])), fill=coat_light, width=_s(2.7)
    )
    # Hands.
    draw.ellipse(
        _box(l2[0] - 3.5, l2[1] - 3.5, l2[0] + 3.5, l2[1] + 3.5),
        fill=skin,
        outline=shadow,
    )
    draw.ellipse(
        _box(r2[0] - 3.3, r2[1] - 3.3, r2[0] + 3.3, r2[1] + 3.3),
        fill=skin,
        outline=shadow,
    )

    # Right hand held codex / tablet.
    codex = [
        (r2[0] - 7.0, r2[1] - 8.0),
        (r2[0] + 3.0, r2[1] - 10.0),
        (r2[0] + 7.0, r2[1] + 4.0),
        (r2[0] - 4.0, r2[1] + 6.0),
    ]
    draw.polygon([_pt(*p) for p in codex], fill=_rgba("#d8dce8"), outline=shadow)
    draw.line(
        _box(r2[0] - 2.0, r2[1] - 7.0, r2[0] + 2.0, r2[1] + 4.0),
        fill=_rgba("#a59bcf"),
        width=_s(0.55),
    )

    # Head.
    draw.ellipse(
        _box(66.0, head_y - 1.0, 96.0, head_y + 30.0), fill=skin, outline=shadow
    )
    # Hair.
    hair_back = [
        (66.0, head_y + 5.0),
        (72.0, head_y - 5.0),
        (88.0, head_y - 7.0),
        (96.0, head_y + 2.0),
        (94.0, head_y + 22.0),
        (84.0, head_y + 28.0),
        (72.0, head_y + 26.0),
    ]
    draw.polygon([_pt(*p) for p in hair_back], fill=hair, outline=shadow)
    hair_front = [
        (69.0, head_y + 1.0),
        (79.0, head_y - 6.0),
        (92.0, head_y + 1.5),
        (88.0, head_y + 7.0),
        (76.0, head_y + 8.0),
    ]
    draw.polygon([_pt(*p) for p in hair_front], fill=_rgba("#faf8f2"), outline=shadow)

    # Face.
    eye_y = head_y + 11.0
    draw.line(_box(75.0, eye_y, 79.5, eye_y), fill=shadow, width=_s(0.55))
    draw.line(_box(84.0, eye_y, 88.5, eye_y), fill=shadow, width=_s(0.55))
    draw.arc(
        _box(77.5, head_y + 15.0, 85.5, head_y + 23.0 + speak_open * 2.8),
        20,
        160,
        fill=_rgba("#7e3f56"),
        width=_s(0.55),
    )
    draw.line(
        _box(81.0, head_y + 11.0, 80.0, head_y + 16.0),
        fill=_rgba("#b98f86"),
        width=_s(0.35),
    )

    # Halo frame behind the head.
    halo = Image.new("RGBA", img.size, (0, 0, 0, 0))
    hd = ImageDraw.Draw(halo, "RGBA")
    halo_cx = 81.0
    halo_cy = head_y + 10.0
    outer_r = 23.0
    inner_r = 18.0
    hd.ellipse(
        _box(
            halo_cx - outer_r, halo_cy - outer_r, halo_cx + outer_r, halo_cy + outer_r
        ),
        outline=_rgba("#6fdfff", 180),
        width=_s(1.0),
    )
    hd.ellipse(
        _box(
            halo_cx - inner_r, halo_cy - inner_r, halo_cx + inner_r, halo_cy + inner_r
        ),
        outline=_rgba("#cc9cff", 160),
        width=_s(0.7),
    )
    for i in range(6):
        ang = halo_phase + i * (math.tau / 6.0)
        x1 = halo_cx + math.cos(ang) * 15.0
        y1 = halo_cy + math.sin(ang) * 15.0
        x2 = halo_cx + math.cos(ang) * 24.5
        y2 = halo_cy + math.sin(ang) * 24.5
        hd.line(
            (_s(x1), _s(y1), _s(x2), _s(y2)), fill=_rgba("#a7f3ff", 160), width=_s(0.55)
        )
        shard = [
            (x2, y2),
            (x2 + math.cos(ang + 0.45) * 3.5, y2 + math.sin(ang + 0.45) * 3.5),
            (x2 + math.cos(ang - 0.45) * 3.5, y2 + math.sin(ang - 0.45) * 3.5),
        ]
        hd.polygon([_pt(*p) for p in shard], fill=_rgba("#d5c0ff", 135))
    halo = halo.filter(ImageFilter.GaussianBlur(radius=2.0))
    img.alpha_composite(halo)

    # Chest sigil / clasp.
    sigil_cx, sigil_cy = 80.0, 95.0 + bob
    _glow_ellipse(
        img,
        _box(sigil_cx - 7.0, sigil_cy - 7.0, sigil_cx + 7.0, sigil_cy + 7.0),
        _rgba("#7ceeff", 105),
        blur=4.6,
    )
    _glow_polygon(
        img,
        [
            (sigil_cx, sigil_cy - 6.0),
            (sigil_cx + 6.0, sigil_cy),
            (sigil_cx, sigil_cy + 6.0),
            (sigil_cx - 6.0, sigil_cy),
        ],
        _rgba("#b88cff", 80),
        blur=3.5,
    )
    draw.polygon(
        [
            _pt(sigil_cx, sigil_cy - 4.0),
            _pt(sigil_cx + 4.0, sigil_cy),
            _pt(sigil_cx, sigil_cy + 4.0),
            _pt(sigil_cx - 4.0, sigil_cy),
        ],
        fill=glow_cyan,
        outline=_rgba("#effdff"),
    )
    draw.ellipse(
        _box(sigil_cx - 2.1, sigil_cy - 2.1, sigil_cx + 2.1, sigil_cy + 2.1),
        fill=glow_violet,
    )

    # Gold trim and sash.
    draw.line(_box(73.0, 81.0 + bob, 79.0, 156.0 + bob), fill=brass, width=_s(0.55))
    draw.line(_box(87.0, 81.0 + bob, 81.0, 156.0 + bob), fill=brass, width=_s(0.55))
    sash = [
        (90.0, 100.0 + bob),
        (101.0, 103.0 + bob),
        (94.0, 149.0 + bob),
        (84.0, 145.0 + bob),
    ]
    draw.polygon([_pt(*p) for p in sash], fill=_rgba("#7e4a92"), outline=shadow)
    draw.line(
        _box(92.0, 105.0 + bob, 88.0, 145.0 + bob), fill=cloth_light, width=_s(0.42)
    )

    # Small floating particles around the raised hand during gesture / speech.
    particle_amp = 0.0
    if anim in {"gesture", "speak"}:
        particle_amp = 0.55 if anim == "gesture" else 0.35
        p_layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
        pd = ImageDraw.Draw(p_layer, "RGBA")
        for i in range(5):
            ang = cyc * 0.8 + i * 1.18
            px = l2[0] + math.cos(ang) * (9.0 + i * 1.4)
            py = l2[1] + math.sin(ang * 1.3) * (6.0 + i * 0.8)
            r = 1.1 + (i % 2) * 0.5
            pd.ellipse(
                _box(px - r, py - r, px + r, py + r),
                fill=_rgba("#9ff8ff", int(135 + 40 * particle_amp)),
            )
        p_layer = p_layer.filter(ImageFilter.GaussianBlur(radius=1.4))
        img.alpha_composite(p_layer)

    return _downsample(img)


def render_frame(animation: str, frame_idx: int, nframes: int) -> Image.Image:
    return _draw_creator(animation, frame_idx, nframes)


def render(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=render_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        label_width=108,
    )
    return [
        outputs["canonical"],
        outputs["canonical_transparent"],
        outputs["spritesheet"],
        outputs["yaml"],
        outputs["preview"],
    ]
