from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple

import yaml

DEFAULT_ANIMATIONS = [
    "idle",
    "walk",
    "run",
    "jump",
    "fall",
    "slash",
    "hit",
    "death",
    "blink_out",
    "blink_in",
    "dash",
]


@dataclass
class RenderConfig:
    frame_width: int = 128
    frame_height: int = 128
    single_width: int = 128
    single_height: int = 128
    supersample: int = 4
    downsample: str = "lanczos"
    background: str = "transparent"
    sheet_background: str = "transparent"
    border: int = 0
    label_width: int = 96


@dataclass
class CharacterJob:
    target: str
    seed: int = 0
    archetype: str = "default"
    held_item: Optional[str] = None
    animations: List[str] = field(default_factory=lambda: list(DEFAULT_ANIMATIONS))
    render: RenderConfig = field(default_factory=RenderConfig)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "CharacterJob":
        render = RenderConfig(**dict(data.get("render") or {}))
        animations = list(data.get("animations") or DEFAULT_ANIMATIONS)
        return cls(
            target=str(data["target"]),
            seed=int(data.get("seed", 0)),
            archetype=str(data.get("archetype", "default")),
            held_item=data.get("held_item"),
            animations=animations,
            render=render,
        )

    @classmethod
    def load(cls, path: str | Path) -> "CharacterJob":
        with open(path, "r", encoding="utf8") as file:
            data = yaml.safe_load(file) or {}
        if not isinstance(data, dict):
            raise TypeError(f"expected mapping in {path!s}")
        return cls.from_dict(data)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "target": self.target,
            "seed": self.seed,
            "archetype": self.archetype,
            "held_item": self.held_item,
            "animations": list(self.animations),
            "render": dict(self.render.__dict__),
        }


def load_jobs(config_dir: str | Path) -> List[Tuple[Path, CharacterJob]]:
    config_dir = Path(config_dir)
    jobs: List[Tuple[Path, CharacterJob]] = []
    for path in sorted(config_dir.glob("*.yaml")):
        jobs.append((path, CharacterJob.load(path)))
    if not jobs:
        raise FileNotFoundError(f"no .yaml configs found in {config_dir}")
    return jobs
