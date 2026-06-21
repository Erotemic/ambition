"""Bespoke target for Alice — cryptographer.

Third bespoke template after ``trent_elder`` and ``bob_engineer``. The
silhouette philosophy is intentionally different from both:

  - **Trent**: robe-first, fully draped, no visible legs. Reads as a
    council elder.
  - **Bob**: visible legs + workshop boots + tool belt. Reads as a
    workshop engineer.
  - **Alice** (this file): a knee-length traveling coat over the
    OTP-checker tabard, ankle-cuffed leggings, ankle boots, layered
    hair with proper bangs + a long side braid. Reads as a working
    field cryptographer: portable, ready to move, the cipher
    pattern is a literal panel on her front, the tools are tucked
    into the coat.

Improvements layered on top of the trent + bob lessons:

  - **Layered hair construction.** Hair is a stack of three
    primitives: back mass (big), side curtains (frames cheeks), and
    a separate **bangs** strip across the forehead. Each is its
    own polygon so the front fringe doesn't fight the back mass.
    The long side braid is a fourth primitive that hangs visibly
    past the chest.
  - **Cinched coat with a flared hem.** Not a robe (Trent), not a
    vest (Bob). A waist-cinched traveling coat that pinches at the
    waist and flares slightly at the knee-length hem. Two front
    panels reveal the OTP-tabard underneath.
  - **Cleaner face geometry.** Smaller pupils, defined upper-lid
    line, faint eyelash hint, no chin shadow (per the established
    feminine-archetype rule from toon_side). Mouth is a small soft
    arc, not a cartoon smile.
  - **Proper head-to-shoulder distance** via the same
    ``head_anchor + neck_h + neck_w`` pattern Bob uses, so the head
    sits naturally above the body with a short visible neck.

Multi-view scope: ships **3/4** (canonical + idle + talk) and
**side** (walk + idle_side) for now. Front view is queued in TODO
when the dialog system needs an "Alice looking at the player"
frame. Single archetype today (``alice``).
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from enum import Enum
from typing import Dict, Optional, Tuple

from PIL import Image, ImageColor, ImageDraw
from ambition_sprite2d_renderer.core.draw import rgba, bbox_from_center as _bbox

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]




def parse_background(value: str) -> Optional[Color]:
    return None if str(value).lower() == "transparent" else rgba(str(value))




def _scale_color(color: Color, factor: float) -> Color:
    return (
        int(max(0, min(255, color[0] * factor))),
        int(max(0, min(255, color[1] * factor))),
        int(max(0, min(255, color[2] * factor))),
        color[3],
    )


# ── Palette ──────────────────────────────────────────────────────────────────

ALICE_PALETTE: Dict[str, Color] = {
    "skin": rgba("#E5C5A6"),
    "skin_shadow": rgba("#B3936F"),
    # Hair: deep blue-black with a cool shine, so the long mass
    # reads against the warm skin without looking flat.
    "hair": rgba("#15131C"),
    "hair_shine": rgba("#3A3850"),
    "hair_band": rgba("#C97A4A"),  # the small ribbon at the braid tip
    # Coat: deep teal with darker shadow + brighter shine.
    "coat": rgba("#1B5E6E"),
    "coat_dark": rgba("#0F3A47"),
    "coat_light": rgba("#28828F"),
    # OTP tabard underneath: cream with charcoal checker cells.
    "tabard": rgba("#F0EAD2"),
    "tabard_dark": rgba("#A89C7A"),
    "cipher": rgba("#15131C"),  # the dark checker cells
    # Tights + boots in cool grey-brown.
    "tights": rgba("#3B3540"),
    "tights_shade": rgba("#22202A"),
    "boot": rgba("#2A1F14"),
    "boot_cuff": rgba("#5A3F22"),
    # Accent: warm amber for the sash + scroll ribbon.
    "amber": rgba("#D08A3A"),
    "amber_dark": rgba("#8A5418"),
    # Ink + paper for the scroll prop.
    "paper": rgba("#F5EFE2"),
    "ink": rgba("#16121A"),
    "white": rgba("#F8F2E0"),
    "outline": rgba("#10141A"),
}


class AliceView(str, Enum):
    THREE_QUARTER = "three_quarter"
    SIDE = "side"


ANIMATION_VIEWS: Dict[str, AliceView] = {
    "idle": AliceView.THREE_QUARTER,
    "talk": AliceView.THREE_QUARTER,
    "interact": AliceView.THREE_QUARTER,
    "walk": AliceView.SIDE,
    "idle_side": AliceView.SIDE,
}


@dataclass(frozen=True)
class AliceSpec:
    target: str
    seed: int
    archetype: str
    name: str
    role: str
    palette_name: str
    # Design units (~1px at 128 canvas).
    head_w: float = 23.0
    head_h: float = 27.5
    jaw_h: float = 5.0
    neck_h: float = 2.0
    head_anchor: float = 0.50
    neck_w: float = 5.5
    # Hair — long-haired silhouette is the dominant cue. Trimmed
    # `back_hair_w_extra` from 6 → 4 so the face has more visual
    # weight relative to the hair envelope (it was reading as
    # "small face peeking out of a hood").
    back_hair_w_extra: float = 4.0
    back_hair_h_extra: float = 14.0  # how much it extends past the chin
    bangs_h: float = 3.6  # was 5.0 — was reading as a heavy visor
    curtain_drop: float = 14.0  # how far cheek curtains hang past the jaw
    braid_segments: int = 10  # long forward braid (10 stacked ellipses)
    # Body — slim academic. New scaffold (NOT Trent-derived):
    # short fitted jacket at the shoulders → skirt at the hips →
    # leggings + ankle boots. No flowing robe, no waist sash, no
    # placket/tabard front. The cipher motif lives on a SCARF
    # draped over one shoulder, not as a front panel.
    shoulder_w: float = 22.0
    torso_w: float = 16.0
    # Hip-length fitted jacket (was the long coat).
    jacket_top_w: float = 24.0
    jacket_hem_w: float = 22.0
    jacket_h: float = 22.0
    # A-line knee-length skirt — narrower at the waist, wider at
    # the knee. Distinct from any coat hem in the cast.
    skirt_top_w: float = 22.0
    skirt_hem_w: float = 32.0
    skirt_h: float = 18.0
    # Leggings under the skirt; ankle boots below.
    leg_h: float = 16.0
    leg_w: float = 6.0
    boot_w: float = 10.0
    boot_h: float = 6.0
    # Cipher scarf — wraps the neck and trails down the camera-side
    # shoulder. Its width is independent of any tabard.
    scarf_thickness: float = 4.0
    scarf_drop: float = 28.0
    # Arms.
    arm_len: float = 25.0
    sleeve_w_shoulder: float = 7.0
    sleeve_w_cuff: float = 5.0
    cuff_h: float = 2.5


@dataclass
class AlicePose:
    view: AliceView = AliceView.THREE_QUARTER
    body_bob: float = 0.0
    head_tilt: float = 0.0
    talk_open: float = 0.0
    blink: bool = False
    arm_lift: float = 0.0
    step_phase: float = 0.0
    # Off-arm tucks a stack of papers/notes under the elbow — a
    # different silhouette from Bob (key ring in hand), Trent
    # (handheld scales), and the old toon Alice (cipher scroll out
    # to the side). Both arms hang at the side; the papers stick
    # out forward of the elbow on the camera-far side.
    carry_notes: bool = True


class AliceCryptographerGenerator:
    name = "alice_cryptographer"

    ANIMATIONS: Dict[str, Dict[str, int]] = {
        "idle": {"frames": 6, "duration_ms": 140},
        "walk": {"frames": 8, "duration_ms": 100},
        "talk": {"frames": 6, "duration_ms": 110},
        "interact": {"frames": 6, "duration_ms": 130},
        "idle_side": {"frames": 6, "duration_ms": 140},
    }

    def sample_spec(self, seed: int, archetype: str = "alice") -> AliceSpec:
        if archetype != "alice":
            raise KeyError(
                f"alice_cryptographer target only ships 'alice' archetype; got {archetype!r}."
            )
        return AliceSpec(
            target=self.name,
            seed=seed,
            archetype=archetype,
            name="Alice",
            role="npc",
            palette_name="alice",
        )

    # --- pose -----------------------------------------------------------------

    def pose_for_animation(
        self, animation: str, frame_index: int, frame_count: int
    ) -> AlicePose:
        view = ANIMATION_VIEWS.get(animation, AliceView.THREE_QUARTER)
        p = AlicePose(view=view)
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        wave = math.sin(t * math.tau)
        half_wave = math.sin(t * math.pi)
        if animation == "idle":
            p.body_bob = wave * 0.5
            p.head_tilt = wave * 1.2
            p.blink = frame_index == frame_count - 1
        elif animation == "idle_side":
            p.body_bob = wave * 0.5
            p.blink = frame_index == frame_count - 1
        elif animation == "walk":
            # Phase-shifted so frame 0 already shows a stride peak
            # (sin(pi/2) = +1) instead of the neutral midpoint
            # (sin(0) = 0). Spot checks of frame 0 should look like
            # walking, not standing.
            phase_wave = math.sin((t + 0.25) * math.tau)
            p.step_phase = phase_wave
            p.body_bob = abs(phase_wave) * 1.0
            p.head_tilt = -phase_wave * 0.6
        elif animation == "talk":
            p.talk_open = (0.5 + 0.5 * wave) * 0.85
            p.head_tilt = wave * 1.2
            p.arm_lift = max(0.0, half_wave) * 0.6
        elif animation == "interact":
            p.arm_lift = half_wave * 1.0
            p.head_tilt = wave * 0.4
        return p

    # --- top-level frame ------------------------------------------------------

    def render_animation_frame(
        self,
        spec: AliceSpec,
        animation: str,
        frame_index: int,
        frame_count: int,
        size: Tuple[int, int],
        *,
        background: Optional[Color] = None,
        supersample: int = 4,
        downsample: str = "lanczos",
    ) -> Image.Image:
        W, H = size
        ss = max(1, int(supersample))
        img = Image.new("RGBA", (W * ss, H * ss), background or (0, 0, 0, 0))
        S = (W / 128.0) * ss
        pal = ALICE_PALETTE
        pose = self.pose_for_animation(animation, frame_index, frame_count)

        cx = 64.0 * S
        feet_y = 116.0 * S + pose.body_bob * S
        if pose.view == AliceView.SIDE:
            self._render_side(img, cx, feet_y, spec, pal, S, pose)
        else:
            self._render_three_quarter(img, cx, feet_y, spec, pal, S, pose)

        if ss > 1:
            img = img.resize((W, H), Image.LANCZOS)
        return img

    # ─────────────────────────────────────────────────────────────────
    # THREE-QUARTER view
    # ─────────────────────────────────────────────────────────────────

    def _render_three_quarter(
        self,
        base: Image.Image,
        cx: float,
        feet_y: float,
        spec: AliceSpec,
        pal: Dict[str, Color],
        S: float,
        pose: AlicePose,
    ) -> None:
        boot_top_y = feet_y - spec.boot_h * S
        leg_top_y = boot_top_y - spec.leg_h * S
        # Stack the new scaffold from feet up: legs → skirt → jacket.
        # The skirt hem sits just above the leggings; the jacket hem
        # tops the skirt. NO continuous robe, NO waist sash, NO front
        # placket — explicitly NOT a Trent-derived silhouette.
        skirt_hem_y = leg_top_y + 0.0 * S
        skirt_top_y = skirt_hem_y - spec.skirt_h * S
        jacket_hem_y = skirt_top_y + 1.0 * S
        shoulder_y = jacket_hem_y - spec.jacket_h * S
        head_center = (
            cx + 2.0 * S,
            shoulder_y - spec.head_h * spec.head_anchor * S - spec.neck_h * S,
        )

        self._tq_draw_legs(base, cx, leg_top_y, boot_top_y, feet_y, spec, pal, S)
        self._tq_draw_skirt(base, cx, skirt_top_y, skirt_hem_y, spec, pal, S)
        self._tq_draw_arms_back(base, cx, shoulder_y, jacket_hem_y, spec, pal, S, pose)
        self._tq_draw_jacket(base, cx, shoulder_y, jacket_hem_y, spec, pal, S)
        self._tq_draw_arms_front(base, cx, shoulder_y, jacket_hem_y, spec, pal, S, pose)
        # The scarf wraps the neck and drapes down OVER the jacket
        # front, so it draws after the jacket but before the head.
        self._tq_draw_scarf(base, cx, shoulder_y, spec, pal, S)
        self._tq_draw_head(base, head_center, spec, pal, S, pose)

    def _tq_draw_legs(
        self,
        base: Image.Image,
        cx: float,
        leg_top_y: float,
        boot_top_y: float,
        feet_y: float,
        spec: AliceSpec,
        pal: Dict[str, Color],
        S: float,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Spread the legs so they don't merge at the centerline (was
        # sign * 4.0 with leg_w 6 — gap was 2 design units, hard to
        # read at downsample). New spacing leaves a clear ankle gap.
        for sign in (-1, 1):
            hip_x = cx + sign * 5.5 * S
            ankle_x = cx + sign * 5.5 * S
            leg = [
                (hip_x - spec.leg_w * 0.5 * S, leg_top_y),
                (hip_x + spec.leg_w * 0.5 * S, leg_top_y),
                (ankle_x + spec.boot_w * 0.4 * S, boot_top_y),
                (ankle_x - spec.boot_w * 0.4 * S, boot_top_y),
            ]
            d.polygon(leg, fill=pal["tights"], outline=outline)
            # Boot — short ankle boot with a brown cuff at the top.
            d.rounded_rectangle(
                (
                    ankle_x - spec.boot_w * 0.55 * S,
                    boot_top_y - 0.5 * S,
                    ankle_x + spec.boot_w * 0.55 * S,
                    feet_y,
                ),
                radius=2.0 * S,
                fill=pal["boot"],
                outline=outline,
                width=max(1, int(1.0 * S)),
            )
            d.rectangle(
                (
                    ankle_x - spec.boot_w * 0.55 * S,
                    boot_top_y - 0.5 * S,
                    ankle_x + spec.boot_w * 0.55 * S,
                    boot_top_y + 1.5 * S,
                ),
                fill=pal["boot_cuff"],
                outline=outline,
                width=max(1, int(0.7 * S)),
            )

    def _tq_draw_skirt(
        self,
        base: Image.Image,
        cx: float,
        top_y: float,
        hem_y: float,
        spec: AliceSpec,
        pal: Dict[str, Color],
        S: float,
    ) -> None:
        """A-line knee-length skirt — narrow at the waist, wider at
        the knee. Drawn before the jacket so the jacket hem covers
        the top edge of the skirt for a layered read."""
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        skirt = [
            (cx - spec.skirt_top_w * 0.5 * S, top_y),
            (cx + spec.skirt_top_w * 0.5 * S, top_y),
            (cx + spec.skirt_hem_w * 0.5 * S, hem_y),
            (cx - spec.skirt_hem_w * 0.5 * S, hem_y),
        ]
        d.polygon(skirt, fill=pal["coat_dark"], outline=outline)
        # Two pleat folds suggesting the A-line drape.
        for sign in (-1, 1):
            d.line(
                [
                    (cx + sign * spec.skirt_top_w * 0.18 * S, top_y + 2.0 * S),
                    (cx + sign * spec.skirt_hem_w * 0.30 * S, hem_y - 1.0 * S),
                ],
                fill=pal["coat_dark"],
                width=max(1, int(0.8 * S)),
            )
        # A subtler highlight on the camera-side fold.
        d.line(
            [
                (cx + spec.skirt_top_w * 0.04 * S, top_y + 2.0 * S),
                (cx + spec.skirt_hem_w * 0.10 * S, hem_y - 1.0 * S),
            ],
            fill=pal["coat"],
            width=max(1, int(0.7 * S)),
        )
        # Hem band — thin lighter stripe along the bottom edge.
        d.line(
            [
                (cx - spec.skirt_hem_w * 0.50 * S, hem_y - 0.5 * S),
                (cx + spec.skirt_hem_w * 0.50 * S, hem_y - 0.5 * S),
            ],
            fill=pal["coat"],
            width=max(1, int(0.9 * S)),
        )

    def _tq_draw_jacket(
        self,
        base: Image.Image,
        cx: float,
        shoulder_y: float,
        hem_y: float,
        spec: AliceSpec,
        pal: Dict[str, Color],
        S: float,
    ) -> None:
        """Short fitted hip-length jacket. Pinches in slightly at the
        hip rather than flaring; closed button-front rather than
        open with a placket. Deliberately not a Trent-style robe."""
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        jacket = [
            (cx - spec.jacket_top_w * 0.5 * S, shoulder_y),
            (cx + spec.jacket_top_w * 0.5 * S, shoulder_y),
            (cx + spec.jacket_hem_w * 0.5 * S, hem_y),
            (cx - spec.jacket_hem_w * 0.5 * S, hem_y),
        ]
        d.polygon(jacket, fill=pal["coat"], outline=outline)
        # Three button-front buttons down the center.
        button_x = cx + 0.5 * S
        for i in range(3):
            y = shoulder_y + (4.0 + i * 5.5) * S
            d.ellipse(
                _bbox((button_x, y), 1.4 * S, 1.4 * S),
                fill=pal["amber"],
                outline=outline,
                width=max(1, int(0.5 * S)),
            )
        # Jacket collar — a small V-shaped notched lapel at the throat.
        collar = [
            (cx - spec.jacket_top_w * 0.30 * S, shoulder_y),
            (cx + spec.jacket_top_w * 0.30 * S, shoulder_y),
            (cx + 2.0 * S, shoulder_y + 4.0 * S),
            (cx, shoulder_y + 6.0 * S),
            (cx - 2.0 * S, shoulder_y + 4.0 * S),
        ]
        d.polygon(collar, fill=pal["coat_dark"], outline=outline)
        # Subtle highlight along the camera-side seam.
        d.line(
            [
                (cx + spec.jacket_top_w * 0.40 * S, shoulder_y + 2.0 * S),
                (cx + spec.jacket_hem_w * 0.40 * S, hem_y - 1.0 * S),
            ],
            fill=pal["coat_light"],
            width=max(1, int(0.7 * S)),
        )

    def _tq_draw_scarf(
        self,
        base: Image.Image,
        cx: float,
        shoulder_y: float,
        spec: AliceSpec,
        pal: Dict[str, Color],
        S: float,
    ) -> None:
        """Long cream scarf wrapped around the neck with a forward
        drape down the camera-side. The cipher motif lives HERE as
        ciphertext characters along the trailing edge, not as a
        front-panel checker tabard. New visual cipher signature
        that doesn't repeat the previous OTP-grid placement."""
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Wrap loop around the back of the neck (visible as a low
        # arc just above the collar).
        wrap_top = shoulder_y - 4.0 * S
        wrap_bot = shoulder_y + 2.0 * S
        d.rounded_rectangle(
            (cx - 9.0 * S, wrap_top, cx + 9.0 * S, wrap_bot),
            radius=2.0 * S,
            fill=pal["tabard"],
            outline=outline,
            width=max(1, int(0.9 * S)),
        )
        # Forward drape — a long rectangle of scarf hanging down the
        # camera-side, past the jacket hem.
        drape_top_x = cx + 5.0 * S
        drape_top_y = shoulder_y + 1.0 * S
        drape_bot_x = cx + 8.0 * S
        drape_bot_y = drape_top_y + spec.scarf_drop * S
        drape = [
            (drape_top_x - spec.scarf_thickness * 0.5 * S, drape_top_y),
            (drape_top_x + spec.scarf_thickness * 0.5 * S, drape_top_y),
            (drape_bot_x + spec.scarf_thickness * 0.55 * S, drape_bot_y),
            (drape_bot_x - spec.scarf_thickness * 0.55 * S, drape_bot_y),
        ]
        d.polygon(drape, fill=pal["tabard"], outline=outline)
        # Frayed tassel ends at the bottom.
        for tx in (-1.5, 0.0, 1.5):
            d.line(
                [
                    (drape_bot_x + tx * S, drape_bot_y - 0.5 * S),
                    (drape_bot_x + tx * S, drape_bot_y + 3.0 * S),
                ],
                fill=pal["tabard_dark"],
                width=max(1, int(0.7 * S)),
            )
        # Cipher characters running down the drape — short ink dashes
        # in alternating orientations, NOT the OTP-grid checker.
        rng_chars = [(-0.6, 0.6), (0.4, -0.5), (-0.4, 0.4), (0.5, -0.3), (-0.3, 0.5)]
        for i, (dx_off, dy_off) in enumerate(rng_chars):
            ch_y = drape_top_y + (4.0 + i * 5.0) * S
            ch_x = drape_top_x + (drape_bot_x - drape_top_x) * (
                i / float(len(rng_chars) - 1)
            )
            d.line(
                [
                    (ch_x - 1.0 * S + dx_off * S, ch_y + dy_off * S),
                    (ch_x + 1.0 * S + dx_off * S, ch_y - dy_off * S),
                ],
                fill=pal["ink"],
                width=max(1, int(0.6 * S)),
            )

    def _tq_draw_arms_back(
        self,
        base: Image.Image,
        cx: float,
        shoulder_y: float,
        jacket_hem_y: float,
        spec: AliceSpec,
        pal: Dict[str, Color],
        S: float,
        pose: AlicePose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Far arm — tucks a stack of papers under the elbow. The
        # sleeve hangs at the side; the papers stick out forward of
        # the elbow as a small white rectangle peeking out behind
        # the jacket.
        sx = cx - spec.shoulder_w * 0.42 * S
        sy = shoulder_y + 2.0 * S
        ex = sx - 0.5 * S
        ey = sy + spec.arm_len * S
        sleeve = [
            (sx - spec.sleeve_w_shoulder * 0.5 * S, sy),
            (sx + spec.sleeve_w_shoulder * 0.5 * S, sy),
            (ex + spec.sleeve_w_cuff * 0.5 * S, ey),
            (ex - spec.sleeve_w_cuff * 0.5 * S, ey),
        ]
        d.polygon(sleeve, fill=pal["coat_dark"], outline=outline)
        d.rectangle(
            (
                ex - spec.sleeve_w_cuff * 0.6 * S,
                ey - spec.cuff_h * 0.5 * S,
                ex + spec.sleeve_w_cuff * 0.6 * S,
                ey + spec.cuff_h * 0.5 * S,
            ),
            fill=pal["coat_light"],
            outline=outline,
            width=max(1, int(0.7 * S)),
        )
        d.ellipse(
            _bbox((ex, ey + 3.0 * S), 3.0 * S, 2.6 * S),
            fill=pal["skin"],
            outline=outline,
            width=max(1, int(0.7 * S)),
        )
        if pose.carry_notes:
            # A small stack of papers (3 sheets) clipped to the side
            # of the body just above the jacket hem. Visible behind
            # the camera-far elbow.
            self._draw_papers_stack(
                d, (cx - spec.shoulder_w * 0.30 * S, jacket_hem_y - 6.0 * S), S, pal
            )

    def _draw_papers_stack(
        self, d: ImageDraw.ImageDraw, anchor: Point, S: float, pal: Dict[str, Color]
    ) -> None:
        """Three offset sheets of paper tucked under the elbow.
        Sits on the camera-FAR side so it doesn't fight the scarf."""
        outline = pal["outline"]
        for i, (dx, dy) in enumerate(((0.0, 0.0), (-1.5, -1.2), (1.0, -2.4))):
            sx0 = anchor[0] + dx * S - 5.0 * S
            sy0 = anchor[1] + dy * S - 6.0 * S
            sx1 = anchor[0] + dx * S + 5.0 * S
            sy1 = anchor[1] + dy * S + 6.0 * S
            d.rectangle(
                (sx0, sy0, sx1, sy1),
                fill=pal["paper"],
                outline=outline,
                width=max(1, int(0.6 * S)),
            )
            # A couple of ink lines on the topmost sheet.
            if i == 2:
                for ly in (-3.0, 0.0, 3.0):
                    d.line(
                        [
                            (sx0 + 1.5 * S, anchor[1] + dy * S + ly * S),
                            (sx1 - 1.5 * S, anchor[1] + dy * S + ly * S),
                        ],
                        fill=pal["ink"],
                        width=max(1, int(0.5 * S)),
                    )

    def _tq_draw_arms_front(
        self,
        base: Image.Image,
        cx: float,
        shoulder_y: float,
        jacket_hem_y: float,
        spec: AliceSpec,
        pal: Dict[str, Color],
        S: float,
        pose: AlicePose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Near arm — hangs at the side with a slight forward lean
        # during talk/interact (arm_lift). No prop in the front
        # hand; the prop (papers stack) is tucked under the far arm.
        sx = cx + spec.shoulder_w * 0.38 * S
        sy = shoulder_y + 2.0 * S
        ex = sx + 1.0 * S + pose.arm_lift * 5.0 * S
        ey = sy + spec.arm_len * S - pose.arm_lift * 8.0 * S
        sleeve = [
            (sx - spec.sleeve_w_shoulder * 0.5 * S, sy),
            (sx + spec.sleeve_w_shoulder * 0.5 * S, sy),
            (ex + spec.sleeve_w_cuff * 0.55 * S, ey),
            (ex - spec.sleeve_w_cuff * 0.55 * S, ey),
        ]
        d.polygon(sleeve, fill=pal["coat"], outline=outline)
        d.line(
            [
                (sx - 2.0 * S, sy + 1.0 * S),
                (ex - spec.sleeve_w_cuff * 0.30 * S, ey - 1.0 * S),
            ],
            fill=pal["coat_light"],
            width=max(1, int(0.8 * S)),
        )
        d.rectangle(
            (
                ex - spec.sleeve_w_cuff * 0.6 * S,
                ey - spec.cuff_h * 0.5 * S,
                ex + spec.sleeve_w_cuff * 0.6 * S,
                ey + spec.cuff_h * 0.5 * S,
            ),
            fill=pal["coat_light"],
            outline=outline,
            width=max(1, int(0.7 * S)),
        )
        hand_c = (ex + 0.5 * S, ey + 3.5 * S)
        d.ellipse(
            _bbox(hand_c, 3.6 * S, 3.0 * S),
            fill=pal["skin"],
            outline=outline,
            width=max(1, int(0.8 * S)),
        )

    def _tq_draw_head(
        self,
        base: Image.Image,
        c: Point,
        spec: AliceSpec,
        pal: Dict[str, Color],
        S: float,
        pose: AlicePose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Hair z-order — explicit, documented stack (previous revision
        # drew curtains AFTER the face, so the curtain inner edge cut
        # into the face oval and the cheeks turned dark):
        #   1. Neck   (chin will cover the top of it)
        #   2. Back hair mass  (sits behind everything)
        #   3. Cheek curtains  (sit behind the face so the face oval
        #                       crops their inner edge cleanly)
        #   4. Face oval        ← key: drawn AFTER curtains
        #   5. Bangs            (sit over the upper face only)
        #   6. Forward braid    (drapes in front of the body)
        #   7. Eyes / nose / mouth
        # The order is named with comments so the failure mode
        # "curtain bites into the cheek" can't recur silently.

        # 1. Neck.
        self._draw_neck(d, c, spec, pal, S, slant=+0.5)
        # 2. Back hair mass — extends WIDE and LONG past the head.
        back_hair_w = (spec.head_w + spec.back_hair_w_extra) * S
        back_hair_h = (spec.head_h + spec.back_hair_h_extra) * S
        back_hair_cy = c[1] + spec.back_hair_h_extra * 0.30 * S
        d.ellipse(
            _bbox((c[0] - 0.5 * S, back_hair_cy), back_hair_w, back_hair_h),
            fill=pal["hair"],
            outline=outline,
            width=max(1, int(1.0 * S)),
        )
        # 3. Side hair curtains BEFORE the face. They start outside
        # the face (head_w * 0.50) and only their lower-outer halves
        # show — the face oval cleanly crops the inner part. The
        # previous shape had sharp polygon corners that read as
        # triangular hangs; the new vertex set adds an intermediate
        # bulge so each curtain reads as a softer cascade.
        for sign in (-1, 1):
            curtain = [
                (c[0] + sign * spec.head_w * 0.50 * S, c[1] - spec.head_h * 0.22 * S),
                (c[0] + sign * (spec.head_w * 0.58) * S, c[1] + spec.head_h * 0.06 * S),
                (
                    c[0] + sign * (spec.head_w * 0.54) * S,
                    c[1] + (spec.head_h * 0.26 + spec.curtain_drop * 0.3) * S,
                ),
                (
                    c[0] + sign * (spec.head_w * 0.42) * S,
                    c[1] + (spec.head_h * 0.40 + spec.curtain_drop * 0.5) * S,
                ),
                (
                    c[0] + sign * (spec.head_w * 0.28) * S,
                    c[1] + (spec.head_h * 0.32 + spec.curtain_drop * 0.4) * S,
                ),
                (c[0] + sign * (spec.head_w * 0.24) * S, c[1] + spec.head_h * 0.10 * S),
            ]
            d.polygon(curtain, fill=pal["hair"], outline=outline)
        # 4. Face oval — drawn AFTER curtains so the cheeks stay
        # uncovered. This is the z-order fix.
        d.ellipse(
            _bbox(c, spec.head_w * S, spec.head_h * S),
            fill=pal["skin"],
            outline=outline,
            width=max(1, int(1.2 * S)),
        )
        # 5. Bangs — across the forehead, drawn over the face top.
        # Outer corners pulled INSIDE the head_w envelope (was 0.50,
        # now 0.42) and slightly lower so they tuck under the back
        # hair top edge instead of poking up past it as the two
        # pointy "horns" the previous revision had. Top edge slopes
        # gently from outer to outer for a softer arc.
        bangs_top = c[1] - spec.head_h * 0.42 * S
        bangs_bot = bangs_top + spec.bangs_h * S
        d.polygon(
            [
                (c[0] - spec.head_w * 0.42 * S, bangs_top + 0.5 * S),
                (c[0] - spec.head_w * 0.20 * S, bangs_top - 0.6 * S),
                (c[0] + spec.head_w * 0.22 * S, bangs_top - 0.6 * S),
                (c[0] + spec.head_w * 0.42 * S, bangs_top + 0.5 * S),
                (c[0] + spec.head_w * 0.38 * S, bangs_bot),
                (c[0] + spec.head_w * 0.04 * S, bangs_bot - 1.0 * S),
                (c[0] - spec.head_w * 0.10 * S, bangs_bot),
                (c[0] - spec.head_w * 0.38 * S, bangs_bot - 1.0 * S),
            ],
            fill=pal["hair"],
            outline=outline,
        )
        # Tiny skin sliver between the two bang clumps.
        d.line(
            [
                (c[0] - spec.head_w * 0.06 * S, bangs_bot - 0.5 * S),
                (c[0] + spec.head_w * 0.00 * S, bangs_bot + 1.5 * S),
            ],
            fill=pal["skin"],
            width=max(1, int(0.7 * S)),
        )
        # Long forward braid — 10 stacked ellipses falling over the
        # camera-side shoulder past the chest.
        braid_anchor_x = c[0] + spec.head_w * 0.34 * S
        braid_anchor_y = c[1] + spec.head_h * 0.38 * S
        for i in range(spec.braid_segments):
            t_seg = i / float(spec.braid_segments - 1)
            w = (4.6 - t_seg * 2.6) * S
            dy = (2.0 + i * 5.2) * S
            seg_c = (braid_anchor_x - t_seg * 4.0 * S, braid_anchor_y + dy)
            d.ellipse(
                _bbox(seg_c, w, 3.2 * S),
                fill=pal["hair"],
                outline=outline,
                width=max(1, int(0.9 * S)),
            )
            d.ellipse(
                _bbox((seg_c[0] - 0.8 * S, seg_c[1] - 1.0 * S), 1.5 * S, 0.9 * S),
                fill=pal["hair_shine"],
                outline=None,
            )
        # Ribbon tying the braid.
        tip_x = braid_anchor_x - 4.0 * S
        tip_y = braid_anchor_y + (2.0 + (spec.braid_segments - 1) * 5.2 + 4.0) * S
        d.rounded_rectangle(
            (tip_x - 3.0 * S, tip_y - 1.6 * S, tip_x + 1.0 * S, tip_y + 1.4 * S),
            radius=0.9 * S,
            fill=pal["hair_band"],
            outline=outline,
            width=max(1, int(0.6 * S)),
        )
        # Face: eyes, eyelashes, nose, mouth.
        self._draw_eyes_three_quarter(d, c, spec, pal, S, pose)
        # Nose (subtle skin-shadow stroke).
        d.line(
            [
                (c[0] + 3.0 * S, c[1] + 1.0 * S),
                (c[0] + 4.0 * S, c[1] + 3.0 * S),
                (c[0] + 3.0 * S, c[1] + 4.0 * S),
            ],
            fill=pal["skin_shadow"],
            width=max(1, int(0.7 * S)),
        )
        # Mouth — small soft arc, opens during talk.
        mouth_y = c[1] + spec.head_h * 0.30 * S
        if pose.talk_open > 0.2:
            d.ellipse(
                _bbox(
                    (c[0] + 2.5 * S, mouth_y), 3.0 * S, (1.0 + pose.talk_open * 1.4) * S
                ),
                fill=outline,
            )
        else:
            d.arc(
                (c[0] + 0.5 * S, mouth_y - 1.2 * S, c[0] + 5.0 * S, mouth_y + 2.0 * S),
                start=10,
                end=160,
                fill=outline,
                width=max(1, int(0.9 * S)),
            )

    def _draw_eyes_three_quarter(
        self,
        d: ImageDraw.ImageDraw,
        c: Point,
        spec: AliceSpec,
        pal: Dict[str, Color],
        S: float,
        pose: AlicePose,
    ) -> None:
        outline = pal["outline"]
        eye_y = c[1] - 2.0 * S
        near = (c[0] + 1.0 * S, eye_y)
        far = (c[0] + 7.0 * S, eye_y - 0.2 * S)
        if pose.blink:
            d.line(
                [(near[0] - 1.6 * S, near[1]), (near[0] + 2.0 * S, near[1])],
                fill=outline,
                width=max(1, int(1.0 * S)),
            )
            d.line(
                [(far[0] - 1.2 * S, far[1]), (far[0] + 1.2 * S, far[1])],
                fill=outline,
                width=max(1, int(0.8 * S)),
            )
            return
        # Eyes: white sclera + iris + pupil for a slightly finer read
        # than the toon/bob eye block.
        d.ellipse(
            _bbox(near, 3.0 * S, 1.6 * S),
            fill=pal["white"],
            outline=outline,
            width=max(1, int(0.9 * S)),
        )
        d.ellipse(
            _bbox((near[0] + 0.4 * S, near[1]), 1.6 * S, 1.6 * S),
            fill=pal["coat_dark"],
            outline=None,
        )
        d.ellipse(_bbox((near[0] + 0.4 * S, near[1]), 0.8 * S, 1.0 * S), fill=outline)
        d.ellipse(
            _bbox(far, 2.4 * S, 1.4 * S),
            fill=pal["white"],
            outline=outline,
            width=max(1, int(0.8 * S)),
        )
        d.ellipse(
            _bbox((far[0] + 0.3 * S, far[1]), 1.4 * S, 1.4 * S),
            fill=pal["coat_dark"],
            outline=None,
        )
        d.ellipse(_bbox((far[0] + 0.3 * S, far[1]), 0.7 * S, 0.9 * S), fill=outline)
        # Single small eyelash tick at the outer corner of each eye.
        d.line(
            [
                (near[0] + 1.8 * S, near[1] - 1.4 * S),
                (near[0] + 2.2 * S, near[1] - 2.4 * S),
            ],
            fill=outline,
            width=max(1, int(0.5 * S)),
        )
        d.line(
            [
                (far[0] + 1.4 * S, far[1] - 1.2 * S),
                (far[0] + 1.7 * S, far[1] - 2.0 * S),
            ],
            fill=outline,
            width=max(1, int(0.4 * S)),
        )

    def _draw_neck(
        self,
        d: ImageDraw.ImageDraw,
        head_center: Point,
        spec: AliceSpec,
        pal: Dict[str, Color],
        S: float,
        *,
        slant: float = 0.0,
    ) -> None:
        outline = pal["outline"]
        chin_y = head_center[1] + spec.head_h * 0.42 * S
        shoulder_y = (
            head_center[1] + spec.head_h * spec.head_anchor * S + spec.neck_h * S
        )
        top_w = spec.neck_w * 0.80
        bot_w = spec.neck_w * 1.10
        neck = [
            (head_center[0] - top_w * 0.5 * S + slant * S, chin_y),
            (head_center[0] + top_w * 0.5 * S + slant * S, chin_y),
            (head_center[0] + bot_w * 0.5 * S, shoulder_y + 1.5 * S),
            (head_center[0] - bot_w * 0.5 * S, shoulder_y + 1.5 * S),
        ]
        d.polygon(neck, fill=pal["skin"], outline=outline)
        d.line(
            [
                (head_center[0] - top_w * 0.40 * S + slant * S, chin_y + 0.5 * S),
                (head_center[0] - bot_w * 0.42 * S, shoulder_y),
            ],
            fill=pal["skin_shadow"],
            width=max(1, int(0.7 * S)),
        )

    # ─────────────────────────────────────────────────────────────────
    # SIDE view — used for walking. Simpler than 3/4 (one visible eye,
    # one ear-curtain of hair, no checker visible on the tabard).
    # ─────────────────────────────────────────────────────────────────

    def _render_side(
        self,
        base: Image.Image,
        cx: float,
        feet_y: float,
        spec: AliceSpec,
        pal: Dict[str, Color],
        S: float,
        pose: AlicePose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        boot_top_y = feet_y - spec.boot_h * S
        leg_top_y = boot_top_y - spec.leg_h * S
        skirt_hem_y = leg_top_y + 0.0 * S
        skirt_top_y = skirt_hem_y - spec.skirt_h * S
        jacket_hem_y = skirt_top_y + 1.0 * S
        shoulder_y = jacket_hem_y - spec.jacket_h * S
        head_center = (
            cx + 3.0 * S,
            shoulder_y - spec.head_h * spec.head_anchor * S - spec.neck_h * S,
        )

        step = pose.step_phase
        # Two legs with opposite-phase swing.
        for side, sgn in (("far", -step), ("near", +step)):
            knee_x = cx + sgn * 3.0 * S
            ankle_x = cx + sgn * 6.0 * S
            knee_y = leg_top_y + spec.leg_h * 0.55 * S
            ankle_y = boot_top_y - abs(sgn) * 1.2 * S
            leg = [
                (cx + sgn * 1.0 * S - spec.leg_w * 0.5 * S, leg_top_y),
                (cx + sgn * 1.0 * S + spec.leg_w * 0.5 * S, leg_top_y),
                (knee_x + spec.leg_w * 0.4 * S, knee_y),
                (ankle_x + spec.boot_w * 0.4 * S, ankle_y),
                (ankle_x - spec.boot_w * 0.4 * S, ankle_y),
                (knee_x - spec.leg_w * 0.4 * S, knee_y),
            ]
            d.polygon(
                leg,
                fill=(pal["tights"] if side == "near" else pal["tights_shade"]),
                outline=outline,
            )
            d.rounded_rectangle(
                (
                    ankle_x - spec.boot_w * 0.55 * S,
                    ankle_y - 0.5 * S,
                    ankle_x + spec.boot_w * 0.90 * S,
                    ankle_y + spec.boot_h * S,
                ),
                radius=2.0 * S,
                fill=pal["boot"],
                outline=outline,
                width=max(1, int(1.0 * S)),
            )
            d.rectangle(
                (
                    ankle_x - spec.boot_w * 0.55 * S,
                    ankle_y - 0.5 * S,
                    ankle_x + spec.boot_w * 0.90 * S,
                    ankle_y + 1.5 * S,
                ),
                fill=pal["boot_cuff"],
                outline=outline,
                width=max(1, int(0.7 * S)),
            )

        # Skirt in profile — narrow at waist, flared at hem.
        skirt = [
            (cx - spec.skirt_top_w * 0.18 * S, skirt_top_y),
            (cx + spec.skirt_top_w * 0.30 * S, skirt_top_y),
            (cx + spec.skirt_hem_w * 0.36 * S, skirt_hem_y),
            (cx - spec.skirt_hem_w * 0.30 * S, skirt_hem_y),
        ]
        d.polygon(skirt, fill=pal["coat_dark"], outline=outline)
        d.line(
            [
                (cx - spec.skirt_hem_w * 0.30 * S, skirt_hem_y - 0.5 * S),
                (cx + spec.skirt_hem_w * 0.36 * S, skirt_hem_y - 0.5 * S),
            ],
            fill=pal["coat"],
            width=max(1, int(0.9 * S)),
        )

        # Short fitted jacket in profile.
        jacket = [
            (cx - spec.jacket_top_w * 0.18 * S, shoulder_y),
            (cx + spec.jacket_top_w * 0.36 * S, shoulder_y),
            (cx + spec.jacket_hem_w * 0.32 * S, jacket_hem_y),
            (cx - spec.jacket_hem_w * 0.22 * S, jacket_hem_y),
        ]
        d.polygon(jacket, fill=pal["coat"], outline=outline)
        # One visible button on the camera-side front.
        d.ellipse(
            _bbox(
                (cx + spec.jacket_hem_w * 0.20 * S, shoulder_y + 8.0 * S),
                1.2 * S,
                1.2 * S,
            ),
            fill=pal["amber"],
            outline=outline,
            width=max(1, int(0.5 * S)),
        )
        # Notched collar.
        d.polygon(
            [
                (cx + spec.jacket_top_w * 0.10 * S, shoulder_y),
                (cx + spec.jacket_top_w * 0.34 * S, shoulder_y),
                (cx + spec.jacket_top_w * 0.10 * S, shoulder_y + 5.0 * S),
            ],
            fill=pal["coat_dark"],
            outline=outline,
        )

        # Cipher scarf — wraps the neck + drapes down the camera-side
        # over the jacket front.
        d.rounded_rectangle(
            (cx - 5.0 * S, shoulder_y - 4.0 * S, cx + 7.0 * S, shoulder_y + 2.0 * S),
            radius=2.0 * S,
            fill=pal["tabard"],
            outline=outline,
            width=max(1, int(0.9 * S)),
        )
        drape_top_x = cx + 5.0 * S
        drape_bot_x = cx + 8.0 * S
        drape_top_y = shoulder_y + 1.0 * S
        drape_bot_y = drape_top_y + spec.scarf_drop * S
        d.polygon(
            [
                (drape_top_x - spec.scarf_thickness * 0.5 * S, drape_top_y),
                (drape_top_x + spec.scarf_thickness * 0.5 * S, drape_top_y),
                (drape_bot_x + spec.scarf_thickness * 0.55 * S, drape_bot_y),
                (drape_bot_x - spec.scarf_thickness * 0.55 * S, drape_bot_y),
            ],
            fill=pal["tabard"],
            outline=outline,
        )
        for tx in (-1.5, 0.0, 1.5):
            d.line(
                [
                    (drape_bot_x + tx * S, drape_bot_y - 0.5 * S),
                    (drape_bot_x + tx * S, drape_bot_y + 3.0 * S),
                ],
                fill=pal["tabard_dark"],
                width=max(1, int(0.7 * S)),
            )

        # Arms swing opposite the legs.
        arm_swing = -step
        far_sh = (cx - spec.shoulder_w * 0.06 * S, shoulder_y + 2.0 * S)
        far_hand = (cx - 2.5 * S + arm_swing * 4.0 * S, shoulder_y + spec.arm_len * S)
        d.line(
            [far_sh, far_hand],
            fill=pal["coat_dark"],
            width=max(1, int(spec.sleeve_w_shoulder * 0.7 * S)),
        )
        d.ellipse(
            _bbox((far_hand[0], far_hand[1] + 3.0 * S), 3.0 * S, 2.6 * S),
            fill=pal["skin"],
            outline=outline,
            width=max(1, int(0.7 * S)),
        )
        near_sh = (cx + spec.shoulder_w * 0.28 * S, shoulder_y + 2.0 * S)
        near_hand = (cx + 5.0 * S - arm_swing * 4.0 * S, shoulder_y + spec.arm_len * S)
        d.line(
            [near_sh, near_hand],
            fill=pal["coat"],
            width=max(1, int(spec.sleeve_w_shoulder * 0.7 * S)),
        )
        d.ellipse(
            _bbox((near_hand[0], near_hand[1] + 3.0 * S), 3.4 * S, 3.0 * S),
            fill=pal["skin"],
            outline=outline,
            width=max(1, int(0.8 * S)),
        )

        # Head in profile.
        self._side_draw_head(base, head_center, spec, pal, S, pose)

    def _side_draw_head(
        self,
        base: Image.Image,
        c: Point,
        spec: AliceSpec,
        pal: Dict[str, Color],
        S: float,
        pose: AlicePose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Neck.
        self._draw_neck(d, c, spec, pal, S, slant=+1.5)
        # Back hair — drawn as a polygon that hugs the BACK of the
        # head (camera-left) and flows down past the shoulder.
        # Earlier revision used a single ellipse centered slightly
        # left of c, but in profile that ellipse extended forward
        # over the face and the head read as a black egg. The
        # polygon stops at the camera-side hairline (where the bangs
        # take over) so the face has room to be visible.
        back_hair = [
            (
                c[0] - spec.head_w * 0.34 * S,
                c[1] - spec.head_h * 0.46 * S,
            ),  # top-back of skull
            (c[0] + spec.head_w * 0.10 * S, c[1] - spec.head_h * 0.50 * S),  # crown
            (
                c[0] + spec.head_w * 0.20 * S,
                c[1] - spec.head_h * 0.40 * S,
            ),  # top-front (just above bang start)
            (
                c[0] + spec.head_w * 0.04 * S,
                c[1] - spec.head_h * 0.16 * S,
            ),  # behind the temple
            # Down the back of the neck + shoulder.
            (c[0] - spec.head_w * 0.20 * S, c[1] + spec.head_h * 0.10 * S),
            (c[0] - spec.head_w * 0.46 * S, c[1] + spec.head_h * 0.30 * S),
            (
                c[0] - spec.head_w * 0.58 * S,
                c[1] + (spec.head_h * 0.55 + spec.back_hair_h_extra * 0.6) * S,
            ),
            (
                c[0] - spec.head_w * 0.34 * S,
                c[1] + (spec.head_h * 0.65 + spec.back_hair_h_extra * 0.7) * S,
            ),
            (
                c[0] - spec.head_w * 0.18 * S,
                c[1] + (spec.head_h * 0.55 + spec.back_hair_h_extra * 0.5) * S,
            ),
            (c[0] - spec.head_w * 0.10 * S, c[1] + spec.head_h * 0.30 * S),
            (c[0] - spec.head_w * 0.36 * S, c[1] + spec.head_h * 0.10 * S),
            (c[0] - spec.head_w * 0.46 * S, c[1] - spec.head_h * 0.10 * S),
        ]
        d.polygon(back_hair, fill=pal["hair"], outline=outline)
        # Face polygon (forehead → brow → nose → upper lip → chin →
        # jawline back to cheek).
        face = [
            (c[0] - spec.head_w * 0.28 * S, c[1] - spec.head_h * 0.46 * S),
            (c[0] + spec.head_w * 0.26 * S, c[1] - spec.head_h * 0.38 * S),
            (c[0] + spec.head_w * 0.32 * S, c[1] - spec.head_h * 0.14 * S),  # brow
            (c[0] + spec.head_w * 0.44 * S, c[1] - spec.head_h * 0.02 * S),  # nose tip
            (
                c[0] + spec.head_w * 0.32 * S,
                c[1] + spec.head_h * 0.08 * S,
            ),  # under-nose
            (c[0] + spec.head_w * 0.36 * S, c[1] + spec.head_h * 0.20 * S),  # upper lip
            (c[0] + spec.head_w * 0.28 * S, c[1] + spec.head_h * 0.28 * S),  # chin
            (c[0] + spec.head_w * 0.04 * S, c[1] + spec.head_h * 0.32 * S),
            (c[0] - spec.head_w * 0.30 * S, c[1] + spec.head_h * 0.20 * S),
            (c[0] - spec.head_w * 0.36 * S, c[1] - spec.head_h * 0.10 * S),
        ]
        d.polygon(face, fill=pal["skin"], outline=outline)
        # Side bang sweeping across the forehead in profile. Keep it
        # tight to the brow line so the eye remains visible below.
        d.polygon(
            [
                (c[0] - spec.head_w * 0.26 * S, c[1] - spec.head_h * 0.44 * S),
                (c[0] + spec.head_w * 0.24 * S, c[1] - spec.head_h * 0.36 * S),
                (c[0] + spec.head_w * 0.16 * S, c[1] - spec.head_h * 0.26 * S),
                (c[0] - spec.head_w * 0.18 * S, c[1] - spec.head_h * 0.30 * S),
            ],
            fill=pal["hair"],
            outline=outline,
        )
        # Single visible eye.
        eye_x = c[0] + spec.head_w * 0.18 * S
        eye_y = c[1] - spec.head_h * 0.04 * S
        if pose.blink:
            d.line(
                [(eye_x - 1.2 * S, eye_y), (eye_x + 1.2 * S, eye_y)],
                fill=outline,
                width=max(1, int(1.0 * S)),
            )
        else:
            d.ellipse(
                _bbox((eye_x, eye_y), 1.8 * S, 1.3 * S),
                fill=pal["white"],
                outline=outline,
                width=max(1, int(0.7 * S)),
            )
            d.ellipse(
                _bbox((eye_x + 0.2 * S, eye_y), 1.0 * S, 1.2 * S),
                fill=pal["coat_dark"],
                outline=None,
            )
            d.ellipse(_bbox((eye_x + 0.2 * S, eye_y), 0.6 * S, 0.9 * S), fill=outline)
            # Tiny outer eyelash tick.
            d.line(
                [
                    (eye_x + 1.4 * S, eye_y - 1.0 * S),
                    (eye_x + 1.7 * S, eye_y - 1.8 * S),
                ],
                fill=outline,
                width=max(1, int(0.5 * S)),
            )
        # Lip line.
        d.line(
            [
                (c[0] + spec.head_w * 0.32 * S, c[1] + spec.head_h * 0.22 * S),
                (c[0] + spec.head_w * 0.38 * S, c[1] + spec.head_h * 0.22 * S),
            ],
            fill=outline,
            width=max(1, int(0.8 * S)),
        )
        # Forward braid hangs over the camera-side shoulder.
        braid_anchor_x = c[0] + spec.head_w * 0.16 * S
        braid_anchor_y = c[1] + spec.head_h * 0.40 * S
        for i in range(spec.braid_segments):
            t_seg = i / float(spec.braid_segments - 1)
            w = (4.2 - t_seg * 2.4) * S
            dy = (2.0 + i * 5.0) * S
            seg_c = (braid_anchor_x - t_seg * 3.0 * S, braid_anchor_y + dy)
            d.ellipse(
                _bbox(seg_c, w, 3.0 * S),
                fill=pal["hair"],
                outline=outline,
                width=max(1, int(0.9 * S)),
            )
        # Ribbon tip.
        tip_x = braid_anchor_x - 3.0 * S
        tip_y = braid_anchor_y + (2.0 + (spec.braid_segments - 1) * 5.0 + 4.0) * S
        d.rounded_rectangle(
            (tip_x - 3.0 * S, tip_y - 1.5 * S, tip_x + 1.0 * S, tip_y + 1.3 * S),
            radius=0.9 * S,
            fill=pal["hair_band"],
            outline=outline,
            width=max(1, int(0.6 * S)),
        )
