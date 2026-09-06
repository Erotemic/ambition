#!/usr/bin/env python3
"""
Stage 3 — Vanity card animation demo.

Reads config.yaml for all timing, colour, and layout settings.
Reads assets/vanity_card/final/panel_{1-4}.png for panel images.

Keys:
  Space / Right   advance to next panel immediately
  R               restart from the beginning
  Escape / Q      quit

Run:  python3 demo.py
"""

import os
import sys
import math
import pygame

from utils import load_config, out_path

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(SCRIPT_DIR))


# ── Helpers ───────────────────────────────────────────────────────────────────


def find_font(preferred: list, size: int) -> pygame.font.Font:
    available = set(pygame.font.get_fonts())
    for name in preferred:
        key = name.lower().replace(" ", "")
        if key in available:
            return pygame.font.SysFont(name, size, bold=True)
    return pygame.font.SysFont(None, size, bold=True)


def lerp(a, b, t):
    return a + (b - a) * max(0.0, min(1.0, t))


def ease_out(t):
    return 1.0 - (1.0 - t) ** 2


# ── Speech bubble drawing ─────────────────────────────────────────────────────


def draw_bubble(
    surface: pygame.Surface,
    text: str,
    font: pygame.font.Font,
    panel_rect: pygame.Rect,
    side: str,
    alpha: int,
    cfg: dict,
) -> None:
    """Render a speech bubble overlaid on panel_rect, faded to *alpha* (0-255)."""
    if alpha <= 0:
        return

    ac = cfg["animation"]
    pad = ac["bubble_padding"]
    bw = ac["bubble_border_width"]
    tail_l = ac["bubble_tail_length"]
    bg = tuple(ac["bubble_bg"])
    border = tuple(ac["bubble_border"])

    lines = text.split("\n")
    rendered = [font.render(ln, True, border) for ln in lines]
    text_w = max(r.get_width() for r in rendered)
    line_h = rendered[0].get_height() if rendered else 20
    text_h = len(rendered) * line_h + max(0, len(rendered) - 1) * 4

    bub_w = text_w + pad * 2
    bub_h = text_h + pad * 2

    # Position: upper quadrant of the panel, left or right half
    pr = panel_rect
    if side == "right":
        bub_x = pr.x + pr.width // 2 + 10
        tail_ox = bub_x + bub_w // 4  # tail exits from left-centre-bottom
        tail_oy = pr.y + int(pr.height * 0.55)  # aim toward character mid-body
    else:
        bub_x = pr.x + pr.width // 4 - bub_w // 2
        tail_ox = bub_x + bub_w * 3 // 4
        tail_oy = pr.y + int(pr.height * 0.55)

    bub_y = max(pr.y + 12, tail_oy - tail_l - bub_h)

    # Build on a temporary surface for alpha blending
    buf_w = max(bub_w + abs(tail_ox - bub_x) + 20, bub_w + 20)
    buf_h = bub_h + tail_l + 10
    buf = pygame.Surface((buf_w + bub_w, buf_h + bub_h), pygame.SRCALPHA)
    # offsets so everything fits in buf
    ox = 10
    oy = 10

    # Tail (triangle pointing down to character)
    tx0 = tail_ox - bub_x + ox
    ty0 = bub_h + oy
    tw = 24
    pts = [
        (ox + bub_w // 2 - tw // 2, bub_h + oy - 2),
        (ox + bub_w // 2 + tw // 2, bub_h + oy - 2),
        (tx0, bub_h + tail_l + oy),
    ]
    pygame.draw.polygon(buf, bg + (255,), pts)

    # Bubble body (rounded rect)
    body = pygame.Rect(ox, oy, bub_w, bub_h)
    pygame.draw.rect(buf, bg + (255,), body, border_radius=14)
    pygame.draw.rect(buf, border + (255,), body, bw, border_radius=14)
    # Cover the gap between tail and body
    pygame.draw.rect(
        buf,
        bg + (255,),
        (ox + bub_w // 2 - tw // 2 + bw, bub_h + oy - bw - 1, tw - 2 * bw + 2, bw + 2),
    )

    # Text
    ty = oy + pad
    for r in rendered:
        buf.blit(r, (ox + pad + (text_w - r.get_width()) // 2, ty))
        ty += line_h + 4

    # Apply alpha and blit to main surface
    buf.set_alpha(alpha)
    surface.blit(buf, (bub_x - ox, bub_y - oy))


# ── Panel display ─────────────────────────────────────────────────────────────


def scale_panel(img: pygame.Surface, display_rect: pygame.Rect) -> tuple:
    """Scale img to fill display_rect preserving aspect, return (surface, dest_rect)."""
    iw, ih = img.get_size()
    dw, dh = display_rect.size
    scale = min(dw / iw, dh / ih)
    nw, nh = int(iw * scale), int(ih * scale)
    scaled = pygame.transform.smoothscale(img, (nw, nh))
    dest = pygame.Rect(
        display_rect.x + (dw - nw) // 2,
        display_rect.y + (dh - nh) // 2,
        nw,
        nh,
    )
    return scaled, dest


# ── Animation state machine ───────────────────────────────────────────────────

STATES = ["fade_in", "hold", "bubble_fade", "hold_bubble", "fade_out"]


class VanityCardDemo:
    def __init__(self, cfg: dict):
        self.cfg = cfg
        self.acfg = cfg["animation"]
        self.dcfg = cfg["demo"]

        pygame.init()
        pygame.display.set_caption("I Made This")
        flags = pygame.FULLSCREEN if self.dcfg["fullscreen"] else 0
        self.screen = pygame.display.set_mode(
            (self.dcfg["width"], self.dcfg["height"]), flags
        )
        self.clock = pygame.time.Clock()

        self.panels = self._load_panels()
        if not self.panels:
            print("ERROR: No panels found in assets/vanity_card/final/")
            print("       Run python3 compose.py first.")
            sys.exit(1)

        self.bubble_font = find_font(
            ["Comic Sans MS", "Bangers", "Impact", "Arial Black", "Arial"],
            self.acfg["bubble_font_size"],
        )

        self._reset()

    # ── Loading ───────────────────────────────────────────────────────────────

    def _load_panels(self) -> list:
        panels = []
        final_dir = out_path(self.cfg, "final")
        for pc in self.cfg["panels"]:
            beat = pc["beat"]
            path = os.path.join(final_dir, f"panel_{beat}.png")
            if not os.path.exists(path):
                print(f"WARNING: missing {path}")
                continue
            surf = pygame.image.load(path).convert_alpha()
            panels.append(
                {
                    "surf": surf,
                    "bubble": pc.get("speech_bubble"),
                    "side": pc.get("bubble_side", "right"),
                    "beat": beat,
                }
            )
        return panels

    # ── State management ──────────────────────────────────────────────────────

    def _reset(self):
        self.panel_idx = 0
        self.state = "fade_in"
        self.timer = 0.0
        self.global_fade = 0.0  # 0 = black, 1 = fully visible
        self.bubble_fade = 0.0  # 0 = hidden, 1 = fully visible

    def _advance(self):
        """Skip to the next panel (or restart if on the last)."""
        self.panel_idx += 1
        if self.panel_idx >= len(self.panels):
            if self.dcfg["loop"]:
                self._reset()
            else:
                self.state = "done"
            return
        self.state = "fade_in"
        self.timer = 0.0
        self.global_fade = 0.0
        self.bubble_fade = 0.0

    # ── Update ────────────────────────────────────────────────────────────────

    def update(self, dt: float):
        if self.state == "done":
            return

        anim = self.acfg
        fd = anim["fade_duration"]
        bd = anim["bubble_delay"]
        bf = anim["bubble_fade"]
        is_last = self.panel_idx == len(self.panels) - 1
        hold = anim["punchline_hold"] if is_last else anim["hold_duration"]

        self.timer += dt

        if self.state == "fade_in":
            self.global_fade = ease_out(self.timer / max(fd, 1e-6))
            self.bubble_fade = 0.0
            if self.timer >= fd:
                self.state = "hold"
                self.timer = 0.0
                self.global_fade = 1.0

        elif self.state == "hold":
            panel = self.panels[self.panel_idx]
            if self.timer >= bd and panel["bubble"]:
                self.state = "bubble_fade"
                self.timer = 0.0
            elif self.timer >= hold and not panel["bubble"]:
                self.state = "fade_out"
                self.timer = 0.0

        elif self.state == "bubble_fade":
            self.bubble_fade = ease_out(self.timer / max(bf, 1e-6))
            if self.timer >= bf:
                self.state = "hold_bubble"
                self.timer = 0.0
                self.bubble_fade = 1.0

        elif self.state == "hold_bubble":
            remaining_hold = hold - bd - bf
            if self.timer >= max(remaining_hold, 0.05):
                self.state = "fade_out"
                self.timer = 0.0

        elif self.state == "fade_out":
            self.global_fade = 1.0 - ease_out(self.timer / max(fd, 1e-6))
            if self.timer >= fd:
                self.global_fade = 0.0
                self._advance()

    # ── Draw ──────────────────────────────────────────────────────────────────

    def draw(self):
        anim = self.acfg
        bg = tuple(anim["background_color"])
        pbc = tuple(anim["panel_bg_color"])
        bdc = tuple(anim["panel_border_color"])
        bdw = anim["panel_border_width"]
        shad = anim["shadow_offset"]

        W, H = self.screen.get_size()
        self.screen.fill(bg)

        if self.state == "done" or self.panel_idx >= len(self.panels):
            self._draw_fade(0.0)
            pygame.display.flip()
            return

        panel_data = self.panels[self.panel_idx]
        cfg_pw, cfg_ph = self.cfg["panel_size"]

        # Compute display rect: fill ~85 % of screen height, centered
        disp_h = int(H * 0.87)
        disp_w = int(disp_h * cfg_pw / cfg_ph)
        if disp_w > W * 0.95:
            disp_w = int(W * 0.95)
            disp_h = int(disp_w * cfg_ph / cfg_pw)
        disp_rect = pygame.Rect((W - disp_w) // 2, (H - disp_h) // 2, disp_w, disp_h)

        ga = int(self.global_fade * 255)
        ba = int(self.bubble_fade * 255)

        # --- Panel frame (shadow + border + cream bg) -----------------------
        if ga > 0:
            shadow_surf = pygame.Surface(
                (disp_w + bdw * 2, disp_h + bdw * 2), pygame.SRCALPHA
            )
            shadow_col = tuple(anim["shadow_color"])
            shadow_surf.fill(shadow_col)
            shadow_surf.set_alpha(ga * shadow_col[3] // 255)
            self.screen.blit(
                shadow_surf, (disp_rect.x + shad - bdw, disp_rect.y + shad - bdw)
            )

            border_rect = disp_rect.inflate(bdw * 2, bdw * 2)
            border_surf = pygame.Surface(border_rect.size, pygame.SRCALPHA)
            pygame.draw.rect(border_surf, bdc + (255,), border_surf.get_rect())
            border_surf.set_alpha(ga)
            self.screen.blit(border_surf, border_rect.topleft)

            bg_surf = pygame.Surface(disp_rect.size, pygame.SRCALPHA)
            bg_surf.fill(pbc + (255,))
            bg_surf.set_alpha(ga)
            self.screen.blit(bg_surf, disp_rect.topleft)

        # --- Character image ------------------------------------------------
        if ga > 0:
            scaled, dest = scale_panel(panel_data["surf"], disp_rect)
            scaled.set_alpha(ga)
            self.screen.blit(scaled, dest)

        # --- Speech bubble --------------------------------------------------
        if ba > 0 and panel_data["bubble"]:
            draw_bubble(
                self.screen,
                panel_data["bubble"],
                self.bubble_font,
                disp_rect,
                panel_data["side"],
                ba,
                self.cfg,
            )

        # --- Black fade overlay (for transitions) ---------------------------
        self._draw_fade(1.0 - self.global_fade)

    def _draw_fade(self, darkness: float):
        if darkness <= 0.0:
            return
        overlay = pygame.Surface(self.screen.get_size())
        overlay.fill((0, 0, 0))
        overlay.set_alpha(int(darkness * 255))
        self.screen.blit(overlay, (0, 0))

    # ── Run ───────────────────────────────────────────────────────────────────

    def run(self):
        running = True
        while running:
            dt = self.clock.tick(self.dcfg["fps"]) / 1000.0
            dt = min(dt, 0.1)  # clamp for window dragging / sleep spikes

            for event in pygame.event.get():
                if event.type == pygame.QUIT:
                    running = False
                elif event.type == pygame.KEYDOWN:
                    if event.key in (pygame.K_ESCAPE, pygame.K_q):
                        running = False
                    elif event.key in (pygame.K_SPACE, pygame.K_RIGHT):
                        if self.state not in ("fade_out", "done"):
                            self.state = "fade_out"
                            self.timer = 0.0
                    elif event.key == pygame.K_r:
                        self._reset()

            self.update(dt)
            self.draw()
            pygame.display.flip()

        pygame.quit()


# ── Strip mode ────────────────────────────────────────────────────────────────


class StripDemo(VanityCardDemo):
    """
    Alternative display: all 4 panels side-by-side, revealing left to right.
    Inherits __init__ and _load_panels from VanityCardDemo.
    """

    def _reset(self):
        self.revealed = 0  # how many panels have been fully shown
        self.state = "fade_in"
        self.timer = 0.0
        self.global_fade = 0.0
        self.bubble_fade = 0.0

    def update(self, dt: float):
        anim = self.acfg
        fd = anim["fade_duration"]
        bd = anim["bubble_delay"]
        bf = anim["bubble_fade"]
        n = len(self.panels)
        if self.revealed >= n and self.state == "done":
            if self.dcfg["loop"]:
                self._reset()
            return

        self.timer += dt

        if self.state == "fade_in":
            self.global_fade = ease_out(self.timer / max(fd, 1e-6))
            if self.timer >= fd:
                self.state = "hold"
                self.timer = 0.0

        elif self.state == "hold":
            panel = self.panels[self.revealed]
            if self.timer >= bd and panel["bubble"]:
                self.state = "bubble_fade"
                self.timer = 0.0
            elif self.timer >= anim["hold_duration"] and not panel["bubble"]:
                self._next_strip_panel()

        elif self.state == "bubble_fade":
            self.bubble_fade = ease_out(self.timer / max(bf, 1e-6))
            if self.timer >= bf:
                self.state = "hold_bubble"
                self.timer = 0.0

        elif self.state == "hold_bubble":
            is_last = self.revealed == n - 1
            hold = anim["punchline_hold"] if is_last else anim["hold_duration"]
            if self.timer >= max(hold - bd - bf, 0.05):
                self._next_strip_panel()

    def _next_strip_panel(self):
        self.revealed += 1
        if self.revealed >= len(self.panels):
            self.state = "done"
        else:
            self.state = "fade_in"
            self.timer = 0.0
            self.global_fade = 0.0
            self.bubble_fade = 0.0

    def draw(self):
        anim = self.acfg
        bg = tuple(anim["background_color"])
        pbc = tuple(anim["panel_bg_color"])
        bdc = tuple(anim["panel_border_color"])
        bdw = anim["panel_border_width"]
        shad = anim["shadow_offset"]

        W, H = self.screen.get_size()
        self.screen.fill(bg)

        n = len(self.panels)
        gap = 10
        cfg_pw, cfg_ph = self.cfg["panel_size"]
        total_w = W - 2 * gap
        panel_display_w = (total_w - (n - 1) * gap) // n
        panel_display_h = int(panel_display_w * cfg_ph / cfg_pw)
        top = (H - panel_display_h) // 2

        for i in range(self.revealed + 1 if self.state != "done" else n):
            panel_data = self.panels[i]
            px = gap + i * (panel_display_w + gap)
            pr = pygame.Rect(px, top, panel_display_w, panel_display_h)

            is_current = i == self.revealed
            ga = int((self.global_fade if is_current else 1.0) * 255)
            ba = int(
                (
                    self.bubble_fade
                    if is_current
                    else (1.0 if i < self.revealed else 0.0)
                )
                * 255
            )

            # Shadow + border + bg
            shd = pygame.Surface(
                (panel_display_w + bdw * 2, panel_display_h + bdw * 2), pygame.SRCALPHA
            )
            shd.fill(tuple(anim["shadow_color"]))
            shd.set_alpha(ga * anim["shadow_color"][3] // 255)
            self.screen.blit(shd, (pr.x + shad - bdw, pr.y + shad - bdw))

            bdr = pr.inflate(bdw * 2, bdw * 2)
            bdr_s = pygame.Surface(bdr.size, pygame.SRCALPHA)
            bdr_s.fill(bdc + (255,))
            bdr_s.set_alpha(ga)
            self.screen.blit(bdr_s, bdr.topleft)

            bg_s = pygame.Surface(pr.size, pygame.SRCALPHA)
            bg_s.fill(pbc + (255,))
            bg_s.set_alpha(ga)
            self.screen.blit(bg_s, pr.topleft)

            scaled, dest = scale_panel(panel_data["surf"], pr)
            scaled.set_alpha(ga)
            self.screen.blit(scaled, dest)

            if ba > 0 and panel_data["bubble"]:
                draw_bubble(
                    self.screen,
                    panel_data["bubble"],
                    self.bubble_font,
                    pr,
                    panel_data["side"],
                    ba,
                    self.cfg,
                )

        self._draw_fade(0.0)  # no global fade in strip mode


# ── Entry point ───────────────────────────────────────────────────────────────


def main():
    cfg = load_config()
    mode = cfg["animation"].get("display_mode", "sequential")
    if mode == "strip":
        demo = StripDemo(cfg)
    else:
        demo = VanityCardDemo(cfg)
    demo.run()


if __name__ == "__main__":
    main()
