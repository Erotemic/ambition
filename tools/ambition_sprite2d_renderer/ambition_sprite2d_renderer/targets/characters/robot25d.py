from __future__ import annotations

from dataclasses import dataclass
from typing import Optional, Tuple

from PIL import ImageColor

from ...authoring.rig import clamp, ease_in_out_sine, ease_out_cubic, lerp, smoothstep

Color = Tuple[int, int, int, int]


def rgb(hex_color: str, alpha: int = 255) -> Color:
    r, g, b = ImageColor.getrgb(hex_color)
    return (r, g, b, alpha)


def parse_background(value: str) -> Optional[Color]:
    return None if str(value).lower() == "transparent" else rgb(str(value))


@dataclass(frozen=True)
class BotSpec:
    target: str = "robot"
    seed: int = 0
    archetype: str = "cute_scout"
    palette_name: str = "classic"
    head_w: float = 42.0
    head_h: float = 34.0
    body_w: float = 26.0
    body_h: float = 25.0
    arm_upper: float = 14.0
    arm_lower: float = 12.0
    leg_upper: float = 14.0
    leg_lower: float = 12.0
    visor_w: float = 24.0
    visor_h: float = 12.0
    antenna_h: float = 12.0
    blade_len: float = 31.0
    # Multiplier applied to the side-robot renderer's hard-coded vertical
    # silhouette offsets (body/head/hip/shoulder anchor distances from the
    # ground line). Default 1.0 keeps every existing character unchanged.
    # < 1.0 produces a chibi/compact silhouette: pair with shorter
    # arm/leg lengths so the legs still reach the ground anchor.
    vertical_scale: float = 1.0


@dataclass
class Pose:
    root_x: float = 0.0
    root_y: float = 0.0
    body_bob: float = 0.0
    body_tilt: float = 0.0
    head_tilt: float = 0.0
    blink: bool = False
    eye_squint: float = 0.0
    far_arm_upper: float = 145.0
    far_arm_lower: float = 120.0
    near_arm_upper: float = 35.0
    near_arm_lower: float = 18.0
    far_leg_upper: float = 105.0
    far_leg_lower: float = 95.0
    near_leg_upper: float = 72.0
    near_leg_lower: float = 85.0
    slash: float = 0.0
    slash_arc: float = 0.0
    # Direction of the active slash arc. Drives the blade base angle and the
    # decorative arc visuals. Values: "side" (default forward slash), "up",
    # "down", "back", and the aerial variants "air_neutral", "air_forward",
    # "air_back", "air_down", "air_up". "side" preserves the legacy behaviour.
    slash_dir: str = "side"
    dash: float = 0.0
    collapse: float = 0.0
    dead: bool = False
    # +1.0 = facing toward screen-right (default), -1.0 = turned back toward
    # screen-left. Intermediate values softly slide the visor + antenna inside
    # the rigid head shell so wall / ledge poses can glance backward.
    head_look: float = 1.0
    # Optional local offsets for the rigid head anchor, expressed in unscaled
    # sprite pixels.
    head_dx: float = 0.0
    head_dy: float = 0.0
    # Additional post-pose rotation, applied to the fully assembled actor layer.
    # Used for tuck-and-roll motions so the entire silhouette rotates around its
    # approximate center of mass rather than only spinning the head/body parts.
    whole_body_rotation: float = 0.0


class Robot25DGenerator:
    """Compatibility shim for the older robot target name."""

    name = "robot"
