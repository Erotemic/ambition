from __future__ import annotations

import math
from typing import Tuple

Point = Tuple[float, float]
Color = Tuple[int, int, int, int]


def clamp(v: float, lo: float, hi: float) -> float:
    return max(lo, min(hi, v))


def lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def smoothstep(t: float) -> float:
    t = clamp(t, 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def ease_in_out_sine(t: float) -> float:
    t = clamp(t, 0.0, 1.0)
    return -(math.cos(math.pi * t) - 1.0) / 2.0


def ease_out_cubic(t: float) -> float:
    t = clamp(t, 0.0, 1.0)
    return 1.0 - (1.0 - t) ** 3


def vec(length: float, degrees: float) -> Point:
    a = math.radians(degrees)
    return (math.cos(a) * length, math.sin(a) * length)


def add(a: Point, b: Point) -> Point:
    return (a[0] + b[0], a[1] + b[1])


def bbox_from_center(center: Point, w: float, h: float):
    return (
        center[0] - w / 2.0,
        center[1] - h / 2.0,
        center[0] + w / 2.0,
        center[1] + h / 2.0,
    )
