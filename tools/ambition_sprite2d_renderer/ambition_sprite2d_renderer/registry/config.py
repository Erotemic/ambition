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
    # Native texture-resolution multiplier for the published spritesheet.
    # The toon generator designs in a 128-base space scaled to the frame
    # width, so rendering at `render_scale * frame_width` draws the SAME
    # character with more pixels. In-game display size is collision-driven
    # and takes only ASPECT from the frame, so this is pure anti-pixelation:
    # higher resolution under the same on-screen quad, no gameplay change.
    # Default 2 because most sheets are upscaled in game and read soft at 1x.
    render_scale: int = 2
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
    # Optional legacy sheet tuning emitted into the SheetRecord RON.
    # `sheet_tuning:` is canonical; `tuning:` is accepted as a YAML alias.
    sheet_tuning: Optional[Dict[str, Any]] = None
    # Optional sidecar contract fields. These are emitted into
    # <stem>_actor.ron and ignored by current sandbox builds. Keep them
    # loose dictionaries so existing configs remain compatible while the
    # renderer grows a richer actor-spec vocabulary.
    actor: Dict[str, Any] = field(default_factory=dict)
    visual: Dict[str, Any] = field(default_factory=dict)
    body: Dict[str, Any] = field(default_factory=dict)
    capabilities: Dict[str, Any] = field(default_factory=dict)
    brain: Dict[str, Any] = field(default_factory=dict)
    actions: Dict[str, Any] = field(default_factory=dict)
    animation_bindings: Dict[str, Any] = field(default_factory=dict)
    sockets: Dict[str, Any] = field(default_factory=dict)
    missing_information: List[str] = field(default_factory=list)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "CharacterJob":
        render = RenderConfig(**dict(data.get("render") or {}))
        animations = list(data.get("animations") or DEFAULT_ANIMATIONS)
        spec_overrides = dict(data.get("spec") or data.get("spec_overrides") or {})
        # Preserve the existing sheet-tuning contract used by Rust SheetRecord
        # loading. Prefer the explicit `sheet_tuning:` key when both are
        # present; accept `tuning:` as a short alias for hand-authored YAML.
        raw_sheet_tuning = data.get("sheet_tuning")
        if raw_sheet_tuning is None:
            raw_sheet_tuning = data.get("tuning")
        sheet_tuning = (
            dict(raw_sheet_tuning) if isinstance(raw_sheet_tuning, dict) else None
        )
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
            sheet_tuning=sheet_tuning,
            actor=dict(data.get("actor") or {}),
            visual=dict(data.get("visual") or {}),
            body=dict(data.get("body") or {}),
            capabilities=dict(data.get("capabilities") or {}),
            brain=dict(data.get("brain") or {}),
            actions=dict(data.get("actions") or {}),
            animation_bindings=dict(data.get("animation_bindings") or {}),
            sockets=dict(data.get("sockets") or {}),
            missing_information=list(data.get("missing_information") or []),
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
            "sheet_tuning": dict(self.sheet_tuning)
            if self.sheet_tuning is not None
            else None,
            "actor": dict(self.actor),
            "visual": dict(self.visual),
            "body": dict(self.body),
            "capabilities": dict(self.capabilities),
            "brain": dict(self.brain),
            "actions": dict(self.actions),
            "animation_bindings": dict(self.animation_bindings),
            "sockets": dict(self.sockets),
            "missing_information": list(self.missing_information),
        }


def load_jobs(config_dir: str | Path) -> List[Tuple[Path, CharacterJob]]:
    config_dir = Path(config_dir)
    jobs: List[Tuple[Path, CharacterJob]] = []
    for path in sorted(config_dir.glob("*.yaml")):
        jobs.append((path, CharacterJob.load(path)))
    if not jobs:
        raise FileNotFoundError(f"no .yaml configs found in {config_dir}")
    return jobs
