from __future__ import annotations

from dataclasses import asdict, dataclass
from typing import Any, Dict, Optional


@dataclass
class ViewSpec:
    facing: str = "right"
    camera_style: str = "side_3q"
    camera_x: float = 1.05
    camera_y: float = -6.0
    camera_z: float = 1.45
    target_x: float = 0.0
    target_y: float = 0.0
    target_z: float = 1.1
    ortho_scale: float = 2.75


@dataclass
class RobotSpec:
    target: str
    seed: int
    archetype: str
    view: ViewSpec
    head_size: float = 0.70
    body_width: float = 0.52
    body_height: float = 0.40
    body_depth: float = 0.34
    arm_length: float = 0.29
    forearm_length: float = 0.22
    leg_length: float = 0.28
    shin_length: float = 0.23
    primary_color: str = "#F8F9FC"
    primary_shadow: str = "#D3DAE8"
    accent_color: str = "#27E7FF"
    accent2_color: str = "#B285FF"
    dark_color: str = "#19171F"
    metal_color: str = "#B8C2D2"


@dataclass
class GoblinSpec:
    target: str
    seed: int
    archetype: str
    held_item: Optional[str]
    view: ViewSpec
    head_size: float = 0.72
    body_width: float = 0.42
    body_height: float = 0.42
    body_depth: float = 0.32
    arm_length: float = 0.30
    forearm_length: float = 0.24
    leg_length: float = 0.27
    shin_length: float = 0.24
    ear_length: float = 0.42
    skin_color: str = "#554B60"
    skin_shadow: str = "#3D3444"
    cloth_color: str = "#241D2E"
    cloth_shadow: str = "#17131D"
    accent_color: str = "#E04FFF"
    accent2_color: str = "#8B57FF"
    eye_color: str = "#EC58FF"
    metal_color: str = "#CFBEFF"


CANONICAL_POSES = {
    "robot": ("idle", 2),
    "goblin": ("idle", 2),
}


ROBOT_ANIMATIONS = {
    "idle": {"frames": 7, "duration_ms": 110},
    "walk": {"frames": 7, "duration_ms": 90},
    "run": {"frames": 7, "duration_ms": 75},
    "jump": {"frames": 7, "duration_ms": 90},
    "fly": {"frames": 7, "duration_ms": 90},
    "dash": {"frames": 6, "duration_ms": 70},
    "slash": {"frames": 6, "duration_ms": 85},
    "hit": {"frames": 6, "duration_ms": 95},
}


GOBLIN_ANIMATIONS = {
    "idle": {"frames": 7, "duration_ms": 110},
    "walk": {"frames": 7, "duration_ms": 90},
    "run": {"frames": 7, "duration_ms": 75},
    "jump": {"frames": 7, "duration_ms": 95},
    "fall": {"frames": 7, "duration_ms": 95},
    "slash": {"frames": 6, "duration_ms": 85},
    "hurt": {"frames": 6, "duration_ms": 95},
    "death": {"frames": 7, "duration_ms": 120},
}


def spec_to_dict(spec: Any) -> Dict[str, Any]:
    return asdict(spec)
