from __future__ import annotations

import random
from dataclasses import asdict, is_dataclass
from typing import Any, Dict, List, Tuple

from .config import CharacterJob
from .specs import (
    CANONICAL_POSES,
    GOBLIN_ANIMATIONS,
    ROBOT_ANIMATIONS,
    GoblinSpec,
    RobotSpec,
    ViewSpec,
)


class BaseAdapter:
    target: str

    def animations(self) -> Dict[str, Dict[str, int]]:
        raise NotImplementedError

    def default_animations(self) -> List[str]:
        return list(self.animations().keys())

    def sample_spec(self, job: CharacterJob) -> Any:
        raise NotImplementedError

    def spec_dict(self, spec: Any) -> Dict[str, Any]:
        return asdict(spec) if is_dataclass(spec) else dict(spec)

    def canonical_pose(self) -> Tuple[str, int]:
        return CANONICAL_POSES[self.target]


class RobotAdapter(BaseAdapter):
    target = "robot"

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(ROBOT_ANIMATIONS)

    def sample_spec(self, job: CharacterJob) -> Any:
        rng = random.Random(job.seed)
        view = ViewSpec()
        head = 0.70 + rng.uniform(-0.03, 0.03)
        return RobotSpec(
            target="robot",
            seed=job.seed,
            archetype=job.archetype,
            view=view,
            head_size=head,
            body_width=0.52 + rng.uniform(-0.02, 0.02),
            body_height=0.40 + rng.uniform(-0.02, 0.02),
            body_depth=0.34 + rng.uniform(-0.02, 0.02),
            arm_length=0.29 + rng.uniform(-0.02, 0.02),
            forearm_length=0.22 + rng.uniform(-0.02, 0.02),
            leg_length=0.28 + rng.uniform(-0.02, 0.02),
            shin_length=0.23 + rng.uniform(-0.02, 0.02),
        )


class GoblinAdapter(BaseAdapter):
    target = "goblin"

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(GOBLIN_ANIMATIONS)

    def sample_spec(self, job: CharacterJob) -> Any:
        rng = random.Random(job.seed)
        view = ViewSpec(camera_x=0.95, camera_y=-6.2, camera_z=1.34, target_z=1.06, ortho_scale=2.95)
        archetype = job.archetype or "shadow_scout"
        item = job.held_item or "spear"
        return GoblinSpec(
            target="goblin",
            seed=job.seed,
            archetype=archetype,
            held_item=item,
            view=view,
            head_size=0.72 + rng.uniform(-0.03, 0.03),
            body_width=0.42 + rng.uniform(-0.02, 0.02),
            body_height=0.42 + rng.uniform(-0.02, 0.02),
            body_depth=0.32 + rng.uniform(-0.02, 0.02),
            arm_length=0.30 + rng.uniform(-0.02, 0.02),
            forearm_length=0.24 + rng.uniform(-0.02, 0.02),
            leg_length=0.27 + rng.uniform(-0.02, 0.02),
            shin_length=0.24 + rng.uniform(-0.02, 0.02),
            ear_length=0.42 + rng.uniform(-0.03, 0.03),
        )


TARGETS: Dict[str, BaseAdapter] = {
    "robot": RobotAdapter(),
    "goblin": GoblinAdapter(),
}


def get_adapter(target: str) -> BaseAdapter:
    try:
        return TARGETS[target]
    except KeyError as ex:
        raise KeyError(f"unknown target {target!r}; available={sorted(TARGETS)}") from ex
