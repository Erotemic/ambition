#!/usr/bin/env python3
"""
GNU-ton boss sprite generator.

GNU-ton is a scholar who stands on the shoulders of a giant GNU (wildebeest).
He mutters things like "I can see further than everyone else... and it's not Unix."

Visual design:
  - Giant GNU body: massive wildebeest with iconic curved horns, shaggy mane
  - Two huge stylized hooves/hands at the sides (the primary attack objects)
  - Small academic figure (GNU-ton) perched atop the GNU's neck
  - The GNU head is the primary vulnerable target — it descends to player level
    during the vulnerability window

Animation rows (mapping to BossAnim vocabulary):
  Row 0  rest       (6 frames) -> BossAnim::Rest
  Row 1  hand_slam  (7 frames) -> BossAnim::FloorSlam
  Row 2  hand_sweep (7 frames) -> BossAnim::SideSweep
  Row 3  head_down  (6 frames) -> BossAnim::SpikeHalo  (vulnerability window)
  Row 4  hit        (5 frames) -> BossAnim::Hit
  Row 5  death      (8 frames) -> BossAnim::Death

Frame size: 512x384  (wide to accommodate side-extending horns and hands)

Dependencies: python -m pip install pillow
"""
from __future__ import annotations

import json
import math
import shutil
from pathlib import Path
from typing import List, Optional, Tuple

from PIL import Image, ImageDraw, ImageFilter

RGBA = Tuple[int, int, int, int]

TARGET_NAME = "gnu_ton_boss"
DATA_DIR = Path(__file__).resolve().parent
TOOL_ROOT = DATA_DIR.parents[2]

FRAME_W = 512
FRAME_H = 384
FRAME_SIZE = (FRAME_W, FRAME_H)
SUPERSAMPLE = 2  # render at 2x then downsample for clean edges

# Origin in design coordinates (center of frame)
OX = FRAME_W // 2
OY = FRAME_H // 2

ANIMATIONS: List[Tuple[str, int, int]] = [
    ("rest",       6, 120),
    ("hand_slam",  7,  85),
    ("hand_sweep", 7,  75),
    ("head_down",  6,  95),
    ("hit",        5,  80),
    ("death",      8, 110),
]

OUTPUT_FILES = [
    f"{TARGET_NAME}_spritesheet.png",
    f"{TARGET_NAME}_spritesheet_manifest.json",
    f"{TARGET_NAME}_canonical.png",
    f"{TARGET_NAME}_preview_labeled.png",
]

# ── Palette ──────────────────────────────────────────────────────────────────
C_OUTLINE      = (20,  14,   8, 255)
C_BODY_DARK    = (48,  34,  20, 255)
C_BODY_MID     = (82,  60,  38, 255)
C_BODY_LIGHT   = (118, 88,  56, 255)
C_BODY_SPEC    = (148, 112, 72, 255)
C_HORN         = (212, 188, 142, 255)
C_HORN_TIP     = (238, 218, 176, 255)
C_HORN_DARK    = (162, 138,  98, 255)
C_MANE_DARK    = (58,  38,  22, 255)
C_MANE_MID     = (92,  66,  42, 255)
C_MANE_LIGHT   = (128, 96,  62, 255)
C_SNOUT        = (98,  74,  50, 255)
C_NOSTRIL      = (32,  20,  10, 255)
C_EYE_WHITE    = (230, 218, 200, 255)
C_EYE_IRIS     = (185, 138,  44, 255)
C_EYE_GLOW    = (255, 200,  60, 255)
C_EYE_GLOW2   = (255, 240, 140, 180)
C_PUPIL        = (12,   8,   4, 255)
C_HAND_DARK    = (52,  36,  22, 255)
C_HAND_MID     = (90,  66,  44, 255)
C_HAND_LIGHT   = (128, 98,  66, 255)
C_HOOF_DARK    = (32,  22,  12, 255)
C_HOOF_MID     = (58,  42,  26, 255)
C_KNUCKLE      = (148, 112, 72, 255)
C_MAN_ROBE     = (55,  78, 148, 255)
C_MAN_ROBE_D   = (38,  55, 108, 255)
C_MAN_ROBE_L   = (78, 108, 190, 255)
C_MAN_SKIN     = (198, 162, 126, 255)
C_MAN_HAIR     = (72,  52,  32, 255)
C_MAN_BEARD    = (88,  66,  44, 255)
C_MAN_SPEC     = (240, 220, 180, 255)
C_SPEECH_BG    = (248, 246, 238, 220)
C_SPEECH_EDGE  = (180, 170, 148, 255)
C_SPEECH_TXT   = (28,  22,  14, 255)
C_HIT_FLASH    = (255, 160,  60, 200)
C_DEATH_GREY   = (88,  80,  72, 255)
C_GLOW_RING    = (255, 200,  60,  60)
C_AMBER_GLOW   = (255, 180,  40,  80)
C_BG           = (0,   0,   0,   0)  # transparent background


def wave(phase: float, freq: float = 1.0, offset: float = 0.0) -> float:
    return math.sin(math.tau * (phase * freq + offset))


def blink01(phase: float, freq: float = 1.0, offset: float = 0.0) -> float:
    return 0.5 + 0.5 * wave(phase, freq, offset)


def lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def clamp(v: float, lo: float, hi: float) -> float:
    return max(lo, min(hi, v))


def smoothstep(t: float) -> float:
    t = clamp(t, 0.0, 1.0)
    return t * t * (3 - 2 * t)


class Canvas:
    """Drawing helper with design-space coordinates (origin at frame center)."""

    def __init__(self, w: int, h: int, bg: RGBA = C_BG, scale: int = 1):
        self.scale = scale
        self.sw = w * scale
        self.sh = h * scale
        self.img = Image.new("RGBA", (self.sw, self.sh), bg)
        self.draw = ImageDraw.Draw(self.img)
        # Origin in scaled pixel space
        self.ox = (w // 2) * scale
        self.oy = (h // 2) * scale

    def P(self, x: float, y: float) -> Tuple[int, int]:
        """Design-space point -> pixel coordinate."""
        return (int(round(self.ox + x * self.scale)),
                int(round(self.oy + y * self.scale)))

    def Ps(self, pts):
        return [self.P(x, y) for x, y in pts]

    def polygon(self, pts, fill: RGBA, outline: Optional[RGBA] = None, width: float = 1.5):
        px = self.Ps(pts)
        self.draw.polygon(px, fill=fill)
        if outline:
            self.draw.line(px + [px[0]], fill=outline,
                           width=max(1, int(round(width * self.scale))), joint="curve")

    def ellipse(self, cx: float, cy: float, rx: float, ry: float,
                fill: RGBA, outline: Optional[RGBA] = None, width: float = 1.5):
        p0 = self.P(cx - rx, cy - ry)
        p1 = self.P(cx + rx, cy + ry)
        self.draw.ellipse([p0, p1], fill=fill, outline=outline,
                          width=max(1, int(round(width * self.scale))))

    def line(self, pts, fill: RGBA, width: float = 2.0):
        px = self.Ps(pts)
        self.draw.line(px, fill=fill,
                       width=max(1, int(round(width * self.scale))), joint="curve")

    def arc_pts(self, cx: float, cy: float, rx: float, ry: float,
                start_deg: float, end_deg: float, n: int = 20) -> list:
        pts = []
        for i in range(n + 1):
            t = i / n
            a = math.radians(lerp(start_deg, end_deg, t))
            pts.append((cx + math.cos(a) * rx, cy + math.sin(a) * ry))
        return pts

    def finish(self) -> Image.Image:
        if self.scale > 1:
            return self.img.resize((self.sw // self.scale, self.sh // self.scale),
                                   Image.LANCZOS)
        return self.img


# ── Drawing primitives ───────────────────────────────────────────────────────

def draw_gnu_body(c: Canvas, body_y: float = 0.0, alpha_scale: float = 1.0,
                  phase: float = 0.0, anim: str = "rest") -> None:
    """Giant GNU torso, hindquarters, and legs."""
    # Subtle breathing
    breathe = wave(phase, 0.8) * 2.5 if anim not in ("death",) else 0
    by = body_y + breathe

    # ── Hindquarters ──
    hq = [(-140, by + 50), (-180, by + 30), (-200, by + 10),
          (-185, by - 20), (-140, by - 32), (-90, by - 28),
          (-55, by - 15), (-60, by + 40)]
    c.polygon(hq, C_BODY_DARK, C_OUTLINE, 1.5)
    # Highlight
    hq_hi = [(-150, by + 10), (-175, by - 5), (-160, by - 22), (-115, by - 25), (-85, by - 20)]
    c.polygon(hq_hi, C_BODY_MID)

    # ── Main torso / barrel ──
    torso = [(-60, by + 50), (-55, by - 15), (0, by - 38),
             (70, by - 32), (100, by - 10), (95, by + 45),
             (40, by + 58), (-20, by + 58)]
    c.polygon(torso, C_BODY_MID, C_OUTLINE, 1.5)

    # Torso highlight
    hi = [(-10, by - 28), (50, by - 24), (72, by - 8), (60, by + 10), (10, by + 6)]
    c.polygon(hi, C_BODY_LIGHT)

    # ── Shoulder hump ──
    hump = [(-20, by - 38), (10, by - 58), (40, by - 62),
            (65, by - 50), (70, by - 32), (20, by - 35)]
    c.polygon(hump, C_BODY_LIGHT, C_OUTLINE, 1.2)

    # ── Four legs ──
    legs = [
        (-130, by + 50, -145, by + 100, -138, by + 115),   # hind left
        (-70,  by + 52, -80,  by + 102, -72,  by + 118),   # hind right-ish
        (20,   by + 55, 12,   by + 108, 22,   by + 122),   # fore left
        (75,   by + 48, 82,   by + 100, 74,   by + 115),   # fore right
    ]
    for x0, y0, x1, y1, x2, y2 in legs:
        # Upper leg
        c.polygon([(x0 - 14, y0), (x0 + 10, y0), (x1 + 8, y1), (x1 - 12, y1)],
                  C_BODY_DARK, C_OUTLINE, 1.2)
        # Lower leg (slightly lighter)
        c.polygon([(x1 - 10, y1), (x1 + 7, y1), (x2 + 6, y2), (x2 - 9, y2)],
                  C_BODY_MID, C_OUTLINE, 1.2)
        # Hoof
        c.ellipse(x2 - 2, y2 + 6, 12, 6, C_HOOF_DARK, C_OUTLINE, 1.0)

    # ── Shaggy mane along chest/neck base ──
    for i in range(8):
        mx = lerp(-50, 50, i / 7)
        my = by - 25 + wave(i * 1.3, 1.0) * 5
        c.polygon([
            (mx - 8, my),
            (mx, my - 20 - i * 1.5),
            (mx + 8, my - 2),
        ], C_MANE_DARK if i % 2 == 0 else C_MANE_MID)


def draw_gnu_neck(c: Canvas, head_y_offset: float = 0.0, tilt: float = 0.0,
                  phase: float = 0.0, anim: str = "rest") -> None:
    """Thick muscular neck connecting body to head."""
    sway = wave(phase, 0.7) * 3.0
    nx = sway + tilt * 15
    ny = head_y_offset

    neck = [
        (-28 + nx * 0.3, ny + 30),
        (-22 + nx, ny - 20),
        (-12 + nx, ny - 60),
        (18 + nx, ny - 60),
        (28 + nx * 0.8, ny - 20),
        (30 + nx * 0.3, ny + 30),
    ]
    c.polygon(neck, C_BODY_MID, C_OUTLINE, 1.8)
    # Highlight
    hi = [(-4 + nx, ny + 10), (0 + nx, ny - 40), (14 + nx, ny - 52), (18 + nx, ny + 8)]
    c.polygon(hi, C_BODY_LIGHT)


def draw_gnu_horns(c: Canvas, hx: float = 0.0, hy: float = 0.0,
                   scale: float = 1.0, phase: float = 0.0, anim: str = "rest") -> None:
    """GNU's iconic curved horns (C-shaped: outward, down, then back up at tips)."""
    droop = 0.0
    if anim == "death":
        settle = min(1.0, phase * 1.5)
        droop = settle * 35

    for side, sx in (("left", -1), ("right", 1)):
        # Horn base: emerges from skull wide and sweeping outward
        bx = hx + sx * 32 * scale
        by = hy - 18 * scale

        # Control points for the C-curve
        # The GNU horn goes: out → curves down → then the tip curls back inward/up
        pts = [
            (bx,                      by),
            (bx + sx * 40 * scale,    by - 10 * scale + droop),
            (bx + sx * 80 * scale,    by + 20 * scale + droop),
            (bx + sx * 88 * scale,    by + 55 * scale + droop),
            (bx + sx * 70 * scale,    by + 85 * scale + droop * 0.5),
            (bx + sx * 45 * scale,    by + 90 * scale),
        ]

        # Draw horn as thick tapered line with decreasing width
        for i in range(len(pts) - 1):
            t = i / (len(pts) - 1)
            w = lerp(14, 5, t) * scale
            p0, p1 = pts[i], pts[i + 1]
            # Cross-section polygon for each segment
            dx = p1[0] - p0[0]
            dy = p1[1] - p0[1]
            length = math.hypot(dx, dy) or 1
            nx_v = -dy / length
            ny_v = dx / length
            color = C_HORN if i < len(pts) - 2 else C_HORN_TIP
            seg = [
                (p0[0] + nx_v * w, p0[1] + ny_v * w),
                (p0[0] - nx_v * w, p0[1] - ny_v * w),
                (p1[0] - nx_v * w * 0.7, p1[1] - ny_v * w * 0.7),
                (p1[0] + nx_v * w * 0.7, p1[1] + ny_v * w * 0.7),
            ]
            c.polygon(seg, color, C_OUTLINE, 1.0)

        # Horn tip cap
        tip = pts[-1]
        c.ellipse(tip[0], tip[1], 5 * scale, 5 * scale, C_HORN_TIP)


def draw_gnu_head(c: Canvas, hx: float = 0.0, hy: float = 0.0,
                  phase: float = 0.0, anim: str = "rest",
                  enraged: bool = False) -> None:
    """GNU's massive head: skull, snout, eyes, and horns."""
    sway = wave(phase, 0.5) * 4.0

    # Head sway
    if anim == "death":
        slump = min(1.0, phase * 1.4)
        hx += slump * -15
        hy += slump * 25
        sway = 0

    # ── Skull base ──
    skull = [
        (hx - 55 + sway, hy - 25),
        (hx - 60 + sway, hy + 5),
        (hx - 45 + sway, hy + 28),
        (hx - 10 + sway, hy + 38),
        (hx + 25 + sway, hy + 32),
        (hx + 50 + sway, hy + 12),
        (hx + 52 + sway, hy - 15),
        (hx + 35 + sway, hy - 30),
        (hx + 5 + sway,  hy - 34),
        (hx - 28 + sway, hy - 30),
    ]
    c.polygon(skull, C_BODY_MID, C_OUTLINE, 2.0)

    # Skull highlight
    hi = [(hx - 20 + sway, hy - 22), (hx + 15 + sway, hy - 26),
          (hx + 32 + sway, hy - 12), (hx + 20 + sway, hy + 5),
          (hx - 5 + sway, hy + 2), (hx - 22 + sway, hy - 10)]
    c.polygon(hi, C_BODY_LIGHT)

    # ── Wide snout ──
    snout = [
        (hx + 25 + sway, hy + 8),
        (hx + 50 + sway, hy + 12),
        (hx + 78 + sway, hy + 24),
        (hx + 82 + sway, hy + 42),
        (hx + 60 + sway, hy + 52),
        (hx + 28 + sway, hy + 48),
        (hx + 15 + sway, hy + 35),
    ]
    c.polygon(snout, C_SNOUT, C_OUTLINE, 1.8)

    # Snout highlight
    c.ellipse(hx + 55 + sway, hy + 32, 16, 10, C_BODY_LIGHT)

    # Nostrils
    c.ellipse(hx + 55 + sway, hy + 38, 7, 5, C_NOSTRIL)
    c.ellipse(hx + 72 + sway, hy + 36, 6, 4, C_NOSTRIL)

    # ── Eyes ──
    # The left eye (the one facing us more directly)
    ex = hx - 18 + sway
    ey = hy - 4
    c.ellipse(ex, ey, 14, 12, C_EYE_WHITE, C_OUTLINE, 1.5)
    c.ellipse(ex + 3, ey + 1, 9, 9, C_EYE_IRIS)
    c.ellipse(ex + 4, ey + 2, 5, 5, C_PUPIL)
    # Eye shine
    c.ellipse(ex + 1, ey - 3, 3, 3, (255, 255, 255, 180))

    # Right eye (partially occluded by snout angle)
    ex2 = hx + 20 + sway
    ey2 = hy - 8
    c.ellipse(ex2, ey2, 11, 9, C_EYE_WHITE, C_OUTLINE, 1.0)
    c.ellipse(ex2 + 2, ey2 + 1, 7, 7, C_EYE_IRIS)
    c.ellipse(ex2 + 3, ey2 + 1, 4, 4, C_PUPIL)

    # # Keep both pupils looking toward the player. Earlier versions used
    # # oversized, differently aimed circles that read as crossed/goofy eyes.
    # ex = hx - 20 + sway
    # ey = hy - 7
    # ex2 = hx + 18 + sway
    # ey2 = hy - 9
    # c.ellipse(ex, ey, 12, 9, C_EYE_WHITE, C_OUTLINE, 1.4)
    # c.ellipse(ex + 1, ey + 1, 7, 7, C_EYE_IRIS)
    # c.ellipse(ex + 1, ey + 1, 4, 4, C_PUPIL)
    # c.ellipse(ex - 2, ey - 2, 2.5, 2.5, (255, 255, 255, 180))

    # # Far eye is slightly smaller and partly shadowed by the snout angle,
    # # but its pupil still aims the same direction as the near eye.
    # c.ellipse(ex2, ey2, 9, 7, C_EYE_WHITE, C_OUTLINE, 1.0)
    # c.ellipse(ex2 + 1, ey2 + 1, 5.5, 5.5, C_EYE_IRIS)
    # c.ellipse(ex2 + 1, ey2 + 1, 3.2, 3.2, C_PUPIL)
    # c.ellipse(ex2 - 1.5, ey2 - 1.5, 2.0, 2.0, (255, 255, 255, 160))

    # brow = C_MANE_DARK
    # c.line([(ex - 14, ey - 14), (ex - 2, ey - 18), (ex + 12, ey - 14)], brow, 3.0)
    # c.line([(ex2 - 10, ey2 - 11), (ex2 + 1, ey2 - 14), (ex2 + 10, ey2 - 11)], brow, 2.2)

    if enraged or anim in ("head_down", "hand_slam", "hand_sweep"):
        # Angry glow around eyes
        intensity = 0.7 + 0.3 * blink01(phase, 2.5)
        glow = (int(255 * intensity), int(160 * intensity), 20, 120)
        c.ellipse(ex, ey, 22, 18, glow)
        c.ellipse(ex2, ey2, 18, 14, glow)

    # ── Horns ──
    draw_gnu_horns(c, hx + sway, hy, 1.0, phase, anim)


def draw_hand(c: Canvas, cx: float, cy: float, side: int = 1,
              phase: float = 0.0, anim: str = "rest",
              slam_progress: float = 0.0, sweep_progress: float = 0.0) -> None:
    """One of the giant stylized hoof-hands at the sides."""
    # side: +1 = right, -1 = left

    # Main mass: large rounded hoof shape
    # The "knuckle" side faces inward (toward the player)
    hw = 56 + abs(math.sin(phase * 0.6)) * 3
    hh = 48 + abs(math.cos(phase * 0.8)) * 2

    # Hoof shape: wider at knuckle end, narrowing to hoof tip
    # For left hand: tip points right; for right: tip points left
    tip_x = cx + side * -48
    knuckle_x = cx + side * 20

    pts = [
        (knuckle_x, cy - hh * 0.7),
        (knuckle_x + side * 15, cy - hh * 0.4),
        (knuckle_x + side * 18, cy + hh * 0.2),
        (knuckle_x, cy + hh * 0.7),
        (tip_x, cy + hh * 0.4),
        (tip_x - side * 8, cy),
        (tip_x, cy - hh * 0.5),
    ]
    c.polygon(pts, C_HAND_DARK, C_OUTLINE, 2.0)

    # Mid-tone fill panel
    mid_pts = [
        (knuckle_x - side * 5, cy - hh * 0.5),
        (knuckle_x + side * 10, cy - hh * 0.2),
        (knuckle_x + side * 12, cy + hh * 0.1),
        (knuckle_x - side * 5, cy + hh * 0.5),
        (tip_x + side * 8, cy + hh * 0.25),
        (tip_x + side * 8, cy - hh * 0.25),
    ]
    c.polygon(mid_pts, C_HAND_MID)

    # Knuckle ridges
    for i in range(3):
        ky = cy + (i - 1) * hh * 0.26
        c.ellipse(knuckle_x, ky, 8, 5, C_KNUCKLE, C_OUTLINE, 0.8)

    # Hoof tip (darker, hard)
    tip_pts = [
        (tip_x, cy - hh * 0.4),
        (tip_x - side * 12, cy),
        (tip_x, cy + hh * 0.35),
    ]
    c.polygon(tip_pts, C_HOOF_DARK, C_OUTLINE, 1.0)

    # Impact glow during slam
    if slam_progress > 0.3 and anim == "hand_slam":
        glow_alpha = int(120 * min(1.0, (slam_progress - 0.3) * 4))
        glow = (255, 180, 60, glow_alpha)
        c.ellipse(cx, cy + hh, 40, 16, glow)

    # Wind trail during sweep
    if sweep_progress > 0.4 and anim == "hand_sweep":
        trail_alpha = int(80 * min(1.0, sweep_progress))
        trail = (120, 180, 255, trail_alpha)
        c.polygon([
            (cx + side * 30, cy - 20),
            (cx + side * 80, cy - 10),
            (cx + side * 80, cy + 10),
            (cx + side * 30, cy + 20),
        ], trail)


def draw_gnu_ton_man(c: Canvas, hx: float = 0.0, hy: float = 0.0,
                     phase: float = 0.0, anim: str = "rest",
                     show_speech: bool = False) -> None:
    """The GNU-ton scholar standing atop the GNU's neck/back."""
    # Small figure - roughly 40px tall
    # Head
    c.ellipse(hx, hy - 16, 8, 8, C_MAN_SKIN, C_OUTLINE, 1.0)

    # Hair (scraggly academic hair)
    c.polygon([
        (hx - 8, hy - 22), (hx - 5, hy - 28),
        (hx + 2, hy - 30), (hx + 8, hy - 24),
        (hx + 6, hy - 18),
    ], C_MAN_HAIR)

    # Tiny beard
    beard_bob = wave(phase, 1.2) * 1.5
    c.polygon([
        (hx - 4, hy - 10),
        (hx + 4, hy - 10),
        (hx + 3, hy - 4 + beard_bob),
        (hx, hy - 2 + beard_bob),
        (hx - 3, hy - 4 + beard_bob),
    ], C_MAN_BEARD)

    # Robe body
    arm_swing = wave(phase, 1.4) * 4
    robe = [
        (hx - 10, hy - 10),
        (hx + 10, hy - 10),
        (hx + 14, hy + 18),
        (hx, hy + 22),
        (hx - 14, hy + 18),
    ]
    c.polygon(robe, C_MAN_ROBE, C_OUTLINE, 1.0)
    # Robe highlight stripe
    c.polygon([
        (hx - 2, hy - 8), (hx + 3, hy - 8),
        (hx + 4, hy + 12), (hx - 3, hy + 12),
    ], C_MAN_ROBE_L)

    # Arms
    if anim == "rest":
        # Gesturing arm (muttering)
        arm_phase = wave(phase, 1.8) * 6
        c.line([
            (hx + 10, hy - 2),
            (hx + 22, hy - 8 + arm_phase),
            (hx + 28, hy - 4 + arm_phase),
        ], C_MAN_SKIN, 3.0)
        c.line([
            (hx - 10, hy - 2),
            (hx - 16, hy + 6),
        ], C_MAN_ROBE_D, 2.5)
    elif anim == "death":
        settle = min(1.0, phase * 1.5)
        c.line([
            (hx + 10, hy - 2),
            (hx + 20 + settle * 10, hy + 10 + settle * 20),
        ], C_MAN_SKIN, 2.5)
        c.line([
            (hx - 10, hy - 2),
            (hx - 20 - settle * 8, hy + 8 + settle * 16),
        ], C_MAN_SKIN, 2.5)
    else:
        c.line([(hx + 10, hy - 2), (hx + 16, hy + 8)], C_MAN_SKIN, 2.5)
        c.line([(hx - 10, hy - 2), (hx - 16, hy + 6)], C_MAN_SKIN, 2.5)

    # Tiny feet
    c.ellipse(hx - 6, hy + 22, 5, 3, C_MAN_HAIR, C_OUTLINE, 0.8)
    c.ellipse(hx + 6, hy + 22, 5, 3, C_MAN_HAIR, C_OUTLINE, 0.8)

    # Speech bubble (shows during rest anim, first 3 frames)
    if show_speech:
        bx = hx + 18
        by = hy - 38
        bw, bh = 72, 30
        # Bubble body
        c.polygon([
            (bx, by), (bx + bw, by),
            (bx + bw, by + bh), (bx + 8, by + bh),
            (bx + 4, by + bh + 10), (bx + 16, by + bh),
            (bx, by + bh),
        ], C_SPEECH_BG, C_SPEECH_EDGE, 1.0)
        # Draw text as tiny lines (no PIL text font needed)
        # "...not Unix!" visualized as horizontal bars
        lines = [
            (bx + 6, by + 8,  bx + 52, by + 8),
            (bx + 6, by + 14, bx + 60, by + 14),
            (bx + 6, by + 20, bx + 40, by + 20),
        ]
        for x0, y0, x1, y1 in lines:
            c.line([(x0, y0), (x1, y1)], C_SPEECH_TXT, 1.5)


# ── Per-animation frame drawing ──────────────────────────────────────────────

def draw_frame(anim: str, frame_idx: int, frame_count: int) -> Image.Image:
    """Render one animation frame and return a FRAME_SIZE RGBA image."""
    phase = frame_idx / max(1, frame_count)
    c = Canvas(FRAME_W, FRAME_H, C_BG, scale=SUPERSAMPLE)

    if anim == "rest":
        _draw_rest(c, phase, frame_idx)
    elif anim == "hand_slam":
        _draw_hand_slam(c, phase)
    elif anim == "hand_sweep":
        _draw_hand_sweep(c, phase)
    elif anim == "head_down":
        _draw_head_down(c, phase)
    elif anim == "hit":
        _draw_hit(c, phase)
    elif anim == "death":
        _draw_death(c, phase)
    else:
        _draw_rest(c, phase, frame_idx)

    return c.finish()


# Giant GNU is raised so the full silhouette reads clearly:
#   Body at +50 below center  (250px from top in 384px frame)
#   Shoulder hump peak: body_y - 62 = -12 (center of frame = 192px from top)
#   Head at -95 above center  (97px from top)
#   Hands at +20 below center (212px from top)
#
# Scholar (GNU-ton) stands on the RIGHT shoulder of the giant (x≈+28), feet
# resting on the hump peak (y≈-12), so his center is at y≈-38. He is NOT
# on the head — the head is the attack target that periodically descends.
REST_HEAD_Y = -95.0
REST_HAND_Y = 20.0
REST_BODY_Y = 50.0

# Shoulder contact point: body hump peak is at (body_y - 62).
# Man feet sit here; man center (hy in draw_gnu_ton_man) = foot_y - 22.
_SHOULDER_TOP_Y = REST_BODY_Y - 62  # ≈ -12
_MAN_CENTER_Y   = _SHOULDER_TOP_Y - 22  # ≈ -34
_MAN_CENTER_X   = 28.0  # right shoulder


def _draw_rest(c: Canvas, phase: float, frame_idx: int) -> None:
    """Idle: gentle sway, scholar muttering from the GNU's right shoulder."""
    bob = wave(phase, 0.9) * 3.5
    head_y = REST_HEAD_Y + bob * 0.6
    # Keep the original 469cea7 neck overlap: top tucks into the head and
    # bottom reaches the raised shoulder/torso instead of floating above it.
    neck_offset = head_y + 70

    # Body in background (lower portion)
    draw_gnu_body(c, body_y=REST_BODY_Y, phase=phase, anim="rest")
    draw_gnu_neck(c, head_y_offset=neck_offset, tilt=0.0, phase=phase, anim="rest")

    # Hands resting at sides
    lhx = wave(phase, 0.6) * 5 - 185
    lhy = REST_HAND_Y + wave(phase, 0.85, 0.1) * 4
    rhx = wave(phase, 0.6, 0.25) * 5 + 185
    rhy = REST_HAND_Y + wave(phase, 0.85, 0.35) * 4
    draw_hand(c, lhx, lhy, side=-1, phase=phase, anim="rest")
    draw_hand(c, rhx, rhy, side=+1, phase=phase, anim="rest")

    # Head (above body — the attack target, not where scholar sits)
    draw_gnu_head(c, 0.0, head_y, phase=phase, anim="rest")

    # Scholar on the right shoulder (NOT on the head).
    # Slight bob shares the body breathing rhythm.
    man_y = _MAN_CENTER_Y + bob * 0.4
    man_x = _MAN_CENTER_X + wave(phase, 0.7, 0.2) * 1.5
    draw_gnu_ton_man(c, man_x, man_y, phase=phase, anim="rest", show_speech=False)


def _draw_hand_slam(c: Canvas, phase: float) -> None:
    """Hands raise then slam down from above."""
    # 0.0-0.3: raising; 0.3-0.6: fast slam; 0.6-1.0: recover
    if phase < 0.3:
        t = phase / 0.3
        slam_y = lerp(REST_HAND_Y, -100, smoothstep(t))
        slam_alpha = 0.0
    elif phase < 0.6:
        t = (phase - 0.3) / 0.3
        slam_y = lerp(-100, 120, smoothstep(t) * 1.2)
        slam_alpha = smoothstep(t)
    else:
        t = (phase - 0.6) / 0.4
        slam_y = lerp(120, REST_HAND_Y, smoothstep(t))
        slam_alpha = lerp(1.0, 0.0, t)

    head_y = REST_HEAD_Y
    draw_gnu_body(c, body_y=REST_BODY_Y, phase=phase, anim="hand_slam")
    draw_gnu_neck(c, head_y_offset=head_y + 70, phase=phase, anim="hand_slam")

    draw_hand(c, -185, slam_y, side=-1, phase=phase, anim="hand_slam",
              slam_progress=slam_alpha)
    draw_hand(c, 185, slam_y, side=+1, phase=phase, anim="hand_slam",
              slam_progress=slam_alpha)

    # Impact shockwave at slam
    if slam_alpha > 0.5:
        ws = (slam_alpha - 0.5) * 2.0
        for r in [0.4, 0.7, 1.0]:
            a = int(100 * ws * (1 - r * 0.6))
            c.ellipse(0, 120, 180 * r * ws, 22 * r * ws, (255, 200, 80, a))

    draw_gnu_head(c, 0.0, head_y, phase=phase, anim="hand_slam", enraged=True)
    draw_gnu_ton_man(c, _MAN_CENTER_X, _MAN_CENTER_Y, phase=phase, anim="hand_slam")


def _draw_hand_sweep(c: Canvas, phase: float) -> None:
    """Hands sweep in from the far sides."""
    # 0.0-0.2: wind up; 0.2-0.65: fast sweep; 0.65-1.0: recover
    if phase < 0.2:
        t = phase / 0.2
        lhx = lerp(-185, -240, smoothstep(t))
        rhx = lerp(185,  240, smoothstep(t))
        sweep_prog = 0.0
    elif phase < 0.65:
        t = (phase - 0.2) / 0.45
        lhx = lerp(-240, -80, smoothstep(t))
        rhx = lerp(240,   80, smoothstep(t))
        sweep_prog = smoothstep(t)
    else:
        t = (phase - 0.65) / 0.35
        lhx = lerp(-80, -185, smoothstep(t))
        rhx = lerp(80,   185, smoothstep(t))
        sweep_prog = lerp(1.0, 0.0, t)

    head_y = REST_HEAD_Y
    draw_gnu_body(c, body_y=REST_BODY_Y, phase=phase, anim="hand_sweep")
    draw_gnu_neck(c, head_y_offset=head_y + 70, phase=phase, anim="hand_sweep")

    draw_hand(c, lhx, REST_HAND_Y, side=-1, phase=phase, anim="hand_sweep",
              sweep_progress=sweep_prog)
    draw_hand(c, rhx, REST_HAND_Y, side=+1, phase=phase, anim="hand_sweep",
              sweep_progress=sweep_prog)

    draw_gnu_head(c, 0.0, head_y, phase=phase, anim="hand_sweep", enraged=True)
    draw_gnu_ton_man(c, _MAN_CENTER_X, _MAN_CENTER_Y, phase=phase, anim="hand_sweep")


def _draw_head_down(c: Canvas, phase: float) -> None:
    """Head descends dramatically — vulnerability window."""
    # 0.0-0.45: descend; 0.45-0.75: held low (player can attack); 0.75-1.0: rise
    if phase < 0.45:
        t = phase / 0.45
        head_y = lerp(REST_HEAD_Y, 30, smoothstep(t))
        enrage_scale = smoothstep(t)
    elif phase < 0.75:
        head_y = 30.0
        enrage_scale = 1.0
    else:
        t = (phase - 0.75) / 0.25
        head_y = lerp(30, REST_HEAD_Y, smoothstep(t))
        enrage_scale = lerp(1.0, 0.0, t)

    draw_gnu_body(c, body_y=REST_BODY_Y, phase=phase, anim="head_down")
    draw_gnu_neck(c, head_y_offset=head_y + 55, tilt=0.3, phase=phase, anim="head_down")

    c_sway = wave(phase, 1.5) * 8
    draw_hand(c, -185 + c_sway, REST_HAND_Y, side=-1, phase=phase, anim="head_down")
    draw_hand(c, 185 - c_sway, REST_HAND_Y, side=+1, phase=phase, anim="head_down")

    # Vulnerable head glow ring
    if enrage_scale > 0.3:
        ga = int(80 * enrage_scale * (0.7 + 0.3 * blink01(phase, 3.0)))
        c.ellipse(0.0, head_y, 100 * enrage_scale, 80 * enrage_scale, (255, 220, 60, ga))

    draw_gnu_head(c, 0.0, head_y, phase=phase, anim="head_down", enraged=(enrage_scale > 0.5))
    draw_gnu_ton_man(c, _MAN_CENTER_X, _MAN_CENTER_Y, phase=phase, anim="head_down")


def _draw_hit(c: Canvas, phase: float) -> None:
    """Hit flash and brief recoil."""
    jolt = wave(phase, 2.0) * 8
    flash_alpha = int(150 * (1.0 - phase))

    body_y_hit = REST_BODY_Y + jolt * 0.3
    head_y = REST_HEAD_Y + jolt * 0.5
    draw_gnu_body(c, body_y=body_y_hit, phase=phase, anim="hit")
    draw_gnu_neck(c, head_y_offset=head_y + 70, phase=phase, anim="hit")
    draw_hand(c, -185 + jolt, REST_HAND_Y - jolt * 0.5, side=-1, phase=phase, anim="hit")
    draw_hand(c, 185 + jolt, REST_HAND_Y + jolt * 0.3, side=+1, phase=phase, anim="hit")
    draw_gnu_head(c, jolt * 0.7, head_y, phase=phase, anim="hit")
    draw_gnu_ton_man(c, _MAN_CENTER_X + jolt * 0.4, _MAN_CENTER_Y + jolt * 0.2, phase=phase, anim="hit")

    if flash_alpha > 0:
        flash_img = Image.new("RGBA", (c.sw, c.sh), (255, 140, 40, flash_alpha))
        c.img = Image.alpha_composite(c.img, flash_img)
        c.draw = ImageDraw.Draw(c.img)


def _draw_death(c: Canvas, phase: float) -> None:
    """Boss collapses: horns droop, body slumps, man tumbles off."""
    settle = min(1.0, phase * 1.2)

    head_y = lerp(REST_HEAD_Y, 60, smoothstep(settle * 1.1))
    body_y = lerp(REST_BODY_Y, 100.0, smoothstep(settle))
    # Scholar starts on shoulder then tumbles sideways as body collapses
    man_y = _MAN_CENTER_Y + settle * 110
    man_x = _MAN_CENTER_X + settle * 60

    draw_gnu_body(c, body_y=body_y, phase=phase, anim="death")
    draw_gnu_neck(c, head_y_offset=head_y + 70, phase=phase, anim="death")

    lhx = lerp(-185, -205, settle)
    lhy = lerp(REST_HAND_Y, 110, smoothstep(settle))
    rhx = lerp(185, 210, settle)
    rhy = lerp(REST_HAND_Y, 105, smoothstep(settle))
    draw_hand(c, lhx, lhy, side=-1, phase=phase, anim="death")
    draw_hand(c, rhx, rhy, side=+1, phase=phase, anim="death")

    draw_gnu_head(c, 0.0, head_y, phase=phase, anim="death")

    if settle < 0.9:
        draw_gnu_ton_man(c, man_x, man_y, phase=phase, anim="death")

    grey_blend = settle * 0.7
    if grey_blend > 0:
        grey = Image.new("RGBA", (c.sw, c.sh), (100, 90, 80, int(grey_blend * 140)))
        c.img = Image.alpha_composite(c.img, grey)
        c.draw = ImageDraw.Draw(c.img)


# ── Sheet assembly ───────────────────────────────────────────────────────────

def build_spritesheet(outdir: Path) -> Tuple[Path, Path]:
    """Render all animation frames and assemble into a spritesheet PNG + manifest."""
    max_frames = max(frames for _, frames, _ in ANIMATIONS)
    rows = len(ANIMATIONS)
    sheet_w = max_frames * FRAME_W
    sheet_h = rows * FRAME_H
    sheet = Image.new("RGBA", (sheet_w, sheet_h), (0, 0, 0, 0))

    manifest = {
        "target": TARGET_NAME,
        "frame_size": [FRAME_W, FRAME_H],
        "rows": [],
    }

    for row_idx, (anim_name, frame_count, duration_ms) in enumerate(ANIMATIONS):
        frames_out = []
        for f in range(frame_count):
            img = draw_frame(anim_name, f, frame_count)
            x = f * FRAME_W
            y = row_idx * FRAME_H
            sheet.paste(img, (x, y))
            frames_out.append(f"row{row_idx}_frame{f:02d}")
        manifest["rows"].append({
            "name": anim_name,
            "row": row_idx,
            "frames": frame_count,
            "duration_ms": duration_ms,
        })
        print(f"  [{row_idx + 1}/{rows}] {anim_name} ({frame_count} frames)")

    sheet_path = outdir / f"{TARGET_NAME}_spritesheet.png"
    manifest_path = outdir / f"{TARGET_NAME}_spritesheet_manifest.json"
    sheet.save(str(sheet_path), "PNG")
    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)

    return sheet_path, manifest_path


def build_canonical(outdir: Path) -> Path:
    """Render a single canonical reference pose."""
    img = draw_frame("rest", 0, 6)
    path = outdir / f"{TARGET_NAME}_canonical.png"
    big = img.resize((img.width * 2, img.height * 2), Image.LANCZOS)
    big.save(str(path), "PNG")
    return path


def build_preview_labeled(outdir: Path) -> Path:
    """Render a labeled preview strip showing all animations."""
    frames_per_row = [frame_count for _, frame_count, _ in ANIMATIONS]
    preview_h = len(ANIMATIONS) * (FRAME_H // 2 + 18)
    preview_w = max(frames_per_row) * (FRAME_W // 2)
    preview = Image.new("RGBA", (preview_w, preview_h), (30, 22, 15, 255))

    for row_idx, (anim_name, frame_count, _) in enumerate(ANIMATIONS):
        y_base = row_idx * (FRAME_H // 2 + 18)
        for f in range(frame_count):
            img = draw_frame(anim_name, f, frame_count)
            thumb = img.resize((FRAME_W // 2, FRAME_H // 2), Image.LANCZOS)
            x = f * (FRAME_W // 2)
            preview.paste(thumb, (x, y_base), thumb)

    path = outdir / f"{TARGET_NAME}_preview_labeled.png"
    preview.save(str(path), "PNG")
    return path


def render_outputs(outdir: Path, quick: bool = False) -> List[Path]:
    """Render all outputs into outdir. Returns list of generated paths."""
    outdir.mkdir(parents=True, exist_ok=True)
    print(f"[{TARGET_NAME}] rendering to {outdir}/")

    paths = []
    print(f"  spritesheet...")
    sp, mp = build_spritesheet(outdir)
    paths += [sp, mp]

    print(f"  canonical...")
    paths.append(build_canonical(outdir))

    if not quick:
        print(f"  preview...")
        paths.append(build_preview_labeled(outdir))

    print(f"  done. {len(paths)} files.")
    return paths


def install_outputs(render_dir: Path, install_dir: Path) -> List[Path]:
    """Copy generated PNG + manifest into the sandbox assets tree."""
    install_dir.mkdir(parents=True, exist_ok=True)
    copied = []
    for fname in OUTPUT_FILES:
        src = render_dir / fname
        if not src.exists():
            print(f"  [WARN] missing: {src.name}")
            continue
        dst = install_dir / fname
        shutil.copy2(src, dst)
        copied.append(dst)
        print(f"  installed: {dst.relative_to(install_dir.parents[3])}")
    return copied


if __name__ == "__main__":
    import argparse, sys
    p = argparse.ArgumentParser()
    p.add_argument("outdir", nargs="?", default="generated/gnu_ton_boss")
    p.add_argument("--quick", action="store_true")
    args = p.parse_args()
    paths = render_outputs(Path(args.outdir), quick=args.quick)
    for path in paths:
        print(path)
