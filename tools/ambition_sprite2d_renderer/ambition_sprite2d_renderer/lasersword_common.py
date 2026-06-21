from __future__ import annotations

"""Shared drawing primitives for the laser-sword family.

Two sprite targets build on this module:

- `lasersword_with_guns` — the wielded weapon. A broad, leaf-shaped
  cyan energy blade mounted on a dense gunmetal chassis with brass
  trim, plus a cluster of three gatling-style ion barrels and two
  forward "stinger" antennae.
- `lasersword` — the projectile fired by the weapon. Identical blade
  + hilt; the gun chassis is stripped down to bare hilt.

The weapon is built in blade-local space: the pommel base sits at
canvas center, the grip runs in -Y, the crossguard sits at -Y_GUARD,
and the blade extends from there into +Y. Rotation about canvas
center therefore swings the blade out from the grip the way a real
sword does, which keeps swing arcs readable across frames.

Anchors (grip / muzzle / tip / forward vector) are tracked alongside
the rendered frame so the game can pin the weapon to a character's
hand at runtime — see `frame_anchors`.
"""

import math
from dataclasses import dataclass
from typing import Optional, Tuple

from PIL import Image, ImageColor, ImageDraw, ImageFilter
from ambition_sprite2d_renderer.core.draw import rgba

RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]




def with_alpha(color: RGBA, alpha: int) -> RGBA:
    return (color[0], color[1], color[2], max(0, min(255, alpha)))


def lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def ease_in_out(t: float) -> float:
    t = max(0.0, min(1.0, t))
    return 0.5 - 0.5 * math.cos(math.pi * t)


# ---- Palette ----------------------------------------------------------------
# Cyan-blue plasma blade with a hard white core; dark gunmetal chassis
# with brass trim. No magenta — the energy reads as "weapon" rather
# than "magic crystal".

BLADE_CORE = rgba("#F4FBFF")
BLADE_HOT = rgba("#D8F1FF")
BLADE_PRIMARY = rgba("#5BCBFF")
BLADE_MID = rgba("#48A5F0")
BLADE_RIM = rgba("#1E5BB8")
BLADE_HALO = rgba("#A2D6FF")

GUN_DARK = rgba("#11141C")
GUN_BODY = rgba("#1A2030")
GUN_PLATE = rgba("#2C3447")
GUN_PLATE_HI = rgba("#444E66")
GUN_RIM = rgba("#7C879F")
GUN_HI = rgba("#A6B0C6")

BRASS_DEEP = rgba("#7A571F")
BRASS = rgba("#C99647")
BRASS_HI = rgba("#FFD68A")

EMITTER_HOT = rgba("#9EE0FF")
EMITTER_CORE = rgba("#FFFFFF")
EMITTER_DEEP = rgba("#2C7DBD")


# ---- Geometry (blade-local, in design units) -------------------------------
#
# Three knobs control rendering:
#
# - ``RENDER_SCALE`` multiplies every design unit into output pixels.
#   1.0 = base resolution; bump to 2.0 / 3.0 for higher-detail sheets
#   without changing any geometry. Auto-crop in `build_sheet` then
#   tightens the frame around the alpha so the only thing the user
#   has to pick is "how detailed do I want this".
# - ``SUPER`` is the supersample factor for anti-aliasing. The
#   drawing happens at ``SUPER × RENDER_SCALE`` of the final output
#   resolution, then a LANCZOS downsample folds the SUPER away.
#   Higher SUPER = smoother edges, same output pixel count.
# - ``FRAME_PX_BASE`` is the size of the square working canvas in
#   design units. It needs to be large enough that the full geometry
#   (longest blade-local extent from the pommel pivot, plus halo)
#   fits inside the canvas without clipping at any rotation angle.
#   Set generously; the auto-crop pass throws away the wasted edges.
RENDER_SCALE: float = 1.0
SUPER: int = 4
FRAME_PX_BASE: int = 320
FRAME_PX: int = int(FRAME_PX_BASE * RENDER_SCALE)
W: int = FRAME_PX * SUPER
H: int = FRAME_PX * SUPER

HILT_LEN = 26  # grip length (pommel → crossguard back)
GRIP_W_BOT = 4.6  # grip width near pommel
GRIP_W_TOP = 5.6  # grip width near crossguard

GUARD_THICK = 5.4
GUARD_HALFW = 17.0

# Receiver = the big block at the blade base that houses the guns.
RECEIVER_THICK = 7.4  # along blade-direction
RECEIVER_HALFW = 15.0  # across blade-direction

BLADE_LEN = 108  # blade emission → tip (significantly longer)
BLADE_MID_HALFW = 9.2  # widest point of the leaf blade
BLADE_TIP_OFFSET = 0.0  # tip is centered (x=0)

# Gatling cluster: 3 short barrels in a triangular arrangement
# OUTBOARD of the blade, on the +X side (one side only — like the
# reference image which has the gatling on one flank).
CLUSTER_OFFSET_X = 12.5  # cluster center, x offset from blade centerline
CLUSTER_OFFSET_Y = 5.0  # cluster center, +Y offset past crossguard
CLUSTER_BARREL_LEN = 22.0
CLUSTER_BARREL_W = 3.4
CLUSTER_BARREL_GAP = 4.6  # spacing between cluster barrels

# Forward "stinger" antennae — thin rods sticking forward from the
# crossguard above and below the blade. Visually the small rifle-rod
# poles in the reference image.
STINGER_LEN = 18.0
STINGER_W = 1.2
STINGER_OFFSET_X = 6.0  # how far above/below blade centerline
STINGER_TIP_BULB = 1.6


# Blade-local Y reference points (+Y = toward blade tip).
POMMEL_Y = 0.0
GRIP_Y0 = POMMEL_Y
GRIP_Y1 = POMMEL_Y + HILT_LEN
GUARD_Y0 = GRIP_Y1
GUARD_Y1 = GRIP_Y1 + GUARD_THICK
RECEIVER_Y0 = GUARD_Y1
RECEIVER_Y1 = RECEIVER_Y0 + RECEIVER_THICK


def blade_y_range(with_receiver: bool) -> Tuple[float, float]:
    """Blade emission and tip Y in blade-local coordinates.

    When the receiver block is drawn (wielded weapon), the blade
    starts at ``RECEIVER_Y1`` — right past the chassis. When the
    receiver is omitted (projectile), the blade starts at
    ``GUARD_Y1`` — right at the crossguard front face — so the blade
    is always TOUCHING the hilt with no visible gap.
    """
    y0 = RECEIVER_Y1 if with_receiver else GUARD_Y1
    return (y0, y0 + BLADE_LEN)


# Frame anchor: where the pommel of the rotated weapon ends up inside
# the FINAL frame. Pick based on frame aspect ratio AND whether the
# gun cluster is mounted. Each branch is tuned for the corresponding
# target's frame size (see `lasersword.py` / `lasersword_with_guns.py`)
# so the silhouette nearly fills the frame with only a few px of
# margin around the active art:
#
# - Landscape (frame_w > frame_h * 1.5) without barrels (projectile):
#   pommel near the left edge, vertically centered. The blade extends
#   right and is the dominant element; nothing sticks above or below.
# - Landscape WITH barrels: pommel still near the left, but biased
#   DOWN so the gun cluster (which sticks "up" in image space after
#   the 90° rotation) has room without clipping the top edge.
# - Square / portrait: pommel toward lower-left so a diagonal pose
#   has room to extend the blade upward.
def _frame_anchor(with_barrels: bool, frame_size: Tuple[int, int]) -> Point:
    fw, fh = frame_size
    if fw > fh * 1.5:
        if with_barrels:
            return (fw * 0.085, fh * 0.66)
        return (fw * 0.085, fh * 0.50)
    if with_barrels:
        return (fw * 0.40, fh * 0.68)
    return (fw * 0.50, fh * 0.50)


def s(v: float) -> int:
    """Convert a design-unit length into supersample-canvas pixels.

    Design units are the same everywhere in this module (geometry
    constants, polygon vertex offsets, blur radii, etc.). Output is
    in the working ``W × H`` canvas pixel space, which is
    ``RENDER_SCALE × SUPER`` larger than the design size — so one
    design unit becomes ``RENDER_SCALE × SUPER`` super pixels and,
    after the LANCZOS downsample, ``RENDER_SCALE`` final pixels.
    """
    return int(round(v * SUPER * RENDER_SCALE))


# ---- Anchor projection -----------------------------------------------------


@dataclass(frozen=True)
class WeaponAnchors:
    """Pixel-space anchors on a rendered weapon frame.

    All coordinates are in the FINAL FRAME's pixel space (post
    rotation and downsample), with image-coord conventions (+x right,
    +y down). ``forward`` is a unit vector pointing in the direction
    the blade points — useful when the game wants to spawn a
    projectile in the same direction the visible blade is facing
    without having to inspect the angle field.
    """

    grip: Point  # mid-grip — where the character's primary hand sits
    pommel: Point  # back of grip (rotation pivot)
    guard: Point  # crossguard center (between grip and blade)
    muzzle: Point  # projectile-emission point (gun-variant only)
    tip: Point  # blade tip
    forward: Point  # unit vector along the blade
    angle_deg: float  # rotation in degrees about the pommel pivot


# Blade-local anchor positions. The tip anchor depends on whether the
# receiver is drawn, so it's computed in `frame_anchors` rather than
# living as a module constant.
GRIP_ANCHOR_LOCAL: Point = (0.0, HILT_LEN * 0.55)
POMMEL_ANCHOR_LOCAL: Point = (0.0, POMMEL_Y)
GUARD_ANCHOR_LOCAL: Point = (0.0, (GUARD_Y0 + GUARD_Y1) * 0.5)
# Muzzle: front of the center barrel in the gun cluster. Matches the
# barrel positions in `draw_gun_cluster_layer`.
MUZZLE_ANCHOR_LOCAL: Point = (
    CLUSTER_OFFSET_X - CLUSTER_BARREL_GAP * 0.05,
    RECEIVER_Y1 + CLUSTER_OFFSET_Y + CLUSTER_BARREL_GAP * 0.70 + CLUSTER_BARREL_LEN,
)


def _project_anchor(
    local: Point,
    angle_deg: float,
    with_barrels: bool,
    frame_size: Tuple[int, int],
    offset_px: Point,
) -> Point:
    """Project a blade-local anchor through the same transform
    `draw_weapon` applies and return the final frame-pixel
    coordinates.

    The transform stack matches `draw_weapon`:
      1. Blade-local (px, py) → super canvas at (W/2 + s(px), H/2 + s(py))
      2. Rotate about (W/2, H/2) by ``angle_deg`` (PIL CCW)
      3. Paste rotated working canvas into a super-sized output canvas
         with offset chosen so the pommel (rotation pivot) lands at
         ``_frame_anchor`` in frame-pixel coords.
      4. ``offset_px`` is in design units; convert by × RENDER_SCALE
         to get frame-pixel offset.
      5. Divide by SUPER (downsample) — already implicit in
         ``rx_super / SUPER`` below.
    """
    px, py = local
    rad = math.radians(angle_deg)
    cos_a, sin_a = math.cos(rad), math.sin(rad)
    # Rotated offset from pivot, in super pixels. ``s()`` already
    # encodes RENDER_SCALE, so this gives the correct number of super
    # pixels at the current render resolution.
    rx_super = s(px) * cos_a + s(py) * sin_a
    ry_super = -s(px) * sin_a + s(py) * cos_a
    rx_frame = rx_super / SUPER
    ry_frame = ry_super / SUPER
    anchor = _frame_anchor(with_barrels, frame_size)
    final_x = anchor[0] + rx_frame + offset_px[0] * RENDER_SCALE
    final_y = anchor[1] + ry_frame + offset_px[1] * RENDER_SCALE
    return (final_x, final_y)


def frame_anchors(
    *,
    angle_deg: float,
    with_barrels: bool,
    frame_size: Tuple[int, int] = (FRAME_PX, FRAME_PX),
    offset_px: Point = (0.0, 0.0),
) -> WeaponAnchors:
    """Compute pixel-space anchors for a rendered weapon frame."""

    def p(local: Point) -> Point:
        return _project_anchor(local, angle_deg, with_barrels, frame_size, offset_px)

    blade_y0, blade_y1 = blade_y_range(with_barrels)
    tip_local: Point = (0.0, blade_y1)
    blade_base_local: Point = (0.0, blade_y0)

    grip = p(GRIP_ANCHOR_LOCAL)
    pommel = p(POMMEL_ANCHOR_LOCAL)
    guard = p(GUARD_ANCHOR_LOCAL)
    tip = p(tip_local)
    if with_barrels:
        muzzle = p(MUZZLE_ANCHOR_LOCAL)
    else:
        # For the projectile, "muzzle" doesn't apply; we still return a
        # value (the blade base) so consumers don't have to special-case
        # missing fields. Game code should ignore it for the projectile.
        muzzle = p(blade_base_local)

    rad = math.radians(angle_deg)
    forward = (math.sin(rad), -math.cos(rad))  # unit blade-direction in frame coords

    return WeaponAnchors(
        grip=grip,
        pommel=pommel,
        guard=guard,
        muzzle=muzzle,
        tip=tip,
        forward=forward,
        angle_deg=angle_deg,
    )


def downsample(layer: Image.Image, size: Tuple[int, int]) -> Image.Image:
    return layer.resize(size, Image.Resampling.LANCZOS)


# ---- Blade (leaf-shaped energy edge) ---------------------------------------


def _leaf_polygon(mid_half_w: float, y0: float, y1: float) -> list[Point]:
    """A solid leaf-shaped blade polygon centered on x=0.

    Widest at ~36% along the blade from the emitter, then a long
    gradual taper to a fine point at the tip. Multiple vertices in
    the taper region keep the point sharp at small render sizes —
    a 2-vertex taper would lose its sharpness to anti-aliasing
    after the 4× downsample.
    """
    length = y1 - y0
    widest_y = y0 + length * 0.36
    pts: list[Point] = []
    # Left edge: emitter → widest → tip (extra vertices in the taper)
    pts.append((-mid_half_w * 0.32, y0))  # emitter base, narrow
    pts.append((-mid_half_w * 0.78, y0 + length * 0.10))  # ramp out
    pts.append((-mid_half_w, widest_y))  # widest
    pts.append((-mid_half_w * 0.85, y0 + length * 0.58))  # past widest
    pts.append((-mid_half_w * 0.58, y0 + length * 0.76))  # taper start
    pts.append((-mid_half_w * 0.30, y0 + length * 0.90))  # taper mid
    pts.append((-mid_half_w * 0.10, y0 + length * 0.975))  # near tip, very narrow
    pts.append((0.0, y1))  # sharp point
    # Right edge mirrored
    pts.append((mid_half_w * 0.10, y0 + length * 0.975))
    pts.append((mid_half_w * 0.30, y0 + length * 0.90))
    pts.append((mid_half_w * 0.58, y0 + length * 0.76))
    pts.append((mid_half_w * 0.85, y0 + length * 0.58))
    pts.append((mid_half_w, widest_y))
    pts.append((mid_half_w * 0.78, y0 + length * 0.10))
    pts.append((mid_half_w * 0.32, y0))
    return pts


def draw_blade_layer(
    pulse: float = 1.0,
    slash_streak: float = 0.0,
    *,
    with_receiver: bool = True,
    tip_flare: float = 0.0,
    pulse_position: Optional[float] = None,
    pulse_intensity: float = 0.0,
) -> Image.Image:
    """Render the energy blade as a solid leaf-shaped edge.

    ``with_receiver`` controls whether the blade starts at
    ``RECEIVER_Y1`` (wielded weapon) or ``GUARD_Y1`` (projectile, no
    receiver) so the blade base is always flush with the hilt.

    ``tip_flare`` (0..1) draws a bright radial spot at the tip. Default
    is 0 (no flare) — the natural leaf-tip point is enough on its own.

    ``pulse_position`` (None or 0..1) places a bright white dot at
    that fraction along the blade, from hilt (0) to tip (1). Used by
    the fire animation to show an energy pulse traveling outward.
    ``pulse_intensity`` (0..1) scales the brightness of that dot.
    """
    blade_y0, blade_y1 = blade_y_range(with_receiver)
    blade_len = blade_y1 - blade_y0

    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    cx, cy = W / 2, H / 2

    # Outer halo
    glow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow, "RGBA")
    halo_w = BLADE_MID_HALFW * (1.45 + 0.20 * pulse)
    halo_pts = _leaf_polygon(halo_w, blade_y0 - 2.0, blade_y1 + 2.0)
    gd.polygon(
        [(cx + s(x), cy + s(y)) for (x, y) in halo_pts],
        fill=with_alpha(BLADE_HALO, int(95 * pulse)),
    )
    glow = glow.filter(ImageFilter.GaussianBlur(radius=max(5, int(SUPER * 1.5))))
    layer.alpha_composite(glow)

    # Dark rim (deep blue) — silhouette edge.
    rim_w = BLADE_MID_HALFW * (1.06 + 0.04 * pulse)
    rim_pts = _leaf_polygon(rim_w, blade_y0 + 0.5, blade_y1 + 0.5)
    rim_layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    rd = ImageDraw.Draw(rim_layer, "RGBA")
    rd.polygon(
        [(cx + s(x), cy + s(y)) for (x, y) in rim_pts],
        fill=with_alpha(BLADE_RIM, int(220 * pulse)),
    )
    layer.alpha_composite(rim_layer)

    # Primary blade body — solid cyan leaf.
    body_w = BLADE_MID_HALFW
    body_pts = _leaf_polygon(body_w, blade_y0, blade_y1)
    body_layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    bd = ImageDraw.Draw(body_layer, "RGBA")
    bd.polygon(
        [(cx + s(x), cy + s(y)) for (x, y) in body_pts],
        fill=BLADE_PRIMARY,
    )
    layer.alpha_composite(body_layer)

    # Mid tone (slight darker gradient toward edges).
    mid_pts = _leaf_polygon(body_w * 0.90, blade_y0 + 0.5, blade_y1 - 0.5)
    mid_layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    md = ImageDraw.Draw(mid_layer, "RGBA")
    md.polygon(
        [(cx + s(x), cy + s(y)) for (x, y) in mid_pts],
        fill=BLADE_MID,
    )
    mid_layer = mid_layer.filter(
        ImageFilter.GaussianBlur(radius=max(2, int(SUPER * 0.4)))
    )
    layer.alpha_composite(mid_layer)

    # Hot inner core leaf.
    inner_pts = _leaf_polygon(body_w * 0.55, blade_y0 + 2.0, blade_y1 - 1.0)
    inner_layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    id_ = ImageDraw.Draw(inner_layer, "RGBA")
    id_.polygon(
        [(cx + s(x), cy + s(y)) for (x, y) in inner_pts],
        fill=BLADE_HOT,
    )
    inner_layer = inner_layer.filter(
        ImageFilter.GaussianBlur(radius=max(2, int(SUPER * 0.6)))
    )
    layer.alpha_composite(inner_layer)

    # Bright spine — thin near-white line down the middle.
    spine_pts = _leaf_polygon(body_w * 0.18, blade_y0 + 3.0, blade_y1 - 2.0)
    spine_layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    sd_ = ImageDraw.Draw(spine_layer, "RGBA")
    sd_.polygon(
        [(cx + s(x), cy + s(y)) for (x, y) in spine_pts],
        fill=BLADE_CORE,
    )
    spine_layer = spine_layer.filter(
        ImageFilter.GaussianBlur(radius=max(1, int(SUPER * 0.3)))
    )
    layer.alpha_composite(spine_layer)

    # Tachyon streaks (legacy slash effect — left available but not
    # exercised by the default animations).
    if slash_streak > 0.01:
        streak = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        sd = ImageDraw.Draw(streak, "RGBA")
        for i in range(8):
            t = i / 8.0
            y0_ = blade_y0 + blade_len * (0.10 + 0.78 * t)
            y1_ = y0_ + blade_len * 0.07
            sd.line(
                [(cx + s(0.0), cy + s(y0_)), (cx + s(0.0), cy + s(y1_))],
                fill=with_alpha(BLADE_CORE, int(255 * slash_streak)),
                width=max(1, int(SUPER * 0.55)),
            )
        streak = streak.filter(
            ImageFilter.GaussianBlur(radius=max(1, int(SUPER * 0.45)))
        )
        layer.alpha_composite(streak)

    # Traveling pulse — bright white dot moving from hilt (0) to tip
    # (1). Used by the fire animation to show an energy pulse
    # discharging outward. The dot has a soft trail behind it
    # (toward the hilt) so it reads as motion, not a static blob.
    if pulse_position is not None and pulse_intensity > 0.01:
        intensity = max(0.0, min(1.0, pulse_intensity))
        # Allow values slightly outside [0, 1] so the dot can fade in
        # off the hilt and continue off the tip; the trail uses the
        # in-range region.
        dot_y = blade_y0 + blade_len * pulse_position
        pulse_layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        pd = ImageDraw.Draw(pulse_layer, "RGBA")
        # Trail: short capsule extending BACK toward the hilt.
        trail_len = blade_len * 0.18
        trail_y0 = dot_y - trail_len
        trail_y1 = dot_y
        n_trail = 6
        for i in range(n_trail):
            t_ = (i + 1) / n_trail
            ty = lerp(trail_y0, trail_y1, t_)
            if ty < blade_y0 or ty > blade_y1:
                continue
            tr = body_w * 0.40 * t_
            ta = int(220 * intensity * t_ * t_)
            pd.ellipse(
                (
                    cx - s(tr),
                    cy + s(ty) - s(tr * 0.7),
                    cx + s(tr),
                    cy + s(ty) + s(tr * 0.7),
                ),
                fill=with_alpha(BLADE_CORE, ta),
            )
        # Bright head dot.
        if blade_y0 - 1.0 <= dot_y <= blade_y1 + 1.0:
            head_r = body_w * 0.62
            for r_mult, a_mult in ((1.6, 0.4), (1.0, 0.85), (0.55, 1.0)):
                pr = head_r * r_mult
                pd.ellipse(
                    (
                        cx - s(pr),
                        cy + s(dot_y) - s(pr),
                        cx + s(pr),
                        cy + s(dot_y) + s(pr),
                    ),
                    fill=with_alpha(BLADE_CORE, int(245 * intensity * a_mult)),
                )
        pulse_layer = pulse_layer.filter(
            ImageFilter.GaussianBlur(radius=max(1, int(SUPER * 0.4)))
        )
        layer.alpha_composite(pulse_layer)

    # Optional tip flare. OFF by default — the leaf-tip naturally
    # tapers to a point and the bright spine takes the look home.
    # Animations can request a flare (e.g. fire impact) by passing
    # `tip_flare > 0`.
    if tip_flare > 0.001:
        tip_layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        td = ImageDraw.Draw(tip_layer, "RGBA")
        tip_y = cy + s(blade_y1)
        for r, a in [(5.5, 70), (3.0, 145), (1.6, 245)]:
            td.ellipse(
                (cx - s(r), tip_y - s(r), cx + s(r), tip_y + s(r)),
                fill=with_alpha(BLADE_CORE, int(a * tip_flare)),
            )
        tip_layer = tip_layer.filter(
            ImageFilter.GaussianBlur(radius=max(1, int(SUPER * 0.4)))
        )
        layer.alpha_composite(tip_layer)

    return layer


# ---- Hilt (chassis, grip, crossguard, receiver, emitter) -------------------


def draw_hilt_layer(
    *, crystal_pulse: float = 1.0, with_receiver: bool = True
) -> Image.Image:
    """Render the techno-hilt with grip + crossguard + (optional) receiver.

    ``crystal_pulse`` modulates the emitter at the blade base. The
    receiver is the heavy gunmetal block above the crossguard that
    houses the gun cluster on the wielded variant — for the projectile
    we skip it so the blade looks like just-blade-plus-hilt.
    """
    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer, "RGBA")
    cx, cy = W / 2, H / 2

    # --- Pommel cap ---
    pommel_w = 5.6
    pommel_h = 4.2
    pommel_outer = [
        (-pommel_w * 0.82, POMMEL_Y - pommel_h),
        (pommel_w * 0.82, POMMEL_Y - pommel_h),
        (pommel_w, POMMEL_Y),
        (-pommel_w, POMMEL_Y),
    ]
    d.polygon(
        [(cx + s(x), cy + s(y)) for (x, y) in pommel_outer],
        fill=GUN_PLATE,
        outline=GUN_DARK,
    )
    # Brass band across the pommel.
    d.line(
        [
            (cx + s(-pommel_w * 0.78), cy + s(POMMEL_Y - 1.4)),
            (cx + s(pommel_w * 0.78), cy + s(POMMEL_Y - 1.4)),
        ],
        fill=BRASS,
        width=max(1, int(SUPER * 0.65)),
    )
    d.line(
        [
            (cx + s(-pommel_w * 0.78), cy + s(POMMEL_Y - 1.4)),
            (cx + s(pommel_w * 0.78), cy + s(POMMEL_Y - 1.4)),
        ],
        fill=BRASS_HI,
        width=max(1, int(SUPER * 0.25)),
    )
    # Cyan status diode at the back of the pommel.
    pe_cy = POMMEL_Y - pommel_h * 0.55
    pe_r = 1.4
    d.ellipse(
        (cx - s(pe_r), cy + s(pe_cy) - s(pe_r), cx + s(pe_r), cy + s(pe_cy) + s(pe_r)),
        fill=EMITTER_HOT,
        outline=GUN_DARK,
    )
    d.ellipse(
        (
            cx - s(pe_r * 0.4),
            cy + s(pe_cy) - s(pe_r * 0.4),
            cx + s(pe_r * 0.4),
            cy + s(pe_cy) + s(pe_r * 0.4),
        ),
        fill=EMITTER_CORE,
    )

    # --- Grip (pistol-style mid-section) ---
    grip_pts = [
        (-GRIP_W_BOT, GRIP_Y0),
        (GRIP_W_BOT, GRIP_Y0),
        (GRIP_W_TOP, GRIP_Y1),
        (-GRIP_W_TOP, GRIP_Y1),
    ]
    d.polygon(
        [(cx + s(x), cy + s(y)) for (x, y) in grip_pts],
        fill=GUN_BODY,
        outline=GUN_DARK,
    )
    # Wrap bands — 5 thin lines for grip texture.
    for i in range(5):
        t = (i + 1) / 6.0
        y = GRIP_Y0 + HILT_LEN * t
        bw = GRIP_W_BOT + (GRIP_W_TOP - GRIP_W_BOT) * t
        d.line(
            [(cx + s(-bw * 1.06), cy + s(y)), (cx + s(bw * 1.06), cy + s(y))],
            fill=GUN_RIM,
            width=max(1, int(SUPER * 0.50)),
        )
        d.line(
            [
                (cx + s(-bw * 0.45), cy + s(y + 0.65)),
                (cx + s(bw * 0.45), cy + s(y + 0.65)),
            ],
            fill=GUN_DARK,
            width=max(1, int(SUPER * 0.30)),
        )
    # Grip outer highlight stripe.
    d.line(
        [
            (cx + s(GRIP_W_BOT * 0.88), cy + s(GRIP_Y0 + 1.5)),
            (cx + s(GRIP_W_TOP * 0.88), cy + s(GRIP_Y1 - 1.0)),
        ],
        fill=GUN_HI,
        width=max(1, int(SUPER * 0.45)),
    )

    # --- Crossguard ---
    gh = GUARD_HALFW
    guard_pts = [
        (-gh, GUARD_Y0),
        (gh, GUARD_Y0),
        (gh * 0.88, GUARD_Y1),
        (gh * 0.40, GUARD_Y1 - 0.4),
        (-gh * 0.40, GUARD_Y1 - 0.4),
        (-gh * 0.88, GUARD_Y1),
    ]
    d.polygon(
        [(cx + s(x), cy + s(y)) for (x, y) in guard_pts],
        fill=GUN_PLATE,
        outline=GUN_DARK,
    )
    # Brass border along the front face of the crossguard.
    d.line(
        [(cx + s(-gh * 0.88), cy + s(GUARD_Y1)), (cx + s(gh * 0.88), cy + s(GUARD_Y1))],
        fill=BRASS,
        width=max(1, int(SUPER * 0.55)),
    )
    # Brass rivets on the crossguard wings.
    for rx in (-gh * 0.78, -gh * 0.42, gh * 0.42, gh * 0.78):
        d.ellipse(
            (
                cx + s(rx) - s(1.05),
                cy + s(GUARD_Y0 + 1.6) - s(1.05),
                cx + s(rx) + s(1.05),
                cy + s(GUARD_Y0 + 1.6) + s(1.05),
            ),
            fill=BRASS,
            outline=GUN_DARK,
        )
        d.ellipse(
            (
                cx + s(rx) - s(0.45),
                cy + s(GUARD_Y0 + 1.6) - s(0.45),
                cx + s(rx) + s(0.45),
                cy + s(GUARD_Y0 + 1.6) + s(0.45),
            ),
            fill=BRASS_HI,
        )

    # --- Receiver (gunmetal block above the crossguard, only when
    # the gun cluster is attached). ---
    if with_receiver:
        rcv_pts = [
            (-RECEIVER_HALFW, RECEIVER_Y0),
            (RECEIVER_HALFW, RECEIVER_Y0),
            (RECEIVER_HALFW * 0.85, RECEIVER_Y1),
            (-RECEIVER_HALFW * 0.85, RECEIVER_Y1),
        ]
        d.polygon(
            [(cx + s(x), cy + s(y)) for (x, y) in rcv_pts],
            fill=GUN_PLATE,
            outline=GUN_DARK,
        )
        # Top plate (lighter band)
        plate_pts = [
            (-RECEIVER_HALFW * 0.92, RECEIVER_Y0 + 0.6),
            (RECEIVER_HALFW * 0.92, RECEIVER_Y0 + 0.6),
            (RECEIVER_HALFW * 0.85, RECEIVER_Y0 + RECEIVER_THICK * 0.45),
            (-RECEIVER_HALFW * 0.85, RECEIVER_Y0 + RECEIVER_THICK * 0.45),
        ]
        d.polygon(
            [(cx + s(x), cy + s(y)) for (x, y) in plate_pts],
            fill=GUN_PLATE_HI,
        )
        # Brass band across the receiver.
        d.line(
            [
                (
                    cx + s(-RECEIVER_HALFW * 0.85),
                    cy + s(RECEIVER_Y0 + RECEIVER_THICK * 0.65),
                ),
                (
                    cx + s(RECEIVER_HALFW * 0.85),
                    cy + s(RECEIVER_Y0 + RECEIVER_THICK * 0.65),
                ),
            ],
            fill=BRASS,
            width=max(1, int(SUPER * 0.65)),
        )
        # Cyan diodes on the receiver corners — eight little LEDs.
        for cyx, cyy in (
            (-RECEIVER_HALFW * 0.72, RECEIVER_Y0 + 1.2),
            (-RECEIVER_HALFW * 0.28, RECEIVER_Y0 + 1.2),
            (RECEIVER_HALFW * 0.28, RECEIVER_Y0 + 1.2),
            (RECEIVER_HALFW * 0.72, RECEIVER_Y0 + 1.2),
        ):
            d.ellipse(
                (
                    cx + s(cyx) - s(0.7),
                    cy + s(cyy) - s(0.7),
                    cx + s(cyx) + s(0.7),
                    cy + s(cyy) + s(0.7),
                ),
                fill=EMITTER_HOT,
                outline=GUN_DARK,
            )
        # Vent slits on the bottom row.
        for vx in (
            -RECEIVER_HALFW * 0.62,
            -RECEIVER_HALFW * 0.18,
            RECEIVER_HALFW * 0.18,
            RECEIVER_HALFW * 0.62,
        ):
            d.line(
                [
                    (cx + s(vx - 1.4), cy + s(RECEIVER_Y0 + RECEIVER_THICK * 0.85)),
                    (cx + s(vx + 1.4), cy + s(RECEIVER_Y0 + RECEIVER_THICK * 0.85)),
                ],
                fill=GUN_DARK,
                width=max(1, int(SUPER * 0.45)),
            )

    # --- Emitter band where the blade comes out --------------------------
    # Tied to the blade's actual base position so the glow sits right
    # at the seam between hilt and blade (no visible gap on the
    # projectile, which omits the receiver).
    em_y, _ = blade_y_range(with_receiver)
    emitter_layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    ed = ImageDraw.Draw(emitter_layer, "RGBA")
    em_half = BLADE_MID_HALFW * 0.65
    # Bright horizontal band glow at the emission line.
    for off, alpha in (
        (1.6, int(180 * crystal_pulse)),
        (1.0, int(220 * crystal_pulse)),
        (0.4, int(255 * min(1.0, crystal_pulse))),
    ):
        ed.ellipse(
            (
                cx + s(-em_half - off),
                cy + s(em_y - 0.8 * off / 1.6) - s(0.6),
                cx + s(em_half + off),
                cy + s(em_y - 0.8 * off / 1.6) + s(1.4),
            ),
            fill=with_alpha(EMITTER_HOT, alpha),
        )
    emitter_layer = emitter_layer.filter(
        ImageFilter.GaussianBlur(radius=max(2, int(SUPER * 0.45)))
    )
    layer.alpha_composite(emitter_layer)
    # Sharp pinpoint at emitter midline.
    d.ellipse(
        (cx - s(0.8), cy + s(em_y) - s(0.8), cx + s(0.8), cy + s(em_y) + s(0.8)),
        fill=EMITTER_CORE,
    )

    return layer


# ---- Gun cluster + stinger antennae (wielded variant only) -----------------


def draw_gun_cluster_layer(*, charge: float = 0.0, flash: float = 0.0) -> Image.Image:
    """Render the gatling-style barrel cluster + forward stinger antennae.

    The cluster sits OUTBOARD of the blade on +X side — three small
    barrels in a triangular packing pointing forward. Two thin stinger
    antennae run forward from the crossguard above and below the
    blade base for extra silhouette interest.
    """
    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer, "RGBA")
    cx, cy = W / 2, H / 2

    # --- Mounting bracket from the receiver out to the cluster -------------
    bracket_pts = [
        (RECEIVER_HALFW * 0.85, RECEIVER_Y0 + 0.5),
        (CLUSTER_OFFSET_X + 3.0, RECEIVER_Y0 + 0.5),
        (CLUSTER_OFFSET_X + 3.0, RECEIVER_Y1 + CLUSTER_OFFSET_Y * 0.5),
        (RECEIVER_HALFW * 0.85, RECEIVER_Y1 - 0.4),
    ]
    d.polygon(
        [(cx + s(x), cy + s(y)) for (x, y) in bracket_pts],
        fill=GUN_PLATE,
        outline=GUN_DARK,
    )
    # Brass rivet at the mount joint.
    d.ellipse(
        (
            cx + s(RECEIVER_HALFW * 0.78) - s(1.1),
            cy + s(RECEIVER_Y0 + 2.2) - s(1.1),
            cx + s(RECEIVER_HALFW * 0.78) + s(1.1),
            cy + s(RECEIVER_Y0 + 2.2) + s(1.1),
        ),
        fill=BRASS,
        outline=GUN_DARK,
    )

    # --- Cluster of three short barrels ------------------------------------
    # Triangular packing: one barrel forward-top, two barrels forward-bottom.
    barrel_positions = [
        (
            CLUSTER_OFFSET_X - CLUSTER_BARREL_GAP * 0.35,
            RECEIVER_Y1 + CLUSTER_OFFSET_Y - 0.6,
        ),  # top
        (
            CLUSTER_OFFSET_X - CLUSTER_BARREL_GAP * 0.05,
            RECEIVER_Y1 + CLUSTER_OFFSET_Y + CLUSTER_BARREL_GAP * 0.70,
        ),  # middle
        (
            CLUSTER_OFFSET_X + CLUSTER_BARREL_GAP * 0.55,
            RECEIVER_Y1 + CLUSTER_OFFSET_Y - 0.1,
        ),  # outer-top
    ]
    bw = CLUSTER_BARREL_W * 0.5

    for bx, by in barrel_positions:
        b_y1 = by + CLUSTER_BARREL_LEN
        # Barrel body
        body_pts = [
            (bx - bw, by),
            (bx + bw, by),
            (bx + bw * 0.85, b_y1 - 1.0),
            (bx + bw * 0.5, b_y1),
            (bx - bw * 0.5, b_y1),
            (bx - bw * 0.85, b_y1 - 1.0),
        ]
        d.polygon(
            [(cx + s(x), cy + s(y)) for (x, y) in body_pts],
            fill=GUN_BODY,
            outline=GUN_DARK,
        )
        # Bright top stripe
        d.line(
            [
                (cx + s(bx - bw * 0.55), cy + s(by + 0.6)),
                (cx + s(bx - bw * 0.55), cy + s(b_y1 - 1.4)),
            ],
            fill=GUN_HI,
            width=max(1, int(SUPER * 0.45)),
        )
        # Muzzle ring
        ring_back_y = b_y1 - 2.2
        ring_pts = [
            (bx - bw * 1.25, ring_back_y),
            (bx + bw * 1.25, ring_back_y),
            (bx + bw * 0.95, b_y1),
            (bx - bw * 0.95, b_y1),
        ]
        d.polygon(
            [(cx + s(x), cy + s(y)) for (x, y) in ring_pts],
            fill=GUN_PLATE,
            outline=GUN_DARK,
        )
        # Cyan ion glow on the front of the ring.
        d.line(
            [
                (cx + s(bx - bw * 1.15), cy + s(b_y1 - 0.9)),
                (cx + s(bx + bw * 1.15), cy + s(b_y1 - 0.9)),
            ],
            fill=with_alpha(EMITTER_HOT, int(150 + 105 * charge)),
            width=max(1, int(SUPER * 0.55)),
        )
        # Muzzle hole.
        mh_w = bw * 0.6
        mh_h = bw * 0.35
        d.ellipse(
            (
                cx + s(bx - mh_w),
                cy + s(b_y1 - mh_h),
                cx + s(bx + mh_w),
                cy + s(b_y1 + mh_h),
            ),
            fill=GUN_DARK,
            outline=GUN_DARK,
        )
        # Charge glow.
        if charge > 0.01:
            glow_layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
            gd_ = ImageDraw.Draw(glow_layer, "RGBA")
            for r, a in ((4.0, 60), (2.4, 130), (1.3, 215)):
                gd_.ellipse(
                    (
                        cx + s(bx) - s(r),
                        cy + s(b_y1 - 0.3) - s(r),
                        cx + s(bx) + s(r),
                        cy + s(b_y1 - 0.3) + s(r),
                    ),
                    fill=with_alpha(EMITTER_HOT, int(a * charge)),
                )
            glow_layer = glow_layer.filter(
                ImageFilter.GaussianBlur(radius=max(2, int(SUPER * 0.45)))
            )
            layer.alpha_composite(glow_layer)
        # Muzzle flash.
        if flash > 0.01:
            flash_layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
            fd = ImageDraw.Draw(flash_layer, "RGBA")
            fcx = cx + s(bx)
            fcy = cy + s(b_y1 + 1.4)
            spikes = []
            for k in range(10):
                ang = k / 10.0 * math.tau
                r = (5.5 if k % 2 == 0 else 2.2) * (0.45 + 0.55 * flash)
                spikes.append((fcx + math.cos(ang) * s(r), fcy + math.sin(ang) * s(r)))
            fd.polygon(spikes, fill=with_alpha(BRASS_HI, int(220 * flash)))
            fd.ellipse(
                (
                    fcx - s(2.0 * flash),
                    fcy - s(2.0 * flash),
                    fcx + s(2.0 * flash),
                    fcy + s(2.0 * flash),
                ),
                fill=with_alpha(BLADE_CORE, int(245 * flash)),
            )
            flash_layer = flash_layer.filter(
                ImageFilter.GaussianBlur(radius=max(2, int(SUPER * 0.5)))
            )
            layer.alpha_composite(flash_layer)

    # --- Drum/ammo cylinder under the cluster ------------------------------
    drum_cx = CLUSTER_OFFSET_X - 0.4
    drum_cy = RECEIVER_Y1 + CLUSTER_OFFSET_Y * 0.8
    drum_r = 3.8
    d.ellipse(
        (
            cx + s(drum_cx - drum_r),
            cy + s(drum_cy - drum_r * 0.55),
            cx + s(drum_cx + drum_r),
            cy + s(drum_cy + drum_r * 0.55),
        ),
        fill=GUN_PLATE,
        outline=GUN_DARK,
    )
    d.ellipse(
        (
            cx + s(drum_cx - drum_r * 0.45),
            cy + s(drum_cy - drum_r * 0.25),
            cx + s(drum_cx + drum_r * 0.45),
            cy + s(drum_cy + drum_r * 0.25),
        ),
        fill=BRASS,
        outline=GUN_DARK,
    )

    # --- Stinger antennae (forward rods above + below the blade) -----------
    for offset_x in (-STINGER_OFFSET_X, STINGER_OFFSET_X):
        st_y0 = GUARD_Y1 - 0.5
        st_y1 = GUARD_Y1 + STINGER_LEN
        # Rod
        d.line(
            [(cx + s(offset_x), cy + s(st_y0)), (cx + s(offset_x), cy + s(st_y1))],
            fill=GUN_HI,
            width=max(1, int(SUPER * STINGER_W * 0.7)),
        )
        # Rod outline (thin dark line for definition)
        d.line(
            [
                (cx + s(offset_x + STINGER_W * 0.5), cy + s(st_y0)),
                (cx + s(offset_x + STINGER_W * 0.5), cy + s(st_y1)),
            ],
            fill=GUN_DARK,
            width=max(1, int(SUPER * 0.25)),
        )
        # Base mount cap (a small plate at the crossguard end).
        d.rectangle(
            (
                cx + s(offset_x - 1.4),
                cy + s(st_y0 - 1.0),
                cx + s(offset_x + 1.4),
                cy + s(st_y0 + 1.0),
            ),
            fill=GUN_PLATE,
            outline=GUN_DARK,
        )
        # Tip bulb / antenna head.
        d.ellipse(
            (
                cx + s(offset_x - STINGER_TIP_BULB),
                cy + s(st_y1 - STINGER_TIP_BULB),
                cx + s(offset_x + STINGER_TIP_BULB),
                cy + s(st_y1 + STINGER_TIP_BULB),
            ),
            fill=BRASS,
            outline=GUN_DARK,
        )
        d.ellipse(
            (
                cx + s(offset_x - STINGER_TIP_BULB * 0.45),
                cy + s(st_y1 - STINGER_TIP_BULB * 0.45),
                cx + s(offset_x + STINGER_TIP_BULB * 0.45),
                cy + s(st_y1 + STINGER_TIP_BULB * 0.45),
            ),
            fill=BRASS_HI,
        )

    return layer


# ---- Composite weapon ------------------------------------------------------


def draw_weapon(
    *,
    angle_deg: float,
    pulse: float = 1.0,
    crystal_pulse: float = 1.0,
    slash_streak: float = 0.0,
    with_barrels: bool = True,
    barrel_charge: float = 0.0,
    barrel_flash: float = 0.0,
    blade_fade: float = 0.0,
    tip_flare: float = 0.0,
    pulse_position: Optional[float] = None,
    pulse_intensity: float = 0.0,
    extra_offset: Point = (0.0, 0.0),
    frame_size: Tuple[int, int] = (FRAME_PX, FRAME_PX),
) -> Image.Image:
    """Compose blade + hilt (+gun cluster) as one weapon, rotated by ``angle_deg``.

    Pipeline:
      1. Draw blade + hilt + (optional) gun cluster onto a square
         ``W × H`` working canvas with the pommel pivot at the canvas
         center. ``W`` is sized generously via ``FRAME_PX_BASE`` so
         the longest blade extent (plus halo) fits at any rotation.
      2. Rotate the working canvas about its center by ``angle_deg``.
         Pivot is the pommel; the swing arc therefore rotates around
         the grip, not the blade midpoint.
      3. Paste the rotated working canvas into a fresh ``super_size``
         output canvas — sized at ``frame_size × SUPER`` — with the
         pommel landing at the target ``_frame_anchor`` position.
         No resize step (which previously warped the geometry when
         ``frame_size`` aspect ≠ 1:1).
      4. LANCZOS-downsample the super canvas to ``frame_size``.

    ``blade_fade`` (0..1) scales the blade layer's alpha BEFORE
    compositing with the hilt, so a dissipating projectile can
    visually lose its blade while the metal hilt stays solid.

    ``extra_offset`` is in **design units** (same scale as the
    geometry constants), so e.g. a recoil offset stays visually
    proportional when ``RENDER_SCALE`` changes.
    """
    super_size = (frame_size[0] * SUPER, frame_size[1] * SUPER)

    composed = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    blade = draw_blade_layer(
        pulse=pulse,
        slash_streak=slash_streak,
        with_receiver=with_barrels,
        tip_flare=tip_flare,
        pulse_position=pulse_position,
        pulse_intensity=pulse_intensity,
    )
    if blade_fade > 0.001:
        # Multiply the blade's alpha by (1 - blade_fade) so only the
        # energy edge fades. The hilt is composited fresh at full
        # alpha below.
        keep = max(0.0, 1.0 - blade_fade)
        blade_alpha = blade.split()[-1].point(lambda v, k=keep: int(v * k))
        blade.putalpha(blade_alpha)
    composed.alpha_composite(blade)
    composed.alpha_composite(
        draw_hilt_layer(crystal_pulse=crystal_pulse, with_receiver=with_barrels)
    )
    if with_barrels:
        composed.alpha_composite(
            draw_gun_cluster_layer(charge=barrel_charge, flash=barrel_flash)
        )

    rotated = composed.rotate(
        angle_deg, expand=False, resample=Image.Resampling.BICUBIC
    )

    # Where the pommel should land in the final super canvas. The
    # frame anchor is in frame-pixel space; multiply by SUPER to
    # cross into super-pixel space. ``extra_offset`` is in design
    # units, so it scales through ``s()``-style conversion
    # (SUPER × RENDER_SCALE).
    anchor = _frame_anchor(with_barrels, frame_size)
    anchor_super = (
        anchor[0] * SUPER + extra_offset[0] * SUPER * RENDER_SCALE,
        anchor[1] * SUPER + extra_offset[1] * SUPER * RENDER_SCALE,
    )

    # In the rotated working canvas the pommel (rotation pivot)
    # sits at the canvas center. To land it at ``anchor_super`` in
    # the output, paste with offset = anchor_super - canvas_center.
    paste_x = int(round(anchor_super[0] - W / 2.0))
    paste_y = int(round(anchor_super[1] - H / 2.0))

    canvas_super = Image.new("RGBA", super_size, (0, 0, 0, 0))
    canvas_super.alpha_composite(rotated, (paste_x, paste_y))

    return downsample(canvas_super, frame_size)
