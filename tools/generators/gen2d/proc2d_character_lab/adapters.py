from __future__ import annotations

from dataclasses import asdict, is_dataclass
from typing import Any, Dict, List, Tuple

from PIL import Image

from .config import CharacterJob
from .targets.goblin_side import SideGoblinGenerator, parse_background as goblin_parse_background
from .targets.robot_side import SideRobotGenerator
from .targets.robot25d import parse_background as robot_parse_background


def _dataclass_dict(obj: Any) -> Dict[str, Any]:
    return asdict(obj) if is_dataclass(obj) else dict(obj)


class BaseAdapter:
    target: str

    def animations(self) -> Dict[str, Dict[str, int]]:
        raise NotImplementedError

    def default_animations(self) -> List[str]:
        return list(self.animations().keys())

    def canonical_pose(self) -> Tuple[str, int]:
        return ("idle", 1)

    def sample_spec(self, job: CharacterJob) -> Any:
        raise NotImplementedError

    def spec_dict(self, spec: Any) -> Dict[str, Any]:
        return _dataclass_dict(spec)

    def render_frame(self, spec: Any, animation: str, frame_index: int, size: Tuple[int, int], job: CharacterJob) -> Image.Image:
        raise NotImplementedError

    def render_single(self, spec: Any, animation: str, frame_index: int, job: CharacterJob) -> Image.Image:
        r = job.render
        return self.render_frame(spec, animation, frame_index, (r.single_width, r.single_height), job)

    def render_canonical(self, spec: Any, job: CharacterJob) -> Image.Image:
        animation, frame_index = self.canonical_pose()
        return self.render_single(spec, animation, frame_index, job)


class GoblinAdapter(BaseAdapter):
    target = "goblin"

    def __init__(self) -> None:
        self.generator = SideGoblinGenerator()

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(self.generator.SPRITESHEET_ANIMATIONS)

    def sample_spec(self, job: CharacterJob) -> Any:
        return self.generator.sample_spec(job.seed, job.archetype, job.held_item)

    def render_frame(self, spec: Any, animation: str, frame_index: int, size: Tuple[int, int], job: CharacterJob) -> Image.Image:
        anim = self.animations()[animation]
        return self.generator.render_animation_frame(
            spec,
            animation,
            frame_index % anim["frames"],
            anim["frames"],
            size,
            background=goblin_parse_background(job.render.background),
            supersample=job.render.supersample,
            downsample=job.render.downsample,
        )


class RobotAdapter(BaseAdapter):
    target = "robot"

    def __init__(self) -> None:
        self.generator = SideRobotGenerator()

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(self.generator.ANIMATIONS)

    def sample_spec(self, job: CharacterJob) -> Any:
        return self.generator.sample_spec(job.seed, job.archetype)

    def render_frame(self, spec: Any, animation: str, frame_index: int, size: Tuple[int, int], job: CharacterJob) -> Image.Image:
        anim = self.animations()[animation]
        # Both robot and goblin are now natively right-facing; no adapter flip.
        return self.generator.render_animation_frame(
            spec,
            animation,
            frame_index % anim["frames"],
            anim["frames"],
            size,
            background=robot_parse_background(job.render.background),
            supersample=job.render.supersample,
            downsample=job.render.downsample,
        )


TARGETS: Dict[str, BaseAdapter] = {
    "goblin": GoblinAdapter(),
    "robot": RobotAdapter(),
}


def get_adapter(target: str) -> BaseAdapter:
    try:
        return TARGETS[target]
    except KeyError as ex:
        raise KeyError(f"unknown target {target!r}; available={sorted(TARGETS)}") from ex
