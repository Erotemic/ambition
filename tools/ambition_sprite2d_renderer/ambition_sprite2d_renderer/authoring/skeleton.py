"""Reusable 2D bone + keyframe toolkit for procedural sprite targets.

The pieces, smallest to largest:

- [`Skeleton`] / [`Bone`] — a named FK bone tree. Each bone attaches to its
  parent at a parent-local ``offset`` and contributes ``rest_angle`` plus a
  per-frame pose angle. ``Skeleton.world()`` evaluates the tree into
  [`BoneWorld`] transforms (origin + world angle + tip).
- [`two_bone_ik`] — analytic two-segment IK (knees / elbows). Authoring leg
  motion as *foot trajectories* and letting IK place the knees is what keeps
  feet from sliding, which hand-tuned joint angles cannot guarantee.
- [`Channel`] / [`Clip`] — scalar keyframe tracks with per-key easing and
  loop wrapping. A clip samples every channel at normalized time ``t`` into a
  flat ``{name: value}`` dict; by convention names matching bones become pose
  angles and everything else is a free parameter for the part painters.
- [`Rig`] / [`PartCtx`] — z-ordered part painters bound to bones. A part is
  a function drawing in bone-local coordinates via the ctx helpers, so it
  inherits its bone's motion for free.

Coordinate conventions (matching the rest of the renderer): screen space
with +y down, angles in degrees where 0° points to +x (screen right) and
positive angles rotate clockwise on screen. So ``rest_angle=90`` hangs a
limb straight down and a forward lean of the torso is a positive angle.

Geometry is authored in base-frame pixels (e.g. 128×128); ``Rig.draw``
multiplies by the supersample scale at paint time.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Callable, Dict, List, Mapping, Optional, Sequence, Tuple, Union

from PIL import Image, ImageDraw

from .rig import add, clamp, ease_in_out_sine, ease_out_cubic, lerp, smoothstep, vec

Point = Tuple[float, float]
Color = Tuple[int, int, int, int]
EaseFn = Callable[[float], float]


def rotate_point(p: Point, degrees: float) -> Point:
    a = math.radians(degrees)
    c, s = math.cos(a), math.sin(a)
    return (p[0] * c - p[1] * s, p[0] * s + p[1] * c)


# ---- Bones -------------------------------------------------------------------


@dataclass(frozen=True)
class Bone:
    name: str
    parent: Optional[str]
    offset: Point
    """Attachment point in the parent's local frame (rotated by the parent's
    world angle at evaluation time). For a bone that continues its parent
    (forearm after upper arm) this is ``(parent_length, 0)``."""
    length: float
    rest_angle: float
    """Degrees added on top of the parent's world angle before the pose angle."""


@dataclass(frozen=True)
class BoneWorld:
    origin: Point
    angle: float
    length: float

    @property
    def tip(self) -> Point:
        return add(self.origin, vec(self.length, self.angle))

    def to_world(self, local: Point) -> Point:
        return add(self.origin, rotate_point(local, self.angle))


class Skeleton:
    """An ordered FK bone tree. Bones must be added parents-first, which makes
    a single in-order pass sufficient for world evaluation."""

    def __init__(self) -> None:
        self.bones: Dict[str, Bone] = {}

    def bone(
        self,
        name: str,
        parent: Optional[str] = None,
        offset: Point = (0.0, 0.0),
        length: float = 0.0,
        rest_angle: float = 0.0,
    ) -> "Skeleton":
        if name in self.bones:
            raise ValueError(f"duplicate bone {name!r}")
        if parent is not None and parent not in self.bones:
            raise ValueError(f"bone {name!r}: parent {parent!r} not defined yet")
        self.bones[name] = Bone(name, parent, offset, length, rest_angle)
        return self

    def world(
        self,
        pose: Optional[Mapping[str, float]] = None,
        root: Point = (0.0, 0.0),
        root_angle: float = 0.0,
    ) -> Dict[str, BoneWorld]:
        pose = pose or {}
        out: Dict[str, BoneWorld] = {}
        for name, b in self.bones.items():
            if b.parent is None:
                p_origin, p_angle = root, root_angle
            else:
                pw = out[b.parent]
                p_origin, p_angle = pw.origin, pw.angle
            angle = p_angle + b.rest_angle + float(pose.get(name, 0.0))
            origin = add(p_origin, rotate_point(b.offset, p_angle))
            out[name] = BoneWorld(origin, angle, b.length)
        return out

    def pose_angle_for_world(
        self,
        name: str,
        world_deg: float,
        world: Mapping[str, BoneWorld],
        root_angle: float = 0.0,
    ) -> float:
        """Pose angle that would give ``name`` the world angle ``world_deg``,
        given its parent's already-evaluated world transform."""
        b = self.bones[name]
        parent_angle = world[b.parent].angle if b.parent else root_angle
        return world_deg - parent_angle - b.rest_angle


def two_bone_ik(
    root: Point,
    target: Point,
    len1: float,
    len2: float,
    bend: float = 1.0,
) -> Tuple[float, float]:
    """Solve a two-segment chain from ``root`` toward ``target``.

    Returns ``(world_deg_upper, world_deg_lower)``. ``bend=+1`` puts the
    middle joint on the clockwise side of the root→target line — for a
    right-facing character with limbs hanging down that is the +x side,
    i.e. knees forward; use ``bend=-1`` for elbows. Unreachable targets are
    clamped to the chain's reach (segments straighten / fold)."""
    dx, dy = target[0] - root[0], target[1] - root[1]
    d = math.hypot(dx, dy)
    lo = abs(len1 - len2) + 1e-6
    hi = len1 + len2 - 1e-6
    d = clamp(d, lo, hi)
    base = math.atan2(dy, dx)
    cos_a = (len1 * len1 + d * d - len2 * len2) / (2.0 * len1 * d)
    a = math.acos(clamp(cos_a, -1.0, 1.0))
    a1 = base - (1.0 if bend >= 0 else -1.0) * a
    joint = add(root, (math.cos(a1) * len1, math.sin(a1) * len1))
    # Aim the lower segment at the *clamped* target so the chain stays
    # consistent even when the requested target was out of reach.
    clamped_target = add(root, (math.cos(base) * d, math.sin(base) * d))
    a2 = math.atan2(clamped_target[1] - joint[1], clamped_target[0] - joint[0])
    return math.degrees(a1), math.degrees(a2)


# ---- Keyframe channels -------------------------------------------------------


def _linear(t: float) -> float:
    return t


def _ease_in_cubic(t: float) -> float:
    t = clamp(t, 0.0, 1.0)
    return t * t * t


_EASES: Dict[str, EaseFn] = {
    "linear": _linear,
    "smooth": smoothstep,
    "out": ease_out_cubic,
    "in": _ease_in_cubic,
    "sine": ease_in_out_sine,
}


@dataclass(frozen=True)
class Key:
    t: float
    value: float
    ease: EaseFn
    """Easing applied over the segment arriving at this key."""


KeySpec = Union[Tuple[float, float], Tuple[float, float, Union[str, EaseFn]]]


class Channel:
    """A keyframed scalar. Keys are ``(t, value)`` or ``(t, value, ease)``
    with ``t`` in [0, 1]; ``ease`` names an entry in the ease table or is a
    callable, and shapes the segment *into* that key (default ``smooth``).
    Looping clips wrap the segment from the last key back to the first."""

    def __init__(self, *keys: KeySpec, default_ease: Union[str, EaseFn] = "smooth"):
        if not keys:
            raise ValueError("Channel needs at least one key")
        default_fn = _EASES[default_ease] if isinstance(default_ease, str) else default_ease
        parsed: List[Key] = []
        for k in keys:
            if len(k) == 2:
                t, v = k  # type: ignore[misc]
                ease: EaseFn = default_fn
            else:
                t, v, e = k  # type: ignore[misc]
                ease = _EASES[e] if isinstance(e, str) else e
            parsed.append(Key(float(t), float(v), ease))
        parsed.sort(key=lambda key: key.t)
        self.keys: Tuple[Key, ...] = tuple(parsed)

    def sample(self, t: float, loop: bool = True) -> float:
        keys = self.keys
        if len(keys) == 1:
            return keys[0].value
        if loop:
            t = t % 1.0
            if t < keys[0].t or t >= keys[-1].t:
                a, b = keys[-1], keys[0]
                span = (1.0 - a.t) + b.t
                if span <= 1e-9:
                    return b.value
                u = ((t - a.t) % 1.0) / span
                return lerp(a.value, b.value, b.ease(u))
        else:
            if t <= keys[0].t:
                return keys[0].value
            if t >= keys[-1].t:
                return keys[-1].value
        for i in range(len(keys) - 1):
            a, b = keys[i], keys[i + 1]
            if a.t <= t <= b.t:
                span = b.t - a.t
                u = 0.0 if span <= 1e-9 else (t - a.t) / span
                return lerp(a.value, b.value, b.ease(u))
        return keys[-1].value  # pragma: no cover - unreachable with sorted keys


ChannelLike = Union[Channel, Callable[[float], float], float, int]


class Clip:
    """A named bundle of channels sampled together at normalized time ``t``.

    Channel values may be [`Channel`] instances, plain callables ``t ->
    value`` (procedural tracks like IK foot trajectories), or constants."""

    def __init__(self, loop: bool = True, channels: Optional[Mapping[str, ChannelLike]] = None):
        self.loop = loop
        self.channels: Dict[str, ChannelLike] = dict(channels or {})

    def sample(self, t: float) -> Dict[str, float]:
        out: Dict[str, float] = {}
        for name, ch in self.channels.items():
            if isinstance(ch, Channel):
                out[name] = ch.sample(t, self.loop)
            elif callable(ch):
                out[name] = float(ch(t % 1.0 if self.loop else clamp(t, 0.0, 1.0)))
            else:
                out[name] = float(ch)
        return out


# ---- Rig: z-ordered part painters bound to bones ------------------------------


class PartCtx:
    """Paint-time context handed to each part function.

    ``bw`` is the part's own bone; ``world`` has every bone for parts that
    span several (an arm painter reading both segments). Helpers map
    bone-local or world points into supersampled canvas pixels."""

    def __init__(
        self,
        img: Image.Image,
        draw: ImageDraw.ImageDraw,
        bw: BoneWorld,
        world: Mapping[str, BoneWorld],
        scale: float,
        params: Mapping[str, float],
    ) -> None:
        self.img = img
        self.draw = draw
        self.bw = bw
        self.world = world
        self.scale = scale
        self.params = params

    def L(self, v: float) -> float:
        """Scalar length in canvas pixels."""
        return v * self.scale

    def cw(self, world_pt: Point) -> Point:
        """World point -> canvas pixels."""
        return (world_pt[0] * self.scale, world_pt[1] * self.scale)

    def pt(self, local: Point) -> Point:
        """Bone-local point -> canvas pixels."""
        return self.cw(self.bw.to_world(local))

    def pts(self, locals_: Sequence[Point]) -> List[Point]:
        return [self.pt(p) for p in locals_]


@dataclass(frozen=True)
class Part:
    name: str
    bone: str
    z: float
    fn: Callable[[PartCtx], None]


class Rig:
    def __init__(self, skeleton: Skeleton) -> None:
        self.skeleton = skeleton
        self.parts: List[Part] = []

    def part(self, name: str, bone: str, z: float, fn: Callable[[PartCtx], None]) -> "Rig":
        if bone not in self.skeleton.bones:
            raise ValueError(f"part {name!r}: unknown bone {bone!r}")
        self.parts.append(Part(name, bone, z, fn))
        return self

    def draw(
        self,
        img: Image.Image,
        draw: ImageDraw.ImageDraw,
        world: Mapping[str, BoneWorld],
        scale: float,
        params: Mapping[str, float],
    ) -> None:
        for part in sorted(self.parts, key=lambda p: p.z):
            part.fn(PartCtx(img, draw, world[part.bone], world, scale, params))


# ---- Polygon helpers -----------------------------------------------------------


def rounded_polygon(pts: Sequence[Point], radius: float, steps: int = 6) -> List[Point]:
    """Replace each corner of ``pts`` with an arc of the given radius
    (clamped per-corner to fit the adjacent edges). Returns a denser point
    list suitable for ``ImageDraw.polygon``."""
    n = len(pts)
    out: List[Point] = []
    for i in range(n):
        p_prev, v, p_next = pts[(i - 1) % n], pts[i], pts[(i + 1) % n]
        d1 = (p_prev[0] - v[0], p_prev[1] - v[1])
        d2 = (p_next[0] - v[0], p_next[1] - v[1])
        l1, l2 = math.hypot(*d1), math.hypot(*d2)
        if l1 < 1e-6 or l2 < 1e-6:
            out.append(v)
            continue
        u1 = (d1[0] / l1, d1[1] / l1)
        u2 = (d2[0] / l2, d2[1] / l2)
        dot = clamp(u1[0] * u2[0] + u1[1] * u2[1], -1.0, 1.0)
        theta = math.acos(dot)
        if theta < 1e-3 or math.pi - theta < 1e-3:
            out.append(v)
            continue
        tl = radius / math.tan(theta / 2.0)
        tl = min(tl, 0.45 * l1, 0.45 * l2)
        r = tl * math.tan(theta / 2.0)
        t1 = (v[0] + u1[0] * tl, v[1] + u1[1] * tl)
        t2 = (v[0] + u2[0] * tl, v[1] + u2[1] * tl)
        bis = (u1[0] + u2[0], u1[1] + u2[1])
        bl = math.hypot(*bis)
        center = (
            v[0] + bis[0] / bl * (r / math.sin(theta / 2.0)),
            v[1] + bis[1] / bl * (r / math.sin(theta / 2.0)),
        )
        a1 = math.atan2(t1[1] - center[1], t1[0] - center[0])
        a2 = math.atan2(t2[1] - center[1], t2[0] - center[0])
        da = a2 - a1
        while da > math.pi:
            da -= math.tau
        while da < -math.pi:
            da += math.tau
        for k in range(steps + 1):
            a = a1 + da * (k / steps)
            out.append((center[0] + r * math.cos(a), center[1] + r * math.sin(a)))
    return out


def draw_polygon(
    draw: ImageDraw.ImageDraw,
    pts: Sequence[Point],
    fill: Color,
    outline: Optional[Color] = None,
    outline_w: float = 0.0,
) -> None:
    """Filled polygon with an optional stroked outline (round joints)."""
    draw.polygon(list(pts), fill=fill)
    if outline is not None and outline_w > 0:
        closed = list(pts) + [pts[0]]
        draw.line(closed, fill=outline, width=max(1, int(round(outline_w))), joint="curve")


def composite_polygon(
    img: Image.Image,
    pts: Sequence[Point],
    fill: Color,
    outline: Optional[Color] = None,
    outline_w: float = 0.0,
) -> None:
    """Translucent polygon via REAL alpha compositing (the gnu_ton pattern).

    Draws onto a fresh transparent layer and ``alpha_composite``s it over
    ``img`` in place. This is the only correct way to lay a translucent
    detail over already-drawn art — drawing directly replaces destination
    alpha, and ImageDraw's "RGBA" blend mode does not composite reliably."""
    layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
    draw_polygon(ImageDraw.Draw(layer), pts, fill, outline, outline_w)
    img.alpha_composite(layer)
