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

import math
import shutil
from pathlib import Path
from typing import List, Optional, Tuple

from PIL import Image, ImageDraw, ImageFilter

RGBA = Tuple[int, int, int, int]

TARGET_NAME = "gnu_ton_boss"
DATA_DIR = Path(__file__).resolve().parent
TOOL_ROOT = DATA_DIR.parents[2]

# Frame canvas — bumped from 512×384 → 768×576 for less in-game pixelation
# at the boss's blown-up render scale. Design-space coordinates and anchor
# offsets stay frame-relative so the existing layout still reads correctly.
FRAME_W = 768
FRAME_H = 576
FRAME_SIZE = (FRAME_W, FRAME_H)
SUPERSAMPLE = 2  # render at 2x then downsample for clean edges

# Origin in design coordinates (center of frame)
OX = FRAME_W // 2
OY = FRAME_H // 2

# Frame counts bumped for smoother in-game animation. Rest cycles a longer
# breathing loop; attack rows interpolate the windup → strike → recover arc
# across more frames so the giant reads as deliberate, not jerky.
ANIMATIONS: List[Tuple[str, int, int]] = [
    ("rest", 10, 110),
    ("hand_slam", 10, 72),
    ("hand_sweep", 10, 65),
    ("head_down", 9, 90),
    ("hit", 6, 80),
    ("death", 10, 105),
]

# Output files: the runtime consumes the split body/hands pair plus the
# canonical SheetRecord RON. Runtime collision/hurtbox metadata is stored in
# the RON's `body_metrics` block, so the old JSON manifest/parts sidecars are
# no longer emitted for GNU-ton.
OUTPUT_FILES = [
    f"{TARGET_NAME}_spritesheet.png",  # alias for _full (back-compat)
    f"{TARGET_NAME}_full_spritesheet.png",
    f"{TARGET_NAME}_body_spritesheet.png",
    f"{TARGET_NAME}_hands_spritesheet.png",
    f"{TARGET_NAME}_spritesheet.ron",
    f"{TARGET_NAME}_actor.ron",
    f"{TARGET_NAME}_canonical.png",
    f"{TARGET_NAME}_preview_labeled.png",
]

# Review-only output generated next to the sheets. It is intentionally not in
# OUTPUT_FILES, so `publish` does not install it into the runtime asset tree.
HITBOX_DEBUG_FILE = f"{TARGET_NAME}_hitboxes_debug.png"

ACTOR_METADATA = {
    "actor": {"character_id": f"npc_{TARGET_NAME}"},
    "body": {
        "body_plan": "BossMultipart",
        "body_kind": "Wide",
        "traits": ["boss", "multipart"],
    },
    "brain": {"default_preset": "stand_still"},
    "actions": {"default_preset": "peaceful"},
    "tags": ["boss", "multipart"],
    "missing_information": [
        "boss schedule/action specials: not authored in the sprite actor contract yet",
    ],
}

# ── Palette ──────────────────────────────────────────────────────────────────
C_OUTLINE = (20, 14, 8, 255)
C_BODY_DARK = (48, 34, 20, 255)
C_BODY_MID = (82, 60, 38, 255)
C_BODY_LIGHT = (118, 88, 56, 255)
C_BODY_SPEC = (148, 112, 72, 255)
C_HORN = (212, 188, 142, 255)
C_HORN_TIP = (238, 218, 176, 255)
C_HORN_DARK = (162, 138, 98, 255)
C_MANE_DARK = (58, 38, 22, 255)
C_MANE_MID = (92, 66, 42, 255)
C_MANE_LIGHT = (128, 96, 62, 255)
C_SNOUT = (98, 74, 50, 255)
C_NOSTRIL = (32, 20, 10, 255)
C_EYE_WHITE = (230, 218, 200, 255)
C_EYE_IRIS = (185, 138, 44, 255)
C_EYE_GLOW = (255, 200, 60, 255)
C_EYE_GLOW2 = (255, 240, 140, 180)
C_PUPIL = (12, 8, 4, 255)
C_HAND_DARK = (52, 36, 22, 255)
C_HAND_MID = (90, 66, 44, 255)
C_HAND_LIGHT = (128, 98, 66, 255)
C_HOOF_DARK = (32, 22, 12, 255)
C_HOOF_MID = (58, 42, 26, 255)
C_KNUCKLE = (148, 112, 72, 255)
C_MAN_ROBE = (55, 78, 148, 255)
C_MAN_ROBE_D = (38, 55, 108, 255)
C_MAN_ROBE_L = (78, 108, 190, 255)
C_MAN_SKIN = (198, 162, 126, 255)
C_MAN_HAIR = (72, 52, 32, 255)
C_MAN_BEARD = (88, 66, 44, 255)
C_MAN_SPEC = (240, 220, 180, 255)
# Powdered wig (Isaac Newton style): cream-white with a soft warm shadow
# so the curls read against the dark wildebeest mane behind the scholar.
C_MAN_WIG = (240, 234, 220, 255)
C_MAN_WIG_S = (190, 180, 158, 255)
C_MAN_WIG_D = (138, 128, 110, 255)
C_SPEECH_BG = (248, 246, 238, 220)
C_SPEECH_EDGE = (180, 170, 148, 255)
C_SPEECH_TXT = (28, 22, 14, 255)
C_HIT_FLASH = (255, 160, 60, 200)
C_DEATH_GREY = (88, 80, 72, 255)
C_GLOW_RING = (255, 200, 60, 60)
C_AMBER_GLOW = (255, 180, 40, 80)
C_BG = (0, 0, 0, 0)  # transparent background


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
        return (
            int(round(self.ox + x * self.scale)),
            int(round(self.oy + y * self.scale)),
        )

    def Ps(self, pts):
        return [self.P(x, y) for x, y in pts]

    def polygon(
        self, pts, fill: RGBA, outline: Optional[RGBA] = None, width: float = 1.5
    ):
        px = self.Ps(pts)
        self.draw.polygon(px, fill=fill)
        if outline:
            self.draw.line(
                px + [px[0]],
                fill=outline,
                width=max(1, int(round(width * self.scale))),
                joint="curve",
            )

    def ellipse(
        self,
        cx: float,
        cy: float,
        rx: float,
        ry: float,
        fill: RGBA,
        outline: Optional[RGBA] = None,
        width: float = 1.5,
    ):
        p0 = self.P(cx - rx, cy - ry)
        p1 = self.P(cx + rx, cy + ry)
        self.draw.ellipse(
            [p0, p1],
            fill=fill,
            outline=outline,
            width=max(1, int(round(width * self.scale))),
        )

    def alpha_ellipse(
        self,
        cx: float,
        cy: float,
        rx: float,
        ry: float,
        fill: RGBA,
        blur: float = 0.0,
        outline: Optional[RGBA] = None,
        width: float = 1.5,
    ):
        """Draw a translucent ellipse using real alpha compositing."""
        layer = Image.new("RGBA", (self.sw, self.sh), (0, 0, 0, 0))
        d = ImageDraw.Draw(layer, "RGBA")

        p0 = self.P(cx - rx, cy - ry)
        p1 = self.P(cx + rx, cy + ry)

        d.ellipse([p0, p1], fill=fill)

        if outline:
            d.ellipse(
                [p0, p1],
                outline=outline,
                width=max(1, int(round(width * self.scale))),
            )

        if blur > 0:
            layer = layer.filter(
                ImageFilter.GaussianBlur(max(1, int(round(blur * self.scale))))
            )

        self.img = Image.alpha_composite(self.img, layer)
        self.draw = ImageDraw.Draw(self.img)

    def line(self, pts, fill: RGBA, width: float = 2.0):
        px = self.Ps(pts)
        self.draw.line(
            px, fill=fill, width=max(1, int(round(width * self.scale))), joint="curve"
        )

    def arc_pts(
        self,
        cx: float,
        cy: float,
        rx: float,
        ry: float,
        start_deg: float,
        end_deg: float,
        n: int = 20,
    ) -> list:
        pts = []
        for i in range(n + 1):
            t = i / n
            a = math.radians(lerp(start_deg, end_deg, t))
            pts.append((cx + math.cos(a) * rx, cy + math.sin(a) * ry))
        return pts

    def finish(self) -> Image.Image:
        if self.scale > 1:
            return self.img.resize(
                (self.sw // self.scale, self.sh // self.scale), Image.LANCZOS
            )
        return self.img


# ── Drawing primitives ───────────────────────────────────────────────────────


def _leg_lift(leg_idx: int, phase: float, anim: str) -> float:
    """Per-leg vertical lift in design pixels for walk-in-place shuffle.

    Diagonal pairs (hind-left + fore-right) walk in counter-phase to the
    other pair so the giant reads as a quadruped marching in place rather
    than bouncing all four hooves at once. Amplitude grows during attacks
    so the body sells the brace step.
    """
    pair_phase = phase if leg_idx in (0, 3) else phase + 0.5
    if anim == "death":
        return 0.0
    if anim in ("hand_slam", "hand_sweep"):
        # Wider shuffle while bracing for an attack — the giant rocks.
        return max(0.0, wave(pair_phase, 1.2)) * 7.0 - 2.5
    if anim == "head_down":
        return max(0.0, wave(pair_phase, 1.0)) * 4.0 - 1.0
    if anim == "hit":
        return wave(pair_phase, 2.5) * 3.0
    # Rest: continuous in-place walking shuffle so the giant isn't a statue.
    return max(0.0, wave(pair_phase, 0.85)) * 5.5 - 1.5


def draw_gnu_body(
    c: Canvas,
    body_y: float = 0.0,
    alpha_scale: float = 1.0,
    phase: float = 0.0,
    anim: str = "rest",
) -> None:
    """Giant GNU torso, hindquarters, and legs."""
    # Subtle breathing
    breathe = wave(phase, 0.8) * 2.5 if anim not in ("death",) else 0
    by = body_y + breathe

    # ── Hindquarters ──
    hq = [
        (-140, by + 50),
        (-180, by + 30),
        (-200, by + 10),
        (-185, by - 20),
        (-140, by - 32),
        (-90, by - 28),
        (-55, by - 15),
        (-60, by + 40),
    ]
    c.polygon(hq, C_BODY_DARK, C_OUTLINE, 1.5)
    # Highlight
    hq_hi = [
        (-150, by + 10),
        (-175, by - 5),
        (-160, by - 22),
        (-115, by - 25),
        (-85, by - 20),
    ]
    c.polygon(hq_hi, C_BODY_MID)

    # ── Main torso / barrel ──
    torso = [
        (-60, by + 50),
        (-55, by - 15),
        (0, by - 38),
        (70, by - 32),
        (100, by - 10),
        (95, by + 45),
        (40, by + 58),
        (-20, by + 58),
    ]
    c.polygon(torso, C_BODY_MID, C_OUTLINE, 1.5)

    # Torso highlight
    hi = [(-10, by - 28), (50, by - 24), (72, by - 8), (60, by + 10), (10, by + 6)]
    c.polygon(hi, C_BODY_LIGHT)

    # ── Shoulder hump ──
    hump = [
        (-20, by - 38),
        (10, by - 58),
        (40, by - 62),
        (65, by - 50),
        (70, by - 32),
        (20, by - 35),
    ]
    c.polygon(hump, C_BODY_LIGHT, C_OUTLINE, 1.2)

    # ── Four legs (with walk-in-place shuffle) ──
    legs = [
        (-130, by + 50, -145, by + 100, -138, by + 115),  # 0: hind left
        (-70, by + 52, -80, by + 102, -72, by + 118),  # 1: hind right-ish
        (20, by + 55, 12, by + 108, 22, by + 122),  # 2: fore left
        (75, by + 48, 82, by + 100, 74, by + 115),  # 3: fore right
    ]
    for idx, (x0, y0, x1, y1, x2, y2) in enumerate(legs):
        lift = _leg_lift(idx, phase, anim)
        # Lift hits the knee and hoof — upper-leg attachment stays put so
        # the silhouette doesn't pop off the body.
        knee_lift = lift * 0.65
        hoof_lift = lift
        y1l = y1 - knee_lift
        y2l = y2 - hoof_lift
        # Upper leg
        c.polygon(
            [(x0 - 14, y0), (x0 + 10, y0), (x1 + 8, y1l), (x1 - 12, y1l)],
            C_BODY_DARK,
            C_OUTLINE,
            1.2,
        )
        # Lower leg (slightly lighter)
        c.polygon(
            [(x1 - 10, y1l), (x1 + 7, y1l), (x2 + 6, y2l), (x2 - 9, y2l)],
            C_BODY_MID,
            C_OUTLINE,
            1.2,
        )
        # Hoof
        c.ellipse(x2 - 2, y2l + 6, 12, 6, C_HOOF_DARK, C_OUTLINE, 1.0)

    # ── Shaggy mane along chest/neck base ──
    for i in range(8):
        mx = lerp(-50, 50, i / 7)
        my = by - 25 + wave(i * 1.3, 1.0) * 5
        c.polygon(
            [
                (mx - 8, my),
                (mx, my - 20 - i * 1.5),
                (mx + 8, my - 2),
            ],
            C_MANE_DARK if i % 2 == 0 else C_MANE_MID,
        )


def draw_gnu_neck(
    c: Canvas,
    head_y_offset: float = 0.0,
    tilt: float = 0.0,
    phase: float = 0.0,
    anim: str = "rest",
) -> None:
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


def draw_gnu_horns(
    c: Canvas,
    hx: float = 0.0,
    hy: float = 0.0,
    scale: float = 1.0,
    phase: float = 0.0,
    anim: str = "rest",
) -> None:
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
            (bx, by),
            (bx + sx * 40 * scale, by - 10 * scale + droop),
            (bx + sx * 80 * scale, by + 20 * scale + droop),
            (bx + sx * 88 * scale, by + 55 * scale + droop),
            (bx + sx * 70 * scale, by + 85 * scale + droop * 0.5),
            (bx + sx * 45 * scale, by + 90 * scale),
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


def draw_gnu_head(
    c: Canvas,
    hx: float = 0.0,
    hy: float = 0.0,
    phase: float = 0.0,
    anim: str = "rest",
    enraged: bool = False,
) -> None:
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
        (hx + 5 + sway, hy - 34),
        (hx - 28 + sway, hy - 30),
    ]
    c.polygon(skull, C_BODY_MID, C_OUTLINE, 2.0)

    # Skull highlight
    hi = [
        (hx - 20 + sway, hy - 22),
        (hx + 15 + sway, hy - 26),
        (hx + 32 + sway, hy - 12),
        (hx + 20 + sway, hy + 5),
        (hx - 5 + sway, hy + 2),
        (hx - 22 + sway, hy - 10),
    ]
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

    if enraged or anim in ("head_down", "hand_slam", "hand_sweep"):
        # Angry glow around eyes
        intensity = 0.7 + 0.3 * blink01(phase, 2.5)
        glow = (int(255 * intensity), int(160 * intensity), 20, 120)
        c.alpha_ellipse(ex, ey, 22, 18, glow)
        c.alpha_ellipse(ex2, ey2, 18, 14, glow)

    # ── Horns ──
    draw_gnu_horns(c, hx + sway, hy, 1.0, phase, anim)


def draw_hand(
    c: Canvas,
    cx: float,
    cy: float,
    side: int = 1,
    phase: float = 0.0,
    anim: str = "rest",
    slam_progress: float = 0.0,
    sweep_progress: float = 0.0,
) -> None:
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
        c.alpha_ellipse(cx, cy + hh, 40, 16, glow)

    # Wind trail during sweep
    if sweep_progress > 0.4 and anim == "hand_sweep":
        trail_alpha = int(80 * min(1.0, sweep_progress))
        trail = (120, 180, 255, trail_alpha)
        c.polygon(
            [
                (cx + side * 30, cy - 20),
                (cx + side * 80, cy - 10),
                (cx + side * 80, cy + 10),
                (cx + side * 30, cy + 20),
            ],
            trail,
        )


def draw_gnu_ton_man(
    c: Canvas,
    hx: float = 0.0,
    hy: float = 0.0,
    phase: float = 0.0,
    anim: str = "rest",
    show_speech: bool = False,
) -> None:
    """The GNU-ton scholar standing atop the GNU's neck/back.

    Isaac-Newton-coded: white powdered wig with side curls and a tied
    queue at the back, no glasses, triangular beard kept for silhouette,
    a robe with cinched belt, and one arm raised holding a scroll while
    gesticulating. Higher source resolution (768×576) means the wig
    curls still resolve cleanly at the in-game render scale.
    """
    # Body proportions referenced from `hy` (scholar's torso center). Head
    # at hy - 12, feet at hy + 16 → ~28 px tall.

    # ── Powdered wig (drawn FIRST so the face can sit on top) ──
    # Tied queue at the back — implied by a small ribbon-and-tail bump
    # just behind the right side curl. Drawn first so the side curls
    # overlap and visually anchor it.
    c.ellipse(hx + 7, hy - 4, 1.6, 2.4, C_MAN_WIG, C_OUTLINE, 0.6)
    c.line([(hx + 7, hy - 5.5), (hx + 7, hy - 2.5)], C_MAN_ROBE_D, 0.7)
    # Crown: shorter cap that sits ABOVE the face area. Center moved
    # up + half-height shrunk so the bottom of the crown (y = hy-18+5
    # = hy-13) clears the top of the face (y = hy-18). Earlier version
    # had crown bottom at hy-9.5, which buried the eyeline.
    c.ellipse(hx, hy - 21, 8.5, 5.5, C_MAN_WIG, C_OUTLINE, 1.0)
    # Side curls: a stack of three small ellipses cascading down each side
    # of the face. Pulled outward (x = ±9 instead of ±8) so they frame
    # the cheek instead of overlapping it.
    for dy in (-13, -10, -7):
        c.ellipse(hx - 9, hy + dy, 3.0, 2.6, C_MAN_WIG, C_OUTLINE, 0.7)
        c.ellipse(hx + 9, hy + dy, 3.0, 2.6, C_MAN_WIG, C_OUTLINE, 0.7)
    # Inner curl shading so the stack reads as 3D, not three flat dots.
    for dy in (-13, -10, -7):
        c.ellipse(hx - 9.3, hy + dy + 0.5, 1.3, 1.0, C_MAN_WIG_S)
        c.ellipse(hx + 9.3, hy + dy + 0.5, 1.3, 1.0, C_MAN_WIG_S)
    # Crown highlight that catches the (imagined) overhead light. Sits
    # on the wig, not on the forehead, so the face still reads clean.
    c.line([(hx - 4, hy - 23), (hx + 4, hy - 23)], C_MAN_WIG_S, 0.8)

    # ── Head (drawn AFTER the wig so the face wins the z-fight) ──
    # The face is intentionally last among the head-area primitives:
    # earlier the wig crown overlapped the upper half of the face and
    # the scholar read as "Harry Potter under a mop". Drawing skin on
    # top guarantees the face is always visible regardless of how the
    # wig silhouette grows later.
    c.ellipse(hx, hy - 12, 6, 6, C_MAN_SKIN, C_OUTLINE, 1.0)
    # Tiny face features so the face has something to be — without
    # them the skin disk reads as a featureless ball. Two small dark
    # dots for eyes; a faint mouth line. Kept dim so they don't fight
    # the wig at small scales.
    c.ellipse(hx - 1.8, hy - 12.5, 0.6, 0.7, C_OUTLINE)
    c.ellipse(hx + 1.8, hy - 12.5, 0.6, 0.7, C_OUTLINE)
    c.line([(hx - 1.2, hy - 9.5), (hx + 1.2, hy - 9.5)], C_OUTLINE, 0.6)

    # ── Triangular beard with shading ──
    beard_bob = wave(phase, 1.2) * 0.8
    c.polygon(
        [
            (hx - 4, hy - 8),
            (hx + 4, hy - 8),
            (hx + 2, hy - 1 + beard_bob),
            (hx, hy + 1 + beard_bob),
            (hx - 2, hy - 1 + beard_bob),
        ],
        C_MAN_BEARD,
        C_OUTLINE,
        0.8,
    )
    # Beard highlight stripe
    c.line([(hx - 1, hy - 6), (hx, hy - 2 + beard_bob)], C_MAN_SPEC, 0.8)

    # ── Robe ──
    robe = [
        (hx - 8, hy - 7),
        (hx + 8, hy - 7),
        (hx + 11, hy + 13),
        (hx + 6, hy + 17),
        (hx, hy + 18),
        (hx - 6, hy + 17),
        (hx - 11, hy + 13),
    ]
    c.polygon(robe, C_MAN_ROBE, C_OUTLINE, 1.0)
    # Robe lining (lighter)
    c.polygon(
        [
            (hx - 2, hy - 6),
            (hx + 3, hy - 6),
            (hx + 3, hy + 9),
            (hx - 2, hy + 9),
        ],
        C_MAN_ROBE_L,
    )
    # Cinched belt
    c.line([(hx - 8, hy + 2), (hx + 8, hy + 2)], C_MAN_ROBE_D, 1.2)
    c.ellipse(hx, hy + 2, 1.4, 1.0, C_HORN_TIP)
    # Robe fold shadows
    c.line([(hx - 5, hy + 4), (hx - 6, hy + 14)], C_MAN_ROBE_D, 0.9)
    c.line([(hx + 5, hy + 4), (hx + 6, hy + 14)], C_MAN_ROBE_D, 0.9)

    # ── Arms ──
    # Resting arm hangs at left side; right arm gestures or holds scroll.
    if anim == "rest":
        arm_phase = wave(phase, 1.8) * 4.0
        # Raised right arm with tiny scroll
        elbow = (hx + 10, hy - 4 + arm_phase * 0.4)
        hand = (hx + 16, hy - 9 + arm_phase)
        c.line([(hx + 7, hy - 4), elbow, hand], C_MAN_SKIN, 2.0)
        # Tiny scroll (parchment) in raised hand
        c.polygon(
            [
                (hand[0] - 1.6, hand[1] - 3.6),
                (hand[0] + 4.2, hand[1] - 3.6),
                (hand[0] + 4.2, hand[1] + 1.0),
                (hand[0] - 1.6, hand[1] + 1.0),
            ],
            C_SPEECH_BG,
            C_SPEECH_EDGE,
            0.8,
        )
        # Scroll text bar
        c.line(
            [(hand[0] - 0.5, hand[1] - 2), (hand[0] + 3.0, hand[1] - 2)],
            C_SPEECH_TXT,
            0.6,
        )
        # Left arm hangs along robe
        c.line([(hx - 7, hy - 4), (hx - 9, hy + 4)], C_MAN_ROBE_D, 1.8)
    elif anim == "death":
        settle = min(1.0, phase * 1.5)
        c.line(
            [
                (hx + 7, hy - 4),
                (hx + 14 + settle * 8, hy + 6 + settle * 16),
            ],
            C_MAN_SKIN,
            1.8,
        )
        c.line(
            [
                (hx - 7, hy - 4),
                (hx - 14 - settle * 6, hy + 4 + settle * 12),
            ],
            C_MAN_SKIN,
            1.8,
        )
    elif anim in ("hand_slam", "hand_sweep", "head_down"):
        # Pointing dramatically with right arm during attacks
        c.line(
            [
                (hx + 7, hy - 4),
                (hx + 13, hy - 8),
                (hx + 18, hy - 11),
            ],
            C_MAN_SKIN,
            2.0,
        )
        # Bracing left hand on robe
        c.line([(hx - 7, hy - 4), (hx - 11, hy + 2)], C_MAN_SKIN, 1.8)
    else:
        # Hit etc: both arms flailing
        flail = wave(phase, 4.0) * 4
        c.line([(hx + 7, hy - 4), (hx + 12 + flail, hy + 4)], C_MAN_SKIN, 1.8)
        c.line([(hx - 7, hy - 4), (hx - 12 - flail, hy + 4)], C_MAN_SKIN, 1.8)

    # Tiny sandaled feet
    c.ellipse(hx - 4, hy + 17, 3, 1.8, C_MAN_HAIR, C_OUTLINE, 0.6)
    c.ellipse(hx + 4, hy + 17, 3, 1.8, C_MAN_HAIR, C_OUTLINE, 0.6)

    # Speech bubble (kept for canonical / debug renders; in-game uses the
    # bark system instead). Smaller to match the smaller scholar.
    if show_speech:
        bx = hx + 14
        by = hy - 30
        bw, bh = 60, 24
        c.polygon(
            [
                (bx, by),
                (bx + bw, by),
                (bx + bw, by + bh),
                (bx + 6, by + bh),
                (bx + 2, by + bh + 8),
                (bx + 12, by + bh),
                (bx, by + bh),
            ],
            C_SPEECH_BG,
            C_SPEECH_EDGE,
            1.0,
        )
        lines = [
            (bx + 5, by + 6, bx + 44, by + 6),
            (bx + 5, by + 12, bx + 50, by + 12),
            (bx + 5, by + 18, bx + 34, by + 18),
        ]
        for x0, y0, x1, y1 in lines:
            c.line([(x0, y0), (x1, y1)], C_SPEECH_TXT, 1.2)


# ── Per-animation frame drawing ──────────────────────────────────────────────


def draw_frame(
    anim: str,
    frame_idx: int,
    frame_count: int,
    layer: str = "full",
    parts: Optional[dict] = None,
) -> Image.Image:
    """Render one animation frame and return a FRAME_SIZE RGBA image.

    `layer` selects which silhouette layers to draw:
      - "full"  : everything (used for canonical + debug sheet)
      - "body"  : body, neck, head, scholar — no hands, no attack VFX
      - "hands" : hands + attack VFX (shockwaves, vulnerability rings)
    `parts` collects design-space anchor positions for runtime hitboxes.
    Pass the same dict across all frames of a row to accumulate the row.
    """
    phase = frame_idx / max(1, frame_count)
    c = Canvas(FRAME_W, FRAME_H, C_BG, scale=SUPERSAMPLE)

    if anim == "rest":
        _draw_rest(c, phase, frame_idx, layer=layer, parts=parts)
    elif anim == "hand_slam":
        _draw_hand_slam(c, phase, frame_idx, layer=layer, parts=parts)
    elif anim == "hand_sweep":
        _draw_hand_sweep(c, phase, frame_idx, layer=layer, parts=parts)
    elif anim == "head_down":
        _draw_head_down(c, phase, frame_idx, layer=layer, parts=parts)
    elif anim == "hit":
        _draw_hit(c, phase, frame_idx, layer=layer, parts=parts)
    elif anim == "death":
        _draw_death(c, phase, frame_idx, layer=layer, parts=parts)
    else:
        _draw_rest(c, phase, frame_idx, layer=layer, parts=parts)

    return c.finish()


# Giant GNU layout in design-space (y=0 is frame center; positive = down).
# Frame is now 768×576 so we have ~96 extra design pixels of vertical room.
#
#   Body at +60 below center      (lower so the body fills the bottom)
#   Shoulder hump peak: body_y - 62 = -2  (just above frame center)
#   Head at -75 above center      (lowered ~20px so neck better connects)
#   Hands at +20 below center
#
# Scholar (GNU-ton) stands on the RIGHT shoulder of the giant (nudged
# further right to x≈+44 per direction). His center sits ~18 px above the
# hump peak (smaller silhouette than the older 22 px gap). He is NOT on
# the head — the head is the attack target that periodically descends.
REST_HEAD_Y = -75.0
REST_HAND_Y = 20.0
REST_BODY_Y = 60.0

# Shoulder contact point: body hump peak is at (body_y - 62).
# Scholar feet sit here; scholar center = foot_y - 18 (smaller figure now).
_SHOULDER_TOP_Y = REST_BODY_Y - 62  # ≈ -2
_MAN_CENTER_Y = _SHOULDER_TOP_Y - 18  # ≈ -20
_MAN_CENTER_X = 44.0  # nudged further onto the right shoulder

# Hand x-anchors. Slightly wider than the older 185 px so the wider 768
# frame still places hands near the edges.
REST_HAND_X = 235.0

# Hand-slam strike depth (design-space y). Below the leg hooves at
# REST_BODY_Y + 115 = 175 so the slam visually hits the floor. The
# Rust-side hardcoded `HandSlam` hitbox uses the same y so the
# collision overlays the drawn hand.
SLAM_STRIKE_Y = 195.0


def _draw_body_layer(
    c: Canvas,
    body_y: float,
    head_y: float,
    head_x: float,
    neck_offset: float,
    neck_tilt: float,
    man_x: float,
    man_y: float,
    anim: str,
    phase: float,
    enraged: bool,
    draw_man: bool = True,
) -> None:
    """Draw the static silhouette: body, neck, head, scholar.

    Excludes the hands and any attack VFX (shockwave, glow rings) — those
    live in the hands layer so they render in front of platforms.
    """
    draw_gnu_body(c, body_y=body_y, phase=phase, anim=anim)
    draw_gnu_neck(c, head_y_offset=neck_offset, tilt=neck_tilt, phase=phase, anim=anim)
    draw_gnu_head(c, head_x, head_y, phase=phase, anim=anim, enraged=enraged)
    if draw_man:
        draw_gnu_ton_man(c, man_x, man_y, phase=phase, anim=anim, show_speech=False)


def _draw_hands_layer(
    c: Canvas,
    lhx: float,
    lhy: float,
    rhx: float,
    rhy: float,
    anim: str,
    phase: float,
    slam_alpha: float = 0.0,
    sweep_prog: float = 0.0,
    head_y_for_glow: Optional[float] = None,
    vulnerability_alpha: float = 0.0,
    shockwave_radius: float = 0.0,
    shockwave_alpha: float = 0.0,
) -> None:
    """Draw the hands plus attack VFX (shockwaves, glow rings, sweep trails)."""
    draw_hand(
        c,
        lhx,
        lhy,
        side=-1,
        phase=phase,
        anim=anim,
        slam_progress=slam_alpha,
        sweep_progress=sweep_prog,
    )
    draw_hand(
        c,
        rhx,
        rhy,
        side=+1,
        phase=phase,
        anim=anim,
        slam_progress=slam_alpha,
        sweep_progress=sweep_prog,
    )
    # Vulnerability ring around the head, lives in the hands layer so it
    # renders in front of platforms (it's a readability signal).
    if vulnerability_alpha > 0.1 and head_y_for_glow is not None:
        ga = int(80 * vulnerability_alpha * (0.7 + 0.3 * blink01(phase, 3.0)))
        c.alpha_ellipse(
            0.0,
            head_y_for_glow,
            100 * vulnerability_alpha,
            80 * vulnerability_alpha,
            (255, 220, 60, ga),
        )
    # Hand-slam impact shockwave at floor level — same layer as hands.
    if shockwave_radius > 0 and shockwave_alpha > 0:
        ws = shockwave_alpha
        for r in [0.4, 0.7, 1.0]:
            a = int(100 * ws * (1 - r * 0.6))
            c.ellipse(0, 120, shockwave_radius * r, 22 * r * ws, (255, 200, 80, a))


def _record_parts(
    parts: Optional[dict],
    anim: str,
    frame_idx: int,
    head: Tuple[float, float],
    hand_left: Tuple[float, float],
    hand_right: Tuple[float, float],
    scholar: Tuple[float, float],
    extra: Optional[dict] = None,
) -> None:
    """Append one frame's design-space anchors into the parts dict.

    Consumed by the runtime to position hitboxes against the actually-drawn
    parts rather than guessing at fixed offsets.
    """
    if parts is None:
        return
    entry = {
        "frame": frame_idx,
        "head": [round(head[0], 2), round(head[1], 2)],
        "hand_left": [round(hand_left[0], 2), round(hand_left[1], 2)],
        "hand_right": [round(hand_right[0], 2), round(hand_right[1], 2)],
        "scholar": [round(scholar[0], 2), round(scholar[1], 2)],
    }
    if extra:
        entry.update(extra)
    parts.setdefault(anim, []).append(entry)


def _draw_rest(
    c: Canvas,
    phase: float,
    frame_idx: int,
    layer: str = "full",
    parts: Optional[dict] = None,
) -> None:
    """Idle: gentle sway, scholar muttering from the GNU's right shoulder."""
    bob = wave(phase, 0.9) * 3.5
    head_y = REST_HEAD_Y + bob * 0.6
    neck_offset = head_y + 70

    lhx = wave(phase, 0.6) * 5 - REST_HAND_X
    lhy = REST_HAND_Y + wave(phase, 0.85, 0.1) * 4
    rhx = wave(phase, 0.6, 0.25) * 5 + REST_HAND_X
    rhy = REST_HAND_Y + wave(phase, 0.85, 0.35) * 4

    man_y = _MAN_CENTER_Y + bob * 0.4
    man_x = _MAN_CENTER_X + wave(phase, 0.7, 0.2) * 1.5

    if layer in ("full", "body"):
        _draw_body_layer(
            c,
            REST_BODY_Y,
            head_y,
            0.0,
            neck_offset,
            0.0,
            man_x,
            man_y,
            anim="rest",
            phase=phase,
            enraged=False,
        )
    if layer in ("full", "hands"):
        _draw_hands_layer(c, lhx, lhy, rhx, rhy, anim="rest", phase=phase)

    _record_parts(
        parts,
        "rest",
        frame_idx,
        head=(0.0, head_y),
        hand_left=(lhx, lhy),
        hand_right=(rhx, rhy),
        scholar=(man_x, man_y),
    )


def _draw_hand_slam(
    c: Canvas,
    phase: float,
    frame_idx: int,
    layer: str = "full",
    parts: Optional[dict] = None,
) -> None:
    """Hands raise then slam down from above.

    Strike peak is design y = SLAM_STRIKE_Y (below the leg hooves) so the
    hands visually hit the floor. The Rust-side `gnu_ton_part_aabb`
    consumers use the same y, so collision boxes overlay where the hands
    actually appear.
    """
    if phase < 0.3:
        t = phase / 0.3
        slam_y = lerp(REST_HAND_Y, -110, smoothstep(t))
        slam_alpha = 0.0
    elif phase < 0.6:
        t = (phase - 0.3) / 0.3
        slam_y = lerp(-110, SLAM_STRIKE_Y, smoothstep(t) * 1.05)
        slam_alpha = smoothstep(t)
    else:
        t = (phase - 0.6) / 0.4
        slam_y = lerp(SLAM_STRIKE_Y, REST_HAND_Y, smoothstep(t))
        slam_alpha = lerp(1.0, 0.0, t)

    head_y = REST_HEAD_Y
    if layer in ("full", "body"):
        _draw_body_layer(
            c,
            REST_BODY_Y,
            head_y,
            0.0,
            head_y + 70,
            0.0,
            _MAN_CENTER_X,
            _MAN_CENTER_Y,
            anim="hand_slam",
            phase=phase,
            enraged=True,
        )

    shockwave_radius = 0.0
    shockwave_alpha = 0.0
    if slam_alpha > 0.5:
        shockwave_alpha = (slam_alpha - 0.5) * 2.0
        shockwave_radius = 260.0 * shockwave_alpha

    if layer in ("full", "hands"):
        _draw_hands_layer(
            c,
            -REST_HAND_X,
            slam_y,
            REST_HAND_X,
            slam_y,
            anim="hand_slam",
            phase=phase,
            slam_alpha=slam_alpha,
            shockwave_radius=shockwave_radius,
            shockwave_alpha=shockwave_alpha,
        )

    _record_parts(
        parts,
        "hand_slam",
        frame_idx,
        head=(0.0, head_y),
        hand_left=(-REST_HAND_X, slam_y),
        hand_right=(REST_HAND_X, slam_y),
        scholar=(_MAN_CENTER_X, _MAN_CENTER_Y),
        extra={"slam_alpha": round(slam_alpha, 3)},
    )


def _draw_hand_sweep(
    c: Canvas,
    phase: float,
    frame_idx: int,
    layer: str = "full",
    parts: Optional[dict] = None,
) -> None:
    """Hands sweep in from the far sides."""
    if phase < 0.2:
        t = phase / 0.2
        lhx = lerp(-REST_HAND_X, -290, smoothstep(t))
        rhx = lerp(REST_HAND_X, 290, smoothstep(t))
        sweep_prog = 0.0
    elif phase < 0.65:
        t = (phase - 0.2) / 0.45
        lhx = lerp(-290, -80, smoothstep(t))
        rhx = lerp(290, 80, smoothstep(t))
        sweep_prog = smoothstep(t)
    else:
        t = (phase - 0.65) / 0.35
        lhx = lerp(-80, -REST_HAND_X, smoothstep(t))
        rhx = lerp(80, REST_HAND_X, smoothstep(t))
        sweep_prog = lerp(1.0, 0.0, t)

    head_y = REST_HEAD_Y
    if layer in ("full", "body"):
        _draw_body_layer(
            c,
            REST_BODY_Y,
            head_y,
            0.0,
            head_y + 70,
            0.0,
            _MAN_CENTER_X,
            _MAN_CENTER_Y,
            anim="hand_sweep",
            phase=phase,
            enraged=True,
        )
    if layer in ("full", "hands"):
        _draw_hands_layer(
            c,
            lhx,
            REST_HAND_Y,
            rhx,
            REST_HAND_Y,
            anim="hand_sweep",
            phase=phase,
            sweep_prog=sweep_prog,
        )

    _record_parts(
        parts,
        "hand_sweep",
        frame_idx,
        head=(0.0, head_y),
        hand_left=(lhx, REST_HAND_Y),
        hand_right=(rhx, REST_HAND_Y),
        scholar=(_MAN_CENTER_X, _MAN_CENTER_Y),
        extra={"sweep_prog": round(sweep_prog, 3)},
    )


def _draw_head_down(
    c: Canvas,
    phase: float,
    frame_idx: int,
    layer: str = "full",
    parts: Optional[dict] = None,
) -> None:
    """Head descends dramatically — vulnerability window."""
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

    c_sway = wave(phase, 1.5) * 8
    lhx = -REST_HAND_X + c_sway
    rhx = REST_HAND_X - c_sway

    if layer in ("full", "body"):
        _draw_body_layer(
            c,
            REST_BODY_Y,
            head_y,
            0.0,
            head_y + 55,
            0.3,
            _MAN_CENTER_X,
            _MAN_CENTER_Y,
            anim="head_down",
            phase=phase,
            enraged=(enrage_scale > 0.5),
        )
    if layer in ("full", "hands"):
        _draw_hands_layer(
            c,
            lhx,
            REST_HAND_Y,
            rhx,
            REST_HAND_Y,
            anim="head_down",
            phase=phase,
            head_y_for_glow=head_y,
            vulnerability_alpha=enrage_scale,
        )

    _record_parts(
        parts,
        "head_down",
        frame_idx,
        head=(0.0, head_y),
        hand_left=(lhx, REST_HAND_Y),
        hand_right=(rhx, REST_HAND_Y),
        scholar=(_MAN_CENTER_X, _MAN_CENTER_Y),
        extra={"vulnerability": round(enrage_scale, 3)},
    )


def _draw_hit(
    c: Canvas,
    phase: float,
    frame_idx: int,
    layer: str = "full",
    parts: Optional[dict] = None,
) -> None:
    """Hit flash and brief recoil."""
    jolt = wave(phase, 2.0) * 8
    flash_alpha = int(150 * (1.0 - phase))

    body_y_hit = REST_BODY_Y + jolt * 0.3
    head_y = REST_HEAD_Y + jolt * 0.5
    lhx = -REST_HAND_X + jolt
    lhy = REST_HAND_Y - jolt * 0.5
    rhx = REST_HAND_X + jolt
    rhy = REST_HAND_Y + jolt * 0.3
    man_x = _MAN_CENTER_X + jolt * 0.4
    man_y = _MAN_CENTER_Y + jolt * 0.2

    if layer in ("full", "body"):
        _draw_body_layer(
            c,
            body_y_hit,
            head_y,
            jolt * 0.7,
            head_y + 70,
            0.0,
            man_x,
            man_y,
            anim="hit",
            phase=phase,
            enraged=False,
        )
    if layer in ("full", "hands"):
        _draw_hands_layer(c, lhx, lhy, rhx, rhy, anim="hit", phase=phase)

    # Hit-flash overlay only on the full sheet; the runtime applies its own
    # flash via the hit_flash timer so the body/hands sheets stay clean.
    if layer == "full" and flash_alpha > 0:
        flash_img = Image.new("RGBA", (c.sw, c.sh), (255, 140, 40, flash_alpha))
        c.img = Image.alpha_composite(c.img, flash_img)
        c.draw = ImageDraw.Draw(c.img)

    _record_parts(
        parts,
        "hit",
        frame_idx,
        head=(jolt * 0.7, head_y),
        hand_left=(lhx, lhy),
        hand_right=(rhx, rhy),
        scholar=(man_x, man_y),
    )


def _draw_death(
    c: Canvas,
    phase: float,
    frame_idx: int,
    layer: str = "full",
    parts: Optional[dict] = None,
) -> None:
    """Boss collapses: horns droop, body slumps, man tumbles off."""
    settle = min(1.0, phase * 1.2)

    head_y = lerp(REST_HEAD_Y, 60, smoothstep(settle * 1.1))
    body_y = lerp(REST_BODY_Y, 100.0, smoothstep(settle))
    man_y = _MAN_CENTER_Y + settle * 110
    man_x = _MAN_CENTER_X + settle * 60

    lhx = lerp(-REST_HAND_X, -REST_HAND_X - 20, settle)
    lhy = lerp(REST_HAND_Y, 110, smoothstep(settle))
    rhx = lerp(REST_HAND_X, REST_HAND_X + 25, settle)
    rhy = lerp(REST_HAND_Y, 105, smoothstep(settle))

    if layer in ("full", "body"):
        _draw_body_layer(
            c,
            body_y,
            head_y,
            0.0,
            head_y + 70,
            0.0,
            man_x,
            man_y,
            anim="death",
            phase=phase,
            enraged=False,
            draw_man=(settle < 0.9),
        )
    if layer in ("full", "hands"):
        _draw_hands_layer(c, lhx, lhy, rhx, rhy, anim="death", phase=phase)

    if layer == "full":
        grey_blend = settle * 0.7
        if grey_blend > 0:
            grey = Image.new("RGBA", (c.sw, c.sh), (100, 90, 80, int(grey_blend * 140)))
            c.img = Image.alpha_composite(c.img, grey)
            c.draw = ImageDraw.Draw(c.img)

    _record_parts(
        parts,
        "death",
        frame_idx,
        head=(0.0, head_y),
        hand_left=(lhx, lhy),
        hand_right=(rhx, rhy),
        scholar=(man_x, man_y),
        extra={"settle": round(settle, 3)},
    )


# ── Runtime RON manifest ────────────────────────────────────────────────────

_HEAD_BOX_W = 184
_HEAD_BOX_H = 148
_HAND_BOX_W = 156
_HAND_BOX_H = 120
# GNU-ton's damageable head hurtbox is intentionally a little tighter
# than the dangerous head hitbox. Keeping both boxes centered on the
# same anchor makes the debug preview readable (cyan inside red) and
# gives gameplay a small forgiving margin near the visual edge.
_HEAD_HURTBOX_SCALE = 0.90


def _pixel_rect_from_center(cx: float, cy: float, w: int, h: int) -> dict:
    return {
        "x": int(round(OX + cx - w * 0.5)),
        "y": int(round(OY + cy - h * 0.5)),
        "w": int(w),
        "h": int(h),
    }


def _part_rect(name: str, rect: dict) -> dict:
    out = {"name": name}
    out.update(rect)
    return out


def _head_rect(head_anchor) -> dict:
    _x, y = head_anchor
    return _part_rect(
        "head", _pixel_rect_from_center(0.0, float(y), _HEAD_BOX_W, _HEAD_BOX_H)
    )


def _scale_part_rect_centered(part: dict, scale: float) -> dict:
    """Scale a generated pixel rect around its current center."""
    out = dict(part)
    scale = clamp(float(scale), 0.0, 1.0)
    cx = float(part["x"]) + float(part["w"]) * 0.5
    cy = float(part["y"]) + float(part["h"]) * 0.5
    w = max(1, int(round(float(part["w"]) * scale)))
    h = max(1, int(round(float(part["h"]) * scale)))
    out["x"] = int(round(cx - w * 0.5))
    out["y"] = int(round(cy - h * 0.5))
    out["w"] = w
    out["h"] = h
    return out


def _head_hurt_rect(head_anchor) -> dict:
    return _scale_part_rect_centered(_head_rect(head_anchor), _HEAD_HURTBOX_SCALE)


def _ron_part(part: dict) -> str:
    return (
        f'(name: "{part["name"]}", x: {part["x"]}, y: {part["y"]}, '
        f"w: {part['w']}, h: {part['h']})"
    )


def _ron_part_list(parts: list[dict]) -> str:
    if not parts:
        return "[]"
    return "[" + ", ".join(_ron_part(p) for p in parts) + "]"


def _ron_box(box: dict) -> str:
    fields = [f"parts: {_ron_part_list(box.get('parts', []))}"]
    frames = box.get("frames")
    if frames:
        frame_items = [
            f"(parts: {_ron_part_list(frame_box.get('parts', []))})"
            for frame_box in frames
        ]
        fields.append("frames: [" + ", ".join(frame_items) + "]")
    return "(" + ", ".join(fields) + ")"


def _ron_anim_entry(entry: dict) -> str:
    fields = []
    frame_duration_secs = entry.get("frame_duration_secs")
    if frame_duration_secs is not None:
        fields.append(f"frame_duration_secs: Some({float(frame_duration_secs):.6g})")
    hurtbox = entry.get("hurtbox")
    hitbox = entry.get("hitbox")
    if hurtbox is not None:
        fields.append(f"hurtbox: Some({_ron_box(hurtbox)})")
    if hitbox is not None:
        fields.append(f"hitbox: Some({_ron_box(hitbox)})")
    return "(" + ", ".join(fields) + ")"


def _box(parts: list[dict], frames: list[dict] | None = None) -> dict:
    out = {"parts": parts}
    if frames:
        out["frames"] = frames
    return out


def _animation_duration_secs(anim: str) -> float:
    for name, _frame_count, duration_ms in ANIMATIONS:
        if name == anim:
            return float(duration_ms) / 1000.0
    raise KeyError(anim)


def _head_hurt_frames(parts_doc: dict, anim: str) -> list[dict]:
    return [
        {"parts": [_head_hurt_rect(frame["head"])]}
        for frame in parts_doc["anchors"].get(anim, [])
    ]


def _head_hit_frames(parts_doc: dict, anim: str) -> list[dict]:
    return [
        {"parts": [_head_rect(frame["head"])]}
        for frame in parts_doc["anchors"].get(anim, [])
    ]


def _hand_hit_frames(parts_doc: dict, anim: str) -> list[dict]:
    frames = []
    for frame in parts_doc["anchors"].get(anim, []):
        left_x, left_y = frame["hand_left"]
        right_x, right_y = frame["hand_right"]
        frames.append(
            {
                "parts": [
                    _part_rect(
                        "left_hand",
                        _pixel_rect_from_center(
                            float(left_x), float(left_y), _HAND_BOX_W, _HAND_BOX_H
                        ),
                    ),
                    _part_rect(
                        "right_hand",
                        _pixel_rect_from_center(
                            float(right_x), float(right_y), _HAND_BOX_W, _HAND_BOX_H
                        ),
                    ),
                ]
            }
        )
    return frames


def _first_part(frames: list[dict], fallback: dict) -> dict:
    for frame in frames:
        parts = frame.get("parts", [])
        if parts:
            return parts[0]
    return fallback


def _deepest_frame_parts(frames: list[dict], fallback: list[dict]) -> list[dict]:
    if not frames:
        return list(fallback)
    return list(
        max(
            frames,
            key=lambda frame: max(
                (part["y"] for part in frame.get("parts", [])), default=-10_000
            ),
        ).get("parts", fallback)
    )


def _widest_frame_parts(frames: list[dict], fallback: list[dict]) -> list[dict]:
    if not frames:
        return list(fallback)
    return list(
        max(
            frames,
            key=lambda frame: max(
                (
                    abs(part["x"] + part["w"] * 0.5 - OX)
                    for part in frame.get("parts", [])
                ),
                default=-1.0,
            ),
        ).get("parts", fallback)
    )


def _row_hurt_entry(parts_doc: dict, anim: str) -> dict:
    frames = _head_hurt_frames(parts_doc, anim)
    fallback = _head_hurt_rect((0.0, REST_HEAD_Y))
    entry = {
        "frame_duration_secs": _animation_duration_secs(anim),
        "hurtbox": _box([_first_part(frames, fallback)], frames),
    }
    return {k: v for k, v in entry.items() if v is not None}


def _row_head_hit_entry(parts_doc: dict, anim: str) -> dict:
    hurt_frames = _head_hurt_frames(parts_doc, anim)
    hit_frames = _head_hit_frames(parts_doc, anim)
    hurt_fallback = _head_hurt_rect((0.0, REST_HEAD_Y))
    hit_fallback = _head_rect((0.0, REST_HEAD_Y))
    hurt_parts = _deepest_frame_parts(hurt_frames, [hurt_fallback])
    hit_parts = _deepest_frame_parts(hit_frames, [hit_fallback])
    entry = {
        "frame_duration_secs": _animation_duration_secs(anim),
        "hurtbox": _box(hurt_parts, hurt_frames),
        "hitbox": _box(hit_parts, hit_frames),
    }
    return {k: v for k, v in entry.items() if v is not None}


def _gnu_ton_body_metrics(parts_doc: dict) -> dict:
    """Return the exact per-animation metrics emitted into the runtime RON.

    The hitbox debug preview also consumes this data, keeping the visual
    review path tied to the same rectangles the Rust runtime loads. Every
    moving gameplay box is derived from anchors recorded while drawing the
    frames; no gameplay-only coordinates should be invented here.
    """
    rest_hurt = _row_hurt_entry(parts_doc, "rest")
    hit_hurt = _row_hurt_entry(parts_doc, "hit")
    hand_slam_hurt = _row_hurt_entry(parts_doc, "hand_slam")
    hand_sweep_hurt = _row_hurt_entry(parts_doc, "hand_sweep")
    head_down_boxes = _row_head_hit_entry(parts_doc, "head_down")

    hand_slam_frames = _hand_hit_frames(parts_doc, "hand_slam")
    hand_sweep_frames = _hand_hit_frames(parts_doc, "hand_sweep")
    hand_slam_fallback = [
        _part_rect(
            "left_hand",
            _pixel_rect_from_center(
                -REST_HAND_X, SLAM_STRIKE_Y, _HAND_BOX_W, _HAND_BOX_H
            ),
        ),
        _part_rect(
            "right_hand",
            _pixel_rect_from_center(
                REST_HAND_X, SLAM_STRIKE_Y, _HAND_BOX_W, _HAND_BOX_H
            ),
        ),
    ]
    hand_sweep_fallback = [
        _part_rect(
            "left_hand",
            _pixel_rect_from_center(
                -REST_HAND_X, REST_HAND_Y, _HAND_BOX_W, _HAND_BOX_H
            ),
        ),
        _part_rect(
            "right_hand",
            _pixel_rect_from_center(REST_HAND_X, REST_HAND_Y, _HAND_BOX_W, _HAND_BOX_H),
        ),
    ]
    shockwave = _part_rect("shockwave", {"x": 84, "y": 465, "w": 600, "h": 36})

    gnu_hand_slam = dict(hand_slam_hurt)
    gnu_hand_slam["hitbox"] = _box(
        _deepest_frame_parts(hand_slam_frames, hand_slam_fallback),
        hand_slam_frames,
    )
    gnu_shockwave = dict(hand_slam_hurt)
    gnu_shockwave["hitbox"] = _box([shockwave])
    gnu_hand_sweep = dict(hand_sweep_hurt)
    gnu_hand_sweep["hitbox"] = _box(
        _widest_frame_parts(hand_sweep_frames, hand_sweep_fallback),
        hand_sweep_frames,
    )

    return {
        # Visual-row keys. The runtime's live animation sample uses these so
        # damageable hurtboxes follow the exact row/frame being rendered.
        "rest": rest_hurt,
        "hand_slam": hand_slam_hurt,
        "hand_sweep": hand_sweep_hurt,
        "head_down": head_down_boxes,
        "hit": hit_hurt,
        # Profile-specific keys. Dangerous attack hitboxes still use these so
        # multiple gameplay profiles can share one visual row without losing
        # their distinct damage volumes.
        "gnu_hand_slam": gnu_hand_slam,
        "gnu_shockwave": gnu_shockwave,
        "gnu_hand_sweep": gnu_hand_sweep,
        "gnu_head_descent": head_down_boxes,
    }


def _gnu_ton_body_metrics_ron(parts_doc: dict) -> str:
    animations = _gnu_ton_body_metrics(parts_doc)
    lines = ["        body_metrics: Some((", "            animations: {"]
    for name, body in animations.items():
        lines.append(f'                "{name}": {_ron_anim_entry(body)},')
    lines.extend(["            },", "        )),"])
    return "\n".join(lines)


def _row_ron(anim_name: str, row_index: int, frame_count: int, duration_ms: int) -> str:
    rects = []
    for frame_index in range(frame_count):
        rects.append(
            "                    ("
            f"x: {frame_index * FRAME_W}, y: {row_index * FRAME_H}, "
            f"w: {FRAME_W}, h: {FRAME_H}, anchors: {{}}"
            "),"
        )
    rect_body = "\n".join(rects)
    duration_secs = duration_ms / 1000.0
    return (
        "            (\n"
        f'                animation: "{anim_name}",\n'
        f"                row_index: {row_index},\n"
        f"                frame_count: {frame_count},\n"
        f"                duration_ms: {duration_ms},\n"
        f"                duration_secs: {duration_secs:.6g},\n"
        "                rects: [\n"
        f"{rect_body}\n"
        "                ],\n"
        "            ),"
    )


def _runtime_spritesheet_ron(manifest: dict, parts_doc: dict) -> str:
    metrics = _gnu_ton_body_metrics_ron(parts_doc)
    row_blocks = []
    for row in manifest["rows"]:
        row_blocks.append(
            _row_ron(row["name"], row["row"], row["frames"], row["duration_ms"])
        )
    rows = "\n".join(row_blocks)
    return (
        "[\n"
        "    (\n"
        f'        target: "{TARGET_NAME}",\n'
        f'        image: "{TARGET_NAME}_spritesheet.png",\n'
        "        label_width: 0,\n"
        f"        frame_width: {FRAME_W},\n"
        f"        frame_height: {FRAME_H},\n"
        f"{metrics}\n"
        "        rows: [\n"
        f"{rows}\n"
        "        ],\n"
        "        tuning: None,\n"
        "    ),\n"
        "]\n"
    )


# ── Sheet assembly ───────────────────────────────────────────────────────────

_LAYERS = ("full", "body", "hands")


def build_spritesheet(outdir: Path) -> List[Path]:
    """Render all animation frames and emit runtime sheets + RON metadata.

    Emits:
      - `<target>_full_spritesheet.png`   (every layer composited)
      - `<target>_body_spritesheet.png`   (everything except hands / VFX)
      - `<target>_hands_spritesheet.png`  (hands + attack VFX only)
      - `<target>_spritesheet.png`        (back-compat alias of the full sheet)
      - `<target>_spritesheet.ron`        (runtime frame + body_metrics data)
      - `<target>_actor.ron`              (actor catalog sidecar)
      - `<target>_hitboxes_debug.png`     (review-only overlay, generated only)
    """
    max_frames = max(frames for _, frames, _ in ANIMATIONS)
    rows = len(ANIMATIONS)
    sheet_w = max_frames * FRAME_W
    sheet_h = rows * FRAME_H

    sheets = {
        layer: Image.new("RGBA", (sheet_w, sheet_h), (0, 0, 0, 0)) for layer in _LAYERS
    }

    manifest = {
        "target": TARGET_NAME,
        "frame_size": [FRAME_W, FRAME_H],
        "rows": [],
        "layers": list(_LAYERS),
    }
    parts: dict = {}

    for row_idx, (anim_name, frame_count, duration_ms) in enumerate(ANIMATIONS):
        for f in range(frame_count):
            x = f * FRAME_W
            y = row_idx * FRAME_H
            # `parts` is populated on the full pass (one entry per frame).
            for layer in _LAYERS:
                pass_parts = parts if layer == "full" else None
                img = draw_frame(
                    anim_name, f, frame_count, layer=layer, parts=pass_parts
                )
                sheets[layer].paste(img, (x, y))
        manifest["rows"].append(
            {
                "name": anim_name,
                "row": row_idx,
                "frames": frame_count,
                "duration_ms": duration_ms,
            }
        )
        print(f"  [{row_idx + 1}/{rows}] {anim_name} ({frame_count} frames)")

    outputs: List[Path] = []
    for layer in _LAYERS:
        path = outdir / f"{TARGET_NAME}_{layer}_spritesheet.png"
        sheets[layer].save(str(path), "PNG")
        outputs.append(path)

    # Back-compat alias: keep `<target>_spritesheet.png` pointing at the
    # full sheet so existing consumers (catalog id `gnu_ton`) still
    # resolve until they migrate to the layered loader.
    alias_path = outdir / f"{TARGET_NAME}_spritesheet.png"
    sheets["full"].save(str(alias_path), "PNG")
    outputs.append(alias_path)

    parts_doc = {
        "target": TARGET_NAME,
        "frame_size": [FRAME_W, FRAME_H],
        "design_origin": [OX, OY],
        "coordinate_space": (
            "design-space origin at frame center; y-positive = down. "
            "Runtime collision_anchor places this origin at the entity transform."
        ),
        "anchors": parts,
    }

    ron_path = outdir / f"{TARGET_NAME}_spritesheet.ron"
    ron_path.write_text(_runtime_spritesheet_ron(manifest, parts_doc), encoding="utf8")
    outputs.append(ron_path)

    actor_path = _write_actor_contract(outdir, manifest, ron_path)
    outputs.append(actor_path)

    debug_path = build_hitbox_debug(outdir, sheets["full"], manifest, parts_doc)
    outputs.append(debug_path)

    return outputs


HURTBOX_OUTLINE = (0, 230, 255, 235)
HURTBOX_FILL = (0, 230, 255, 42)
HITBOX_OUTLINE = (255, 60, 60, 245)
HITBOX_FILL = (255, 60, 60, 62)
LABEL_FILL = (255, 255, 255, 230)

_ROW_METRIC_KEYS = {
    "rest": ("rest",),
    "hand_slam": ("hand_slam", "gnu_hand_slam", "gnu_shockwave"),
    "hand_sweep": ("hand_sweep", "gnu_hand_sweep"),
    "head_down": ("head_down", "gnu_head_descent"),
    "hit": ("hit",),
}


def _box_parts_for_frame(box: dict, frame_index: int) -> list[dict]:
    frames = box.get("frames")
    if frames:
        frame_box = frames[min(frame_index, len(frames) - 1)]
        return list(frame_box.get("parts", []))
    return list(box.get("parts", []))


def _draw_metric_parts(
    draw: ImageDraw.ImageDraw,
    frame_x: int,
    frame_y: int,
    parts: list[dict],
    *,
    is_hitbox: bool,
) -> None:
    outline = HITBOX_OUTLINE if is_hitbox else HURTBOX_OUTLINE
    fill = HITBOX_FILL if is_hitbox else HURTBOX_FILL
    prefix = "X" if is_hitbox else "H"
    for part in parts:
        x = int(part["x"])
        y = int(part["y"])
        w = int(part["w"])
        h = int(part["h"])
        if w <= 0 or h <= 0:
            continue
        rect = (frame_x + x, frame_y + y, frame_x + x + w - 1, frame_y + y + h - 1)
        draw.rectangle(rect, fill=fill, outline=outline, width=2)
        name = str(part.get("name", ""))
        if name:
            draw.text(
                (frame_x + x + 3, frame_y + y + 2), f"{prefix} {name}", fill=LABEL_FILL
            )


def build_hitbox_debug(
    outdir: Path, full_sheet: Image.Image, manifest: dict, parts_doc: dict
) -> Path:
    """Write a per-frame visual review of the exact RON hit/hurt boxes."""
    metrics = _gnu_ton_body_metrics(parts_doc)
    overlay = Image.new("RGBA", full_sheet.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay, "RGBA")
    for row in manifest["rows"]:
        anim = row["name"]
        row_index = int(row["row"])
        frame_count = int(row["frames"])
        metric_keys = _ROW_METRIC_KEYS.get(anim, ())
        if not metric_keys:
            continue
        for frame_index in range(frame_count):
            frame_x = frame_index * FRAME_W
            frame_y = row_index * FRAME_H
            for metric_key in metric_keys:
                metric = metrics.get(metric_key, {})
                hurtbox = metric.get("hurtbox")
                hitbox = metric.get("hitbox")
                # Draw the dangerous hitbox first, then the smaller damageable
                # hurtbox on top. This keeps the review image readable when
                # both boxes track the same moving GNU-ton head.
                if hitbox:
                    _draw_metric_parts(
                        draw,
                        frame_x,
                        frame_y,
                        _box_parts_for_frame(hitbox, frame_index),
                        is_hitbox=True,
                    )
                if hurtbox:
                    _draw_metric_parts(
                        draw,
                        frame_x,
                        frame_y,
                        _box_parts_for_frame(hurtbox, frame_index),
                        is_hitbox=False,
                    )
            draw.text(
                (frame_x + 6, frame_y + 6), f"{anim} #{frame_index}", fill=LABEL_FILL
            )
    debug = Image.alpha_composite(full_sheet.convert("RGBA"), overlay)
    path = outdir / HITBOX_DEBUG_FILE
    debug.save(path, "PNG")
    return path


def _write_actor_contract(outdir: Path, manifest: dict, ron_path: Path) -> Path:
    from ambition_sprite2d_renderer.authoring.actor_contract import (
        write_actor_contract_for_tackon,
    )

    return write_actor_contract_for_tackon(
        target=TARGET_NAME,
        image_out=outdir / f"{TARGET_NAME}_spritesheet.png",
        sheet_ron_out=ron_path,
        manifest=manifest,
        actor_metadata=ACTOR_METADATA,
    )


def build_canonical(outdir: Path) -> Path:
    """Render a single canonical reference pose (full layer)."""
    img = draw_frame("rest", 0, ANIMATIONS[0][1], layer="full")
    path = outdir / f"{TARGET_NAME}_canonical.png"
    big = img.resize((img.width * 2, img.height * 2), Image.LANCZOS)
    big.save(str(path), "PNG")
    return path


def build_preview_labeled(outdir: Path) -> Path:
    """Render a labeled preview strip showing all animations (full layer)."""
    frames_per_row = [frame_count for _, frame_count, _ in ANIMATIONS]
    preview_h = len(ANIMATIONS) * (FRAME_H // 2 + 18)
    preview_w = max(frames_per_row) * (FRAME_W // 2)
    preview = Image.new("RGBA", (preview_w, preview_h), (30, 22, 15, 255))

    for row_idx, (anim_name, frame_count, _) in enumerate(ANIMATIONS):
        y_base = row_idx * (FRAME_H // 2 + 18)
        for f in range(frame_count):
            img = draw_frame(anim_name, f, frame_count, layer="full")
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
    print("  spritesheets (full / body / hands)...")
    paths.extend(build_spritesheet(outdir))

    print("  canonical...")
    paths.append(build_canonical(outdir))

    if not quick:
        print("  preview...")
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
        try:
            display = dst.relative_to(TOOL_ROOT)
        except ValueError:
            display = dst
        print(f"  installed: {display}")
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
