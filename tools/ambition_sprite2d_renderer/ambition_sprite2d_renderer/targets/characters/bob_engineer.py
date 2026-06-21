"""Bespoke target for Bob — practical key/hardware engineer.

This is the second bespoke template after `trent_elder`. The design
goals carry over (no stick-figure capsule limbs, integrated rather
than applied costume) and add one new capability that Trent didn't
need:

  - **Multi-view rendering.** Bob is drawn in three distinct views
    depending on the animation: a default **three-quarter** view
    (camera-right facing, used for canonical previews + idle), a
    **side profile** view (used for walking — feet pointing forward,
    one shoulder forward, head in pure profile), and a **front**
    view (used for talking + interact — Bob looks straight at the
    camera). The animation table picks the view; per-view draw
    functions handle the geometry differences.

Improvements layered on top of the trent_elder lessons:

  - **Smoother head.** Trent's polygonal head was a touch flat-
    topped. Bob's head is an ellipse with separate jaw + chin
    primitives so the silhouette has a softer organic read.
  - **Visible legs.** Bob is a workshop figure, not a robed elder.
    He gets actual leg primitives below the vest, ending in
    workboots — not a stick-figure capsule pair, but two segmented
    tapered shapes with a knee bend in walk frames.
  - **Tool belt.** A wrap-around belt with three hanging tools
    (key ring, wrench, hammer) — gives him a distinct silhouette
    at the waist that none of the toon or trent_elder figures have.
  - **Per-view face geometry.** Front-view eyes vs side-view eyes
    vs three-quarter eyes are all drawn differently rather than
    hacking the same eye block. Profile face has only one visible
    eye, a clearly drawn ear, and a forward-pointing nose; front
    face is symmetric.

Single-archetype today (`bob`). The shape vocabulary (head + jaw +
vest + belt + legs + boots) is exposed as constants so future
"engineer batch 2" characters could reuse it.
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

BOB_PALETTE: Dict[str, Color] = {
    # Deeper warm-olive skin so Bob reads as distinctly different
    # from Alice's lighter peach (#E5C5A6) and Trent's mid-tan
    # (#C9A78B). The skin_shadow uses the same hue rotated darker
    # so the 5 o'clock stubble has obvious contrast against the
    # face without going purple-grey.
    "skin": rgba("#B58968"),
    "skin_shadow": rgba("#7E5A3C"),
    "hair": rgba("#3D2B22"),
    "hair_shine": rgba("#6A4B3A"),
    # Workshop vest — warm tan over a slate-blue tee.
    "vest": rgba("#9D7548"),
    "vest_dark": rgba("#6A4C2A"),
    "vest_light": rgba("#B98D60"),
    "tee": rgba("#3D5A78"),
    "tee_dark": rgba("#243A53"),
    # Safety-yellow reflective stripe + accents.
    "hi_vis": rgba("#F2C752"),
    "hi_vis_dark": rgba("#A47616"),
    # Trousers + boots in grimy work tones.
    "pants": rgba("#2D2B2A"),
    "pants_shadow": rgba("#1B1A19"),
    "boot": rgba("#1A1614"),
    "boot_sole": rgba("#0B0908"),
    # Tool belt + tools (brass and steel).
    "leather": rgba("#5A3C20"),
    "leather_dark": rgba("#2F1E0E"),
    "steel": rgba("#B0B2B6"),
    "steel_dark": rgba("#5B5C60"),
    "brass": rgba("#D89A3A"),
    "brass_dark": rgba("#8C5E18"),
    "white": rgba("#FBF0DC"),
    "outline": rgba("#1A130E"),
    "shadow": rgba("#000000", 50),
}


class BobView(str, Enum):
    """Which side of Bob is facing the camera."""

    THREE_QUARTER = "three_quarter"
    SIDE = "side"
    FRONT = "front"


# Each animation locks a view; the runtime never needs to ask the
# question "which way is Bob facing in this clip?" — the view is
# encoded into the animation name + this table.
ANIMATION_VIEWS: Dict[str, BobView] = {
    "idle": BobView.THREE_QUARTER,
    "walk": BobView.SIDE,  # ← side profile walking
    "talk": BobView.FRONT,  # ← facing camera
    "interact": BobView.FRONT,  # ← facing camera, examining
    "idle_front": BobView.FRONT,  # ← extra: front idle for dialog
    "idle_side": BobView.SIDE,  # ← extra: side idle for crowd shots
}


@dataclass(frozen=True)
class BobSpec:
    target: str
    seed: int
    archetype: str
    name: str
    role: str
    palette_name: str
    # All measurements in design units (~1px at 128px canvas).
    head_w: float = 26.0
    head_h: float = 28.0
    head_depth: float = 22.0  # used for side-view head ellipse
    jaw_h: float = 6.0
    # Head sits ~2.5 design units above the shoulder yoke; a short
    # neck polygon fills that gap. Was 4.0 + 0.55 (a 5+ unit gap
    # that read as a floating head); 2.5 + 0.50 keeps a visible
    # neck strip without making the head look attached at the
    # collarbone.
    neck_h: float = 2.5
    head_anchor: float = 0.50  # fraction of head_h above shoulder_y
    neck_w: float = 7.0  # width of the visible neck skin polygon
    shoulder_w: float = 36.0
    chest_h: float = 20.0
    vest_h: float = 26.0
    waist_w: float = 26.0
    hip_w: float = 28.0
    leg_h: float = 28.0
    leg_w: float = 8.0
    boot_w: float = 11.0
    boot_h: float = 6.0
    arm_len: float = 24.0
    arm_w: float = 6.0
    cuff_w: float = 5.5


@dataclass
class BobPose:
    view: BobView = BobView.THREE_QUARTER
    body_bob: float = 0.0
    head_tilt: float = 0.0
    arm_lift: float = 0.0
    # Walk-specific: step phase in [-1, +1]. +1 means the camera-side
    # leg is forward, -1 means back. Used only by the SIDE view.
    step_phase: float = 0.0
    talk_open: float = 0.0
    blink: bool = False
    hold_keys: bool = True


class BobEngineerGenerator:
    """Bespoke geometry for Bob with three view modes."""

    name = "bob_engineer"

    ANIMATIONS: Dict[str, Dict[str, int]] = {
        "idle": {"frames": 6, "duration_ms": 140},
        "walk": {"frames": 8, "duration_ms": 100},
        "talk": {"frames": 6, "duration_ms": 110},
        "interact": {"frames": 6, "duration_ms": 130},
        "idle_front": {"frames": 6, "duration_ms": 140},
        "idle_side": {"frames": 6, "duration_ms": 140},
    }

    def sample_spec(self, seed: int, archetype: str = "bob") -> BobSpec:
        if archetype != "bob":
            raise KeyError(
                f"bob_engineer target only ships 'bob' archetype; got {archetype!r}. "
                f"Add a per-character archetype + proportions if you want to "
                f"re-use the engineer geometry for another character."
            )
        return BobSpec(
            target=self.name,
            seed=seed,
            archetype=archetype,
            name="Bob",
            role="npc",
            palette_name="bob",
        )

    # --- pose -----------------------------------------------------------------

    def pose_for_animation(
        self, animation: str, frame_index: int, frame_count: int
    ) -> BobPose:
        view = ANIMATION_VIEWS.get(animation, BobView.THREE_QUARTER)
        p = BobPose(view=view)
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        wave = math.sin(t * math.tau)
        if animation == "idle" or animation == "idle_front" or animation == "idle_side":
            p.body_bob = wave * 0.6
            p.head_tilt = wave * 1.4
            # Blink on the last frame of each loop.
            p.blink = frame_index == frame_count - 1
        elif animation == "walk":
            # SIDE view walk — step_phase drives leg + arm swing.
            # Offset the phase by a quarter cycle so frame 0 is at
            # a clear stride peak (step_phase = sin(pi/2) = +1), not
            # the neutral midpoint (step_phase = sin(0) = 0). Spot
            # checks of frame 0 should show a real walking pose so
            # the spritesheet preview is honest about motion.
            phase_wave = math.sin((t + 0.25) * math.tau)
            p.step_phase = phase_wave
            p.body_bob = abs(phase_wave) * 1.0
            # Subtle counter-tilt of head with the gait.
            p.head_tilt = -phase_wave * 1.0
        elif animation == "talk":
            p.talk_open = (0.5 + 0.5 * wave) * 0.9
            p.head_tilt = wave * 1.2
            p.arm_lift = max(0.0, math.sin(t * math.pi)) * 0.5
        elif animation == "interact":
            p.arm_lift = math.sin(t * math.pi) * 1.0
            p.head_tilt = wave * 0.4
        return p

    # --- top-level frame ------------------------------------------------------

    def render_animation_frame(
        self,
        spec: BobSpec,
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
        pal = BOB_PALETTE
        pose = self.pose_for_animation(animation, frame_index, frame_count)

        cx = 64.0 * S
        feet_y = 116.0 * S + pose.body_bob * S
        # No drop shadow — the in-game renderer composites characters
        # over scene geometry that already provides ground contact,
        # and the baked-in shadow ellipse fought camera angles and
        # transparent backgrounds for review previews.

        if pose.view == BobView.SIDE:
            self._render_side(img, cx, feet_y, spec, pal, S, pose)
        elif pose.view == BobView.FRONT:
            self._render_front(img, cx, feet_y, spec, pal, S, pose)
        else:
            self._render_three_quarter(img, cx, feet_y, spec, pal, S, pose)

        if ss > 1:
            img = img.resize((W, H), Image.LANCZOS)
        return img

    # ─────────────────────────────────────────────────────────────────
    # THREE-QUARTER view — default canonical pose, idle.
    # ─────────────────────────────────────────────────────────────────

    def _render_three_quarter(
        self,
        base: Image.Image,
        cx: float,
        feet_y: float,
        spec: BobSpec,
        pal: Dict[str, Color],
        S: float,
        pose: BobPose,
    ) -> None:
        # Anchor stack from feet up.
        boot_top_y = feet_y - spec.boot_h * S
        pants_top_y = boot_top_y - spec.leg_h * S
        waist_y = pants_top_y - 2.0 * S
        vest_top_y = waist_y - spec.vest_h * S
        shoulder_y = vest_top_y - 4.0 * S
        head_center = (
            cx + 2.0 * S,
            shoulder_y - spec.head_h * spec.head_anchor * S - spec.neck_h * S,
        )

        self._tq_draw_legs(
            base, cx, pants_top_y, boot_top_y, feet_y, spec, pal, S, pose
        )
        self._tq_draw_arms_back(base, cx, shoulder_y, spec, pal, S, pose)
        self._tq_draw_vest(base, cx, vest_top_y, waist_y, spec, pal, S, pose)
        self._tq_draw_belt(base, cx, waist_y, spec, pal, S, pose)
        self._tq_draw_arms_front(base, cx, shoulder_y, spec, pal, S, pose)
        self._tq_draw_head(base, head_center, spec, pal, S, pose)

    def _tq_draw_legs(
        self,
        base: Image.Image,
        cx: float,
        pants_top_y: float,
        boot_top_y: float,
        feet_y: float,
        spec: BobSpec,
        pal: Dict[str, Color],
        S: float,
        pose: BobPose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Two tapered leg shapes — narrower at the boot, wider at the
        # hip — in pants color, with a subtle inner-shadow seam.
        # Spread the legs so they don't visually merge at the centerline:
        # was hip * 0.22 + leg_w 0.5 (legs touched at x=0); bumped to
        # hip * 0.38 so there's a clear gap between them.
        for sign, dx in zip((-1, 1), (-1, 1)):
            hip = (cx + dx * spec.hip_w * 0.38 * S, pants_top_y)
            ankle = (cx + dx * spec.boot_w * 0.28 * S, boot_top_y)
            leg = [
                (hip[0] - spec.leg_w * 0.5 * S, hip[1]),
                (hip[0] + spec.leg_w * 0.5 * S, hip[1]),
                (ankle[0] + spec.boot_w * 0.5 * S, ankle[1]),
                (ankle[0] - spec.boot_w * 0.5 * S, ankle[1]),
            ]
            d.polygon(leg, fill=pal["pants"], outline=outline)
            # Inner-leg shadow strip.
            d.line(
                [(hip[0], hip[1] + 1.0 * S), (ankle[0], ankle[1] - 1.0 * S)],
                fill=pal["pants_shadow"],
                width=max(1, int(0.7 * S)),
            )
            # Boot — a chunky rounded rectangle with a darker sole.
            boot_c = (ankle[0], (ankle[1] + feet_y) * 0.5)
            d.rounded_rectangle(
                (
                    boot_c[0] - spec.boot_w * 0.55 * S,
                    ankle[1] - 0.5 * S,
                    boot_c[0] + spec.boot_w * 0.55 * S,
                    feet_y,
                ),
                radius=2.0 * S,
                fill=pal["boot"],
                outline=outline,
                width=max(1, int(1.0 * S)),
            )
            # Boot sole as a thinner band at the very bottom.
            d.rectangle(
                (
                    boot_c[0] - spec.boot_w * 0.55 * S,
                    feet_y - 1.6 * S,
                    boot_c[0] + spec.boot_w * 0.55 * S,
                    feet_y,
                ),
                fill=pal["boot_sole"],
                outline=outline,
                width=max(1, int(0.7 * S)),
            )
            # Laces — two short horizontal ticks across the boot face.
            for ly in (3.0, 1.2):
                d.line(
                    [
                        (boot_c[0] - 2.4 * S, feet_y - ly * S),
                        (boot_c[0] + 2.4 * S, feet_y - ly * S),
                    ],
                    fill=pal["leather"],
                    width=max(1, int(0.6 * S)),
                )

    def _tq_draw_arms_back(
        self,
        base: Image.Image,
        cx: float,
        shoulder_y: float,
        spec: BobSpec,
        pal: Dict[str, Color],
        S: float,
        pose: BobPose,
    ) -> None:
        # Back arm (camera-far). Hangs at the side, mostly hidden
        # behind the vest. Visible only as a slim sleeve + hand.
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        sx = cx - spec.shoulder_w * 0.42 * S
        sy = shoulder_y + 2.0 * S
        ex = sx - 1.0 * S
        ey = sy + spec.arm_len * S
        sleeve = [
            (sx - 3.0 * S, sy),
            (sx + 3.0 * S, sy),
            (ex + spec.cuff_w * 0.5 * S, ey),
            (ex - spec.cuff_w * 0.5 * S, ey),
        ]
        d.polygon(sleeve, fill=pal["tee_dark"], outline=outline)
        # Hand peek.
        d.ellipse(
            _bbox((ex, ey + 3.0 * S), 3.4 * S, 3.0 * S),
            fill=pal["skin"],
            outline=outline,
            width=max(1, int(0.8 * S)),
        )

    def _tq_draw_vest(
        self,
        base: Image.Image,
        cx: float,
        vest_top_y: float,
        waist_y: float,
        spec: BobSpec,
        pal: Dict[str, Color],
        S: float,
        pose: BobPose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Tee underneath — visible at neck + shoulders + sleeves stubs.
        tee = [
            (cx - spec.shoulder_w * 0.50 * S, vest_top_y - 2.0 * S),
            (cx + spec.shoulder_w * 0.42 * S, vest_top_y - 2.0 * S),
            (cx + spec.waist_w * 0.50 * S, waist_y),
            (cx - spec.waist_w * 0.50 * S, waist_y),
        ]
        d.polygon(tee, fill=pal["tee"], outline=outline)
        # Vest open at the front — drawn as two side panels, leaving
        # a 6-px-wide strip of the tee visible down the center.
        for sign in (-1, 1):
            x_outer = cx + sign * spec.shoulder_w * 0.48 * S
            x_inner = cx + sign * 3.0 * S
            panel = [
                (x_outer, vest_top_y),
                (x_inner, vest_top_y + 1.0 * S),
                (cx + sign * spec.waist_w * 0.30 * S, waist_y),
                (cx + sign * spec.waist_w * 0.48 * S, waist_y),
            ]
            d.polygon(panel, fill=pal["vest"], outline=outline)
            # Reflective hi-vis stripe across each panel at chest level.
            stripe_y = vest_top_y + spec.vest_h * 0.40 * S
            stripe_cx = cx + sign * spec.shoulder_w * 0.28 * S
            d.rectangle(
                (
                    stripe_cx - 4.0 * S,
                    stripe_y,
                    stripe_cx + 4.0 * S,
                    stripe_y + 2.4 * S,
                ),
                fill=pal["hi_vis"],
                outline=outline,
                width=max(1, int(0.6 * S)),
            )
        # Vest lapel highlight (camera-right side).
        d.line(
            [
                (cx + 3.0 * S, vest_top_y + 1.0 * S),
                (cx + spec.waist_w * 0.32 * S, waist_y - 1.0 * S),
            ],
            fill=pal["vest_light"],
            width=max(1, int(0.9 * S)),
        )
        # Chest patch pocket on the camera-right panel.
        d.rounded_rectangle(
            (
                cx + 6.0 * S,
                vest_top_y + spec.vest_h * 0.18 * S,
                cx + 13.0 * S,
                vest_top_y + spec.vest_h * 0.42 * S,
            ),
            radius=1.2 * S,
            fill=pal["vest_dark"],
            outline=outline,
            width=max(1, int(0.6 * S)),
        )
        # Tee crew-neck band (small dark arc at the throat).
        d.arc(
            (cx - 6.0 * S, vest_top_y - 3.0 * S, cx + 8.0 * S, vest_top_y + 3.0 * S),
            start=10,
            end=170,
            fill=pal["tee_dark"],
            width=max(1, int(1.0 * S)),
        )

    def _tq_draw_belt(
        self,
        base: Image.Image,
        cx: float,
        waist_y: float,
        spec: BobSpec,
        pal: Dict[str, Color],
        S: float,
        pose: BobPose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Belt strap.
        d.rounded_rectangle(
            (
                cx - spec.waist_w * 0.52 * S,
                waist_y - 0.5 * S,
                cx + spec.waist_w * 0.52 * S,
                waist_y + 3.5 * S,
            ),
            radius=1.4 * S,
            fill=pal["leather"],
            outline=outline,
            width=max(1, int(0.9 * S)),
        )
        # Belt buckle (steel).
        d.rectangle(
            (cx - 2.6 * S, waist_y - 0.3 * S, cx + 2.6 * S, waist_y + 3.3 * S),
            fill=pal["steel"],
            outline=outline,
            width=max(1, int(0.7 * S)),
        )
        # Three hanging tools across the belt: key ring (camera-right),
        # wrench (center), small hammer (camera-left).
        self._draw_belt_keyring(
            d, cx + spec.waist_w * 0.30 * S, waist_y + 3.5 * S, S, pal
        )
        self._draw_belt_wrench(d, cx + 0.0 * S, waist_y + 3.5 * S, S, pal)
        self._draw_belt_hammer(
            d, cx - spec.waist_w * 0.30 * S, waist_y + 3.5 * S, S, pal
        )

    def _draw_belt_keyring(
        self,
        d: ImageDraw.ImageDraw,
        x: float,
        y: float,
        S: float,
        pal: Dict[str, Color],
    ) -> None:
        outline = pal["outline"]
        # A small carabiner ring with two short keys dangling.
        ring_c = (x, y + 2.5 * S)
        d.ellipse(
            _bbox(ring_c, 2.4 * S, 2.4 * S), outline=outline, width=max(1, int(1.0 * S))
        )
        for dx in (-1.6, 1.6):
            kx = ring_c[0] + dx * S
            d.line(
                [(kx, ring_c[1] + 2.4 * S), (kx, ring_c[1] + 8.0 * S)],
                fill=pal["brass"],
                width=max(1, int(1.1 * S)),
            )
            d.line(
                [(kx, ring_c[1] + 7.0 * S), (kx + 0.9 * S, ring_c[1] + 7.0 * S)],
                fill=pal["brass_dark"],
                width=max(1, int(0.7 * S)),
            )

    def _draw_belt_wrench(
        self,
        d: ImageDraw.ImageDraw,
        x: float,
        y: float,
        S: float,
        pal: Dict[str, Color],
    ) -> None:
        outline = pal["outline"]
        # Loop attaching wrench to belt.
        d.ellipse(
            _bbox((x, y + 1.5 * S), 1.6 * S, 1.6 * S),
            outline=outline,
            width=max(1, int(0.8 * S)),
        )
        # Wrench body — long shaft + open jaw at the bottom.
        d.line(
            [(x, y + 2.5 * S), (x - 0.4 * S, y + 9.0 * S)],
            fill=pal["steel"],
            width=max(1, int(1.8 * S)),
        )
        d.line(
            [(x, y + 2.5 * S), (x - 0.4 * S, y + 9.0 * S)],
            fill=pal["steel_dark"],
            width=max(1, int(0.9 * S)),
        )
        # Wrench jaw — small open-end at the tip.
        d.polygon(
            [
                (x - 0.4 * S - 1.8 * S, y + 9.0 * S),
                (x - 0.4 * S + 1.8 * S, y + 9.0 * S),
                (x - 0.4 * S + 1.2 * S, y + 11.5 * S),
                (x - 0.4 * S - 1.2 * S, y + 11.5 * S),
            ],
            fill=pal["steel"],
            outline=outline,
        )
        d.rectangle(
            (x - 0.4 * S - 0.6 * S, y + 10.5 * S, x - 0.4 * S + 0.6 * S, y + 11.5 * S),
            fill=pal["outline"],
        )

    def _draw_belt_hammer(
        self,
        d: ImageDraw.ImageDraw,
        x: float,
        y: float,
        S: float,
        pal: Dict[str, Color],
    ) -> None:
        outline = pal["outline"]
        # Hammer hanging upside-down (head down, handle clipped to belt).
        d.ellipse(
            _bbox((x, y + 1.5 * S), 1.4 * S, 1.4 * S),
            outline=outline,
            width=max(1, int(0.8 * S)),
        )
        # Handle.
        d.line(
            [(x, y + 2.4 * S), (x + 0.4 * S, y + 8.0 * S)],
            fill=pal["leather"],
            width=max(1, int(1.4 * S)),
        )
        d.line(
            [(x, y + 2.4 * S), (x + 0.4 * S, y + 8.0 * S)],
            fill=pal["leather_dark"],
            width=max(1, int(0.7 * S)),
        )
        # Head — steel rectangle perpendicular to the handle.
        head_c = (x + 0.4 * S, y + 8.5 * S)
        d.rectangle(
            (
                head_c[0] - 3.2 * S,
                head_c[1] - 0.5 * S,
                head_c[0] + 1.8 * S,
                head_c[1] + 2.5 * S,
            ),
            fill=pal["steel"],
            outline=outline,
            width=max(1, int(0.8 * S)),
        )
        # Claw on the back of the head.
        d.polygon(
            [
                (head_c[0] - 3.2 * S, head_c[1] - 0.5 * S),
                (head_c[0] - 4.4 * S, head_c[1] + 0.3 * S),
                (head_c[0] - 4.4 * S, head_c[1] + 1.6 * S),
                (head_c[0] - 3.2 * S, head_c[1] + 2.5 * S),
            ],
            fill=pal["steel_dark"],
            outline=outline,
        )

    def _tq_draw_arms_front(
        self,
        base: Image.Image,
        cx: float,
        shoulder_y: float,
        spec: BobSpec,
        pal: Dict[str, Color],
        S: float,
        pose: BobPose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Front arm (camera-near). Held slightly forward; hand visible
        # at hip level, holding a key ring (the iconic Bob prop).
        sx = cx + spec.shoulder_w * 0.38 * S
        sy = shoulder_y + 2.0 * S
        ex = sx + 1.5 * S + pose.arm_lift * 6.0 * S
        ey = sy + spec.arm_len * S - pose.arm_lift * 8.0 * S
        sleeve = [
            (sx - 3.0 * S, sy),
            (sx + 4.0 * S, sy),
            (ex + spec.cuff_w * 0.55 * S, ey),
            (ex - spec.cuff_w * 0.55 * S, ey),
        ]
        d.polygon(sleeve, fill=pal["tee"], outline=outline)
        # Sleeve highlight along the upper edge.
        d.line(
            [(sx - 2.0 * S, sy + 1.0 * S), (ex - spec.cuff_w * 0.30 * S, ey - 1.0 * S)],
            fill=pal["vest_light"],
            width=max(1, int(0.7 * S)),
        )
        # Hand.
        hand_c = (ex + 1.0 * S, ey + 3.5 * S)
        d.ellipse(
            _bbox(hand_c, 3.8 * S, 3.4 * S),
            fill=pal["skin"],
            outline=outline,
            width=max(1, int(0.8 * S)),
        )
        if pose.hold_keys:
            self._draw_held_keyring(d, hand_c, S, pal)

    def _draw_held_keyring(
        self, d: ImageDraw.ImageDraw, hand: Point, S: float, pal: Dict[str, Color]
    ) -> None:
        outline = pal["outline"]
        # Larger key ring than the belt one — Bob's primary prop.
        ring_c = (hand[0] + 5.0 * S, hand[1] + 0.0 * S)
        d.ellipse(
            _bbox(ring_c, 3.4 * S, 3.4 * S), outline=outline, width=max(1, int(1.2 * S))
        )
        # Three keys hanging.
        for i, ddx in enumerate((-2.5, 0.0, 2.5)):
            key_top = (ring_c[0] + ddx * S, ring_c[1] + 3.4 * S)
            key_tip = (ring_c[0] + ddx * S, ring_c[1] + 10.0 * S)
            d.line([key_top, key_tip], fill=pal["brass"], width=max(1, int(1.3 * S)))
            for ty in (7.0, 9.5):
                d.line(
                    [
                        (ring_c[0] + ddx * S, ring_c[1] + ty * S),
                        (ring_c[0] + (ddx + 1.4) * S, ring_c[1] + ty * S),
                    ],
                    fill=pal["brass_dark"],
                    width=max(1, int(0.9 * S)),
                )
            d.ellipse(
                _bbox((ring_c[0] + ddx * S, ring_c[1] + 4.2 * S), 1.0 * S, 1.2 * S),
                fill=pal["brass_dark"],
                outline=outline,
                width=max(1, int(0.5 * S)),
            )

    def _tq_draw_head(
        self,
        base: Image.Image,
        c: Point,
        spec: BobSpec,
        pal: Dict[str, Color],
        S: float,
        pose: BobPose,
    ) -> None:
        """Three-quarter view head.

        Z-order (matches the pattern documented in
        alice_cryptographer._tq_draw_head — even though Bob has no
        cheek curtains, the structure stays parallel so a future
        long-haired engineer-style character can reuse this scaffold
        without re-deriving the layering):
          1. Neck    (chin will cover the top of it)
          2. Back hair mass
          3. Face oval        ← if curtains existed, they'd go here
                                BEFORE the face oval, like Alice
          4. Five o'clock shadow over the lower face
          5. Tousled-crop bangs (forehead clumps, drawn over the face)
          6. Eyes / nose / mouth
        """
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # 1. Neck strip — chin will overlap the top of it.
        self._draw_neck(d, c, spec, pal, S, slant=+0.5)
        # 2. Back hair mass. Was offset to (c[0]-1, c[1]-4) which
        # made the silhouette asymmetric and left the camera-right
        # side of the head with visible "bald edge" between face and
        # bangs. Now centered on c[0] with a taller envelope so it
        # wraps the skull and meets the bangs without a visible seam.
        d.ellipse(
            _bbox(
                (c[0] + 0.5 * S, c[1] - 3.0 * S),
                (spec.head_w + 6.0) * S,
                (spec.head_h * 0.92) * S,
            ),
            fill=pal["hair"],
            outline=outline,
            width=max(1, int(1.0 * S)),
        )
        # 3. Face oval.
        d.ellipse(
            _bbox(c, spec.head_w * S, spec.head_h * S),
            fill=pal["skin"],
            outline=outline,
            width=max(1, int(1.2 * S)),
        )
        # Five o'clock shadow: a FLATTER ellipse following the jaw
        # line instead of the previous near-circular blob. Was
        # head_w*0.74 wide × jaw_h*2.4 tall (≈19×14 — basically a
        # circle pooling on the chin). New shape is wider + much
        # flatter (head_w*0.78 × jaw_h*1.4 ≈ 20×8) and centered
        # lower so it traces the underside of the jaw rather than
        # the cheeks. Drawn before the mouth so the mouth line still
        # reads through the stubble.
        d.ellipse(
            _bbox(
                (c[0] + 1.0 * S, c[1] + spec.head_h * 0.34 * S),
                (spec.head_w * 0.78) * S,
                (spec.jaw_h * 1.4) * S,
            ),
            fill=pal["skin_shadow"],
            outline=None,
        )
        # Tousled-crop bangs — tightened to two cleaner clumps that
        # don't fight the eyes below. Previous version had a
        # randomish "+/- (3 + sign * 4)" geometry that looked noisy
        # at downsample. New version uses a deliberate left clump +
        # right clump with a small forehead skin gap between them.
        left_clump = [
            (c[0] - spec.head_w * 0.42 * S, c[1] - spec.head_h * 0.48 * S),
            (c[0] - spec.head_w * 0.06 * S, c[1] - spec.head_h * 0.46 * S),
            (c[0] - spec.head_w * 0.10 * S, c[1] - spec.head_h * 0.22 * S),
            (c[0] - spec.head_w * 0.34 * S, c[1] - spec.head_h * 0.18 * S),
        ]
        right_clump = [
            (c[0] + spec.head_w * 0.00 * S, c[1] - spec.head_h * 0.48 * S),
            (c[0] + spec.head_w * 0.34 * S, c[1] - spec.head_h * 0.42 * S),
            (c[0] + spec.head_w * 0.30 * S, c[1] - spec.head_h * 0.18 * S),
            (c[0] + spec.head_w * 0.02 * S, c[1] - spec.head_h * 0.22 * S),
        ]
        d.polygon(left_clump, fill=pal["hair"], outline=outline)
        d.polygon(right_clump, fill=pal["hair"], outline=outline)
        # Brow strokes for expression — one above each eye, drawn in
        # the hair color so they tie the face to the hair without
        # being as heavy as the outline.
        d.line(
            [
                (c[0] - spec.head_w * 0.10 * S, c[1] - spec.head_h * 0.10 * S),
                (c[0] + spec.head_w * 0.06 * S, c[1] - spec.head_h * 0.12 * S),
            ],
            fill=pal["hair"],
            width=max(1, int(0.9 * S)),
        )
        d.line(
            [
                (c[0] + spec.head_w * 0.18 * S, c[1] - spec.head_h * 0.12 * S),
                (c[0] + spec.head_w * 0.34 * S, c[1] - spec.head_h * 0.10 * S),
            ],
            fill=pal["hair"],
            width=max(1, int(0.8 * S)),
        )
        # 3/4 eyes.
        self._draw_eyes_three_quarter(d, c, spec, pal, S, pose)
        # Nose.
        d.line(
            [
                (c[0] + 3.5 * S, c[1] + 1.5 * S),
                (c[0] + 5.0 * S, c[1] + 3.5 * S),
                (c[0] + 3.5 * S, c[1] + 4.5 * S),
            ],
            fill=pal["skin_shadow"],
            width=max(1, int(0.9 * S)),
        )
        # Mouth — drawn LAST (on top of the 5 o'clock shadow) so the
        # lip line stays visible through the stubble.
        mouth_y = c[1] + spec.head_h * 0.28 * S
        if pose.talk_open > 0.2:
            d.ellipse(
                _bbox(
                    (c[0] + 3.0 * S, mouth_y), 3.6 * S, (1.0 + pose.talk_open * 1.6) * S
                ),
                fill=outline,
            )
        else:
            d.arc(
                (c[0] + 0.0 * S, mouth_y - 1.5 * S, c[0] + 6.0 * S, mouth_y + 2.5 * S),
                start=10,
                end=160,
                fill=outline,
                width=max(1, int(1.0 * S)),
            )

    def _draw_neck(
        self,
        d: ImageDraw.ImageDraw,
        head_center: Point,
        spec: BobSpec,
        pal: Dict[str, Color],
        S: float,
        *,
        slant: float = 0.0,
    ) -> None:
        """Draw a short skin-colored neck strip below the head.

        The neck runs from a point just inside the face's bottom
        (head_h * 0.42 below center) down to the shoulder yoke. The
        shoulder yoke's vertical position is reconstructed from
        ``spec.head_anchor`` + ``spec.neck_h`` rather than being
        passed in — this keeps the neck function callable from any
        view's head renderer without threading an extra argument.

        ``slant`` lets the neck tilt a few units to one side so the
        3/4 and side views can suggest a tiny head turn. Front view
        passes 0.0 for a perfectly vertical neck.
        """
        outline = pal["outline"]
        chin_y = head_center[1] + spec.head_h * 0.42 * S
        shoulder_y = (
            head_center[1] + spec.head_h * spec.head_anchor * S + spec.neck_h * S
        )
        # 4-point polygon: narrower at the chin, slightly wider at
        # the shoulder where it meets the trapezius.
        top_w = spec.neck_w * 0.85
        bot_w = spec.neck_w * 1.10
        neck = [
            (head_center[0] - top_w * 0.5 * S + slant * S, chin_y),
            (head_center[0] + top_w * 0.5 * S + slant * S, chin_y),
            (head_center[0] + bot_w * 0.5 * S, shoulder_y + 1.5 * S),
            (head_center[0] - bot_w * 0.5 * S, shoulder_y + 1.5 * S),
        ]
        d.polygon(neck, fill=pal["skin"], outline=outline)
        # Subtle shadow on the camera-far side of the neck so it
        # doesn't read as a flat tab of skin.
        d.line(
            [
                (head_center[0] - top_w * 0.40 * S + slant * S, chin_y + 0.5 * S),
                (head_center[0] - bot_w * 0.42 * S, shoulder_y),
            ],
            fill=pal["skin_shadow"],
            width=max(1, int(0.7 * S)),
        )

    def _draw_eyes_three_quarter(
        self,
        d: ImageDraw.ImageDraw,
        c: Point,
        spec: BobSpec,
        pal: Dict[str, Color],
        S: float,
        pose: BobPose,
    ) -> None:
        outline = pal["outline"]
        eye_y = c[1] - 2.0 * S
        near = (c[0] + 1.0 * S, eye_y)
        far = (c[0] + 8.0 * S, eye_y - 0.2 * S)
        if pose.blink:
            d.line(
                [(near[0] - 2.0 * S, near[1]), (near[0] + 2.0 * S, near[1])],
                fill=outline,
                width=max(1, int(1.1 * S)),
            )
            d.line(
                [(far[0] - 1.4 * S, far[1]), (far[0] + 1.4 * S, far[1])],
                fill=outline,
                width=max(1, int(0.9 * S)),
            )
            return
        d.ellipse(
            _bbox(near, 3.4 * S, 1.6 * S),
            fill=pal["white"],
            outline=outline,
            width=max(1, int(0.9 * S)),
        )
        d.ellipse(_bbox((near[0] + 0.4 * S, near[1]), 1.2 * S, 1.8 * S), fill=outline)
        d.ellipse(
            _bbox(far, 2.6 * S, 1.4 * S),
            fill=pal["white"],
            outline=outline,
            width=max(1, int(0.8 * S)),
        )
        d.ellipse(_bbox((far[0] + 0.3 * S, far[1]), 0.9 * S, 1.4 * S), fill=outline)

    # ─────────────────────────────────────────────────────────────────
    # SIDE view — pure profile, used for walking.
    # ─────────────────────────────────────────────────────────────────

    def _render_side(
        self,
        base: Image.Image,
        cx: float,
        feet_y: float,
        spec: BobSpec,
        pal: Dict[str, Color],
        S: float,
        pose: BobPose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Phase-driven leg + arm swing for the walk loop.
        step = pose.step_phase  # -1 (back) .. +1 (forward)
        boot_top_y = feet_y - spec.boot_h * S
        pants_top_y = boot_top_y - spec.leg_h * S
        waist_y = pants_top_y - 2.0 * S
        vest_top_y = waist_y - spec.vest_h * S
        shoulder_y = vest_top_y - 4.0 * S
        # In profile, the head sits forward of the shoulder centerline.
        head_center = (
            cx + 3.0 * S,
            shoulder_y - spec.head_h * spec.head_anchor * S - spec.neck_h * S,
        )

        # Back leg first (gets covered by the body), then front leg.
        # camera-near leg follows +step (forward when step > 0),
        # camera-far leg follows -step.
        for side, sgn in (("far", -step), ("near", +step)):
            knee_x = cx + sgn * 4.0 * S
            ankle_x = cx + sgn * 8.0 * S
            knee_y = pants_top_y + spec.leg_h * 0.55 * S
            ankle_y = boot_top_y - abs(sgn) * 1.5 * S  # tiny lift on the swing leg
            hip = (cx + sgn * 1.0 * S, pants_top_y)
            leg = [
                (hip[0] - spec.leg_w * 0.5 * S, hip[1]),
                (hip[0] + spec.leg_w * 0.5 * S, hip[1]),
                (knee_x + spec.leg_w * 0.45 * S, knee_y),
                (ankle_x + spec.boot_w * 0.5 * S, ankle_y),
                (ankle_x - spec.boot_w * 0.5 * S, ankle_y),
                (knee_x - spec.leg_w * 0.45 * S, knee_y),
            ]
            leg_fill = pal["pants"] if side == "near" else pal["pants_shadow"]
            d.polygon(leg, fill=leg_fill, outline=outline)
            # Boot.
            d.rounded_rectangle(
                (
                    ankle_x - spec.boot_w * 0.55 * S,
                    ankle_y - 0.5 * S,
                    ankle_x + spec.boot_w * 0.95 * S,
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
                    ankle_y + spec.boot_h * S - 1.6 * S,
                    ankle_x + spec.boot_w * 0.95 * S,
                    ankle_y + spec.boot_h * S,
                ),
                fill=pal["boot_sole"],
                outline=outline,
                width=max(1, int(0.7 * S)),
            )

        # Torso in profile — narrower than 3/4 view, vest visible as a
        # single side-panel polygon.
        body_back = cx - spec.shoulder_w * 0.12 * S  # back-of-shoulder
        body_front = cx + spec.shoulder_w * 0.34 * S  # front-of-chest
        torso = [
            (body_back, shoulder_y),
            (body_front, shoulder_y),
            (cx + spec.waist_w * 0.34 * S, waist_y),
            (cx - spec.waist_w * 0.18 * S, waist_y),
        ]
        d.polygon(torso, fill=pal["vest"], outline=outline)
        # Tee collar peeking at the throat.
        d.polygon(
            [
                (cx + spec.shoulder_w * 0.14 * S, shoulder_y - 1.0 * S),
                (cx + spec.shoulder_w * 0.30 * S, shoulder_y - 1.0 * S),
                (cx + spec.shoulder_w * 0.26 * S, shoulder_y + 4.0 * S),
                (cx + spec.shoulder_w * 0.10 * S, shoulder_y + 4.0 * S),
            ],
            fill=pal["tee"],
            outline=outline,
        )
        # Hi-vis stripe across the side of the vest at chest level.
        stripe_y = shoulder_y + spec.vest_h * 0.50 * S
        d.rectangle(
            (
                cx - spec.shoulder_w * 0.10 * S,
                stripe_y,
                cx + spec.shoulder_w * 0.32 * S,
                stripe_y + 2.4 * S,
            ),
            fill=pal["hi_vis"],
            outline=outline,
            width=max(1, int(0.6 * S)),
        )
        # Belt.
        d.rounded_rectangle(
            (
                cx - spec.waist_w * 0.22 * S,
                waist_y - 0.5 * S,
                cx + spec.waist_w * 0.40 * S,
                waist_y + 3.5 * S,
            ),
            radius=1.2 * S,
            fill=pal["leather"],
            outline=outline,
            width=max(1, int(0.9 * S)),
        )
        d.rectangle(
            (
                cx + spec.waist_w * 0.06 * S,
                waist_y - 0.3 * S,
                cx + spec.waist_w * 0.14 * S,
                waist_y + 3.3 * S,
            ),
            fill=pal["steel"],
            outline=outline,
            width=max(1, int(0.7 * S)),
        )
        # One hanging tool (the wrench) — keep it simple in profile.
        self._draw_belt_wrench(
            d, cx + spec.waist_w * 0.20 * S, waist_y + 3.5 * S, S, pal
        )

        # Arms — opposite phase to the legs.
        arm_swing = -step
        # Far arm (drawn first, gets clipped by torso).
        far_shoulder = (cx - spec.shoulder_w * 0.06 * S, shoulder_y + 2.0 * S)
        far_hand = (cx - 3.0 * S + arm_swing * 4.0 * S, shoulder_y + spec.arm_len * S)
        d.line(
            [far_shoulder, far_hand],
            fill=pal["tee_dark"],
            width=max(1, int(spec.arm_w * S)),
        )
        d.ellipse(
            _bbox((far_hand[0], far_hand[1] + 3.0 * S), 3.0 * S, 2.6 * S),
            fill=pal["skin"],
            outline=outline,
            width=max(1, int(0.7 * S)),
        )
        # Near arm.
        near_shoulder = (cx + spec.shoulder_w * 0.28 * S, shoulder_y + 2.0 * S)
        near_hand = (cx + 6.0 * S + arm_swing * -4.0 * S, shoulder_y + spec.arm_len * S)
        d.line(
            [near_shoulder, near_hand],
            fill=pal["tee"],
            width=max(1, int(spec.arm_w * S)),
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
        spec: BobSpec,
        pal: Dict[str, Color],
        S: float,
        pose: BobPose,
    ) -> None:
        """Profile head with a proper inline neck that connects the
        jawline down to the shoulder.

        Z-order (explicit, matches Alice's documented pattern):
          1. Inline profile neck  (chin will overlap the top edge)
          2. Back hair cap
          3. Full face polygon    (extends to head_h * 0.46 chin — a
                                   real jaw, not the truncated 0.34
                                   polygon the previous revision used)
          4. Stub / 5-oc shadow over the lower face
          5. Ear (camera-far side)
          6. Forward bang sweep
          7. Eye + brow
          8. Mouth lip line
        """
        d = ImageDraw.Draw(base)
        outline = pal["outline"]

        # 1. Inline profile neck — connects the jaw point to the
        # shoulder. The generic `_draw_neck` helper centers the neck
        # under head_center.x, but in profile the chin is offset
        # forward (+head_w * 0.34) and the neck needs to angle back
        # to the shoulder. Draw the neck polygon explicitly here so
        # the front edge follows the chin and the back edge follows
        # the nape.
        shoulder_y = c[1] + spec.head_h * spec.head_anchor * S + spec.neck_h * S
        chin_front_x = c[0] + spec.head_w * 0.36 * S
        chin_front_y = c[1] + spec.head_h * 0.36 * S
        nape_x = c[0] - spec.head_w * 0.10 * S
        nape_y = c[1] + spec.head_h * 0.32 * S
        neck = [
            (chin_front_x, chin_front_y),
            (chin_front_x - 1.5 * S, shoulder_y + 2.0 * S),
            (nape_x - 1.0 * S, shoulder_y + 2.0 * S),
            (nape_x, nape_y),
        ]
        d.polygon(neck, fill=pal["skin"], outline=outline)
        # Throat shadow — a thin darker stroke along the back of the
        # neck so the front-vs-back planes read.
        d.line(
            [
                (nape_x + 0.5 * S, nape_y + 1.0 * S),
                (nape_x - 0.5 * S, shoulder_y + 1.0 * S),
            ],
            fill=pal["skin_shadow"],
            width=max(1, int(0.7 * S)),
        )

        # 2. Back hair cap.
        d.ellipse(
            _bbox(
                (c[0] - 2.0 * S, c[1] - 3.0 * S),
                (spec.head_w + 4.0) * S,
                (spec.head_h * 0.86) * S,
            ),
            fill=pal["hair"],
            outline=outline,
            width=max(1, int(1.0 * S)),
        )

        # 3. Face polygon — extended down to head_h * 0.46 chin
        # (was 0.34, which cut the bottom of the head off and left a
        # gap above the neck). The forward-chin point is at the same
        # x as the neck top so the silhouette connects cleanly.
        face = [
            (c[0] - spec.head_w * 0.30 * S, c[1] - spec.head_h * 0.46 * S),
            (c[0] + spec.head_w * 0.28 * S, c[1] - spec.head_h * 0.36 * S),
            (c[0] + spec.head_w * 0.40 * S, c[1] - spec.head_h * 0.14 * S),  # brow
            (
                c[0] + spec.head_w * 0.54 * S,
                c[1] - spec.head_h * 0.02 * S,
            ),  # nose bridge
            (c[0] + spec.head_w * 0.56 * S, c[1] + spec.head_h * 0.06 * S),  # nose tip
            (
                c[0] + spec.head_w * 0.40 * S,
                c[1] + spec.head_h * 0.14 * S,
            ),  # under-nose
            (c[0] + spec.head_w * 0.44 * S, c[1] + spec.head_h * 0.22 * S),  # upper lip
            (c[0] + spec.head_w * 0.40 * S, c[1] + spec.head_h * 0.30 * S),  # lower lip
            (
                c[0] + spec.head_w * 0.36 * S,
                c[1] + spec.head_h * 0.36 * S,
            ),  # chin connects to neck top
            (
                c[0] + spec.head_w * 0.10 * S,
                c[1] + spec.head_h * 0.46 * S,
            ),  # jaw under-chin
            (
                c[0] - spec.head_w * 0.10 * S,
                c[1] + spec.head_h * 0.36 * S,
            ),  # nape connects to neck top
            (
                c[0] - spec.head_w * 0.36 * S,
                c[1] + spec.head_h * 0.18 * S,
            ),  # jaw back corner
            (c[0] - spec.head_w * 0.42 * S, c[1] - spec.head_h * 0.10 * S),  # cheek
        ]
        d.polygon(face, fill=pal["skin"], outline=outline)

        # 4. Five o'clock shadow on the lower face — repositioned to
        # follow the new larger jawline. Covers from the cheekbone
        # down to the chin including the mouth area.
        stub = [
            (c[0] - spec.head_w * 0.34 * S, c[1] + spec.head_h * 0.10 * S),
            (c[0] + spec.head_w * 0.40 * S, c[1] + spec.head_h * 0.16 * S),
            (c[0] + spec.head_w * 0.44 * S, c[1] + spec.head_h * 0.26 * S),
            (c[0] + spec.head_w * 0.36 * S, c[1] + spec.head_h * 0.36 * S),
            (c[0] + spec.head_w * 0.10 * S, c[1] + spec.head_h * 0.44 * S),
            (c[0] - spec.head_w * 0.10 * S, c[1] + spec.head_h * 0.34 * S),
            (c[0] - spec.head_w * 0.34 * S, c[1] + spec.head_h * 0.20 * S),
        ]
        d.polygon(stub, fill=pal["skin_shadow"], outline=None)

        # 5. Ear (camera-far side) — slightly lower + larger than
        # before so the silhouette reads as having a real ear.
        ear_c = (c[0] - spec.head_w * 0.22 * S, c[1] + spec.head_h * 0.04 * S)
        d.ellipse(
            _bbox(ear_c, 3.4 * S, 4.6 * S),
            fill=pal["skin"],
            outline=outline,
            width=max(1, int(0.8 * S)),
        )
        # Ear inner crease.
        d.line(
            [
                (ear_c[0] + 0.5 * S, ear_c[1] - 1.5 * S),
                (ear_c[0] - 0.5 * S, ear_c[1] + 1.5 * S),
            ],
            fill=pal["skin_shadow"],
            width=max(1, int(0.7 * S)),
        )

        # 6. Forward bang sweep across the forehead.
        d.polygon(
            [
                (c[0] - spec.head_w * 0.30 * S, c[1] - spec.head_h * 0.46 * S),
                (c[0] + spec.head_w * 0.30 * S, c[1] - spec.head_h * 0.34 * S),
                (c[0] + spec.head_w * 0.26 * S, c[1] - spec.head_h * 0.20 * S),
                (c[0] - spec.head_w * 0.20 * S, c[1] - spec.head_h * 0.18 * S),
            ],
            fill=pal["hair"],
            outline=outline,
        )

        # 7. Eye + brow.
        eye_x = c[0] + spec.head_w * 0.22 * S
        eye_y = c[1] - spec.head_h * 0.06 * S
        # Brow stroke (subtle).
        d.line(
            [
                (c[0] + spec.head_w * 0.12 * S, c[1] - spec.head_h * 0.14 * S),
                (c[0] + spec.head_w * 0.32 * S, c[1] - spec.head_h * 0.12 * S),
            ],
            fill=pal["hair"],
            width=max(1, int(0.8 * S)),
        )
        if pose.blink:
            d.line(
                [(eye_x - 1.2 * S, eye_y), (eye_x + 1.2 * S, eye_y)],
                fill=outline,
                width=max(1, int(1.0 * S)),
            )
        else:
            d.ellipse(
                _bbox((eye_x, eye_y), 2.0 * S, 1.4 * S),
                fill=pal["white"],
                outline=outline,
                width=max(1, int(0.7 * S)),
            )
            d.ellipse(_bbox((eye_x + 0.4 * S, eye_y), 1.0 * S, 1.2 * S), fill=outline)

        # 8. Mouth lip line.
        d.line(
            [
                (c[0] + spec.head_w * 0.34 * S, c[1] + spec.head_h * 0.24 * S),
                (c[0] + spec.head_w * 0.42 * S, c[1] + spec.head_h * 0.24 * S),
            ],
            fill=outline,
            width=max(1, int(0.9 * S)),
        )

    # ─────────────────────────────────────────────────────────────────
    # FRONT view — facing the camera, used for talk + interact.
    # ─────────────────────────────────────────────────────────────────

    def _render_front(
        self,
        base: Image.Image,
        cx: float,
        feet_y: float,
        spec: BobSpec,
        pal: Dict[str, Color],
        S: float,
        pose: BobPose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        boot_top_y = feet_y - spec.boot_h * S
        pants_top_y = boot_top_y - spec.leg_h * S
        waist_y = pants_top_y - 2.0 * S
        vest_top_y = waist_y - spec.vest_h * S
        shoulder_y = vest_top_y - 4.0 * S
        head_center = (
            cx,
            shoulder_y - spec.head_h * spec.head_anchor * S - spec.neck_h * S,
        )

        # Legs (symmetric).
        for sgn in (-1, 1):
            hip = (cx + sgn * 5.0 * S, pants_top_y)
            ankle = (cx + sgn * 5.0 * S, boot_top_y)
            leg = [
                (hip[0] - spec.leg_w * 0.5 * S, hip[1]),
                (hip[0] + spec.leg_w * 0.5 * S, hip[1]),
                (ankle[0] + spec.boot_w * 0.4 * S, ankle[1]),
                (ankle[0] - spec.boot_w * 0.4 * S, ankle[1]),
            ]
            d.polygon(leg, fill=pal["pants"], outline=outline)
            d.rounded_rectangle(
                (
                    ankle[0] - spec.boot_w * 0.55 * S,
                    ankle[1] - 0.5 * S,
                    ankle[0] + spec.boot_w * 0.55 * S,
                    feet_y,
                ),
                radius=2.0 * S,
                fill=pal["boot"],
                outline=outline,
                width=max(1, int(1.0 * S)),
            )
            d.rectangle(
                (
                    ankle[0] - spec.boot_w * 0.55 * S,
                    feet_y - 1.6 * S,
                    ankle[0] + spec.boot_w * 0.55 * S,
                    feet_y,
                ),
                fill=pal["boot_sole"],
                outline=outline,
                width=max(1, int(0.7 * S)),
            )

        # Tee (symmetric).
        tee = [
            (cx - spec.shoulder_w * 0.50 * S, vest_top_y - 2.0 * S),
            (cx + spec.shoulder_w * 0.50 * S, vest_top_y - 2.0 * S),
            (cx + spec.waist_w * 0.50 * S, waist_y),
            (cx - spec.waist_w * 0.50 * S, waist_y),
        ]
        d.polygon(tee, fill=pal["tee"], outline=outline)
        # Vest open at front — two side panels.
        for sgn in (-1, 1):
            panel = [
                (cx + sgn * spec.shoulder_w * 0.48 * S, vest_top_y),
                (cx + sgn * 3.0 * S, vest_top_y + 1.0 * S),
                (cx + sgn * spec.waist_w * 0.30 * S, waist_y),
                (cx + sgn * spec.waist_w * 0.48 * S, waist_y),
            ]
            d.polygon(panel, fill=pal["vest"], outline=outline)
            # Hi-vis stripe on each panel.
            stripe_y = vest_top_y + spec.vest_h * 0.40 * S
            d.rectangle(
                (
                    cx + sgn * spec.shoulder_w * 0.30 * S - 4.0 * S,
                    stripe_y,
                    cx + sgn * spec.shoulder_w * 0.30 * S + 4.0 * S,
                    stripe_y + 2.4 * S,
                ),
                fill=pal["hi_vis"],
                outline=outline,
                width=max(1, int(0.6 * S)),
            )
        # Tee crew-neck band.
        d.arc(
            (cx - 7.0 * S, vest_top_y - 3.0 * S, cx + 7.0 * S, vest_top_y + 3.0 * S),
            start=10,
            end=170,
            fill=pal["tee_dark"],
            width=max(1, int(1.0 * S)),
        )
        # Belt (symmetric).
        d.rounded_rectangle(
            (
                cx - spec.waist_w * 0.52 * S,
                waist_y - 0.5 * S,
                cx + spec.waist_w * 0.52 * S,
                waist_y + 3.5 * S,
            ),
            radius=1.4 * S,
            fill=pal["leather"],
            outline=outline,
            width=max(1, int(0.9 * S)),
        )
        d.rectangle(
            (cx - 2.6 * S, waist_y - 0.3 * S, cx + 2.6 * S, waist_y + 3.3 * S),
            fill=pal["steel"],
            outline=outline,
            width=max(1, int(0.7 * S)),
        )
        # Tools laid out across the belt (both sides).
        self._draw_belt_keyring(
            d, cx + spec.waist_w * 0.32 * S, waist_y + 3.5 * S, S, pal
        )
        self._draw_belt_hammer(
            d, cx - spec.waist_w * 0.32 * S, waist_y + 3.5 * S, S, pal
        )

        # Arms (symmetric).
        for sgn in (-1, 1):
            sx = cx + sgn * spec.shoulder_w * 0.46 * S
            sy = shoulder_y + 2.0 * S
            ex = sx + sgn * 1.0 * S
            ey = sy + spec.arm_len * S - pose.arm_lift * 8.0 * S
            sleeve = [
                (sx - spec.cuff_w * 0.6 * S, sy),
                (sx + spec.cuff_w * 0.6 * S, sy),
                (ex + spec.cuff_w * 0.55 * S, ey),
                (ex - spec.cuff_w * 0.55 * S, ey),
            ]
            d.polygon(sleeve, fill=pal["tee"], outline=outline)
            d.ellipse(
                _bbox((ex, ey + 3.0 * S), 3.6 * S, 3.0 * S),
                fill=pal["skin"],
                outline=outline,
                width=max(1, int(0.8 * S)),
            )

        # Head front-view.
        self._front_draw_head(base, head_center, spec, pal, S, pose)

    def _front_draw_head(
        self,
        base: Image.Image,
        c: Point,
        spec: BobSpec,
        pal: Dict[str, Color],
        S: float,
        pose: BobPose,
    ) -> None:
        d = ImageDraw.Draw(base)
        outline = pal["outline"]
        # Neck strip (front view: vertical, no slant).
        self._draw_neck(d, c, spec, pal, S, slant=0.0)
        # Back hair cap.
        d.ellipse(
            _bbox(
                (c[0], c[1] - 3.0 * S),
                (spec.head_w + 4.0) * S,
                (spec.head_h * 0.86) * S,
            ),
            fill=pal["hair"],
            outline=outline,
            width=max(1, int(1.0 * S)),
        )
        # Face.
        d.ellipse(
            _bbox(c, spec.head_w * S, spec.head_h * S),
            fill=pal["skin"],
            outline=outline,
            width=max(1, int(1.2 * S)),
        )
        # Five o'clock shadow (symmetric, lower than the previous
        # `jaw shadow` so it covers the mouth area).
        d.ellipse(
            _bbox(
                (c[0], c[1] + spec.head_h * 0.32 * S),
                (spec.head_w * 0.78) * S,
                (spec.jaw_h * 2.4) * S,
            ),
            fill=pal["skin_shadow"],
            outline=None,
        )
        # Ears (two, one per side).
        for sgn in (-1, 1):
            d.ellipse(
                _bbox(
                    (c[0] + sgn * spec.head_w * 0.48 * S, c[1] + 0.5 * S),
                    2.6 * S,
                    3.4 * S,
                ),
                fill=pal["skin"],
                outline=outline,
                width=max(1, int(0.7 * S)),
            )
        # Tousled-crop bangs — symmetric three-clump fringe.
        for dx in (-7.0, 0.0, 7.0):
            d.polygon(
                [
                    (c[0] + (dx - 3.0) * S, c[1] - spec.head_h * 0.48 * S),
                    (c[0] + (dx + 3.0) * S, c[1] - spec.head_h * 0.48 * S),
                    (c[0] + (dx + 4.0) * S, c[1] - spec.head_h * 0.20 * S),
                    (c[0] + (dx - 4.0) * S, c[1] - spec.head_h * 0.20 * S),
                ],
                fill=pal["hair"],
                outline=outline,
            )
        # Symmetric brow strokes — one above each eye, in hair color.
        for sgn in (-1, 1):
            ex = c[0] + sgn * spec.head_w * 0.18 * S
            d.line(
                [
                    (ex - 2.4 * S, c[1] - spec.head_h * 0.12 * S),
                    (ex + 2.4 * S, c[1] - spec.head_h * 0.12 * S),
                ],
                fill=pal["hair"],
                width=max(1, int(0.9 * S)),
            )
        # Two symmetric eyes.
        eye_y = c[1] - 2.0 * S
        for sgn in (-1, 1):
            ex = c[0] + sgn * spec.head_w * 0.18 * S
            if pose.blink:
                d.line(
                    [(ex - 2.0 * S, eye_y), (ex + 2.0 * S, eye_y)],
                    fill=outline,
                    width=max(1, int(1.1 * S)),
                )
            else:
                d.ellipse(
                    _bbox((ex, eye_y), 2.8 * S, 1.6 * S),
                    fill=pal["white"],
                    outline=outline,
                    width=max(1, int(0.9 * S)),
                )
                d.ellipse(_bbox((ex, eye_y), 1.2 * S, 1.6 * S), fill=outline)
        # Symmetric nose: a small triangle.
        d.polygon(
            [
                (c[0] - 1.4 * S, c[1] + 1.0 * S),
                (c[0] + 1.4 * S, c[1] + 1.0 * S),
                (c[0] + 0.0 * S, c[1] + 4.5 * S),
            ],
            fill=pal["skin_shadow"],
            outline=None,
        )
        # Mouth.
        mouth_y = c[1] + spec.head_h * 0.30 * S
        if pose.talk_open > 0.2:
            d.ellipse(
                _bbox((c[0], mouth_y), 4.0 * S, (1.0 + pose.talk_open * 1.6) * S),
                fill=outline,
            )
        else:
            d.arc(
                (c[0] - 3.5 * S, mouth_y - 1.5 * S, c[0] + 3.5 * S, mouth_y + 2.5 * S),
                start=10,
                end=170,
                fill=outline,
                width=max(1, int(1.0 * S)),
            )
