#!/usr/bin/env python3
"""
Panel-based vanity card animation.

Loads the six pre-keyed PNGs from assets/vanity_card/panels/ (all frames have
both characters), drives a beat-based animation, and writes an animated GIF
before opening the pygame preview window.

Outputs
-------
  assets/vanity_card/vanity_card.gif   — looping animated GIF (always written)
  pygame window                        — interactive preview (requires display)

Beat types (panel_animation.beats in config.yaml)
--------------------------------------------------
  hold  — freeze on frame N, fade in speech bubble, wait duration seconds
  play  — step through frames[] list at fps (no bubble)

Keys:  Space / Right = skip beat   R = restart   Escape = quit

Run:  python3 frame_demo.py
"""

import os
import sys
import numpy as np
from PIL import Image, ImageDraw, ImageFont
import pygame

from utils import (
    load_config, out_path,
    chroma_key as do_chroma_key, cleanup_green_residue, src_path,
)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

# ── Font helpers ──────────────────────────────────────────────────────────────

_FONT_CANDIDATES = [
    "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/truetype/freefont/FreeSansBold.ttf",
    "/usr/share/fonts/truetype/ubuntu/Ubuntu-B.ttf",
]


def _find_font_path() -> str | None:
    for p in _FONT_CANDIDATES:
        if os.path.exists(p):
            return p
    return None


def load_pil_font(size: int) -> ImageFont.FreeTypeFont:
    p = _find_font_path()
    return ImageFont.truetype(p, size) if p else ImageFont.load_default()


def load_pygame_font(size: int) -> pygame.font.Font:
    p = _find_font_path()
    return pygame.font.Font(p, size) if p else pygame.font.Font(None, size)


# ── Frame loading ─────────────────────────────────────────────────────────────

def load_panel_frames(cfg: dict) -> list:
    """Load PNG files from panels/ in the sequence defined by panel_animation."""
    pa = cfg["panel_animation"]
    panels_dir = out_path(cfg, "panels")
    frames = []
    for fname in pa["sequence"]:
        path = os.path.join(panels_dir, fname)
        if not os.path.exists(path):
            print(f"  WARNING: panel not found: {path}")
            continue
        img = Image.open(path).convert("RGBA")
        frames.append(img)
    print(f"  loaded {len(frames)} panels")
    return frames


def load_spritesheet_frames(cfg: dict) -> list:
    """Fallback: chroma-key the greenscreen spritesheet and slice into frames."""
    ss = cfg["spritesheet"]
    path = src_path(cfg, ss["file"])
    raw = Image.open(path).convert("RGBA")
    ck = cfg.get("chroma_key", {})
    keyed = do_chroma_key(
        raw,
        inner=ck.get("inner_radius", 25.0),
        outer=ck.get("outer_radius", 70.0),
        spill=ck.get("spill_reduction", True),
    )
    keyed = cleanup_green_residue(keyed)

    col_spans = [tuple(s) for s in ss["col_spans"]]
    row_spans = [tuple(s) for s in ss["row_spans"]]
    margin = 6
    frames = []
    for ry0, ry1 in row_spans:
        for cx0, cx1 in col_spans:
            cell = keyed.crop((cx0, ry0, cx1, ry1))
            bbox = cell.getbbox()
            if bbox:
                l, t, r, b = bbox
                cell = cell.crop((max(0, l - margin), max(0, t - margin),
                                   min(cell.width, r + margin),
                                   min(cell.height, b + margin)))
            frames.append(cell)
    print(f"  loaded {len(frames)} frames from spritesheet")
    return frames


# ── Shared panel-rect helper ──────────────────────────────────────────────────

def make_panel_rect(W: int, H: int, sample: Image.Image) -> tuple:
    """Compute (px, py, pw, ph) for the panel area, centred in WxH."""
    ph = int(H * 0.87)
    fw, fh = sample.size
    pw = int(ph * fw / max(fh, 1))
    if pw > int(W * 0.92):
        pw = int(W * 0.92)
        ph = int(pw * fh / max(fw, 1))
    return ((W - pw) // 2, (H - ph) // 2, pw, ph)


# ── PIL speech bubble ─────────────────────────────────────────────────────────

def draw_bubble_pil(draw: ImageDraw.ImageDraw,
                    text: str, font: ImageFont.FreeTypeFont,
                    panel_rect: tuple, side: str,
                    bubble_alpha: int, cfg: dict) -> None:
    if not text or bubble_alpha <= 0:
        return
    ac  = cfg["animation"]
    pad = ac["bubble_padding"]
    bw  = ac["bubble_border_width"]
    bg  = tuple(ac["bubble_bg"])
    border = tuple(ac["bubble_border"])
    tail_h = ac["bubble_tail_length"]

    px, py, pw, ph = panel_rect
    lines = text.split("\n")
    boxes = [draw.textbbox((0, 0), ln, font=font) for ln in lines]
    line_h  = max(b[3] - b[1] for b in boxes) if boxes else 20
    text_w  = max(b[2] - b[0] for b in boxes) if boxes else 60
    text_h  = len(lines) * line_h + max(0, len(lines) - 1) * 4

    bub_w = text_w + pad * 2
    bub_h = text_h + pad * 2

    if side == "left":
        bub_x = px + pw // 8
        tail_cx = bub_x + bub_w // 3
    else:
        bub_x = px + pw - bub_w - pw // 8
        tail_cx = bub_x + bub_w * 2 // 3
    bub_y = py + int(ph * 0.06)

    # Blend bubble colour with background at bubble_alpha
    def blend(c, a):
        # c is bubble colour, a is 0-255; used for soft appearance in PIL
        return c  # PIL doesn't do per-draw alpha; we handle at composite step

    draw.rounded_rectangle(
        [bub_x, bub_y, bub_x + bub_w, bub_y + bub_h],
        radius=14, fill=bg, outline=border, width=bw,
    )
    tail_base_y = bub_y + bub_h - 1
    tail_pts = [
        (tail_cx - 12, tail_base_y),
        (tail_cx + 12, tail_base_y),
        (tail_cx,      tail_base_y + tail_h),
    ]
    draw.polygon(tail_pts, fill=bg)
    draw.rectangle(
        [bub_x + bw, bub_y + bub_h - bw * 2, bub_x + bub_w - bw, bub_y + bub_h],
        fill=bg,
    )
    ty = bub_y + pad
    for i, line in enumerate(lines):
        lb = draw.textbbox((0, 0), line, font=font)
        lw = lb[2] - lb[0]
        draw.text((bub_x + pad + (text_w - lw) // 2, ty), line,
                  fill=border, font=font)
        ty += line_h + 4


# ── PIL frame renderer (used by GIF export) ───────────────────────────────────

def render_frame_pil(frame_img: Image.Image,
                     bubble_text: str | None, bubble_side: str, bubble_alpha: int,
                     panel_rect: tuple, cfg: dict, W: int, H: int,
                     pil_font: ImageFont.FreeTypeFont) -> Image.Image:
    ac = cfg["animation"]
    bg_col   = tuple(ac["background_color"])
    panel_bg = tuple(ac["panel_bg_color"])
    border   = tuple(ac["panel_border_color"])
    bdw      = ac["panel_border_width"]
    shad     = ac["shadow_offset"]
    shad_col = tuple(ac["shadow_color"][:3])

    out  = Image.new("RGB", (W, H), bg_col)
    draw = ImageDraw.Draw(out)
    px, py, pw, ph = panel_rect

    draw.rectangle(
        [px - bdw + shad, py - bdw + shad,
         px + pw + bdw + shad, py + ph + bdw + shad],
        fill=shad_col,
    )
    draw.rectangle([px - bdw, py - bdw, px + pw + bdw, py + ph + bdw], fill=border)
    draw.rectangle([px, py, px + pw, py + ph], fill=panel_bg)

    iw, ih = frame_img.size
    scale = min(pw / max(iw, 1), ph / max(ih, 1))
    nw, nh = max(1, int(iw * scale)), max(1, int(ih * scale))
    scaled = frame_img.resize((nw, nh), Image.LANCZOS)
    out.paste(scaled, (px + (pw - nw) // 2, py + (ph - nh) // 2), scaled)

    if bubble_text and bubble_alpha > 0:
        draw_bubble_pil(draw, bubble_text, pil_font, panel_rect,
                        bubble_side, bubble_alpha, cfg)
    return out


# ── GIF export ────────────────────────────────────────────────────────────────

def export_gif(frames_pil: list, beats: list, cfg: dict,
               gif_path: str, W: int = 640, H: int = 360,
               fps: int = 12) -> None:
    """Simulate the animation timeline and write an animated GIF."""
    ac = cfg["animation"]
    pil_font = load_pil_font(ac["bubble_font_size"])
    panel_rect = make_panel_rect(W, H, frames_pil[0])
    dt = 1.0 / fps
    ms_per_frame = int(1000 / fps)

    gif_frames: list[Image.Image] = []

    beat_idx = 0
    while beat_idx < len(beats):
        beat = beats[beat_idx]

        if beat["type"] == "hold":
            duration    = beat["duration"]
            bubble_text = beat.get("speech_bubble")
            bubble_side = beat.get("bubble_side", "right")
            bubble_delay = ac["bubble_delay"]
            frame_img   = frames_pil[beat["frame"]]
            t = 0.0
            while t < duration:
                alpha = 255 if bubble_text and t >= bubble_delay else 0
                gif_frames.append(render_frame_pil(
                    frame_img, bubble_text, bubble_side, alpha,
                    panel_rect, cfg, W, H, pil_font,
                ))
                t += dt

        elif beat["type"] == "play":
            seq       = beat.get("frames", [])
            beat_fps  = beat.get("fps", 8)
            frame_dur = 1.0 / max(beat_fps, 0.01)
            total_dur = len(seq) * frame_dur
            t = 0.0
            while t < total_dur:
                idx = min(int(t / frame_dur), len(seq) - 1)
                gif_frames.append(render_frame_pil(
                    frames_pil[seq[idx]], None, "right", 0,
                    panel_rect, cfg, W, H, pil_font,
                ))
                t += dt

        beat_idx += 1

    if not gif_frames:
        print("  GIF: no frames to write")
        return

    os.makedirs(os.path.dirname(gif_path), exist_ok=True)
    gif_frames[0].save(
        gif_path,
        save_all=True,
        append_images=gif_frames[1:],
        optimize=True,
        loop=0,
        duration=ms_per_frame,
    )
    rel = os.path.relpath(gif_path)
    print(f"  GIF ({len(gif_frames)} frames @ {fps}fps) → {rel}")


# ── pygame helpers ────────────────────────────────────────────────────────────

def pil_to_surf(img: Image.Image) -> pygame.Surface:
    return pygame.image.fromstring(img.tobytes(), img.size, "RGBA").convert_alpha()


def draw_panel_frame_pg(screen: pygame.Surface,
                         frame_surf: pygame.Surface,
                         panel_rect: tuple, cfg: dict) -> None:
    ac  = cfg["animation"]
    bdw = ac["panel_border_width"]
    shd = ac["shadow_offset"]
    px, py, pw, ph = panel_rect

    pygame.draw.rect(screen, tuple(ac["shadow_color"][:3]),
                     (px - bdw + shd, py - bdw + shd,
                      pw + bdw * 2, ph + bdw * 2))
    pygame.draw.rect(screen, tuple(ac["panel_border_color"]),
                     (px - bdw, py - bdw, pw + bdw * 2, ph + bdw * 2))
    pygame.draw.rect(screen, tuple(ac["panel_bg_color"]),
                     (px, py, pw, ph))

    iw, ih = frame_surf.get_size()
    scale = min(pw / max(iw, 1), ph / max(ih, 1))
    nw, nh = max(1, int(iw * scale)), max(1, int(ih * scale))
    scaled = pygame.transform.smoothscale(frame_surf, (nw, nh))
    screen.blit(scaled, (px + (pw - nw) // 2, py + (ph - nh) // 2))


def draw_bubble_pg(screen: pygame.Surface,
                   text: str, font: pygame.font.Font,
                   panel_rect: tuple, side: str,
                   alpha: int, cfg: dict) -> None:
    if not text or alpha <= 0:
        return
    ac  = cfg["animation"]
    pad = ac["bubble_padding"]
    bw  = ac["bubble_border_width"]
    bg  = tuple(ac["bubble_bg"])
    border = tuple(ac["bubble_border"])
    tail_h = ac["bubble_tail_length"]

    px, py, pw, ph = panel_rect
    lines = text.split("\n")
    surfs = [font.render(ln, True, border) for ln in lines]
    text_w = max(s.get_width()  for s in surfs) if surfs else 60
    line_h = max(s.get_height() for s in surfs) if surfs else 20

    bub_w = text_w + pad * 2
    bub_h = len(lines) * line_h + max(0, len(lines) - 1) * 4 + pad * 2

    if side == "left":
        bub_x = pw // 8
        tail_cx = bub_x + bub_w // 3
    else:
        bub_x = pw - bub_w - pw // 8
        tail_cx = bub_x + bub_w * 2 // 3
    bub_y = int(ph * 0.06)

    buf = pygame.Surface((pw, ph), pygame.SRCALPHA)
    body = pygame.Rect(bub_x, bub_y, bub_w, bub_h)
    pygame.draw.rect(buf, (*bg, 255),     body, border_radius=14)
    pygame.draw.rect(buf, (*border, 255), body, width=bw, border_radius=14)
    tail_base = bub_y + bub_h - 1
    pygame.draw.polygon(buf, (*bg, 255), [
        (tail_cx - 12, tail_base),
        (tail_cx + 12, tail_base),
        (tail_cx,      tail_base + tail_h),
    ])
    pygame.draw.rect(buf, (*bg, 255),
                     (bub_x + bw, bub_y + bub_h - bw * 2,
                      bub_w - bw * 2, bw * 3))
    ty = bub_y + pad
    for s in surfs:
        buf.blit(s, (bub_x + pad + (text_w - s.get_width()) // 2, ty))
        ty += line_h + 4

    buf.set_alpha(alpha)
    screen.blit(buf, (px, py))


# ── Beat state machine ────────────────────────────────────────────────────────

class BeatAnimator:
    def __init__(self, beats: list, frames: list, cfg: dict):
        self.beats  = beats
        self.frames = frames
        self.cfg    = cfg
        self._reset()

    def _reset(self):
        self.beat_idx       = 0
        self.elapsed        = 0.0
        self.play_frame_pos = 0
        self.bubble_alpha   = 0
        self.done           = False

    def restart(self):
        self._reset()

    def _beat(self) -> dict:
        return self.beats[self.beat_idx]

    def _next(self):
        self.beat_idx      += 1
        self.elapsed        = 0.0
        self.play_frame_pos = 0
        self.bubble_alpha   = 0
        if self.beat_idx >= len(self.beats):
            self.done = True

    def skip(self):
        self._next()

    def update(self, dt: float):
        if self.done:
            return
        self.elapsed += dt
        beat = self._beat()

        if beat["type"] == "hold":
            delay    = self.cfg["animation"]["bubble_delay"]
            fade_dur = self.cfg["animation"]["bubble_fade"]
            if self.elapsed > delay:
                t = min(1.0, (self.elapsed - delay) / max(fade_dur, 0.01))
                self.bubble_alpha = int(255 * t)
            if self.elapsed >= beat["duration"]:
                self._next()

        elif beat["type"] == "play":
            fps   = beat.get("fps", 8)
            seq   = beat.get("frames", [])
            fdur  = 1.0 / max(fps, 0.01)
            self.play_frame_pos = min(int(self.elapsed / fdur), len(seq) - 1)
            if self.elapsed >= len(seq) * fdur:
                self._next()

    def current_frame(self) -> pygame.Surface:
        beat = self._beat()
        if beat["type"] == "hold":
            return self.frames[beat["frame"]]
        seq = beat.get("frames", [0])
        return self.frames[seq[min(self.play_frame_pos, len(seq) - 1)]]

    def current_bubble(self) -> tuple:
        beat = self._beat()
        if beat["type"] == "hold":
            return (beat.get("speech_bubble"), beat.get("bubble_side", "right"),
                    self.bubble_alpha)
        return (None, "right", 0)


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    cfg  = load_config()
    demo = cfg["demo"]
    W, H = demo["width"], demo["height"]

    # ── Load frames & beats ───────────────────────────────────────────────────
    if "panel_animation" in cfg:
        print("Loading panel frames ...")
        frames_pil = load_panel_frames(cfg)
        beats = cfg["panel_animation"]["beats"]
    elif "spritesheet" in cfg:
        print("Loading spritesheet frames ...")
        frames_pil = load_spritesheet_frames(cfg)
        beats = cfg.get("spritesheet_beats", [])
    else:
        print("ERROR: no panel_animation or spritesheet in config.yaml", file=sys.stderr)
        sys.exit(1)

    if not beats:
        print("ERROR: no beats defined in config.yaml", file=sys.stderr)
        sys.exit(1)

    # ── Export GIF ────────────────────────────────────────────────────────────
    import sys as _sys
    gif_path = out_path(cfg, "vanity_card.gif")
    print("Exporting GIF ...")
    export_gif(frames_pil, beats, cfg, gif_path)

    # ── pygame setup ──────────────────────────────────────────────────────────
    pygame.init()
    flags  = pygame.FULLSCREEN if demo.get("fullscreen") else 0
    screen = pygame.display.set_mode((W, H), flags)
    pygame.display.set_caption("Vanity Card  —  Space/Right=skip  R=restart  Esc=quit")
    clock  = pygame.time.Clock()

    frames_surf = [pil_to_surf(f) for f in frames_pil]
    pg_font     = load_pygame_font(cfg["animation"]["bubble_font_size"])
    panel_rect  = make_panel_rect(W, H, frames_pil[0])
    bg_color    = tuple(cfg["animation"]["background_color"])
    loop        = demo.get("loop", True)

    animator = BeatAnimator(beats, frames_surf, cfg)

    # ── Event loop ────────────────────────────────────────────────────────────
    running = True
    while running:
        dt = clock.tick(demo["fps"]) / 1000.0

        for event in pygame.event.get():
            if event.type == pygame.QUIT:
                running = False
            elif event.type == pygame.KEYDOWN:
                if event.key == pygame.K_ESCAPE:
                    running = False
                elif event.key in (pygame.K_SPACE, pygame.K_RIGHT):
                    animator.skip()
                elif event.key == pygame.K_r:
                    animator.restart()

        if not animator.done:
            animator.update(dt)

        if animator.done:
            if loop:
                animator.restart()
            else:
                running = False
                continue

        screen.fill(bg_color)
        draw_panel_frame_pg(screen, animator.current_frame(), panel_rect, cfg)
        text, side, alpha = animator.current_bubble()
        if text and alpha > 0:
            draw_bubble_pg(screen, text, pg_font, panel_rect, side, alpha, cfg)
        pygame.display.flip()

    pygame.quit()


if __name__ == "__main__":
    main()
