from __future__ import annotations

"""Reusable rig primitives for future non-legacy characters.

The package still includes legacy-compatible goblin and robot renderers, but this
module is the intended shared authoring layer: bones expose named sockets,
attachments use local forward/side/up axes, and validators can check that facial
or weapon anchors remain coherent before rendering.
"""

from dataclasses import dataclass, field
from typing import Dict, Tuple

import numpy as np
from pydantic import BaseModel, ConfigDict, Field, field_validator

Point3 = Tuple[float, float, float]


class SocketSpec(BaseModel):
    model_config = ConfigDict(extra="forbid")
    bone: str
    offset: Point3 = (0.0, 0.0, 0.0)
    forward: Point3 = (1.0, 0.0, 0.0)
    up: Point3 = (0.0, 1.0, 0.0)


class FaceGuide(BaseModel):
    """Editable constraints for face placement."""

    model_config = ConfigDict(extra="forbid")
    near_eye: SocketSpec
    far_eye: SocketSpec
    nose: SocketSpec
    mouth: SocketSpec
    min_eye_gap: float = 0.12
    max_eye_width_ratio: float = 0.20

    @field_validator("min_eye_gap", "max_eye_width_ratio")
    @classmethod
    def positive_ratio(cls, value: float) -> float:
        if value <= 0:
            raise ValueError("face ratios must be positive")
        return value


@dataclass
class Bone:
    name: str
    head: Point3
    tail: Point3
    parent: str | None = None
    sockets: Dict[str, SocketSpec] = field(default_factory=dict)

    @property
    def vector(self) -> np.ndarray:
        return np.asarray(self.tail, dtype=float) - np.asarray(self.head, dtype=float)

    @property
    def length(self) -> float:
        return float(np.linalg.norm(self.vector))


@dataclass
class Rig:
    bones: Dict[str, Bone] = field(default_factory=dict)

    def add_bone(self, bone: Bone) -> None:
        if bone.name in self.bones:
            raise KeyError(f"bone already exists: {bone.name}")
        if bone.parent is not None and bone.parent not in self.bones:
            raise KeyError(f"unknown parent bone {bone.parent!r} for {bone.name!r}")
        self.bones[bone.name] = bone

    def validate(self) -> None:
        for bone in self.bones.values():
            if bone.length <= 1e-9:
                raise ValueError(f"bone {bone.name!r} has zero length")
            for socket_name, socket in bone.sockets.items():
                if socket.bone != bone.name:
                    raise ValueError(f"socket {socket_name!r} attached to {bone.name!r} names bone {socket.bone!r}")
