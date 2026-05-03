from __future__ import annotations

"""Polished 2.5D procedural robot generator.

The robot target is a stylized, side-facing enemy sprite system.  It uses a
small 3D-aware rig, orthographic projection, and painterly 2D rendering to keep
head, visor, limbs, depth ordering, and effects spatially consistent across
animations.
"""

import math
import random
from dataclasses import asdict, dataclass
from typing import Dict, Iterable, List, Optional, Sequence, Tuple

from PIL import Image, ImageColor, ImageDraw, ImageFilter, ImageFont

try:
    RESAMPLING = Image.Resampling
except AttributeError:  # pragma: no cover
    RESAMPLING = Image


Color = Tuple[int, int, int, int]
Point2 = Tuple[float, float]
Point3 = Tuple[float, float, float]


# -----------------------------------------------------------------------------
# Utility
# -----------------------------------------------------------------------------


def clamp(v: float, lo: float, hi: float) -> float:
    return max(lo, min(hi, v))



def lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t



def smoothstep(t: float) -> float:
    t = clamp(t, 0.0, 1.0)
    return t * t * (3 - 2 * t)



def ease_out_cubic(t: float) -> float:
    t = clamp(t, 0.0, 1.0)
    return 1.0 - (1.0 - t) ** 3



def ease_in_out_sine(t: float) -> float:
    t = clamp(t, 0.0, 1.0)
    return -(math.cos(math.pi * t) - 1.0) / 2.0



def pingpong01(t: float) -> float:
    t = t % 1.0
    return t * 2.0 if t <= 0.5 else 2.0 - t * 2.0



def rgb(hex_color: str, alpha: int = 255) -> Color:
    r, g, b = ImageColor.getrgb(hex_color)
    return (r, g, b, alpha)



def with_alpha(color: Color, alpha: int) -> Color:
    return (color[0], color[1], color[2], alpha)



def lighten(color: Color, amount: float) -> Color:
    r, g, b, a = color
    return (
        int(clamp(lerp(r, 255, amount), 0, 255)),
        int(clamp(lerp(g, 255, amount), 0, 255)),
        int(clamp(lerp(b, 255, amount), 0, 255)),
        a,
    )



def darken(color: Color, amount: float) -> Color:
    r, g, b, a = color
    return (
        int(clamp(r * (1.0 - amount), 0, 255)),
        int(clamp(g * (1.0 - amount), 0, 255)),
        int(clamp(b * (1.0 - amount), 0, 255)),
        a,
    )



def parse_background(value: str) -> Optional[Color]:
    return None if value.lower() == "transparent" else rgb(value)


def load_font_preferred(size: int):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size=max(8, int(size)))
        except OSError:
            pass
    return ImageFont.load_default()



def add2(a: Point2, b: Point2) -> Point2:
    return (a[0] + b[0], a[1] + b[1])



def add3(a: Point3, b: Point3) -> Point3:
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])



def sub3(a: Point3, b: Point3) -> Point3:
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])



def mul3(a: Point3, s: float) -> Point3:
    return (a[0] * s, a[1] * s, a[2] * s)



def cross(a: Point3, b: Point3) -> Point3:
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )



def dot(a: Point3, b: Point3) -> float:
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]



def rotate_x(p: Point3, degrees: float) -> Point3:
    a = math.radians(degrees)
    c = math.cos(a)
    s = math.sin(a)
    return (p[0], p[1] * c - p[2] * s, p[1] * s + p[2] * c)



def rotate_y(p: Point3, degrees: float) -> Point3:
    a = math.radians(degrees)
    c = math.cos(a)
    s = math.sin(a)
    return (p[0] * c + p[2] * s, p[1], -p[0] * s + p[2] * c)



def rotate_z(p: Point3, degrees: float) -> Point3:
    a = math.radians(degrees)
    c = math.cos(a)
    s = math.sin(a)
    return (p[0] * c - p[1] * s, p[0] * s + p[1] * c, p[2])



def transform_local_point(
    p: Point3,
    translate: Point3 = (0.0, 0.0, 0.0),
    rot_x_deg: float = 0.0,
    rot_y_deg: float = 0.0,
    rot_z_deg: float = 0.0,
) -> Point3:
    q = p
    if rot_x_deg:
        q = rotate_x(q, rot_x_deg)
    if rot_y_deg:
        q = rotate_y(q, rot_y_deg)
    if rot_z_deg:
        q = rotate_z(q, rot_z_deg)
    return add3(q, translate)



def segment_point(a: Point2, b: Point2, t: float) -> Point2:
    return (lerp(a[0], b[0], t), lerp(a[1], b[1], t))



def rounded_rect_path(w: float, h: float, r: float, samples_per_corner: int = 7) -> List[Point2]:
    r = max(0.0, min(r, min(w, h) / 2.0 - 1e-6))
    xs = (-w / 2.0 + r, w / 2.0 - r)
    ys = (-h / 2.0 + r, h / 2.0 - r)
    corners = [
        (xs[1], ys[0], -90, 0),
        (xs[1], ys[1], 0, 90),
        (xs[0], ys[1], 90, 180),
        (xs[0], ys[0], 180, 270),
    ]
    pts: List[Point2] = []
    for cx, cy, a0, a1 in corners:
        for i in range(samples_per_corner + 1):
            t = i / float(samples_per_corner)
            a = math.radians(lerp(a0, a1, t))
            pts.append((cx + math.cos(a) * r, cy + math.sin(a) * r))
    return pts



def ellipse_path(rx: float, ry: float, samples: int = 16) -> List[Point2]:
    pts = []
    for i in range(samples):
        a = i / float(samples) * math.tau
        pts.append((math.cos(a) * rx, math.sin(a) * ry))
    return pts


def angled_vector(length: float, degrees: float) -> Point3:
    a = math.radians(degrees)
    return (math.cos(a) * length, -math.sin(a) * length, 0.0)


# -----------------------------------------------------------------------------
# Data
# -----------------------------------------------------------------------------


@dataclass
class Palette:
    shell: Color
    shell_top: Color
    shell_side: Color
    shell_shadow: Color
    shell_rim: Color
    outline: Color
    visor: Color
    visor_glow: Color
    accent: Color
    accent_dark: Color
    joint: Color
    joint_dark: Color
    metal: Color
    energy: Color
    energy_soft: Color
    shadow: Color


@dataclass
class BotSpec:
    target: str
    seed: int
    archetype: str
    palette_name: str
    head_w: float
    head_h: float
    head_d: float
    head_round: float
    body_w: float
    body_h: float
    body_d: float
    body_round: float
    shoulder_span: float
    hip_span: float
    arm_upper: float
    arm_lower: float
    leg_upper: float
    leg_lower: float
    arm_z_back: float
    arm_z_front: float
    leg_z_back: float
    leg_z_front: float
    arm_width: float
    leg_width: float
    hand_r: float
    joint_r: float
    foot_w: float
    foot_h: float
    foot_d: float
    visor_w: float
    visor_h: float
    visor_round: float
    visor_inset_y: float
    eye_w: float
    eye_h: float
    eye_gap: float
    antenna_h: float
    antenna_ball: float
    chest_w: float
    chest_h: float
    chest_style: str
    mood: str
    blade_len: float
    panel_w: float
    panel_h: float
    char_yaw: float
    char_pitch: float
    head_yaw: float
    head_pitch: float


@dataclass
class Pose:
    root_x: float = 0.0
    root_y: float = 0.0
    root_tilt: float = 0.0
    head_yaw_delta: float = 0.0
    head_pitch_delta: float = 0.0
    head_roll: float = 0.0
    body_bob: float = 0.0
    blink: bool = False
    eye_squint: float = 0.0
    left_arm_upper: float = 152.0
    left_arm_lower: float = 160.0
    right_arm_upper: float = 28.0
    right_arm_lower: float = 20.0
    left_leg_upper: float = 114.0
    left_leg_lower: float = 98.0
    right_leg_upper: float = 68.0
    right_leg_lower: float = 82.0
    speed_lines: float = 0.0
    blade: float = 0.0
    blade_arc: float = 0.0
    blade_vertical: float = 0.0
    hologram: float = 0.0
    pedestal_console: bool = False
    particles: float = 0.0
    collapse: float = 0.0
    eye_x_amount: float = 0.0
    eye_power: float = 1.0


# -----------------------------------------------------------------------------
# Generator
# -----------------------------------------------------------------------------


class Robot25DGenerator:
    name = "robot"

    CHEST_STYLES = ["screen", "plate", "bar"]
    MOODS = ["neutral", "happy", "alert", "focused"]

    ARCHETYPES = {
        "default": {},
        "scout": {"head_h": -1, "body_h": -2, "leg_upper": 2, "leg_lower": 1, "char_yaw": -18},
        "operator": {"panel_w": 5, "panel_h": 4, "head_w": 2},
        "striker": {"body_w": 2, "arm_upper": 1, "arm_lower": 2, "blade_len": 6, "char_yaw": -16},
    }

    ANIMATIONS = {
        "idle": {"frames": 8, "duration_ms": 120},
        "walk": {"frames": 8, "duration_ms": 90},
        "boost": {"frames": 8, "duration_ms": 80},
        "slash": {"frames": 8, "duration_ms": 70},
        "hurt": {"frames": 6, "duration_ms": 85},
        "death": {"frames": 8, "duration_ms": 110},
    }

    PALETTES: Dict[str, Palette] = {
        "classic": Palette(
            shell=rgb("#FFFFFF"),
            shell_top=rgb("#FFFFFF"),
            shell_side=rgb("#EEE9E3"),
            shell_shadow=rgb("#E5E0D9"),
            shell_rim=rgb("#C8C1B9"),
            outline=rgb("#1B1C20"),
            visor=rgb("#0A0F18"),
            visor_glow=rgb("#0DEBFF"),
            accent=rgb("#C88AFF"),
            accent_dark=rgb("#9B5BDE"),
            joint=rgb("#5A6068"),
            joint_dark=rgb("#343942"),
            metal=rgb("#9CA5AF"),
            energy=rgb("#6BDFFF"),
            energy_soft=rgb("#C7F5FF", 170),
            shadow=rgb("#000000", 52),
        ),
        "cool": Palette(
            shell=rgb("#FFFFFF"),
            shell_top=rgb("#FCFEFF"),
            shell_side=rgb("#E8EEF4"),
            shell_shadow=rgb("#DDE5EC"),
            shell_rim=rgb("#BBC8D5"),
            outline=rgb("#181A1F"),
            visor=rgb("#091119"),
            visor_glow=rgb("#13EEFF"),
            accent=rgb("#B787FF"),
            accent_dark=rgb("#8457D8"),
            joint=rgb("#56606B"),
            joint_dark=rgb("#313741"),
            metal=rgb("#A0AAB5"),
            energy=rgb("#70E2FF"),
            energy_soft=rgb("#D0F8FF", 170),
            shadow=rgb("#000000", 52),
        ),
        "lavender": Palette(
            shell=rgb("#FFFEFF"),
            shell_top=rgb("#FFFEFF"),
            shell_side=rgb("#F0E8EF"),
            shell_shadow=rgb("#E6DEE6"),
            shell_rim=rgb("#C8BCC9"),
            outline=rgb("#1A1B1F"),
            visor=rgb("#090E16"),
            visor_glow=rgb("#11EAFF"),
            accent=rgb("#CC92FF"),
            accent_dark=rgb("#A163E5"),
            joint=rgb("#5A5D67"),
            joint_dark=rgb("#373A42"),
            metal=rgb("#A7A1AA"),
            energy=rgb("#67DFFF"),
            energy_soft=rgb("#C9F3FF", 170),
            shadow=rgb("#000000", 52),
        ),
    }

    def sample_spec(self, seed: int, archetype: str = "default") -> BotSpec:
        rng = random.Random(seed)
        tweaks = self.ARCHETYPES.get(archetype, {})

        def tweak(name: str) -> float:
            return float(tweaks.get(name, 0.0))

        palette_name = rng.choice(list(self.PALETTES.keys()))
        return BotSpec(
            target=self.name,
            seed=seed,
            archetype=archetype,
            palette_name=palette_name,
            head_w=rng.randint(37, 40) + tweak("head_w"),
            head_h=rng.randint(30, 33) + tweak("head_h"),
            head_d=rng.randint(16, 19),
            head_round=rng.randint(7, 9),
            body_w=rng.randint(25, 29) + tweak("body_w"),
            body_h=rng.randint(25, 29) + tweak("body_h"),
            body_d=rng.randint(14, 17),
            body_round=rng.randint(5, 7),
            shoulder_span=rng.randint(25, 29),
            hip_span=rng.randint(14, 17),
            arm_upper=rng.randint(12, 15) + tweak("arm_upper"),
            arm_lower=rng.randint(10, 13) + tweak("arm_lower"),
            leg_upper=rng.randint(10, 12) + tweak("leg_upper"),
            leg_lower=rng.randint(9, 11) + tweak("leg_lower"),
            arm_z_back=-1.5,
            arm_z_front=5.2,
            leg_z_back=-3.5,
            leg_z_front=4.5,
            arm_width=rng.uniform(3.8, 4.4),
            leg_width=rng.uniform(4.1, 4.8),
            hand_r=rng.uniform(4.0, 4.8),
            joint_r=rng.uniform(3.2, 4.1),
            foot_w=rng.uniform(9.6, 11.8),
            foot_h=rng.uniform(5.4, 6.6),
            foot_d=rng.uniform(8.0, 10.5),
            visor_w=rng.uniform(21.8, 24.8),
            visor_h=rng.uniform(12.2, 14.3),
            visor_round=rng.uniform(4.0, 5.1),
            visor_inset_y=rng.uniform(0.2, 1.4),
            eye_w=rng.uniform(4.8, 6.0),
            eye_h=rng.uniform(8.4, 9.8),
            eye_gap=rng.uniform(7.0, 8.9),
            antenna_h=rng.uniform(13.0, 16.0),
            antenna_ball=rng.uniform(3.0, 4.0),
            chest_w=rng.uniform(7.6, 10.0),
            chest_h=rng.uniform(7.6, 10.2),
            chest_style=rng.choice(self.CHEST_STYLES),
            mood=rng.choice(self.MOODS),
            blade_len=rng.uniform(31.0, 36.0) + tweak("blade_len"),
            panel_w=rng.uniform(18.0, 22.0) + tweak("panel_w"),
            panel_h=rng.uniform(18.0, 22.0) + tweak("panel_h"),
            char_yaw=tweak("char_yaw") if "char_yaw" in tweaks else rng.uniform(-20.0, -15.0),
            char_pitch=rng.uniform(-2.0, 1.0),
            head_yaw=rng.uniform(-9.0, -5.0),
            head_pitch=rng.uniform(-6.0, -3.0),
        )

    def pose_for_animation(self, animation: str, frame_index: int, frame_count: int) -> Pose:
        p = Pose()
        t = 0.0 if frame_count <= 1 else frame_index / float(frame_count - 1)
        wave = math.sin(t * math.tau)

        if animation == "idle":
            p.body_bob = abs(wave) * 0.9
            p.head_pitch_delta = -abs(wave) * 0.8
            p.left_arm_upper = 108 + wave * 2
            p.left_arm_lower = 96 + wave * 2
            p.right_arm_upper = 72 - wave * 2
            p.right_arm_lower = 84 - wave * 2
            p.left_leg_upper = 114
            p.left_leg_lower = 100
            p.right_leg_upper = 68
            p.right_leg_lower = 82
            p.blink = frame_index == frame_count // 2
            p.eye_squint = 0.08 if frame_index in {1, frame_count - 2} else 0.0
        elif animation == "walk":
            stride = math.sin(t * math.tau)
            contact = math.cos(t * math.tau)
            bounce = (1.0 - math.cos(t * math.tau * 2.0)) * 0.5
            left_forward = -stride
            right_forward = stride
            left_lift = max(0.0, left_forward)
            right_lift = max(0.0, right_forward)
            left_push = max(0.0, -left_forward)
            right_push = max(0.0, -right_forward)

            p.root_x = stride * 1.1
            p.body_bob = 0.35 + bounce * 1.7
            p.root_tilt = -stride * 4.8
            p.head_pitch_delta = -bounce * 1.0 + contact * 0.35
            p.head_yaw_delta = -stride * 1.8

            p.left_arm_upper = 106 + stride * 12
            p.left_arm_lower = 92 + stride * 8
            p.right_arm_upper = 74 - stride * 12
            p.right_arm_lower = 88 - stride * 8

            p.left_leg_upper = 116 - left_forward * 24 + left_lift * 2
            p.left_leg_lower = 99 - left_lift * 26 + left_push * 9
            p.right_leg_upper = 66 - right_forward * 24 + right_lift * 2
            p.right_leg_lower = 83 - right_lift * 26 + right_push * 9
            p.eye_squint = 0.05 + bounce * 0.10
        elif animation == "boost":
            # Directional boost to screen-left.
            #
            # The robot's face/front reads to the left in this rig, so the
            # boost pose must drive left as well: torso/head lead left, limbs
            # trail to the right, and streaks stay behind on the right.
            surge = ease_in_out_sine(t)
            cycle = math.sin(t * math.tau)
            pulse = math.sin(t * math.pi)

            # Keep the body slightly right-of-center so there is more space in
            # front of the boost direction (screen-left).
            p.root_x = 4.6 + surge * 1.6
            p.root_y = 0.2 + pulse * 0.35
            p.root_tilt = 24.0 - cycle * 1.4
            p.head_pitch_delta = -5.5 - surge * 1.0
            p.head_yaw_delta = 2.5

            # Mirror and restage the arms so they trail backward to the right,
            # but remain tucked and aerodynamic.
            p.left_arm_upper = 18 + cycle * 3.0
            p.left_arm_lower = -2 + cycle * 4.0
            p.right_arm_upper = 40 + cycle * 3.0
            p.right_arm_lower = 16 + cycle * 4.0

            # Legs also trail backward to the right.
            p.left_leg_upper = 26 + cycle * 4.0
            p.left_leg_lower = 6 + cycle * 5.0
            p.right_leg_upper = 48 + cycle * 4.0
            p.right_leg_lower = 26 + cycle * 5.0

            p.speed_lines = 1.0
            p.eye_squint = 0.30
        elif animation == "slash":
            wind = 1.0 - smoothstep(clamp(t / 0.28, 0.0, 1.0))
            strike = smoothstep(clamp((t - 0.28) / 0.38, 0.0, 1.0))
            recover = smoothstep(clamp((t - 0.70) / 0.30, 0.0, 1.0))
            p.root_tilt = -5.5 * wind + 7.0 * strike - 1.5 * recover
            p.root_x = -2.0 * wind + 4.0 * strike
            p.root_y = 0.8 * strike
            p.head_yaw_delta = -4.0 * wind + 4.0 * strike
            p.left_arm_upper = 232
            p.left_arm_lower = 244
            p.right_arm_upper = 286 - 14 * wind - 18 * strike + 8 * recover
            p.right_arm_lower = 294 - 18 * wind - 42 * strike + 14 * recover
            p.left_leg_upper = 118 - 8 * wind
            p.left_leg_lower = 100
            p.right_leg_upper = 64 + 16 * strike
            p.right_leg_lower = 82
            p.blade = max(wind, strike, 0.2)
            p.blade_arc = strike
            p.blade_vertical = 1.0 if frame_index in {0, 1, frame_count - 1} else 0.0
            p.eye_squint = 0.18 + strike * 0.35
        elif animation == "console":
            show = ease_in_out_sine(t)
            p.body_bob = abs(wave) * 0.5
            p.head_yaw_delta = 3.0
            p.left_arm_upper = 244
            p.left_arm_lower = 256
            p.right_arm_upper = 304
            p.right_arm_lower = 320
            p.hologram = show
            p.pedestal_console = frame_index >= frame_count // 2
        elif animation == "hurt":
            j = pingpong01(t)
            p.root_x = -4.0 * j
            p.root_y = 1.2 * j
            p.root_tilt = -10.0 * j
            p.head_roll = -10.0 * j
            p.left_arm_upper = 232
            p.left_arm_lower = 244
            p.right_arm_upper = 314
            p.right_arm_lower = 322
            p.left_leg_upper = 118
            p.left_leg_lower = 102
            p.right_leg_upper = 68
            p.right_leg_lower = 84
            p.eye_squint = 0.55
            p.particles = 0.12 + 0.18 * j
        elif animation == "death":
            fall = ease_out_cubic(t)
            eye_cross = smoothstep((t - 0.04) / 0.18)
            eye_power = 1.0 - smoothstep((t - 0.46) / 0.36)
            p.root_x = lerp(0.0, -7.0, fall)
            p.root_y = lerp(0.0, 13.0, fall)
            p.root_tilt = lerp(0.0, 63.0, fall)
            p.head_roll = lerp(0.0, 12.0, fall)
            p.left_arm_upper = lerp(244.0, 216.0, fall)
            p.left_arm_lower = lerp(256.0, 232.0, fall)
            p.right_arm_upper = lerp(300.0, 326.0, fall)
            p.right_arm_lower = lerp(286.0, 336.0, fall)
            p.left_leg_upper = lerp(114.0, 156.0, fall)
            p.left_leg_lower = lerp(100.0, 162.0, fall)
            p.right_leg_upper = lerp(68.0, 110.0, fall)
            p.right_leg_lower = lerp(82.0, 132.0, fall)
            p.collapse = fall
            p.particles = max(0.0, (t - 0.42) / 0.58)
            p.eye_squint = 0.08 + 0.08 * fall
            p.eye_x_amount = eye_cross
            p.eye_power = eye_power
        return p

    # ------------------------------------------------------------------
    # drawing helpers
    # ------------------------------------------------------------------

    def _project_factory(self, size: Tuple[int, int], scale_units: float, ground_y: float):
        w, _ = size
        cx = w * 0.50
        cam_pitch = 9.0

        def project(world: Point3) -> Tuple[Point2, float]:
            q = rotate_x(world, cam_pitch)
            sx = cx + q[0] * scale_units
            sy = ground_y - q[1] * scale_units
            return (sx, sy), q[2]

        return project

    def _world(self, p: Point3, root: Point3, yaw: float = 0.0, pitch: float = 0.0, roll: float = 0.0) -> Point3:
        q = p
        if pitch:
            q = rotate_x(q, pitch)
        if yaw:
            q = rotate_y(q, yaw)
        if roll:
            q = rotate_z(q, roll)
        return add3(q, root)

    def _draw_poly(self, draw: ImageDraw.ImageDraw, pts: Sequence[Point2], fill: Color, outline: Color, outline_w: int) -> None:
        draw.polygon(list(pts), fill=fill)
        draw.line(list(pts) + [pts[0]], fill=outline, width=outline_w, joint="curve")

    def _draw_disc(self, draw: ImageDraw.ImageDraw, center: Point2, r: float, fill: Color, outline: Color, outline_w: int = 1) -> None:
        x, y = center
        draw.ellipse((x - r, y - r, x + r, y + r), fill=fill, outline=outline, width=outline_w)

    def _draw_joint(self, draw: ImageDraw.ImageDraw, center: Point2, r: float, palette: Palette, outline_w: int) -> None:
        self._draw_disc(draw, center, r * 1.18, palette.joint, palette.outline, outline_w)
        self._draw_disc(draw, center, r * 0.58, palette.joint_dark, palette.outline, max(1, outline_w - 1))
        hx, hy = center[0] - r * 0.33, center[1] - r * 0.38
        self._draw_disc(draw, (hx, hy), r * 0.20, with_alpha(lighten(palette.metal, 0.35), 220), with_alpha(lighten(palette.metal, 0.35), 0), 1)

    def _draw_capsule_limb(self, draw: ImageDraw.ImageDraw, a: Point2, b: Point2, c: Point2, width: float, palette: Palette, outline_w: int, back: bool = False) -> None:
        shell = palette.shell_shadow if back else palette.shell
        shell_mid = palette.shell_side if back else palette.shell_top
        draw.line([a, b], fill=palette.outline, width=max(1, int(width * 2.0)), joint="curve")
        draw.line([b, c], fill=palette.outline, width=max(1, int(width * 2.0)), joint="curve")
        draw.line([a, b], fill=shell_mid, width=max(1, int(width * 1.45)), joint="curve")
        draw.line([b, c], fill=shell_mid, width=max(1, int(width * 1.45)), joint="curve")
        draw.line([a, b], fill=shell, width=max(1, int(width * 1.05)), joint="curve")
        draw.line([b, c], fill=shell, width=max(1, int(width * 1.05)), joint="curve")
        self._draw_joint(draw, a, width * 0.55, palette, outline_w)
        self._draw_joint(draw, b, width * 0.50, palette, outline_w)
        self._draw_disc(draw, c, width * 0.66, shell, palette.outline, outline_w)
        self._draw_disc(draw, c, width * 0.26, palette.joint_dark, palette.outline, max(1, outline_w - 1))

    def _draw_foot(self, draw: ImageDraw.ImageDraw, ankle: Point2, foot_center: Point2, width: float, height: float, palette: Palette, outline_w: int, back: bool = False) -> None:
        shell = palette.shell_shadow if back else palette.shell
        shell_mid = palette.shell_side if back else palette.shell_top
        pts = [
            (foot_center[0] - width * 0.62, foot_center[1] - height * 0.55),
            (foot_center[0] + width * 0.10, foot_center[1] - height * 0.55),
            (foot_center[0] + width * 0.40, foot_center[1] - height * 0.10),
            (foot_center[0] + width * 0.12, foot_center[1] + height * 0.25),
            (foot_center[0] - width * 0.56, foot_center[1] + height * 0.18),
        ]
        self._draw_poly(draw, pts, shell_mid, palette.outline, outline_w)
        inset = [segment_point(p, (sum(x for x, _ in pts) / len(pts), sum(y for _, y in pts) / len(pts)), 0.18) for p in pts]
        self._draw_poly(draw, inset, shell, palette.outline, max(1, outline_w - 1))
        draw.line([(ankle[0], ankle[1]), (foot_center[0] - width * 0.20, foot_center[1] - height * 0.18)], fill=palette.outline, width=outline_w + 1)

    def _draw_shadow(self, draw: ImageDraw.ImageDraw, center: Point2, rx: float, ry: float, color: Color) -> None:
        draw.ellipse((center[0] - rx, center[1] - ry, center[0] + rx, center[1] + ry), fill=color)

    def _draw_speed_lines(self, img: Image.Image, palette: Palette, anchor_x: float, center_y: float, amount: float, width_scale: float, direction: int = 1) -> None:
        if amount <= 0.0:
            return
        overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
        draw = ImageDraw.Draw(overlay)
        n = 18
        sign = 1 if direction >= 0 else -1
        for i in range(n):
            y = center_y + (i - n / 2) * width_scale * 0.55
            length = img.size[0] * (0.24 + 0.08 * math.sin(i * 1.6))
            end_x = anchor_x - sign * (width_scale * (2.0 + 0.10 * abs(i - n / 2)))
            start_x = end_x - sign * length
            alpha = int((28 + 62 * (1.0 - abs(i - n / 2) / (n / 2))) * amount)
            draw.line([(start_x, y), (end_x, y)], fill=with_alpha(palette.energy, alpha), width=max(1, int(width_scale * 0.20)))
        overlay = overlay.filter(ImageFilter.GaussianBlur(radius=max(1, int(width_scale * 0.15))))
        img.alpha_composite(overlay)

    def _draw_particles(self, img: Image.Image, origin: Point2, palette: Palette, amount: float, pixel_scale: float, seed: int) -> None:
        if amount <= 0.0:
            return
        rng = random.Random(seed)
        overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
        draw = ImageDraw.Draw(overlay)
        count = int(8 + amount * 38)
        spread_x = 18 * pixel_scale * (0.35 + amount)
        spread_y = 12 * pixel_scale * (0.35 + amount)
        for _ in range(count):
            x = origin[0] + rng.uniform(-0.3, 1.0) * spread_x
            y = origin[1] + rng.uniform(-1.0, 1.0) * spread_y
            s = rng.uniform(0.6, 1.7) * pixel_scale
            draw.rounded_rectangle((x - s, y - s, x + s, y + s), radius=s * 0.25, fill=with_alpha(palette.energy, rng.randint(70, 180)))
        overlay = overlay.filter(ImageFilter.GaussianBlur(radius=max(1, int(pixel_scale * 0.08))))
        img.alpha_composite(overlay)

    def _draw_blade(self, img: Image.Image, hand: Point2, palette: Palette, blade_len: float, pixel_scale: float, amount: float, arc: float, vertical: float) -> None:
        if amount <= 0.0:
            return
        overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
        draw = ImageDraw.Draw(overlay)
        dx = blade_len * (0.95 + 0.08 * amount)
        dy = blade_len * (0.12 + 0.58 * vertical)
        tip = (hand[0] + dx, hand[1] - dy)
        poly = [
            (hand[0] - 1.3 * pixel_scale, hand[1] - 0.8 * pixel_scale),
            tip,
            (hand[0] + 2.8 * pixel_scale, hand[1] + 2.6 * pixel_scale),
        ]
        draw.polygon(poly, fill=with_alpha(palette.energy_soft, 190), outline=with_alpha(palette.energy, 245))
        if arc > 0.02:
            bbox = (hand[0] - 20 * pixel_scale, hand[1] - 24 * pixel_scale, hand[0] + 36 * pixel_scale, hand[1] + 30 * pixel_scale)
            draw.arc(bbox, start=-34, end=228, fill=with_alpha(palette.energy, int(200 * arc)), width=max(1, int(pixel_scale * 0.35)))
            draw.arc((bbox[0] - 5 * pixel_scale, bbox[1] - 4 * pixel_scale, bbox[2] + 2 * pixel_scale, bbox[3] + 2 * pixel_scale), start=-28, end=210, fill=with_alpha(palette.energy_soft, int(140 * arc)), width=max(1, int(pixel_scale * 0.22)))
        overlay = overlay.filter(ImageFilter.GaussianBlur(radius=max(1, int(pixel_scale * 0.10))))
        img.alpha_composite(overlay)

    def _draw_console_panel(self, img: Image.Image, center: Point2, w: float, h: float, palette: Palette, opacity: float, pixel_scale: float, pedestal: bool = False) -> None:
        if opacity <= 0.0:
            return
        overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
        draw = ImageDraw.Draw(overlay)
        x0 = center[0] - w / 2
        y0 = center[1] - h / 2
        fill = with_alpha(palette.energy_soft, int(40 + 95 * opacity))
        edge = with_alpha(palette.energy, int(160 + 70 * opacity))
        draw.rounded_rectangle((x0, y0, x0 + w, y0 + h), radius=3.0 * pixel_scale, fill=fill, outline=edge, width=max(1, int(pixel_scale * 0.20)))
        draw.rounded_rectangle((x0 + 1.8 * pixel_scale, y0 + 1.8 * pixel_scale, x0 + w - 1.8 * pixel_scale, y0 + h - 1.8 * pixel_scale), radius=2.4 * pixel_scale, fill=with_alpha(palette.visor, int(110 * opacity)), outline=edge, width=max(1, int(pixel_scale * 0.12)))
        cx = center[0]
        cy = center[1]
        draw.ellipse((cx - 4 * pixel_scale, cy - 4 * pixel_scale, cx + 4 * pixel_scale, cy + 4 * pixel_scale), outline=edge, width=max(1, int(pixel_scale * 0.15)))
        draw.line([(cx - 5 * pixel_scale, cy), (cx + 5 * pixel_scale, cy)], fill=edge, width=max(1, int(pixel_scale * 0.12)))
        draw.line([(cx, cy - 5 * pixel_scale), (cx, cy + 5 * pixel_scale)], fill=edge, width=max(1, int(pixel_scale * 0.12)))
        if pedestal:
            base_y = y0 + h + 2 * pixel_scale
            draw.rounded_rectangle((x0 + w * 0.32, base_y, x0 + w * 0.68, base_y + 6 * pixel_scale), radius=1.6 * pixel_scale, fill=with_alpha(palette.metal, int(230 * opacity)), outline=palette.outline, width=max(1, int(pixel_scale * 0.12)))
            draw.rounded_rectangle((x0 + w * 0.46, base_y + 6 * pixel_scale, x0 + w * 0.54, base_y + 17 * pixel_scale), radius=1.2 * pixel_scale, fill=with_alpha(palette.metal, int(230 * opacity)), outline=palette.outline, width=max(1, int(pixel_scale * 0.12)))
            draw.rounded_rectangle((x0 + w * 0.20, base_y + 17 * pixel_scale, x0 + w * 0.80, base_y + 23 * pixel_scale), radius=1.8 * pixel_scale, fill=with_alpha(palette.metal, int(230 * opacity)), outline=palette.outline, width=max(1, int(pixel_scale * 0.12)))
        overlay = overlay.filter(ImageFilter.GaussianBlur(radius=max(1, int(pixel_scale * 0.08))))
        img.alpha_composite(overlay)

    # ------------------------------------------------------------------
    # rendering
    # ------------------------------------------------------------------

    def render_animation_frame(
        self,
        spec: BotSpec,
        animation: str,
        frame_index: int,
        frame_count: int,
        size: Tuple[int, int],
        background: Optional[Color] = None,
        supersample: int = 6,
        downsample: str = "lanczos",
    ) -> Image.Image:
        pose = self.pose_for_animation(animation, frame_index, frame_count)
        resample = RESAMPLING.LANCZOS if downsample == "lanczos" else RESAMPLING.NEAREST
        base_size = size if supersample <= 1 else (size[0] * supersample, size[1] * supersample)
        if animation == "death":
            overscan = 2.20
            margin_frac = 0.10
            pad_frac = 0.14
        elif animation == "slash":
            overscan = 1.76
            margin_frac = 0.075
            pad_frac = 0.075
        else:
            overscan = 1.56
            margin_frac = 0.055
            pad_frac = 0.05
        canvas_size = (max(base_size[0] + 8, int(base_size[0] * overscan)), max(base_size[1] + 8, int(base_size[1] * overscan)))
        hi = self._render_core(spec, canvas_size, None, pose, scale_reference_size=base_size)
        alpha = hi.getchannel("A")
        bbox = alpha.getbbox()
        if bbox is None:
            composed = Image.new("RGBA", base_size, background if background is not None else (0, 0, 0, 0))
        else:
            margin = max(4, int(min(base_size) * margin_frac))
            x0 = max(0, bbox[0] - margin)
            y0 = max(0, bbox[1] - margin)
            x1 = min(hi.width, bbox[2] + margin)
            y1 = min(hi.height, bbox[3] + margin)
            crop = hi.crop((x0, y0, x1, y1))
            pad = max(2, int(min(base_size) * pad_frac))
            avail_w = max(1, base_size[0] - pad * 2)
            avail_h = max(1, base_size[1] - pad * 2)
            fit_scale = min(avail_w / max(1, crop.width), avail_h / max(1, crop.height))

            # Keep collapse / death frames closer to the standing body scale so
            # they remain usable in-engine. The death pose naturally produces a
            # much wider bbox, which would otherwise shrink the whole sprite.
            if animation == "death":
                ref_pose = self.pose_for_animation("idle", 0, max(1, self.ANIMATIONS["idle"]["frames"]))
                ref_hi = self._render_core(spec, canvas_size, None, ref_pose, scale_reference_size=base_size)
                ref_bbox = ref_hi.getchannel("A").getbbox()
                if ref_bbox is not None:
                    rx0 = max(0, ref_bbox[0] - margin)
                    ry0 = max(0, ref_bbox[1] - margin)
                    rx1 = min(ref_hi.width, ref_bbox[2] + margin)
                    ry1 = min(ref_hi.height, ref_bbox[3] + margin)
                    ref_w = max(1, rx1 - rx0)
                    ref_h = max(1, ry1 - ry0)
                    ref_fit_scale = min(avail_w / ref_w, avail_h / ref_h)
                    fit_scale = max(fit_scale, ref_fit_scale * 0.985)

            fitted_size = (max(1, int(round(crop.width * fit_scale))), max(1, int(round(crop.height * fit_scale))))
            crop = crop.resize(fitted_size, resample)
            composed = Image.new("RGBA", base_size, background if background is not None else (0, 0, 0, 0))

            if animation == "death":
                paste_xy = ((base_size[0] - fitted_size[0]) // 2, base_size[1] - fitted_size[1] - pad)
            else:
                paste_xy = ((base_size[0] - fitted_size[0]) // 2, (base_size[1] - fitted_size[1]) // 2)
            composed.alpha_composite(crop, paste_xy)
        if supersample <= 1:
            return composed
        return composed.resize(size, resample)

    def _render_core(self, spec: BotSpec, size: Tuple[int, int], background: Optional[Color], pose: Pose, scale_reference_size: Optional[Tuple[int, int]] = None) -> Image.Image:
        w, h = size
        img = Image.new("RGBA", size, background if background is not None else (0, 0, 0, 0))
        draw = ImageDraw.Draw(img)
        palette = self.PALETTES[spec.palette_name]

        scale_w, scale_h = scale_reference_size if scale_reference_size is not None else size
        pixel_scale = min(scale_w, scale_h) / 96.0
        world_scale = pixel_scale
        outline_w = max(1, int(pixel_scale * 0.22))
        project = self._project_factory(size, world_scale, h * 0.84 + pose.root_y * pixel_scale)

        # root / world anchors in world units, then projected with world_scale
        root = (pose.root_x, pose.body_bob, 0.0)

        body_center = add3(root, (0.0, 24.0, 0.0))
        head_center = add3(root, (0.0, 50.0, 0.0))
        neck_base = add3(root, (0.0, 38.5, 0.0))
        left_shoulder = add3(root, (-spec.shoulder_span / 2.0, 29.0, spec.arm_z_back))
        right_shoulder = add3(root, (spec.shoulder_span / 2.0, 29.0, spec.arm_z_front))
        left_hip = add3(root, (-spec.hip_span / 2.0, 13.0, spec.leg_z_back))
        right_hip = add3(root, (spec.hip_span / 2.0, 13.0, spec.leg_z_front))

        # Global body rotation
        def rig_world(p: Point3) -> Point3:
            return self._world(p, (0.0, 0.0, 0.0), yaw=spec.char_yaw, pitch=spec.char_pitch, roll=pose.root_tilt)

        def rig_project(p: Point3) -> Tuple[Point2, float]:
            return project(rig_world(p))

        # shadow first
        shadow_center, _ = rig_project((0.0, 0.0, 0.0))
        speed_anchor_x = shadow_center[0] + pixel_scale * 10.0
        self._draw_speed_lines(img, palette, speed_anchor_x, h * 0.56, pose.speed_lines, pixel_scale, direction=-1)

        # back limbs
        left_elbow = add3(left_shoulder, angled_vector(spec.arm_upper, pose.left_arm_upper))
        left_hand = add3(left_elbow, angled_vector(spec.arm_lower, pose.left_arm_lower))
        a0, _ = rig_project(left_shoulder)
        a1, _ = rig_project(left_elbow)
        a2, _ = rig_project(left_hand)
        self._draw_capsule_limb(draw, a0, a1, a2, spec.arm_width * pixel_scale, palette, outline_w, back=True)

        left_knee = add3(left_hip, angled_vector(spec.leg_upper, pose.left_leg_upper))
        left_ankle = add3(left_knee, angled_vector(spec.leg_lower, pose.left_leg_lower))
        l0, _ = rig_project(left_hip)
        l1, _ = rig_project(left_knee)
        l2, _ = rig_project(left_ankle)
        self._draw_capsule_limb(draw, l0, l1, l2, spec.leg_width * pixel_scale, palette, outline_w, back=True)
        lfoot_center = add2(l2, (-spec.foot_w * 0.10 * pixel_scale, 1.0 * pixel_scale))
        self._draw_foot(draw, l2, lfoot_center, spec.foot_w * pixel_scale, spec.foot_h * pixel_scale, palette, outline_w, back=True)

        # torso shell faces
        body_root = body_center
        body_yaw = spec.char_yaw
        body_roll = pose.root_tilt
        body_pitch = spec.char_pitch

        def body_face_local(face: str) -> List[Point3]:
            w2 = spec.body_w / 2.0
            h2 = spec.body_h / 2.0
            d2 = spec.body_d / 2.0
            if face == "front":
                path = rounded_rect_path(spec.body_w, spec.body_h, spec.body_round, 8)
                return [(x, y, d2) for x, y in path]
            if face == "left":
                return [(-w2, -h2, d2), (-w2, h2, d2), (-w2, h2, -d2), (-w2, -h2, -d2)]
            if face == "right":
                return [(w2, -h2, -d2), (w2, h2, -d2), (w2, h2, d2), (w2, -h2, d2)]
            if face == "top":
                return [(-w2, h2, -d2), (-w2, h2, d2), (w2, h2, d2), (w2, h2, -d2)]
            raise KeyError(face)

        def transform_part_points(local_points: Iterable[Point3], center: Point3, yaw: float, pitch: float, roll: float) -> List[Point3]:
            pts = [transform_local_point(p, center, rot_x_deg=pitch, rot_y_deg=yaw, rot_z_deg=roll) for p in local_points]
            return [rig_world(p) for p in pts]

        def visible_face(points: Sequence[Point3]) -> bool:
            if len(points) < 3:
                return False
            v1 = sub3(points[1], points[0])
            v2 = sub3(points[2], points[1])
            n = cross(v1, v2)
            cam_n = rotate_x(n, 9.0)
            return cam_n[2] > 0

        def draw_face(local_points: Sequence[Point3], center: Point3, yaw: float, pitch: float, roll: float, fill: Color, gloss: Optional[Color] = None, inset_scale: float = 0.90):
            world_points = transform_part_points(local_points, center, yaw, pitch, roll)
            if not visible_face(world_points):
                return
            proj = [project(p)[0] for p in world_points]
            self._draw_poly(draw, proj, fill, palette.outline, outline_w)
            # soft inset panel
            cx = sum(p[0] for p in proj) / len(proj)
            cy = sum(p[1] for p in proj) / len(proj)
            inset = [segment_point(p, (cx, cy), 1.0 - inset_scale) for p in proj]
            self._draw_poly(draw, inset, gloss or lighten(fill, 0.08), palette.outline, max(1, outline_w - 1))
            # tiny highlight strip
            if len(inset) >= 6:
                hi = inset[: max(2, len(inset) // 4)]
                draw.line(hi, fill=with_alpha(lighten(fill, 0.35), 220), width=max(1, outline_w))

        side_face_name = "left" if spec.char_yaw < 0 else "right"
        other_side_face_name = "right" if side_face_name == "left" else "left"

        for face_name, fill in [
            ("top", palette.shell_top),
            (side_face_name, palette.shell_side),
            (other_side_face_name, palette.shell_shadow),
            ("front", palette.shell),
        ]:
            draw_face(body_face_local(face_name), body_root, 0.0, 0.0, 0.0, fill, gloss=lighten(fill, 0.06), inset_scale=0.93 if face_name == "front" else 0.88)

        # chest module on torso front plane
        chest_path = rounded_rect_path(spec.chest_w, spec.chest_h, 2.2, 6)
        chest_local = [(x, y - spec.body_h * 0.05, spec.body_d / 2.0 + 0.1) for x, y in chest_path]
        chest_world = transform_part_points(chest_local, body_root, 0.0, 0.0, 0.0)
        chest_proj = [project(p)[0] for p in chest_world]
        if spec.chest_style == "screen":
            fill = palette.visor
            gloss = with_alpha(palette.visor_glow, 160)
        elif spec.chest_style == "bar":
            fill = palette.shell_top
            gloss = with_alpha(palette.visor_glow, 160)
        else:
            fill = with_alpha(palette.visor_glow, 160)
            gloss = with_alpha(lighten(palette.visor_glow, 0.30), 220)
        self._draw_poly(draw, chest_proj, fill, palette.outline, max(1, outline_w - 1))
        cxp = sum(p[0] for p in chest_proj) / len(chest_proj)
        cyp = sum(p[1] for p in chest_proj) / len(chest_proj)
        chest_inset = [segment_point(p, (cxp, cyp), 0.14) for p in chest_proj]
        draw.line(chest_inset[: max(2, len(chest_inset) // 3)], fill=gloss, width=max(1, outline_w - 1))

        # neck
        neck_a, _ = rig_project((0.0, 37.0, -2.0))
        neck_b, _ = rig_project((0.0, 43.5, -1.0))
        draw.line([neck_a, neck_b], fill=palette.outline, width=max(1, int(pixel_scale * 1.4)))
        draw.line([neck_a, neck_b], fill=palette.joint, width=max(1, int(pixel_scale * 0.9)))

        # head shell faces
        head_root = head_center
        head_yaw = spec.head_yaw + pose.head_yaw_delta
        head_pitch = spec.head_pitch + pose.head_pitch_delta
        head_roll = pose.head_roll

        def head_face_local(face: str) -> List[Point3]:
            w2 = spec.head_w / 2.0
            h2 = spec.head_h / 2.0
            d2 = spec.head_d / 2.0
            if face == "front":
                path = rounded_rect_path(spec.head_w, spec.head_h, spec.head_round, 10)
                return [(x, y, d2) for x, y in path]
            if face == "left":
                return [(-w2, -h2, d2), (-w2, h2, d2), (-w2, h2, -d2), (-w2, -h2, -d2)]
            if face == "right":
                return [(w2, -h2, -d2), (w2, h2, -d2), (w2, h2, d2), (w2, -h2, d2)]
            if face == "top":
                return [(-w2, h2, -d2), (-w2, h2, d2), (w2, h2, d2), (w2, h2, -d2)]
            raise KeyError(face)

        for face_name, fill in [
            ("top", palette.shell_top),
            (side_face_name, palette.shell_side),
            (other_side_face_name, palette.shell_shadow),
            ("front", palette.shell),
        ]:
            draw_face(head_face_local(face_name), head_root, head_yaw, head_pitch, head_roll, fill, gloss=lighten(fill, 0.05), inset_scale=0.94 if face_name == "front" else 0.89)

        # visor
        visor_path = rounded_rect_path(spec.visor_w, spec.visor_h, spec.visor_round, 8)
        visor_local = [(x, y - spec.visor_inset_y, spec.head_d / 2.0 + 0.16) for x, y in visor_path]
        visor_world = transform_part_points(visor_local, head_root, head_yaw, head_pitch, head_roll)
        visor_proj = [project(p)[0] for p in visor_world]
        self._draw_poly(draw, visor_proj, palette.visor, palette.outline, outline_w)
        vcx = sum(p[0] for p in visor_proj) / len(visor_proj)
        vcy = sum(p[1] for p in visor_proj) / len(visor_proj)
        visor_inset = [segment_point(p, (vcx, vcy), 0.08) for p in visor_proj]
        draw.line(visor_inset[: max(2, len(visor_inset) // 4)], fill=with_alpha(lighten(palette.visor, 0.35), 90), width=max(1, outline_w))

        # eyes / glow in visor-local coordinates
        eye_glow_overlay = Image.new("RGBA", size, (0, 0, 0, 0))
        eye_glow_draw = ImageDraw.Draw(eye_glow_overlay)
        eye_fx_overlay = Image.new("RGBA", size, (0, 0, 0, 0))
        eye_fx_draw = ImageDraw.Draw(eye_fx_overlay)
        eye_h = spec.eye_h * (0.25 if pose.blink else 1.0 - 0.45 * pose.eye_squint)
        eye_samples = ellipse_path(spec.eye_w / 2.0, eye_h / 2.0, 18)
        eye_offsets = [-(spec.eye_gap / 2.0 + spec.eye_w * 0.42), spec.eye_gap / 2.0 + spec.eye_w * 0.42]
        eye_polys: List[List[Point2]] = []
        eye_x_amount = clamp(pose.eye_x_amount, 0.0, 1.0)
        eye_power = clamp(pose.eye_power, 0.0, 1.0)
        normal_alpha = int(255 * eye_power * ((1.0 - eye_x_amount) ** 2))
        glow_alpha = int(115 * eye_power * max(0.0, 1.0 - eye_x_amount))
        x_alpha = int(255 * eye_power * eye_x_amount)
        for ex in eye_offsets:
            local_pts = [(x + ex, y - spec.visor_inset_y, spec.head_d / 2.0 + 0.18) for x, y in eye_samples]
            world_pts = transform_part_points(local_pts, head_root, head_yaw, head_pitch, head_roll)
            proj_pts = [project(p)[0] for p in world_pts]
            eye_polys.append(proj_pts)
            if glow_alpha > 0:
                eye_glow_draw.polygon(proj_pts, fill=with_alpha(palette.visor_glow, glow_alpha))
            if x_alpha > 0:
                xs = [p[0] for p in proj_pts]
                ys = [p[1] for p in proj_pts]
                pad = 0.4 * pixel_scale
                x0 = min(xs) + pad
                x1 = max(xs) - pad
                y0 = min(ys) + pad
                y1 = max(ys) - pad
                eye_fx_draw.line([(x0, y0), (x1, y1)], fill=with_alpha(palette.visor_glow, x_alpha), width=max(1, int(pixel_scale * 1.15)))
                eye_fx_draw.line([(x0, y1), (x1, y0)], fill=with_alpha(palette.visor_glow, x_alpha), width=max(1, int(pixel_scale * 1.15)))
        eye_glow_overlay = eye_glow_overlay.filter(ImageFilter.GaussianBlur(radius=max(1, int(pixel_scale * 0.28))))
        img.alpha_composite(eye_glow_overlay)
        if x_alpha > 0:
            eye_fx_overlay = eye_fx_overlay.filter(ImageFilter.GaussianBlur(radius=max(1, int(pixel_scale * 0.08))))
            img.alpha_composite(eye_fx_overlay)
        for poly in eye_polys:
            if normal_alpha > 0:
                self._draw_poly(draw, poly, with_alpha(palette.visor_glow, normal_alpha), with_alpha(palette.visor_glow, 0), 1)
                ex = sum(p[0] for p in poly) / len(poly)
                ey = sum(p[1] for p in poly) / len(poly)
                highlight_alpha = min(255, int(220 * eye_power * (1.0 - eye_x_amount)))
                self._draw_disc(draw, (ex - pixel_scale * 0.6, ey - pixel_scale * 0.7), pixel_scale * 0.35, with_alpha(lighten(palette.visor_glow, 0.55), highlight_alpha), with_alpha(palette.visor_glow, 0), 1)

        # antenna and side module anchored on visible side
        side_sign = -1.0 if side_face_name == "left" else 1.0
        module_center_local = (side_sign * (spec.head_w / 2.0 + 3.2), 4.0, 1.0)
        module_pts_local = [
            (module_center_local[0] - 3.2 * side_sign, module_center_local[1] - 5.0, module_center_local[2] + 2.0),
            (module_center_local[0] - 3.2 * side_sign, module_center_local[1] + 5.0, module_center_local[2] + 2.0),
            (module_center_local[0] + 3.2 * side_sign, module_center_local[1] + 5.0, module_center_local[2] - 2.0),
            (module_center_local[0] + 3.2 * side_sign, module_center_local[1] - 5.0, module_center_local[2] - 2.0),
        ]
        module_world = transform_part_points(module_pts_local, head_root, head_yaw, head_pitch, head_roll)
        module_proj = [project(p)[0] for p in module_world]
        self._draw_poly(draw, module_proj, palette.accent, palette.outline, outline_w)
        # antenna rod and ball
        antenna_base_local = (side_sign * (spec.head_w / 2.0 - 3.0), spec.head_h / 2.0 - 5.0, 1.5)
        antenna_tip_local = (antenna_base_local[0], antenna_base_local[1] + spec.antenna_h, antenna_base_local[2])
        antenna_ball_local = (antenna_tip_local[0], antenna_tip_local[1] + spec.antenna_ball * 0.2, antenna_tip_local[2])
        ant_base, _ = project(transform_part_points([antenna_base_local], head_root, head_yaw, head_pitch, head_roll)[0])
        ant_tip, _ = project(transform_part_points([antenna_tip_local], head_root, head_yaw, head_pitch, head_roll)[0])
        ant_ball, _ = project(transform_part_points([antenna_ball_local], head_root, head_yaw, head_pitch, head_roll)[0])
        draw.line([ant_base, ant_tip], fill=palette.outline, width=max(1, int(pixel_scale * 0.35)))
        draw.line([ant_base, ant_tip], fill=palette.accent_dark, width=max(1, int(pixel_scale * 0.22)))
        self._draw_disc(draw, ant_ball, spec.antenna_ball * pixel_scale, palette.accent, palette.outline, outline_w)
        self._draw_disc(draw, (ant_ball[0] - pixel_scale * 0.6, ant_ball[1] - pixel_scale * 0.6), pixel_scale * 0.30, with_alpha(lighten(palette.accent, 0.45), 220), with_alpha(palette.accent, 0), 1)

        # front limbs
        right_elbow = add3(right_shoulder, angled_vector(spec.arm_upper, pose.right_arm_upper))
        right_hand = add3(right_elbow, angled_vector(spec.arm_lower, pose.right_arm_lower))
        b0, _ = rig_project(right_shoulder)
        b1, _ = rig_project(right_elbow)
        b2, _ = rig_project(right_hand)
        self._draw_capsule_limb(draw, b0, b1, b2, spec.arm_width * pixel_scale, palette, outline_w, back=False)

        right_knee = add3(right_hip, angled_vector(spec.leg_upper, pose.right_leg_upper))
        right_ankle = add3(right_knee, angled_vector(spec.leg_lower, pose.right_leg_lower))
        r0, _ = rig_project(right_hip)
        r1, _ = rig_project(right_knee)
        r2, _ = rig_project(right_ankle)
        self._draw_capsule_limb(draw, r0, r1, r2, spec.leg_width * pixel_scale, palette, outline_w, back=False)
        rfoot_center = add2(r2, (spec.foot_w * 0.06 * pixel_scale, 1.2 * pixel_scale))
        self._draw_foot(draw, r2, rfoot_center, spec.foot_w * pixel_scale, spec.foot_h * pixel_scale, palette, outline_w, back=False)

        # overlay only the near-side joints so the far-side arm remains occluded by the torso
        self._draw_joint(draw, rig_project(right_shoulder)[0], spec.joint_r * pixel_scale, palette, outline_w)
        self._draw_joint(draw, rig_project(right_hip)[0], spec.joint_r * pixel_scale * 0.94, palette, outline_w)

        # FX layers
        self._draw_blade(img, b2, palette, spec.blade_len * pixel_scale, pixel_scale, pose.blade, pose.blade_arc, pose.blade_vertical)
        if pose.hologram > 0:
            self._draw_console_panel(img, (b2[0] + 12 * pixel_scale, b2[1] - 8 * pixel_scale), spec.panel_w * pixel_scale, spec.panel_h * pixel_scale, palette, pose.hologram, pixel_scale, pedestal=False)
        if pose.pedestal_console:
            self._draw_console_panel(img, (shadow_center[0] + 29 * pixel_scale, h * 0.60), spec.panel_w * pixel_scale, spec.panel_h * pixel_scale, palette, pose.hologram, pixel_scale, pedestal=True)
        if pose.particles > 0.0:
            chest_anchor = (cxp + 2 * pixel_scale, cyp)
            self._draw_particles(img, chest_anchor, palette, pose.particles, pixel_scale, spec.seed + int(pose.particles * 1000))
        if pose.collapse > 0.55:
            self._draw_particles(img, (shadow_center[0] + 12 * pixel_scale, h * 0.90), palette, pose.collapse * 0.90, pixel_scale, spec.seed + 999)

        return img


TARGETS: Dict[str, Robot25DGenerator] = {Robot25DGenerator.name: Robot25DGenerator()}
