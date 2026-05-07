from __future__ import annotations

from dataclasses import asdict, dataclass
from typing import Any, Dict, Optional


@dataclass
class ViewSpec:
    facing: str = "right"
    camera_style: str = "side_3q"
    camera_x: float = 2.05
    camera_y: float = -6.2
    camera_z: float = 1.42
    target_x: float = 0.0
    target_y: float = 0.0
    target_z: float = 1.1
    ortho_scale: float = 2.92


@dataclass
class RobotSpec:
    target: str
    seed: int
    archetype: str
    view: ViewSpec
    head_size: float = 0.70
    body_width: float = 0.46
    body_height: float = 0.44
    body_depth: float = 0.32
    arm_length: float = 0.34
    forearm_length: float = 0.24
    leg_length: float = 0.30
    shin_length: float = 0.25
    primary_color: str = "#F6F7FB"
    primary_shadow: str = "#CBD5E6"
    accent_color: str = "#00E5FF"
    accent2_color: str = "#8E5BFF"
    dark_color: str = "#17141F"
    metal_color: str = "#AEB8C8"


@dataclass
class GoblinSpec:
    target: str
    seed: int
    archetype: str
    held_item: Optional[str]
    view: ViewSpec
    head_size: float = 0.62
    body_width: float = 0.38
    body_height: float = 0.42
    body_depth: float = 0.30
    arm_length: float = 0.30
    forearm_length: float = 0.26
    leg_length: float = 0.30
    shin_length: float = 0.26
    ear_length: float = 0.34
    skin_color: str = "#7A6A97"
    skin_shadow: str = "#564870"
    cloth_color: str = "#4B2A6B"
    cloth_shadow: str = "#2F1847"
    accent_color: str = "#FF4FD8"
    accent2_color: str = "#7D4DFF"
    eye_color: str = "#FF9AF6"
    metal_color: str = "#D5C1FF"


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
