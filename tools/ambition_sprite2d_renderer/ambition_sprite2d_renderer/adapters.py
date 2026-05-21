from __future__ import annotations

from dataclasses import asdict, fields, is_dataclass, replace
from typing import Any, Dict, List, Tuple

from PIL import Image

from .animation_vocab import FULL_PLAYER_ANIMATION_ORDER, ordered_subset
from .config import CharacterJob
from .targets.boss_side import AISlopZetaGenerator, parse_background as boss_parse_background
from .targets.goblin_side import SideGoblinGenerator, parse_background as goblin_parse_background
from .targets.ninja_side import NinjaSideGenerator, parse_background as ninja_parse_background
from .targets.robot_side import SideRobotGenerator
from .targets.sandbag import ADAPTER_ANIMATIONS as SANDBAG_ANIMATIONS, SandbagSpec, render_frame as render_sandbag_frame
from .targets.robot25d import parse_background as robot_parse_background
from .targets.toon_side import ToonSideGenerator, parse_background as toon_parse_background
from .targets.trent_elder import TrentElderGenerator, parse_background as trent_parse_background


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

    def _apply_overrides(self, spec: Any, job: CharacterJob) -> Any:
        overrides = dict(getattr(job, "spec_overrides", {}) or {})
        if not overrides:
            return spec
        if not is_dataclass(spec):
            raise TypeError(f"spec overrides are only supported for dataclass specs (target={job.target})")
        known = {f.name for f in fields(spec)}
        unknown = sorted(set(overrides) - known)
        if unknown:
            raise KeyError(f"unknown spec override keys for {job.target}: {unknown}; available={sorted(known)}")
        return replace(spec, **overrides)


class GoblinAdapter(BaseAdapter):
    target = "goblin"

    def __init__(self) -> None:
        self.generator = SideGoblinGenerator()

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(self.generator.SPRITESHEET_ANIMATIONS)

    def sample_spec(self, job: CharacterJob) -> Any:
        spec = self.generator.sample_spec(job.seed, job.archetype, job.held_item)
        return self._apply_overrides(spec, job)

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


class BossAdapter(BaseAdapter):
    target = "boss"

    def __init__(self) -> None:
        self.generator = AISlopZetaGenerator()

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(self.generator.ANIMATIONS)

    def canonical_pose(self) -> Tuple[str, int]:
        return ("rest", 1)

    def sample_spec(self, job: CharacterJob) -> Any:
        spec = self.generator.sample_spec(job.seed, job.archetype)
        return self._apply_overrides(spec, job)

    def render_frame(self, spec: Any, animation: str, frame_index: int, size: Tuple[int, int], job: CharacterJob) -> Image.Image:
        anim = self.animations()[animation]
        return self.generator.render_animation_frame(
            spec,
            animation,
            frame_index % anim["frames"],
            anim["frames"],
            size,
            background=boss_parse_background(job.render.background),
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
        spec = self.generator.sample_spec(job.seed, job.archetype)
        return self._apply_overrides(spec, job)

    def render_frame(self, spec: Any, animation: str, frame_index: int, size: Tuple[int, int], job: CharacterJob) -> Image.Image:
        anim = self.animations()[animation]
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


class NinjaAdapter(BaseAdapter):
    target = "ninja"

    def __init__(self) -> None:
        self.generator = NinjaSideGenerator()

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(self.generator.ANIMATIONS)

    def sample_spec(self, job: CharacterJob) -> Any:
        spec = self.generator.sample_spec(job.seed, job.archetype)
        return self._apply_overrides(spec, job)

    def spec_dict(self, spec: Any) -> Dict[str, Any]:
        return spec.to_dict()

    def render_frame(self, spec: Any, animation: str, frame_index: int, size: Tuple[int, int], job: CharacterJob) -> Image.Image:
        anim = self.animations()[animation]
        return self.generator.render_animation_frame(
            spec,
            animation,
            frame_index % anim["frames"],
            anim["frames"],
            size,
            background=ninja_parse_background(job.render.background),
            supersample=job.render.supersample,
            downsample=job.render.downsample,
        )


class SandbagAdapter(BaseAdapter):
    target = "sandbag"

    def animations(self) -> Dict[str, Dict[str, int]]:
        return ordered_subset(SANDBAG_ANIMATIONS, FULL_PLAYER_ANIMATION_ORDER)

    def canonical_pose(self) -> Tuple[str, int]:
        return ("idle", 1)

    def sample_spec(self, job: CharacterJob) -> SandbagSpec:
        spec = SandbagSpec(
            seed=job.seed,
            archetype=job.archetype,
            variant=str(job.variant or "classic"),
        )
        return self._apply_overrides(spec, job)

    def spec_dict(self, spec: SandbagSpec) -> Dict[str, Any]:
        return spec.to_dict()

    def render_frame(self, spec: SandbagSpec, animation: str, frame_index: int, size: Tuple[int, int], job: CharacterJob) -> Image.Image:
        anim = self.animations()[animation]
        frame = render_sandbag_frame(animation, frame_index % anim["frames"], anim["frames"])
        if frame.size != size:
            frame = frame.resize(size, Image.Resampling.LANCZOS)
        return frame


class ToonAdapter(BaseAdapter):
    target = "toon"

    def __init__(self) -> None:
        self.generator = ToonSideGenerator()

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(self.generator.ANIMATIONS)

    def sample_spec(self, job: CharacterJob) -> Any:
        spec = self.generator.sample_spec(job.seed, job.archetype)
        if job.name:
            spec = replace(spec, name=job.name)
        return self._apply_overrides(spec, job)

    def render_frame(self, spec: Any, animation: str, frame_index: int, size: Tuple[int, int], job: CharacterJob) -> Image.Image:
        anim = self.animations()[animation]
        return self.generator.render_animation_frame(
            spec,
            animation,
            frame_index % anim["frames"],
            anim["frames"],
            size,
            background=toon_parse_background(job.render.background),
            supersample=job.render.supersample,
            downsample=job.render.downsample,
        )


class TrentElderAdapter(BaseAdapter):
    """Bespoke target for Trent. Single-archetype; see
    `targets/trent_elder.py` for the design rationale."""

    target = "trent_elder"

    def __init__(self) -> None:
        self.generator = TrentElderGenerator()

    def animations(self) -> Dict[str, Dict[str, int]]:
        return dict(self.generator.ANIMATIONS)

    def sample_spec(self, job: CharacterJob) -> Any:
        spec = self.generator.sample_spec(job.seed, job.archetype)
        if job.name:
            spec = replace(spec, name=job.name)
        return self._apply_overrides(spec, job)

    def render_frame(self, spec: Any, animation: str, frame_index: int, size: Tuple[int, int], job: CharacterJob) -> Image.Image:
        anim = self.animations()[animation]
        return self.generator.render_animation_frame(
            spec,
            animation,
            frame_index % anim["frames"],
            anim["frames"],
            size,
            background=trent_parse_background(job.render.background),
            supersample=job.render.supersample,
            downsample=job.render.downsample,
        )


TARGETS: Dict[str, BaseAdapter] = {
    "boss": BossAdapter(),
    "goblin": GoblinAdapter(),
    "ninja": NinjaAdapter(),
    "robot": RobotAdapter(),
    "sandbag": SandbagAdapter(),
    "toon": ToonAdapter(),
    "trent_elder": TrentElderAdapter(),
}


def get_adapter(target: str) -> BaseAdapter:
    try:
        return TARGETS[target]
    except KeyError as ex:
        raise KeyError(f"unknown target {target!r}; available={sorted(TARGETS)}") from ex
