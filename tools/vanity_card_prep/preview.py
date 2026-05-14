#!/usr/bin/env python3
"""
Preview renderer (no display required).

Renders each animation beat as a static PNG that looks identical to what
demo.py would show on screen.  Useful for checking composition, bubble
placement, and colours without a physical display.

Output:  assets/vanity_card/preview/beat_{N}.png
         assets/vanity_card/preview/strip.png   (all 4 side by side)

Run:  python3 preview.py
"""

import os
import textwrap
import numpy as np
from PIL import Image, ImageDraw, ImageFont

from utils import load_config, out_path

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(SCRIPT_DIR))


# ── Font helpers ──────────────────────────────────────────────────────────────

_FONT_CANDIDATES = [
    "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/truetype/freefont/FreeSansBold.ttf",
    "/usr/share/fonts/truetype/ubuntu/Ubuntu-B.ttf",
]


def load_pil_font(size: int) -> ImageFont.FreeTypeFont:
    for path in _FONT_CANDIDATES:
        if os.path.exists(path):
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


# ── Speech bubble ─────────────────────────────────────────────────────────────

def draw_bubble_pil(draw: ImageDraw.ImageDraw, text: str, font: ImageFont.FreeTypeFont,
                    panel_rect: tuple, side: str, cfg: dict) -> None:
    """
    Draw a speech bubble onto *draw* over the panel.
    panel_rect: (x, y, w, h)
    side:       'left' | 'right'
    """
    ac = cfg["animation"]
    pad    = ac["bubble_padding"]
    bw     = ac["bubble_border_width"]
    bg     = tuple(ac["bubble_bg"])
    border = tuple(ac["bubble_border"])

    px, py, pw, ph = panel_rect
    lines = text.split("\n")
    line_boxes = [draw.textbbox((0, 0), ln, font=font) for ln in lines]
    line_h  = max(b[3] - b[1] for b in line_boxes) if line_boxes else 20
    text_w  = max(b[2] - b[0] for b in line_boxes) if line_boxes else 60
    text_h  = len(lines) * line_h + max(0, len(lines) - 1) * 4

    bub_w = text_w + pad * 2
    bub_h = text_h + pad * 2
    tail_h = 28

    if side == "left":
        bub_x = px + pw // 8
        tail_cx = bub_x + bub_w // 3
    else:
        bub_x = px + pw - bub_w - pw // 8
        tail_cx = bub_x + bub_w * 2 // 3

    bub_y = py + int(ph * 0.06)

    # Bubble body
    body = [bub_x, bub_y, bub_x + bub_w, bub_y + bub_h]
    draw.rounded_rectangle(body, radius=14, fill=bg, outline=border, width=bw)

    # Tail (filled triangle)
    tail_base_y = bub_y + bub_h - 1
    tail_tip_y  = bub_y + bub_h + tail_h
    tail_pts = [
        (tail_cx - 12, tail_base_y),
        (tail_cx + 12, tail_base_y),
        (tail_cx,      tail_tip_y),
    ]
    draw.polygon(tail_pts, fill=bg, outline=None)
    # Re-draw bubble bottom to cover the tail's top edge
    draw.rectangle([bub_x + bw, bub_y + bub_h - bw * 2,
                    bub_x + bub_w - bw, bub_y + bub_h], fill=bg)

    # Text
    ty = bub_y + pad
    for i, line in enumerate(lines):
        lb = draw.textbbox((0, 0), line, font=font)
        lw = lb[2] - lb[0]
        tx = bub_x + pad + (text_w - lw) // 2
        draw.text((tx, ty), line, fill=border, font=font)
        ty += line_h + 4


# ── Panel rendering ───────────────────────────────────────────────────────────

def render_beat(panel_cfg: dict, cfg: dict, display_w: int, display_h: int,
                font: ImageFont.FreeTypeFont) -> Image.Image:
    """Render one animation beat to a PIL Image."""
    ac = cfg["animation"]
    bg_col   = tuple(ac["background_color"])
    panel_bg = tuple(ac["panel_bg_color"])
    border   = tuple(ac["panel_border_color"])
    bdw      = ac["panel_border_width"]
    shad     = ac["shadow_offset"]

    frame = Image.new("RGB", (display_w, display_h), bg_col)
    draw  = ImageDraw.Draw(frame)

    # Panel display rect (87% of height, centred)
    cfg_pw, cfg_ph = cfg["panel_size"]
    disp_h = int(display_h * 0.87)
    disp_w = int(disp_h * cfg_pw / cfg_ph)
    if disp_w > display_w * 0.95:
        disp_w = int(display_w * 0.95)
        disp_h = int(disp_w * cfg_ph / cfg_pw)
    px = (display_w - disp_w) // 2
    py = (display_h - disp_h) // 2

    # Shadow
    shad_col = tuple(ac["shadow_color"][:3])
    draw.rectangle([px + shad, py + shad,
                    px + disp_w + bdw * 2 + shad,
                    py + disp_h + bdw * 2 + shad], fill=shad_col)

    # Border
    draw.rectangle([px - bdw, py - bdw, px + disp_w + bdw, py + disp_h + bdw],
                   fill=border)

    # Panel background
    draw.rectangle([px, py, px + disp_w, py + disp_h], fill=panel_bg)

    # Character image
    beat = panel_cfg["beat"]
    panel_path = out_path(cfg, "final", f"panel_{beat}.png")
    if os.path.exists(panel_path):
        char_img = Image.open(panel_path).convert("RGBA")
        # Scale to fit panel rect, preserve aspect
        iw, ih = char_img.size
        scale = min(disp_w / iw, disp_h / ih)
        nw, nh = int(iw * scale), int(ih * scale)
        char_img = char_img.resize((nw, nh), Image.LANCZOS)
        # Composite over panel background
        cx = px + (disp_w - nw) // 2
        cy = py + (disp_h - nh) // 2
        frame.paste(char_img, (cx, cy), char_img)

    # Speech bubble
    bubble_text = panel_cfg.get("speech_bubble")
    if bubble_text:
        draw_bubble_pil(draw, bubble_text, font,
                        (px, py, disp_w, disp_h),
                        panel_cfg.get("bubble_side", "right"), cfg)

    return frame


# ── Strip ────────────────────────────────────────────────────────────────────

def render_strip(beat_frames: list, gap: int = 8) -> Image.Image:
    """Composite all beats side by side."""
    n   = len(beat_frames)
    w   = beat_frames[0].width if beat_frames else 320
    h   = beat_frames[0].height if beat_frames else 240
    strip_w = n * (w // 2) + (n + 1) * gap
    strip_h = h // 2 + 2 * gap

    strip = Image.new("RGB", (strip_w, strip_h), (10, 10, 22))
    for i, frame in enumerate(beat_frames):
        thumb = frame.resize((w // 2, h // 2), Image.LANCZOS)
        x = gap + i * (w // 2 + gap)
        strip.paste(thumb, (x, gap))

    return strip


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    cfg     = load_config()
    demo    = cfg["demo"]
    W, H    = demo["width"], demo["height"]
    font_sz = cfg["animation"]["bubble_font_size"]
    font    = load_pil_font(font_sz)

    preview_dir = out_path(cfg, "preview")
    os.makedirs(preview_dir, exist_ok=True)

    frames = []
    for panel_cfg in cfg["panels"]:
        beat = panel_cfg["beat"]
        print(f"  rendering beat {beat} ({panel_cfg['label']}) ...", end=" ")
        frame = render_beat(panel_cfg, cfg, W, H, font)
        path  = os.path.join(preview_dir, f"beat_{beat}.png")
        frame.save(path)
        frames.append(frame)
        print(f"→ preview/beat_{beat}.png")

    strip = render_strip(frames)
    strip_path = os.path.join(preview_dir, "strip.png")
    strip.save(strip_path)
    print(f"  strip → preview/strip.png")
    print("\nDone.  Open assets/vanity_card/preview/ to review all frames.")


if __name__ == "__main__":
    main()
