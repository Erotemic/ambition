from __future__ import annotations

"""Procedural sci-fi lab prop spritesheet.

This tack-on target renders a sheet of environment objects that could live in
an artificial creator / fabrication lab: containment vats, specimen jars,
resonance coils, control consoles, and related machinery.  The goal is not to
ship a fully interactive runtime format yet; instead, this provides a curated,
deterministic prop library that can be reviewed, sliced, and later wired into
the game as needed.

Invoked through the parent CLI::

    python -m ambition_sprite2d_renderer render creator_lab_props
    python -m ambition_sprite2d_renderer render-publish creator_lab_props
"""

import math
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Callable, Dict, List, Tuple

import yaml
from PIL import Image, ImageDraw, ImageFont

RGBA = Tuple[int, int, int, int]

FRAME_W = 128
FRAME_H = 128
LABEL_W = 160
SCALE = 4
CELL_PAD = 8
FRAME_COUNT = 4


@dataclass(frozen=True)
class PropSpec:
    key: str
    display_name: str
    category: str
    description: str
    drawer: str
    frames: int = FRAME_COUNT

    def to_dict(self) -> Dict[str, object]:
        return asdict(self)


PROP_SPECS: List[PropSpec] = [
    PropSpec(
        key="genesis_vat",
        display_name="Genesis Vat",
        category="containment",
        description="Tall cloning / fabrication vat with cloudy suspension fluid and a dim silhouette.",
        drawer="genesis_vat",
    ),
    PropSpec(
        key="specimen_jar",
        display_name="Specimen Jar",
        category="containment",
        description="Sealed jar holding a preserved anomalous specimen on a powered base.",
        drawer="specimen_jar",
    ),
    PropSpec(
        key="neural_console",
        display_name="Neural Console",
        category="control",
        description="Angled console desk with holographic display, status bars, and dense instrumentation.",
        drawer="neural_console",
    ),
    PropSpec(
        key="resonance_coil",
        display_name="Resonance Coil",
        category="power",
        description="Laboratory coil tower with inductive cage, amber core, and intermittent arc discharge.",
        drawer="resonance_coil",
    ),
    PropSpec(
        key="power_core",
        display_name="Power Core",
        category="power",
        description="Levitation cage containing a pulsing energy crystal in a magnetic cradle.",
        drawer="power_core",
    ),
    PropSpec(
        key="repair_cradle",
        display_name="Repair Cradle",
        category="fabrication",
        description="Low surgical / assembly slab with harnesses, monitor strip, and articulated side arms.",
        drawer="repair_cradle",
    ),
    PropSpec(
        key="drone_cradle",
        display_name="Drone Cradle",
        category="fabrication",
        description="Maintenance stand for a half-assembled drone head with a scanning optic.",
        drawer="drone_cradle",
    ),
    PropSpec(
        key="portal_calibrator",
        display_name="Portal Calibrator",
        category="research",
        description="Gyroscopic calibration rig for dimensional experiments and aperture tuning.",
        drawer="portal_calibrator",
    ),
]


TARGET_NAME = "creator_lab_props"
SHEET_FILES = (
    "creator_lab_props_spritesheet.png",
    "creator_lab_props_spritesheet.yaml",
)
CONTACT_FILE = "creator_lab_props_contact_sheet.png"


# ---- Shared drawing helpers -------------------------------------------------


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


def _with_alpha(color: RGBA, alpha: int) -> RGBA:
    return (color[0], color[1], color[2], alpha)


def _mix(c1: RGBA, c2: RGBA, t: float, alpha: int | None = None) -> RGBA:
    t = max(0.0, min(1.0, float(t)))
    a = alpha if alpha is not None else int(round(c1[3] * (1 - t) + c2[3] * t))
    return (
        int(round(c1[0] * (1 - t) + c2[0] * t)),
        int(round(c1[1] * (1 - t) + c2[1] * t)),
        int(round(c1[2] * (1 - t) + c2[2] * t)),
        a,
    )


def _font(size: int = 12):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size=size)
        except OSError:
            pass
    return ImageFont.load_default()


def _phase(frame_index: int, frame_count: int) -> float:
    return math.sin((frame_index / max(1, frame_count)) * math.tau)


def _floor_shadow(
    draw: ImageDraw.ImageDraw,
    cx: float,
    cy: float,
    rx: float,
    ry: float,
    alpha: int = 64,
) -> None:
    # No-op. Project rule: NO baked drop shadows on sprites — foot
    # alignment is the engine's job (the renderer uses the alpha
    # bbox + `feet_anchor_y` to land the silhouette's true bottom
    # on the floor). A baked ellipse here would push every lab
    # prop's alpha bbox down by ~8 px, making the prop hover. Call
    # sites are kept so future shadow-as-VFX work can wire a
    # separate cast-shadow layer if a room wants it.
    _ = (draw, cx, cy, rx, ry, alpha)


def _outline_text(
    draw: ImageDraw.ImageDraw,
    xy: Tuple[int, int],
    text: str,
    *,
    font,
    fill: RGBA,
    outline: RGBA,
) -> None:
    x, y = xy
    for ox in (-1, 0, 1):
        for oy in (-1, 0, 1):
            if ox == 0 and oy == 0:
                continue
            draw.text((x + ox, y + oy), text, font=font, fill=outline)
    draw.text((x, y), text, font=font, fill=fill)


def _draw_led(
    draw: ImageDraw.ImageDraw,
    x: float,
    y: float,
    color: RGBA,
    *,
    r: float = 2.0,
    outline: RGBA | None = None,
) -> None:
    outline = outline or _rgba("10141f")
    draw.ellipse(_box(x - r - 0.8, y - r - 0.8, x + r + 0.8, y + r + 0.8), fill=outline)
    draw.ellipse(_box(x - r, y - r, x + r, y + r), fill=color)
    draw.ellipse(
        _box(x - r * 0.6, y - r * 0.6, x - r * 0.1, y - r * 0.1),
        fill=(255, 255, 255, min(220, color[3])),
    )


def _draw_panel_grill(
    draw: ImageDraw.ImageDraw, x1: float, y1: float, x2: float, y2: float, color: RGBA
) -> None:
    draw.rounded_rectangle(_box(x1, y1, x2, y2), radius=_s(2), fill=color)
    for y in range(int(y1) + 2, int(y2), 3):
        draw.line(
            (_s(x1 + 2), _s(y), _s(x2 - 2), _s(y)), fill=(0, 0, 0, 40), width=_s(0.8)
        )


def _base_canvas() -> Image.Image:
    return Image.new("RGBA", (_s(FRAME_W), _s(FRAME_H)), (0, 0, 0, 0))


def _glass_overlay(
    layer: Image.Image,
    box_xy: Tuple[float, float, float, float],
    *,
    fill: RGBA,
    outline: RGBA,
    highlight_alpha: int = 85,
) -> None:
    d = ImageDraw.Draw(layer, "RGBA")
    x1, y1, x2, y2 = box_xy
    d.rounded_rectangle(
        _box(x1, y1, x2, y2), radius=_s(8), fill=fill, outline=outline, width=_s(1.3)
    )
    d.rounded_rectangle(
        _box(x1 + 3, y1 + 3, x1 + 9, y2 - 4),
        radius=_s(3),
        fill=(255, 255, 255, highlight_alpha),
    )
    d.arc(
        _box(x1 + 6, y1 + 6, x2 - 7, y2 - 6),
        290,
        68,
        fill=(255, 255, 255, highlight_alpha // 2),
        width=_s(1.1),
    )


# ---- Prop drawers -----------------------------------------------------------


def _draw_genesis_vat(frame_index: int, frame_count: int) -> Image.Image:
    img = _base_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    p = _phase(frame_index, frame_count)
    cool = _rgba("5FD2E7")
    cool_soft = _rgba("A5F0FF", 120)
    steel = _rgba("4A556B")
    steel_dark = _rgba("212A39")
    amber = _rgba("F6B55E")

    _floor_shadow(d, 64, 112, 26, 8, 58)
    d.rounded_rectangle(
        _box(31, 21, 97, 112),
        radius=_s(10),
        fill=steel_dark,
        outline=_rgba("111722"),
        width=_s(2),
    )
    d.rounded_rectangle(
        _box(38, 29, 90, 104),
        radius=_s(9),
        fill=_rgba("172433"),
        outline=_rgba("536179"),
        width=_s(1.4),
    )

    fluid_top = 42 + p * 1.3
    d.rounded_rectangle(
        _box(42, fluid_top, 86, 96),
        radius=_s(7),
        fill=_rgba("40B7D1", 150),
        outline=None,
    )
    silhouette = [
        (62, 49),
        (57, 55),
        (54, 65),
        (53, 76),
        (56, 88),
        (64, 92),
        (72, 88),
        (75, 76),
        (74, 65),
        (70, 55),
        (66, 49),
    ]
    d.polygon(
        [
            (_s(x), _s(y + (0.5 if i % 2 else -0.5) * p))
            for i, (x, y) in enumerate(silhouette)
        ],
        fill=_rgba("0B202A", 82),
    )
    d.ellipse(_box(57, 44, 71, 58), fill=_rgba("0B202A", 88))
    for idx, bx in enumerate((49, 59, 73)):
        by = 88 - ((frame_index * 7 + idx * 11) % 28)
        br = 1.8 + (idx % 2) * 0.8
        d.ellipse(_box(bx - br, by - br, bx + br, by + br), fill=_rgba("DAFBFF", 128))
    _glass_overlay(
        img, (40, 27, 88, 100), fill=_rgba("6EE8FF", 44), outline=_rgba("C8F7FF", 118)
    )

    d.rounded_rectangle(
        _box(47, 12, 81, 23),
        radius=_s(4),
        fill=steel,
        outline=_rgba("121823"),
        width=_s(1.2),
    )
    d.rounded_rectangle(
        _box(44, 104, 84, 116),
        radius=_s(4),
        fill=steel,
        outline=_rgba("121823"),
        width=_s(1.2),
    )
    for x in (48, 56, 64, 72, 80):
        _draw_led(d, x, 17, cool if x != 64 else amber, r=1.8)
        _draw_led(d, x, 110, amber if x in {56, 72} else cool_soft, r=1.6)
    return img.resize((FRAME_W, FRAME_H), Image.Resampling.LANCZOS)


def _draw_specimen_jar(frame_index: int, frame_count: int) -> Image.Image:
    img = _base_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    p = _phase(frame_index, frame_count)
    glass = _rgba("A6F4F8", 38)
    outline = _rgba("D4FFFF", 118)
    base = _rgba("404C61")
    mauve = _rgba("C48CFF")
    bio = _rgba("8EF28E")

    _floor_shadow(d, 64, 110, 21, 7, 54)
    d.rounded_rectangle(
        _box(46, 98, 82, 111),
        radius=_s(4),
        fill=base,
        outline=_rgba("121823"),
        width=_s(1.2),
    )
    d.rounded_rectangle(
        _box(42, 24, 86, 98),
        radius=_s(9),
        fill=_rgba("253144"),
        outline=_rgba("546378"),
        width=_s(1.3),
    )
    d.rounded_rectangle(
        _box(46, 29, 82, 94), radius=_s(8), fill=_rgba("294C56", 110), outline=None
    )
    d.rounded_rectangle(
        _box(49, 15, 79, 26),
        radius=_s(4),
        fill=base,
        outline=_rgba("121823"),
        width=_s(1.2),
    )

    core_y = 60 + p * 2.0
    d.ellipse(_box(56, core_y - 9, 72, core_y + 9), fill=mauve)
    d.ellipse(_box(60, core_y - 4, 68, core_y + 4), fill=_rgba("351B58", 220))
    d.arc(
        _box(53, core_y - 10, 75, core_y + 10),
        200,
        340,
        fill=_rgba("F3DEFF", 160),
        width=_s(1.4),
    )
    for idx, bx in enumerate((55, 63, 73)):
        by = 90 - ((frame_index * 9 + idx * 8) % 30)
        d.ellipse(
            _box(bx - 1.5, by - 1.5, bx + 1.5, by + 1.5), fill=_rgba("FFFFFF", 110)
        )
    for x in (50, 58, 66, 74):
        _draw_led(d, x, 104, bio if x in {58, 74} else mauve, r=1.7)
    _glass_overlay(img, (45, 27, 83, 95), fill=glass, outline=outline)
    return img.resize((FRAME_W, FRAME_H), Image.Resampling.LANCZOS)


def _draw_neural_console(frame_index: int, frame_count: int) -> Image.Image:
    img = _base_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    p = _phase(frame_index, frame_count)
    steel = _rgba("425167")
    steel_dark = _rgba("1E2635")
    holo = _rgba("6BE9FF")
    amber = _rgba("F4B255")

    _floor_shadow(d, 62, 112, 30, 8, 58)
    d.polygon(
        [(_s(35), _s(96)), (_s(53), _s(43)), (_s(88), _s(43)), (_s(96), _s(96))],
        fill=steel_dark,
        outline=_rgba("111722"),
    )
    d.polygon(
        [(_s(40), _s(92)), (_s(56), _s(50)), (_s(85), _s(50)), (_s(91), _s(92))],
        fill=steel,
        outline=_rgba("5C6B84"),
    )
    d.rounded_rectangle(
        _box(50, 54, 84, 71),
        radius=_s(3),
        fill=_rgba("142536"),
        outline=_rgba("7EDFFF"),
        width=_s(1.2),
    )
    d.arc(_box(47, 29, 87, 69), 20, 160, fill=_rgba("7DE9FF", 110), width=_s(1.3))
    d.line((_s(67), _s(55), _s(67), _s(33)), fill=_rgba("7DE9FF", 90), width=_s(1.0))
    for i in range(4):
        yy = 59 + i * 3
        xx2 = 81 - (i % 2) * 6 - p * 2.0
        d.line(
            (_s(54), _s(yy), _s(xx2), _s(yy)), fill=_rgba("67E2FF", 150), width=_s(1.0)
        )
    for x in (52, 57, 62, 67, 72, 77, 82):
        _draw_led(d, x, 76, amber if x in {57, 72} else holo, r=1.2)
    _draw_panel_grill(d, 49, 80, 84, 88, _rgba("263347"))
    d.rectangle(_box(44, 92, 52, 113), fill=steel_dark)
    d.rectangle(_box(83, 92, 91, 113), fill=steel_dark)
    return img.resize((FRAME_W, FRAME_H), Image.Resampling.LANCZOS)


def _draw_resonance_coil(frame_index: int, frame_count: int) -> Image.Image:
    img = _base_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    p = _phase(frame_index, frame_count)
    steel = _rgba("465066")
    amber = _rgba("F6B55E")
    cool = _rgba("87E8FF")

    _floor_shadow(d, 64, 112, 24, 7, 56)
    d.rounded_rectangle(
        _box(48, 96, 80, 112),
        radius=_s(4),
        fill=_rgba("2A3345"),
        outline=_rgba("111722"),
        width=_s(1.2),
    )
    d.rectangle(_box(60, 28, 68, 96), fill=steel)
    d.ellipse(
        _box(52, 22, 76, 38),
        fill=_rgba("384458"),
        outline=_rgba("111722"),
        width=_s(1.1),
    )
    d.ellipse(
        _box(52, 84, 76, 100),
        fill=_rgba("384458"),
        outline=_rgba("111722"),
        width=_s(1.1),
    )
    for y in range(34, 83, 6):
        d.arc(_box(49, y, 79, y + 10), 180, 360, fill=amber, width=_s(1.3))
    glow_r = 7.5 + 1.2 * (p + 1)
    d.ellipse(
        _box(64 - glow_r, 50 - glow_r, 64 + glow_r, 50 + glow_r),
        fill=_rgba("F9CA79", 190),
    )
    d.ellipse(_box(64 - 3, 50 - 3, 64 + 3, 50 + 3), fill=_rgba("FFF4D6", 240))
    for x in (44, 84):
        d.rectangle(_box(x - 2, 43, x + 2, 77), fill=steel)
    # alternating arc shapes
    if frame_index % 2 == 0:
        d.arc(_box(42, 34, 65, 59), 310, 35, fill=_rgba("99F2FF", 150), width=_s(1.2))
        d.arc(_box(63, 47, 87, 70), 140, 220, fill=_rgba("99F2FF", 150), width=_s(1.2))
    else:
        d.arc(_box(41, 49, 64, 73), 320, 45, fill=_rgba("99F2FF", 150), width=_s(1.2))
        d.arc(_box(64, 33, 88, 58), 135, 230, fill=_rgba("99F2FF", 150), width=_s(1.2))
    _draw_led(d, 64, 105, cool, r=1.8)
    return img.resize((FRAME_W, FRAME_H), Image.Resampling.LANCZOS)


def _draw_power_core(frame_index: int, frame_count: int) -> Image.Image:
    img = _base_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    p = _phase(frame_index, frame_count)
    steel = _rgba("47546A")
    violet = _rgba("B88CFF")
    teal = _rgba("69E9FF")

    _floor_shadow(d, 64, 112, 23, 7, 56)
    d.rounded_rectangle(
        _box(46, 97, 82, 112),
        radius=_s(4),
        fill=_rgba("263042"),
        outline=_rgba("111722"),
        width=_s(1.2),
    )
    for x in (50, 78):
        d.rectangle(_box(x - 2, 45, x + 2, 97), fill=steel)
    d.arc(_box(44, 39, 84, 80), 0, 180, fill=steel, width=_s(2.0))
    d.arc(_box(44, 47, 84, 88), 180, 360, fill=steel, width=_s(2.0))
    # floating crystal
    offset = p * 2.0
    crystal = [
        (_s(64), _s(45 + offset)),
        (_s(73), _s(58 + offset)),
        (_s(64), _s(77 + offset)),
        (_s(55), _s(58 + offset)),
    ]
    d.polygon(crystal, fill=violet, outline=_rgba("F1E6FF"))
    d.polygon(
        [
            (_s(64), _s(50 + offset)),
            (_s(69), _s(58 + offset)),
            (_s(64), _s(71 + offset)),
            (_s(59), _s(58 + offset)),
        ],
        fill=_rgba("F6EDFF", 150),
    )
    halo_r = 14 + 2 * (p + 1)
    d.ellipse(
        _box(64 - halo_r, 59 - halo_r, 64 + halo_r, 59 + halo_r),
        outline=_rgba("7FF0FF", 110),
        width=_s(1.2),
    )
    for angle in (0, 90):
        d.arc(
            _box(52, 47, 76, 71),
            angle + frame_index * 12,
            angle + 55 + frame_index * 12,
            fill=teal,
            width=_s(1.1),
        )
    for x in (54, 64, 74):
        _draw_led(d, x, 104, teal if x != 64 else violet, r=1.6)
    return img.resize((FRAME_W, FRAME_H), Image.Resampling.LANCZOS)


def _draw_repair_cradle(frame_index: int, frame_count: int) -> Image.Image:
    img = _base_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    p = _phase(frame_index, frame_count)
    steel = _rgba("4A556A")
    orange = _rgba("F4B255")
    green = _rgba("87E08F")

    _floor_shadow(d, 64, 112, 34, 8, 54)
    d.rounded_rectangle(
        _box(31, 72, 97, 92),
        radius=_s(4),
        fill=steel,
        outline=_rgba("111722"),
        width=_s(1.2),
    )
    d.rounded_rectangle(
        _box(35, 58, 93, 75),
        radius=_s(5),
        fill=_rgba("6B788F"),
        outline=_rgba("192331"),
        width=_s(1.1),
    )
    d.rounded_rectangle(_box(39, 61, 89, 72), radius=_s(4), fill=_rgba("1F2837"))
    for x in (43, 58, 72, 86):
        d.line((_s(x), _s(75), _s(x), _s(107)), fill=_rgba("313A4B"), width=_s(2.2))
    # harness straps
    for x in (48, 64, 80):
        d.rounded_rectangle(
            _box(x - 3, 61, x + 3, 72),
            radius=_s(1.5),
            fill=_rgba("C24A4A"),
            outline=_rgba("4A1414"),
            width=_s(0.8),
        )
    # side manipulator arm + monitor strip
    d.line((_s(24), _s(62), _s(30), _s(55 + p)), fill=_rgba("56647C"), width=_s(2.2))
    d.line((_s(30), _s(55 + p), _s(38), _s(60)), fill=_rgba("56647C"), width=_s(2.2))
    d.ellipse(_box(19, 56, 29, 66), fill=_rgba("313B4D"))
    d.rounded_rectangle(
        _box(99, 58, 111, 83),
        radius=_s(3),
        fill=_rgba("202839"),
        outline=_rgba("5E6B83"),
        width=_s(1.0),
    )
    for i, y in enumerate((63, 68, 73, 78)):
        d.line(
            (_s(102), _s(y), _s(108 - (i % 2) * 2), _s(y)),
            fill=green if i < 2 else orange,
            width=_s(1.0),
        )
    return img.resize((FRAME_W, FRAME_H), Image.Resampling.LANCZOS)


def _draw_drone_cradle(frame_index: int, frame_count: int) -> Image.Image:
    img = _base_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    p = _phase(frame_index, frame_count)
    steel = _rgba("4A556A")
    red = _rgba("E65B5B")
    cool = _rgba("74ECFF")

    _floor_shadow(d, 64, 112, 22, 7, 54)
    d.rounded_rectangle(
        _box(42, 98, 86, 112),
        radius=_s(4),
        fill=_rgba("273042"),
        outline=_rgba("111722"),
        width=_s(1.2),
    )
    for x in (47, 81):
        d.rectangle(_box(x - 2, 45, x + 2, 98), fill=steel)
    d.arc(_box(46, 40, 82, 78), 210, -30, fill=steel, width=_s(2.0))
    # suspended drone head
    bob = p * 1.3
    d.ellipse(
        _box(52, 51 + bob, 76, 73 + bob),
        fill=_rgba("6B768C"),
        outline=_rgba("111722"),
        width=_s(1.0),
    )
    d.ellipse(_box(58, 57 + bob, 70, 69 + bob), fill=_rgba("182130"))
    eye_x = 64 + (frame_index - 1.5) * 1.5
    d.ellipse(_box(eye_x - 3, 61 + bob - 3, eye_x + 3, 61 + bob + 3), fill=cool)
    d.line([(_s(56), _s(76)), (_s(51), _s(88))], fill=steel, width=_s(1.8))
    d.line([(_s(72), _s(76)), (_s(77), _s(88))], fill=steel, width=_s(1.8))
    _draw_led(d, 64, 104, red if frame_index % 2 == 0 else cool, r=1.8)
    return img.resize((FRAME_W, FRAME_H), Image.Resampling.LANCZOS)


def _draw_portal_calibrator(frame_index: int, frame_count: int) -> Image.Image:
    img = _base_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    p = _phase(frame_index, frame_count)
    steel = _rgba("4D586E")
    violet = _rgba("BA8BFF")
    teal = _rgba("72EBFF")
    amber = _rgba("F5B55D")

    _floor_shadow(d, 64, 112, 25, 7, 54)
    d.rounded_rectangle(
        _box(48, 98, 80, 112),
        radius=_s(4),
        fill=_rgba("273042"),
        outline=_rgba("111722"),
        width=_s(1.2),
    )
    d.rectangle(_box(62, 84, 66, 98), fill=steel)
    d.ellipse(_box(43, 45, 85, 87), outline=steel, width=_s(2.0))
    d.ellipse(_box(49, 51, 79, 81), outline=_rgba("8894AB"), width=_s(1.4))
    # rotating calibration arcs
    start = frame_index * 18
    d.arc(_box(45, 47, 83, 85), start + 10, start + 95, fill=teal, width=_s(1.3))
    d.arc(_box(50, 52, 78, 80), -start + 160, -start + 245, fill=violet, width=_s(1.3))
    d.ellipse(_box(60, 57, 68, 65), fill=amber)
    for x, y in ((48, 66), (80, 66), (64, 50), (64, 82)):
        _draw_led(d, x, y, teal if (x + y + frame_index) % 2 == 0 else violet, r=1.4)
    return img.resize((FRAME_W, FRAME_H), Image.Resampling.LANCZOS)


DRAWERS: Dict[str, Callable[[int, int], Image.Image]] = {
    "genesis_vat": _draw_genesis_vat,
    "specimen_jar": _draw_specimen_jar,
    "neural_console": _draw_neural_console,
    "resonance_coil": _draw_resonance_coil,
    "power_core": _draw_power_core,
    "repair_cradle": _draw_repair_cradle,
    "drone_cradle": _draw_drone_cradle,
    "portal_calibrator": _draw_portal_calibrator,
}


# ---- Sheet assembly ---------------------------------------------------------


def render_prop_frame(spec: PropSpec, frame_index: int) -> Image.Image:
    return DRAWERS[spec.drawer](frame_index, spec.frames)


def build_sheet(
    props: List[PropSpec] = PROP_SPECS,
) -> Tuple[Image.Image, Dict[str, object]]:
    max_frames = max(spec.frames for spec in props)
    width = LABEL_W + max_frames * FRAME_W
    height = len(props) * FRAME_H
    sheet = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(sheet, "RGBA")
    font = _font(12)
    small = _font(10)

    manifest: Dict[str, object] = {
        "target": TARGET_NAME,
        "sheet_kind": "prop_spritesheet",
        "frame_width": FRAME_W,
        "frame_height": FRAME_H,
        "label_width": LABEL_W,
        "prop_order": [spec.key for spec in props],
        "notes": (
            "Procedural creator-lab prop library with subtle idle animation. "
            "These are environment objects rather than character rows."
        ),
        "props": {},
    }

    for row_idx, spec in enumerate(props):
        y = row_idx * FRAME_H
        draw.rectangle((0, y, LABEL_W, y + FRAME_H), fill=(22, 24, 34, 204))
        draw.text((10, y + 10), spec.display_name, fill=(238, 240, 255, 255), font=font)
        draw.text((10, y + 28), spec.category, fill=(135, 226, 244, 255), font=small)
        desc = spec.description
        if len(desc) > 44:
            desc = desc[:43] + "…"
        draw.text((10, y + 44), desc, fill=(182, 188, 210, 255), font=small)
        draw.text(
            (10, y + 97), f"{spec.frames}f idle", fill=(205, 205, 222, 255), font=small
        )

        frame_records = []
        for frame_index in range(spec.frames):
            frame = render_prop_frame(spec, frame_index)
            x = LABEL_W + frame_index * FRAME_W
            sheet.alpha_composite(frame, (x, y))
            frame_records.append(
                {
                    "index": frame_index,
                    "x": x,
                    "y": y,
                    "w": FRAME_W,
                    "h": FRAME_H,
                    "duration_ms": 140,
                }
            )

        manifest["props"][spec.key] = {
            "display_name": spec.display_name,
            "category": spec.category,
            "description": spec.description,
            "frames": frame_records,
        }

    return sheet, manifest


def build_contact_sheet(
    props: List[PropSpec] = PROP_SPECS, *, columns: int = 4
) -> Image.Image:
    thumbs = [render_prop_frame(spec, 0) for spec in props]
    cell_w = FRAME_W + 16
    cell_h = FRAME_H + 28
    rows = math.ceil(len(thumbs) / columns)
    sheet = Image.new("RGBA", (columns * cell_w, rows * cell_h), (18, 20, 28, 255))
    draw = ImageDraw.Draw(sheet, "RGBA")
    font = _font(11)
    for idx, (spec, img) in enumerate(zip(props, thumbs)):
        col = idx % columns
        row = idx // columns
        ox = col * cell_w
        oy = row * cell_h
        sheet.alpha_composite(img, (ox + 8, oy + 4))
        _outline_text(
            draw,
            (ox + 8, oy + FRAME_H + 6),
            spec.display_name,
            font=font,
            fill=(239, 241, 255, 255),
            outline=(0, 0, 0, 150),
        )
    return sheet


def write_outputs(out_dir: Path) -> List[Path]:
    out_dir.mkdir(parents=True, exist_ok=True)
    sheet, manifest = build_sheet(PROP_SPECS)
    png_path = out_dir / SHEET_FILES[0]
    yaml_path = out_dir / SHEET_FILES[1]
    ron_path = out_dir / f"{TARGET_NAME}_spritesheet.ron"
    contact_path = out_dir / CONTACT_FILE
    sheet.save(png_path)
    yaml_path.write_text(
        yaml.safe_dump(manifest, sort_keys=False, allow_unicode=True), encoding="utf8"
    )
    ron_path.write_text(_emit_props_ron(manifest), encoding="utf8")
    build_contact_sheet(PROP_SPECS).save(contact_path)
    return [png_path, yaml_path, ron_path, contact_path]


def _emit_props_ron(manifest: Dict[str, object]) -> str:
    """Serialize the lab-props manifest to the multi-record RON shape
    consumed by `SheetRegistry`. One `SheetRecord` per prop key, each
    with `y_offset` = `row_idx * FRAME_H` so it addresses its own row
    band of the shared `creator_lab_props_spritesheet.png`.
    """
    from ...authoring.tackon_sheet import (
        _ron_sheet_record,
    )  # local import: tooling-only dependency

    image = SHEET_FILES[0]
    props = manifest.get("props", {}) if isinstance(manifest, dict) else {}
    records: List[str] = []
    for row_idx, (key, info) in enumerate(
        props.items() if isinstance(props, dict) else []
    ):
        frames = info.get("frames", []) if isinstance(info, dict) else []
        rects = [
            {
                "x": int(f["x"]),
                "y": int(f["y"]),
                "w": int(f.get("w", FRAME_W)),
                "h": int(f.get("h", FRAME_H)),
            }
            for f in frames
        ]
        duration_ms = int(frames[0].get("duration_ms", 140)) if frames else 140
        record = {
            "target": key,
            "image": image,
            "label_width": LABEL_W,
            "frame_width": FRAME_W,
            "frame_height": FRAME_H,
            "y_offset": row_idx * FRAME_H,
            "body_metrics": None,
            "rows": [
                {
                    "animation": "idle",
                    "row_index": 0,
                    "frame_count": len(rects),
                    "duration_ms": duration_ms,
                    "duration_secs": round(duration_ms / 1000.0, 6),
                    "rects": rects,
                }
            ],
        }
        records.append(_ron_sheet_record(record))

    header = (
        f"// Auto-emitted from {TARGET_NAME}_spritesheet.yaml — see\n"
        f"// `presentation::character_sprites::registry`.\n"
        f"//\n"
        f"// 8 lab props share `{image}`; each record below claims its own\n"
        f"// row band of the packed image via `y_offset`.\n"
    )
    if not records:
        return header + "[\n]\n"
    return header + "[\n" + ",\n".join(records) + ",\n]\n"


def render(out_dir: Path) -> List[Path]:
    return write_outputs(out_dir)
