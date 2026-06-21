from __future__ import annotations

"""Procedural news board sprite — a wall-mounted bulletin board used
in `drain_alley` and other intro slice rooms.

Reads as a static prop, NOT a character: dark wooden frame around a
cork/databoard interior, three small notice clippings, a corporate
"DISRUPTOR INDUSTRIES" header, and a blinking power LED in the upper
right. Single idle row with a tiny LED blink + paper-flutter cycle
so the board feels alive without animating like an NPC.

No baked drop shadow (project rule, see memory
`feedback_no_drop_shadows`). The bottom row of the sprite IS the
true foot — the in-game `feet_anchor_y` lands directly on it.
"""

import math
from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageColor, ImageDraw

from ...authoring.tackon_sheet import build_sheet

RGBA = Tuple[int, int, int, int]

TARGET_NAME = "news_board"
SHEET_FILES = [f"{TARGET_NAME}_spritesheet.png", f"{TARGET_NAME}_spritesheet.yaml"]

ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 165),
]

FRAME_SIZE = (64, 96)
SUPER = 4
W, H = FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER


def _rgba(color: str, alpha: int = 255) -> RGBA:
    r, g, b = ImageColor.getrgb(color)
    return (r, g, b, alpha)


def _s(v: float) -> int:
    return int(round(v * SUPER))


def _box(x1: float, y1: float, x2: float, y2: float) -> Tuple[int, int, int, int]:
    return (_s(x1), _s(y1), _s(x2), _s(y2))


def _downsample(img: Image.Image) -> Image.Image:
    return img.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


# --- Palette: corporate-dystopia info kiosk -----------------------
FRAME_DARK = _rgba("#1B1F2A")
FRAME_LIGHT = _rgba("#374050")
FRAME_TRIM = _rgba("#7C8AA6")
FRAME_BOLT = _rgba("#C8964A")

CORK = _rgba("#2D2A3B")
CORK_HILITE = _rgba("#3E3A4F")

HEADER_BG = _rgba("#3A2455")
HEADER_INK = _rgba("#E8D7FF")

PAPER_A = _rgba("#F4F0DE")
PAPER_B = _rgba("#FFEDA8")
PAPER_C = _rgba("#D7E0FF")
PAPER_INK = _rgba("#1F1D29")
PIN_RED = _rgba("#E14640")
PIN_BLACK = _rgba("#0B0D14")

LED_ON = _rgba("#5BFFA8")
LED_DIM = _rgba("#1A4A2F")


def _render_idle_frame(frame_idx: int, n_frames: int) -> Image.Image:
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(img, "RGBA")

    # --- Outer frame (dark wooden / gunmetal border) -------------
    d.rectangle(
        _box(2, 6, 62, 88), fill=FRAME_DARK, outline=(0, 0, 0, 255), width=_s(0.8)
    )
    # Inner lit band (highlight along top + left for depth)
    d.line(_box(3.5, 7.5, 60.5, 7.5)[:4], fill=FRAME_TRIM, width=_s(0.5))
    d.line(_box(3.5, 7.5, 3.5, 87)[:4], fill=FRAME_TRIM, width=_s(0.4))

    # --- Brass bolts in the four corners (frame fasteners) -------
    bolt_r = 1.2
    for bx, by in [(6, 10), (58, 10), (6, 84), (58, 84)]:
        d.ellipse(
            _box(bx - bolt_r, by - bolt_r, bx + bolt_r, by + bolt_r),
            fill=FRAME_BOLT,
            outline=FRAME_DARK,
        )
        d.ellipse(_box(bx - 0.5, by - 0.5, bx + 0.5, by + 0.5), fill=_rgba("#FFE69A"))

    # --- Header bar (purple corporate brand banner) --------------
    d.rectangle(_box(6, 12, 58, 22), fill=HEADER_BG, outline=FRAME_DARK, width=_s(0.4))
    # Header glyphs — a stylized "DI" mark (Disruptor Industries) on
    # the left, plus a row of pseudo-letterforms suggesting NEWS.
    d.rectangle(_box(8, 14, 13, 20), fill=HEADER_INK)
    d.rectangle(_box(9.6, 15.6, 11.4, 18.4), fill=HEADER_BG)
    # NEWS letterforms as small filled rects
    for i, x in enumerate((16, 22, 29, 36, 43, 50)):
        # Subtle column variation so the header reads as text
        hgt = 4 if i % 2 == 0 else 3
        d.rectangle(_box(x, 15.5, x + 3.0, 15.5 + hgt), fill=HEADER_INK)

    # --- Cork/data board interior --------------------------------
    d.rectangle(_box(6, 24, 58, 80), fill=CORK, outline=FRAME_DARK, width=_s(0.4))
    # Dotted cork texture
    for cy in range(26, 80, 4):
        for cx in range(8, 58, 4):
            if (cx + cy) % 7 == 0:
                d.point((_s(cx), _s(cy)), fill=CORK_HILITE)

    # --- Pinned papers ------------------------------------------
    # Paper A: white memo with horizontal lines
    d.rectangle(_box(10, 28, 28, 44), fill=PAPER_A, outline=PAPER_INK, width=_s(0.4))
    for i, ly in enumerate((31, 33.5, 36, 38.5, 41)):
        # Last line is short (signature)
        x_end = 18 if i == 4 else 26
        d.line(_box(12, ly, x_end, ly)[:4], fill=PAPER_INK, width=_s(0.3))
    # Red push-pin top-left of memo
    d.ellipse(_box(11.0, 28.5, 13.0, 30.5), fill=PIN_RED, outline=PIN_BLACK)

    # Paper B: yellow notice, large headline
    d.rectangle(_box(32, 26, 56, 50), fill=PAPER_B, outline=PAPER_INK, width=_s(0.4))
    # Headline (one fat dark band)
    d.rectangle(_box(34, 30, 54, 33), fill=PAPER_INK)
    # Body text (4 thin lines)
    for ly in (35.5, 37.5, 39.5, 41.5):
        d.line(_box(34, ly, 53, ly)[:4], fill=PAPER_INK, width=_s(0.25))
    # Stamp / logo lower-right
    d.rectangle(_box(46, 44, 54, 48), fill=PIN_RED, outline=PIN_BLACK, width=_s(0.3))
    # Red push-pin top-right
    d.ellipse(_box(53, 26.6, 55, 28.6), fill=PIN_RED, outline=PIN_BLACK)

    # Paper C: blue clipping, dossier-style
    d.rectangle(_box(12, 52, 38, 76), fill=PAPER_C, outline=PAPER_INK, width=_s(0.4))
    # Photo placeholder (top half)
    d.rectangle(
        _box(14, 54, 22, 62), fill=_rgba("#646D85"), outline=PAPER_INK, width=_s(0.25)
    )
    # Caption lines
    for ly in (64, 66.2, 68.4, 70.6, 72.8):
        d.line(_box(14, ly, 35, ly)[:4], fill=PAPER_INK, width=_s(0.25))
    # Black pushpin top-left of clipping
    d.ellipse(_box(13, 52.5, 15, 54.5), fill=PIN_BLACK, outline=PIN_BLACK)

    # Sticky note (small square) — gentle flutter
    flutter = math.sin((frame_idx / max(1, n_frames)) * math.tau) * 0.5
    nx = 42 + flutter
    d.rectangle(
        _box(nx, 54, nx + 12, 66), fill=PAPER_A, outline=PAPER_INK, width=_s(0.3)
    )
    # Scrawled lines
    for ly in (56.5, 58.5, 60.5):
        d.line(_box(nx + 1.0, ly, nx + 11.0, ly)[:4], fill=PAPER_INK, width=_s(0.25))
    # Yellow pin
    d.ellipse(
        _box(nx + 4.5, 53.6, nx + 6.5, 55.6), fill=_rgba("#E0B23F"), outline=PIN_BLACK
    )

    # --- Power LED (blinks slowly across the idle cycle) ---------
    # On for frames 0..3, off for 4..5 — gentle "alive" indicator.
    led_on = frame_idx < (n_frames - 2)
    led_color = LED_ON if led_on else LED_DIM
    d.ellipse(_box(55, 14, 57, 16), fill=led_color, outline=FRAME_DARK)

    # Hint of glow under the LED when on
    if led_on:
        d.ellipse(
            _box(54.5, 13.5, 57.5, 16.5), outline=_rgba("#5BFFA8", 130), width=_s(0.3)
        )

    # No drop shadow under the board (project rule). The board's
    # bottom row at y=88 IS the visible foot — feet_anchor_y in
    # the runtime spec lands here directly.

    return _downsample(img)


def render_frame(animation: str, frame_idx: int, nframes: int) -> Image.Image:
    if animation == "idle":
        return _render_idle_frame(frame_idx, nframes)
    raise ValueError(f"unknown animation: {animation}")


def render(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=render_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        label_width=104,
        auto_crop=False,  # the board's footprint should stay stable
        # frame-to-frame; auto-crop would shrink the
        # frame around the active art.
    )
    return [
        outputs["canonical"],
        outputs["canonical_transparent"],
        outputs["spritesheet"],
        outputs["yaml"],
        outputs["preview"],
    ]
