from __future__ import annotations

from pathlib import Path
from typing import Any, Dict, List, Literal, Optional

import yaml
from pydantic import BaseModel, ConfigDict, Field, field_validator


class RenderConfig(BaseModel):
    model_config = ConfigDict(extra="forbid")

    frame_width: int = 192
    frame_height: int = 192
    supersample: int = 8
    downsample: Literal["lanczos", "nearest"] = "lanczos"
    background: str = "transparent"
    sheet_background: str = "transparent"
    border: int = 0
    label_width: int = 96
    single_width: int = 512
    single_height: int = 512

    @field_validator("frame_width", "frame_height", "single_width", "single_height")
    @classmethod
    def positive_size(cls, value: int) -> int:
        if value <= 0:
            raise ValueError("render dimensions must be positive")
        return value


class CharacterJob(BaseModel):
    """Editable YAML job for a procedural character render."""

    model_config = ConfigDict(extra="forbid")

    target: Literal["goblin", "robot"] = "goblin"
    seed: int = 0
    archetype: str = "default"
    held_item: Optional[str] = None
    animations: List[str] = Field(default_factory=list)
    render: RenderConfig = Field(default_factory=RenderConfig)
    notes: str = ""

    @field_validator("animations")
    @classmethod
    def no_blank_animations(cls, value: List[str]) -> List[str]:
        return [v for v in value if v]


def load_job(path: str | Path) -> CharacterJob:
    data = yaml.safe_load(Path(path).read_text())
    if data is None:
        data = {}
    return CharacterJob.model_validate(data)


def save_job(job: CharacterJob, path: str | Path) -> None:
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    data: Dict[str, Any] = job.model_dump(exclude_none=True)
    Path(path).write_text(yaml.safe_dump(data, sort_keys=False), encoding="utf-8")
