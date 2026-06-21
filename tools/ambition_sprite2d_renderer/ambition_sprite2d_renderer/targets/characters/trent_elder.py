"""Bespoke target for Trent — trusted arbitrator / council elder.

This is a **separate adapter** from `toon_side`, not a preset variant.
The shared toon template (`targets/toon_side.py`) leans on a stick-
figure construction (capsule arms + capsule legs + oval head) that
makes every character read the same way under the costume. Trent
needed a different silhouette philosophy:

  - **Robe-first composition.** The robe is the primary shape, not
    an overlay on top of a humanoid skeleton. Arms emerge from the
    robe; legs are hidden beneath it. The silhouette stops at the
    hem instead of showing feet poking out.
  - **Distinct head geometry.** A slightly elongated head with a
    pronounced jaw extension that makes room for a real flowing
    beard (not just a "chin shadow" tint).
  - **Two-tone draped fabric.** The robe uses front + back panels
    + a lighter inner placket that catches a "light from above"
    suggestion, instead of a single polygon.
  - **No capsule limbs.** Arms are tapered fabric tubes (sleeve →
    cuff → hand), not the toon target's "upper capsule + lower
    capsule + circle for hand."

The target is currently single-archetype (`trent`). The shape
vocabulary (head + jaw + beard + robe + side-fringe + chain of
office) is exposed as constants so a future "council batch 2" can
re-use it for other council-coded characters by overriding palette
and a few proportions.

Animation budget is deliberately small: idle (breath bob), walk
(slow sway as the robe rocks), talk (small head tilt + raised
hand), interact (slight lean forward). The toon target's huge
hair / outfit / prop dispatch tables don't exist here; this is
focused, opinionated geometry.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
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
# Trent's palette echoes the "trent" entry in toon_side.PALETTES but is
# duplicated here so the new target doesn't import toon-target state. If
# a future council character lands, factor this into a shared
# `palettes.py` module — for now, one character means one palette.

TRENT_PALETTE: Dict[str, Color] = {
    "skin": rgba("#C9A78B"),
    "skin_shadow": rgba("#9B7C63"),
    # Beard + side fringe — same cream so they read as the same hair.
    "beard": rgba("#EFE6D2"),
    "beard_shine": rgba("#FAF4E2"),
    "beard_shadow": rgba("#BAB29D"),
    "robe": rgba("#264E32"),
    "robe_dark": rgba("#142A1B"),
    "robe_light": rgba("#3A6B47"),
    "placket": rgba("#1A3B23"),
    "gold": rgba("#CBA653"),
    "gold_dark": rgba("#866820"),
    "white": rgba("#F5EEDA"),
    "outline": rgba("#0C0F0C"),
    "shadow": rgba("#000000", 50),
}


@dataclass(frozen=True)
class TrentSpec:
    target: str
    seed: int
    archetype: str
    name: str
    role: str
    palette_name: str
    # Dimensions are in "design units" where 1 unit = ~1px at 128px
    # canvas. Everything scales linearly with `S` at render time.
    head_w: float = 28.0
    head_h: float = 30.0
    jaw_h: float = 10.0  # jaw extension below face center
    neck_h: float = 4.0
    shoulder_w: float = 36.0
    robe_top_w: float = 32.0  # width at the shoulder yoke
    robe_mid_w: float = 38.0  # width at the waist
    robe_hem_w: float = 48.0  # width at the floor hem (wider = more drape)
    robe_h: float = 78.0  # robe height from shoulder yoke to hem
    arm_len: float = 28.0
    cuff_w: float = 6.5
    beard_w: float = 22.0  # widest part of the beard fan
    beard_h: float = 30.0  # how far the beard hangs below the chin
    fringe_w: float = 5.0  # side-fringe width (half-circle behind ears)


@dataclass
class TrentPose:
    body_bob: float = 0.0
    head_tilt: float = 0.0
    arm_lift: float = 0.0  # 0=arms hanging in sleeves, 1=raised
    hold_scales: bool = True  # is the off-arm holding the balance scales
    talk_open: float = 0.0  # 0=closed mouth, 1=speaking
    blink: bool = False


class TrentElderGenerator:
    """Bespoke geometry for Trent."""

    name = "trent_elder"

    ANIMATIONS: Dict[str, Dict[str, int]] = {
        "idle": {"frames": 6, "duration_ms": 140},
        "walk": {"frames": 6, "duration_ms": 130},
        "talk": {"frames": 6, "duration_ms": 110},
        "interact": {"frames": 6, "duration_ms": 130},
    }

    def sample_spec(self, seed: int, archetype: str = "trent") -> TrentSpec:
        if archetype != "trent":
            raise KeyError(
                f"trent_elder target only ships 'trent' archetype; got {archetype!r}. "
                f"Add a new archetype + per-character proportions if you want to "
                f"re-use the council-elder geometry for another character."
            )
        return TrentSpec(
            target=self.name,
            seed=seed,
            archetype=archetype,
            name="Trent",
            role="npc",
            palette_name="trent",
        )

    # --- pose -----------------------------------------------------------------

    def pose_for_animation(
        self, animation: str, frame_index: int, frame_count: int
    ) -> TrentPose:
        p = TrentPose()
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        wave = math.sin(t * math.tau)
        half_wave = math.sin(t * math.pi)
        if animation == "idle":
            p.body_bob = wave * 0.6
            p.head_tilt = wave * 1.4
            p.blink = (frame_index % frame_count) == frame_count - 1
        elif animation == "walk":
            # Robe sways more than the body for the dignified shuffle.
            p.body_bob = abs(wave) * 1.0
            p.head_tilt = wave * 1.0
        elif animation == "talk":
            p.talk_open = (0.5 + 0.5 * wave) * 0.9
            p.head_tilt = wave * 2.0
            p.arm_lift = max(0.0, half_wave) * 0.6
        elif animation == "interact":
            p.arm_lift = half_wave * 1.0
            p.head_tilt = wave * 0.5
        return p

    # --- rendering ------------------------------------------------------------

    def render_animation_frame(
        self,
        spec: TrentSpec,
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
        pal = TRENT_PALETTE
        p = self.pose_for_animation(animation, frame_index, frame_count)

        # Anchor the figure: feet on the bottom-third of the frame, head
        # slightly above center. The robe drapes down to the hem near
        # the bottom of the canvas, no visible feet (the silhouette
        # stops at the hem). No drop shadow — the in-game renderer
        # composites characters over scene geometry that already
        # provides ground contact.
        cx = 64.0 * S
        hem_y = 116.0 * S
        shoulder_y = (hem_y - spec.robe_h * S) + p.body_bob * S
        head_center = (
            cx + 1.5 * S,
            shoulder_y - spec.head_h * 0.55 * S - spec.neck_h * S,
        )

        # Order: robe back-cuffs → robe body → arms in sleeves → head + beard.
        self._draw_robe(img, cx, shoulder_y, hem_y, spec, pal, S, p)
        self._draw_arms(img, cx, shoulder_y, spec, pal, S, p)
        self._draw_head(img, head_center, spec, pal, S, p)
        # Chain of office over the robe yoke, drawn last so its links
        # sit on top of the placket.
        self._draw_chain(img, cx, shoulder_y, spec, pal, S, p)

        if ss > 1:
            img = img.resize((W, H), Image.LANCZOS)
        return img

    def _draw_robe(
        self,
        base: Image.Image,
        cx: float,
        shoulder_y: float,
        hem_y: float,
        spec: TrentSpec,
        pal: Dict[str, Color],
        S: float,
        pose: TrentPose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Main robe silhouette — a hex-ish shape with shoulder yoke,
        # waist pinch, and flared hem. Pure polygon, no stacked
        # capsules; reads as drapery, not a body.
        top_y = shoulder_y
        waist_y = shoulder_y + spec.robe_h * 0.45 * S
        robe_outline = [
            (cx - spec.robe_top_w * 0.5 * S, top_y),
            (cx + spec.robe_top_w * 0.5 * S, top_y),
            (cx + spec.robe_mid_w * 0.5 * S, waist_y),
            (cx + spec.robe_hem_w * 0.5 * S, hem_y),
            (cx - spec.robe_hem_w * 0.5 * S, hem_y),
            (cx - spec.robe_mid_w * 0.5 * S, waist_y),
        ]
        d.polygon(robe_outline, fill=pal["robe"], outline=outline)
        # Inner-front placket: narrower vertical strip that catches a
        # darker tone — gives the robe a real front-vs-side suggestion.
        placket = [
            (cx - 6.0 * S, top_y + 2.0 * S),
            (cx + 6.0 * S, top_y + 2.0 * S),
            (cx + 5.0 * S, hem_y - 2.0 * S),
            (cx - 5.0 * S, hem_y - 2.0 * S),
        ]
        d.polygon(placket, fill=pal["placket"], outline=None)
        # Two long vertical fold-shadows on the side panels so the
        # fabric reads as draped, not painted.
        for sign in (-1, 1):
            d.line(
                [
                    (cx + sign * spec.robe_mid_w * 0.30 * S, top_y + 6.0 * S),
                    (cx + sign * spec.robe_hem_w * 0.30 * S, hem_y - 2.0 * S),
                ],
                fill=pal["robe_dark"],
                width=max(1, int(1.6 * S)),
            )
        # Subtle highlight along the camera-side edge of the placket
        # so the lighter color survives downsampling.
        d.line(
            [
                (cx + 6.5 * S, top_y + 2.0 * S),
                (cx + 5.5 * S, hem_y - 2.0 * S),
            ],
            fill=pal["robe_light"],
            width=max(1, int(1.0 * S)),
        )
        # Hem trim — a gold band along the floor edge of the robe.
        d.rounded_rectangle(
            (
                cx - spec.robe_hem_w * 0.5 * S,
                hem_y - 5.0 * S,
                cx + spec.robe_hem_w * 0.5 * S,
                hem_y - 1.0 * S,
            ),
            radius=2.0 * S,
            fill=pal["gold"],
            outline=outline,
            width=max(1, int(0.9 * S)),
        )
        # Three small gold studs along the hem.
        for stud_x in (-12.0, 0.0, 12.0):
            d.ellipse(
                _bbox((cx + stud_x * S, hem_y - 3.0 * S), 1.6 * S, 1.6 * S),
                fill=pal["gold_dark"],
                outline=outline,
                width=max(1, int(0.6 * S)),
            )
        # Yoke band across the shoulders — separate from the placket so
        # the chain of office has a structured surface to sit on.
        d.rounded_rectangle(
            (
                cx - spec.robe_top_w * 0.50 * S,
                top_y - 1.0 * S,
                cx + spec.robe_top_w * 0.50 * S,
                top_y + 6.0 * S,
            ),
            radius=2.0 * S,
            fill=pal["robe_dark"],
            outline=outline,
            width=max(1, int(1.0 * S)),
        )

    def _draw_arms(
        self,
        base: Image.Image,
        cx: float,
        shoulder_y: float,
        spec: TrentSpec,
        pal: Dict[str, Color],
        S: float,
        pose: TrentPose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Both arms are tapered sleeves (wide at shoulder, narrow at
        # cuff). The near arm (camera-right, +x) optionally holds the
        # balance scales; the far arm hangs at the side, partially
        # tucked behind the robe.
        # Far arm — hanging at the side.
        far_shoulder = (cx - spec.shoulder_w * 0.40 * S, shoulder_y + 4.0 * S)
        far_cuff = (
            cx - spec.shoulder_w * 0.36 * S,
            shoulder_y + spec.arm_len * S + 2.0 * S,
        )
        far_sleeve = [
            (far_shoulder[0] - 5.0 * S, far_shoulder[1]),
            (far_shoulder[0] + 5.0 * S, far_shoulder[1]),
            (far_cuff[0] + spec.cuff_w * 0.5 * S, far_cuff[1]),
            (far_cuff[0] - spec.cuff_w * 0.5 * S, far_cuff[1]),
        ]
        d.polygon(far_sleeve, fill=pal["robe_dark"], outline=outline)
        # Far cuff: a gold band where the sleeve ends.
        d.rounded_rectangle(
            (
                far_cuff[0] - spec.cuff_w * 0.6 * S,
                far_cuff[1] - 1.5 * S,
                far_cuff[0] + spec.cuff_w * 0.6 * S,
                far_cuff[1] + 1.5 * S,
            ),
            radius=1.4 * S,
            fill=pal["gold_dark"],
            outline=outline,
            width=max(1, int(0.7 * S)),
        )
        # A flash of hand peeking out of the far cuff.
        d.ellipse(
            _bbox((far_cuff[0], far_cuff[1] + 3.5 * S), 3.5 * S, 3.0 * S),
            fill=pal["skin"],
            outline=outline,
            width=max(1, int(0.7 * S)),
        )

        # Near arm — slightly raised (arm_lift drives elbow + cuff y)
        # and ending in the balance-scales hold by default.
        lift = pose.arm_lift
        near_shoulder = (cx + spec.shoulder_w * 0.36 * S, shoulder_y + 4.0 * S)
        near_cuff_y = shoulder_y + spec.arm_len * S + 4.0 * S - lift * 10.0 * S
        near_cuff_x = cx + spec.shoulder_w * 0.32 * S + lift * 4.0 * S
        near_cuff = (near_cuff_x, near_cuff_y)
        # The sleeve curves slightly forward — a 4-point polygon with
        # the lower edge skewed by `lift` for a raised-arm feel.
        near_sleeve = [
            (near_shoulder[0] - 4.0 * S, near_shoulder[1]),
            (near_shoulder[0] + 6.0 * S, near_shoulder[1]),
            (near_cuff[0] + spec.cuff_w * 0.55 * S, near_cuff[1]),
            (near_cuff[0] - spec.cuff_w * 0.55 * S, near_cuff[1]),
        ]
        d.polygon(near_sleeve, fill=pal["robe"], outline=outline)
        # Sleeve highlight along the upper edge — fabric catching light.
        d.line(
            [
                (near_shoulder[0] - 3.0 * S, near_shoulder[1] + 1.0 * S),
                (near_cuff[0] - spec.cuff_w * 0.30 * S, near_cuff[1] - 1.0 * S),
            ],
            fill=pal["robe_light"],
            width=max(1, int(1.0 * S)),
        )
        # Near cuff: gold band, slightly bigger than the far cuff.
        d.rounded_rectangle(
            (
                near_cuff[0] - spec.cuff_w * 0.65 * S,
                near_cuff[1] - 1.5 * S,
                near_cuff[0] + spec.cuff_w * 0.65 * S,
                near_cuff[1] + 1.8 * S,
            ),
            radius=1.5 * S,
            fill=pal["gold"],
            outline=outline,
            width=max(1, int(0.8 * S)),
        )
        # Hand emerging from the near cuff.
        hand_c = (near_cuff[0] + 1.0 * S, near_cuff[1] + 4.0 * S)
        d.ellipse(
            _bbox(hand_c, 4.0 * S, 3.4 * S),
            fill=pal["skin"],
            outline=outline,
            width=max(1, int(0.8 * S)),
        )
        # The balance scales hanging from the hand (optional via pose).
        if pose.hold_scales:
            self._draw_scales(base, hand_c, pal, S)

    def _draw_scales(
        self, base: Image.Image, hand: Point, pal: Dict[str, Color], S: float
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Pillar rising from the hand.
        pillar_top = (hand[0] + 0.5 * S, hand[1] - 16.0 * S)
        d.line([hand, pillar_top], fill=pal["gold_dark"], width=max(1, int(1.6 * S)))
        # Beam.
        beam_a = (pillar_top[0] - 8.0 * S, pillar_top[1])
        beam_b = (pillar_top[0] + 8.0 * S, pillar_top[1])
        d.line([beam_a, beam_b], fill=pal["gold"], width=max(1, int(1.6 * S)))
        # Finial.
        d.ellipse(
            _bbox(pillar_top, 2.0 * S, 2.0 * S),
            fill=pal["gold"],
            outline=outline,
            width=max(1, int(0.6 * S)),
        )
        # Pans (arcs hanging from the beam tips).
        for end in (beam_a, beam_b):
            pan_anchor = (end[0], end[1] + 4.0 * S)
            d.line([end, pan_anchor], fill=pal["gold_dark"], width=max(1, int(0.7 * S)))
            d.arc(
                (
                    end[0] - 4.0 * S,
                    pan_anchor[1] - 0.5 * S,
                    end[0] + 4.0 * S,
                    pan_anchor[1] + 5.0 * S,
                ),
                start=0,
                end=180,
                fill=outline,
                width=max(1, int(1.0 * S)),
            )
            d.line(
                [
                    (end[0] - 4.0 * S, pan_anchor[1] + 0.5 * S),
                    (end[0] + 4.0 * S, pan_anchor[1] + 0.5 * S),
                ],
                fill=pal["gold_dark"],
                width=max(1, int(0.8 * S)),
            )

    def _draw_head(
        self,
        base: Image.Image,
        c: Point,
        spec: TrentSpec,
        pal: Dict[str, Color],
        S: float,
        pose: TrentPose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]

        # Side fringe of hair (cream/white) wrapping the back of the
        # head — drawn first so the face sits on top.
        d.pieslice(
            _bbox(
                (c[0] - 0.5 * S, c[1] + spec.head_h * 0.04 * S),
                (spec.head_w + spec.fringe_w * 2.0) * S,
                spec.head_h * 0.46 * S,
            ),
            start=180,
            end=360,
            fill=pal["beard"],
            outline=outline,
            width=max(1, int(1.0 * S)),
        )
        # Small tuft just behind the camera-side ear for silhouette
        # interest (matches the toon `clean_bald` cue, but rounder).
        d.ellipse(
            _bbox(
                (c[0] + spec.head_w * 0.42 * S, c[1] + spec.head_h * 0.10 * S),
                3.6 * S,
                3.0 * S,
            ),
            fill=pal["beard"],
            outline=outline,
            width=max(1, int(0.8 * S)),
        )

        # Head with a longer-than-wide oval + jaw extension. The jaw
        # tapers to a slight point so the beard has a triangular base
        # to grow from.
        head_outline = [
            (c[0] - spec.head_w * 0.50 * S, c[1] - spec.head_h * 0.30 * S),
            (c[0] - spec.head_w * 0.48 * S, c[1] - spec.head_h * 0.50 * S),
            (c[0] - spec.head_w * 0.24 * S, c[1] - spec.head_h * 0.58 * S),
            (c[0] + spec.head_w * 0.24 * S, c[1] - spec.head_h * 0.58 * S),
            (c[0] + spec.head_w * 0.48 * S, c[1] - spec.head_h * 0.50 * S),
            (c[0] + spec.head_w * 0.50 * S, c[1] - spec.head_h * 0.20 * S),
            # Jaw — tapers in below the cheek line.
            (c[0] + spec.head_w * 0.40 * S, c[1] + spec.head_h * 0.10 * S),
            (c[0] + spec.head_w * 0.18 * S, c[1] + spec.head_h * 0.20 * S),
            (c[0], c[1] + spec.head_h * 0.22 * S + spec.jaw_h * 0.2 * S),
            (c[0] - spec.head_w * 0.18 * S, c[1] + spec.head_h * 0.20 * S),
            (c[0] - spec.head_w * 0.40 * S, c[1] + spec.head_h * 0.10 * S),
        ]
        d.polygon(head_outline, fill=pal["skin"], outline=outline)

        # Cheekbone shadow on the camera-far side — gives the head
        # planar definition the toon-oval never gets.
        d.polygon(
            [
                (c[0] - spec.head_w * 0.42 * S, c[1] - spec.head_h * 0.20 * S),
                (c[0] - spec.head_w * 0.12 * S, c[1] - spec.head_h * 0.20 * S),
                (c[0] - spec.head_w * 0.22 * S, c[1] + spec.head_h * 0.06 * S),
                (c[0] - spec.head_w * 0.40 * S, c[1] - spec.head_h * 0.02 * S),
            ],
            fill=pal["skin_shadow"],
            outline=None,
        )

        # Brow line — a thin dark stroke setting up the eye sockets.
        d.line(
            [
                (c[0] - spec.head_w * 0.30 * S, c[1] - spec.head_h * 0.16 * S),
                (c[0] + spec.head_w * 0.32 * S, c[1] - spec.head_h * 0.18 * S),
            ],
            fill=outline,
            width=max(1, int(0.9 * S)),
        )
        # Eyes — recessed, dignified. Smaller than the toon target's
        # cartoon eyes; two short oval pupils set under the brow.
        eye_y = c[1] - spec.head_h * 0.06 * S
        if pose.blink:
            d.line(
                [
                    (c[0] - spec.head_w * 0.20 * S, eye_y),
                    (c[0] - spec.head_w * 0.06 * S, eye_y),
                ],
                fill=outline,
                width=max(1, int(1.1 * S)),
            )
            d.line(
                [
                    (c[0] + spec.head_w * 0.06 * S, eye_y),
                    (c[0] + spec.head_w * 0.22 * S, eye_y),
                ],
                fill=outline,
                width=max(1, int(1.1 * S)),
            )
        else:
            for ex in (-spec.head_w * 0.14, spec.head_w * 0.14):
                d.ellipse(
                    _bbox((c[0] + ex * S, eye_y), 2.4 * S, 1.6 * S),
                    fill=pal["white"],
                    outline=outline,
                    width=max(1, int(0.7 * S)),
                )
                d.ellipse(
                    _bbox((c[0] + (ex + 0.2) * S, eye_y), 1.2 * S, 1.4 * S),
                    fill=outline,
                )
        # Nose — a small wedge below and to the camera-right.
        d.polygon(
            [
                (c[0] + 1.0 * S, c[1] - spec.head_h * 0.04 * S),
                (c[0] + 3.5 * S, c[1] + spec.head_h * 0.04 * S),
                (c[0] + 1.0 * S, c[1] + spec.head_h * 0.08 * S),
            ],
            fill=pal["skin_shadow"],
            outline=None,
        )

        # The BEARD — a flowing two-tone shape that hangs from the
        # jaw to mid-chest. This is the silhouette feature the toon
        # template couldn't deliver: an actual integrated beard,
        # not a chin shadow.
        chin = (c[0], c[1] + spec.head_h * 0.24 * S + spec.jaw_h * 0.2 * S)
        beard_outline = [
            (chin[0] - spec.head_w * 0.36 * S, c[1] + spec.head_h * 0.08 * S),
            (chin[0] - spec.beard_w * 0.55 * S, chin[1] + spec.beard_h * 0.30 * S),
            (chin[0] - spec.beard_w * 0.42 * S, chin[1] + spec.beard_h * 0.78 * S),
            (chin[0] - spec.beard_w * 0.12 * S, chin[1] + spec.beard_h * 1.00 * S),
            (chin[0] + spec.beard_w * 0.12 * S, chin[1] + spec.beard_h * 1.00 * S),
            (chin[0] + spec.beard_w * 0.42 * S, chin[1] + spec.beard_h * 0.78 * S),
            (chin[0] + spec.beard_w * 0.55 * S, chin[1] + spec.beard_h * 0.30 * S),
            (chin[0] + spec.head_w * 0.34 * S, c[1] + spec.head_h * 0.06 * S),
        ]
        d.polygon(beard_outline, fill=pal["beard"], outline=outline)
        # Beard center crease — a darker vertical taper that breaks
        # the beard into two combed halves.
        d.polygon(
            [
                (chin[0] - 1.0 * S, chin[1]),
                (chin[0] + 1.0 * S, chin[1]),
                (chin[0] + 0.5 * S, chin[1] + spec.beard_h * 0.95 * S),
                (chin[0] - 0.5 * S, chin[1] + spec.beard_h * 0.95 * S),
            ],
            fill=pal["beard_shadow"],
            outline=None,
        )
        # A few combed-strand highlights so the beard doesn't read as
        # a single beige blob.
        for sign in (-1, 1):
            for i, (dx, dy_start, dy_end) in enumerate(
                (
                    (4.0, 0.10, 0.75),
                    (7.0, 0.18, 0.65),
                    (10.0, 0.28, 0.55),
                )
            ):
                start = (
                    chin[0] + sign * dx * 0.6 * S,
                    chin[1] + spec.beard_h * dy_start * S,
                )
                end = (
                    chin[0] + sign * (dx + 1.2) * 0.6 * S,
                    chin[1] + spec.beard_h * dy_end * S,
                )
                d.line(
                    [start, end], fill=pal["beard_shine"], width=max(1, int(0.7 * S))
                )
        # Mustache — drapes over the upper lip into the top of the
        # beard, with a small notch in the middle.
        mustache = [
            (c[0] - spec.head_w * 0.22 * S, c[1] + spec.head_h * 0.10 * S),
            (c[0] - spec.head_w * 0.04 * S, c[1] + spec.head_h * 0.16 * S),
            (c[0] - 1.0 * S, c[1] + spec.head_h * 0.12 * S),
            (c[0] + 1.0 * S, c[1] + spec.head_h * 0.12 * S),
            (c[0] + spec.head_w * 0.06 * S, c[1] + spec.head_h * 0.16 * S),
            (c[0] + spec.head_w * 0.24 * S, c[1] + spec.head_h * 0.10 * S),
            (c[0] + spec.head_w * 0.20 * S, c[1] + spec.head_h * 0.20 * S),
            (c[0] - spec.head_w * 0.20 * S, c[1] + spec.head_h * 0.20 * S),
        ]
        d.polygon(mustache, fill=pal["beard"], outline=outline)

        # Mouth — only visible during talk. A small dark oval between
        # the mustache and the beard center crease.
        if pose.talk_open > 0.2:
            mouth_y = c[1] + spec.head_h * 0.22 * S
            mw = (1.6 + pose.talk_open * 1.4) * S
            mh = (1.0 + pose.talk_open * 1.6) * S
            d.ellipse(_bbox((c[0], mouth_y), mw, mh), fill=outline)

    def _draw_chain(
        self,
        base: Image.Image,
        cx: float,
        shoulder_y: float,
        spec: TrentSpec,
        pal: Dict[str, Color],
        S: float,
        pose: TrentPose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Chain of office — a U-shape of small gold links spanning the
        # robe yoke, with a larger pendant medallion centered on the
        # placket.
        chain_top = shoulder_y + 4.0 * S
        chain_dip = shoulder_y + 14.0 * S
        # Render the chain as a sequence of small filled ellipses
        # along a parabolic-ish curve from one shoulder to the other.
        n_links = 11
        for i in range(n_links):
            t = i / float(n_links - 1)
            # Bezier-ish: parabola dip.
            x = cx + (t - 0.5) * spec.robe_top_w * 0.82 * S
            y_off = (1.0 - (2.0 * t - 1.0) ** 2) * (chain_dip - chain_top)
            y = chain_top + y_off
            d.ellipse(
                _bbox((x, y), 1.6 * S, 1.6 * S),
                fill=pal["gold"],
                outline=outline,
                width=max(1, int(0.5 * S)),
            )
        # Pendant medallion at the bottom of the curve.
        med_c = (cx, chain_dip + 2.0 * S)
        d.ellipse(
            _bbox(med_c, 4.4 * S, 4.4 * S),
            fill=pal["gold_dark"],
            outline=outline,
            width=max(1, int(0.9 * S)),
        )
        d.ellipse(_bbox(med_c, 2.4 * S, 2.4 * S), fill=pal["gold"], outline=None)
        # A small motif inside the medallion — a simple set of balance
        # scales suggestion (two dots flanking a vertical tick).
        d.line(
            [(med_c[0], med_c[1] - 1.4 * S), (med_c[0], med_c[1] + 1.4 * S)],
            fill=pal["outline"],
            width=max(1, int(0.7 * S)),
        )
        d.ellipse(
            _bbox((med_c[0] - 1.4 * S, med_c[1] + 0.4 * S), 0.8 * S, 0.8 * S),
            fill=pal["outline"],
        )
        d.ellipse(
            _bbox((med_c[0] + 1.4 * S, med_c[1] + 0.4 * S), 0.8 * S, 0.8 * S),
            fill=pal["outline"],
        )
