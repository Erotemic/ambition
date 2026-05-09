from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

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
    crop: bool = True
    crop_padding: int = 2


@dataclass
class CharacterJob:
    target: str
    name: Optional[str] = None
    output_name: Optional[str] = None
    seed: int = 0
    archetype: str = "default"
    variant: Optional[str] = None
    held_item: Optional[str] = None
    spec_overrides: Dict[str, Any] = field(default_factory=dict)
    animations: List[str] = field(default_factory=lambda: list(DEFAULT_ANIMATIONS))
    render: RenderConfig = field(default_factory=RenderConfig)
    faction: Optional[str] = None
    role: Optional[str] = None
    music_cue: Optional[str] = None
    tags: List[str] = field(default_factory=list)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "CharacterJob":
        render = RenderConfig(**dict(data.get("render") or {}))
        animations = list(data.get("animations") or DEFAULT_ANIMATIONS)
        spec_overrides = dict(data.get("spec") or data.get("spec_overrides") or {})
        return cls(
            target=str(data["target"]),
            name=data.get("name"),
            output_name=data.get("output_name"),
            seed=int(data.get("seed", 0)),
            archetype=str(data.get("archetype", "default")),
            variant=data.get("variant"),
            held_item=data.get("held_item"),
            spec_overrides=spec_overrides,
            animations=animations,
            render=render,
            faction=data.get("faction"),
            role=data.get("role"),
            music_cue=data.get("music_cue"),
            tags=list(data.get("tags") or []),
        )

    @classmethod
    def load(cls, path: str | Path) -> "CharacterJob":
        with open(path, "r", encoding="utf8") as file:
            data = yaml.safe_load(file) or {}
        if not isinstance(data, dict):
            raise TypeError(f"expected mapping in {path!s}")
        return cls.from_dict(data)

    def output_stem(self, source_path: str | Path | None = None) -> str:
        if self.output_name:
            return self.output_name
        if source_path is not None:
            return Path(source_path).stem
        if self.name:
            return self.name.lower().replace(" ", "_")
        return self.target

    def to_dict(self) -> Dict[str, Any]:
        return {
            "target": self.target,
            "name": self.name,
            "output_name": self.output_name,
            "seed": self.seed,
            "archetype": self.archetype,
            "variant": self.variant,
            "held_item": self.held_item,
            "spec": dict(self.spec_overrides),
            "animations": list(self.animations),
            "render": dict(self.render.__dict__),
            "faction": self.faction,
            "role": self.role,
            "music_cue": self.music_cue,
            "tags": list(self.tags),
        }


def load_jobs(config_dir: str | Path) -> List[Tuple[Path, CharacterJob]]:
    config_dir = Path(config_dir)
    jobs: List[Tuple[Path, CharacterJob]] = []
    for path in sorted(config_dir.glob("*.yaml")):
        jobs.append((path, CharacterJob.load(path)))
    if not jobs:
        raise FileNotFoundError(f"no .yaml configs found in {config_dir}")
    return jobs
