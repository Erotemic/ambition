"""Candidate player robot rebuilt on the bone/keyframe toolkit.

First demonstration of ``ambition_sprite2d_renderer.authoring.skeleton``: the chibi
robot's parts (head, torso, pelvis, limbs, feet) are rounded polygons and
capsules attached to an FK bone tree; idle/walk/slash are authored as
keyframe clips, and the legs are driven by *foot trajectories* solved
through two-bone IK so planted feet never slide.

Conventions match the existing player sheet: right-facing, 128x128 frames,
ground line at y=101, same palette as ``robot_side`` (white shell, cyan
visor glow, purple accents). Render rows are idle / walk / slash only —
the runtime maps missing rows to idle.

    python -m ambition_sprite2d_renderer publish player_robot_fable
    ./regen_sprites.sh --target player_robot_fable
"""

from __future__ import annotations

import math
from pathlib import Path
from typing import Dict, List, Tuple

from PIL import Image, ImageDraw, ImageColor

from ...authoring.common_draw import draw_capsule
from ...authoring.rig import add, clamp, lerp, smoothstep, vec
from ...authoring.skeleton import (
    Channel,
    Clip,
    PartCtx,
    Rig,
    Skeleton,
    composite_polygon,
    draw_polygon,
    rounded_polygon,
    two_bone_ik,
)
from ...authoring.tackon_sheet import build_sheet, write_canonical

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_NAME = "player_robot_fable"
FRAME_W, FRAME_H = 128, 128
SS = 4  # supersample factor; draw at 512px, LANCZOS down to 128px

GROUND_Y = 101.0
CENTER_X = 64.0
ANKLE_H = 2.6  # ankle height above the ground line when the foot is flat

LEG_U, LEG_L = 10.0, 8.5
ARM_U, ARM_L = 9.5, 8.0
BLADE_LEN = 26.0
OUTLINE_W = 1.15  # in 128-frame units

ROWS: List[Tuple[str, int, int]] = [
    ("idle", 8, 120),
    ("walk", 8, 95),
    ("slash", 8, 75),
]
LOOPS = {"idle", "walk"}

ACTOR_METADATA = {
    "actor": {"character_id": "player_robot_fable", "display_name": "Player Robot (Fable)"},
    "body": {
        "body_plan": "HumanoidBiped",
        "body_kind": "Standard",
        "traits": ["robot", "player_candidate"],
    },
    "visual": {"default_pose": "idle"},
    "tags": ["robot", "candidate"],
}


def _rgba(hex_color: str, alpha: int = 255) -> Color:
    r, g, b = ImageColor.getrgb(hex_color)
    return (r, g, b, alpha)


PAL: Dict[str, Color] = {
    "shell": _rgba("#FDFDFB"),
    "shell_top": _rgba("#FFFFFF"),
    "shell_side": _rgba("#E8E2DB"),
    "outline": _rgba("#17191F"),
    "joint": _rgba("#EDEAE4"),
    "joint_dark": _rgba("#DDD9D1"),
    "visor": _rgba("#0B111C"),
    "glow": _rgba("#0CEBFF"),
    "accent": _rgba("#C58AFF"),
    "accent_dark": _rgba("#8E56D8"),
}


# ---- Skeleton ----------------------------------------------------------------
# Root sits on the ground line at the frame center; everything hangs off the
# pelvis. Offsets are parent-local (rotated with the parent), so a torso lean
# carries the head and shoulders along automatically.


def _build_skeleton() -> Skeleton:
    sk = Skeleton()
    sk.bone("pelvis", offset=(0.0, -20.5))
    sk.bone("torso", parent="pelvis", offset=(0.0, -4.0))
    sk.bone("head", parent="torso", offset=(1.0, -26.0))
    sk.bone("antenna", parent="head", offset=(-9.0, -14.5), length=10.0, rest_angle=-90.0)
    sk.bone("far_arm_u", parent="torso", offset=(-1.5, -9.5), length=ARM_U, rest_angle=90.0)
    sk.bone("far_arm_l", parent="far_arm_u", offset=(ARM_U, 0.0), length=ARM_L)
    sk.bone("near_arm_u", parent="torso", offset=(3.0, -9.5), length=ARM_U, rest_angle=90.0)
    sk.bone("near_arm_l", parent="near_arm_u", offset=(ARM_U, 0.0), length=ARM_L)
    sk.bone("far_leg_u", parent="pelvis", offset=(-2.0, 2.0), length=LEG_U, rest_angle=90.0)
    sk.bone("far_leg_l", parent="far_leg_u", offset=(LEG_U, 0.0), length=LEG_L)
    sk.bone("far_foot", parent="far_leg_l", offset=(LEG_L, 0.0), length=6.0, rest_angle=-90.0)
    sk.bone("near_leg_u", parent="pelvis", offset=(2.5, 2.0), length=LEG_U, rest_angle=90.0)
    sk.bone("near_leg_l", parent="near_leg_u", offset=(LEG_U, 0.0), length=LEG_L)
    sk.bone("near_foot", parent="near_leg_l", offset=(LEG_L, 0.0), length=6.0, rest_angle=-90.0)
    return sk


_SKEL = _build_skeleton()


# ---- Parts -------------------------------------------------------------------


def _limb_painter(upper: str, lower: str, tint_u: Color, tint_l: Color, r_u: float, r_l: float, hand_r: float = 0.0):
    def fn(ctx: PartCtx) -> None:
        u, low = ctx.world[upper], ctx.world[lower]
        ow = ctx.L(0.55)
        draw_capsule(ctx.draw, ctx.cw(u.origin), ctx.cw(u.tip), ctx.L(r_u), tint_u, PAL["outline"], ow)
        draw_capsule(ctx.draw, ctx.cw(low.origin), ctx.cw(low.tip), ctx.L(r_l), tint_l, PAL["outline"], ow)
        # Joint cap at the knee/elbow so the seam reads as a hinge.
        jx, jy = ctx.cw(low.origin)
        jr = ctx.L(r_u * 0.62)
        ctx.draw.ellipse((jx - jr, jy - jr, jx + jr, jy + jr), fill=PAL["joint_dark"])
        if hand_r > 0:
            hx, hy = ctx.cw(low.tip)
            hr = ctx.L(hand_r)
            ctx.draw.ellipse(
                (hx - hr, hy - hr, hx + hr, hy + hr),
                fill=PAL["shell"],
                outline=PAL["outline"],
                width=max(1, int(ctx.L(0.5))),
            )
    return fn


def _foot_painter():
    # Boot in foot-local coords: origin at the ankle, +x toward the toe,
    # sole at +y ANKLE_H when flat.
    pts = [(-2.6, -1.0), (4.2, -1.0), (6.0, 0.9), (5.6, 2.6), (-2.2, 2.6)]

    def fn(ctx: PartCtx) -> None:
        poly = rounded_polygon(ctx.pts(pts), radius=ctx.L(1.1))
        draw_polygon(ctx.draw, poly, PAL["shell"], PAL["outline"], ctx.L(OUTLINE_W * 0.8))
    return fn


def _pelvis_painter(ctx: PartCtx) -> None:
    # Slight forward (+x) taper so the hips read as facing right.
    pts = [(-8.6, -2.8), (9.2, -2.8), (8.4, 4.2), (-7.4, 4.2)]
    poly = rounded_polygon(ctx.pts(pts), radius=ctx.L(2.6))
    draw_polygon(ctx.draw, poly, PAL["joint"], PAL["outline"], ctx.L(OUTLINE_W * 0.8))


def _torso_painter(ctx: PartCtx) -> None:
    # Chest/belly edge bulges toward +x (the facing direction); the back
    # edge stays straighter. Shade rides the BACK edge, highlight the front.
    pts = [(-9.8, -13.5), (10.2, -13.5), (11.4, -6.0), (9.6, 1.5), (-8.8, 1.5)]
    poly = rounded_polygon(ctx.pts(pts), radius=ctx.L(3.6))
    draw_polygon(ctx.draw, poly, PAL["shell"], PAL["outline"], ctx.L(OUTLINE_W))
    # Translucent details go through composite_polygon (scratch layer +
    # alpha_composite) — drawing them directly would clobber the alpha.
    shade = [(-9.0, -12.6), (-5.6, -12.8), (-5.0, 0.8), (-8.0, 0.8)]
    composite_polygon(ctx.img, rounded_polygon(ctx.pts(shade), radius=ctx.L(2.0)), (*PAL["shell_side"][:3], 110))
    hi = [(5.4, -12.8), (9.8, -12.6), (10.8, -6.4), (9.2, -2.2), (5.8, -2.6)]
    composite_polygon(ctx.img, rounded_polygon(ctx.pts(hi), radius=ctx.L(2.0)), (255, 255, 255, 90))
    # Chest light (the concept's teal square), mounted on the front plate.
    light = [(3.2, -9.4), (7.6, -9.4), (7.6, -5.0), (3.2, -5.0)]
    draw_polygon(
        ctx.draw,
        rounded_polygon(ctx.pts(light), radius=ctx.L(1.0)),
        PAL["glow"],
        PAL["outline"],
        ctx.L(0.55),
    )


def _head_painter(ctx: PartCtx) -> None:
    p = ctx.params
    shell = [(-18.0, -14.5), (18.0, -14.5), (18.0, 14.5), (-18.0, 14.5)]
    poly = rounded_polygon(ctx.pts(shell), radius=ctx.L(8.5), steps=8)
    draw_polygon(ctx.draw, poly, PAL["shell_top"], PAL["outline"], ctx.L(OUTLINE_W))
    # Lower-jaw shading band.
    jaw = [(-16.0, 6.5), (16.0, 6.5), (15.0, 13.0), (-15.0, 13.0)]
    composite_polygon(ctx.img, rounded_polygon(ctx.pts(jaw), radius=ctx.L(3.0)), (*PAL["shell_side"][:3], 120))
    # Visor.
    visor = [(-5.5, -5.8), (16.5, -5.8), (16.5, 4.8), (-5.5, 4.8)]
    draw_polygon(
        ctx.draw,
        rounded_polygon(ctx.pts(visor), radius=ctx.L(3.6)),
        PAL["visor"],
        PAL["outline"],
        ctx.L(0.7),
    )
    # Eyes: two cyan pills; blink squashes them to slivers, squint narrows.
    blink = p.get("blink", 0.0) > 0.5
    squint = clamp(p.get("eye_squint", 0.0), 0.0, 1.0)
    eye_h = 6.4 * (1.0 - 0.55 * squint)
    if blink:
        eye_h = 1.2
    for ex in (1.6, 9.4):
        c = ctx.pt((ex, -0.5))
        w = ctx.L(3.4)
        h = ctx.L(eye_h)
        ctx.draw.ellipse((c[0] - w / 2, c[1] - h / 2, c[0] + w / 2, c[1] + h / 2), fill=PAL["glow"])


def _antenna_painter(ctx: PartCtx) -> None:
    a, b = ctx.cw(ctx.bw.origin), ctx.cw(ctx.bw.tip)
    draw_capsule(ctx.draw, a, b, ctx.L(0.9), PAL["accent_dark"], PAL["outline"], ctx.L(0.4))
    r = ctx.L(3.0)
    ctx.draw.ellipse(
        (b[0] - r, b[1] - r, b[0] + r, b[1] + r),
        fill=PAL["accent"],
        outline=PAL["outline"],
        width=max(1, int(ctx.L(0.55))),
    )


def _build_rig() -> Rig:
    rig = Rig(_SKEL)
    rig.part("far_arm", "far_arm_u", 10, _limb_painter("far_arm_u", "far_arm_l", PAL["joint_dark"], PAL["shell_side"], 2.3, 2.1, hand_r=3.0))
    rig.part("far_leg", "far_leg_u", 20, _limb_painter("far_leg_u", "far_leg_l", PAL["joint_dark"], PAL["shell_side"], 2.7, 2.5))
    rig.part("far_foot", "far_foot", 21, _foot_painter())
    rig.part("pelvis", "pelvis", 30, _pelvis_painter)
    rig.part("torso", "torso", 40, _torso_painter)
    rig.part("near_leg", "near_leg_u", 50, _limb_painter("near_leg_u", "near_leg_l", PAL["joint"], PAL["shell"], 2.7, 2.5))
    rig.part("near_foot", "near_foot", 51, _foot_painter())
    rig.part("head", "head", 60, _head_painter)
    rig.part("antenna", "antenna", 61, _antenna_painter)
    rig.part("near_arm", "near_arm_u", 70, _limb_painter("near_arm_u", "near_arm_l", PAL["joint"], PAL["shell"], 2.3, 2.1, hand_r=3.2))
    return rig


_RIG = _build_rig()


# ---- Clips -------------------------------------------------------------------
# Channel names matching bones are pose angles (degrees). Free parameters:
#   root_x / root_y           — root offset from (CENTER_X, GROUND_Y)
#   {side}_foot_x             — ankle x offset from CENTER_X (world, NOT root-
#                               relative: planted feet stay put while the body
#                               sways over them)
#   {side}_foot_lift          — ankle rise above standing height
#   {side}_foot_pitch         — foot world angle (0 = flat, -ve = toe up)
#   blink / eye_squint / slash_vis / blade_pitch — part painter params

_TAU = math.tau


def _stride(phase_off: float, hip_off: float, duty: float = 0.55, half: float = 5.5, lift_h: float = 4.6):
    """Foot-trajectory callables for one leg of an in-place walk cycle.

    Stance (``ph < duty``): the foot is on the ground moving backward at
    constant speed — the treadmill counterpart of forward locomotion. The
    final stretch of stance is toe-off: the ankle rises and the foot pitches
    forward while the toe stays at ground level. Swing: the foot arcs
    forward, toe tucked, landing toe-up into the next heel strike."""

    def x(t: float) -> float:
        ph = (t + phase_off) % 1.0
        if ph < duty:
            return hip_off + lerp(half, -half, ph / duty)
        u = (ph - duty) / (1.0 - duty)
        return hip_off + lerp(-half, half, smoothstep(u))

    def lift(t: float) -> float:
        ph = (t + phase_off) % 1.0
        if ph < duty:
            # Toe-off ankle rise (heel leaves the ground while the toe pivots).
            if ph > 0.40:
                return lerp(0.0, 1.8, (ph - 0.40) / (duty - 0.40))
            return 0.0
        u = (ph - duty) / (1.0 - duty)
        return lerp(1.8, 0.0, u) + lift_h * math.sin(math.pi * u)

    def pitch(t: float) -> float:
        ph = (t + phase_off) % 1.0
        if ph < 0.10:
            return lerp(-14.0, 0.0, ph / 0.10)  # heel-strike settle
        if ph < 0.40:
            return 0.0  # flat mid-stance
        if ph < duty:
            return lerp(0.0, 24.0, (ph - 0.40) / (duty - 0.40))  # toe-off
        u = (ph - duty) / (1.0 - duty)
        return lerp(24.0, -14.0, smoothstep(u))  # swing tuck into heel strike

    return x, lift, pitch


_NEAR_X, _NEAR_LIFT, _NEAR_PITCH = _stride(0.0, 2.5)
_FAR_X, _FAR_LIFT, _FAR_PITCH = _stride(0.5, -2.0)


CLIP_IDLE = Clip(
    loop=True,
    channels={
        "root_y": lambda t: 1.3 * math.sin(_TAU * t),
        "root_x": lambda t: 0.9 * math.sin(_TAU * t + 0.7),
        "pelvis": lambda t: 1.3 * math.sin(_TAU * t),
        "torso": lambda t: 2.8 * math.sin(_TAU * t),
        # Head and antenna trail the torso with increasing phase lag so the
        # bob whips through the chain instead of moving as one rigid block.
        "head": lambda t: -3.4 * math.sin(_TAU * (t - 0.10)),
        "antenna": lambda t: -15.0 * math.sin(_TAU * (t - 0.20)),
        # Elbows hinge backward: a relaxed forearm angles the hand slightly
        # FORWARD of the upper arm, so the bend pose is negative.
        "near_arm_u": lambda t: 6.0 * math.sin(_TAU * t),
        "near_arm_l": lambda t: -15.0 - 4.5 * math.sin(_TAU * (t - 0.08)),
        "far_arm_u": lambda t: -5.0 * math.sin(_TAU * t),
        "far_arm_l": lambda t: -15.0 - 4.0 * math.sin(_TAU * (t - 0.12)),
        "near_foot_x": 5.0,
        "far_foot_x": -4.5,
        "blink": Channel((0.0, 0), (0.42, 0, "linear"), (0.48, 1, "linear"), (0.58, 1, "linear"), (0.64, 0, "linear")),
        "eye_squint": lambda t: 0.05 + 0.05 * math.sin(_TAU * t),
    },
)

CLIP_WALK = Clip(
    loop=True,
    channels={
        # Body lowest at each foot contact (t=0, 0.5), highest at passing.
        "root_y": lambda t: 1.5 * math.cos(2.0 * _TAU * t) - 0.3,
        "pelvis": lambda t: 3.2 * math.sin(_TAU * t),
        "torso": lambda t: 5.0 - 2.8 * math.sin(_TAU * t),
        "head": lambda t: -4.0 + 2.0 * math.sin(_TAU * (t - 0.10)),
        "antenna": lambda t: -13.0 * math.sin(2.0 * _TAU * (t - 0.09)),
        # Arms counter-swing their same-side legs; elbows hinge backward and
        # bend deepest when the arm swings forward.
        "near_arm_u": lambda t: 27.0 * math.cos(_TAU * t),
        "near_arm_l": lambda t: -(16.0 - 11.0 * math.cos(_TAU * (t - 0.04))),
        "far_arm_u": lambda t: -27.0 * math.cos(_TAU * t),
        "far_arm_l": lambda t: -(16.0 + 11.0 * math.cos(_TAU * (t - 0.04))),
        "near_foot_x": _NEAR_X,
        "near_foot_lift": _NEAR_LIFT,
        "near_foot_pitch": _NEAR_PITCH,
        "far_foot_x": _FAR_X,
        "far_foot_lift": _FAR_LIFT,
        "far_foot_pitch": _FAR_PITCH,
        "eye_squint": 0.08,
    },
)

# Forward tilt (smash-style f-tilt): standing forward swing — no crouch,
# root_y stays on the ground line. The near arm cocks up-back behind the
# head, sweeps OVER THE TOP (pose angle increases monotonically through a
# full forward circle, ending at 368 == 8 mod 360 so it lands back on the
# idle hang), and strikes out to a forward-horizontal blade. The windup
# blade is rendered behind the body — see render_frame's layer swap.
CLIP_SLASH = Clip(
    loop=False,
    channels={
        "root_x": Channel((0, 0), (0.2, -1.6), (0.5, 3.4, "out"), (0.8, 2.2), (1, 0)),
        "pelvis": Channel((0, 0), (0.2, -3), (0.5, 6, "out"), (1, 0)),
        "torso": Channel((0, 0), (0.2, -7), (0.5, 11, "out"), (0.75, 7), (1, 0)),
        "head": Channel((0, 0), (0.2, -4), (0.5, 6, "out"), (1, 0)),
        "near_arm_u": Channel((0, 8), (0.12, 55), (0.22, 120), (0.34, 120), (0.5, 263, "out"), (0.74, 300), (1, 368)),
        "near_arm_l": Channel((0, -15), (0.12, 28), (0.22, 60), (0.34, 60), (0.5, -5, "out"), (0.74, 24), (1, -15)),
        "far_arm_u": Channel((0, -4), (0.22, -32), (0.5, 28, "out"), (0.78, 12), (1, -4)),
        "far_arm_l": Channel((0, -15), (0.22, 18), (0.5, -12), (1, -15)),
        "antenna": Channel((0, 0), (0.25, 12), (0.52, -18, "out"), (0.78, -6), (1, 0)),
        "near_foot_x": 7.0,
        "far_foot_x": -5.5,
        # Back heel pivots up through the lunge; ankle rises with the pitch so
        # the toe stays at ground level.
        "far_foot_pitch": Channel((0, 0), (0.35, 0), (0.5, 22, "out"), (0.8, 12), (1, 0)),
        "far_foot_lift": Channel((0, 0), (0.35, 0), (0.5, 2.2, "out"), (0.8, 1.2), (1, 0)),
        "slash_vis": Channel((0, 0), (0.1, 0), (0.2, 1, "out"), (0.62, 1), (0.82, 0), (1, 0)),
        # Blade rides nearly along the forearm; windup holds it angled
        # slightly back so the cocked silhouette pokes up behind the head.
        "blade_pitch": Channel((0, -15), (0.2, -45), (0.34, -45), (0.5, 5, "out"), (1, 0)),
        "eye_squint": Channel((0, 0.1), (0.3, 0.25), (0.5, 0.45), (1, 0.1)),
    },
)

CLIPS: Dict[str, Clip] = {"idle": CLIP_IDLE, "walk": CLIP_WALK, "slash": CLIP_SLASH}

_DEFAULT_FOOT_X = {"near": 5.0, "far": -4.5}


def _foot_target(sampled: Dict[str, float], side: str) -> Point:
    ax = CENTER_X + sampled.get(f"{side}_foot_x", _DEFAULT_FOOT_X[side])
    ay = GROUND_Y - ANKLE_H - sampled.get(f"{side}_foot_lift", 0.0)
    return (ax, ay)


def _solve(animation: str, t: float):
    """Sample the clip, run leg IK, and return (bone worlds, params)."""
    sampled = CLIPS[animation].sample(t)
    root = (CENTER_X + sampled.get("root_x", 0.0), GROUND_Y + sampled.get("root_y", 0.0))
    angles = {name: val for name, val in sampled.items() if name in _SKEL.bones}
    w0 = _SKEL.world(angles, root=root)
    for side in ("far", "near"):
        hip = w0[f"{side}_leg_u"].origin
        ankle = _foot_target(sampled, side)
        a1, a2 = two_bone_ik(hip, ankle, LEG_U, LEG_L, bend=1.0)
        pelvis_angle = w0["pelvis"].angle
        angles[f"{side}_leg_u"] = a1 - pelvis_angle - 90.0
        angles[f"{side}_leg_l"] = a2 - a1
        pitch = sampled.get(f"{side}_foot_pitch", 0.0)
        angles[f"{side}_foot"] = pitch - a2 + 90.0
    world = _SKEL.world(angles, root=root)
    return world, sampled


def _blade_line(world, params) -> Tuple[Point, Point, float]:
    hand = world["near_arm_l"].tip
    ang = world["near_arm_l"].angle + params.get("blade_pitch", 0.0)
    vis = clamp(params.get("slash_vis", 0.0), 0.0, 1.0)
    length = BLADE_LEN * (0.35 + 0.65 * vis)
    return hand, add(hand, vec(length, ang)), vis


def _draw_slash_fx(draw: ImageDraw.ImageDraw, t: float, world, params) -> None:
    hand, tip, vis = _blade_line(world, params)
    if vis <= 0.01:
        return

    def c(p: Point) -> Point:
        return (p[0] * SS, p[1] * SS)

    # Smear during the over-the-top sweep: tapered swish strokes traced
    # along the recent blade-tip path (and a fainter inner arc), like the
    # concept art's slash lines — NOT a filled wedge.
    if 0.36 <= t <= 0.55:
        tips: List[Point] = []
        mids: List[Point] = []
        steps = 10
        for k in range(steps + 1):
            tt = t - 0.10 * (1.0 - k / steps)
            w2, p2 = _solve("slash", max(0.0, tt))
            h2, tp2, v2 = _blade_line(w2, p2)
            if v2 > 0.3:
                tips.append(tp2)
                mids.append((lerp(h2[0], tp2[0], 0.72), lerp(h2[1], tp2[1], 0.72)))
        for path, w_head, a_head in ((tips, 2.6, 200), (mids, 1.5, 110)):
            n = len(path)
            for i in range(n - 1):
                fade = (i + 1) / max(1, n - 1)  # 0 tail -> 1 head
                draw.line(
                    [c(path[i]), c(path[i + 1])],
                    fill=(120, 245, 255, int(a_head * (0.18 + 0.82 * fade))),
                    width=max(1, int(SS * w_head * (0.3 + 0.7 * fade))),
                )

    draw.line([c(hand), c(tip)], fill=PAL["outline"], width=max(1, int(3.4 * SS)))
    draw.line([c(hand), c(tip)], fill=PAL["accent"], width=max(1, int(2.0 * SS)))
    core_tip = (lerp(hand[0], tip[0], 0.8), lerp(hand[1], tip[1], 0.8))
    draw.line([c(hand), c(core_tip)], fill=(255, 255, 255, 230), width=max(1, int(0.9 * SS)))


def render_frame(animation: str, frame_idx: int, nframes: int) -> Image.Image:
    # Looping rows sample t = i/n so the last frame is NOT a duplicate of the
    # first; one-shots sample i/(n-1) so the final pose is reached exactly.
    if animation in LOOPS:
        t = frame_idx / max(1, nframes)
    else:
        t = frame_idx / max(1, nframes - 1)
    img = Image.new("RGBA", (FRAME_W * SS, FRAME_H * SS), (0, 0, 0, 0))
    world, params = _solve(animation, t)
    actor = Image.new("RGBA", img.size, (0, 0, 0, 0))
    # Opaque parts draw directly; translucent details inside the painters
    # go through composite_polygon (scratch layer + alpha_composite).
    _RIG.draw(actor, ImageDraw.Draw(actor), world, SS, params)
    if animation == "slash":
        fx = Image.new("RGBA", img.size, (0, 0, 0, 0))
        _draw_slash_fx(ImageDraw.Draw(fx), t, world, params)
        # During the windup the cocked blade sits behind the body so it
        # never crosses the face; from the sweep onward it leads the swing.
        layers = (fx, actor) if t < 0.40 else (actor, fx)
        for layer in layers:
            img.alpha_composite(layer)
    else:
        img.alpha_composite(actor)
    return img.resize((FRAME_W, FRAME_H), Image.Resampling.LANCZOS)


# ---- Target registration hooks -------------------------------------------------


def render(out_dir: Path, **opts) -> List[Path]:
    del opts
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=render_frame,
        out_dir=Path(out_dir),
        frame_size=(FRAME_W, FRAME_H),
        actor_metadata=ACTOR_METADATA,
    )
    keys = ("spritesheet", "yaml", "ron", "actor", "canonical", "canonical_transparent", "preview")
    return [Path(outputs[k]) for k in keys if outputs.get(k)]


def render_canonical(out_dir: Path, **opts) -> Path:
    del opts
    return write_canonical(TARGET_NAME, ROWS, render_frame, Path(out_dir))
